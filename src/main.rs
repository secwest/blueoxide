use blueoxide::advertising::{
    ConnectRequest, ExtendedAdvertisingChainConfig, ExtendedAdvertisingChainProgress,
    ExtendedAdvertisingChainTracker, ExtendedAdvertisingPduKind, FirstCentralTransmission,
    decode_advertising_pdu, decode_contextual_extended_advertising_pdu,
};
use blueoxide::att::{AttPdu, AttUuid, DecodedAttPdu};
use blueoxide::backends::bladerf::{BladeRfOptions, BladeRfSource};
use blueoxide::backends::limesdr::{LimeSdrOptions, LimeSdrSource};
use blueoxide::backends::xtrx::{XtrxOptions, XtrxSource};
use blueoxide::ble::{BleChannel, LeFrameConfig};
use blueoxide::capture::{
    CaptureLimits, CaptureStats, CapturedAdvertisingPdu, CapturedDataChannelPdu,
    FixedChannelCentralObservationConfig, FixedChannelCentralObservationTracker,
    capture_data_channel, capture_primary_advertising,
};
use blueoxide::demod::{
    Le1mDemodConfig, Le1mStreamDecoder, LePeriodicAdvertisingStreamDecoder,
    LeSecondaryAdvertisingStreamDecoder, LeUncodedDemodConfig, LeUncodedPacketStreamDecoder,
    LeUncodedPhy, ReceivedAdvertisingPdu, ReceivedLePdu,
};
use blueoxide::iq::{IqFormat, open_iq_file};
use blueoxide::l2cap::{
    IncompleteL2capCreditBasedSdu, L2capCreditBasedChannel, L2capCreditBasedChannelTracker,
    L2capCreditBasedEvent, L2capCreditBasedSdu,
};
use blueoxide::link_layer::{
    ChannelSelectionAlgorithm, ConnectionEventTiming, ConnectionParameters, ConnectionPhyState,
    ConnectionTracker, ConnectionTrackerConfig, ControlPdu, DataChannelMap, DataChannelPdu,
    DecodedL2capSignalingCommand, IncompleteL2capPdu, L2capPdu, L2capReassembler,
    L2capReassemblyOutcome, L2capSignalingCommand, LE_ACL_MAXIMUM_COUNTER_SKIP, LeAclDecryption,
    LeAclDecryptionStatus, LeAclDecryptor, LePhy, LinkDirection, LogicalLinkId, PhyUpdateInd,
    SampleTimingError, SleepClockAccuracy,
};
use blueoxide::ll_control::{
    ChannelClassification, CsConfigAction, DecodedControlPdu, LeEncryptionMaterialTracker,
    LeEncryptionSessionTracker,
};
use blueoxide::pcapng::{PcapNgWriter, sample_timestamp_ns};
use blueoxide::periodic::{
    PeriodicAdvertisingEvent, PeriodicAdvertisingTracker, PeriodicAdvertisingTrackerConfig,
};
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

#[derive(Debug)]
struct DecodeSecondaryArgs {
    input: PathBuf,
    format: IqFormat,
    channel: BleChannel,
    phy: LeUncodedPhy,
    sample_rate_hz: u32,
    max_samples: usize,
    block_samples: usize,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: u64,
}

#[derive(Debug)]
struct DecodePeriodicArgs {
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
struct EncryptionTraceArgs {
    long_term_key: [u8; 16],
    maximum_counter_skip: u64,
    packets: Vec<DirectionalDataPacketArg>,
}

#[derive(Debug)]
struct L2capTraceArgs {
    pdus: Vec<L2capPdu>,
}

#[derive(Clone, Debug)]
struct DirectionalDataPacketArg {
    direction: LinkDirection,
    packet: DataChannelPdu,
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
    frame: CaptureFrame,
    central_observation_tracking: Option<FixedChannelCentralObservationConfig>,
}

#[derive(Clone, Copy, Debug)]
enum CaptureFrame {
    Advertising,
    Data {
        access_address: u32,
        crc_init: u32,
        phy: LeUncodedPhy,
    },
}

impl CaptureFrame {
    const fn command_name(self) -> &'static str {
        match self {
            Self::Advertising => "capture",
            Self::Data { .. } => "capture-data",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureCommand {
    Advertising,
    Data,
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
    initial_phy: ConnectionPhyState,
    phy_update: Option<PhyUpdateInd>,
}

#[derive(Clone, Copy, Debug)]
struct ConnectionObservationArg {
    channel: BleChannel,
    access_address_sample: u64,
}

#[derive(Debug)]
struct ExtendedAdvertisingPlanArgs {
    sample_rate_hz: u32,
    receiver_clock_accuracy_ppm: u32,
    maximum_advertising_data_length: usize,
    packets: Vec<ExtendedAdvertisingPacketArg>,
}

#[derive(Clone, Debug)]
struct ExtendedAdvertisingPacketArg {
    pdu: blueoxide::ble::AdvertisingPdu,
    phy: LePhy,
    access_address_sample: u64,
}

#[derive(Clone, Copy, Debug)]
struct PeriodicAdvertisingObservationArg {
    channel: BleChannel,
    phy: LePhy,
    access_address_sample: u64,
}

#[derive(Debug)]
struct PeriodicAdvertisingPlanArgs {
    sync_packet: ExtendedAdvertisingPacketArg,
    sample_rate_hz: u32,
    receiver_clock_accuracy_ppm: u32,
    event_count: usize,
    maximum_event_advance: u16,
    observations: Vec<PeriodicAdvertisingObservationArg>,
}

fn usage() -> &'static str {
    "blueoxide - Bluetooth/BLE SDR receive and capture tools

USAGE:
  blueoxide channels
  blueoxide backends
  blueoxide decode --input FILE --channel 37|38|39 --sample-rate HZ [OPTIONS]
  blueoxide decode-secondary --input FILE --channel 0..36 --sample-rate HZ [OPTIONS]
  blueoxide decode-periodic --input FILE --channel 0..36 --sample-rate HZ \
    --access-address 0xNNNNNNNN --crc-init 0xNNNNNN [OPTIONS]
  blueoxide decode-data --input FILE --channel 0..36 --sample-rate HZ \
    --access-address 0xNNNNNNNN --crc-init 0xNNNNNN [OPTIONS]
  blueoxide encryption-trace --ltk HEX \
    --packet DIRECTION:HEADERPAYLOADHEX [--packet ...] [OPTIONS]
  blueoxide l2cap-trace --pdu DIRECTION:CID:PAYLOADHEX [--pdu ...]
  blueoxide connection-plan --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --interval N --sample-rate HZ [OPTIONS]
  blueoxide connection-sync --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --interval N --sample-rate HZ \
    --observe CHANNEL:SAMPLE [OPTIONS]
  blueoxide connection-acquire --access-address 0xNNNNNNNN \
    --channel-map HEX --csa 1|2 --hop N --interval N --sample-rate HZ \
    --connect-sample N --central-observe CHANNEL:SAMPLE [OPTIONS]
  blueoxide extended-advertising-plan --sample-rate HZ \
    --packet CHANNEL:PHY:SAMPLE:PDUHEX [--packet ...]
  blueoxide periodic-advertising-plan --sample-rate HZ \
    --sync-packet CHANNEL:PHY:SAMPLE:PDUHEX [OPTIONS]
  blueoxide capture --device bladerf|limesdr|xtrx --channel 37|38|39 [OPTIONS]
  blueoxide capture-data --device bladerf|limesdr|xtrx --channel 0..36 \
    --access-address 0xNNNNNNNN --crc-init 0xNNNNNN [OPTIONS]

DECODE OPTIONS:
  --format f32le|s16le    Interleaved little-endian I/Q (default: f32le)
  --max-samples N         Maximum samples accepted from the file (default: 16000000)
  --block-samples N       Streaming decode block size (default: 262144)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Unix capture start in nanoseconds (default: 0)
  -h, --help              Show this help

DECODE-SECONDARY OPTIONS:
  Uses the DECODE OPTIONS above on one asserted secondary advertising channel.
  --phy 1m|2m             Uncoded secondary advertising PHY (default: 1m)

DECODE-PERIODIC OPTIONS:
  Uses the DECODE OPTIONS above on one asserted periodic advertising channel.
  --phy 1m|2m             Uncoded periodic advertising PHY (default: 1m)
  --access-address HEX    SyncInfo periodic advertising access address
  --crc-init HEX          SyncInfo 24-bit CRC initialization value

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

ENCRYPTION-TRACE OPTIONS:
  --ltk HEX               16 LTK octets in HCI/SMP field order
  --packet DIRECTION:HEX  Directed two-octet data header plus Length-counted
                          payload/MIC; repeat in observed wire order
  --max-counter-skip N    MIC-search skipped counters, 0..=65535 (default: 0)

L2CAP-TRACE OPTIONS:
  --pdu DIRECTION:CID:HEX Complete plaintext L2CAP PDU after the basic header;
                          CID accepts decimal or 0x-prefixed hexadecimal

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
  --c2p-phy 1m|2m|coded  PHY at the anchor, central to peripheral (default: 1m)
  --p2c-phy 1m|2m|coded  PHY at the anchor, peripheral to central (default: 1m)
  --phy-update C2P:P2C:INSTANT
                          Schedule directional PHYs; use unchanged for no change
  --events N              Number of events to print (default: 10)
  --peer-sca N            CONNECT_IND sleep-clock accuracy, 0..=7 (default: 0)
  --receiver-ppm N        Receiver sample-clock error bound (default: 20)
  --max-event-advance N   Maximum event advancement searched (default: 32)
  --central-observe CHANNEL:SAMPLE
                          CRC-valid central transmission for event-0 acquisition
  --observe CHANNEL:SAMPLE
                          Later CRC-valid observation; repeat as needed
  --window-size N         CONNECT_IND WinSize in 1.25 ms units (default: 1)
  --window-offset N       CONNECT_IND WinOffset in 1.25 ms units (default: 0)
  --connect-sample N      CONNECT_IND access-address sample for acquisition

EXTENDED-ADVERTISING-PLAN OPTIONS:
  --sample-rate HZ        Shared exact sample-coordinate rate
  --receiver-ppm N        Receiver sample-clock error bound (default: 20)
  --max-data N            Maximum assembled advertising data (default: 1650)
  --packet C:P:S:HEX      CRC-valid header+payload at channel, PHY, and sample;
                          repeat in ADV_EXT_IND/AUX_ADV_IND/AUX_CHAIN_IND order
                          PHY is 1m, 2m, or coded

PERIODIC-ADVERTISING-PLAN OPTIONS:
  --sync-packet C:P:S:HEX Packet containing SyncInfo at channel, PHY, and sample
  --sample-rate HZ        Shared exact sample-coordinate rate
  --receiver-ppm N        Receiver sample-clock error bound (default: 20)
  --events N              Number of scheduled events to print (default: 10)
  --observe C:P:S         CRC-valid periodic observation; repeat as needed
  --max-event-advance N   Maximum event advancement searched (default: 32)
                          PHY is 1m, 2m, or coded

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

CAPTURE-DATA OPTIONS:
  Uses the CAPTURE OPTIONS above on one fixed data channel.
  --access-address HEX    Connection access address
  --crc-init HEX          24-bit connection CRC initialization value
  --phy 1m|2m             Uncoded LE data PHY (default: 1m)
  --assert-central-observations
                          Treat every decoded packet as a central anchor candidate
  --first-event N         Event counter assigned to the first central observation
  --channel-map HEX       Five connection map octets in over-the-air order
  --csa 1|2               Connection channel selection algorithm
  --hop N                 CSA#1 hop increment, 5..=16 (default: 5)
  --interval N            Connection interval in 1.25 ms units
  --latency N             Peripheral latency (default: 0)
  --timeout N             Supervision timeout in 10 ms units (default: 3200)
  --peer-sca N            Peer sleep-clock accuracy, 0..=7 (default: 0)
  --receiver-ppm N        Receiver sample-clock error bound (default: 20)
  --max-event-advance N   Maximum event advancement searched (default: 32)
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

fn parse_hex_bytes(value: &str, option: &str) -> Result<Vec<u8>> {
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if !hex.len().is_multiple_of(2) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected an even number of hexadecimal digits"
        )));
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for index in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[index..index + 2], 16).map_err(|_| {
            Error::InvalidConfiguration(format!("invalid value {value:?} for {option}"))
        })?);
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

