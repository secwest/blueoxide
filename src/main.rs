use blueoxide::advertising::{ConnectRequest, FirstCentralTransmission, decode_advertising_pdu};
use blueoxide::att::{AttPdu, AttUuid, DecodedAttPdu};
use blueoxide::backends::bladerf::{BladeRfOptions, BladeRfSource};
use blueoxide::backends::limesdr::{LimeSdrOptions, LimeSdrSource};
use blueoxide::backends::xtrx::{XtrxOptions, XtrxSource};
use blueoxide::ble::{BleChannel, LeFrameConfig};
use blueoxide::capture::{
    CaptureLimits, CaptureStats, CapturedAdvertisingPdu, capture_primary_advertising,
};
use blueoxide::demod::{
    Le1mDemodConfig, Le1mStreamDecoder, LeUncodedDemodConfig, LeUncodedPacketStreamDecoder,
    LeUncodedPhy, ReceivedAdvertisingPdu, ReceivedLePdu,
};
use blueoxide::iq::{IqFormat, open_iq_file};
use blueoxide::link_layer::{
    ChannelSelectionAlgorithm, ConnectionEventTiming, ConnectionParameters, ConnectionTracker,
    ConnectionTrackerConfig, ControlPdu, DataChannelMap, DataChannelPdu,
    DecodedL2capSignalingCommand, IncompleteL2capPdu, L2capReassembler, L2capReassemblyOutcome,
    L2capSignalingCommand, LE_ACL_MAXIMUM_COUNTER_SKIP, LeAclDecryption, LeAclDecryptionStatus,
    LeAclDecryptor, LinkDirection, LogicalLinkId, SampleTimingError, SleepClockAccuracy,
};
use blueoxide::ll_control::{
    ChannelClassification, CsConfigAction, DecodedControlPdu, LeEncryptionMaterialTracker,
};
use blueoxide::pcapng::{PcapNgWriter, sample_timestamp_ns};
use blueoxide::sdr::{IqSource, SdrConfig};
use blueoxide::smp::{DecodedSmpPdu, SmpAuthenticationRequirements, SmpKeyDistribution, SmpPdu};
use blueoxide::{Error, Result};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_SAMPLES: usize = 16_000_000;
const DEFAULT_BLOCK_SAMPLES: usize = 262_144;

#[derive(Debug)]
struct DecodeArgs {
    input: PathBuf,
    format: IqFormat,
    channel: BleChannel,
    sample_rate_hz: u32,
    max_samples: usize,
    block_samples: usize,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: u64,
}

struct DecodeDataArgs {
    input: PathBuf,
    format: IqFormat,
    channel: BleChannel,
    phy: LeUncodedPhy,
    sample_rate_hz: u32,
    access_address: u32,
    crc_init: u32,
    max_samples: usize,
    block_samples: usize,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: u64,
    plaintext_l2cap_direction: Option<LinkDirection>,
    maximum_l2cap_payload_length: usize,
    decryption: Option<DecodeDataDecryptionArgs>,
}

struct DecodeDataDecryptionArgs {
    session_key: [u8; 16],
    initialization_vector: [u8; 8],
    direction: LinkDirection,
    initial_packet_counter: u64,
    maximum_counter_skip: u64,
}

#[derive(Debug)]
struct CaptureArgs {
    device: String,
    identifier: Option<String>,
    channel: BleChannel,
    sample_rate_hz: u32,
    bandwidth_hz: u32,
    gain_db: f32,
    rx_channel: u8,
    duration: Duration,
    block_samples: usize,
    read_timeout_ms: u64,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: Option<u64>,
}

#[derive(Debug)]
struct ConnectionPlanArgs {
    access_address: u32,
    channel_map: DataChannelMap,
    channel_selection_algorithm: ChannelSelectionAlgorithm,
    hop_increment: u8,
    parameters: ConnectionParameters,
    sample_rate_hz: u32,
    anchor_event_counter: u16,
    anchor_access_address_sample: u64,
    event_count: usize,
    peer_clock_accuracy: SleepClockAccuracy,
    receiver_clock_accuracy_ppm: u32,
    maximum_event_advance: u16,
    first_central_transmission: Option<ConnectionObservationArg>,
    observations: Vec<ConnectionObservationArg>,
    window_size: u8,
    window_offset: u16,
    connect_ind_access_address_sample: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct ConnectionObservationArg {
    channel: BleChannel,
    access_address_sample: u64,
}

fn usage() -> &'static str {
    "blueoxide - Bluetooth/BLE SDR receive and capture tools

USAGE:
  blueoxide channels
  blueoxide backends
  blueoxide decode --input FILE --channel 37|38|39 --sample-rate HZ [OPTIONS]
  blueoxide decode-data --input FILE --channel 0..36 --sample-rate HZ \
    --access-address 0xNNNNNNNN --crc-init 0xNNNNNN [OPTIONS]
  blueoxide connection-plan --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --interval N --sample-rate HZ [OPTIONS]
  blueoxide connection-sync --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --interval N --sample-rate HZ \
    --observe CHANNEL:SAMPLE [OPTIONS]
  blueoxide connection-acquire --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --hop N --interval N --sample-rate HZ \
    --connect-sample N --central-observe CHANNEL:SAMPLE [OPTIONS]
  blueoxide capture --device bladerf|limesdr|xtrx --channel 37|38|39 [OPTIONS]

DECODE OPTIONS:
  --format f32le|s16le    Interleaved little-endian I/Q (default: f32le)
  --max-samples N         Maximum samples accepted from the file (default: 16000000)
  --block-samples N       Streaming decode block size (default: 262144)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Unix capture start in nanoseconds (default: 0)
  -h, --help              Show this help

DECODE-DATA OPTIONS:
  Uses the DECODE OPTIONS above and requires a connection access address and
  24-bit CRC initialization value.
  --phy 1m|2m             Uncoded LE data PHY (default: 1m)
  --plaintext-l2cap-direction central-to-peripheral|peripheral-to-central
                          Reassemble an asserted single-direction plaintext stream
  --max-l2cap-payload N   Maximum reassembled payload length (default: 65535)
  --session-key HEX       16 AES session-key octets, left to right
  --iv HEX                Eight combined LL initialization-vector octets
  --ltk HEX               16 LTK octets in HCI/SMP field order
  --enc-req HEX           Complete 23-octet LL_ENC_REQ control payload
  --enc-rsp HEX           Complete 13-octet LL_ENC_RSP control payload
  --decrypt-direction central-to-peripheral|peripheral-to-central
                          Assert the transmitter direction for AES-CCM
  --packet-counter N      Initial 39-bit direction-specific packet counter
  --max-counter-skip N    MIC-search skipped counters, 0..=65535 (default: 0)

CONNECTION-PLAN OPTIONS:
  --channel-map HEX       Five map octets in over-the-air order
  --csa 1|2               Channel Selection Algorithm
  --hop N                 CSA#1 hop increment, 5..=16 (default: 5)
  --interval N            Connection interval in 1.25 ms units
  --latency N             Peripheral latency (default: 0)
  --timeout N             Supervision timeout in 10 ms units (default: 3200)
  --sample-rate HZ        Hardware sample rate
  --anchor-event N        Observed 16-bit event counter (default: 0)
  --anchor-sample N       Observed access-address sample index (default: 0)
  --events N              Number of events to print (default: 10)
  --peer-sca N            CONNECT_IND sleep-clock accuracy, 0..=7 (default: 0)
  --receiver-ppm N        Receiver sample-clock error bound (default: 20)
  --max-event-advance N   Events searched per observation (default: 32)
  --central-observe CHANNEL:SAMPLE
                          CRC-valid central transmission for event-0 acquisition
  --observe CHANNEL:SAMPLE
                          Later CRC-valid observation; repeat as needed
  --window-size N         CONNECT_IND WinSize in 1.25 ms units (default: 1)
  --window-offset N       CONNECT_IND WinOffset in 1.25 ms units (default: 0)
  --connect-sample N      CONNECT_IND access-address sample for acquisition

CAPTURE OPTIONS:
  --identifier STRING     Native backend device identifier
  --sample-rate HZ        Complex sample rate (default: 4000000)
  --bandwidth HZ          RX bandwidth (default: 2000000)
  --gain DB               RX gain in dB (default: 30)
  --rx-channel N          Hardware RX channel (default: 0)
  --seconds N             Capture duration (default: 10)
  --block-samples N       Native read size (default: 8192)
  --read-timeout-ms N     Native read timeout (default: 1000)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Override Unix capture start in nanoseconds
"
}

fn value_after(args: &[String], index: &mut usize, option: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| Error::InvalidConfiguration(format!("missing value after {option}")))
}

fn parse_number<T: std::str::FromStr>(value: &str, option: &str) -> Result<T> {
    value
        .parse()
        .map_err(|_| Error::InvalidConfiguration(format!("invalid value {value:?} for {option}")))
}

fn parse_u32(value: &str, option: &str) -> Result<u32> {
    let parsed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map(|hex| u32::from_str_radix(hex, 16))
        .unwrap_or_else(|| value.parse());
    parsed.map_err(|_| Error::InvalidConfiguration(format!("invalid value {value:?} for {option}")))
}

fn parse_u64(value: &str, option: &str) -> Result<u64> {
    let parsed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map(|hex| u64::from_str_radix(hex, 16))
        .unwrap_or_else(|| value.parse());
    parsed.map_err(|_| Error::InvalidConfiguration(format!("invalid value {value:?} for {option}")))
}

fn parse_fixed_hex<const OCTETS: usize>(value: &str, option: &str) -> Result<[u8; OCTETS]> {
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if hex.len() != OCTETS * 2 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected {OCTETS} hexadecimal octets"
        )));
    }
    let mut bytes = [0u8; OCTETS];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).map_err(|_| {
            Error::InvalidConfiguration(format!("invalid value {value:?} for {option}"))
        })?;
    }
    Ok(bytes)
}

fn parse_link_direction(value: &str, option: &str) -> Result<LinkDirection> {
    match value {
        "central-to-peripheral" | "central" | "c2p" => Ok(LinkDirection::CentralToPeripheral),
        "peripheral-to-central" | "peripheral" | "p2c" => Ok(LinkDirection::PeripheralToCentral),
        _ => Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected central-to-peripheral or peripheral-to-central"
        ))),
    }
}

fn parse_uncoded_phy(value: &str, option: &str) -> Result<LeUncodedPhy> {
    match value.to_ascii_lowercase().as_str() {
        "1m" | "le-1m" => Ok(LeUncodedPhy::Le1M),
        "2m" | "le-2m" => Ok(LeUncodedPhy::Le2M),
        _ => Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected 1m or 2m"
        ))),
    }
}

fn parse_channel_map(value: &str) -> Result<DataChannelMap> {
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if hex.len() != 10 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --channel-map; expected five hexadecimal octets"
        )));
    }
    let mut bytes = [0u8; 5];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).map_err(|_| {
            Error::InvalidConfiguration(format!("invalid value {value:?} for --channel-map"))
        })?;
    }
    DataChannelMap::new(bytes)
}

fn parse_connection_observation(value: &str, option: &str) -> Result<ConnectionObservationArg> {
    let (channel, sample) = value.split_once(':').ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected CHANNEL:SAMPLE"
        ))
    })?;
    let channel = BleChannel::new(parse_number(channel, &format!("{option} channel"))?)?;
    if channel.index() > 36 {
        return Err(Error::InvalidConfiguration(format!(
            "{option} requires a data channel in 0..=36; got {}",
            channel.index()
        )));
    }
    Ok(ConnectionObservationArg {
        channel,
        access_address_sample: parse_number(sample, &format!("{option} sample"))?,
    })
}

