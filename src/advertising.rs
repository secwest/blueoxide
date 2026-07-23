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
            Self::AdvExtInd { header } => {
                write!(
                    formatter,
                    "ADV_EXT_IND mode={} ext_header_octets={} flags=0x{:02x}",
                    header.mode, header.extended_header_length, header.flags
                )?;
                if let (Some(address), Some(kind)) =
                    (header.advertiser_address, header.advertiser_address_kind)
                {
                    write!(formatter, " advertiser={address} advertiser_type={kind}")?;
                }
                if let (Some(address), Some(kind)) =
                    (header.target_address, header.target_address_kind)
                {
                    write!(formatter, " target={address} target_type={kind}")?;
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
            Self::ExtendedOrReserved { pdu_type, payload } => write!(
                formatter,
                "PDU_TYPE_{pdu_type} undecoded_payload_octets={}",
                payload.len()
            ),
        }
    }
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

fn decode_extended_advertising_header(pdu: &AdvertisingPdu) -> Result<ExtendedAdvertisingHeader> {
    if pdu.header[0] & 0x30 != 0 {
        return Err(Error::InvalidInput(format!(
            "ADV_EXT_IND sets reserved first-header bits 0x{:02x}",
            pdu.header[0] & 0x30
        )));
    }
    if pdu.header[1] & 0xc0 != 0 {
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
            packet_offset: offset_fields & 0x1fff,
            offset_units_us: if offset_fields & 0x2000 != 0 { 300 } else { 30 },
            offset_adjust: offset_fields & 0x4000 != 0,
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
    use crate::ble::BleChannel;

    fn pdu(pdu_type: u8, flags: u8, payload: Vec<u8>) -> AdvertisingPdu {
        AdvertisingPdu {
            channel: BleChannel::new(37).unwrap(),
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [pdu_type | flags, payload.len() as u8],
            payload,
            crc: [0; 3],
        }
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
        let mut sync_info = vec![
            0x13, 0x20, 0x21, 0x63, 0x20, 0x00, 0xff, 0xff, 0xff, 0xff, 0x7f, 0x78, 0x56, 0x34,
            0x12, 0xef, 0xcd, 0xab, 0x67, 0x45,
        ];

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