fn parse_connection_phy(value: &str, option: &str) -> Result<LePhy> {
    match value.to_ascii_lowercase().as_str() {
        "1m" | "le-1m" => Ok(LePhy::Le1M),
        "2m" | "le-2m" => Ok(LePhy::Le2M),
        "coded" | "le-coded" => Ok(LePhy::LeCoded),
        _ => Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected 1m, 2m, or coded"
        ))),
    }
}

fn parse_connection_phy_update(value: &str) -> Result<PhyUpdateInd> {
    let mut fields = value.split(':');
    let central_to_peripheral = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --phy-update; expected C2P:P2C:INSTANT".to_owned())
    })?;
    let peripheral_to_central = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --phy-update; expected C2P:P2C:INSTANT".to_owned())
    })?;
    let instant = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --phy-update; expected C2P:P2C:INSTANT".to_owned())
    })?;
    if fields.next().is_some() {
        return Err(Error::InvalidConfiguration(
            "invalid --phy-update; expected C2P:P2C:INSTANT".to_owned(),
        ));
    }
    let parse_field = |field: &str, option: &str| -> Result<u8> {
        if matches!(
            field.to_ascii_lowercase().as_str(),
            "0" | "same" | "unchanged"
        ) {
            Ok(0)
        } else {
            Ok(parse_connection_phy(field, option)?.raw())
        }
    };
    PhyUpdateInd::new(
        parse_field(central_to_peripheral, "--phy-update C2P")?,
        parse_field(peripheral_to_central, "--phy-update P2C")?,
        parse_number(instant, "--phy-update instant")?,
    )
}

fn parse_channel_selection_algorithm(
    value: &str,
    option: &str,
) -> Result<ChannelSelectionAlgorithm> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "csa1" | "csa#1" => Ok(ChannelSelectionAlgorithm::Csa1),
        "2" | "csa2" | "csa#2" => Ok(ChannelSelectionAlgorithm::Csa2),
        _ => Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for {option}; expected 1 or 2"
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

fn parse_advertising_packet_arg(value: &str, option: &str) -> Result<ExtendedAdvertisingPacketArg> {
    let mut fields = value.splitn(4, ':');
    let channel = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid {option}; expected CHANNEL:PHY:SAMPLE:PDUHEX"
        ))
    })?;
    let phy = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid {option}; expected CHANNEL:PHY:SAMPLE:PDUHEX"
        ))
    })?;
    let sample = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid {option}; expected CHANNEL:PHY:SAMPLE:PDUHEX"
        ))
    })?;
    let bytes = parse_hex_bytes(
        fields.next().ok_or_else(|| {
            Error::InvalidConfiguration(format!(
                "invalid {option}; expected CHANNEL:PHY:SAMPLE:PDUHEX"
            ))
        })?,
        &format!("{option} PDUHEX"),
    )?;
    if bytes.len() < 2 {
        return Err(Error::InvalidConfiguration(format!(
            "{option} PDUHEX must contain the two-octet advertising header"
        )));
    }
    let channel = BleChannel::new(parse_number(channel, &format!("{option} channel"))?)?;
    let header = [bytes[0], bytes[1]];
    let declared_length = if channel.is_primary_advertising() {
        usize::from(header[1] & 0x3f)
    } else {
        usize::from(header[1])
    };
    if bytes.len() != 2 + declared_length {
        return Err(Error::InvalidConfiguration(format!(
            "{option} PDUHEX declares {declared_length} payload octets but contains {}",
            bytes.len() - 2
        )));
    }
    Ok(ExtendedAdvertisingPacketArg {
        pdu: blueoxide::ble::AdvertisingPdu {
            channel,
            access_address: blueoxide::ble::LE_ADV_ACCESS_ADDRESS,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header,
            payload: bytes[2..].to_vec(),
            crc: [0; 3],
        },
        phy: parse_connection_phy(phy, &format!("{option} PHY"))?,
        access_address_sample: parse_u64(sample, &format!("{option} sample"))?,
    })
}

