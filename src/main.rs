use blueoxide::advertising::{ConnectRequest, FirstCentralTransmission, decode_advertising_pdu};
use blueoxide::backends::bladerf::{BladeRfOptions, BladeRfSource};
use blueoxide::backends::limesdr::{LimeSdrOptions, LimeSdrSource};
use blueoxide::backends::xtrx::{XtrxOptions, XtrxSource};
use blueoxide::ble::{BleChannel, LeFrameConfig};
use blueoxide::capture::{
    CaptureLimits, CaptureStats, CapturedAdvertisingPdu, capture_primary_advertising,
};
use blueoxide::demod::{
    Le1mDemodConfig, Le1mPacketStreamDecoder, Le1mStreamDecoder, ReceivedAdvertisingPdu,
    ReceivedLePdu,
};
use blueoxide::iq::{IqFormat, open_iq_file};
use blueoxide::link_layer::{
    ChannelSelectionAlgorithm, ConnectionEventTiming, ConnectionParameters, ConnectionTracker,
    ConnectionTrackerConfig, ControlPdu, DataChannelMap, DataChannelPdu, IncompleteL2capPdu,
    L2capReassembler, L2capReassemblyOutcome, LinkDirection, LogicalLinkId, SampleTimingError,
    SleepClockAccuracy,
};
use blueoxide::pcapng::{PcapNgWriter, sample_timestamp_ns};
use blueoxide::sdr::{IqSource, SdrConfig};
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

#[derive(Debug)]
struct DecodeDataArgs {
    input: PathBuf,
    format: IqFormat,
    channel: BleChannel,
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
  --plaintext-l2cap-direction central-to-peripheral|peripheral-to-central
                          Reassemble an asserted single-direction plaintext stream
  --max-l2cap-payload N   Maximum reassembled payload length (default: 65535)

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
                plaintext_l2cap_direction = Some(match value.as_str() {
                    "central-to-peripheral" | "central" | "c2p" => {
                        LinkDirection::CentralToPeripheral
                    }
                    "peripheral-to-central" | "peripheral" | "p2c" => {
                        LinkDirection::PeripheralToCentral
                    }
                    _ => {
                        return Err(Error::InvalidConfiguration(format!(
                            "invalid value {value:?} for --plaintext-l2cap-direction; expected central-to-peripheral or peripheral-to-central"
                        )));
                    }
                });
            }
            "--max-l2cap-payload" => {
                let value = value_after(args, &mut index, "--max-l2cap-payload")?;
                maximum_l2cap_payload_length = parse_number(&value, "--max-l2cap-payload")?;
                maximum_l2cap_payload_length_supplied = true;
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

    Ok(DecodeDataArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode-data requires --input FILE".to_owned())
        })?,
        format,
        channel,
        sample_rate_hz: sample_rate_hz.ok_or_else(|| {
            Error::InvalidConfiguration("decode-data requires --sample-rate HZ".to_owned())
        })?,
        access_address,
        crc_init,
        max_samples,
        block_samples,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
        plaintext_l2cap_direction,
        maximum_l2cap_payload_length,
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
    if let Some(update) = control.connection_update_ind()? {
        return Ok(format!(
            "{} opcode=0x{:02x} window_offset={} window_size={} interval={} latency={} timeout={} instant={}",
            control.opcode_name(),
            control.opcode,
            update.window_offset,
            update.window_size,
            update.parameters.interval,
            update.parameters.latency,
            update.parameters.supervision_timeout,
            update.instant
        ));
    }
    if let Some(update) = control.channel_map_ind()? {
        return Ok(format!(
            "{} opcode=0x{:02x} channel_map={} channels={} instant={}",
            control.opcode_name(),
            control.opcode,
            print_hex(&update.channel_map.bytes()),
            update.channel_map.used_count(),
            update.instant
        ));
    }
    Ok(format!(
        "{} opcode=0x{:02x} parameter_octets={}",
        control.opcode_name(),
        control.opcode,
        control.parameters.len()
    ))
}

fn describe_data_pdu(packet: &DataChannelPdu) -> String {
    match packet.llid() {
        LogicalLinkId::StartOrComplete => packet
            .l2cap_start()
            .map(|start| match start {
                Some(start) => format!(
                    "L2CAP length={} cid=0x{:04x} fragment_octets={}",
                    start.payload_length,
                    start.channel_id,
                    start.fragment.len()
                ),
                None => unreachable!(),
            })
            .unwrap_or_else(|error| format!("decode_error={error}")),
        LogicalLinkId::Control => packet
            .control()
            .map(|control| match control {
                Some(control) => describe_control_pdu(control)
                    .unwrap_or_else(|error| format!("decode_error={error}")),
                None => unreachable!(),
            })
            .unwrap_or_else(|error| format!("decode_error={error}")),
        LogicalLinkId::ContinuationOrEmpty if packet.payload.is_empty() => "empty".to_owned(),
        LogicalLinkId::ContinuationOrEmpty => {
            format!("L2CAP continuation_octets={}", packet.payload.len())
        }
        LogicalLinkId::Reserved => {
            format!("reserved_llid payload_octets={}", packet.payload.len())
        }
    }
}

fn print_data_packet(packet: &ReceivedLePdu, data: &DataChannelPdu) {
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
    println!(
        "channel={} sample={} phase={} access_address={:08x} inverted={} aa_errors={} llid={} nesn={} sn={} md={} cp={} cte={} rfu={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} plaintext_hint=\"{}\"",
        data.channel.index(),
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
        describe_data_pdu(data).replace('"', "'"),
    );
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

    let demod_config = Le1mDemodConfig {
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let frame_config = LeFrameConfig::data(args.access_address, args.crc_init)?;
    let mut decoder = Le1mPacketStreamDecoder::new(args.channel, frame_config, demod_config)?;
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
            let data = DataChannelPdu::from(packet.pdu.clone());
            print_data_packet(packet, &data);
            if let Some((direction, reassembler)) = &mut l2cap_reassembler {
                match reassembler.push(*direction, &data) {
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
                            L2capReassemblyOutcome::Complete(sdu) => {
                                l2cap_pdu_count += 1;
                                println!(
                                    "l2cap_pdu direction={} cid=0x{:04x} length={} fragments={} payload={}",
                                    sdu.direction,
                                    sdu.channel_id,
                                    sdu.payload.len(),
                                    sdu.fragment_count,
                                    print_hex(&sdu.payload)
                                );
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
        "decoded {packet_count} CRC-valid data-channel packet(s) from {sample_count} sample(s)"
    );
    if l2cap_reassembler.is_some() {
        eprintln!(
            "reassembled {l2cap_pdu_count} plaintext L2CAP PDU(s); duplicates={l2cap_duplicate_count} orphan_continuations={l2cap_orphan_count} discarded_incomplete={l2cap_discarded_count} errors={l2cap_error_count}"
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