fn parse_connection_plan_args(args: &[String]) -> Result<ConnectionPlanArgs> {
    let mut access_address = None;
    let mut channel_map = None;
    let mut channel_selection_algorithm = None;
    let mut hop_increment = 5u8;
    let mut interval = None;
    let mut latency = 0u16;
    let mut supervision_timeout = 3_200u16;
    let mut sample_rate_hz = None;
    let mut anchor_event_counter = 0u16;
    let mut anchor_access_address_sample = 0u64;
    let mut event_count = 10usize;
    let mut peer_clock_accuracy = SleepClockAccuracy::new(0)?;
    let mut receiver_clock_accuracy_ppm = 20u32;
    let mut maximum_event_advance = 32u16;
    let mut first_central_transmission = None;
    let mut observations = Vec::new();
    let mut window_size = 1u8;
    let mut window_offset = 0u16;
    let mut connect_ind_access_address_sample = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--access-address" => {
                let value = value_after(args, &mut index, "--access-address")?;
                access_address = Some(parse_u32(&value, "--access-address")?);
            }
            "--channel-map" => {
                channel_map = Some(parse_channel_map(&value_after(
                    args,
                    &mut index,
                    "--channel-map",
                )?)?);
            }
            "--csa" => {
                let value = value_after(args, &mut index, "--csa")?;
                channel_selection_algorithm = Some(match value.to_ascii_lowercase().as_str() {
                    "1" | "csa1" | "csa#1" => ChannelSelectionAlgorithm::Csa1,
                    "2" | "csa2" | "csa#2" => ChannelSelectionAlgorithm::Csa2,
                    _ => {
                        return Err(Error::InvalidConfiguration(format!(
                            "invalid value {value:?} for --csa; expected 1 or 2"
                        )));
                    }
                });
            }
            "--hop" => {
                let value = value_after(args, &mut index, "--hop")?;
                hop_increment = parse_number(&value, "--hop")?;
            }
            "--interval" => {
                let value = value_after(args, &mut index, "--interval")?;
                interval = Some(parse_number(&value, "--interval")?);
            }
            "--latency" => {
                let value = value_after(args, &mut index, "--latency")?;
                latency = parse_number(&value, "--latency")?;
            }
            "--timeout" => {
                let value = value_after(args, &mut index, "--timeout")?;
                supervision_timeout = parse_number(&value, "--timeout")?;
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = Some(parse_number(&value, "--sample-rate")?);
            }
            "--anchor-event" => {
                let value = value_after(args, &mut index, "--anchor-event")?;
                anchor_event_counter = parse_number(&value, "--anchor-event")?;
            }
            "--anchor-sample" => {
                let value = value_after(args, &mut index, "--anchor-sample")?;
                anchor_access_address_sample = parse_number(&value, "--anchor-sample")?;
            }
            "--events" => {
                let value = value_after(args, &mut index, "--events")?;
                event_count = parse_number(&value, "--events")?;
            }
            "--peer-sca" => {
                let value = value_after(args, &mut index, "--peer-sca")?;
                peer_clock_accuracy = SleepClockAccuracy::new(parse_number(&value, "--peer-sca")?)?;
            }
            "--receiver-ppm" => {
                let value = value_after(args, &mut index, "--receiver-ppm")?;
                receiver_clock_accuracy_ppm = parse_number(&value, "--receiver-ppm")?;
            }
            "--max-event-advance" => {
                let value = value_after(args, &mut index, "--max-event-advance")?;
                maximum_event_advance = parse_number(&value, "--max-event-advance")?;
            }
            "--central-observe" => {
                let observation = parse_connection_observation(
                    &value_after(args, &mut index, "--central-observe")?,
                    "--central-observe",
                )?;
                if first_central_transmission.replace(observation).is_some() {
                    return Err(Error::InvalidConfiguration(
                        "--central-observe may only be supplied once".to_owned(),
                    ));
                }
            }
            "--observe" => observations.push(parse_connection_observation(
                &value_after(args, &mut index, "--observe")?,
                "--observe",
            )?),
            "--window-size" => {
                let value = value_after(args, &mut index, "--window-size")?;
                window_size = parse_number(&value, "--window-size")?;
            }
            "--window-offset" => {
                let value = value_after(args, &mut index, "--window-offset")?;
                window_offset = parse_number(&value, "--window-offset")?;
            }
            "--connect-sample" => {
                let value = value_after(args, &mut index, "--connect-sample")?;
                connect_ind_access_address_sample = Some(parse_number(&value, "--connect-sample")?);
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown connection-plan option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    if event_count == 0 || event_count > 1_000_000 {
        return Err(Error::InvalidConfiguration(
            "--events must be in 1..=1000000".to_owned(),
        ));
    }
    Ok(ConnectionPlanArgs {
        access_address: access_address.ok_or_else(|| {
            Error::InvalidConfiguration("connection-plan requires --access-address".to_owned())
        })?,
        channel_map: channel_map.ok_or_else(|| {
            Error::InvalidConfiguration("connection-plan requires --channel-map".to_owned())
        })?,
        channel_selection_algorithm: channel_selection_algorithm.ok_or_else(|| {
            Error::InvalidConfiguration("connection-plan requires --csa 1|2".to_owned())
        })?,
        hop_increment,
        parameters: ConnectionParameters::new(
            interval.ok_or_else(|| {
                Error::InvalidConfiguration("connection-plan requires --interval".to_owned())
            })?,
            latency,
            supervision_timeout,
        )?,
        sample_rate_hz: sample_rate_hz.ok_or_else(|| {
            Error::InvalidConfiguration("connection-plan requires --sample-rate".to_owned())
        })?,
        anchor_event_counter,
        anchor_access_address_sample,
        event_count,
        peer_clock_accuracy,
        receiver_clock_accuracy_ppm,
        maximum_event_advance,
        first_central_transmission,
        observations,
        window_size,
        window_offset,
        connect_ind_access_address_sample,
    })
}

fn parse_decode_args(args: &[String]) -> Result<DecodeArgs> {
    let mut input = None;
    let mut format = IqFormat::F32Le;
    let mut channel = None;
    let mut sample_rate_hz = None;
    let mut max_samples = DEFAULT_MAX_SAMPLES;
    let mut block_samples = DEFAULT_BLOCK_SAMPLES;
    let mut max_access_address_errors = 1u8;
    let mut output_pcap = None;
    let mut capture_start_ns = 0u64;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => input = Some(PathBuf::from(value_after(args, &mut index, "--input")?)),
            "--format" => format = IqFormat::parse(&value_after(args, &mut index, "--format")?)?,
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = Some(parse_number(&value, "--sample-rate")?);
            }
            "--max-samples" => {
                let value = value_after(args, &mut index, "--max-samples")?;
                max_samples = parse_number(&value, "--max-samples")?;
            }
            "--block-samples" => {
                let value = value_after(args, &mut index, "--block-samples")?;
                block_samples = parse_number(&value, "--block-samples")?;
            }
            "--aa-errors" => {
                let value = value_after(args, &mut index, "--aa-errors")?;
                max_access_address_errors = parse_number(&value, "--aa-errors")?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                let value = value_after(args, &mut index, "--capture-start-ns")?;
                capture_start_ns = parse_number(&value, "--capture-start-ns")?;
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown decode option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    Ok(DecodeArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --input FILE".to_owned())
        })?,
        format,
        channel: channel.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --channel 37|38|39".to_owned())
        })?,
        sample_rate_hz: sample_rate_hz.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --sample-rate HZ".to_owned())
        })?,
        max_samples,
        block_samples,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
    })
}