fn parse_extended_advertising_packet(value: &str) -> Result<ExtendedAdvertisingPacketArg> {
    parse_advertising_packet_arg(value, "--packet")
}

fn parse_extended_advertising_plan_args(args: &[String]) -> Result<ExtendedAdvertisingPlanArgs> {
    let mut sample_rate_hz = None;
    let mut receiver_clock_accuracy_ppm = 20u32;
    let mut maximum_advertising_data_length = 1_650usize;
    let mut packets = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--sample-rate" => {
                sample_rate_hz = Some(parse_number(
                    &value_after(args, &mut index, "--sample-rate")?,
                    "--sample-rate",
                )?);
            }
            "--receiver-ppm" => {
                receiver_clock_accuracy_ppm = parse_number(
                    &value_after(args, &mut index, "--receiver-ppm")?,
                    "--receiver-ppm",
                )?;
            }
            "--max-data" => {
                maximum_advertising_data_length =
                    parse_number(&value_after(args, &mut index, "--max-data")?, "--max-data")?;
            }
            "--packet" => packets.push(parse_extended_advertising_packet(&value_after(
                args, &mut index, "--packet",
            )?)?),
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown extended-advertising-plan option {unknown:?}"
                )));
            }
        }
        index += 1;
    }
    if packets.is_empty() {
        return Err(Error::InvalidConfiguration(
            "extended-advertising-plan requires at least one --packet".to_owned(),
        ));
    }
    let plan = ExtendedAdvertisingPlanArgs {
        sample_rate_hz: sample_rate_hz.ok_or_else(|| {
            Error::InvalidConfiguration(
                "extended-advertising-plan requires --sample-rate HZ".to_owned(),
            )
        })?,
        receiver_clock_accuracy_ppm,
        maximum_advertising_data_length,
        packets,
    };
    ExtendedAdvertisingChainConfig {
        sample_rate_hz: plan.sample_rate_hz,
        receiver_clock_accuracy_ppm: plan.receiver_clock_accuracy_ppm,
        maximum_advertising_data_length: plan.maximum_advertising_data_length,
    }
    .validate()?;
    Ok(plan)
}

fn parse_periodic_advertising_observation(
    value: &str,
) -> Result<PeriodicAdvertisingObservationArg> {
    let mut fields = value.split(':');
    let channel = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --observe; expected CHANNEL:PHY:SAMPLE".to_owned())
    })?;
    let phy = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --observe; expected CHANNEL:PHY:SAMPLE".to_owned())
    })?;
    let sample = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration("invalid --observe; expected CHANNEL:PHY:SAMPLE".to_owned())
    })?;
    if fields.next().is_some() {
        return Err(Error::InvalidConfiguration(
            "invalid --observe; expected CHANNEL:PHY:SAMPLE".to_owned(),
        ));
    }
    let channel = BleChannel::new(parse_number(channel, "--observe channel")?)?;
    if channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "--observe requires a periodic advertising channel in 0..=36; got {}",
            channel.index()
        )));
    }
    Ok(PeriodicAdvertisingObservationArg {
        channel,
        phy: parse_connection_phy(phy, "--observe PHY")?,
        access_address_sample: parse_u64(sample, "--observe sample")?,
    })
}

fn parse_periodic_advertising_plan_args(args: &[String]) -> Result<PeriodicAdvertisingPlanArgs> {
    let mut sync_packet = None;
    let mut sample_rate_hz = None;
    let mut receiver_clock_accuracy_ppm = 20u32;
    let mut event_count = 10usize;
    let mut maximum_event_advance = 32u16;
    let mut observations = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--sync-packet" => {
                let packet = parse_advertising_packet_arg(
                    &value_after(args, &mut index, "--sync-packet")?,
                    "--sync-packet",
                )?;
                if sync_packet.replace(packet).is_some() {
                    return Err(Error::InvalidConfiguration(
                        "--sync-packet may only be supplied once".to_owned(),
                    ));
                }
            }
            "--sample-rate" => {
                sample_rate_hz = Some(parse_number(
                    &value_after(args, &mut index, "--sample-rate")?,
                    "--sample-rate",
                )?);
            }
            "--receiver-ppm" => {
                receiver_clock_accuracy_ppm = parse_number(
                    &value_after(args, &mut index, "--receiver-ppm")?,
                    "--receiver-ppm",
                )?;
            }
            "--events" => {
                event_count =
                    parse_number(&value_after(args, &mut index, "--events")?, "--events")?;
            }
            "--max-event-advance" => {
                maximum_event_advance = parse_number(
                    &value_after(args, &mut index, "--max-event-advance")?,
                    "--max-event-advance",
                )?;
            }
            "--observe" => observations.push(parse_periodic_advertising_observation(
                &value_after(args, &mut index, "--observe")?,
            )?),
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown periodic-advertising-plan option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    let sync_packet = sync_packet.ok_or_else(|| {
        Error::InvalidConfiguration(
            "periodic-advertising-plan requires --sync-packet CHANNEL:PHY:SAMPLE:PDUHEX".to_owned(),
        )
    })?;
    if sync_packet.pdu.channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "--sync-packet requires a secondary advertising channel in 0..=36; got {}",
            sync_packet.pdu.channel.index()
        )));
    }
    if event_count == 0 {
        return Err(Error::InvalidConfiguration(
            "--events must be greater than zero".to_owned(),
        ));
    }
    let sample_rate_hz = sample_rate_hz.ok_or_else(|| {
        Error::InvalidConfiguration(
            "periodic-advertising-plan requires --sample-rate HZ".to_owned(),
        )
    })?;
    PeriodicAdvertisingTrackerConfig {
        sample_rate_hz,
        receiver_clock_accuracy_ppm,
    }
    .validate()?;
    Ok(PeriodicAdvertisingPlanArgs {
        sync_packet,
        sample_rate_hz,
        receiver_clock_accuracy_ppm,
        event_count,
        maximum_event_advance,
        observations,
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
    let mut initial_phy = ConnectionPhyState::default();
    let mut phy_update = None;
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
                channel_selection_algorithm =
                    Some(parse_channel_selection_algorithm(&value, "--csa")?);
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
            "--c2p-phy" => {
                let value = value_after(args, &mut index, "--c2p-phy")?;
                initial_phy.central_to_peripheral = parse_connection_phy(&value, "--c2p-phy")?;
            }
            "--p2c-phy" => {
                let value = value_after(args, &mut index, "--p2c-phy")?;
                initial_phy.peripheral_to_central = parse_connection_phy(&value, "--p2c-phy")?;
            }
            "--phy-update" => {
                let update =
                    parse_connection_phy_update(&value_after(args, &mut index, "--phy-update")?)?;
                if phy_update.replace(update).is_some() {
                    return Err(Error::InvalidConfiguration(
                        "--phy-update may only be supplied once".to_owned(),
                    ));
                }
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
        initial_phy,
        phy_update,
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

fn parse_decode_secondary_args(args: &[String]) -> Result<DecodeSecondaryArgs> {
    let mut input = None;
    let mut format = IqFormat::F32Le;
    let mut channel = None;
    let mut phy = LeUncodedPhy::Le1M;
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
            "--phy" => {
                let value = value_after(args, &mut index, "--phy")?;
                phy = parse_uncoded_phy(&value, "--phy")?;
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
                    "unknown decode-secondary option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    let channel = channel.ok_or_else(|| {
        Error::InvalidConfiguration("decode-secondary requires --channel 0..36".to_owned())
    })?;
    if channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "decode-secondary requires a secondary advertising channel in 0..=36; got {}",
            channel.index()
        )));
    }
    let sample_rate_hz = sample_rate_hz.ok_or_else(|| {
        Error::InvalidConfiguration("decode-secondary requires --sample-rate HZ".to_owned())
    })?;
    LeUncodedDemodConfig {
        phy,
        sample_rate_hz,
        max_access_address_errors,
    }
    .validate()?;
    if block_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be greater than zero".to_owned(),
        ));
    }

    Ok(DecodeSecondaryArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode-secondary requires --input FILE".to_owned())
        })?,
        format,
        channel,
        phy,
        sample_rate_hz,
        max_samples,
        block_samples,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
    })
}

