use crate::ble::{AdvertisingPdu, BleChannel};
use crate::demod::Le1mDemodConfig;
use crate::link_layer::{
    ChannelSelectionAlgorithm, ConnectionChannelSelector, ConnectionParameters, ConnectionTracker,
    ConnectionTrackerConfig, ConstantToneExtensionInfo, DataChannelMap, LePhy, SleepClockAccuracy,
};
use crate::{Error, Result};
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressKind {
    Public,
    Random,
}

impl Display for AddressKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => formatter.write_str("public"),
            Self::Random => formatter.write_str("random"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeviceAddress(pub [u8; 6]);

impl DeviceAddress {
    fn from_air_bytes(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; 6] = bytes.try_into().map_err(|_| {
            Error::InvalidInput("Bluetooth device address must contain 6 octets".to_owned())
        })?;
        Ok(Self(bytes))
    }
}

impl Display for DeviceAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        for (index, byte) in self.0.iter().rev().enumerate() {
            if index != 0 {
                formatter.write_str(":")?;
            }
            write!(formatter, "{byte:02X}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtendedAdvertisingMode {
    NonConnectableNonScannable,
    ConnectableNonScannable,
    NonConnectableScannable,
}

impl ExtendedAdvertisingMode {
    fn parse(raw: u8) -> Result<Self> {
        match raw {
            0 => Ok(Self::NonConnectableNonScannable),
            1 => Ok(Self::ConnectableNonScannable),
            2 => Ok(Self::NonConnectableScannable),
            _ => Err(Error::InvalidInput(
                "extended advertising mode 3 is reserved".to_owned(),
            )),
        }
    }
}

impl Display for ExtendedAdvertisingMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonConnectableNonScannable => {
                formatter.write_str("non-connectable-non-scannable")
            }
            Self::ConnectableNonScannable => formatter.write_str("connectable-non-scannable"),
            Self::NonConnectableScannable => formatter.write_str("non-connectable-scannable"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdvertisingDataInfo {
    pub data_id: u16,
    pub advertising_set_id: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuxiliaryClockAccuracy {
    Ppm500,
    Ppm50,
}

impl AuxiliaryClockAccuracy {
    pub const fn maximum_ppm(self) -> u16 {
        match self {
            Self::Ppm500 => 500,
            Self::Ppm50 => 50,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuxiliaryPointer {
    pub channel: BleChannel,
    pub clock_accuracy: AuxiliaryClockAccuracy,
    pub offset_units_us: u16,
    pub offset: u16,
    pub phy: LePhy,
}

impl AuxiliaryPointer {
    pub const fn offset_us(self) -> u32 {
        self.offset as u32 * self.offset_units_us as u32
    }

    pub fn reception_window(
        self,
        parent: &AdvertisingPdu,
        parent_phy: LePhy,
        parent_access_address_sample: u64,
        sample_rate_hz: u32,
        receiver_clock_accuracy_ppm: u32,
        expected_kind: ExtendedAdvertisingPduKind,
    ) -> Result<AuxiliaryReceptionWindow> {
        if expected_kind == ExtendedAdvertisingPduKind::AdvExtInd {
            return Err(Error::InvalidConfiguration(
                "an AuxPtr cannot schedule a primary ADV_EXT_IND".to_owned(),
            ));
        }
        if parent.pdu_type() != 7 {
            return Err(Error::InvalidInput(format!(
                "AuxPtr scheduling requires advertising PDU type 0x07; got 0x{:02x}",
                parent.pdu_type()
            )));
        }
        if sample_rate_hz == 0 {
            return Err(Error::InvalidConfiguration(
                "AuxPtr scheduling sample rate must be greater than zero".to_owned(),
            ));
        }
        if receiver_clock_accuracy_ppm > 1_000_000 {
            return Err(Error::InvalidConfiguration(format!(
                "receiver clock accuracy {receiver_clock_accuracy_ppm} ppm exceeds 1000000"
            )));
        }
        if self.offset == 0 {
            return Err(Error::InvalidInput(
                "AuxPtr offset zero does not identify a schedulable auxiliary packet".to_owned(),
            ));
        }

        let parent_preamble_us = match parent_phy {
            LePhy::Le1M | LePhy::Le2M => 8u128,
            LePhy::LeCoded => {
                return Err(Error::InvalidConfiguration(
                    "AuxPtr scheduling from an LE Coded parent is not implemented".to_owned(),
                ));
            }
        };
        let parent_airtime_us = advertising_packet_airtime_us(parent, parent_phy)?;
        let offset_us = u128::from(self.offset_us());
        let quantization_width_us = u128::from(self.offset_units_us);
        let represented_packet_end_us = offset_us
            .checked_add(quantization_width_us)
            .ok_or_else(|| Error::InvalidState("AuxPtr represented offset overflow".to_owned()))?;
        let minimum_child_start_us = parent_airtime_us
            .checked_add(300)
            .ok_or_else(|| Error::InvalidState("AuxPtr minimum spacing overflow".to_owned()))?;
        if represented_packet_end_us < minimum_child_start_us {
            return Err(Error::InvalidInput(format!(
                "AuxPtr represented interval ends at {represented_packet_end_us} us, before parent airtime {parent_airtime_us} us plus 300 us MAFS"
            )));
        }

        let child_preamble_us = match self.phy {
            LePhy::Le1M | LePhy::Le2M => 8u128,
            LePhy::LeCoded => 80u128,
        };
        let preamble_adjustment_us = child_preamble_us
            .checked_sub(parent_preamble_us)
            .ok_or_else(|| {
                Error::InvalidState("AuxPtr child preamble precedes the parent preamble".to_owned())
            })?;
        let represented_earliest_us = offset_us
            .checked_add(preamble_adjustment_us)
            .ok_or_else(|| Error::InvalidState("AuxPtr earliest-time overflow".to_owned()))?;
        let represented_latest_us = represented_packet_end_us
            .checked_add(preamble_adjustment_us)
            .ok_or_else(|| Error::InvalidState("AuxPtr latest-time overflow".to_owned()))?;

        let represented_earliest_sample = parent_access_address_sample
            .checked_add(microseconds_to_samples_floor(
                represented_earliest_us,
                sample_rate_hz,
            )?)
            .ok_or_else(|| Error::InvalidState("AuxPtr earliest sample exceeds u64".to_owned()))?;
        let represented_latest_sample = parent_access_address_sample
            .checked_add(microseconds_to_samples_ceil(
                represented_latest_us,
                sample_rate_hz,
            )?)
            .ok_or_else(|| Error::InvalidState("AuxPtr latest sample exceeds u64".to_owned()))?;
        let combined_ppm = u128::from(receiver_clock_accuracy_ppm)
            .checked_add(u128::from(self.clock_accuracy.maximum_ppm()))
            .ok_or_else(|| Error::InvalidState("AuxPtr clock-accuracy overflow".to_owned()))?;
        let widening_numerator = offset_us
            .checked_mul(combined_ppm)
            .and_then(|value| value.checked_mul(u128::from(sample_rate_hz)))
            .ok_or_else(|| {
                Error::InvalidState("AuxPtr timing-window arithmetic overflow".to_owned())
            })?;
        let widening_samples = divide_round_up(widening_numerator, 1_000_000_000_000)?;
        let quantization_width_samples =
            microseconds_to_samples_ceil(quantization_width_us, sample_rate_hz)?;

        Ok(AuxiliaryReceptionWindow {
            expected_kind,
            channel: self.channel,
            phy: self.phy,
            represented_earliest_sample,
            represented_latest_sample,
            earliest_sample: represented_earliest_sample.saturating_sub(widening_samples),
            latest_sample: represented_latest_sample
                .checked_add(widening_samples)
                .ok_or_else(|| {
                    Error::InvalidState("AuxPtr widened latest sample exceeds u64".to_owned())
                })?,
            quantization_width_us: self.offset_units_us,
            quantization_width_samples,
            widening_samples,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtendedAdvertisingPduKind {
    AdvExtInd,
    AuxAdvInd,
    AuxSyncInd,
    AuxChainInd,
}

impl Display for ExtendedAdvertisingPduKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdvExtInd => formatter.write_str("ADV_EXT_IND"),
            Self::AuxAdvInd => formatter.write_str("AUX_ADV_IND"),
            Self::AuxSyncInd => formatter.write_str("AUX_SYNC_IND"),
            Self::AuxChainInd => formatter.write_str("AUX_CHAIN_IND"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualExtendedAdvertisingPdu {
    pub kind: ExtendedAdvertisingPduKind,
    pub header: ExtendedAdvertisingHeader,
}

impl Display for ContextualExtendedAdvertisingPdu {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        format_extended_advertising(formatter, self.kind, &self.header)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuxiliaryReceptionWindow {
    pub expected_kind: ExtendedAdvertisingPduKind,
    pub channel: BleChannel,
    pub phy: LePhy,
    pub represented_earliest_sample: u64,
    pub represented_latest_sample: u64,
    pub earliest_sample: u64,
    pub latest_sample: u64,
    pub quantization_width_us: u16,
    pub quantization_width_samples: u64,
    pub widening_samples: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtendedAdvertisingChainConfig {
    pub sample_rate_hz: u32,
    pub receiver_clock_accuracy_ppm: u32,
    pub maximum_advertising_data_length: usize,
}

impl ExtendedAdvertisingChainConfig {
    pub fn validate(&self) -> Result<()> {
        if self.sample_rate_hz == 0 {
            return Err(Error::InvalidConfiguration(
                "extended advertising tracker sample rate must be greater than zero".to_owned(),
            ));
        }
        if self.receiver_clock_accuracy_ppm > 1_000_000 {
            return Err(Error::InvalidConfiguration(format!(
                "receiver clock accuracy {} ppm exceeds 1000000",
                self.receiver_clock_accuracy_ppm
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReassembledExtendedAdvertising {
    pub advertiser_address: Option<DeviceAddress>,
    pub advertiser_address_kind: Option<AddressKind>,
    pub advertising_data_info: Option<AdvertisingDataInfo>,
    pub mode: ExtendedAdvertisingMode,
    pub advertising_data: Vec<u8>,
    pub fragment_count: usize,
    pub first_auxiliary_sample: u64,
    pub last_auxiliary_sample: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtendedAdvertisingChainProgress {
    Awaiting {
        window: AuxiliaryReceptionWindow,
        fragment_count: usize,
        advertising_data_octets: usize,
    },
    Complete(ReassembledExtendedAdvertising),
}

#[derive(Clone, Debug)]
struct ActiveExtendedAdvertisingChain {
    pending: AuxiliaryReceptionWindow,
    advertiser_address: Option<DeviceAddress>,
    advertiser_address_kind: Option<AddressKind>,
    advertising_data_info: Option<AdvertisingDataInfo>,
    mode: Option<ExtendedAdvertisingMode>,
    advertising_data: Vec<u8>,
    fragment_count: usize,
    first_auxiliary_sample: Option<u64>,
    last_auxiliary_sample: Option<u64>,
}

#[derive(Clone, Debug)]
enum ExtendedAdvertisingChainState {
    Idle,
    Active(ActiveExtendedAdvertisingChain),
    Complete(ReassembledExtendedAdvertising),
}

#[derive(Clone, Debug)]
pub struct ExtendedAdvertisingChainTracker {
    config: ExtendedAdvertisingChainConfig,
    state: ExtendedAdvertisingChainState,
}

impl ExtendedAdvertisingChainTracker {
    pub fn new(config: ExtendedAdvertisingChainConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            state: ExtendedAdvertisingChainState::Idle,
        })
    }

    pub fn reset(&mut self) {
        self.state = ExtendedAdvertisingChainState::Idle;
    }

    pub fn progress(&self) -> Option<ExtendedAdvertisingChainProgress> {
        match &self.state {
            ExtendedAdvertisingChainState::Idle => None,
            ExtendedAdvertisingChainState::Active(active) => {
                Some(ExtendedAdvertisingChainProgress::Awaiting {
                    window: active.pending,
                    fragment_count: active.fragment_count,
                    advertising_data_octets: active.advertising_data.len(),
                })
            }
            ExtendedAdvertisingChainState::Complete(chain) => {
                Some(ExtendedAdvertisingChainProgress::Complete(chain.clone()))
            }
        }
    }

    pub fn begin(
        &mut self,
        primary: &AdvertisingPdu,
        phy: LePhy,
        access_address_sample: u64,
    ) -> Result<ExtendedAdvertisingChainProgress> {
        if !matches!(self.state, ExtendedAdvertisingChainState::Idle) {
            return Err(Error::InvalidState(
                "extended advertising tracker must be reset before beginning another chain"
                    .to_owned(),
            ));
        }
        if !primary.channel.is_primary_advertising() {
            return Err(Error::InvalidInput(format!(
                "extended advertising chain must begin on primary channel 37..=39; got {}",
                primary.channel.index()
            )));
        }
        if phy != LePhy::Le1M {
            return Err(Error::InvalidInput(format!(
                "primary ADV_EXT_IND must use LE-1M; got {phy}"
            )));
        }
        let header = extended_header(primary, ExtendedAdvertisingPduKind::AdvExtInd)?;
        let pointer = header.auxiliary_pointer.ok_or_else(|| {
            Error::InvalidInput(
                "primary ADV_EXT_IND does not contain an AuxPtr to follow".to_owned(),
            )
        })?;
        let pending = pointer.reception_window(
            primary,
            phy,
            access_address_sample,
            self.config.sample_rate_hz,
            self.config.receiver_clock_accuracy_ppm,
            ExtendedAdvertisingPduKind::AuxAdvInd,
        )?;
        self.state = ExtendedAdvertisingChainState::Active(ActiveExtendedAdvertisingChain {
            pending,
            advertiser_address: header.advertiser_address,
            advertiser_address_kind: header.advertiser_address_kind,
            advertising_data_info: header.advertising_data_info,
            mode: None,
            advertising_data: Vec::new(),
            fragment_count: 0,
            first_auxiliary_sample: None,
            last_auxiliary_sample: None,
        });
        self.progress().ok_or_else(|| {
            Error::InvalidState("extended advertising tracker failed to start".to_owned())
        })
    }

    pub fn observe(
        &mut self,
        pdu: &AdvertisingPdu,
        phy: LePhy,
        access_address_sample: u64,
    ) -> Result<ExtendedAdvertisingChainProgress> {
        let ExtendedAdvertisingChainState::Active(active) = &self.state else {
            return Err(Error::InvalidState(
                "extended advertising tracker is not awaiting an auxiliary packet".to_owned(),
            ));
        };
        let mut candidate = active.clone();
        let pending = candidate.pending;
        if pdu.channel != pending.channel {
            return Err(Error::InvalidInput(format!(
                "{} expected channel {}, received {}",
                pending.expected_kind,
                pending.channel.index(),
                pdu.channel.index()
            )));
        }
        if phy != pending.phy {
            return Err(Error::InvalidInput(format!(
                "{} expected PHY {}, received {phy}",
                pending.expected_kind, pending.phy
            )));
        }
        if !(pending.earliest_sample..=pending.latest_sample).contains(&access_address_sample) {
            return Err(Error::InvalidInput(format!(
                "{} sample {access_address_sample} is outside {}..={}",
                pending.expected_kind, pending.earliest_sample, pending.latest_sample
            )));
        }
        let header = extended_header(pdu, pending.expected_kind)?;
        validate_contextual_extended_header(pending.expected_kind, &header)?;

        match (
            candidate.advertiser_address,
            header.advertiser_address,
            candidate.advertiser_address_kind,
            header.advertiser_address_kind,
        ) {
            (Some(expected), Some(observed), Some(expected_kind), Some(observed_kind))
                if expected != observed || expected_kind != observed_kind =>
            {
                return Err(Error::InvalidInput(format!(
                    "{} advertiser identity does not match the initiating ADV_EXT_IND",
                    pending.expected_kind
                )));
            }
            (None, Some(observed), _, Some(observed_kind)) => {
                candidate.advertiser_address = Some(observed);
                candidate.advertiser_address_kind = Some(observed_kind);
            }
            _ => {}
        }

        match (
            candidate.advertising_data_info,
            header.advertising_data_info,
        ) {
            (Some(expected), Some(observed)) if expected != observed => {
                return Err(Error::InvalidInput(format!(
                    "{} ADI SID/DID does not match the initiating chain",
                    pending.expected_kind
                )));
            }
            (Some(_), None) => {
                return Err(Error::InvalidInput(format!(
                    "{} omits the chain ADI",
                    pending.expected_kind
                )));
            }
            (None, Some(observed)) => candidate.advertising_data_info = Some(observed),
            (None, None) | (Some(_), Some(_)) => {}
        }

        if header.auxiliary_pointer.is_some() && candidate.advertising_data_info.is_none() {
            return Err(Error::InvalidInput(format!(
                "{} with a continuation AuxPtr requires an ADI",
                pending.expected_kind
            )));
        }

        let assembled_length = candidate
            .advertising_data
            .len()
            .checked_add(header.advertising_data.len())
            .ok_or_else(|| {
                Error::InvalidState(
                    "extended advertising data length arithmetic overflow".to_owned(),
                )
            })?;
        if assembled_length > self.config.maximum_advertising_data_length {
            return Err(Error::InvalidInput(format!(
                "extended advertising data would reach {assembled_length} octets, exceeding configured maximum {}",
                self.config.maximum_advertising_data_length
            )));
        }
        candidate
            .advertising_data
            .extend_from_slice(&header.advertising_data);
        candidate.fragment_count = candidate.fragment_count.checked_add(1).ok_or_else(|| {
            Error::InvalidState("extended advertising fragment count overflow".to_owned())
        })?;
        candidate.first_auxiliary_sample = candidate
            .first_auxiliary_sample
            .or(Some(access_address_sample));
        candidate.last_auxiliary_sample = Some(access_address_sample);
        if pending.expected_kind == ExtendedAdvertisingPduKind::AuxAdvInd {
            candidate.mode = Some(header.mode);
        }

        let progress = if let Some(pointer) = header.auxiliary_pointer {
            candidate.pending = pointer.reception_window(
                pdu,
                phy,
                access_address_sample,
                self.config.sample_rate_hz,
                self.config.receiver_clock_accuracy_ppm,
                ExtendedAdvertisingPduKind::AuxChainInd,
            )?;
            let progress = ExtendedAdvertisingChainProgress::Awaiting {
                window: candidate.pending,
                fragment_count: candidate.fragment_count,
                advertising_data_octets: candidate.advertising_data.len(),
            };
            self.state = ExtendedAdvertisingChainState::Active(candidate);
            progress
        } else {
            let chain = ReassembledExtendedAdvertising {
                advertiser_address: candidate.advertiser_address,
                advertiser_address_kind: candidate.advertiser_address_kind,
                advertising_data_info: candidate.advertising_data_info,
                mode: candidate.mode.ok_or_else(|| {
                    Error::InvalidState(
                        "extended advertising chain completed before AUX_ADV_IND".to_owned(),
                    )
                })?,
                advertising_data: candidate.advertising_data,
                fragment_count: candidate.fragment_count,
                first_auxiliary_sample: candidate.first_auxiliary_sample.ok_or_else(|| {
                    Error::InvalidState(
                        "extended advertising chain has no first auxiliary sample".to_owned(),
                    )
                })?,
                last_auxiliary_sample: candidate.last_auxiliary_sample.ok_or_else(|| {
                    Error::InvalidState(
                        "extended advertising chain has no last auxiliary sample".to_owned(),
                    )
                })?,
            };
            self.state = ExtendedAdvertisingChainState::Complete(chain.clone());
            ExtendedAdvertisingChainProgress::Complete(chain)
        };
        Ok(progress)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodicAdvertisingSyncInfo {
    pub packet_offset: u16,
    pub offset_units_us: u16,
    pub offset_adjust: bool,
    pub interval: u16,
    pub channel_map: DataChannelMap,
    pub sleep_clock_accuracy: SleepClockAccuracy,
    pub access_address: u32,
    pub crc_init: u32,
    pub event_counter: u16,
}

impl PeriodicAdvertisingSyncInfo {
    pub const fn packet_offset_us(&self) -> u32 {
        self.packet_offset as u32 * self.offset_units_us as u32
            + if self.offset_adjust { 2_457_600 } else { 0 }
    }

    pub const fn interval_us(&self) -> u32 {
        self.interval as u32 * 1_250
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtendedAdvertisingHeader {
    pub extended_header_length: u8,
    pub mode: ExtendedAdvertisingMode,
    pub flags: u8,
    pub advertiser_address: Option<DeviceAddress>,
    pub advertiser_address_kind: Option<AddressKind>,
    pub target_address: Option<DeviceAddress>,
    pub target_address_kind: Option<AddressKind>,
    pub constant_tone_extension_info: Option<ConstantToneExtensionInfo>,
    pub advertising_data_info: Option<AdvertisingDataInfo>,
    pub auxiliary_pointer: Option<AuxiliaryPointer>,
    pub sync_info: Option<PeriodicAdvertisingSyncInfo>,
    pub tx_power_dbm: Option<i8>,
    pub additional_controller_advertising_data: Vec<u8>,
    pub advertising_data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdStructure {
    pub ad_type: u8,
    pub data: Vec<u8>,
}

impl AdStructure {
    pub fn type_name(&self) -> &'static str {
        match self.ad_type {
            0x01 => "Flags",
            0x02 => "Incomplete 16-bit Service UUIDs",
            0x03 => "Complete 16-bit Service UUIDs",
            0x04 => "Incomplete 32-bit Service UUIDs",
            0x05 => "Complete 32-bit Service UUIDs",
            0x06 => "Incomplete 128-bit Service UUIDs",
            0x07 => "Complete 128-bit Service UUIDs",
            0x08 => "Shortened Local Name",
            0x09 => "Complete Local Name",
            0x0a => "TX Power Level",
            0x16 => "Service Data - 16-bit UUID",
            0x20 => "Service Data - 32-bit UUID",
            0x21 => "Service Data - 128-bit UUID",
            0xff => "Manufacturer Specific Data",
            _ => "Unknown",
        }
    }

    pub fn text(&self) -> Option<&str> {
        if matches!(self.ad_type, 0x08 | 0x09) {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }
}

pub fn parse_ad_structures(bytes: &[u8]) -> Result<Vec<AdStructure>> {
    let mut structures = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let length = bytes[offset] as usize;
        offset += 1;
        if length == 0 {
            break;
        }
        let end = offset.checked_add(length).ok_or_else(|| {
            Error::InvalidInput("advertising data structure length overflow".to_owned())
        })?;
        if end > bytes.len() {
            return Err(Error::InvalidInput(format!(
                "advertising data structure declares {length} octets with only {} remaining",
                bytes.len() - offset
            )));
        }
        structures.push(AdStructure {
            ad_type: bytes[offset],
            data: bytes[offset + 1..end].to_vec(),
        });
        offset = end;
    }
    Ok(structures)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectRequest {
    pub access_address: u32,
    pub crc_init: u32,
    pub window_size: u8,
    pub window_offset: u16,
    pub interval: u16,
    pub latency: u16,
    pub supervision_timeout: u16,
    pub channel_map: [u8; 5],
    pub hop_increment: u8,
    pub sleep_clock_accuracy: u8,
    pub channel_selection_algorithm: ChannelSelectionAlgorithm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirstConnectionEventWindow {
    pub channel: BleChannel,
    pub nominal_start_sample: u64,
    pub nominal_end_sample: u64,
    pub earliest_sample: u64,
    pub latest_sample: u64,
    pub widening_samples: u64,
}

/// A caller-identified central transmission at the start of connection event 0.
///
/// Blueoxide does not infer packet direction from an isolated data-channel PDU.
/// Callers must only construct this value for a CRC-valid transmission known to
/// have come from the central, not for a peripheral response in the same event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirstCentralTransmission {
    channel: BleChannel,
    access_address_sample: u64,
}

impl FirstCentralTransmission {
    pub fn new(channel: BleChannel, access_address_sample: u64) -> Result<Self> {
        if channel.index() > 36 {
            return Err(Error::InvalidInput(format!(
                "first central transmission requires a data channel in 0..=36; got {}",
                channel.index()
            )));
        }
        Ok(Self {
            channel,
            access_address_sample,
        })
    }

    pub const fn channel(self) -> BleChannel {
        self.channel
    }

    pub const fn access_address_sample(self) -> u64 {
        self.access_address_sample
    }
}

impl ConnectRequest {
    pub fn interval_us(&self) -> u32 {
        self.interval as u32 * 1_250
    }

    pub fn supervision_timeout_us(&self) -> u32 {
        self.supervision_timeout as u32 * 10_000
    }

    pub fn transmit_window_start_us(&self) -> u32 {
        (self.window_offset as u32 + 1) * 1_250
    }

    pub fn transmit_window_size_us(&self) -> u32 {
        self.window_size as u32 * 1_250
    }

    pub fn transmit_window_end_us(&self) -> u32 {
        self.transmit_window_start_us() + self.transmit_window_size_us()
    }

    pub fn event_offset_from_anchor_us(&self, event_counter: u16) -> u64 {
        event_counter as u64 * self.interval_us() as u64
    }

    pub fn enabled_data_channels(&self) -> Vec<u8> {
        (0..=36)
            .filter(|channel| self.channel_map[*channel as usize / 8] & (1 << (*channel % 8)) != 0)
            .collect()
    }

    pub fn channel_selector(&self) -> Result<ConnectionChannelSelector> {
        ConnectionChannelSelector::new(
            self.channel_selection_algorithm,
            DataChannelMap::new(self.channel_map)?,
            self.access_address,
            self.hop_increment,
        )
    }

    pub fn connection_parameters(&self) -> Result<ConnectionParameters> {
        ConnectionParameters::new(self.interval, self.latency, self.supervision_timeout)
    }

    pub fn peer_clock_accuracy(&self) -> Result<SleepClockAccuracy> {
        SleepClockAccuracy::new(self.sleep_clock_accuracy)
    }

    pub fn first_event_window(
        &self,
        connect_ind_access_address_sample: u64,
        sample_rate_hz: u32,
        receiver_clock_accuracy_ppm: u32,
    ) -> Result<FirstConnectionEventWindow> {
        self.connection_parameters()?;
        if !(1..=16).contains(&self.window_size) {
            return Err(Error::InvalidInput(format!(
                "CONNECT_IND window size {} is outside 1..=16",
                self.window_size
            )));
        }
        if self.window_offset > self.interval {
            return Err(Error::InvalidInput(format!(
                "CONNECT_IND window offset {} exceeds interval {}",
                self.window_offset, self.interval
            )));
        }
        let samples_per_symbol = Le1mDemodConfig {
            sample_rate_hz,
            max_access_address_errors: 0,
        }
        .validate()?;
        if receiver_clock_accuracy_ppm > 1_000_000 {
            return Err(Error::InvalidConfiguration(format!(
                "receiver clock accuracy {receiver_clock_accuracy_ppm} ppm exceeds 1000000"
            )));
        }

        const CONNECT_IND_BITS_FROM_ACCESS_ADDRESS: u64 = (4 + 2 + 34 + 3) * 8;
        let connect_ind_end_sample = connect_ind_access_address_sample
            .checked_add(
                CONNECT_IND_BITS_FROM_ACCESS_ADDRESS
                    .checked_mul(samples_per_symbol as u64)
                    .ok_or_else(|| {
                        Error::InvalidInput("CONNECT_IND sample-length overflow".to_owned())
                    })?,
            )
            .ok_or_else(|| Error::InvalidInput("CONNECT_IND end sample overflow".to_owned()))?;
        let nominal_start_sample = connect_ind_end_sample
            .checked_add(
                u64::from(self.transmit_window_start_us())
                    .checked_mul(u64::from(sample_rate_hz))
                    .ok_or_else(|| {
                        Error::InvalidInput(
                            "first transmit-window start arithmetic overflow".to_owned(),
                        )
                    })?
                    / 1_000_000,
            )
            .ok_or_else(|| {
                Error::InvalidInput("first transmit-window start exceeds u64".to_owned())
            })?;
        let nominal_end_sample = connect_ind_end_sample
            .checked_add(
                u64::from(self.transmit_window_end_us())
                    .checked_mul(u64::from(sample_rate_hz))
                    .ok_or_else(|| {
                        Error::InvalidInput(
                            "first transmit-window end arithmetic overflow".to_owned(),
                        )
                    })?
                    / 1_000_000,
            )
            .ok_or_else(|| {
                Error::InvalidInput("first transmit-window end exceeds u64".to_owned())
            })?;
        let combined_ppm = u128::from(
            receiver_clock_accuracy_ppm + u32::from(self.peer_clock_accuracy()?.maximum_ppm()),
        );
        let widening_numerator = u128::from(self.transmit_window_end_us())
            .checked_mul(combined_ppm)
            .and_then(|value| value.checked_mul(u128::from(sample_rate_hz)))
            .ok_or_else(|| {
                Error::InvalidInput("first transmit-window widening overflow".to_owned())
            })?;
        let widening_samples = u64::try_from(widening_numerator.div_ceil(1_000_000_000_000))
            .map_err(|_| {
                Error::InvalidInput("first transmit-window widening exceeds u64".to_owned())
            })?;
        Ok(FirstConnectionEventWindow {
            channel: self.channel_selector()?.channel_for_event(0),
            nominal_start_sample,
            nominal_end_sample,
            earliest_sample: nominal_start_sample.saturating_sub(widening_samples),
            latest_sample: nominal_end_sample
                .checked_add(widening_samples)
                .ok_or_else(|| {
                    Error::InvalidInput("first transmit-window bound exceeds u64".to_owned())
                })?,
            widening_samples,
        })
    }

    pub fn acquire_first_event_anchor(
        &self,
        connect_ind_access_address_sample: u64,
        sample_rate_hz: u32,
        receiver_clock_accuracy_ppm: u32,
        observed_central: FirstCentralTransmission,
    ) -> Result<ConnectionTracker> {
        let window = self.first_event_window(
            connect_ind_access_address_sample,
            sample_rate_hz,
            receiver_clock_accuracy_ppm,
        )?;
        if observed_central.channel() != window.channel {
            return Err(Error::InvalidInput(format!(
                "first connection event was expected on channel {}, observed channel {}",
                window.channel.index(),
                observed_central.channel().index()
            )));
        }
        if !(window.earliest_sample..=window.latest_sample)
            .contains(&observed_central.access_address_sample())
        {
            return Err(Error::InvalidInput(format!(
                "first central connection-event sample {} is outside {}..={}",
                observed_central.access_address_sample(),
                window.earliest_sample,
                window.latest_sample
            )));
        }
        self.connection_tracker(sample_rate_hz, 0, observed_central.access_address_sample())
    }

    pub fn connection_tracker(
        &self,
        sample_rate_hz: u32,
        observed_event_counter: u16,
        observed_access_address_sample: u64,
    ) -> Result<ConnectionTracker> {
        ConnectionTracker::new(
            ConnectionTrackerConfig {
                access_address: self.access_address,
                channel_selection_algorithm: self.channel_selection_algorithm,
                hop_increment: self.hop_increment,
                channel_map: DataChannelMap::new(self.channel_map)?,
                parameters: self.connection_parameters()?,
                sample_rate_hz,
            },
            observed_event_counter,
            observed_access_address_sample,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedAdvertisingPdu {
    AdvInd {
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        data: Vec<AdStructure>,
    },
    AdvDirectInd {
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        target: DeviceAddress,
        target_kind: AddressKind,
    },
    AdvNonconnInd {
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        data: Vec<AdStructure>,
    },
    ScanReq {
        scanner: DeviceAddress,
        scanner_kind: AddressKind,
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
    },
    ScanRsp {
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        data: Vec<AdStructure>,
    },
    ConnectInd {
        initiator: DeviceAddress,
        initiator_kind: AddressKind,
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        request: ConnectRequest,
    },
    AdvScanInd {
        advertiser: DeviceAddress,
        advertiser_kind: AddressKind,
        data: Vec<AdStructure>,
    },
    AdvExtInd {
        header: ExtendedAdvertisingHeader,
    },
    ExtendedOrReserved {
        pdu_type: u8,
        payload: Vec<u8>,
    },
}

impl Display for DecodedAdvertisingPdu {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdvInd {
                advertiser,
                advertiser_kind,
                data,
            } => write!(
                formatter,
                "ADV_IND advertiser={advertiser} address_type={advertiser_kind} ad_structures={}",
                data.len()
            ),
            Self::AdvDirectInd {
                advertiser,
                advertiser_kind,
                target,
                target_kind,
            } => write!(
                formatter,
                "ADV_DIRECT_IND advertiser={advertiser} advertiser_type={advertiser_kind} target={target} target_type={target_kind}"
            ),
            Self::AdvNonconnInd {
                advertiser,
                advertiser_kind,
                data,
            } => write!(
                formatter,
                "ADV_NONCONN_IND advertiser={advertiser} address_type={advertiser_kind} ad_structures={}",
                data.len()
            ),
            Self::ScanReq {
                scanner,
                scanner_kind,
                advertiser,
                advertiser_kind,
            } => write!(
                formatter,
                "SCAN_REQ scanner={scanner} scanner_type={scanner_kind} advertiser={advertiser} advertiser_type={advertiser_kind}"
            ),
            Self::ScanRsp {
                advertiser,
                advertiser_kind,
                data,
            } => write!(
                formatter,
                "SCAN_RSP advertiser={advertiser} address_type={advertiser_kind} ad_structures={}",
                data.len()
            ),
            Self::ConnectInd {
                initiator,
                initiator_kind,
                advertiser,
                advertiser_kind,
                request,
            } => write!(
                formatter,
                "CONNECT_IND initiator={initiator} initiator_type={initiator_kind} advertiser={advertiser} advertiser_type={advertiser_kind} access_address={:08x} interval_us={} latency={} timeout_us={} hop={} channels={} channel_selection={}",
                request.access_address,
                request.interval_us(),
                request.latency,
                request.supervision_timeout_us(),
                request.hop_increment,
                request.enabled_data_channels().len(),
                request.channel_selection_algorithm
            ),
            Self::AdvScanInd {
                advertiser,
                advertiser_kind,
                data,
            } => write!(
                formatter,
                "ADV_SCAN_IND advertiser={advertiser} address_type={advertiser_kind} ad_structures={}",
                data.len()
            ),
            Self::AdvExtInd { header } => format_extended_advertising(
                formatter,
                ExtendedAdvertisingPduKind::AdvExtInd,
                header,
            ),
            Self::ExtendedOrReserved { pdu_type, payload } => write!(
                formatter,
                "PDU_TYPE_{pdu_type} undecoded_payload_octets={}",
                payload.len()
            ),
        }
    }
}

fn format_extended_advertising(
    formatter: &mut Formatter<'_>,
    kind: ExtendedAdvertisingPduKind,
    header: &ExtendedAdvertisingHeader,
) -> std::fmt::Result {
    write!(
        formatter,
        "{kind} mode={} ext_header_octets={} flags=0x{:02x}",
        header.mode, header.extended_header_length, header.flags
    )?;
    if let (Some(address), Some(address_kind)) =
        (header.advertiser_address, header.advertiser_address_kind)
    {
        write!(
            formatter,
            " advertiser={address} advertiser_type={address_kind}"
        )?;
    }
    if let (Some(address), Some(address_kind)) = (header.target_address, header.target_address_kind)
    {
        write!(formatter, " target={address} target_type={address_kind}")?;
    }
    if let Some(info) = header.advertising_data_info {
        write!(
            formatter,
            " sid={} did={}",
            info.advertising_set_id, info.data_id
        )?;
    }
    if let Some(pointer) = header.auxiliary_pointer {
        write!(
            formatter,
            " aux_channel={} aux_offset_us={} aux_phy={}",
            pointer.channel.index(),
            pointer.offset_us(),
            pointer.phy
        )?;
    }
    if let Some(sync) = &header.sync_info {
        write!(
            formatter,
            " sync_offset_us={} sync_interval_us={} sync_event={}",
            sync.packet_offset_us(),
            sync.interval_us(),
            sync.event_counter
        )?;
    }
    if let Some(tx_power_dbm) = header.tx_power_dbm {
        write!(formatter, " tx_power_dbm={tx_power_dbm}")?;
    }
    write!(
        formatter,
        " acad_octets={} advertising_data_octets={}",
        header.additional_controller_advertising_data.len(),
        header.advertising_data.len()
    )
}

fn address_kind(random: bool) -> AddressKind {
    if random {
        AddressKind::Random
    } else {
        AddressKind::Public
    }
}

fn require_payload_length(pdu: &AdvertisingPdu, expected: usize, name: &str) -> Result<()> {
    if pdu.payload.len() == expected {
        Ok(())
    } else {
        Err(Error::InvalidInput(format!(
            "{name} requires {expected} payload octets, received {}",
            pdu.payload.len()
        )))
    }
}

fn decode_advertiser_and_data(
    pdu: &AdvertisingPdu,
) -> Result<(DeviceAddress, AddressKind, Vec<AdStructure>)> {
    if pdu.payload.len() < 6 {
        return Err(Error::InvalidInput(format!(
            "advertising PDU type {} requires a 6-octet advertiser address",
            pdu.pdu_type()
        )));
    }
    Ok((
        DeviceAddress::from_air_bytes(&pdu.payload[..6])?,
        address_kind(pdu.tx_add_random()),
        parse_ad_structures(&pdu.payload[6..])?,
    ))
}

fn take_extended_field<'a>(
    remaining: &mut &'a [u8],
    length: usize,
    name: &str,
) -> Result<&'a [u8]> {
    if remaining.len() < length {
        return Err(Error::InvalidInput(format!(
            "ADV_EXT_IND {name} requires {length} octets with only {} remaining in the extended header",
            remaining.len()
        )));
    }
    let (field, rest) = remaining.split_at(length);
    *remaining = rest;
    Ok(field)
}

fn extended_header(
    pdu: &AdvertisingPdu,
    kind: ExtendedAdvertisingPduKind,
) -> Result<ExtendedAdvertisingHeader> {
    if pdu.pdu_type() != 7 {
        return Err(Error::InvalidInput(format!(
            "{kind} requires advertising PDU type 0x07; got 0x{:02x}",
            pdu.pdu_type()
        )));
    }
    match kind {
        ExtendedAdvertisingPduKind::AdvExtInd if !pdu.channel.is_primary_advertising() => {
            return Err(Error::InvalidInput(format!(
                "ADV_EXT_IND requires primary channel 37..=39; got {}",
                pdu.channel.index()
            )));
        }
        ExtendedAdvertisingPduKind::AuxAdvInd
        | ExtendedAdvertisingPduKind::AuxSyncInd
        | ExtendedAdvertisingPduKind::AuxChainInd
            if pdu.channel.is_primary_advertising() =>
        {
            return Err(Error::InvalidInput(format!(
                "{kind} requires a secondary channel in 0..=36; got {}",
                pdu.channel.index()
            )));
        }
        _ => {}
    }
    decode_extended_advertising_header(pdu)
}

fn validate_contextual_extended_header(
    kind: ExtendedAdvertisingPduKind,
    header: &ExtendedAdvertisingHeader,
) -> Result<()> {
    match kind {
        ExtendedAdvertisingPduKind::AdvExtInd | ExtendedAdvertisingPduKind::AuxSyncInd => {}
        ExtendedAdvertisingPduKind::AuxAdvInd => {
            if header.auxiliary_pointer.is_some()
                && header.mode != ExtendedAdvertisingMode::NonConnectableNonScannable
            {
                return Err(Error::InvalidInput(
                    "connectable or scannable AUX_ADV_IND reserves AuxPtr".to_owned(),
                ));
            }
        }
        ExtendedAdvertisingPduKind::AuxChainInd => {
            if header.mode != ExtendedAdvertisingMode::NonConnectableNonScannable {
                return Err(Error::InvalidInput(
                    "AUX_CHAIN_IND requires non-connectable non-scannable mode".to_owned(),
                ));
            }
            if header.advertiser_address.is_some() || header.target_address.is_some() {
                return Err(Error::InvalidInput(
                    "AUX_CHAIN_IND reserves AdvA and TargetA".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

pub fn decode_contextual_extended_advertising_pdu(
    pdu: &AdvertisingPdu,
    kind: ExtendedAdvertisingPduKind,
) -> Result<ContextualExtendedAdvertisingPdu> {
    let header = extended_header(pdu, kind)?;
    validate_contextual_extended_header(kind, &header)?;
    Ok(ContextualExtendedAdvertisingPdu { kind, header })
}

fn advertising_packet_airtime_us(pdu: &AdvertisingPdu, phy: LePhy) -> Result<u128> {
    let octets = 4usize
        .checked_add(2)
        .and_then(|value| value.checked_add(pdu.payload.len()))
        .and_then(|value| value.checked_add(3))
        .ok_or_else(|| Error::InvalidState("advertising packet length overflow".to_owned()))?;
    let (preamble_octets, microseconds_per_octet) = match phy {
        LePhy::Le1M => (1usize, 8u128),
        LePhy::Le2M => (2usize, 4u128),
        LePhy::LeCoded => {
            return Err(Error::InvalidConfiguration(
                "LE Coded advertising airtime is not implemented".to_owned(),
            ));
        }
    };
    u128::try_from(
        preamble_octets
            .checked_add(octets)
            .ok_or_else(|| Error::InvalidState("advertising airtime overflow".to_owned()))?,
    )
    .map_err(|_| Error::InvalidState("advertising packet length exceeds u128".to_owned()))?
    .checked_mul(microseconds_per_octet)
    .ok_or_else(|| Error::InvalidState("advertising airtime overflow".to_owned()))
}

fn microseconds_to_samples_floor(microseconds: u128, sample_rate_hz: u32) -> Result<u64> {
    let samples = microseconds
        .checked_mul(u128::from(sample_rate_hz))
        .ok_or_else(|| Error::InvalidState("sample-time multiplication overflow".to_owned()))?
        / 1_000_000;
    u64::try_from(samples)
        .map_err(|_| Error::InvalidState("sample-time result exceeds u64".to_owned()))
}

fn microseconds_to_samples_ceil(microseconds: u128, sample_rate_hz: u32) -> Result<u64> {
    let numerator = microseconds
        .checked_mul(u128::from(sample_rate_hz))
        .ok_or_else(|| Error::InvalidState("sample-time multiplication overflow".to_owned()))?;
    divide_round_up(numerator, 1_000_000)
}

fn divide_round_up(numerator: u128, denominator: u128) -> Result<u64> {
    let rounded = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| Error::InvalidState("integer ceiling arithmetic overflow".to_owned()))?
        / denominator;
    u64::try_from(rounded)
        .map_err(|_| Error::InvalidState("integer ceiling result exceeds u64".to_owned()))
}

fn decode_extended_advertising_header(pdu: &AdvertisingPdu) -> Result<ExtendedAdvertisingHeader> {
    if pdu.header[0] & 0x30 != 0 {
        return Err(Error::InvalidInput(format!(
            "ADV_EXT_IND sets reserved first-header bits 0x{:02x}",
            pdu.header[0] & 0x30
        )));
    }
    if pdu.channel.is_primary_advertising() && pdu.header[1] & 0xc0 != 0 {
        return Err(Error::InvalidInput(format!(
            "ADV_EXT_IND sets reserved length-header bits 0x{:02x}",
            pdu.header[1] & 0xc0
        )));
    }
    let Some(&common_header) = pdu.payload.first() else {
        return Err(Error::InvalidInput(
            "ADV_EXT_IND requires the common extended advertising header octet".to_owned(),
        ));
    };
    let extended_header_length = common_header & 0x3f;
    let mode = ExtendedAdvertisingMode::parse(common_header >> 6)?;
    let header_end = 1usize
        .checked_add(extended_header_length as usize)
        .ok_or_else(|| Error::InvalidInput("ADV_EXT_IND header length overflow".to_owned()))?;
    if header_end > pdu.payload.len() {
        return Err(Error::InvalidInput(format!(
            "ADV_EXT_IND declares {extended_header_length} extended-header octets with only {} available",
            pdu.payload.len().saturating_sub(1)
        )));
    }

    let mut extended_header = &pdu.payload[1..header_end];
    let flags = if extended_header_length == 0 {
        0
    } else {
        take_extended_field(&mut extended_header, 1, "flags")?[0]
    };
    if flags & 0x80 != 0 {
        return Err(Error::InvalidInput(
            "ADV_EXT_IND extended-header reserved flag is set".to_owned(),
        ));
    }

    let (advertiser_address, advertiser_address_kind) = if flags & 0x01 != 0 {
        (
            Some(DeviceAddress::from_air_bytes(take_extended_field(
                &mut extended_header,
                6,
                "advertiser address",
            )?)?),
            Some(address_kind(pdu.tx_add_random())),
        )
    } else {
        (None, None)
    };
    let (target_address, target_address_kind) = if flags & 0x02 != 0 {
        (
            Some(DeviceAddress::from_air_bytes(take_extended_field(
                &mut extended_header,
                6,
                "target address",
            )?)?),
            Some(address_kind(pdu.rx_add_random())),
        )
    } else {
        (None, None)
    };
    let constant_tone_extension_info = if flags & 0x04 != 0 {
        Some(ConstantToneExtensionInfo::from_raw(
            take_extended_field(&mut extended_header, 1, "CTEInfo")?[0],
        ))
    } else {
        None
    };
    let advertising_data_info = if flags & 0x08 != 0 {
        let bytes = take_extended_field(&mut extended_header, 2, "ADI")?;
        let raw = u16::from_le_bytes([bytes[0], bytes[1]]);
        Some(AdvertisingDataInfo {
            data_id: raw & 0x0fff,
            advertising_set_id: (raw >> 12) as u8,
        })
    } else {
        None
    };
    let auxiliary_pointer = if flags & 0x10 != 0 {
        let bytes = take_extended_field(&mut extended_header, 3, "AuxPtr")?;
        let channel_index = bytes[0] & 0x3f;
        if channel_index > 36 {
            return Err(Error::InvalidInput(format!(
                "ADV_EXT_IND AuxPtr channel {channel_index} is outside 0..=36"
            )));
        }
        let offset_and_phy = u16::from_le_bytes([bytes[1], bytes[2]]);
        let phy = match offset_and_phy >> 13 {
            0 => LePhy::Le1M,
            1 => LePhy::Le2M,
            2 => LePhy::LeCoded,
            raw => {
                return Err(Error::InvalidInput(format!(
                    "ADV_EXT_IND AuxPtr PHY value {raw} is reserved"
                )));
            }
        };
        Some(AuxiliaryPointer {
            channel: BleChannel::new(channel_index)?,
            clock_accuracy: if bytes[0] & 0x40 != 0 {
                AuxiliaryClockAccuracy::Ppm50
            } else {
                AuxiliaryClockAccuracy::Ppm500
            },
            offset_units_us: if bytes[0] & 0x80 != 0 { 300 } else { 30 },
            offset: offset_and_phy & 0x1fff,
            phy,
        })
    } else {
        None
    };
    let sync_info = if flags & 0x20 != 0 {
        let bytes = take_extended_field(&mut extended_header, 18, "SyncInfo")?;
        let offset_fields = u16::from_le_bytes([bytes[0], bytes[1]]);
        if offset_fields & 0x8000 != 0 {
            return Err(Error::InvalidInput(
                "ADV_EXT_IND SyncInfo reserved offset bit is set".to_owned(),
            ));
        }
        let offset_units_us = if offset_fields & 0x2000 != 0 { 300 } else { 30 };
        let offset_adjust = offset_fields & 0x4000 != 0;
        if offset_adjust && offset_units_us != 300 {
            return Err(Error::InvalidInput(
                "ADV_EXT_IND SyncInfo sets Offset Adjust with 30 us units".to_owned(),
            ));
        }
        let packet_offset = offset_fields & 0x1fff;
        if offset_units_us == 300 && !offset_adjust && u32::from(packet_offset) * 300 < 245_700 {
            return Err(Error::InvalidInput(
                "ADV_EXT_IND SyncInfo uses 300 us units for an offset below 245700 us".to_owned(),
            ));
        }
        let interval = u16::from_le_bytes([bytes[2], bytes[3]]);
        if interval < 6 {
            return Err(Error::InvalidInput(format!(
                "ADV_EXT_IND SyncInfo interval {interval} is outside 6..=65535"
            )));
        }
        let sleep_clock_accuracy = SleepClockAccuracy::new(bytes[8] >> 5)?;
        let mut channel_map = [0u8; 5];
        channel_map.copy_from_slice(&bytes[4..9]);
        channel_map[4] &= 0x1f;
        Some(PeriodicAdvertisingSyncInfo {
            packet_offset,
            offset_units_us,
            offset_adjust,
            interval,
            channel_map: DataChannelMap::new(channel_map).map_err(|error| {
                Error::InvalidInput(format!(
                    "ADV_EXT_IND SyncInfo channel map is invalid: {error}"
                ))
            })?,
            sleep_clock_accuracy,
            access_address: u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]),
            crc_init: bytes[13] as u32 | (bytes[14] as u32) << 8 | (bytes[15] as u32) << 16,
            event_counter: u16::from_le_bytes([bytes[16], bytes[17]]),
        })
    } else {
        None
    };
    let tx_power_dbm = if flags & 0x40 != 0 {
        Some(take_extended_field(&mut extended_header, 1, "TxPower")?[0] as i8)
    } else {
        None
    };

    Ok(ExtendedAdvertisingHeader {
        extended_header_length,
        mode,
        flags,
        advertiser_address,
        advertiser_address_kind,
        target_address,
        target_address_kind,
        constant_tone_extension_info,
        advertising_data_info,
        auxiliary_pointer,
        sync_info,
        tx_power_dbm,
        additional_controller_advertising_data: extended_header.to_vec(),
        advertising_data: pdu.payload[header_end..].to_vec(),
    })
}

pub fn decode_advertising_pdu(pdu: &AdvertisingPdu) -> Result<DecodedAdvertisingPdu> {
    if !pdu.channel.is_primary_advertising() && pdu.pdu_type() != 7 {
        return Ok(DecodedAdvertisingPdu::ExtendedOrReserved {
            pdu_type: pdu.pdu_type(),
            payload: pdu.payload.clone(),
        });
    }
    match pdu.pdu_type() {
        0 => {
            let (advertiser, advertiser_kind, data) = decode_advertiser_and_data(pdu)?;
            Ok(DecodedAdvertisingPdu::AdvInd {
                advertiser,
                advertiser_kind,
                data,
            })
        }
        1 => {
            require_payload_length(pdu, 12, "ADV_DIRECT_IND")?;
            Ok(DecodedAdvertisingPdu::AdvDirectInd {
                advertiser: DeviceAddress::from_air_bytes(&pdu.payload[..6])?,
                advertiser_kind: address_kind(pdu.tx_add_random()),
                target: DeviceAddress::from_air_bytes(&pdu.payload[6..12])?,
                target_kind: address_kind(pdu.rx_add_random()),
            })
        }
        2 => {
            let (advertiser, advertiser_kind, data) = decode_advertiser_and_data(pdu)?;
            Ok(DecodedAdvertisingPdu::AdvNonconnInd {
                advertiser,
                advertiser_kind,
                data,
            })
        }
        3 => {
            require_payload_length(pdu, 12, "SCAN_REQ")?;
            Ok(DecodedAdvertisingPdu::ScanReq {
                scanner: DeviceAddress::from_air_bytes(&pdu.payload[..6])?,
                scanner_kind: address_kind(pdu.tx_add_random()),
                advertiser: DeviceAddress::from_air_bytes(&pdu.payload[6..12])?,
                advertiser_kind: address_kind(pdu.rx_add_random()),
            })
        }
        4 => {
            let (advertiser, advertiser_kind, data) = decode_advertiser_and_data(pdu)?;
            Ok(DecodedAdvertisingPdu::ScanRsp {
                advertiser,
                advertiser_kind,
                data,
            })
        }
        5 => {
            require_payload_length(pdu, 34, "CONNECT_IND")?;
            let parameters = &pdu.payload[12..];
            let hop_and_sca = parameters[21];
            let request = ConnectRequest {
                access_address: u32::from_le_bytes([
                    parameters[0],
                    parameters[1],
                    parameters[2],
                    parameters[3],
                ]),
                crc_init: parameters[4] as u32
                    | (parameters[5] as u32) << 8
                    | (parameters[6] as u32) << 16,
                window_size: parameters[7],
                window_offset: u16::from_le_bytes([parameters[8], parameters[9]]),
                interval: u16::from_le_bytes([parameters[10], parameters[11]]),
                latency: u16::from_le_bytes([parameters[12], parameters[13]]),
                supervision_timeout: u16::from_le_bytes([parameters[14], parameters[15]]),
                channel_map: parameters[16..21].try_into().map_err(|_| {
                    Error::InvalidInput("CONNECT_IND channel map is malformed".to_owned())
                })?,
                hop_increment: hop_and_sca & 0x1f,
                sleep_clock_accuracy: hop_and_sca >> 5,
                channel_selection_algorithm: if pdu.header[0] & 0x20 != 0 {
                    ChannelSelectionAlgorithm::Csa2
                } else {
                    ChannelSelectionAlgorithm::Csa1
                },
            };
            if !(5..=16).contains(&request.hop_increment) {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND hop increment {} is outside 5..=16",
                    request.hop_increment
                )));
            }
            if !(1..=16).contains(&request.window_size) {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND window size {} is outside 1..=16",
                    request.window_size
                )));
            }
            if !(6..=3_200).contains(&request.interval) {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND interval {} is outside 6..=3200",
                    request.interval
                )));
            }
            if request.window_offset > request.interval {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND window offset {} exceeds interval {}",
                    request.window_offset, request.interval
                )));
            }
            if request.latency > 499 {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND latency {} exceeds 499",
                    request.latency
                )));
            }
            if !(10..=3_200).contains(&request.supervision_timeout) {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND supervision timeout {} is outside 10..=3200",
                    request.supervision_timeout
                )));
            }
            let minimum_timeout_us =
                2u64 * (request.latency as u64 + 1) * request.interval_us() as u64;
            if request.supervision_timeout_us() as u64 <= minimum_timeout_us {
                return Err(Error::InvalidInput(format!(
                    "CONNECT_IND supervision timeout {} us must exceed {} us for interval and latency",
                    request.supervision_timeout_us(),
                    minimum_timeout_us
                )));
            }
            DataChannelMap::new(request.channel_map).map_err(|error| {
                Error::InvalidInput(format!("CONNECT_IND channel map is invalid: {error}"))
            })?;
            Ok(DecodedAdvertisingPdu::ConnectInd {
                initiator: DeviceAddress::from_air_bytes(&pdu.payload[..6])?,
                initiator_kind: address_kind(pdu.tx_add_random()),
                advertiser: DeviceAddress::from_air_bytes(&pdu.payload[6..12])?,
                advertiser_kind: address_kind(pdu.rx_add_random()),
                request,
            })
        }
        6 => {
            let (advertiser, advertiser_kind, data) = decode_advertiser_and_data(pdu)?;
            Ok(DecodedAdvertisingPdu::AdvScanInd {
                advertiser,
                advertiser_kind,
                data,
            })
        }
        7 => Ok(DecodedAdvertisingPdu::AdvExtInd {
            header: decode_extended_advertising_header(pdu)?,
        }),
        pdu_type => Ok(DecodedAdvertisingPdu::ExtendedOrReserved {
            pdu_type,
            payload: pdu.payload.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::{BleChannel, LE_ADV_ACCESS_ADDRESS};

    fn pdu(pdu_type: u8, flags: u8, payload: Vec<u8>) -> AdvertisingPdu {
        AdvertisingPdu {
            channel: BleChannel::new(37).unwrap(),
            access_address: LE_ADV_ACCESS_ADDRESS,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [pdu_type | flags, payload.len() as u8],
            payload,
            crc: [0; 3],
        }
    }

    fn extended_pdu(
        channel: u8,
        mode: ExtendedAdvertisingMode,
        header_flags: u8,
        fields: &[u8],
        advertising_data: &[u8],
    ) -> AdvertisingPdu {
        let mode_bits = match mode {
            ExtendedAdvertisingMode::NonConnectableNonScannable => 0,
            ExtendedAdvertisingMode::ConnectableNonScannable => 1,
            ExtendedAdvertisingMode::NonConnectableScannable => 2,
        };
        let extended_header_length = 1 + fields.len();
        let mut payload = Vec::with_capacity(1 + extended_header_length + advertising_data.len());
        payload.push((mode_bits << 6) | extended_header_length as u8);
        payload.push(header_flags);
        payload.extend_from_slice(fields);
        payload.extend_from_slice(advertising_data);
        AdvertisingPdu {
            channel: BleChannel::new(channel).unwrap(),
            access_address: LE_ADV_ACCESS_ADDRESS,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [7, payload.len() as u8],
            payload,
            crc: [0; 3],
        }
    }

    fn adi(data_id: u16, advertising_set_id: u8) -> [u8; 2] {
        (data_id | u16::from(advertising_set_id) << 12).to_le_bytes()
    }

    fn aux_pointer(
        channel: u8,
        clock_accuracy: AuxiliaryClockAccuracy,
        units_us: u16,
        offset: u16,
        phy: LePhy,
    ) -> [u8; 3] {
        let mut first = channel;
        if clock_accuracy == AuxiliaryClockAccuracy::Ppm50 {
            first |= 0x40;
        }
        if units_us == 300 {
            first |= 0x80;
        }
        let phy_bits = match phy {
            LePhy::Le1M => 0,
            LePhy::Le2M => 1,
            LePhy::LeCoded => 2,
        };
        let offset_and_phy = offset | phy_bits << 13;
        let encoded = offset_and_phy.to_le_bytes();
        [first, encoded[0], encoded[1]]
    }

    fn chain_tracker(maximum_advertising_data_length: usize) -> ExtendedAdvertisingChainTracker {
        ExtendedAdvertisingChainTracker::new(ExtendedAdvertisingChainConfig {
            sample_rate_hz: 4_000_000,
            receiver_clock_accuracy_ppm: 20,
            maximum_advertising_data_length,
        })
        .unwrap()
    }

    #[test]
    fn decodes_adv_ind_address_and_ad_structures() {
        let packet = pdu(
            0,
            0x40,
            vec![
                1, 2, 3, 4, 5, 6, 2, 0x01, 0x06, 5, 0x09, b't', b'e', b's', b't',
            ],
        );
        let decoded = decode_advertising_pdu(&packet).unwrap();
        let DecodedAdvertisingPdu::AdvInd {
            advertiser,
            advertiser_kind,
            data,
        } = decoded
        else {
            panic!("expected ADV_IND");
        };
        assert_eq!(advertiser.to_string(), "06:05:04:03:02:01");
        assert_eq!(advertiser_kind, AddressKind::Random);
        assert_eq!(data.len(), 2);
        assert_eq!(data[1].text(), Some("test"));
    }

    #[test]
    fn rejects_truncated_ad_structure() {
        let error = parse_ad_structures(&[4, 0x09, b'a']).unwrap_err();
        assert!(error.to_string().contains("only 2 remaining"));
    }

    #[test]
    fn decodes_connect_ind_parameters() {
        let mut payload = vec![1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1];
        payload.extend_from_slice(&[
            0xd6, 0xbe, 0x89, 0x8e, // access address
            0x12, 0x34, 0x56, // CRC init
            2,    // window size
            3, 0, // window offset
            24, 0, // interval
            1, 0, // latency
            100, 0, // timeout
            0xff, 0xff, 0xff, 0xff, 0x1f, // all 37 channels
            10,   // hop 10, SCA 0
        ]);
        let decoded = decode_advertising_pdu(&pdu(5, 0, payload)).unwrap();
        let DecodedAdvertisingPdu::ConnectInd { request, .. } = decoded else {
            panic!("expected CONNECT_IND");
        };
        assert_eq!(request.access_address, 0x8e89bed6);
        assert_eq!(request.crc_init, 0x563412);
        assert_eq!(request.interval_us(), 30_000);
        assert_eq!(request.supervision_timeout_us(), 1_000_000);
        assert_eq!(request.enabled_data_channels().len(), 37);
        assert_eq!(request.hop_increment, 10);
        assert_eq!(
            request.channel_selection_algorithm,
            ChannelSelectionAlgorithm::Csa1
        );
        assert_eq!(request.transmit_window_start_us(), 5_000);
        assert_eq!(request.transmit_window_size_us(), 2_500);
        assert_eq!(request.transmit_window_end_us(), 7_500);
        assert_eq!(request.event_offset_from_anchor_us(2), 60_000);
        assert_eq!(request.peer_clock_accuracy().unwrap().raw(), 0);
        assert_eq!(request.peer_clock_accuracy().unwrap().maximum_ppm(), 500);
        assert_eq!(
            request.first_event_window(1_000, 4_000_000, 20).unwrap(),
            FirstConnectionEventWindow {
                channel: crate::ble::BleChannel::new(10).unwrap(),
                nominal_start_sample: 22_376,
                nominal_end_sample: 32_376,
                earliest_sample: 22_360,
                latest_sample: 32_392,
                widening_samples: 16,
            }
        );
        assert_eq!(
            request
                .channel_selector()
                .unwrap()
                .channel_for_event(0)
                .index(),
            10
        );
        let mut tracker = request.connection_tracker(4_000_000, 0, 400).unwrap();
        assert_eq!(tracker.current_event().unwrap().channel.index(), 10);
        assert_eq!(tracker.advance().unwrap().channel.index(), 20);
        let tracker = request
            .acquire_first_event_anchor(
                1_000,
                4_000_000,
                20,
                FirstCentralTransmission::new(crate::ble::BleChannel::new(10).unwrap(), 30_000)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(tracker.event_counter(), 0);
        assert_eq!(
            tracker.current_event().unwrap().timing,
            crate::link_layer::ConnectionEventTiming::Expected {
                access_address_sample: 30_000
            }
        );
        assert!(
            request
                .acquire_first_event_anchor(
                    1_000,
                    4_000_000,
                    20,
                    FirstCentralTransmission::new(
                        crate::ble::BleChannel::new(11).unwrap(),
                        30_000,
                    )
                    .unwrap(),
                )
                .is_err()
        );
        assert!(
            request
                .acquire_first_event_anchor(
                    1_000,
                    4_000_000,
                    20,
                    FirstCentralTransmission::new(
                        crate::ble::BleChannel::new(10).unwrap(),
                        40_000,
                    )
                    .unwrap(),
                )
                .is_err()
        );
        assert!(
            FirstCentralTransmission::new(crate::ble::BleChannel::new(37).unwrap(), 30_000)
                .is_err()
        );
        assert!(request.first_event_window(1_000, 3_999_999, 20).is_err());
        assert!(
            request
                .first_event_window(1_000, 4_000_000, 1_000_001)
                .is_err()
        );

        let mut invalid_window = request;
        invalid_window.window_size = 0;
        assert!(
            invalid_window
                .first_event_window(1_000, 4_000_000, 20)
                .is_err()
        );
        invalid_window.window_size = 1;
        invalid_window.sleep_clock_accuracy = 8;
        assert!(
            invalid_window
                .first_event_window(1_000, 4_000_000, 20)
                .is_err()
        );
    }

    #[test]
    fn connect_ind_chsel_bit_selects_csa2() {
        let mut payload = vec![0; 34];
        payload[12..16].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        payload[19] = 1;
        payload[20..22].copy_from_slice(&0u16.to_le_bytes());
        payload[22..24].copy_from_slice(&24u16.to_le_bytes());
        payload[24..26].copy_from_slice(&0u16.to_le_bytes());
        payload[26..28].copy_from_slice(&100u16.to_le_bytes());
        payload[28..33].copy_from_slice(&[3, 0, 0, 0, 0]);
        payload[33] = 5;
        let decoded = decode_advertising_pdu(&pdu(5, 0x20, payload)).unwrap();
        let DecodedAdvertisingPdu::ConnectInd { request, .. } = decoded else {
            panic!("expected CONNECT_IND");
        };
        assert_eq!(
            request.channel_selection_algorithm,
            ChannelSelectionAlgorithm::Csa2
        );
    }

    #[test]
    fn rejects_invalid_connect_ind_timing() {
        let mut payload = vec![0; 34];
        payload[12..16].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        payload[19] = 1;
        payload[20..22].copy_from_slice(&0u16.to_le_bytes());
        payload[22..24].copy_from_slice(&24u16.to_le_bytes());
        payload[24..26].copy_from_slice(&499u16.to_le_bytes());
        payload[26..28].copy_from_slice(&10u16.to_le_bytes());
        payload[28..33].copy_from_slice(&[3, 0, 0, 0, 0]);
        payload[33] = 5;
        let error = decode_advertising_pdu(&pdu(5, 0, payload)).unwrap_err();
        assert!(error.to_string().contains("must exceed"));
    }

    #[test]
    fn decodes_extended_advertising_header_losslessly() {
        let decoded = decode_advertising_pdu(&pdu(
            7,
            0x40,
            vec![
                0x10, 0x59, 1, 2, 3, 4, 5, 6, 0xbc, 0xda, 0xd4, 0x23, 0x21, 0xf4, 2, 1, 6, 2, 1, 5,
            ],
        ))
        .unwrap();
        assert_eq!(
            decoded.to_string(),
            "ADV_EXT_IND mode=non-connectable-non-scannable ext_header_octets=16 flags=0x59 advertiser=06:05:04:03:02:01 advertiser_type=random sid=13 did=2748 aux_channel=20 aux_offset_us=87300 aux_phy=LE-2M tx_power_dbm=-12 acad_octets=3 advertising_data_octets=3"
        );
        let DecodedAdvertisingPdu::AdvExtInd { header } = decoded else {
            panic!("expected ADV_EXT_IND");
        };
        assert_eq!(header.extended_header_length, 16);
        assert_eq!(
            header.mode,
            ExtendedAdvertisingMode::NonConnectableNonScannable
        );
        assert_eq!(header.flags, 0x59);
        assert_eq!(
            header.advertiser_address.unwrap().to_string(),
            "06:05:04:03:02:01"
        );
        assert_eq!(header.advertiser_address_kind, Some(AddressKind::Random));
        assert_eq!(
            header.advertising_data_info,
            Some(AdvertisingDataInfo {
                data_id: 0x0abc,
                advertising_set_id: 0x0d,
            })
        );
        let auxiliary_pointer = header.auxiliary_pointer.unwrap();
        assert_eq!(auxiliary_pointer.channel.index(), 20);
        assert_eq!(
            auxiliary_pointer.clock_accuracy,
            AuxiliaryClockAccuracy::Ppm50
        );
        assert_eq!(auxiliary_pointer.clock_accuracy.maximum_ppm(), 50);
        assert_eq!(auxiliary_pointer.offset_units_us, 300);
        assert_eq!(auxiliary_pointer.offset, 0x0123);
        assert_eq!(auxiliary_pointer.offset_us(), 87_300);
        assert_eq!(auxiliary_pointer.phy, LePhy::Le2M);
        assert_eq!(header.tx_power_dbm, Some(-12));
        assert_eq!(header.additional_controller_advertising_data, [2, 1, 6]);
        assert_eq!(header.advertising_data, [2, 1, 5]);
    }

    #[test]
    fn decodes_periodic_advertising_sync_info() {
        let decoded = decode_advertising_pdu(&pdu(
            7,
            0,
            vec![
                0x13, 0x20, 0x21, 0x63, 0x20, 0x00, 0xff, 0xff, 0xff, 0xff, 0x7f, 0x78, 0x56, 0x34,
                0x12, 0xef, 0xcd, 0xab, 0x67, 0x45,
            ],
        ))
        .unwrap();
        let DecodedAdvertisingPdu::AdvExtInd { header } = decoded else {
            panic!("expected ADV_EXT_IND");
        };
        let sync_info = header.sync_info.unwrap();
        assert_eq!(sync_info.packet_offset, 0x0321);
        assert_eq!(sync_info.offset_units_us, 300);
        assert!(sync_info.offset_adjust);
        assert_eq!(sync_info.packet_offset_us(), 2_697_900);
        assert_eq!(sync_info.interval, 0x0020);
        assert_eq!(sync_info.interval_us(), 40_000);
        assert_eq!(
            sync_info.channel_map.bytes(),
            [0xff, 0xff, 0xff, 0xff, 0x1f]
        );
        assert_eq!(sync_info.channel_map.used_count(), 37);
        assert_eq!(sync_info.sleep_clock_accuracy.raw(), 3);
        assert_eq!(sync_info.sleep_clock_accuracy.maximum_ppm(), 100);
        assert_eq!(sync_info.access_address, 0x1234_5678);
        assert_eq!(sync_info.crc_init, 0x00ab_cdef);
        assert_eq!(sync_info.event_counter, 0x4567);
        assert!(header.additional_controller_advertising_data.is_empty());
        assert!(header.advertising_data.is_empty());
    }

    #[test]
    fn accepts_zero_length_extended_header() {
        let decoded = decode_advertising_pdu(&pdu(7, 0, vec![0, 2, 1, 6])).unwrap();
        let DecodedAdvertisingPdu::AdvExtInd { header } = decoded else {
            panic!("expected ADV_EXT_IND");
        };
        assert_eq!(header.extended_header_length, 0);
        assert_eq!(header.flags, 0);
        assert!(header.additional_controller_advertising_data.is_empty());
        assert_eq!(header.advertising_data, [2, 1, 6]);
    }

    #[test]
    fn secondary_extended_advertising_accepts_full_length_octet() {
        let payload = [vec![0, 2, 1, 6], vec![0x55; 66]].concat();
        let packet = AdvertisingPdu {
            channel: BleChannel::new(20).unwrap(),
            access_address: LE_ADV_ACCESS_ADDRESS,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [7, payload.len() as u8],
            payload,
            crc: [0; 3],
        };
        let decoded = decode_advertising_pdu(&packet).unwrap();
        let DecodedAdvertisingPdu::AdvExtInd { header } = decoded else {
            panic!("expected extended advertising PDU");
        };
        assert_eq!(packet.header[1], 70);
        assert_eq!(header.advertising_data.len(), 69);
    }

    #[test]
    fn secondary_legacy_type_is_preserved_without_primary_semantics() {
        let packet = AdvertisingPdu {
            channel: BleChannel::new(20).unwrap(),
            access_address: LE_ADV_ACCESS_ADDRESS,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [0, 6],
            payload: vec![1, 2, 3, 4, 5, 6],
            crc: [0; 3],
        };
        assert_eq!(
            decode_advertising_pdu(&packet).unwrap(),
            DecodedAdvertisingPdu::ExtendedOrReserved {
                pdu_type: 0,
                payload: vec![1, 2, 3, 4, 5, 6],
            }
        );
    }

    #[test]
    fn contextual_periodic_packet_is_named_aux_sync_ind() {
        let packet = AdvertisingPdu {
            channel: BleChannel::new(27).unwrap(),
            access_address: 0x1234_5678,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [0x07, 0x06],
            payload: vec![0x03, 0x08, 0xbc, 0xda, 0x02, 0x01],
            crc: [0x58, 0xc8, 0xce],
        };
        let decoded = decode_contextual_extended_advertising_pdu(
            &packet,
            ExtendedAdvertisingPduKind::AuxSyncInd,
        )
        .unwrap();
        assert_eq!(decoded.header.advertising_data_info.unwrap().data_id, 0xabc);
        assert_eq!(
            decoded.to_string(),
            "AUX_SYNC_IND mode=non-connectable-non-scannable ext_header_octets=3 flags=0x08 sid=13 did=2748 acad_octets=0 advertising_data_octets=2"
        );
    }

    #[test]
    fn rejects_truncated_extended_advertising_header() {
        let error = decode_advertising_pdu(&pdu(7, 0, vec![3, 0])).unwrap_err();
        assert!(error.to_string().contains("declares 3"));
    }

    #[test]
    fn rejects_truncated_extended_advertising_fields() {
        let fields = [
            (0x01, 6, "advertiser address"),
            (0x02, 6, "target address"),
            (0x04, 1, "CTEInfo"),
            (0x08, 2, "ADI"),
            (0x10, 3, "AuxPtr"),
            (0x20, 18, "SyncInfo"),
            (0x40, 1, "TxPower"),
        ];
        for (flag, field_length, name) in fields {
            let mut payload = vec![field_length as u8, flag];
            payload.resize(field_length + 1, 0);
            let error = decode_advertising_pdu(&pdu(7, 0, payload)).unwrap_err();
            assert!(
                error.to_string().contains(name),
                "{name}: unexpected error: {error}"
            );
        }
    }

    #[test]
    fn rejects_reserved_extended_advertising_values() {
        let reserved_mode = decode_advertising_pdu(&pdu(7, 0, vec![0xc0])).unwrap_err();
        assert!(reserved_mode.to_string().contains("mode 3 is reserved"));

        let reserved_flag = decode_advertising_pdu(&pdu(7, 0, vec![1, 0x80])).unwrap_err();
        assert!(reserved_flag.to_string().contains("reserved flag"));

        let invalid_channel =
            decode_advertising_pdu(&pdu(7, 0, vec![4, 0x10, 37, 1, 0])).unwrap_err();
        assert!(invalid_channel.to_string().contains("channel 37"));

        let reserved_phy =
            decode_advertising_pdu(&pdu(7, 0, vec![4, 0x10, 0, 1, 0x60])).unwrap_err();
        assert!(reserved_phy.to_string().contains("PHY value 3"));
    }

    #[test]
    fn rejects_invalid_periodic_advertising_sync_info() {
        let sync_info = vec![
            0x13, 0x20, 0x21, 0x63, 0x20, 0x00, 0xff, 0xff, 0xff, 0xff, 0x7f, 0x78, 0x56, 0x34,
            0x12, 0xef, 0xcd, 0xab, 0x67, 0x45,
        ];

        let mut adjusted_30_us = sync_info.clone();
        adjusted_30_us[3] &= !0x20;
        let invalid_adjust = decode_advertising_pdu(&pdu(7, 0, adjusted_30_us)).unwrap_err();
        assert!(invalid_adjust.to_string().contains("Offset Adjust"));

        let mut short_300_us = sync_info.clone();
        short_300_us[3] &= !0x40;
        let invalid_unit = decode_advertising_pdu(&pdu(7, 0, short_300_us)).unwrap_err();
        assert!(invalid_unit.to_string().contains("below 245700"));

        let mut sync_info = sync_info;
        sync_info[3] |= 0x80;
        let reserved_bit = decode_advertising_pdu(&pdu(7, 0, sync_info.clone())).unwrap_err();
        assert!(reserved_bit.to_string().contains("reserved offset bit"));

        sync_info[3] &= 0x7f;
        sync_info[4..6].copy_from_slice(&5u16.to_le_bytes());
        let short_interval = decode_advertising_pdu(&pdu(7, 0, sync_info.clone())).unwrap_err();
        assert!(short_interval.to_string().contains("interval 5"));

        sync_info[4..6].copy_from_slice(&6u16.to_le_bytes());
        sync_info[6..11].copy_from_slice(&[1, 0, 0, 0, 0]);
        let invalid_map = decode_advertising_pdu(&pdu(7, 0, sync_info)).unwrap_err();
        assert!(invalid_map.to_string().contains("fewer than two channels"));
    }

    #[test]
    fn schedules_quantized_auxiliary_reception_windows() {
        let pointer_30 = AuxiliaryPointer {
            channel: BleChannel::new(20).unwrap(),
            clock_accuracy: AuxiliaryClockAccuracy::Ppm500,
            offset_units_us: 30,
            offset: 20,
            phy: LePhy::Le2M,
        };
        let parent = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x10,
            &aux_pointer(20, AuxiliaryClockAccuracy::Ppm500, 30, 20, LePhy::Le2M),
            &[],
        );
        assert_eq!(
            pointer_30
                .reception_window(
                    &parent,
                    LePhy::Le1M,
                    1_000,
                    4_000_000,
                    20,
                    ExtendedAdvertisingPduKind::AuxAdvInd,
                )
                .unwrap(),
            AuxiliaryReceptionWindow {
                expected_kind: ExtendedAdvertisingPduKind::AuxAdvInd,
                channel: BleChannel::new(20).unwrap(),
                phy: LePhy::Le2M,
                represented_earliest_sample: 3_400,
                represented_latest_sample: 3_520,
                earliest_sample: 3_398,
                latest_sample: 3_522,
                quantization_width_us: 30,
                quantization_width_samples: 120,
                widening_samples: 2,
            }
        );

        let pointer_300 = AuxiliaryPointer {
            channel: BleChannel::new(20).unwrap(),
            clock_accuracy: AuxiliaryClockAccuracy::Ppm50,
            offset_units_us: 300,
            offset: 291,
            phy: LePhy::Le1M,
        };
        let window = pointer_300
            .reception_window(
                &parent,
                LePhy::Le1M,
                1_000,
                4_000_000,
                20,
                ExtendedAdvertisingPduKind::AuxAdvInd,
            )
            .unwrap();
        assert_eq!(window.represented_earliest_sample, 350_200);
        assert_eq!(window.represented_latest_sample, 351_400);
        assert_eq!(window.quantization_width_samples, 1_200);
        assert_eq!(window.widening_samples, 25);
        assert_eq!(window.earliest_sample, 350_175);
        assert_eq!(window.latest_sample, 351_425);

        let wide_clock = AuxiliaryPointer {
            clock_accuracy: AuxiliaryClockAccuracy::Ppm500,
            ..pointer_300
        }
        .reception_window(
            &parent,
            LePhy::Le1M,
            1_000,
            4_000_000,
            20,
            ExtendedAdvertisingPduKind::AuxAdvInd,
        )
        .unwrap();
        assert_eq!(wide_clock.widening_samples, 182);
    }

    #[test]
    fn schedules_from_le_2m_and_adjusts_for_coded_child_preamble() {
        let parent = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x10,
            &aux_pointer(21, AuxiliaryClockAccuracy::Ppm50, 30, 20, LePhy::Le1M),
            &[1, 2, 3],
        );
        let base_pointer = AuxiliaryPointer {
            channel: BleChannel::new(21).unwrap(),
            clock_accuracy: AuxiliaryClockAccuracy::Ppm50,
            offset_units_us: 30,
            offset: 20,
            phy: LePhy::Le1M,
        };
        let uncoded = base_pointer
            .reception_window(
                &parent,
                LePhy::Le2M,
                10_000,
                4_000_000,
                20,
                ExtendedAdvertisingPduKind::AuxChainInd,
            )
            .unwrap();
        assert_eq!(uncoded.represented_earliest_sample, 12_400);

        let coded = AuxiliaryPointer {
            phy: LePhy::LeCoded,
            ..base_pointer
        }
        .reception_window(
            &parent,
            LePhy::Le2M,
            10_000,
            4_000_000,
            20,
            ExtendedAdvertisingPduKind::AuxChainInd,
        )
        .unwrap();
        assert_eq!(
            coded.represented_earliest_sample - uncoded.represented_earliest_sample,
            288
        );
        assert_eq!(
            coded.represented_latest_sample - uncoded.represented_latest_sample,
            288
        );
    }

    #[test]
    fn rejects_unschedulable_auxiliary_offsets_and_invalid_clock_inputs() {
        let parent = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0,
            &[],
            &[],
        );
        let pointer = AuxiliaryPointer {
            channel: BleChannel::new(20).unwrap(),
            clock_accuracy: AuxiliaryClockAccuracy::Ppm500,
            offset_units_us: 30,
            offset: 0,
            phy: LePhy::Le1M,
        };
        assert!(
            pointer
                .reception_window(
                    &parent,
                    LePhy::Le1M,
                    0,
                    4_000_000,
                    20,
                    ExtendedAdvertisingPduKind::AuxAdvInd,
                )
                .unwrap_err()
                .to_string()
                .contains("offset zero")
        );
        assert!(
            AuxiliaryPointer {
                offset: 10,
                ..pointer
            }
            .reception_window(
                &parent,
                LePhy::Le1M,
                0,
                4_000_000,
                20,
                ExtendedAdvertisingPduKind::AuxAdvInd,
            )
            .unwrap_err()
            .to_string()
            .contains("300 us MAFS")
        );
        assert!(
            AuxiliaryPointer {
                offset: 20,
                ..pointer
            }
            .reception_window(
                &parent,
                LePhy::Le1M,
                0,
                4_000_000,
                1_000_001,
                ExtendedAdvertisingPduKind::AuxAdvInd,
            )
            .unwrap_err()
            .to_string()
            .contains("1000000")
        );
    }

    #[test]
    fn tracks_and_reassembles_an_auxiliary_advertising_chain() {
        let identity = adi(0x0abc, 0x0d);
        let first_pointer = aux_pointer(20, AuxiliaryClockAccuracy::Ppm50, 30, 20, LePhy::Le2M);
        let primary_fields = [
            1,
            2,
            3,
            4,
            5,
            6,
            identity[0],
            identity[1],
            first_pointer[0],
            first_pointer[1],
            first_pointer[2],
        ];
        let primary = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x19,
            &primary_fields,
            &[],
        );
        let mut tracker = chain_tracker(64);
        let ExtendedAdvertisingChainProgress::Awaiting { window, .. } =
            tracker.begin(&primary, LePhy::Le1M, 1_000).unwrap()
        else {
            panic!("expected AUX_ADV_IND window");
        };
        assert_eq!(window.expected_kind, ExtendedAdvertisingPduKind::AuxAdvInd);
        assert_eq!(window.channel.index(), 20);
        assert_eq!(window.phy, LePhy::Le2M);

        let second_pointer = aux_pointer(21, AuxiliaryClockAccuracy::Ppm500, 30, 20, LePhy::Le1M);
        let aux_adv_fields = [
            identity[0],
            identity[1],
            second_pointer[0],
            second_pointer[1],
            second_pointer[2],
        ];
        let aux_adv = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x18,
            &aux_adv_fields,
            &[1, 2, 3],
        );
        let ExtendedAdvertisingChainProgress::Awaiting {
            window: chain_window,
            fragment_count,
            advertising_data_octets,
        } = tracker
            .observe(&aux_adv, LePhy::Le2M, window.represented_earliest_sample)
            .unwrap()
        else {
            panic!("expected AUX_CHAIN_IND window");
        };
        assert_eq!(
            chain_window.expected_kind,
            ExtendedAdvertisingPduKind::AuxChainInd
        );
        assert_eq!(fragment_count, 1);
        assert_eq!(advertising_data_octets, 3);

        let aux_chain = extended_pdu(
            21,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x08,
            &identity,
            &[4, 5, 6, 7],
        );
        let ExtendedAdvertisingChainProgress::Complete(chain) = tracker
            .observe(
                &aux_chain,
                LePhy::Le1M,
                chain_window.represented_earliest_sample,
            )
            .unwrap()
        else {
            panic!("expected complete chain");
        };
        assert_eq!(chain.advertiser_address.unwrap().0, [1, 2, 3, 4, 5, 6]);
        assert_eq!(chain.advertiser_address_kind, Some(AddressKind::Public));
        assert_eq!(
            chain.advertising_data_info,
            Some(AdvertisingDataInfo {
                data_id: 0x0abc,
                advertising_set_id: 0x0d,
            })
        );
        assert_eq!(chain.advertising_data, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(chain.fragment_count, 2);
        assert_eq!(
            tracker.progress(),
            Some(ExtendedAdvertisingChainProgress::Complete(chain))
        );
        tracker.reset();
        assert_eq!(tracker.progress(), None);
    }

    #[test]
    fn chain_rejections_do_not_mutate_tracker_state() {
        let identity = adi(1, 2);
        let pointer = aux_pointer(20, AuxiliaryClockAccuracy::Ppm50, 30, 20, LePhy::Le1M);
        let fields = [identity[0], identity[1], pointer[0], pointer[1], pointer[2]];
        let primary = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x18,
            &fields,
            &[],
        );
        let mut tracker = chain_tracker(3);
        let initial = tracker.begin(&primary, LePhy::Le1M, 1_000).unwrap();
        let wrong_channel = extended_pdu(
            19,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x08,
            &identity,
            &[1],
        );
        assert!(
            tracker
                .observe(&wrong_channel, LePhy::Le1M, 3_400)
                .unwrap_err()
                .to_string()
                .contains("expected channel 20")
        );
        assert_eq!(tracker.progress(), Some(initial.clone()));

        let matching_channel = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x08,
            &identity,
            &[1],
        );
        assert!(
            tracker
                .observe(&matching_channel, LePhy::Le2M, 3_400)
                .unwrap_err()
                .to_string()
                .contains("expected PHY LE-1M")
        );
        assert_eq!(tracker.progress(), Some(initial.clone()));
        assert!(
            tracker
                .observe(&matching_channel, LePhy::Le1M, 4_000)
                .unwrap_err()
                .to_string()
                .contains("outside")
        );
        assert_eq!(tracker.progress(), Some(initial.clone()));

        let oversized = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x08,
            &identity,
            &[1, 2, 3, 4],
        );
        assert!(
            tracker
                .observe(&oversized, LePhy::Le1M, 3_400)
                .unwrap_err()
                .to_string()
                .contains("configured maximum 3")
        );
        assert_eq!(tracker.progress(), Some(initial));
    }

    #[test]
    fn enforces_adi_and_contextual_auxiliary_chain_invariants() {
        let identity = adi(1, 2);
        let pointer = aux_pointer(20, AuxiliaryClockAccuracy::Ppm50, 30, 20, LePhy::Le1M);
        let primary_fields = [identity[0], identity[1], pointer[0], pointer[1], pointer[2]];
        let primary = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x18,
            &primary_fields,
            &[],
        );

        let mut mismatch_tracker = chain_tracker(64);
        mismatch_tracker
            .begin(&primary, LePhy::Le1M, 1_000)
            .unwrap();
        let mismatched_adi = adi(2, 2);
        let mismatched = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x08,
            &mismatched_adi,
            &[],
        );
        assert!(
            mismatch_tracker
                .observe(&mismatched, LePhy::Le1M, 3_400)
                .unwrap_err()
                .to_string()
                .contains("ADI SID/DID")
        );

        let next_pointer = aux_pointer(21, AuxiliaryClockAccuracy::Ppm50, 30, 20, LePhy::Le1M);
        let continuing_fields = [
            identity[0],
            identity[1],
            next_pointer[0],
            next_pointer[1],
            next_pointer[2],
        ];
        let connectable = extended_pdu(
            20,
            ExtendedAdvertisingMode::ConnectableNonScannable,
            0x18,
            &continuing_fields,
            &[],
        );
        assert!(
            mismatch_tracker
                .observe(&connectable, LePhy::Le1M, 3_400)
                .unwrap_err()
                .to_string()
                .contains("reserves AuxPtr")
        );

        let mut contextual_tracker = chain_tracker(64);
        let ExtendedAdvertisingChainProgress::Awaiting { window, .. } = contextual_tracker
            .begin(&primary, LePhy::Le1M, 1_000)
            .unwrap()
        else {
            panic!("expected first window");
        };
        let continuing = extended_pdu(
            20,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x18,
            &continuing_fields,
            &[],
        );
        let ExtendedAdvertisingChainProgress::Awaiting {
            window: chain_window,
            ..
        } = contextual_tracker
            .observe(&continuing, LePhy::Le1M, window.represented_earliest_sample)
            .unwrap()
        else {
            panic!("expected chain window");
        };
        let chain_with_adva = extended_pdu(
            21,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0x09,
            &[7, 8, 9, 10, 11, 12, identity[0], identity[1]],
            &[],
        );
        assert!(
            contextual_tracker
                .observe(
                    &chain_with_adva,
                    LePhy::Le1M,
                    chain_window.represented_earliest_sample,
                )
                .unwrap_err()
                .to_string()
                .contains("reserves AdvA")
        );
        let chain_with_mode = extended_pdu(
            21,
            ExtendedAdvertisingMode::NonConnectableScannable,
            0x08,
            &identity,
            &[],
        );
        assert!(
            contextual_tracker
                .observe(
                    &chain_with_mode,
                    LePhy::Le1M,
                    chain_window.represented_earliest_sample,
                )
                .unwrap_err()
                .to_string()
                .contains("requires non-connectable")
        );
    }

    #[test]
    fn bounded_auxiliary_scheduling_inputs_remain_panic_free() {
        let parent = extended_pdu(
            37,
            ExtendedAdvertisingMode::NonConnectableNonScannable,
            0,
            &[],
            &[],
        );
        for offset in [0, 1, 10, 20, 0x1fff] {
            for units in [30, 300] {
                for phy in [LePhy::Le1M, LePhy::Le2M, LePhy::LeCoded] {
                    let pointer = AuxiliaryPointer {
                        channel: BleChannel::new(36).unwrap(),
                        clock_accuracy: AuxiliaryClockAccuracy::Ppm500,
                        offset_units_us: units,
                        offset,
                        phy,
                    };
                    let _ = pointer.reception_window(
                        &parent,
                        LePhy::Le1M,
                        u64::MAX / 2,
                        u32::MAX,
                        1_000_000,
                        ExtendedAdvertisingPduKind::AuxAdvInd,
                    );
                }
            }
        }
    }

    #[test]
    fn arbitrary_bounded_pdus_do_not_panic() {
        let mut state = 0x1234_5678u32;
        for length in 0..=37 {
            for pdu_type in 0..=15 {
                let mut payload = Vec::with_capacity(length);
                for _ in 0..length {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    payload.push((state >> 24) as u8);
                }
                let _ = decode_advertising_pdu(&pdu(pdu_type, (state >> 16) as u8, payload));
            }
        }
    }
}