fn parse_decode_data_args(args: &[String]) -> Result<DecodeDataArgs> {
    let mut input = None;
    let mut format = IqFormat::F32Le;
    let mut channel = None;
    let mut phy = LeUncodedPhy::Le1M;
    let mut sample_rate_hz = None;
    let mut access_address = None;
    let mut crc_init = None;
    let mut max_samples = DEFAULT_MAX_SAMPLES;
    let mut block_samples = DEFAULT_BLOCK_SAMPLES;
    let mut max_access_address_errors = 1u8;
    let mut output_pcap = None;
    let mut capture_start_ns = 0u64;
    let mut plaintext_l2cap_direction = None;
    let mut maximum_l2cap_payload_length = usize::from(u16::MAX);
    let mut maximum_l2cap_payload_length_supplied = false;
    let mut session_key = None;
    let mut initialization_vector = None;
    let mut long_term_key = None;
    let mut encryption_request: Option<[u8; 23]> = None;
    let mut encryption_response: Option<[u8; 13]> = None;
    let mut decryption_direction = None;
    let mut initial_packet_counter = None;
    let mut maximum_counter_skip = 0u64;
    let mut maximum_counter_skip_supplied = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => input = Some(PathBuf::from(value_after(args, &mut index, "--input")?)),
            "--format" => format = IqFormat::parse(&value_after(args, &mut index, "--format")?)?,
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--phy" => {
                let value = value_after(args, &mut index, "--phy")?;
                phy = parse_uncoded_phy(&value, "--phy")?;
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = Some(parse_number(&value, "--sample-rate")?);
            }
            "--access-address" => {
                let value = value_after(args, &mut index, "--access-address")?;
                access_address = Some(parse_u32(&value, "--access-address")?);
            }
            "--crc-init" => {
                let value = value_after(args, &mut index, "--crc-init")?;
                crc_init = Some(parse_u32(&value, "--crc-init")?);
            }
            "--max-samples" => {
                let value = value_after(args, &mut index, "--max-samples")?;
                max_samples = parse_number(&value, "--max-samples")?;
            }
            "--block-samples" => {
                let value = value_after(args, &mut index, "--block-samples")?;
                block_samples = parse_number(&value, "--block-samples")?;
            }
            "--aa-errors" => {
                let value = value_after(args, &mut index, "--aa-errors")?;
                max_access_address_errors = parse_number(&value, "--aa-errors")?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                let value = value_after(args, &mut index, "--capture-start-ns")?;
                capture_start_ns = parse_number(&value, "--capture-start-ns")?;
            }
            "--plaintext-l2cap-direction" => {
                let value = value_after(args, &mut index, "--plaintext-l2cap-direction")?;
                plaintext_l2cap_direction =
                    Some(parse_link_direction(&value, "--plaintext-l2cap-direction")?);
            }
            "--max-l2cap-payload" => {
                let value = value_after(args, &mut index, "--max-l2cap-payload")?;
                maximum_l2cap_payload_length = parse_number(&value, "--max-l2cap-payload")?;
                maximum_l2cap_payload_length_supplied = true;
            }
            "--session-key" => {
                let value = value_after(args, &mut index, "--session-key")?;
                session_key = Some(parse_fixed_hex(&value, "--session-key")?);
            }
            "--iv" => {
                let value = value_after(args, &mut index, "--iv")?;
                initialization_vector = Some(parse_fixed_hex(&value, "--iv")?);
            }
            "--ltk" => {
                let value = value_after(args, &mut index, "--ltk")?;
                long_term_key = Some(parse_fixed_hex(&value, "--ltk")?);
            }
            "--enc-req" => {
                let value = value_after(args, &mut index, "--enc-req")?;
                encryption_request = Some(parse_fixed_hex(&value, "--enc-req")?);
            }
            "--enc-rsp" => {
                let value = value_after(args, &mut index, "--enc-rsp")?;
                encryption_response = Some(parse_fixed_hex(&value, "--enc-rsp")?);
            }
            "--decrypt-direction" => {
                let value = value_after(args, &mut index, "--decrypt-direction")?;
                decryption_direction = Some(parse_link_direction(&value, "--decrypt-direction")?);
            }
            "--packet-counter" => {
                let value = value_after(args, &mut index, "--packet-counter")?;
                initial_packet_counter = Some(parse_u64(&value, "--packet-counter")?);
            }
            "--max-counter-skip" => {
                let value = value_after(args, &mut index, "--max-counter-skip")?;
                maximum_counter_skip = parse_u64(&value, "--max-counter-skip")?;
                maximum_counter_skip_supplied = true;
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown decode-data option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    let channel = channel.ok_or_else(|| {
        Error::InvalidConfiguration("decode-data requires --channel 0..36".to_owned())
    })?;
    if channel.index() > 36 {
        return Err(Error::InvalidConfiguration(format!(
            "decode-data requires a data channel in 0..=36; got {}",
            channel.index()
        )));
    }
    let sample_rate_hz = sample_rate_hz.ok_or_else(|| {
        Error::InvalidConfiguration("decode-data requires --sample-rate HZ".to_owned())
    })?;
    LeUncodedDemodConfig {
        phy,
        sample_rate_hz,
        max_access_address_errors,
    }
    .validate()?;
    let access_address = access_address.ok_or_else(|| {
        Error::InvalidConfiguration("decode-data requires --access-address".to_owned())
    })?;
    let crc_init = crc_init
        .ok_or_else(|| Error::InvalidConfiguration("decode-data requires --crc-init".to_owned()))?;
    LeFrameConfig::data(access_address, crc_init)?;
    if maximum_l2cap_payload_length_supplied && plaintext_l2cap_direction.is_none() {
        return Err(Error::InvalidConfiguration(
            "--max-l2cap-payload requires --plaintext-l2cap-direction".to_owned(),
        ));
    }
    if plaintext_l2cap_direction.is_some() {
        L2capReassembler::new(maximum_l2cap_payload_length)?;
    }
    if maximum_counter_skip > LE_ACL_MAXIMUM_COUNTER_SKIP {
        return Err(Error::InvalidConfiguration(format!(
            "--max-counter-skip must be in 0..={LE_ACL_MAXIMUM_COUNTER_SKIP}"
        )));
    }
    let direct_material_option_count = [session_key.is_some(), initialization_vector.is_some()]
        .into_iter()
        .filter(|supplied| *supplied)
        .count();
    if direct_material_option_count != 0 && direct_material_option_count != 2 {
        return Err(Error::InvalidConfiguration(
            "--session-key and --iv must be supplied together".to_owned(),
        ));
    }
    let exchange_material_option_count = [
        long_term_key.is_some(),
        encryption_request.is_some(),
        encryption_response.is_some(),
    ]
    .into_iter()
    .filter(|supplied| *supplied)
    .count();
    if exchange_material_option_count != 0 && exchange_material_option_count != 3 {
        return Err(Error::InvalidConfiguration(
            "--ltk, --enc-req, and --enc-rsp must be supplied together".to_owned(),
        ));
    }
    if direct_material_option_count != 0 && exchange_material_option_count != 0 {
        return Err(Error::InvalidConfiguration(
            "--session-key/--iv and --ltk/--enc-req/--enc-rsp are mutually exclusive".to_owned(),
        ));
    }
    let has_key_material = direct_material_option_count == 2 || exchange_material_option_count == 3;
    let decryption_state_option_count = [
        decryption_direction.is_some(),
        initial_packet_counter.is_some(),
    ]
    .into_iter()
    .filter(|supplied| *supplied)
    .count();
    if has_key_material && decryption_state_option_count != 2 {
        return Err(Error::InvalidConfiguration(
            "key material, --decrypt-direction, and --packet-counter must be supplied together"
                .to_owned(),
        ));
    }
    if !has_key_material && decryption_state_option_count != 0 {
        return Err(Error::InvalidConfiguration(
            "--decrypt-direction and --packet-counter require a complete key-material option set"
                .to_owned(),
        ));
    }
    if maximum_counter_skip_supplied && !has_key_material {
        return Err(Error::InvalidConfiguration(
            "--max-counter-skip requires the complete decryption option set".to_owned(),
        ));
    }
    if let (Some(plaintext), Some(decryption)) = (plaintext_l2cap_direction, decryption_direction)
        && plaintext != decryption
    {
        return Err(Error::InvalidConfiguration(
            "--plaintext-l2cap-direction must match --decrypt-direction".to_owned(),
        ));
    }
    let material = match (
        session_key,
        initialization_vector,
        long_term_key,
        encryption_request,
        encryption_response,
    ) {
        (Some(session_key), Some(initialization_vector), None, None, None) => {
            Some((session_key, initialization_vector))
        }
        (None, None, Some(long_term_key), Some(request), Some(response)) => {
            if request[0] != 0x03 {
                return Err(Error::InvalidConfiguration(format!(
                    "--enc-req must begin with LL_ENC_REQ opcode 03, received {:02x}",
                    request[0]
                )));
            }
            if response[0] != 0x04 {
                return Err(Error::InvalidConfiguration(format!(
                    "--enc-rsp must begin with LL_ENC_RSP opcode 04, received {:02x}",
                    response[0]
                )));
            }
            let mut tracker = LeEncryptionMaterialTracker::new(long_term_key);
            tracker.observe(
                LinkDirection::CentralToPeripheral,
                ControlPdu {
                    opcode: request[0],
                    parameters: &request[1..],
                },
            )?;
            let material = tracker
                .observe(
                    LinkDirection::PeripheralToCentral,
                    ControlPdu {
                        opcode: response[0],
                        parameters: &response[1..],
                    },
                )?
                .ok_or_else(|| {
                    Error::InvalidState(
                        "LL encryption exchange did not produce session material".to_owned(),
                    )
                })?;
            Some((material.session_key(), material.initialization_vector()))
        }
        (None, None, None, None, None) => None,
        _ => unreachable!("partial or conflicting key-material options rejected above"),
    };
    let decryption = match (material, decryption_direction, initial_packet_counter) {
        (Some((session_key, initialization_vector)), Some(direction), Some(packet_counter)) => {
            LeAclDecryptor::new(
                session_key,
                initialization_vector,
                direction,
                packet_counter,
                maximum_counter_skip,
            )?;
            Some(DecodeDataDecryptionArgs {
                session_key,
                initialization_vector,
                direction,
                initial_packet_counter: packet_counter,
                maximum_counter_skip,
            })
        }
        (None, None, None) => None,
        _ => unreachable!("partial decryption state options rejected above"),
    };

    Ok(DecodeDataArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode-data requires --input FILE".to_owned())
        })?,
        format,
        channel,
        phy,
        sample_rate_hz,
        access_address,
        crc_init,
        max_samples,
        block_samples,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
        plaintext_l2cap_direction,
        maximum_l2cap_payload_length,
        decryption,
    })
}

fn parse_capture_args(args: &[String]) -> Result<CaptureArgs> {
    let mut device = None;
    let mut identifier = None;
    let mut channel = None;
    let mut sample_rate_hz = 4_000_000u32;
    let mut bandwidth_hz = 2_000_000u32;
    let mut gain_db = 30.0f32;
    let mut rx_channel = 0u8;
    let mut duration = Duration::from_secs(10);
    let mut block_samples = 8_192usize;
    let mut read_timeout_ms = 1_000u64;
    let mut max_access_address_errors = 1u8;
    let mut output_pcap = None;
    let mut capture_start_ns = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--device" => device = Some(value_after(args, &mut index, "--device")?),
            "--identifier" => identifier = Some(value_after(args, &mut index, "--identifier")?),
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = parse_number(&value, "--sample-rate")?;
            }
            "--bandwidth" => {
                let value = value_after(args, &mut index, "--bandwidth")?;
                bandwidth_hz = parse_number(&value, "--bandwidth")?;
            }
            "--gain" => {
                let value = value_after(args, &mut index, "--gain")?;
                gain_db = parse_number(&value, "--gain")?;
            }
            "--rx-channel" => {
                let value = value_after(args, &mut index, "--rx-channel")?;
                rx_channel = parse_number(&value, "--rx-channel")?;
            }
            "--seconds" => {
                let value = value_after(args, &mut index, "--seconds")?;
                let seconds: f64 = parse_number(&value, "--seconds")?;
                duration = Duration::try_from_secs_f64(seconds).map_err(|_| {
                    Error::InvalidConfiguration(
                        "--seconds must be finite, greater than zero, and representable as a duration"
                            .to_owned(),
                    )
                })?;
                if duration == Duration::ZERO {
                    return Err(Error::InvalidConfiguration(
                        "--seconds must be greater than zero".to_owned(),
                    ));
                }
            }
            "--block-samples" => {
                let value = value_after(args, &mut index, "--block-samples")?;
                block_samples = parse_number(&value, "--block-samples")?;
            }
            "--read-timeout-ms" => {
                let value = value_after(args, &mut index, "--read-timeout-ms")?;
                read_timeout_ms = parse_number(&value, "--read-timeout-ms")?;
            }
            "--aa-errors" => {
                let value = value_after(args, &mut index, "--aa-errors")?;
                max_access_address_errors = parse_number(&value, "--aa-errors")?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                let value = value_after(args, &mut index, "--capture-start-ns")?;
                capture_start_ns = Some(parse_number(&value, "--capture-start-ns")?);
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown capture option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    if block_samples == 0 || block_samples > c_int_max_as_usize() {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be in 1..=2147483647 for current live backends".to_owned(),
        ));
    }
    if read_timeout_ms == 0 || read_timeout_ms > u32::MAX as u64 {
        return Err(Error::InvalidConfiguration(
            "--read-timeout-ms must be in 1..=4294967295 for current live backends".to_owned(),
        ));
    }
    if !gain_db.is_finite() {
        return Err(Error::InvalidConfiguration(
            "--gain must be finite".to_owned(),
        ));
    }
    Ok(CaptureArgs {
        device: device.ok_or_else(|| {
            Error::InvalidConfiguration("capture requires --device bladerf|limesdr|xtrx".to_owned())
        })?,
        identifier,
        channel: channel.ok_or_else(|| {
            Error::InvalidConfiguration("capture requires --channel 37|38|39".to_owned())
        })?,
        sample_rate_hz,
        bandwidth_hz,
        gain_db,
        rx_channel,
        duration,
        block_samples,
        read_timeout_ms,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
    })
}

const fn c_int_max_as_usize() -> usize {
    i32::MAX as usize
}

fn print_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn print_signed_hex(bytes: &[i8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{:02x}", *byte as u8);
    }
    output
}

fn print_packet(packet: &ReceivedAdvertisingPdu) {
    let semantic = decode_advertising_pdu(&packet.pdu)
        .map(|decoded| decoded.to_string())
        .unwrap_or_else(|error| format!("decode_error={error}"));
    println!(
        "channel={} sample={} phase={} inverted={} aa_errors={} pdu_type={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} semantic=\"{}\"",
        packet.pdu.channel.index(),
        packet.access_address_sample,
        packet.symbol_phase,
        packet.pdu.inverted,
        packet.pdu.access_address_errors,
        packet.pdu.pdu_type(),
        packet.estimated_carrier_offset_hz,
        packet.estimated_deviation_hz,
        print_hex(&packet.pdu.header),
        print_hex(&packet.pdu.payload),
        print_hex(&packet.pdu.crc),
        semantic.replace('"', "'"),
    );
}