fn parse_decode_periodic_args(args: &[String]) -> Result<DecodePeriodicArgs> {
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
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => input = Some(PathBuf::from(value_after(args, &mut index, "--input")?)),
            "--format" => format = IqFormat::parse(&value_after(args, &mut index, "--format")?)?,
            "--channel" => {
                channel = Some(BleChannel::new(parse_number(
                    &value_after(args, &mut index, "--channel")?,
                    "--channel",
                )?)?);
            }
            "--phy" => {
                phy = parse_uncoded_phy(&value_after(args, &mut index, "--phy")?, "--phy")?;
            }
            "--sample-rate" => {
                sample_rate_hz = Some(parse_number(
                    &value_after(args, &mut index, "--sample-rate")?,
                    "--sample-rate",
                )?);
            }
            "--access-address" => {
                access_address = Some(parse_u32(
                    &value_after(args, &mut index, "--access-address")?,
                    "--access-address",
                )?);
            }
            "--crc-init" => {
                crc_init = Some(parse_u32(
                    &value_after(args, &mut index, "--crc-init")?,
                    "--crc-init",
                )?);
            }
            "--max-samples" => {
                max_samples = parse_number(
                    &value_after(args, &mut index, "--max-samples")?,
                    "--max-samples",
                )?;
            }
            "--block-samples" => {
                block_samples = parse_number(
                    &value_after(args, &mut index, "--block-samples")?,
                    "--block-samples",
                )?;
            }
            "--aa-errors" => {
                max_access_address_errors = parse_number(
                    &value_after(args, &mut index, "--aa-errors")?,
                    "--aa-errors",
                )?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                capture_start_ns = parse_number(
                    &value_after(args, &mut index, "--capture-start-ns")?,
                    "--capture-start-ns",
                )?;
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown decode-periodic option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    let channel = channel.ok_or_else(|| {
        Error::InvalidConfiguration("decode-periodic requires --channel 0..36".to_owned())
    })?;
    if channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "decode-periodic requires a periodic advertising channel in 0..=36; got {}",
            channel.index()
        )));
    }
    let sample_rate_hz = sample_rate_hz.ok_or_else(|| {
        Error::InvalidConfiguration("decode-periodic requires --sample-rate HZ".to_owned())
    })?;
    let access_address = access_address.ok_or_else(|| {
        Error::InvalidConfiguration(
            "decode-periodic requires --access-address 0xNNNNNNNN".to_owned(),
        )
    })?;
    let crc_init = crc_init.ok_or_else(|| {
        Error::InvalidConfiguration("decode-periodic requires --crc-init 0xNNNNNN".to_owned())
    })?;
    LeFrameConfig::periodic_advertising(access_address, crc_init)?;
    LeUncodedDemodConfig {
        phy,
        sample_rate_hz,
        max_access_address_errors,
    }
    .validate()?;
    if block_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be greater than zero".to_owned(),
        ));
    }

    Ok(DecodePeriodicArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode-periodic requires --input FILE".to_owned())
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

fn parse_encryption_trace_packet(value: &str) -> Result<DirectionalDataPacketArg> {
    let (direction, bytes) = value.split_once(':').ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --packet; expected DIRECTION:HEADERPAYLOADHEX"
        ))
    })?;
    let direction = parse_link_direction(direction, "--packet direction")?;
    let bytes = parse_hex_bytes(bytes, "--packet")?;
    if bytes.len() < 2 {
        return Err(Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --packet; expected a two-octet data header"
        )));
    }
    if bytes[0] & 0x20 != 0 {
        return Err(Error::InvalidConfiguration(
            "--packet does not accept a CTEInfo field; clear the data-header CP bit".to_owned(),
        ));
    }
    let payload_length = usize::from(bytes[1]);
    if bytes.len() != payload_length + 2 {
        return Err(Error::InvalidConfiguration(format!(
            "--packet data Length declares {payload_length} payload octets but {} were supplied",
            bytes.len() - 2
        )));
    }
    Ok(DirectionalDataPacketArg {
        direction,
        packet: DataChannelPdu {
            channel: BleChannel::new(0).expect("data channel zero is valid"),
            access_address: 0,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [bytes[0], bytes[1]],
            cte_info: None,
            payload: bytes[2..].to_vec(),
            crc: [0; 3],
        },
    })
}

fn parse_encryption_trace_args(args: &[String]) -> Result<EncryptionTraceArgs> {
    let mut long_term_key = None;
    let mut maximum_counter_skip = 0;
    let mut packets = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--ltk" => {
                long_term_key = Some(parse_fixed_hex(
                    &value_after(args, &mut index, "--ltk")?,
                    "--ltk",
                )?);
            }
            "--max-counter-skip" => {
                maximum_counter_skip = parse_u64(
                    &value_after(args, &mut index, "--max-counter-skip")?,
                    "--max-counter-skip",
                )?;
            }
            "--packet" => {
                packets.push(parse_encryption_trace_packet(&value_after(
                    args, &mut index, "--packet",
                )?)?);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown encryption-trace option {unknown:?}"
                )));
            }
        }
        index += 1;
    }
    let long_term_key = long_term_key
        .ok_or_else(|| Error::InvalidConfiguration("encryption-trace requires --ltk".to_owned()))?;
    if packets.is_empty() {
        return Err(Error::InvalidConfiguration(
            "encryption-trace requires at least one --packet".to_owned(),
        ));
    }
    if maximum_counter_skip > LE_ACL_MAXIMUM_COUNTER_SKIP {
        return Err(Error::InvalidConfiguration(format!(
            "--max-counter-skip must be in 0..={LE_ACL_MAXIMUM_COUNTER_SKIP}"
        )));
    }
    Ok(EncryptionTraceArgs {
        long_term_key,
        maximum_counter_skip,
        packets,
    })
}

fn parse_l2cap_trace_pdu(value: &str) -> Result<L2capPdu> {
    let mut fields = value.splitn(3, ':');
    let direction = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --pdu; expected DIRECTION:CID:PAYLOADHEX"
        ))
    })?;
    let channel_id = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --pdu; expected DIRECTION:CID:PAYLOADHEX"
        ))
    })?;
    let payload = fields.next().ok_or_else(|| {
        Error::InvalidConfiguration(format!(
            "invalid value {value:?} for --pdu; expected DIRECTION:CID:PAYLOADHEX"
        ))
    })?;
    let direction = parse_link_direction(direction, "--pdu direction")?;
    let channel_id = parse_u32(channel_id, "--pdu CID")?;
    let channel_id = u16::try_from(channel_id).map_err(|_| {
        Error::InvalidConfiguration(format!("--pdu CID 0x{channel_id:x} exceeds 16 bits"))
    })?;
    Ok(L2capPdu {
        direction,
        channel_id,
        payload: parse_hex_bytes(payload, "--pdu payload")?,
        fragment_count: 1,
    })
}

fn parse_l2cap_trace_args(args: &[String]) -> Result<L2capTraceArgs> {
    let mut pdus = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--pdu" => pdus.push(parse_l2cap_trace_pdu(&value_after(
                args, &mut index, "--pdu",
            )?)?),
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown l2cap-trace option {unknown:?}"
                )));
            }
        }
        index += 1;
    }
    if pdus.is_empty() {
        return Err(Error::InvalidConfiguration(
            "l2cap-trace requires at least one --pdu".to_owned(),
        ));
    }
    Ok(L2capTraceArgs { pdus })
}