fn describe_control_pdu(control: ControlPdu<'_>) -> Result<String> {
    Ok(match control.decode()? {
        DecodedControlPdu::ConnectionUpdateInd(update) => format!(
            "{} opcode=0x{:02x} window_offset={} window_size={} interval={} latency={} timeout={} instant={}",
            control.opcode_name(),
            control.opcode,
            update.window_offset,
            update.window_size,
            update.parameters.interval,
            update.parameters.latency,
            update.parameters.supervision_timeout,
            update.instant
        ),
        DecodedControlPdu::ChannelMapInd(update) => format!(
            "{} opcode=0x{:02x} channel_map={} channels={} instant={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&update.channel_map.bytes()),
            update.channel_map.used_count(),
            update.instant
        ),
        DecodedControlPdu::TerminateInd(value) | DecodedControlPdu::RejectInd(value) => format!(
            "{} opcode=0x{:02x} error_code=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.error_code
        ),
        DecodedControlPdu::EncryptionRequest(value) => format!(
            "{} opcode=0x{:02x} rand={} ediv=0x{:04x} skd_c={} iv_c={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&value.random_number),
            value.encrypted_diversifier,
            print_hex(&value.central_session_key_diversifier),
            print_hex(&value.central_initialization_vector)
        ),
        DecodedControlPdu::EncryptionResponse(value) => format!(
            "{} opcode=0x{:02x} skd_p={} iv_p={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&value.peripheral_session_key_diversifier),
            print_hex(&value.peripheral_initialization_vector)
        ),
        DecodedControlPdu::StartEncryptionRequest
        | DecodedControlPdu::StartEncryptionResponse
        | DecodedControlPdu::PauseEncryptionRequest
        | DecodedControlPdu::PauseEncryptionResponse
        | DecodedControlPdu::PingRequest
        | DecodedControlPdu::PingResponse
        | DecodedControlPdu::CteResponse
        | DecodedControlPdu::CsFaeRequest => {
            format!("{} opcode=0x{:02x}", control.opcode_name(), control.opcode)
        }
        DecodedControlPdu::UnknownResponse(value) => format!(
            "{} opcode=0x{:02x} unknown_type=0x{:02x} unknown_name={}",
            control.opcode_name(),
            control.opcode,
            value.unknown_type,
            ControlPdu {
                opcode: value.unknown_type,
                parameters: &[]
            }
            .opcode_name()
        ),
        DecodedControlPdu::FeatureRequest(value)
        | DecodedControlPdu::FeatureResponse(value)
        | DecodedControlPdu::PeripheralFeatureRequest(value) => format!(
            "{} opcode=0x{:02x} features={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&value.bytes)
        ),
        DecodedControlPdu::VersionInd(value) => format!(
            "{} opcode=0x{:02x} version=0x{:02x} company=0x{:04x} subversion=0x{:04x}",
            control.opcode_name(),
            control.opcode,
            value.version,
            value.company_identifier,
            value.subversion
        ),
        DecodedControlPdu::ConnectionParameterRequest(value)
        | DecodedControlPdu::ConnectionParameterResponse(value) => format!(
            "{} opcode=0x{:02x} interval_min={} interval_max={} latency={} timeout={} preferred_periodicity={} reference_event={} offsets={:04x},{:04x},{:04x},{:04x},{:04x},{:04x}",
            control.opcode_name(),
            control.opcode,
            value.interval_min,
            value.interval_max,
            value.latency,
            value.supervision_timeout,
            value.preferred_periodicity,
            value.reference_connection_event_count,
            value.offsets[0],
            value.offsets[1],
            value.offsets[2],
            value.offsets[3],
            value.offsets[4],
            value.offsets[5]
        ),
        DecodedControlPdu::RejectExtendedInd(value) => format!(
            "{} opcode=0x{:02x} rejected_opcode=0x{:02x} rejected_name={} error_code=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.rejected_opcode,
            ControlPdu {
                opcode: value.rejected_opcode,
                parameters: &[]
            }
            .opcode_name(),
            value.error_code
        ),
        DecodedControlPdu::LengthRequest(value) | DecodedControlPdu::LengthResponse(value) => {
            format!(
                "{} opcode=0x{:02x} max_rx_octets={} max_rx_time_us={} max_tx_octets={} max_tx_time_us={}",
                control.opcode_name(),
                control.opcode,
                value.maximum_receive_octets,
                value.maximum_receive_time_us,
                value.maximum_transmit_octets,
                value.maximum_transmit_time_us
            )
        }
        DecodedControlPdu::PhyRequest(value) | DecodedControlPdu::PhyResponse(value) => format!(
            "{} opcode=0x{:02x} tx_phys=0x{:02x} rx_phys=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.transmit_phys,
            value.receive_phys
        ),
        DecodedControlPdu::PhyUpdateInd(value) => format!(
            "{} opcode=0x{:02x} central_to_peripheral_phy=0x{:02x} peripheral_to_central_phy=0x{:02x} instant={}",
            control.opcode_name(),
            control.opcode,
            value.central_to_peripheral_phy,
            value.peripheral_to_central_phy,
            value.instant
        ),
        DecodedControlPdu::MinimumUsedChannelsInd(value) => format!(
            "{} opcode=0x{:02x} phys=0x{:02x} minimum_used_channels={}",
            control.opcode_name(),
            control.opcode,
            value.phys,
            value.minimum_used_channels
        ),
        DecodedControlPdu::CteRequest(value) => format!(
            "{} opcode=0x{:02x} minimum_cte_length={} minimum_cte_us={} cte_type={} cte_type_name={}",
            control.opcode_name(),
            control.opcode,
            value.minimum_length_units,
            value.minimum_duration_us(),
            value.cte_type,
            value.cte_type_name()
        ),
        DecodedControlPdu::PeriodicSyncInd(value) => format!(
            "{} opcode=0x{:02x} id=0x{:04x} sync_offset_us={} interval={} channel_map={} access_address={:08x} crc_init={:06x} periodic_event={} connection_event={} last_periodic_event={} sid={} address_type={} sender_sca={} phy=0x{:02x} advertiser={} sync_connection_event={}",
            control.opcode_name(),
            control.opcode,
            value.identifier,
            value.sync_info.packet_window_offset_us(),
            value.sync_info.interval,
            print_hex(&value.sync_info.channel_map.bytes()),
            value.sync_info.access_address,
            value.sync_info.crc_init,
            value.sync_info.periodic_event_counter,
            value.connection_event_count,
            value.last_periodic_event_counter,
            value.advertising_sid,
            if value.advertiser_address_random {
                "random"
            } else {
                "public"
            },
            value.sender_sleep_clock_accuracy.raw(),
            value.phy,
            print_hex(&value.advertiser_address),
            value.sync_connection_event_count
        ),
        DecodedControlPdu::ClockAccuracyRequest(value)
        | DecodedControlPdu::ClockAccuracyResponse(value) => format!(
            "{} opcode=0x{:02x} sca={} maximum_ppm={}",
            control.opcode_name(),
            control.opcode,
            value.raw(),
            value.maximum_ppm()
        ),
        DecodedControlPdu::CisRequest(value) => format!(
            "{} opcode=0x{:02x} cig={} cis={} central_phy=0x{:02x} peripheral_phy=0x{:02x} central_max_sdu={} peripheral_max_sdu={} framed={} framing_mode={} central_sdu_interval_us={} peripheral_sdu_interval_us={} central_max_pdu={} peripheral_max_pdu={} nse={} sub_interval_us={} central_bn={} peripheral_bn={} central_ft={} peripheral_ft={} iso_interval={} offset_min_us={} offset_max_us={} connection_event={}",
            control.opcode_name(),
            control.opcode,
            value.cig_identifier,
            value.cis_identifier,
            value.central_to_peripheral_phy,
            value.peripheral_to_central_phy,
            value.maximum_central_sdu,
            value.maximum_peripheral_sdu,
            value.framed,
            if value.framing_mode_unsegmented {
                "unsegmented"
            } else {
                "segmentable"
            },
            value.central_sdu_interval_us,
            value.peripheral_sdu_interval_us,
            value.maximum_central_pdu,
            value.maximum_peripheral_pdu,
            value.subevents,
            value.subevent_interval_us,
            value.central_burst_number,
            value.peripheral_burst_number,
            value.central_flush_timeout,
            value.peripheral_flush_timeout,
            value.iso_interval,
            value.cis_offset_min_us,
            value.cis_offset_max_us,
            value.connection_event_count
        ),
        DecodedControlPdu::CisResponse(value) => format!(
            "{} opcode=0x{:02x} offset_min_us={} offset_max_us={} connection_event={}",
            control.opcode_name(),
            control.opcode,
            value.cis_offset_min_us,
            value.cis_offset_max_us,
            value.connection_event_count
        ),
        DecodedControlPdu::CisInd(value) => format!(
            "{} opcode=0x{:02x} access_address={:08x} cis_offset_us={} cig_sync_delay_us={} cis_sync_delay_us={} connection_event={}",
            control.opcode_name(),
            control.opcode,
            value.access_address,
            value.cis_offset_us,
            value.cig_sync_delay_us,
            value.cis_sync_delay_us,
            value.connection_event_count
        ),
        DecodedControlPdu::CisTerminateInd(value) => format!(
            "{} opcode=0x{:02x} cig={} cis={} error_code=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.cig_identifier,
            value.cis_identifier,
            value.error_code
        ),
        DecodedControlPdu::PowerControlRequest(value) => format!(
            "{} opcode=0x{:02x} phy=0x{:02x} delta_db={} tx_power_dbm={}",
            control.opcode_name(),
            control.opcode,
            value.phy,
            value.delta_db,
            value.transmit_power_dbm
        ),
        DecodedControlPdu::PowerControlResponse(value) => format!(
            "{} opcode=0x{:02x} minimum={} maximum={} delta_db={} tx_power_dbm={} acceptable_reduction_db={}",
            control.opcode_name(),
            control.opcode,
            value.at_minimum,
            value.at_maximum,
            value.delta_db,
            value.transmit_power_dbm,
            value.acceptable_power_reduction_db
        ),
        DecodedControlPdu::PowerChangeInd(value) => format!(
            "{} opcode=0x{:02x} phys=0x{:02x} minimum={} maximum={} delta_db={} tx_power_dbm={}",
            control.opcode_name(),
            control.opcode,
            value.phys,
            value.at_minimum,
            value.at_maximum,
            value.delta_db,
            value.transmit_power_dbm
        ),
        DecodedControlPdu::SubrateRequest(value) => format!(
            "{} opcode=0x{:02x} factor_min={} factor_max={} maximum_latency={} continuation_number={} timeout={}",
            control.opcode_name(),
            control.opcode,
            value.factor_min,
            value.factor_max,
            value.maximum_latency,
            value.continuation_number,
            value.supervision_timeout
        ),
        DecodedControlPdu::SubrateInd(value) => format!(
            "{} opcode=0x{:02x} factor={} base_event={} latency={} continuation_number={} timeout={}",
            control.opcode_name(),
            control.opcode,
            value.factor,
            value.base_event,
            value.latency,
            value.continuation_number,
            value.supervision_timeout
        ),
        DecodedControlPdu::ChannelReportingInd(value) => format!(
            "{} opcode=0x{:02x} enabled={} minimum_spacing={} maximum_delay={}",
            control.opcode_name(),
            control.opcode,
            value.enabled,
            value.minimum_spacing,
            value.maximum_delay
        ),
        DecodedControlPdu::ChannelStatusInd(value) => {
            let good = value
                .classifications
                .iter()
                .filter(|classification| **classification == ChannelClassification::Good)
                .count();
            let bad = value
                .classifications
                .iter()
                .filter(|classification| **classification == ChannelClassification::Bad)
                .count();
            format!(
                "{} opcode=0x{:02x} good_channels={} bad_channels={} unknown_channels={}",
                control.opcode_name(),
                control.opcode,
                good,
                bad,
                value.classifications.len() - good - bad
            )
        }
        DecodedControlPdu::PeriodicSyncWrInd(value) => format!(
            "{} opcode=0x{:02x} id=0x{:04x} sync_offset_us={} interval={} channel_map={} periodic_access_address={:08x} response_access_address={:08x} subevents={} subevent_interval={} response_slot_delay={} response_slot_spacing={}",
            control.opcode_name(),
            control.opcode,
            value.periodic_sync.identifier,
            value.periodic_sync.sync_info.packet_window_offset_us(),
            value.periodic_sync.sync_info.interval,
            print_hex(&value.periodic_sync.sync_info.channel_map.bytes()),
            value.periodic_sync.sync_info.access_address,
            value.response_access_address,
            value.subevent_count,
            value.subevent_interval,
            value.response_slot_delay,
            value.response_slot_spacing
        ),
        DecodedControlPdu::FeatureExtendedRequest(value)
        | DecodedControlPdu::FeatureExtendedResponse(value) => format!(
            "{} opcode=0x{:02x} maximum_page={} page={} features={}",
            control.opcode_name(),
            control.opcode,
            value.maximum_page,
            value.page_number,
            print_hex(&value.feature_page)
        ),
        DecodedControlPdu::CsSecurityRequest(value)
        | DecodedControlPdu::CsSecurityResponse(value) => format!(
            "{} opcode=0x{:02x} iv={} nonce={} personalization_vector={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&value.initialization_vector),
            print_hex(&value.nonce),
            print_hex(&value.personalization_vector)
        ),
        DecodedControlPdu::CsCapabilitiesRequest(value)
        | DecodedControlPdu::CsCapabilitiesResponse(value) => format!(
            "{} opcode=0x{:02x} mode_types=0x{:02x} rtt_capability=0x{:02x} rtt_aa_only_n={} rtt_sounding_n={} rtt_random_sequence_n={} nadm_sounding=0x{:04x} nadm_random=0x{:04x} cs_sync_phys=0x{:02x} antennas={} maximum_antenna_paths={} roles=0x{:02x} no_fae={} channel_selection_3c={} sounding_pct_estimate={} configurations={} maximum_procedures={} t_sw_us={} t_ip1_capability=0x{:04x} t_ip2_capability=0x{:04x} t_fcs_capability=0x{:04x} t_pm_capability=0x{:04x} tx_snr_capability=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.mode_types,
            value.rtt_capability,
            value.rtt_aa_only_n,
            value.rtt_sounding_n,
            value.rtt_random_sequence_n,
            value.nadm_sounding_capability,
            value.nadm_random_capability,
            value.cs_sync_phy_capability,
            value.antenna_count,
            value.maximum_antenna_paths,
            value.roles,
            value.no_fae,
            value.channel_selection_3c,
            value.sounding_pct_estimate,
            value.configuration_count,
            value.maximum_procedures_supported,
            value.antenna_switch_time_us,
            value.t_ip1_capability,
            value.t_ip2_capability,
            value.t_fcs_capability,
            value.t_pm_capability,
            value.tx_snr_capability
        ),
        DecodedControlPdu::CsConfigRequest(value) => format!(
            "{} opcode=0x{:02x} config={} action={} channel_map={} channels={} channel_map_repetition={} main_mode={} sub_mode={} main_mode_min_steps={} main_mode_max_steps={} main_mode_repetition={} mode_0_steps={} cs_sync_phy=0x{:02x} rtt_type={} role={} channel_selection={} channel_selection_3c_shape={} channel_selection_3c_jump={} t_ip1={} t_ip2={} t_fcs={} t_pm={}",
            control.opcode_name(),
            control.opcode,
            value.config_id,
            match value.action {
                CsConfigAction::Remove => "remove",
                CsConfigAction::Create => "create",
            },
            print_hex(&value.channel_map.bytes()),
            value.channel_map.used_count(),
            value.channel_map_repetition,
            value.main_mode,
            value.sub_mode,
            value.main_mode_min_steps,
            value.main_mode_max_steps,
            value.main_mode_repetition,
            value.mode_0_steps,
            value.cs_sync_phy,
            value.rtt_type,
            value.role,
            value.channel_selection,
            value.channel_selection_3c_shape,
            value.channel_selection_3c_jump,
            value.t_ip1,
            value.t_ip2,
            value.t_fcs,
            value.t_pm
        ),
        DecodedControlPdu::CsConfigResponse(value) => format!(
            "{} opcode=0x{:02x} config={}",
            control.opcode_name(),
            control.opcode,
            value.config_id
        ),
        DecodedControlPdu::CsProcedureRequest(value) => format!(
            "{} opcode=0x{:02x} config={} connection_event={} offset_min_us={} offset_max_us={} maximum_procedure_length_units={} maximum_procedure_length_us={} event_interval={} subevents_per_event={} subevent_interval_units={} subevent_interval_us={} subevent_length_us={} procedure_interval={} procedure_count={} aci={} preferred_peer_antennas=0x{:02x} phy=0x{:02x} power_delta_db={} initiator_snr_index={} reflector_snr_index={}",
            control.opcode_name(),
            control.opcode,
            value.config_id,
            value.connection_event_count,
            value.offset_min_us,
            value.offset_max_us,
            value.maximum_procedure_length_units,
            value.maximum_procedure_length_us(),
            value.event_interval_connection_events,
            value.subevents_per_event,
            value.subevent_interval_units,
            value.subevent_interval_us(),
            value.subevent_length_us,
            value.procedure_interval_connection_events,
            value.procedure_count,
            value.antenna_configuration_index,
            value.preferred_peer_antennas,
            value.phy,
            value.power_delta_db,
            value.initiator_snr_index,
            value.reflector_snr_index
        ),
        DecodedControlPdu::CsProcedureResponse(value) => format!(
            "{} opcode=0x{:02x} config={} connection_event={} offset_min_us={} offset_max_us={} event_interval={} subevents_per_event={} subevent_interval_units={} subevent_interval_us={} subevent_length_us={} aci={} phy=0x{:02x} power_delta_db={}",
            control.opcode_name(),
            control.opcode,
            value.config_id,
            value.connection_event_count,
            value.offset_min_us,
            value.offset_max_us,
            value.event_interval_connection_events,
            value.subevents_per_event,
            value.subevent_interval_units,
            value.subevent_interval_us(),
            value.subevent_length_us,
            value.antenna_configuration_index,
            value.phy,
            value.power_delta_db
        ),
        DecodedControlPdu::CsProcedureIndication(value) => format!(
            "{} opcode=0x{:02x} config={} connection_event={} offset_us={} event_interval={} subevents_per_event={} subevent_interval_units={} subevent_interval_us={} subevent_length_us={} aci={} phy=0x{:02x} power_delta_db={}",
            control.opcode_name(),
            control.opcode,
            value.config_id,
            value.connection_event_count,
            value.offset_us,
            value.event_interval_connection_events,
            value.subevents_per_event,
            value.subevent_interval_units,
            value.subevent_interval_us(),
            value.subevent_length_us,
            value.antenna_configuration_index,
            value.phy,
            value.power_delta_db
        ),
        DecodedControlPdu::CsTerminateRequest(value)
        | DecodedControlPdu::CsTerminateResponse(value) => format!(
            "{} opcode=0x{:02x} config={} procedure_count={} error_code=0x{:02x}",
            control.opcode_name(),
            control.opcode,
            value.config_id,
            value.procedure_count,
            value.error_code
        ),
        DecodedControlPdu::CsFaeResponse(value) => format!(
            "{} opcode=0x{:02x} fae={}",
            control.opcode_name(),
            control.opcode,
            print_signed_hex(&value.values)
        ),
        DecodedControlPdu::CsChannelMapInd(value) => format!(
            "{} opcode=0x{:02x} channel_map={} channels={} instant={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&value.channel_map.bytes()),
            value.channel_map.used_count(),
            value.instant
        ),
        DecodedControlPdu::FrameSpaceRequest(value) => format!(
            "{} opcode=0x{:02x} minimum_us={} maximum_us={} phys=0x{:02x} spacing_types=0x{:04x}",
            control.opcode_name(),
            control.opcode,
            value.minimum_us,
            value.maximum_us,
            value.phys,
            value.spacing_types
        ),
        DecodedControlPdu::FrameSpaceResponse(value) => format!(
            "{} opcode=0x{:02x} frame_space_us={} phys=0x{:02x} spacing_types=0x{:04x}",
            control.opcode_name(),
            control.opcode,
            value.frame_space_us,
            value.phys,
            value.spacing_types
        ),
        DecodedControlPdu::Raw { parameters, .. } => format!(
            "{} opcode=0x{:02x} raw_parameters={}",
            control.opcode_name(),
            control.opcode,
            print_hex(parameters)
        ),
    })
}

fn describe_data_pdu(packet: &DataChannelPdu) -> Result<String> {
    match packet.llid() {
        LogicalLinkId::StartOrComplete => {
            let start = packet
                .l2cap_start()?
                .expect("LLID checked before L2CAP start decode");
            Ok(format!(
                "L2CAP length={} cid=0x{:04x} fragment_octets={}",
                start.payload_length,
                start.channel_id,
                start.fragment.len()
            ))
        }
        LogicalLinkId::Control => describe_control_pdu(
            packet
                .control()?
                .expect("LLID checked before LL control decode"),
        ),
        LogicalLinkId::ContinuationOrEmpty if packet.payload.is_empty() => Ok("empty".to_owned()),
        LogicalLinkId::ContinuationOrEmpty => Ok(format!(
            "L2CAP continuation_octets={}",
            packet.payload.len()
        )),
        LogicalLinkId::Reserved => Ok(format!(
            "reserved_llid payload_octets={}",
            packet.payload.len()
        )),
    }
}

fn print_data_packet(
    packet: &ReceivedLePdu,
    data: &DataChannelPdu,
    payload_is_plaintext: bool,
) -> Result<()> {
    let cte = data
        .constant_tone_extension_info()
        .map(|info| {
            format!(
                "0x{:02x}:{}:{}us:rfu={}:reserved={}",
                info.raw(),
                info.cte_type_name(),
                info.duration_us(),
                info.rfu(),
                info.has_reserved_value()
            )
        })
        .unwrap_or_else(|| "none".to_owned());
    let description = payload_is_plaintext.then(|| describe_data_pdu(data));
    let plaintext_hint = match &description {
        Some(Ok(description)) => description.clone(),
        Some(Err(error)) => format!("decode_error={error}"),
        None => "encrypted".to_owned(),
    };
    println!(
        "channel={} phy={} sample={} phase={} access_address={:08x} inverted={} aa_errors={} llid={} nesn={} sn={} md={} cp={} cte={} rfu={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} plaintext_hint=\"{}\"",
        data.channel.index(),
        packet.phy,
        packet.access_address_sample,
        packet.symbol_phase,
        data.access_address,
        data.inverted,
        data.access_address_errors,
        data.llid(),
        data.next_expected_sequence_number(),
        data.sequence_number(),
        data.more_data(),
        data.constant_tone_extension_present(),
        cte,
        data.reserved_header_bits(),
        packet.estimated_carrier_offset_hz,
        packet.estimated_deviation_hz,
        print_hex(&data.header),
        print_hex(&data.payload),
        print_hex(&data.crc),
        plaintext_hint.replace('"', "'"),
    );
    description.transpose().map(|_| ())
}

fn print_decrypted_data_packet(
    direction: LinkDirection,
    decryption: &LeAclDecryption,
) -> Result<()> {
    let description = describe_data_pdu(&decryption.packet);
    let plaintext_hint = match &description {
        Ok(description) => description.clone(),
        Err(error) => format!("decode_error={error}"),
    };
    let (status, packet_counter) = match decryption.status {
        LeAclDecryptionStatus::New { packet_counter, .. } => ("new", packet_counter.to_string()),
        LeAclDecryptionStatus::Retransmission { packet_counter } => {
            ("retransmission", packet_counter.to_string())
        }
        LeAclDecryptionStatus::UnencryptedEmpty => ("unencrypted-empty", "none".to_owned()),
    };
    println!(
        "decrypted_data direction={} status={} packet_counter={} skipped_counters={} header={} payload={} plaintext_hint=\"{}\"",
        direction,
        status,
        packet_counter,
        decryption.status.skipped_counters(),
        print_hex(&decryption.packet.header),
        print_hex(&decryption.packet.payload),
        plaintext_hint.replace('"', "'"),
    );
    description.map(|_| ())
}

fn describe_incomplete_l2cap(incomplete: IncompleteL2capPdu) -> String {
    format!(
        "direction={} cid=0x{:04x} received={} expected={} fragments={}",
        incomplete.direction,
        incomplete.channel_id,
        incomplete.received_payload_length,
        incomplete.expected_payload_length,
        incomplete.fragment_count
    )
}