fn parse_capture_args(args: &[String], command: CaptureCommand) -> Result<CaptureArgs> {
    let mut device = None;
    let mut identifier = None;
    let mut channel = None;
    let mut access_address = None;
    let mut crc_init = None;
    let mut phy = LeUncodedPhy::Le1M;
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
    let mut assert_central_observations = false;
    let mut tracking_options_supplied = false;
    let mut tracking_first_event_counter = None;
    let mut tracking_channel_map = None;
    let mut tracking_channel_selection_algorithm = None;
    let mut tracking_hop_increment = 5u8;
    let mut tracking_interval = None;
    let mut tracking_latency = 0u16;
    let mut tracking_supervision_timeout = 3_200u16;
    let mut tracking_peer_clock_accuracy = SleepClockAccuracy::new(0)?;
    let mut tracking_receiver_clock_accuracy_ppm = 20u32;
    let mut tracking_maximum_event_advance = 32u16;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--device" => device = Some(value_after(args, &mut index, "--device")?),
            "--identifier" => identifier = Some(value_after(args, &mut index, "--identifier")?),
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--access-address" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--access-address")?;
                access_address = Some(parse_u32(&value, "--access-address")?);
            }
            "--crc-init" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--crc-init")?;
                crc_init = Some(parse_u32(&value, "--crc-init")?);
            }
            "--phy" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--phy")?;
                phy = parse_uncoded_phy(&value, "--phy")?;
            }
            "--assert-central-observations" if command == CaptureCommand::Data => {
                assert_central_observations = true;
                tracking_options_supplied = true;
            }
            "--first-event" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--first-event")?;
                tracking_first_event_counter = Some(parse_number(&value, "--first-event")?);
                tracking_options_supplied = true;
            }
            "--channel-map" if command == CaptureCommand::Data => {
                tracking_channel_map = Some(parse_channel_map(&value_after(
                    args,
                    &mut index,
                    "--channel-map",
                )?)?);
                tracking_options_supplied = true;
            }
            "--csa" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--csa")?;
                tracking_channel_selection_algorithm =
                    Some(parse_channel_selection_algorithm(&value, "--csa")?);
                tracking_options_supplied = true;
            }
            "--hop" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--hop")?;
                tracking_hop_increment = parse_number(&value, "--hop")?;
                tracking_options_supplied = true;
            }
            "--interval" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--interval")?;
                tracking_interval = Some(parse_number(&value, "--interval")?);
                tracking_options_supplied = true;
            }
            "--latency" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--latency")?;
                tracking_latency = parse_number(&value, "--latency")?;
                tracking_options_supplied = true;
            }
            "--timeout" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--timeout")?;
                tracking_supervision_timeout = parse_number(&value, "--timeout")?;
                tracking_options_supplied = true;
            }
            "--peer-sca" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--peer-sca")?;
                tracking_peer_clock_accuracy =
                    SleepClockAccuracy::new(parse_number(&value, "--peer-sca")?)?;
                tracking_options_supplied = true;
            }
            "--receiver-ppm" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--receiver-ppm")?;
                tracking_receiver_clock_accuracy_ppm = parse_number(&value, "--receiver-ppm")?;
                tracking_options_supplied = true;
            }
            "--max-event-advance" if command == CaptureCommand::Data => {
                let value = value_after(args, &mut index, "--max-event-advance")?;
                tracking_maximum_event_advance = parse_number(&value, "--max-event-advance")?;
                tracking_options_supplied = true;
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
                    "unknown {} option {unknown:?}",
                    match command {
                        CaptureCommand::Advertising => "capture",
                        CaptureCommand::Data => "capture-data",
                    }
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
    let command_name = match command {
        CaptureCommand::Advertising => "capture",
        CaptureCommand::Data => "capture-data",
    };
    let channel = channel
        .ok_or_else(|| Error::InvalidConfiguration(format!("{command_name} requires --channel")))?;
    let (frame, central_observation_tracking) = match command {
        CaptureCommand::Advertising => {
            if !channel.is_primary_advertising() {
                return Err(Error::InvalidConfiguration(format!(
                    "capture requires BLE advertising channel 37, 38, or 39; got {}",
                    channel.index()
                )));
            }
            Le1mDemodConfig {
                sample_rate_hz,
                max_access_address_errors,
            }
            .validate()?;
            (CaptureFrame::Advertising, None)
        }
        CaptureCommand::Data => {
            if channel.index() > 36 {
                return Err(Error::InvalidConfiguration(format!(
                    "capture-data requires a data channel in 0..=36; got {}",
                    channel.index()
                )));
            }
            let access_address = access_address.ok_or_else(|| {
                Error::InvalidConfiguration(
                    "capture-data requires --access-address 0xNNNNNNNN".to_owned(),
                )
            })?;
            let crc_init = crc_init.ok_or_else(|| {
                Error::InvalidConfiguration("capture-data requires --crc-init 0xNNNNNN".to_owned())
            })?;
            LeFrameConfig::data(access_address, crc_init)?;
            LeUncodedDemodConfig {
                phy,
                sample_rate_hz,
                max_access_address_errors,
            }
            .validate()?;
            let central_observation_tracking = if tracking_options_supplied {
                if !assert_central_observations {
                    return Err(Error::InvalidConfiguration(
                        "connection-event tracking options require --assert-central-observations"
                            .to_owned(),
                    ));
                }
                let config = FixedChannelCentralObservationConfig {
                    tracker: ConnectionTrackerConfig {
                        access_address,
                        channel_selection_algorithm: tracking_channel_selection_algorithm
                            .ok_or_else(|| {
                                Error::InvalidConfiguration(
                                    "--assert-central-observations requires --csa 1|2".to_owned(),
                                )
                            })?,
                        hop_increment: tracking_hop_increment,
                        channel_map: tracking_channel_map.ok_or_else(|| {
                            Error::InvalidConfiguration(
                                "--assert-central-observations requires --channel-map HEX"
                                    .to_owned(),
                            )
                        })?,
                        parameters: ConnectionParameters::new(
                            tracking_interval.ok_or_else(|| {
                                Error::InvalidConfiguration(
                                    "--assert-central-observations requires --interval N"
                                        .to_owned(),
                                )
                            })?,
                            tracking_latency,
                            tracking_supervision_timeout,
                        )?,
                        sample_rate_hz,
                    },
                    first_event_counter: tracking_first_event_counter.ok_or_else(|| {
                        Error::InvalidConfiguration(
                            "--assert-central-observations requires --first-event N".to_owned(),
                        )
                    })?,
                    peer_clock_accuracy: tracking_peer_clock_accuracy,
                    receiver_clock_accuracy_ppm: tracking_receiver_clock_accuracy_ppm,
                    maximum_event_advance: tracking_maximum_event_advance,
                };
                config.validate(channel)?;
                Some(config)
            } else {
                None
            };
            (
                CaptureFrame::Data {
                    access_address,
                    crc_init,
                    phy,
                },
                central_observation_tracking,
            )
        }
    };
    Ok(CaptureArgs {
        device: device.ok_or_else(|| {
            Error::InvalidConfiguration(format!(
                "{command_name} requires --device bladerf|limesdr|xtrx"
            ))
        })?,
        identifier,
        channel,
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
        frame,
        central_observation_tracking,
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
        "channel={} phy={} sample={} phase={} inverted={} aa_errors={} pdu_type={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} semantic=\"{}\"",
        packet.pdu.channel.index(),
        packet.phy,
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

fn print_periodic_packet(packet: &ReceivedAdvertisingPdu) {
    let semantic = decode_contextual_extended_advertising_pdu(
        &packet.pdu,
        ExtendedAdvertisingPduKind::AuxSyncInd,
    )
    .map(|decoded| decoded.to_string())
    .unwrap_or_else(|error| format!("decode_error={error}"));
    println!(
        "channel={} phy={} sample={} phase={} access_address={:08x} inverted={} aa_errors={} pdu_type={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} semantic=\"{}\"",
        packet.pdu.channel.index(),
        packet.phy,
        packet.access_address_sample,
        packet.symbol_phase,
        packet.pdu.access_address,
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
            "{} opcode=0x{:02x} central_to_peripheral_phy={} peripheral_to_central_phy={} instant={}",
            control.opcode_name(),
            control.opcode,
            value
                .central_to_peripheral_phy
                .map_or_else(|| "unchanged".to_owned(), |phy| phy.to_string()),
            value
                .peripheral_to_central_phy
                .map_or_else(|| "unchanged".to_owned(), |phy| phy.to_string()),
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

fn decode_secondary(args: DecodeSecondaryArgs) -> Result<()> {
    let (mut reader, sample_count) = open_iq_file(&args.input, args.format)?;
    if sample_count > args.max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {}",
            args.max_samples
        )));
    }

    let config = LeUncodedDemodConfig {
        phy: args.phy,
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let mut decoder = LeSecondaryAdvertisingStreamDecoder::new(args.channel, config)?;
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

fn decode_periodic(args: DecodePeriodicArgs) -> Result<()> {
    let (mut reader, sample_count) = open_iq_file(&args.input, args.format)?;
    if sample_count > args.max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {}",
            args.max_samples
        )));
    }

    let config = LeUncodedDemodConfig {
        phy: args.phy,
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let mut decoder = LePeriodicAdvertisingStreamDecoder::new(
        args.channel,
        args.access_address,
        args.crc_init,
        config,
    )?;
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
            print_periodic_packet(packet);
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

fn encryption_trace(args: EncryptionTraceArgs) -> Result<()> {
    let mut tracker =
        LeEncryptionSessionTracker::new(args.long_term_key, args.maximum_counter_skip)?;
    let mut accepted = 0usize;
    let mut errors = 0usize;
    for (index, directed) in args.packets.iter().enumerate() {
        println!(
            "raw_encryption_packet index={index} direction={} header={} payload={}",
            directed.direction,
            print_hex(&directed.packet.header),
            print_hex(&directed.packet.payload)
        );
        match tracker.observe(directed.direction, &directed.packet) {
            Ok(observation) => {
                accepted += 1;
                let (protection, packet_counter, skipped_counters) = match observation.decryption {
                    None => ("plaintext", "none".to_owned(), 0),
                    Some(LeAclDecryptionStatus::New {
                        packet_counter,
                        skipped_counters,
                    }) => (
                        "encrypted-new",
                        packet_counter.to_string(),
                        skipped_counters,
                    ),
                    Some(LeAclDecryptionStatus::Retransmission { packet_counter }) => {
                        ("encrypted-retransmission", packet_counter.to_string(), 0)
                    }
                    Some(LeAclDecryptionStatus::UnencryptedEmpty) => {
                        ("unencrypted-empty", "none".to_owned(), 0)
                    }
                };
                let control = observation
                    .packet
                    .control()
                    .ok()
                    .flatten()
                    .map(ControlPdu::opcode_name)
                    .unwrap_or("none");
                println!(
                    "encryption_observation index={index} direction={} protection={protection} packet_counter={packet_counter} skipped_counters={skipped_counters} state_before={} state_after={} header={} payload={} control={control}",
                    directed.direction,
                    observation.state_before,
                    observation.state_after,
                    print_hex(&observation.packet.header),
                    print_hex(&observation.packet.payload),
                );
            }
            Err(error) => {
                errors += 1;
                eprintln!(
                    "encryption observation error: index={index} direction={} state={} error={error}",
                    directed.direction,
                    tracker.state()
                );
            }
        }
    }
    eprintln!(
        "processed {} directed encryption packet(s); accepted={accepted} errors={errors} final_state={}",
        args.packets.len(),
        tracker.state()
    );
    Ok(())
}

fn describe_credit_channel(channel: &L2capCreditBasedChannel) -> String {
    format!(
        "mode={} spsm=0x{:04x} eatt={} status={} central_cid=0x{:04x} central_mtu={} central_mps={} central_credits={} peripheral_cid=0x{:04x} peripheral_mtu={} peripheral_mps={} peripheral_credits={}",
        channel.mode,
        channel.spsm,
        channel.is_eatt(),
        channel.status,
        channel.central.channel_id,
        channel.central.mtu,
        channel.central.mps,
        channel.central.credits,
        channel.peripheral.channel_id,
        channel.peripheral.mtu,
        channel.peripheral.mps,
        channel.peripheral.credits
    )
}

fn print_credit_sdu(index: usize, sdu: &L2capCreditBasedSdu) -> Result<()> {
    println!(
        "l2cap_credit_sdu index={index} direction={} cid=0x{:04x} spsm=0x{:04x} eatt={} segments={} payload={}",
        sdu.direction,
        sdu.channel_id,
        sdu.spsm,
        sdu.is_eatt(),
        sdu.segment_count,
        print_hex(&sdu.payload)
    );
    if let Some(att) = sdu.att_pdu()? {
        println!(
            "eatt_pdu index={index} direction={} cid=0x{:04x} {}",
            sdu.direction,
            sdu.channel_id,
            describe_att_pdu(att)?
        );
    }
    Ok(())
}

fn print_l2cap_credit_event(index: usize, event: L2capCreditBasedEvent) -> Result<()> {
    match event {
        L2capCreditBasedEvent::Ignored => {
            println!("l2cap_credit_event index={index} kind=ignored");
        }
        L2capCreditBasedEvent::ConnectionRequestPending {
            mode,
            identifier,
            spsm,
            channel_count,
        } => println!(
            "l2cap_credit_event index={index} kind=connection-request-pending mode={mode} identifier={identifier} spsm=0x{spsm:04x} channels={channel_count}"
        ),
        L2capCreditBasedEvent::ConnectionRejected {
            mode,
            identifier,
            result,
        } => println!(
            "l2cap_credit_event index={index} kind=connection-rejected mode={mode} identifier={identifier} result=0x{result:04x}"
        ),
        L2capCreditBasedEvent::ChannelsOpened(channels) => {
            for channel in channels {
                println!(
                    "l2cap_credit_event index={index} kind=channel-opened {}",
                    describe_credit_channel(&channel)
                );
            }
        }
        L2capCreditBasedEvent::CreditsAdded {
            owner,
            channel_id,
            added,
            total,
        } => println!(
            "l2cap_credit_event index={index} kind=credits-added owner={} cid=0x{channel_id:04x} added={added} total={total}",
            match owner {
                LinkDirection::CentralToPeripheral => "central",
                LinkDirection::PeripheralToCentral => "peripheral",
            }
        ),
        L2capCreditBasedEvent::SduInProgress(IncompleteL2capCreditBasedSdu {
            direction,
            channel_id,
            spsm,
            expected_octets,
            received_octets,
            segment_count,
        }) => println!(
            "l2cap_credit_event index={index} kind=sdu-in-progress direction={direction} cid=0x{channel_id:04x} spsm=0x{spsm:04x} received={received_octets} expected={expected_octets} segments={segment_count}"
        ),
        L2capCreditBasedEvent::SduComplete(sdu) => print_credit_sdu(index, &sdu)?,
        L2capCreditBasedEvent::ReconfigurePending {
            identifier,
            owner,
            channel_ids,
            mtu,
            mps,
        } => println!(
            "l2cap_credit_event index={index} kind=reconfigure-pending identifier={identifier} owner={} cids={} mtu={mtu} mps={mps}",
            match owner {
                LinkDirection::CentralToPeripheral => "central",
                LinkDirection::PeripheralToCentral => "peripheral",
            },
            print_l2cap_channel_ids(&channel_ids)
        ),
        L2capCreditBasedEvent::ReconfigureRejected { identifier, result } => println!(
            "l2cap_credit_event index={index} kind=reconfigure-rejected identifier={identifier} result=0x{result:04x}"
        ),
        L2capCreditBasedEvent::Reconfigured {
            owner,
            channel_ids,
            mtu,
            mps,
        } => println!(
            "l2cap_credit_event index={index} kind=reconfigured owner={} cids={} mtu={mtu} mps={mps}",
            match owner {
                LinkDirection::CentralToPeripheral => "central",
                LinkDirection::PeripheralToCentral => "peripheral",
            },
            print_l2cap_channel_ids(&channel_ids)
        ),
        L2capCreditBasedEvent::DisconnectPending {
            identifier,
            central_channel_id,
            peripheral_channel_id,
        } => println!(
            "l2cap_credit_event index={index} kind=disconnect-pending identifier={identifier} central_cid=0x{central_channel_id:04x} peripheral_cid=0x{peripheral_channel_id:04x}"
        ),
        L2capCreditBasedEvent::Disconnected(channel) => println!(
            "l2cap_credit_event index={index} kind=disconnected {}",
            describe_credit_channel(&channel)
        ),
        L2capCreditBasedEvent::CommandRejected {
            identifier,
            removed_pending_procedure,
        } => println!(
            "l2cap_credit_event index={index} kind=command-rejected identifier={identifier} removed_pending={removed_pending_procedure}"
        ),
    }
    Ok(())
}

fn l2cap_trace(args: L2capTraceArgs) -> Result<()> {
    let mut tracker = L2capCreditBasedChannelTracker::default();
    let mut accepted = 0usize;
    let mut errors = 0usize;
    for (index, pdu) in args.pdus.iter().enumerate() {
        println!(
            "raw_l2cap_pdu index={index} direction={} cid=0x{:04x} payload={}",
            pdu.direction,
            pdu.channel_id,
            print_hex(&pdu.payload)
        );
        if pdu.channel_id == blueoxide::link_layer::LE_SIGNALING_CHANNEL_ID {
            match pdu.le_signaling_command() {
                Ok(Some(command)) => match describe_l2cap_signaling(command) {
                    Ok(description) => println!(
                        "l2cap_signal index={index} direction={} {description}",
                        pdu.direction
                    ),
                    Err(error) => eprintln!(
                        "l2cap signaling decode error: index={index} direction={} error={error}",
                        pdu.direction
                    ),
                },
                Ok(None) => {}
                Err(error) => eprintln!(
                    "l2cap signaling envelope error: index={index} direction={} error={error}",
                    pdu.direction
                ),
            }
        }
        match tracker.observe(pdu) {
            Ok(event) => {
                accepted += 1;
                print_l2cap_credit_event(index, event)?;
            }
            Err(error) => {
                errors += 1;
                eprintln!(
                    "l2cap credit observation error: index={index} direction={} cid=0x{:04x} error={error}",
                    pdu.direction, pdu.channel_id
                );
            }
        }
    }
    eprintln!(
        "processed {} directed L2CAP PDU(s); accepted={accepted} errors={errors} open_channels={}",
        args.pdus.len(),
        tracker.channels().count()
    );
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
            "{} device {:?} is not implemented; currently available: bladerf, limesdr, xtrx",
            args.frame.command_name(),
            args.device
        )));
    }
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
            let stats = capture_from_source(&mut source, &args, &radio_config)?;
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
            let stats = capture_from_source(&mut source, &args, &radio_config)?;
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
            let stats = capture_from_source(&mut source, &args, &radio_config)?;
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
) -> Result<CaptureStats> {
    let capture_start_ns = args
        .capture_start_ns
        .map(Ok)
        .unwrap_or_else(current_unix_time_ns)?;
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };
    let mut central_observation_tracker = args
        .central_observation_tracking
        .clone()
        .map(|config| FixedChannelCentralObservationTracker::new(args.channel, config))
        .transpose()?;
    let mut central_observation_matches = 0u64;
    let mut central_observation_errors = 0u64;

    let limits = CaptureLimits {
        maximum_samples: None,
        maximum_duration: Some(args.duration),
        read_timeout: Duration::from_millis(args.read_timeout_ms),
        block_samples: args.block_samples,
    };
    let stats = match args.frame {
        CaptureFrame::Advertising => capture_primary_advertising(
            source,
            radio_config,
            args.channel,
            Le1mDemodConfig {
                sample_rate_hz: args.sample_rate_hz,
                max_access_address_errors: args.max_access_address_errors,
            },
            limits,
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
        )?,
        CaptureFrame::Data {
            access_address,
            crc_init,
            phy,
        } => capture_data_channel(
            source,
            radio_config,
            args.channel,
            LeFrameConfig::data(access_address, crc_init)?,
            LeUncodedDemodConfig {
                phy,
                sample_rate_hz: args.sample_rate_hz,
                max_access_address_errors: args.max_access_address_errors,
            },
            limits,
            |captured: &CapturedDataChannelPdu| {
                let data = DataChannelPdu::from(captured.observation.pdu.clone());
                if let Err(error) = print_data_packet(&captured.observation, &data, true) {
                    eprintln!(
                        "live data-channel plaintext-hint decode error: channel={} sample={} error={error}",
                        data.channel.index(),
                        captured.observation.access_address_sample
                    );
                }
                if let Some(tracker) = &mut central_observation_tracker {
                    match tracker.observe_central(captured.observation.access_address_sample) {
                        Ok(observation) => {
                            central_observation_matches =
                                central_observation_matches.checked_add(1).ok_or_else(|| {
                                    Error::InvalidState(
                                        "central observation match count overflow".to_owned(),
                                    )
                                })?;
                            print_live_central_observation(
                                captured.observation.access_address_sample,
                                observation,
                            );
                        }
                        Err(error) => {
                            central_observation_errors =
                                central_observation_errors.checked_add(1).ok_or_else(|| {
                                    Error::InvalidState(
                                        "central observation error count overflow".to_owned(),
                                    )
                                })?;
                            eprintln!(
                                "central connection observation rejected: channel={} sample={} error={error}",
                                data.channel.index(),
                                captured.observation.access_address_sample
                            );
                        }
                    }
                }
                if let Some(writer) = &mut pcap {
                    let timestamp = sample_timestamp_ns(
                        capture_start_ns,
                        captured.relative_sample_index,
                        args.sample_rate_hz,
                    )?;
                    writer.write_le(&captured.observation, timestamp)?;
                }
                Ok(())
            },
        )?,
    };
    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    if args.central_observation_tracking.is_some() {
        eprintln!(
            "central connection observations: matched={central_observation_matches} rejected={central_observation_errors}"
        );
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
    let mut tracker = ConnectionTracker::new_with_phy(
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
        args.initial_phy,
    )?;
    if let Some(update) = args.phy_update {
        tracker.schedule_phy_update(update)?;
    }
    Ok(tracker)
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
            "event={} channel={} frequency_hz={} central_to_peripheral_phy={} peripheral_to_central_phy={} expected_sample={} earliest_sample={} latest_sample={} widening_samples={}",
            event.event_counter,
            event.channel.index(),
            event.channel.center_frequency_hz(),
            event.phy.central_to_peripheral,
            event.phy.peripheral_to_central,
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
    let timing = describe_sample_timing(observation.timing_error);
    println!(
        "event={} channel={} central_to_peripheral_phy={} peripheral_to_central_phy={} observed_sample={} advanced_events={} missed_events={} expected_sample={} timing={} earliest_sample={} latest_sample={} widening_samples={}",
        observation.event.event_counter,
        observation.event.channel.index(),
        observation.event.phy.central_to_peripheral,
        observation.event.phy.peripheral_to_central,
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

fn describe_sample_timing(timing_error: SampleTimingError) -> String {
    match timing_error {
        SampleTimingError::Early(samples) => format!("early:{samples}"),
        SampleTimingError::OnTime => "on-time:0".to_owned(),
        SampleTimingError::Late(samples) => format!("late:{samples}"),
    }
}

fn print_live_central_observation(
    observed_sample: u64,
    observation: blueoxide::link_layer::ConnectionObservation,
) {
    let missed_events = observation.advanced_events.saturating_sub(1);
    println!(
        "central_connection_event event={} channel={} observed_sample={} advanced_events={} missed_events={} expected_sample={} timing={} earliest_sample={} latest_sample={} widening_samples={}",
        observation.event.event_counter,
        observation.event.channel.index(),
        observed_sample,
        observation.advanced_events,
        missed_events,
        observation.timing_window.expected_sample,
        describe_sample_timing(observation.timing_error),
        observation.timing_window.earliest_sample,
        observation.timing_window.latest_sample,
        observation.timing_window.widening_samples
    );
}

fn connection_acquire(args: ConnectionPlanArgs) -> Result<()> {
    if args.initial_phy != ConnectionPhyState::default() {
        return Err(Error::InvalidConfiguration(
            "connection-acquire starts at event 0 on LE-1M in both directions; anchor PHY overrides are not accepted"
                .to_owned(),
        ));
    }
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
    if let Some(update) = args.phy_update {
        tracker.schedule_phy_update(update)?;
    }
    let event = tracker.current_event()?;
    println!(
        "event=0 channel={} central_to_peripheral_phy={} peripheral_to_central_phy={} central_sample={} connect_ind_sample={} nominal_start_sample={} nominal_end_sample={} earliest_sample={} latest_sample={} widening_samples={}",
        first_window.channel.index(),
        event.phy.central_to_peripheral,
        event.phy.peripheral_to_central,
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

fn print_extended_advertising_progress(
    packet_index: usize,
    kind: ExtendedAdvertisingPduKind,
    packet: &ExtendedAdvertisingPacketArg,
    progress: &ExtendedAdvertisingChainProgress,
) {
    match progress {
        ExtendedAdvertisingChainProgress::Awaiting {
            window,
            fragment_count,
            advertising_data_octets,
        } => println!(
            "packet={} kind={} channel={} phy={} sample={} status=awaiting fragments={} advertising_data_octets={} next_kind={} next_channel={} next_frequency_hz={} next_phy={} represented_earliest_sample={} represented_latest_sample={} earliest_sample={} latest_sample={} quantization_width_us={} quantization_width_samples={} widening_samples={}",
            packet_index,
            kind,
            packet.pdu.channel.index(),
            packet.phy,
            packet.access_address_sample,
            fragment_count,
            advertising_data_octets,
            window.expected_kind,
            window.channel.index(),
            window.channel.center_frequency_hz(),
            window.phy,
            window.represented_earliest_sample,
            window.represented_latest_sample,
            window.earliest_sample,
            window.latest_sample,
            window.quantization_width_us,
            window.quantization_width_samples,
            window.widening_samples,
        ),
        ExtendedAdvertisingChainProgress::Complete(chain) => {
            let advertiser = chain
                .advertiser_address
                .map(|address| address.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let advertiser_kind = chain
                .advertiser_address_kind
                .map(|kind| kind.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let adi = chain
                .advertising_data_info
                .map(|info| format!("sid:{}:did:{}", info.advertising_set_id, info.data_id))
                .unwrap_or_else(|| "none".to_owned());
            println!(
                "packet={} kind={} channel={} phy={} sample={} status=complete mode={} advertiser={} advertiser_type={} adi={} fragments={} first_auxiliary_sample={} last_auxiliary_sample={} advertising_data_octets={} advertising_data={}",
                packet_index,
                kind,
                packet.pdu.channel.index(),
                packet.phy,
                packet.access_address_sample,
                chain.mode,
                advertiser,
                advertiser_kind,
                adi,
                chain.fragment_count,
                chain.first_auxiliary_sample,
                chain.last_auxiliary_sample,
                chain.advertising_data.len(),
                print_hex(&chain.advertising_data),
            );
        }
    }
}

fn extended_advertising_plan(args: ExtendedAdvertisingPlanArgs) -> Result<()> {
    let mut tracker = ExtendedAdvertisingChainTracker::new(ExtendedAdvertisingChainConfig {
        sample_rate_hz: args.sample_rate_hz,
        receiver_clock_accuracy_ppm: args.receiver_clock_accuracy_ppm,
        maximum_advertising_data_length: args.maximum_advertising_data_length,
    })?;
    let mut packets = args.packets.into_iter();
    let primary = packets.next().ok_or_else(|| {
        Error::InvalidConfiguration(
            "extended-advertising-plan requires at least one --packet".to_owned(),
        )
    })?;
    let mut progress = tracker.begin(&primary.pdu, primary.phy, primary.access_address_sample)?;
    print_extended_advertising_progress(
        0,
        ExtendedAdvertisingPduKind::AdvExtInd,
        &primary,
        &progress,
    );

    for (offset, packet) in packets.enumerate() {
        let kind = match &progress {
            ExtendedAdvertisingChainProgress::Awaiting { window, .. } => window.expected_kind,
            ExtendedAdvertisingChainProgress::Complete(_) => {
                return Err(Error::InvalidInput(format!(
                    "packet {} follows an already complete extended advertising chain",
                    offset + 1
                )));
            }
        };
        progress = tracker.observe(&packet.pdu, packet.phy, packet.access_address_sample)?;
        print_extended_advertising_progress(offset + 1, kind, &packet, &progress);
    }
    Ok(())
}

fn print_periodic_advertising_event(index: usize, event: PeriodicAdvertisingEvent) {
    let window = event.timing_window;
    println!(
        "plan={} event={} channel={} frequency_hz={} phy={} represented_earliest_sample={} represented_latest_sample={} earliest_sample={} latest_sample={} quantization_width_samples={} widening_samples={}",
        index,
        event.event_counter,
        event.channel.index(),
        event.channel.center_frequency_hz(),
        event.phy,
        window.represented_earliest_sample,
        window.represented_latest_sample,
        window.earliest_sample,
        window.latest_sample,
        window.quantization_width_samples,
        window.widening_samples,
    );
}

fn periodic_advertising_plan(args: PeriodicAdvertisingPlanArgs) -> Result<()> {
    let decoded = decode_contextual_extended_advertising_pdu(
        &args.sync_packet.pdu,
        ExtendedAdvertisingPduKind::AuxAdvInd,
    )?;
    let sync_info = decoded.header.sync_info.ok_or_else(|| {
        Error::InvalidInput(
            "periodic-advertising-plan --sync-packet does not contain SyncInfo".to_owned(),
        )
    })?;
    println!(
        "sync_packet channel={} phy={} sample={} access_address={:08x} crc_init={:06x} interval_us={} event={} channel_map={} advertiser_sca_ppm={}",
        args.sync_packet.pdu.channel.index(),
        args.sync_packet.phy,
        args.sync_packet.access_address_sample,
        sync_info.access_address,
        sync_info.crc_init,
        sync_info.interval_us(),
        sync_info.event_counter,
        print_hex(&sync_info.channel_map.bytes()),
        sync_info.sleep_clock_accuracy.maximum_ppm(),
    );
    let mut tracker = PeriodicAdvertisingTracker::new(
        sync_info,
        args.sync_packet.phy,
        args.sync_packet.access_address_sample,
        PeriodicAdvertisingTrackerConfig {
            sample_rate_hz: args.sample_rate_hz,
            receiver_clock_accuracy_ppm: args.receiver_clock_accuracy_ppm,
        },
    )?;
    for (index, observed) in args.observations.into_iter().enumerate() {
        let observation = tracker.synchronize_observation(
            observed.channel,
            observed.phy,
            observed.access_address_sample,
            args.maximum_event_advance,
        )?;
        let window = observation.event.timing_window;
        println!(
            "observation={} event={} channel={} phy={} observed_sample={} advanced_events={} missed_events={} timing={} represented_earliest_sample={} represented_latest_sample={} earliest_sample={} latest_sample={} widening_samples={}",
            index,
            observation.event.event_counter,
            observation.event.channel.index(),
            observation.event.phy,
            observed.access_address_sample,
            observation.advanced_events,
            observation.advanced_events,
            describe_sample_timing(observation.timing_error),
            window.represented_earliest_sample,
            window.represented_latest_sample,
            window.earliest_sample,
            window.latest_sample,
            window.widening_samples,
        );
    }
    for index in 0..args.event_count {
        let event = if index == 0 {
            tracker.current_event()?
        } else {
            tracker.advance()?
        };
        print_periodic_advertising_event(index, event);
    }
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("decode") => decode(parse_decode_args(&args[1..])?),
        Some("decode-secondary") => decode_secondary(parse_decode_secondary_args(&args[1..])?),
        Some("decode-periodic") => decode_periodic(parse_decode_periodic_args(&args[1..])?),
        Some("decode-data") => decode_data(parse_decode_data_args(&args[1..])?),
        Some("encryption-trace") => encryption_trace(parse_encryption_trace_args(&args[1..])?),
        Some("l2cap-trace") => l2cap_trace(parse_l2cap_trace_args(&args[1..])?),
        Some("connection-plan") => connection_plan(parse_connection_plan_args(&args[1..])?),
        Some("connection-sync") => connection_sync(parse_connection_plan_args(&args[1..])?),
        Some("connection-acquire") => connection_acquire(parse_connection_plan_args(&args[1..])?),
        Some("extended-advertising-plan") => {
            extended_advertising_plan(parse_extended_advertising_plan_args(&args[1..])?)
        }
        Some("periodic-advertising-plan") => {
            periodic_advertising_plan(parse_periodic_advertising_plan_args(&args[1..])?)
        }
        Some("capture") => capture(parse_capture_args(&args[1..], CaptureCommand::Advertising)?),
        Some("capture-data") => capture(parse_capture_args(&args[1..], CaptureCommand::Data)?),
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