fn print_l2cap_channel_ids(channel_ids: &[u16]) -> String {
    channel_ids
        .iter()
        .map(|channel_id| format!("0x{channel_id:04x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe_l2cap_signaling(command: L2capSignalingCommand<'_>) -> Result<String> {
    let prefix = format!(
        "code=0x{:02x} name={} identifier={}",
        command.code,
        command.code_name(),
        command.identifier
    );
    let details = match command.decode()? {
        DecodedL2capSignalingCommand::CommandReject(reject) => {
            format!(
                "reason=0x{:04x} data={}",
                reject.reason,
                print_hex(reject.data)
            )
        }
        DecodedL2capSignalingCommand::DisconnectionRequest(disconnection)
        | DecodedL2capSignalingCommand::DisconnectionResponse(disconnection) => format!(
            "destination_cid=0x{:04x} source_cid=0x{:04x}",
            disconnection.destination_channel_id, disconnection.source_channel_id
        ),
        DecodedL2capSignalingCommand::ConnectionParameterUpdateRequest(request) => format!(
            "minimum_interval={} maximum_interval={} latency={} supervision_timeout={}",
            request.minimum_interval,
            request.maximum_interval,
            request.latency,
            request.supervision_timeout
        ),
        DecodedL2capSignalingCommand::ConnectionParameterUpdateResponse(response) => {
            format!("result=0x{:04x}", response.result)
        }
        DecodedL2capSignalingCommand::LeCreditBasedConnectionRequest(request) => format!(
            "spsm=0x{:04x} source_cid=0x{:04x} mtu={} mps={} initial_credits={}",
            request.spsm,
            request.source_channel_id,
            request.mtu,
            request.mps,
            request.initial_credits
        ),
        DecodedL2capSignalingCommand::LeCreditBasedConnectionResponse(response) => format!(
            "destination_cid=0x{:04x} mtu={} mps={} initial_credits={} result=0x{:04x}",
            response.destination_channel_id,
            response.mtu,
            response.mps,
            response.initial_credits,
            response.result
        ),
        DecodedL2capSignalingCommand::FlowControlCredit(credit) => {
            format!("cid=0x{:04x} credits={}", credit.channel_id, credit.credits)
        }
        DecodedL2capSignalingCommand::EnhancedCreditBasedConnectionRequest(request) => format!(
            "spsm=0x{:04x} mtu={} mps={} initial_credits={} source_cids={}",
            request.spsm,
            request.mtu,
            request.mps,
            request.initial_credits,
            print_l2cap_channel_ids(request.source_channel_ids.as_slice())
        ),
        DecodedL2capSignalingCommand::EnhancedCreditBasedConnectionResponse(response) => format!(
            "mtu={} mps={} initial_credits={} result=0x{:04x} destination_cids={}",
            response.mtu,
            response.mps,
            response.initial_credits,
            response.result,
            print_l2cap_channel_ids(response.destination_channel_ids.as_slice())
        ),
        DecodedL2capSignalingCommand::EnhancedCreditBasedReconfigureRequest(request) => format!(
            "mtu={} mps={} cids={}",
            request.mtu,
            request.mps,
            print_l2cap_channel_ids(request.channel_ids.as_slice())
        ),
        DecodedL2capSignalingCommand::EnhancedCreditBasedReconfigureResponse(response) => {
            format!("result=0x{:04x}", response.result)
        }
        DecodedL2capSignalingCommand::Unknown { parameters, .. } => {
            format!("parameters={}", print_hex(parameters))
        }
    };
    Ok(format!("{prefix} {details}"))
}

fn describe_att_uuid(uuid: AttUuid) -> String {
    match uuid {
        AttUuid::Uuid16(uuid) => format!("0x{uuid:04x}"),
        AttUuid::Uuid128(uuid) => print_hex(&uuid),
    }
}

fn describe_att_pdu(pdu: AttPdu<'_>) -> Result<String> {
    let prefix = format!(
        "opcode=0x{:02x} name={} type={}",
        pdu.opcode,
        pdu.opcode_name(),
        pdu.pdu_type()
    );
    let details = match pdu.decode()? {
        DecodedAttPdu::ErrorResponse(response) => format!(
            "request_opcode=0x{:02x} handle=0x{:04x} error=0x{:02x} error_name={}",
            response.request_opcode,
            response.handle,
            response.error_code,
            response.error_name()
        ),
        DecodedAttPdu::ExchangeMtuRequest(exchange)
        | DecodedAttPdu::ExchangeMtuResponse(exchange) => {
            format!("mtu={}", exchange.mtu)
        }
        DecodedAttPdu::FindInformationRequest(range) => format!(
            "start_handle=0x{:04x} end_handle=0x{:04x}",
            range.start_handle, range.end_handle
        ),
        DecodedAttPdu::FindInformationResponse(response) => {
            let entries = response
                .entries
                .iter()
                .map(|entry| format!("0x{:04x}:{}", entry.handle, describe_att_uuid(entry.uuid)))
                .collect::<Vec<_>>()
                .join(",");
            format!("uuid_width={} entries={entries}", response.uuid_width())
        }
        DecodedAttPdu::FindByTypeValueRequest(request) => format!(
            "start_handle=0x{:04x} end_handle=0x{:04x} attribute_type=0x{:04x} value={}",
            request.range.start_handle,
            request.range.end_handle,
            request.attribute_type,
            print_hex(request.value)
        ),
        DecodedAttPdu::FindByTypeValueResponse(ranges) => {
            let ranges = ranges
                .iter()
                .map(|range| format!("0x{:04x}-0x{:04x}", range.start_handle, range.end_handle))
                .collect::<Vec<_>>()
                .join(",");
            format!("ranges={ranges}")
        }
        DecodedAttPdu::ReadByTypeRequest(request)
        | DecodedAttPdu::ReadByGroupTypeRequest(request) => format!(
            "start_handle=0x{:04x} end_handle=0x{:04x} attribute_type={}",
            request.range.start_handle,
            request.range.end_handle,
            describe_att_uuid(request.attribute_type)
        ),
        DecodedAttPdu::ReadByTypeResponse(response) => {
            let entries = response
                .entries
                .iter()
                .map(|entry| format!("0x{:04x}:{}", entry.handle, print_hex(entry.value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("entry_length={} entries={entries}", response.entry_length)
        }
        DecodedAttPdu::ReadRequest(request) => {
            format!("handle=0x{:04x}", request.handle)
        }
        DecodedAttPdu::ReadResponse(value)
        | DecodedAttPdu::ReadBlobResponse(value)
        | DecodedAttPdu::ReadMultipleResponse(value) => {
            format!("value={}", print_hex(value))
        }
        DecodedAttPdu::ReadBlobRequest(request) => {
            format!("handle=0x{:04x} offset={}", request.handle, request.offset)
        }
        DecodedAttPdu::ReadMultipleRequest(handles)
        | DecodedAttPdu::ReadMultipleVariableRequest(handles) => {
            let handles = handles
                .iter()
                .map(|handle| format!("0x{handle:04x}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("handles={handles}")
        }
        DecodedAttPdu::ReadByGroupTypeResponse(response) => {
            let entries = response
                .entries
                .iter()
                .map(|entry| {
                    format!(
                        "0x{:04x}-0x{:04x}:{}",
                        entry.range.start_handle,
                        entry.range.end_handle,
                        print_hex(entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("entry_length={} entries={entries}", response.entry_length)
        }
        DecodedAttPdu::WriteRequest(write)
        | DecodedAttPdu::HandleValueNotification(write)
        | DecodedAttPdu::HandleValueIndication(write)
        | DecodedAttPdu::WriteCommand(write) => {
            format!(
                "handle=0x{:04x} value={}",
                write.handle,
                print_hex(write.value)
            )
        }
        DecodedAttPdu::WriteResponse
        | DecodedAttPdu::ExecuteWriteResponse
        | DecodedAttPdu::HandleValueConfirmation => "parameters=none".to_owned(),
        DecodedAttPdu::PrepareWriteRequest(write) | DecodedAttPdu::PrepareWriteResponse(write) => {
            format!(
                "handle=0x{:04x} offset={} value={}",
                write.handle,
                write.offset,
                print_hex(write.value)
            )
        }
        DecodedAttPdu::ExecuteWriteRequest(flags) => format!("flags={flags}"),
        DecodedAttPdu::ReadMultipleVariableResponse(list) => {
            let values = list
                .values
                .iter()
                .map(|value| {
                    format!(
                        "{}:{}:{}",
                        value.declared_length,
                        print_hex(value.value),
                        value.truncated
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("values={values}")
        }
        DecodedAttPdu::MultipleHandleValueNotification(values) => {
            let values = values
                .iter()
                .map(|value| format!("0x{:04x}:{}", value.handle, print_hex(value.value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("values={values}")
        }
        DecodedAttPdu::SignedWriteCommand(write) => format!(
            "handle=0x{:04x} value={} signature={}",
            write.handle,
            print_hex(write.value),
            print_hex(&write.signature)
        ),
        DecodedAttPdu::Unknown { parameters, .. } => {
            format!("parameters={}", print_hex(parameters))
        }
    };
    Ok(format!("{prefix} {details}"))
}

fn describe_smp_authentication(authentication: SmpAuthenticationRequirements) -> String {
    format!(
        "auth=0x{:02x} bonding={} mitm={} secure_connections={} keypress={} ct2={}",
        authentication.raw,
        authentication.bonding(),
        authentication.mitm(),
        authentication.secure_connections(),
        authentication.keypress(),
        authentication.ct2()
    )
}

fn describe_smp_key_distribution(distribution: SmpKeyDistribution) -> String {
    format!(
        "0x{:02x}:enc={}:id={}:sign={}:link={}",
        distribution.raw,
        distribution.encryption_key(),
        distribution.identity_key(),
        distribution.signing_key(),
        distribution.link_key()
    )
}

fn print_device_address(address: [u8; 6]) -> String {
    address
        .iter()
        .rev()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn describe_smp_pdu(pdu: SmpPdu<'_>) -> Result<String> {
    let prefix = format!("code=0x{:02x} name={}", pdu.code, pdu.code_name());
    let details = match pdu.decode()? {
        DecodedSmpPdu::PairingRequest(features) | DecodedSmpPdu::PairingResponse(features) => {
            format!(
                "io_capability={} oob={} {} maximum_key_size={} initiator_keys={} responder_keys={}",
                features.io_capability,
                features.oob_data_present,
                describe_smp_authentication(features.authentication),
                features.maximum_encryption_key_size,
                describe_smp_key_distribution(features.initiator_key_distribution),
                describe_smp_key_distribution(features.responder_key_distribution)
            )
        }
        DecodedSmpPdu::PairingConfirm(value) => {
            format!("confirm={}", print_hex(&value))
        }
        DecodedSmpPdu::PairingRandom(value) => {
            format!("random={}", print_hex(&value))
        }
        DecodedSmpPdu::PairingFailed(failure) => format!(
            "reason=0x{:02x} reason_name={}",
            failure.reason,
            failure.reason_name()
        ),
        DecodedSmpPdu::EncryptionInformation(key) => {
            format!("ltk={}", print_hex(&key))
        }
        DecodedSmpPdu::CentralIdentification(identification) => format!(
            "ediv=0x{:04x} random={}",
            identification.encrypted_diversifier,
            print_hex(&identification.random)
        ),
        DecodedSmpPdu::IdentityInformation(key) => {
            format!("irk={}", print_hex(&key))
        }
        DecodedSmpPdu::IdentityAddressInformation(identity) => format!(
            "address_type={} address={}",
            identity.address_type,
            print_device_address(identity.address)
        ),
        DecodedSmpPdu::SigningInformation(key) => {
            format!("csrk={}", print_hex(&key))
        }
        DecodedSmpPdu::SecurityRequest(authentication) => {
            describe_smp_authentication(authentication)
        }
        DecodedSmpPdu::PairingPublicKey(key) => {
            format!("x={} y={}", print_hex(&key.x), print_hex(&key.y))
        }
        DecodedSmpPdu::PairingDhKeyCheck(value) => {
            format!("dhkey_check={}", print_hex(&value))
        }
        DecodedSmpPdu::KeypressNotification(notification_type) => {
            format!("notification_type={notification_type}")
        }
        DecodedSmpPdu::Unknown { parameters, .. } => {
            format!("parameters={}", print_hex(parameters))
        }
    };
    Ok(format!("{prefix} {details}"))
}

fn decode(args: DecodeArgs) -> Result<()> {
    if args.block_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be greater than zero".to_owned(),
        ));
    }
    let (mut reader, sample_count) = open_iq_file(&args.input, args.format)?;
    if sample_count > args.max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {}",
            args.max_samples
        )));
    }

    let config = Le1mDemodConfig {
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let mut decoder = Le1mStreamDecoder::new(args.channel, config)?;
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };
    let mut packet_count = 0usize;

    loop {
        let first_sample = reader.next_sample_index();
        let samples = reader.read_block(args.block_samples)?;
        if samples.is_empty() {
            break;
        }
        let batch = decoder.push(first_sample, &samples)?;
        if let Some(discontinuity) = batch.discontinuity {
            eprintln!(
                "sample discontinuity: expected {}, observed {}",
                discontinuity.expected_first_sample, discontinuity.observed_first_sample
            );
        }
        for packet in &batch.packets {
            print_packet(packet);
            if let Some(writer) = &mut pcap {
                let timestamp = sample_timestamp_ns(
                    args.capture_start_ns,
                    packet.access_address_sample,
                    args.sample_rate_hz,
                )?;
                writer.write_advertising(packet, timestamp)?;
            }
        }
        packet_count += batch.packets.len();
    }

    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    eprintln!("decoded {packet_count} CRC-valid packet(s) from {sample_count} sample(s)");
    Ok(())
}

fn decode_data(args: DecodeDataArgs) -> Result<()> {
    if args.block_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be greater than zero".to_owned(),
        ));
    }
    let (mut reader, sample_count) = open_iq_file(&args.input, args.format)?;
    if sample_count > args.max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {}",
            args.max_samples
        )));
    }

    let demod_config = LeUncodedDemodConfig {
        phy: args.phy,
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let frame_config = LeFrameConfig::data(args.access_address, args.crc_init)?;
    let mut decoder = LeUncodedPacketStreamDecoder::new(args.channel, frame_config, demod_config)?;
    let mut decryptor = match &args.decryption {
        Some(decryption) => Some(LeAclDecryptor::new(
            decryption.session_key,
            decryption.initialization_vector,
            decryption.direction,
            decryption.initial_packet_counter,
            decryption.maximum_counter_skip,
        )?),
        None => None,
    };
    let mut l2cap_reassembler = match args.plaintext_l2cap_direction {
        Some(direction) => Some((
            direction,
            L2capReassembler::new(args.maximum_l2cap_payload_length)?,
        )),
        None => None,
    };
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };
    let mut packet_count = 0usize;
    let mut l2cap_pdu_count = 0usize;
    let mut l2cap_duplicate_count = 0usize;
    let mut l2cap_orphan_count = 0usize;
    let mut l2cap_discarded_count = 0usize;
    let mut l2cap_error_count = 0usize;
    let mut l2cap_signaling_error_count = 0usize;
    let mut att_error_count = 0usize;
    let mut smp_error_count = 0usize;
    let mut ll_control_error_count = 0usize;
    let mut decrypted_packet_count = 0usize;
    let mut decryption_retransmission_count = 0usize;
    let mut unencrypted_empty_count = 0usize;
    let mut decryption_error_count = 0usize;
    let mut skipped_packet_counter_count = 0u64;

    loop {
        let first_sample = reader.next_sample_index();
        let samples = reader.read_block(args.block_samples)?;
        if samples.is_empty() {
            break;
        }
        let batch = decoder.push(first_sample, &samples)?;
        if let Some(discontinuity) = batch.discontinuity {
            eprintln!(
                "sample discontinuity: expected {}, observed {}",
                discontinuity.expected_first_sample, discontinuity.observed_first_sample
            );
            if let Some((direction, reassembler)) = &mut l2cap_reassembler
                && let Some(incomplete) = reassembler.reset(*direction)
            {
                l2cap_discarded_count += 1;
                eprintln!(
                    "discarded incomplete plaintext L2CAP PDU after sample discontinuity: {}",
                    describe_incomplete_l2cap(incomplete)
                );
            }
        }
        for packet in &batch.packets {
            let raw_data = DataChannelPdu::from(packet.pdu.clone());
            let data = if let Some(decryptor) = &mut decryptor {
                print_data_packet(packet, &raw_data, false)?;
                match decryptor.decrypt(&raw_data) {
                    Ok(decryption) => {
                        match decryption.status {
                            LeAclDecryptionStatus::New {
                                skipped_counters, ..
                            } => {
                                decrypted_packet_count += 1;
                                skipped_packet_counter_count = skipped_packet_counter_count
                                    .checked_add(skipped_counters)
                                    .ok_or_else(|| {
                                        Error::InvalidState(
                                            "skipped packet-counter total overflow".to_owned(),
                                        )
                                    })?;
                                if skipped_counters != 0
                                    && let Some((direction, reassembler)) = &mut l2cap_reassembler
                                    && let Some(incomplete) = reassembler.reset(*direction)
                                {
                                    l2cap_discarded_count += 1;
                                    eprintln!(
                                        "discarded incomplete plaintext L2CAP PDU after {skipped_counters} skipped encrypted packet counter(s): {}",
                                        describe_incomplete_l2cap(incomplete)
                                    );
                                }
                            }
                            LeAclDecryptionStatus::Retransmission { .. } => {
                                decryption_retransmission_count += 1;
                            }
                            LeAclDecryptionStatus::UnencryptedEmpty => {
                                unencrypted_empty_count += 1;
                            }
                        }
                        if let Err(error) =
                            print_decrypted_data_packet(decryptor.direction(), &decryption)
                            && decryption.packet.llid() == LogicalLinkId::Control
                        {
                            ll_control_error_count += 1;
                            eprintln!(
                                "decrypted LL control PDU decode error: opcode={} error={error}",
                                decryption
                                    .packet
                                    .payload
                                    .first()
                                    .map(|opcode| format!("0x{opcode:02x}"))
                                    .unwrap_or_else(|| "missing".to_owned())
                            );
                        }
                        Some(decryption.packet)
                    }
                    Err(error) => {
                        decryption_error_count += 1;
                        eprintln!(
                            "LE ACL decryption error: direction={} error={error}",
                            decryptor.direction()
                        );
                        if let Some((direction, reassembler)) = &mut l2cap_reassembler
                            && let Some(incomplete) = reassembler.reset(*direction)
                        {
                            l2cap_discarded_count += 1;
                            eprintln!(
                                "discarded incomplete plaintext L2CAP PDU after decryption failure: {}",
                                describe_incomplete_l2cap(incomplete)
                            );
                        }
                        None
                    }
                }
            } else {
                if let Err(error) = print_data_packet(packet, &raw_data, true)
                    && raw_data.llid() == LogicalLinkId::Control
                {
                    ll_control_error_count += 1;
                    eprintln!(
                        "LL control PDU decode error: opcode={} error={error}",
                        raw_data
                            .payload
                            .first()
                            .map(|opcode| format!("0x{opcode:02x}"))
                            .unwrap_or_else(|| "missing".to_owned())
                    );
                }
                Some(raw_data.clone())
            };
            if let (Some((direction, reassembler)), Some(data)) =
                (&mut l2cap_reassembler, data.as_ref())
            {
                match reassembler.push(*direction, data) {
                    Ok(update) => {
                        if let Some(replaced) = update.replaced {
                            l2cap_discarded_count += 1;
                            eprintln!(
                                "replaced incomplete plaintext L2CAP PDU: {}",
                                describe_incomplete_l2cap(replaced)
                            );
                        }
                        match update.outcome {
                            L2capReassemblyOutcome::Ignored
                            | L2capReassemblyOutcome::InProgress(_) => {}
                            L2capReassemblyOutcome::Duplicate => {
                                l2cap_duplicate_count += 1;
                            }
                            L2capReassemblyOutcome::OrphanedContinuation { fragment_octets } => {
                                l2cap_orphan_count += 1;
                                eprintln!(
                                    "orphaned plaintext L2CAP continuation: direction={} fragment_octets={fragment_octets}",
                                    direction
                                );
                            }
                            L2capReassemblyOutcome::Complete(pdu) => {
                                l2cap_pdu_count += 1;
                                println!(
                                    "l2cap_pdu direction={} cid=0x{:04x} length={} fragments={} payload={}",
                                    pdu.direction,
                                    pdu.channel_id,
                                    pdu.payload.len(),
                                    pdu.fragment_count,
                                    print_hex(&pdu.payload)
                                );
                                match pdu.le_signaling_command() {
                                    Ok(Some(command)) => match describe_l2cap_signaling(command) {
                                        Ok(description) => println!(
                                            "l2cap_signal direction={} {description}",
                                            pdu.direction
                                        ),
                                        Err(error) => {
                                            l2cap_signaling_error_count += 1;
                                            eprintln!(
                                                "plaintext L2CAP signaling command decode error: direction={} error={error}",
                                                pdu.direction
                                            );
                                        }
                                    },
                                    Ok(None) => {}
                                    Err(error) => {
                                        l2cap_signaling_error_count += 1;
                                        eprintln!(
                                            "plaintext L2CAP signaling envelope decode error: direction={} error={error}",
                                            pdu.direction
                                        );
                                    }
                                }
                                match pdu.att_pdu() {
                                    Ok(Some(att)) => match describe_att_pdu(att) {
                                        Ok(description) => println!(
                                            "att_pdu direction={} {description}",
                                            pdu.direction
                                        ),
                                        Err(error) => {
                                            att_error_count += 1;
                                            eprintln!(
                                                "plaintext ATT PDU decode error: direction={} error={error}",
                                                pdu.direction
                                            );
                                        }
                                    },
                                    Ok(None) => {}
                                    Err(error) => {
                                        att_error_count += 1;
                                        eprintln!(
                                            "plaintext ATT PDU envelope decode error: direction={} error={error}",
                                            pdu.direction
                                        );
                                    }
                                }
                                match pdu.smp_pdu() {
                                    Ok(Some(smp)) => match describe_smp_pdu(smp) {
                                        Ok(description) => println!(
                                            "smp_pdu direction={} {description}",
                                            pdu.direction
                                        ),
                                        Err(error) => {
                                            smp_error_count += 1;
                                            eprintln!(
                                                "plaintext SMP PDU decode error: direction={} error={error}",
                                                pdu.direction
                                            );
                                        }
                                    },
                                    Ok(None) => {}
                                    Err(error) => {
                                        smp_error_count += 1;
                                        eprintln!(
                                            "plaintext SMP PDU envelope decode error: direction={} error={error}",
                                            pdu.direction
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        l2cap_error_count += 1;
                        eprintln!(
                            "plaintext L2CAP reassembly error: direction={} error={error}",
                            direction
                        );
                    }
                }
            }
            if let Some(writer) = &mut pcap {
                let timestamp = sample_timestamp_ns(
                    args.capture_start_ns,
                    packet.access_address_sample,
                    args.sample_rate_hz,
                )?;
                writer.write_le(packet, timestamp)?;
            }
        }
        packet_count += batch.packets.len();
    }

    if let Some((direction, reassembler)) = &mut l2cap_reassembler
        && let Some(incomplete) = reassembler.reset(*direction)
    {
        l2cap_discarded_count += 1;
        eprintln!(
            "discarded incomplete plaintext L2CAP PDU at end of input: {}",
            describe_incomplete_l2cap(incomplete)
        );
    }

    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    eprintln!(
        "decoded {packet_count} CRC-valid data-channel packet(s) from {sample_count} sample(s); ll_control_errors={ll_control_error_count}"
    );
    if decryptor.is_some() {
        eprintln!(
            "authenticated {decrypted_packet_count} new encrypted packet(s); retransmissions={decryption_retransmission_count} unencrypted_empty={unencrypted_empty_count} skipped_counters={skipped_packet_counter_count} errors={decryption_error_count}"
        );
    }
    if l2cap_reassembler.is_some() {
        eprintln!(
            "reassembled {l2cap_pdu_count} plaintext L2CAP PDU(s); duplicates={l2cap_duplicate_count} orphan_continuations={l2cap_orphan_count} discarded_incomplete={l2cap_discarded_count} errors={l2cap_error_count} signaling_errors={l2cap_signaling_error_count} att_errors={att_error_count} smp_errors={smp_error_count}"
        );
    }
    Ok(())
}

fn current_unix_time_ns() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            Error::InvalidInput(format!("system clock precedes Unix epoch: {error}"))
        })?;
    u64::try_from(duration.as_nanos())
        .map_err(|_| Error::InvalidInput("current Unix timestamp exceeds u64".to_owned()))
}

fn capture(args: CaptureArgs) -> Result<()> {
    let device = args.device.to_ascii_lowercase();
    if !matches!(device.as_str(), "bladerf" | "limesdr" | "lime" | "xtrx") {
        return Err(Error::InvalidConfiguration(format!(
            "capture device {:?} is not implemented; currently available: bladerf, limesdr, xtrx",
            args.device
        )));
    }
    if !args.channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "capture currently requires BLE advertising channel 37, 38, or 39; got {}",
            args.channel.index()
        )));
    }
    let demod_config = Le1mDemodConfig {
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    demod_config.validate()?;
    let radio_config = SdrConfig {
        center_frequency_hz: args.channel.center_frequency_hz(),
        sample_rate_hz: args.sample_rate_hz,
        bandwidth_hz: args.bandwidth_hz,
        gain_db: args.gain_db,
        channel: args.rx_channel,
    };

    let stats = match device.as_str() {
        "bladerf" => {
            let mut source =
                BladeRfSource::open(args.identifier.as_deref(), BladeRfOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "bladeRF applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        "limesdr" | "lime" => {
            let mut source =
                LimeSdrSource::open(args.identifier.as_deref(), LimeSdrOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "LimeSDR applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        "xtrx" => {
            let mut source = XtrxSource::open(args.identifier.as_deref(), XtrxOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "XTRX applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        _ => unreachable!(),
    };
    eprintln!(
        "capture complete: samples={} packets={} overruns={} dropped={} discontinuities={}",
        stats.samples_received,
        stats.packets_decoded,
        stats.overruns,
        stats.dropped_samples,
        stats.discontinuities
    );
    Ok(())
}

fn capture_from_source<S: IqSource>(
    source: &mut S,
    args: &CaptureArgs,
    radio_config: &SdrConfig,
    demod_config: Le1mDemodConfig,
) -> Result<CaptureStats> {
    let capture_start_ns = args
        .capture_start_ns
        .map(Ok)
        .unwrap_or_else(current_unix_time_ns)?;
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };

    let stats = capture_primary_advertising(
        source,
        radio_config,
        args.channel,
        demod_config,
        CaptureLimits {
            maximum_samples: None,
            maximum_duration: Some(args.duration),
            read_timeout: Duration::from_millis(args.read_timeout_ms),
            block_samples: args.block_samples,
        },
        |captured: &CapturedAdvertisingPdu| {
            print_packet(&captured.observation);
            if let Some(writer) = &mut pcap {
                let timestamp = sample_timestamp_ns(
                    capture_start_ns,
                    captured.relative_sample_index,
                    args.sample_rate_hz,
                )?;
                writer.write_advertising(&captured.observation, timestamp)?;
            }
            Ok(())
        },
    )?;
    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    Ok(stats)
}

fn backends() {
    match BladeRfSource::probe_library() {
        Ok(library) => println!("bladerf  library available: {library}"),
        Err(error) => println!("bladerf  unavailable: {error}"),
    }
    match LimeSdrSource::probe_library() {
        Ok(library) => println!("limesdr  library available: {library}"),
        Err(error) => println!("limesdr  unavailable: {error}"),
    }
    match XtrxSource::probe_library() {
        Ok(library) => println!("xtrx     library available: {library}"),
        Err(error) => println!("xtrx     unavailable: {error}"),
    }
}

fn connection_tracker(args: &ConnectionPlanArgs) -> Result<ConnectionTracker> {
    ConnectionTracker::new(
        ConnectionTrackerConfig {
            access_address: args.access_address,
            channel_selection_algorithm: args.channel_selection_algorithm,
            hop_increment: args.hop_increment,
            channel_map: args.channel_map.clone(),
            parameters: args.parameters,
            sample_rate_hz: args.sample_rate_hz,
        },
        args.anchor_event_counter,
        args.anchor_access_address_sample,
    )
}

fn connection_plan(args: ConnectionPlanArgs) -> Result<()> {
    if args.first_central_transmission.is_some() || !args.observations.is_empty() {
        return Err(Error::InvalidConfiguration(
            "observation options are not accepted by connection-plan".to_owned(),
        ));
    }
    let mut tracker = connection_tracker(&args)?;

    for index in 0..args.event_count {
        let event = if index == 0 {
            tracker.current_event()?
        } else {
            tracker.advance()?
        };
        let ConnectionEventTiming::Expected {
            access_address_sample,
        } = event.timing
        else {
            return Err(Error::InvalidState(
                "offline connection plan unexpectedly requires an anchor observation".to_owned(),
            ));
        };
        let timing_window = tracker
            .current_timing_window(args.peer_clock_accuracy, args.receiver_clock_accuracy_ppm)?
            .ok_or_else(|| {
                Error::InvalidState(
                    "offline connection plan unexpectedly lost its timing anchor".to_owned(),
                )
            })?;
        println!(
            "event={} channel={} frequency_hz={} expected_sample={} earliest_sample={} latest_sample={} widening_samples={}",
            event.event_counter,
            event.channel.index(),
            event.channel.center_frequency_hz(),
            access_address_sample,
            timing_window.earliest_sample,
            timing_window.latest_sample,
            timing_window.widening_samples
        );
    }
    Ok(())
}

fn connection_sync(args: ConnectionPlanArgs) -> Result<()> {
    if args.first_central_transmission.is_some() {
        return Err(Error::InvalidConfiguration(
            "--central-observe is accepted by connection-acquire, not connection-sync".to_owned(),
        ));
    }
    if args.observations.is_empty() {
        return Err(Error::InvalidConfiguration(
            "connection-sync requires at least one --observe CHANNEL:SAMPLE".to_owned(),
        ));
    }
    let mut tracker = connection_tracker(&args)?;
    for observed in args.observations {
        let observation = tracker.synchronize_observation(
            observed.channel,
            observed.access_address_sample,
            args.peer_clock_accuracy,
            args.receiver_clock_accuracy_ppm,
            args.maximum_event_advance,
        )?;
        print_synchronized_observation(observed, observation);
    }
    Ok(())
}

fn print_synchronized_observation(
    observed: ConnectionObservationArg,
    observation: blueoxide::link_layer::ConnectionObservation,
) {
    let missed_events = observation.advanced_events.saturating_sub(1);
    let timing = match observation.timing_error {
        SampleTimingError::Early(samples) => format!("early:{samples}"),
        SampleTimingError::OnTime => "on-time:0".to_owned(),
        SampleTimingError::Late(samples) => format!("late:{samples}"),
    };
    println!(
        "event={} channel={} observed_sample={} advanced_events={} missed_events={} expected_sample={} timing={} earliest_sample={} latest_sample={} widening_samples={}",
        observation.event.event_counter,
        observation.event.channel.index(),
        observed.access_address_sample,
        observation.advanced_events,
        missed_events,
        observation.timing_window.expected_sample,
        timing,
        observation.timing_window.earliest_sample,
        observation.timing_window.latest_sample,
        observation.timing_window.widening_samples
    );
}

fn connection_acquire(args: ConnectionPlanArgs) -> Result<()> {
    let connect_ind_access_address_sample =
        args.connect_ind_access_address_sample.ok_or_else(|| {
            Error::InvalidConfiguration("connection-acquire requires --connect-sample".to_owned())
        })?;
    let first_central_transmission = args.first_central_transmission.ok_or_else(|| {
        Error::InvalidConfiguration(
            "connection-acquire requires --central-observe CHANNEL:SAMPLE".to_owned(),
        )
    })?;
    let request = ConnectRequest {
        access_address: args.access_address,
        crc_init: 0,
        window_size: args.window_size,
        window_offset: args.window_offset,
        interval: args.parameters.interval,
        latency: args.parameters.latency,
        supervision_timeout: args.parameters.supervision_timeout,
        channel_map: args.channel_map.bytes(),
        hop_increment: args.hop_increment,
        sleep_clock_accuracy: args.peer_clock_accuracy.raw(),
        channel_selection_algorithm: args.channel_selection_algorithm,
    };
    let first_window = request.first_event_window(
        connect_ind_access_address_sample,
        args.sample_rate_hz,
        args.receiver_clock_accuracy_ppm,
    )?;
    let observed_central = FirstCentralTransmission::new(
        first_central_transmission.channel,
        first_central_transmission.access_address_sample,
    )?;
    let mut tracker = request.acquire_first_event_anchor(
        connect_ind_access_address_sample,
        args.sample_rate_hz,
        args.receiver_clock_accuracy_ppm,
        observed_central,
    )?;
    println!(
        "event=0 channel={} central_sample={} connect_ind_sample={} nominal_start_sample={} nominal_end_sample={} earliest_sample={} latest_sample={} widening_samples={}",
        first_window.channel.index(),
        first_central_transmission.access_address_sample,
        connect_ind_access_address_sample,
        first_window.nominal_start_sample,
        first_window.nominal_end_sample,
        first_window.earliest_sample,
        first_window.latest_sample,
        first_window.widening_samples
    );
    for observed in args.observations {
        let observation = tracker.synchronize_observation(
            observed.channel,
            observed.access_address_sample,
            args.peer_clock_accuracy,
            args.receiver_clock_accuracy_ppm,
            args.maximum_event_advance,
        )?;
        print_synchronized_observation(observed, observation);
    }
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("decode") => decode(parse_decode_args(&args[1..])?),
        Some("decode-data") => decode_data(parse_decode_data_args(&args[1..])?),
        Some("connection-plan") => connection_plan(parse_connection_plan_args(&args[1..])?),
        Some("connection-sync") => connection_sync(parse_connection_plan_args(&args[1..])?),
        Some("connection-acquire") => connection_acquire(parse_connection_plan_args(&args[1..])?),
        Some("capture") => capture(parse_capture_args(&args[1..])?),
        Some("backends") => {
            backends();
            Ok(())
        }
        Some("channels") => {
            for index in 0..=39 {
                let channel = BleChannel::new(index)?;
                println!(
                    "{:>2}  {} Hz{}",
                    index,
                    channel.center_frequency_hz(),
                    if channel.is_primary_advertising() {
                        "  primary advertising"
                    } else {
                        ""
                    }
                );
            }
            Ok(())
        }
        Some("-h" | "--help") | None => {
            print!("{}", usage());
            Ok(())
        }
        Some(command) => Err(Error::InvalidConfiguration(format!(
            "unknown command {command:?}\n\n{}",
            usage()
        ))),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("blueoxide: {error}");
        std::process::exit(2);
    }
}
