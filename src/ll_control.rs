use crate::link_layer::{
    ChannelMapInd, ConnectionParameters, ConnectionUpdateInd, ControlPdu, DataChannelMap,
    SleepClockAccuracy,
};
use crate::{Error, Result};

mod cs;
pub use cs::*;

pub const LE_FEATURE_PAGE_OCTETS: usize = 24;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedControlPdu<'a> {
    ConnectionUpdateInd(ConnectionUpdateInd),
    ChannelMapInd(ChannelMapInd),
    TerminateInd(ErrorIndication),
    EncryptionRequest(EncryptionRequest),
    EncryptionResponse(EncryptionResponse),
    StartEncryptionRequest,
    StartEncryptionResponse,
    UnknownResponse(UnknownResponse),
    FeatureRequest(FeatureSet),
    FeatureResponse(FeatureSet),
    PauseEncryptionRequest,
    PauseEncryptionResponse,
    VersionInd(VersionInd),
    RejectInd(ErrorIndication),
    PeripheralFeatureRequest(FeatureSet),
    ConnectionParameterRequest(ConnectionParameterPdu),
    ConnectionParameterResponse(ConnectionParameterPdu),
    RejectExtendedInd(RejectExtendedInd),
    PingRequest,
    PingResponse,
    LengthRequest(DataLengthPdu),
    LengthResponse(DataLengthPdu),
    PhyRequest(PhyPreferences),
    PhyResponse(PhyPreferences),
    PhyUpdateInd(PhyUpdateInd),
    MinimumUsedChannelsInd(MinimumUsedChannelsInd),
    CteRequest(CteRequest),
    CteResponse,
    PeriodicSyncInd(PeriodicSyncInd),
    ClockAccuracyRequest(SleepClockAccuracy),
    ClockAccuracyResponse(SleepClockAccuracy),
    CisRequest(CisRequest),
    CisResponse(CisResponse),
    CisInd(CisInd),
    CisTerminateInd(CisTerminateInd),
    PowerControlRequest(PowerControlRequest),
    PowerControlResponse(PowerControlResponse),
    PowerChangeInd(PowerChangeInd),
    SubrateRequest(SubrateRequest),
    SubrateInd(SubrateInd),
    ChannelReportingInd(ChannelReportingInd),
    ChannelStatusInd(ChannelStatusInd),
    PeriodicSyncWrInd(PeriodicSyncWrInd),
    FeatureExtendedRequest(FeaturePagePdu),
    FeatureExtendedResponse(FeaturePagePdu),
    CsSecurityResponse(CsSecurityParameters),
    CsCapabilitiesRequest(CsCapabilities),
    CsCapabilitiesResponse(CsCapabilities),
    CsConfigRequest(CsConfigRequest),
    CsConfigResponse(CsConfigResponse),
    CsProcedureRequest(CsProcedureRequest),
    CsProcedureResponse(CsProcedureResponse),
    CsProcedureIndication(CsProcedureIndication),
    CsTerminateRequest(CsTermination),
    CsFaeRequest,
    CsFaeResponse(CsFaeTable),
    CsChannelMapInd(CsChannelMapInd),
    CsSecurityRequest(CsSecurityParameters),
    CsTerminateResponse(CsTermination),
    FrameSpaceRequest(FrameSpaceRequest),
    FrameSpaceResponse(FrameSpaceResponse),
    Raw { opcode: u8, parameters: &'a [u8] },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorIndication {
    pub error_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncryptionRequest {
    pub random_number: [u8; 8],
    pub encrypted_diversifier: u16,
    pub central_session_key_diversifier: [u8; 8],
    pub central_initialization_vector: [u8; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncryptionResponse {
    pub peripheral_session_key_diversifier: [u8; 8],
    pub peripheral_initialization_vector: [u8; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownResponse {
    pub unknown_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureSet {
    pub bytes: [u8; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionInd {
    pub version: u8,
    pub company_identifier: u16,
    pub subversion: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionParameterPdu {
    pub interval_min: u16,
    pub interval_max: u16,
    pub latency: u16,
    pub supervision_timeout: u16,
    pub preferred_periodicity: u8,
    pub reference_connection_event_count: u16,
    pub offsets: [u16; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectExtendedInd {
    pub rejected_opcode: u8,
    pub error_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataLengthPdu {
    pub maximum_receive_octets: u16,
    pub maximum_receive_time_us: u16,
    pub maximum_transmit_octets: u16,
    pub maximum_transmit_time_us: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhyPreferences {
    pub transmit_phys: u8,
    pub receive_phys: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhyUpdateInd {
    pub central_to_peripheral_phy: u8,
    pub peripheral_to_central_phy: u8,
    pub instant: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinimumUsedChannelsInd {
    pub phys: u8,
    pub minimum_used_channels: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CteRequest {
    pub minimum_length_units: u8,
    pub cte_type: u8,
}

impl CteRequest {
    pub const fn minimum_duration_us(self) -> u16 {
        self.minimum_length_units as u16 * 8
    }

    pub const fn cte_type_name(self) -> &'static str {
        match self.cte_type {
            0 => "AoA",
            1 => "AoD-1us",
            2 => "AoD-2us",
            _ => "reserved",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodicSyncInfo {
    pub offset_base: u16,
    pub offset_units_300_us: bool,
    pub offset_adjust: bool,
    pub interval: u16,
    pub channel_map: DataChannelMap,
    pub advertiser_sleep_clock_accuracy: SleepClockAccuracy,
    pub access_address: u32,
    pub crc_init: u32,
    pub periodic_event_counter: u16,
}

impl PeriodicSyncInfo {
    pub const fn packet_window_offset_us(&self) -> u32 {
        let unit = if self.offset_units_300_us { 300 } else { 30 };
        self.offset_base as u32 * unit + if self.offset_adjust { 2_457_600 } else { 0 }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodicSyncInd {
    pub identifier: u16,
    pub sync_info: PeriodicSyncInfo,
    pub connection_event_count: u16,
    pub last_periodic_event_counter: u16,
    pub advertising_sid: u8,
    pub advertiser_address_random: bool,
    pub sender_sleep_clock_accuracy: SleepClockAccuracy,
    pub phy: u8,
    pub advertiser_address: [u8; 6],
    pub sync_connection_event_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CisRequest {
    pub cig_identifier: u8,
    pub cis_identifier: u8,
    pub central_to_peripheral_phy: u8,
    pub peripheral_to_central_phy: u8,
    pub maximum_central_sdu: u16,
    pub framed: bool,
    pub framing_mode_unsegmented: bool,
    pub maximum_peripheral_sdu: u16,
    pub central_sdu_interval_us: u32,
    pub peripheral_sdu_interval_us: u32,
    pub maximum_central_pdu: u16,
    pub maximum_peripheral_pdu: u16,
    pub subevents: u8,
    pub subevent_interval_us: u32,
    pub central_burst_number: u8,
    pub peripheral_burst_number: u8,
    pub central_flush_timeout: u8,
    pub peripheral_flush_timeout: u8,
    pub iso_interval: u16,
    pub cis_offset_min_us: u32,
    pub cis_offset_max_us: u32,
    pub connection_event_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CisResponse {
    pub cis_offset_min_us: u32,
    pub cis_offset_max_us: u32,
    pub connection_event_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CisInd {
    pub access_address: u32,
    pub cis_offset_us: u32,
    pub cig_sync_delay_us: u32,
    pub cis_sync_delay_us: u32,
    pub connection_event_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CisTerminateInd {
    pub cig_identifier: u8,
    pub cis_identifier: u8,
    pub error_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PowerControlRequest {
    pub phy: u8,
    pub delta_db: i8,
    pub transmit_power_dbm: i8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PowerControlResponse {
    pub at_minimum: bool,
    pub at_maximum: bool,
    pub delta_db: i8,
    pub transmit_power_dbm: i8,
    pub acceptable_power_reduction_db: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PowerChangeInd {
    pub phys: u8,
    pub at_minimum: bool,
    pub at_maximum: bool,
    pub delta_db: i8,
    pub transmit_power_dbm: i8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubrateRequest {
    pub factor_min: u16,
    pub factor_max: u16,
    pub maximum_latency: u16,
    pub continuation_number: u16,
    pub supervision_timeout: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubrateInd {
    pub factor: u16,
    pub base_event: u16,
    pub latency: u16,
    pub continuation_number: u16,
    pub supervision_timeout: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelReportingInd {
    pub enabled: bool,
    pub minimum_spacing: u8,
    pub maximum_delay: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelClassification {
    Unknown,
    Good,
    Bad,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelStatusInd {
    pub classifications: [ChannelClassification; 37],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodicSyncWrInd {
    pub periodic_sync: PeriodicSyncInd,
    pub response_access_address: u32,
    pub subevent_count: u8,
    pub subevent_interval: u8,
    pub response_slot_delay: u8,
    pub response_slot_spacing: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeaturePagePdu {
    pub maximum_page: u8,
    pub page_number: u8,
    pub feature_page: [u8; LE_FEATURE_PAGE_OCTETS],
}

impl<'a> ControlPdu<'a> {
    pub fn decode(self) -> Result<DecodedControlPdu<'a>> {
        match self.opcode {
            0x00 => Ok(DecodedControlPdu::ConnectionUpdateInd(
                self.connection_update_ind()?
                    .expect("opcode checked before typed decode"),
            )),
            0x01 => Ok(DecodedControlPdu::ChannelMapInd(
                self.channel_map_ind()?
                    .expect("opcode checked before typed decode"),
            )),
            0x02 => {
                require_length(self, 1)?;
                Ok(DecodedControlPdu::TerminateInd(ErrorIndication {
                    error_code: self.parameters[0],
                }))
            }
            0x03 => {
                require_length(self, 22)?;
                Ok(DecodedControlPdu::EncryptionRequest(EncryptionRequest {
                    random_number: array(self.parameters, 0),
                    encrypted_diversifier: le_u16(self.parameters, 8),
                    central_session_key_diversifier: array(self.parameters, 10),
                    central_initialization_vector: array(self.parameters, 18),
                }))
            }
            0x04 => {
                require_length(self, 12)?;
                Ok(DecodedControlPdu::EncryptionResponse(EncryptionResponse {
                    peripheral_session_key_diversifier: array(self.parameters, 0),
                    peripheral_initialization_vector: array(self.parameters, 8),
                }))
            }
            0x05 => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::StartEncryptionRequest)
            }
            0x06 => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::StartEncryptionResponse)
            }
            0x07 => {
                require_length(self, 1)?;
                Ok(DecodedControlPdu::UnknownResponse(UnknownResponse {
                    unknown_type: self.parameters[0],
                }))
            }
            0x08 => Ok(DecodedControlPdu::FeatureRequest(parse_feature_set(self)?)),
            0x09 => Ok(DecodedControlPdu::FeatureResponse(parse_feature_set(self)?)),
            0x0a => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::PauseEncryptionRequest)
            }
            0x0b => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::PauseEncryptionResponse)
            }
            0x0c => {
                require_length(self, 5)?;
                Ok(DecodedControlPdu::VersionInd(VersionInd {
                    version: self.parameters[0],
                    company_identifier: le_u16(self.parameters, 1),
                    subversion: le_u16(self.parameters, 3),
                }))
            }
            0x0d => {
                require_length(self, 1)?;
                Ok(DecodedControlPdu::RejectInd(ErrorIndication {
                    error_code: self.parameters[0],
                }))
            }
            0x0e => Ok(DecodedControlPdu::PeripheralFeatureRequest(
                parse_feature_set(self)?,
            )),
            0x0f => Ok(DecodedControlPdu::ConnectionParameterRequest(
                parse_connection_parameters(self)?,
            )),
            0x10 => Ok(DecodedControlPdu::ConnectionParameterResponse(
                parse_connection_parameters(self)?,
            )),
            0x11 => {
                require_length(self, 2)?;
                Ok(DecodedControlPdu::RejectExtendedInd(RejectExtendedInd {
                    rejected_opcode: self.parameters[0],
                    error_code: self.parameters[1],
                }))
            }
            0x12 => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::PingRequest)
            }
            0x13 => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::PingResponse)
            }
            0x14 => Ok(DecodedControlPdu::LengthRequest(parse_data_length(self)?)),
            0x15 => Ok(DecodedControlPdu::LengthResponse(parse_data_length(self)?)),
            0x16 => Ok(DecodedControlPdu::PhyRequest(parse_phy_preferences(self)?)),
            0x17 => Ok(DecodedControlPdu::PhyResponse(parse_phy_preferences(self)?)),
            0x18 => Ok(DecodedControlPdu::PhyUpdateInd(parse_phy_update(self)?)),
            0x19 => Ok(DecodedControlPdu::MinimumUsedChannelsInd(
                parse_minimum_used_channels(self)?,
            )),
            0x1a => Ok(DecodedControlPdu::CteRequest(parse_cte_request(self)?)),
            0x1b => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::CteResponse)
            }
            0x1c => Ok(DecodedControlPdu::PeriodicSyncInd(parse_periodic_sync_ind(
                self.parameters,
            )?)),
            0x1d => Ok(DecodedControlPdu::ClockAccuracyRequest(
                parse_clock_accuracy(self)?,
            )),
            0x1e => Ok(DecodedControlPdu::ClockAccuracyResponse(
                parse_clock_accuracy(self)?,
            )),
            0x1f => Ok(DecodedControlPdu::CisRequest(parse_cis_request(self)?)),
            0x20 => Ok(DecodedControlPdu::CisResponse(parse_cis_response(self)?)),
            0x21 => Ok(DecodedControlPdu::CisInd(parse_cis_ind(self)?)),
            0x22 => Ok(DecodedControlPdu::CisTerminateInd(parse_cis_terminate(
                self,
            )?)),
            0x23 => Ok(DecodedControlPdu::PowerControlRequest(
                parse_power_control_request(self)?,
            )),
            0x24 => Ok(DecodedControlPdu::PowerControlResponse(
                parse_power_control_response(self)?,
            )),
            0x25 => Ok(DecodedControlPdu::PowerChangeInd(parse_power_change(self)?)),
            0x26 => Ok(DecodedControlPdu::SubrateRequest(parse_subrate_request(
                self,
            )?)),
            0x27 => Ok(DecodedControlPdu::SubrateInd(parse_subrate_ind(self)?)),
            0x28 => Ok(DecodedControlPdu::ChannelReportingInd(
                parse_channel_reporting(self)?,
            )),
            0x29 => Ok(DecodedControlPdu::ChannelStatusInd(parse_channel_status(
                self,
            )?)),
            0x2a => Ok(DecodedControlPdu::PeriodicSyncWrInd(
                parse_periodic_sync_wr(self)?,
            )),
            0x2b => Ok(DecodedControlPdu::FeatureExtendedRequest(
                parse_feature_page(self)?,
            )),
            0x2c => Ok(DecodedControlPdu::FeatureExtendedResponse(
                parse_feature_page(self)?,
            )),
            0x2d => Ok(DecodedControlPdu::CsSecurityResponse(
                cs::parse_cs_security(self)?,
            )),
            0x2e => Ok(DecodedControlPdu::CsCapabilitiesRequest(
                cs::parse_cs_capabilities(self)?,
            )),
            0x2f => Ok(DecodedControlPdu::CsCapabilitiesResponse(
                cs::parse_cs_capabilities(self)?,
            )),
            0x30 => Ok(DecodedControlPdu::CsConfigRequest(
                cs::parse_cs_config_request(self)?,
            )),
            0x31 => Ok(DecodedControlPdu::CsConfigResponse(
                cs::parse_cs_config_response(self)?,
            )),
            0x32 => Ok(DecodedControlPdu::CsProcedureRequest(
                cs::parse_cs_procedure_request(self)?,
            )),
            0x33 => Ok(DecodedControlPdu::CsProcedureResponse(
                cs::parse_cs_procedure_response(self)?,
            )),
            0x34 => Ok(DecodedControlPdu::CsProcedureIndication(
                cs::parse_cs_procedure_indication(self)?,
            )),
            0x35 => Ok(DecodedControlPdu::CsTerminateRequest(
                cs::parse_cs_termination(self)?,
            )),
            0x36 => {
                require_length(self, 0)?;
                Ok(DecodedControlPdu::CsFaeRequest)
            }
            0x37 => Ok(DecodedControlPdu::CsFaeResponse(cs::parse_cs_fae_response(
                self,
            )?)),
            0x38 => Ok(DecodedControlPdu::CsChannelMapInd(
                cs::parse_cs_channel_map_ind(self)?,
            )),
            0x39 => Ok(DecodedControlPdu::CsSecurityRequest(cs::parse_cs_security(
                self,
            )?)),
            0x3a => Ok(DecodedControlPdu::CsTerminateResponse(
                cs::parse_cs_termination(self)?,
            )),
            0x3b => Ok(DecodedControlPdu::FrameSpaceRequest(
                cs::parse_frame_space_request(self)?,
            )),
            0x3c => Ok(DecodedControlPdu::FrameSpaceResponse(
                cs::parse_frame_space_response(self)?,
            )),
            _ => Ok(DecodedControlPdu::Raw {
                opcode: self.opcode,
                parameters: self.parameters,
            }),
        }
    }
}

fn require_length(control: ControlPdu<'_>, expected: usize) -> Result<()> {
    if control.parameters.len() != expected {
        return Err(Error::InvalidInput(format!(
            "{} requires {expected} parameter octet(s), received {}",
            control.opcode_name(),
            control.parameters.len()
        )));
    }
    Ok(())
}

fn array<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    bytes[offset..offset + N]
        .try_into()
        .expect("caller validated fixed LL control PDU length")
}

fn le_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(array(bytes, offset))
}

fn le_u24(bytes: &[u8], offset: usize) -> u32 {
    u32::from(bytes[offset])
        | (u32::from(bytes[offset + 1]) << 8)
        | (u32::from(bytes[offset + 2]) << 16)
}

fn parse_feature_set(control: ControlPdu<'_>) -> Result<FeatureSet> {
    require_length(control, 8)?;
    Ok(FeatureSet {
        bytes: array(control.parameters, 0),
    })
}

fn parse_connection_parameters(control: ControlPdu<'_>) -> Result<ConnectionParameterPdu> {
    require_length(control, 23)?;
    let p = control.parameters;
    let value = ConnectionParameterPdu {
        interval_min: le_u16(p, 0),
        interval_max: le_u16(p, 2),
        latency: le_u16(p, 4),
        supervision_timeout: le_u16(p, 6),
        preferred_periodicity: p[8],
        reference_connection_event_count: le_u16(p, 9),
        offsets: [
            le_u16(p, 11),
            le_u16(p, 13),
            le_u16(p, 15),
            le_u16(p, 17),
            le_u16(p, 19),
            le_u16(p, 21),
        ],
    };
    if !(6..=3_200).contains(&value.interval_min) {
        return Err(Error::InvalidInput(format!(
            "{} interval minimum {} is outside 6..=3200",
            control.opcode_name(),
            value.interval_min
        )));
    }
    if value.interval_max < value.interval_min {
        return Err(Error::InvalidInput(format!(
            "{} interval maximum {} is below minimum {}",
            control.opcode_name(),
            value.interval_max,
            value.interval_min
        )));
    }
    ConnectionParameters::new(value.interval_max, value.latency, value.supervision_timeout)?;
    if u16::from(value.preferred_periodicity) > value.interval_max {
        return Err(Error::InvalidInput(format!(
            "{} preferred periodicity {} exceeds interval maximum {}",
            control.opcode_name(),
            value.preferred_periodicity,
            value.interval_max
        )));
    }
    let mut invalid_seen = false;
    for (index, offset) in value.offsets.iter().copied().enumerate() {
        if offset == u16::MAX {
            invalid_seen = true;
            continue;
        }
        if invalid_seen {
            return Err(Error::InvalidInput(format!(
                "{} offset{index} is valid after an invalid offset",
                control.opcode_name()
            )));
        }
        if offset >= value.interval_max {
            return Err(Error::InvalidInput(format!(
                "{} offset{index} {offset} is not below interval maximum {}",
                control.opcode_name(),
                value.interval_max
            )));
        }
        if value.offsets[..index].contains(&offset) {
            return Err(Error::InvalidInput(format!(
                "{} offset{index} duplicates an earlier valid offset",
                control.opcode_name()
            )));
        }
    }
    Ok(value)
}

fn parse_data_length(control: ControlPdu<'_>) -> Result<DataLengthPdu> {
    require_length(control, 8)?;
    let value = DataLengthPdu {
        maximum_receive_octets: le_u16(control.parameters, 0),
        maximum_receive_time_us: le_u16(control.parameters, 2),
        maximum_transmit_octets: le_u16(control.parameters, 4),
        maximum_transmit_time_us: le_u16(control.parameters, 6),
    };
    for (label, octets) in [
        ("maximum receive octets", value.maximum_receive_octets),
        ("maximum transmit octets", value.maximum_transmit_octets),
    ] {
        if !(27..=251).contains(&octets) {
            return Err(Error::InvalidInput(format!(
                "{} {label} {octets} is outside 27..=251",
                control.opcode_name()
            )));
        }
    }
    for (label, time) in [
        ("maximum receive time", value.maximum_receive_time_us),
        ("maximum transmit time", value.maximum_transmit_time_us),
    ] {
        if !(328..=17_040).contains(&time) {
            return Err(Error::InvalidInput(format!(
                "{} {label} {time} us is outside 328..=17040",
                control.opcode_name()
            )));
        }
    }
    Ok(value)
}

fn validate_phy_mask(mask: u8, allow_zero: bool, allow_multiple: bool, label: &str) -> Result<()> {
    if mask & !0x07 != 0 {
        return Err(Error::InvalidInput(format!(
            "{label} PHY mask 0x{mask:02x} sets reserved bits"
        )));
    }
    if !allow_zero && mask == 0 {
        return Err(Error::InvalidInput(format!(
            "{label} PHY mask must select at least one PHY"
        )));
    }
    if !allow_multiple && mask.count_ones() > 1 {
        return Err(Error::InvalidInput(format!(
            "{label} PHY mask 0x{mask:02x} selects more than one PHY"
        )));
    }
    Ok(())
}

fn parse_phy_preferences(control: ControlPdu<'_>) -> Result<PhyPreferences> {
    require_length(control, 2)?;
    validate_phy_mask(control.parameters[0], false, true, control.opcode_name())?;
    validate_phy_mask(control.parameters[1], false, true, control.opcode_name())?;
    Ok(PhyPreferences {
        transmit_phys: control.parameters[0],
        receive_phys: control.parameters[1],
    })
}

fn parse_phy_update(control: ControlPdu<'_>) -> Result<PhyUpdateInd> {
    require_length(control, 4)?;
    let value = PhyUpdateInd {
        central_to_peripheral_phy: control.parameters[0],
        peripheral_to_central_phy: control.parameters[1],
        instant: le_u16(control.parameters, 2),
    };
    validate_phy_mask(
        value.central_to_peripheral_phy,
        true,
        false,
        "LL_PHY_UPDATE_IND central-to-peripheral",
    )?;
    validate_phy_mask(
        value.peripheral_to_central_phy,
        true,
        false,
        "LL_PHY_UPDATE_IND peripheral-to-central",
    )?;
    if value.central_to_peripheral_phy == 0
        && value.peripheral_to_central_phy == 0
        && value.instant != 0
    {
        return Err(Error::InvalidInput(
            "LL_PHY_UPDATE_IND reserves Instant when neither PHY changes".to_owned(),
        ));
    }
    Ok(value)
}

fn parse_minimum_used_channels(control: ControlPdu<'_>) -> Result<MinimumUsedChannelsInd> {
    require_length(control, 2)?;
    validate_phy_mask(
        control.parameters[0],
        false,
        true,
        "LL_MIN_USED_CHANNELS_IND",
    )?;
    if !(2..=37).contains(&control.parameters[1]) {
        return Err(Error::InvalidInput(format!(
            "LL_MIN_USED_CHANNELS_IND minimum {} is outside 2..=37",
            control.parameters[1]
        )));
    }
    Ok(MinimumUsedChannelsInd {
        phys: control.parameters[0],
        minimum_used_channels: control.parameters[1],
    })
}

fn parse_cte_request(control: ControlPdu<'_>) -> Result<CteRequest> {
    require_length(control, 1)?;
    let raw = control.parameters[0];
    let value = CteRequest {
        minimum_length_units: raw & 0x1f,
        cte_type: raw >> 6,
    };
    if raw & 0x20 != 0 {
        return Err(Error::InvalidInput(
            "LL_CTE_REQ sets its reserved bit".to_owned(),
        ));
    }
    if !(2..=20).contains(&value.minimum_length_units) {
        return Err(Error::InvalidInput(format!(
            "LL_CTE_REQ minimum CTE length {} is outside 2..=20",
            value.minimum_length_units
        )));
    }
    if value.cte_type > 2 {
        return Err(Error::InvalidInput(
            "LL_CTE_REQ uses reserved CTE type 3".to_owned(),
        ));
    }
    Ok(value)
}

fn parse_sync_info(bytes: &[u8]) -> Result<PeriodicSyncInfo> {
    let offset = le_u16(bytes, 0);
    if offset & 0x8000 != 0 {
        return Err(Error::InvalidInput(
            "periodic SyncInfo sets its reserved offset bit".to_owned(),
        ));
    }
    let offset_units_300_us = offset & 0x2000 != 0;
    let offset_adjust = offset & 0x4000 != 0;
    if !offset_units_300_us && offset_adjust {
        return Err(Error::InvalidInput(
            "periodic SyncInfo sets Offset Adjust with 30 us units".to_owned(),
        ));
    }
    let offset_base = offset & 0x1fff;
    if offset_units_300_us && !offset_adjust && u32::from(offset_base) * 300 < 245_700 {
        return Err(Error::InvalidInput(
            "periodic SyncInfo uses 300 us units for an offset below 245700 us".to_owned(),
        ));
    }
    let interval = le_u16(bytes, 2);
    if interval < 6 {
        return Err(Error::InvalidInput(format!(
            "periodic SyncInfo interval {interval} is below 6"
        )));
    }
    let channel_map =
        DataChannelMap::new([bytes[4], bytes[5], bytes[6], bytes[7], bytes[8] & 0x1f])?;
    Ok(PeriodicSyncInfo {
        offset_base,
        offset_units_300_us,
        offset_adjust,
        interval,
        channel_map,
        advertiser_sleep_clock_accuracy: SleepClockAccuracy::new(bytes[8] >> 5)?,
        access_address: u32::from_le_bytes(array(bytes, 9)),
        crc_init: le_u24(bytes, 13),
        periodic_event_counter: le_u16(bytes, 16),
    })
}

fn parse_periodic_sync_ind(parameters: &[u8]) -> Result<PeriodicSyncInd> {
    if parameters.len() != 34 {
        return Err(Error::InvalidInput(format!(
            "LL_PERIODIC_SYNC_IND requires 34 parameter octets, received {}",
            parameters.len()
        )));
    }
    let phy = parameters[25];
    validate_phy_mask(phy, false, false, "LL_PERIODIC_SYNC_IND")?;
    let identity = parameters[24];
    Ok(PeriodicSyncInd {
        identifier: le_u16(parameters, 0),
        sync_info: parse_sync_info(&parameters[2..20])?,
        connection_event_count: le_u16(parameters, 20),
        last_periodic_event_counter: le_u16(parameters, 22),
        advertising_sid: identity & 0x0f,
        advertiser_address_random: identity & 0x10 != 0,
        sender_sleep_clock_accuracy: SleepClockAccuracy::new(identity >> 5)?,
        phy,
        advertiser_address: array(parameters, 26),
        sync_connection_event_count: le_u16(parameters, 32),
    })
}

fn parse_clock_accuracy(control: ControlPdu<'_>) -> Result<SleepClockAccuracy> {
    require_length(control, 1)?;
    SleepClockAccuracy::new(control.parameters[0])
}

fn parse_cis_request(control: ControlPdu<'_>) -> Result<CisRequest> {
    require_length(control, 35)?;
    let p = control.parameters;
    validate_phy_mask(p[2], false, false, "LL_CIS_REQ central-to-peripheral")?;
    validate_phy_mask(p[3], false, false, "LL_CIS_REQ peripheral-to-central")?;
    let central_sdu = le_u16(p, 4);
    if central_sdu & 0x3000 != 0 {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ central Max_SDU sets reserved bits".to_owned(),
        ));
    }
    let maximum_central_sdu = central_sdu & 0x0fff;
    let framing_mode_unsegmented = central_sdu & 0x4000 != 0;
    let framed = central_sdu & 0x8000 != 0;
    if !framed && framing_mode_unsegmented {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ sets Framing_Mode for unframed data".to_owned(),
        ));
    }
    let peripheral_sdu = le_u16(p, 6);
    if peripheral_sdu & 0xf000 != 0 {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ peripheral Max_SDU sets reserved bits".to_owned(),
        ));
    }
    let central_sdu_interval = le_u24(p, 8);
    let peripheral_sdu_interval = le_u24(p, 11);
    if central_sdu_interval & 0xf0_0000 != 0 || peripheral_sdu_interval & 0xf0_0000 != 0 {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ SDU interval sets reserved high bits".to_owned(),
        ));
    }
    let central_sdu_interval_us = central_sdu_interval & 0x0f_ffff;
    let peripheral_sdu_interval_us = peripheral_sdu_interval & 0x0f_ffff;
    for (label, interval) in [
        ("central", central_sdu_interval_us),
        ("peripheral", peripheral_sdu_interval_us),
    ] {
        if !(255..=1_048_575).contains(&interval) {
            return Err(Error::InvalidInput(format!(
                "LL_CIS_REQ {label} SDU interval {interval} us is outside 255..=1048575"
            )));
        }
    }
    let maximum_central_pdu = le_u16(p, 14);
    let maximum_peripheral_pdu = le_u16(p, 16);
    if maximum_central_pdu > 251 || maximum_peripheral_pdu > 251 {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ Max_PDU exceeds 251 octets".to_owned(),
        ));
    }
    let subevents = p[18];
    if !(1..=31).contains(&subevents) {
        return Err(Error::InvalidInput(format!(
            "LL_CIS_REQ NSE {subevents} is outside 1..=31"
        )));
    }
    let subevent_interval_us = le_u24(p, 19);
    let central_burst_number = p[22] & 0x0f;
    let peripheral_burst_number = p[22] >> 4;
    if (maximum_central_pdu == 0) != (central_burst_number == 0)
        || (maximum_peripheral_pdu == 0) != (peripheral_burst_number == 0)
    {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ Max_PDU must be zero exactly when the corresponding BN is zero".to_owned(),
        ));
    }
    let central_flush_timeout = p[23];
    let peripheral_flush_timeout = p[24];
    if central_flush_timeout == 0 || peripheral_flush_timeout == 0 {
        return Err(Error::InvalidInput(
            "LL_CIS_REQ flush timeout must be nonzero".to_owned(),
        ));
    }
    let iso_interval = le_u16(p, 25);
    if !(4..=3_200).contains(&iso_interval) {
        return Err(Error::InvalidInput(format!(
            "LL_CIS_REQ ISO interval {iso_interval} is outside 4..=3200"
        )));
    }
    if subevents == 1 {
        if subevent_interval_us != 0 {
            return Err(Error::InvalidInput(
                "LL_CIS_REQ Sub_Interval must be zero when NSE is one".to_owned(),
            ));
        }
    } else if subevent_interval_us < 400 || subevent_interval_us >= u32::from(iso_interval) * 1_250
    {
        return Err(Error::InvalidInput(format!(
            "LL_CIS_REQ Sub_Interval {subevent_interval_us} us is invalid for NSE and ISO interval"
        )));
    }
    let cis_offset_min_us = le_u24(p, 27);
    let cis_offset_max_us = le_u24(p, 30);
    validate_cis_offsets(cis_offset_min_us, cis_offset_max_us, "LL_CIS_REQ")?;
    Ok(CisRequest {
        cig_identifier: p[0],
        cis_identifier: p[1],
        central_to_peripheral_phy: p[2],
        peripheral_to_central_phy: p[3],
        maximum_central_sdu,
        framed,
        framing_mode_unsegmented,
        maximum_peripheral_sdu: peripheral_sdu & 0x0fff,
        central_sdu_interval_us,
        peripheral_sdu_interval_us,
        maximum_central_pdu,
        maximum_peripheral_pdu,
        subevents,
        subevent_interval_us,
        central_burst_number,
        peripheral_burst_number,
        central_flush_timeout,
        peripheral_flush_timeout,
        iso_interval,
        cis_offset_min_us,
        cis_offset_max_us,
        connection_event_count: le_u16(p, 33),
    })
}

fn validate_cis_offsets(minimum: u32, maximum: u32, pdu_name: &str) -> Result<()> {
    if minimum < 500 {
        return Err(Error::InvalidInput(format!(
            "{pdu_name} CIS offset minimum {minimum} us is below 500"
        )));
    }
    if maximum < minimum {
        return Err(Error::InvalidInput(format!(
            "{pdu_name} CIS offset maximum {maximum} us is below minimum {minimum} us"
        )));
    }
    Ok(())
}

fn parse_cis_response(control: ControlPdu<'_>) -> Result<CisResponse> {
    require_length(control, 8)?;
    let value = CisResponse {
        cis_offset_min_us: le_u24(control.parameters, 0),
        cis_offset_max_us: le_u24(control.parameters, 3),
        connection_event_count: le_u16(control.parameters, 6),
    };
    validate_cis_offsets(
        value.cis_offset_min_us,
        value.cis_offset_max_us,
        control.opcode_name(),
    )?;
    Ok(value)
}

fn parse_cis_ind(control: ControlPdu<'_>) -> Result<CisInd> {
    require_length(control, 15)?;
    Ok(CisInd {
        access_address: u32::from_le_bytes(array(control.parameters, 0)),
        cis_offset_us: le_u24(control.parameters, 4),
        cig_sync_delay_us: le_u24(control.parameters, 7),
        cis_sync_delay_us: le_u24(control.parameters, 10),
        connection_event_count: le_u16(control.parameters, 13),
    })
}

fn parse_cis_terminate(control: ControlPdu<'_>) -> Result<CisTerminateInd> {
    require_length(control, 3)?;
    Ok(CisTerminateInd {
        cig_identifier: control.parameters[0],
        cis_identifier: control.parameters[1],
        error_code: control.parameters[2],
    })
}

fn validate_power_phy(mask: u8, allow_multiple: bool, label: &str) -> Result<()> {
    if mask == 0 || mask & !0x0f != 0 {
        return Err(Error::InvalidInput(format!(
            "{label} PHY mask 0x{mask:02x} is empty or sets reserved bits"
        )));
    }
    if !allow_multiple && mask.count_ones() != 1 {
        return Err(Error::InvalidInput(format!(
            "{label} PHY mask 0x{mask:02x} must select exactly one PHY"
        )));
    }
    Ok(())
}

fn parse_power_control_request(control: ControlPdu<'_>) -> Result<PowerControlRequest> {
    require_length(control, 3)?;
    validate_power_phy(control.parameters[0], false, control.opcode_name())?;
    let value = PowerControlRequest {
        phy: control.parameters[0],
        delta_db: control.parameters[1] as i8,
        transmit_power_dbm: control.parameters[2] as i8,
    };
    if value.transmit_power_dbm == 126 {
        return Err(Error::InvalidInput(
            "LL_POWER_CONTROL_REQ reserves TxPower value 126".to_owned(),
        ));
    }
    Ok(value)
}

fn parse_power_flags(raw: u8, pdu_name: &str) -> Result<(bool, bool)> {
    if raw & 0xfc != 0 {
        return Err(Error::InvalidInput(format!(
            "{pdu_name} power flags set reserved bits"
        )));
    }
    Ok((raw & 1 != 0, raw & 2 != 0))
}

fn parse_power_control_response(control: ControlPdu<'_>) -> Result<PowerControlResponse> {
    require_length(control, 4)?;
    let (at_minimum, at_maximum) = parse_power_flags(control.parameters[0], control.opcode_name())?;
    Ok(PowerControlResponse {
        at_minimum,
        at_maximum,
        delta_db: control.parameters[1] as i8,
        transmit_power_dbm: control.parameters[2] as i8,
        acceptable_power_reduction_db: control.parameters[3],
    })
}

fn parse_power_change(control: ControlPdu<'_>) -> Result<PowerChangeInd> {
    require_length(control, 4)?;
    validate_power_phy(control.parameters[0], true, control.opcode_name())?;
    let (at_minimum, at_maximum) = parse_power_flags(control.parameters[1], control.opcode_name())?;
    Ok(PowerChangeInd {
        phys: control.parameters[0],
        at_minimum,
        at_maximum,
        delta_db: control.parameters[2] as i8,
        transmit_power_dbm: control.parameters[3] as i8,
    })
}

fn validate_subrate_factor(factor: u16, label: &str) -> Result<()> {
    if !(1..=500).contains(&factor) {
        return Err(Error::InvalidInput(format!(
            "{label} subrate factor {factor} is outside 1..=500"
        )));
    }
    Ok(())
}

fn validate_supervision_timeout(timeout: u16, label: &str) -> Result<()> {
    if !(10..=3_200).contains(&timeout) {
        return Err(Error::InvalidInput(format!(
            "{label} supervision timeout {timeout} is outside 10..=3200"
        )));
    }
    Ok(())
}

fn parse_subrate_request(control: ControlPdu<'_>) -> Result<SubrateRequest> {
    require_length(control, 10)?;
    let value = SubrateRequest {
        factor_min: le_u16(control.parameters, 0),
        factor_max: le_u16(control.parameters, 2),
        maximum_latency: le_u16(control.parameters, 4),
        continuation_number: le_u16(control.parameters, 6),
        supervision_timeout: le_u16(control.parameters, 8),
    };
    validate_subrate_factor(value.factor_min, control.opcode_name())?;
    validate_subrate_factor(value.factor_max, control.opcode_name())?;
    if value.factor_max < value.factor_min {
        return Err(Error::InvalidInput(
            "LL_SUBRATE_REQ maximum factor is below minimum factor".to_owned(),
        ));
    }
    if u32::from(value.factor_max) * (u32::from(value.maximum_latency) + 1) > 500 {
        return Err(Error::InvalidInput(
            "LL_SUBRATE_REQ factor and maximum latency exceed 500".to_owned(),
        ));
    }
    if value.continuation_number >= value.factor_min {
        return Err(Error::InvalidInput(
            "LL_SUBRATE_REQ continuation number is not below the minimum factor".to_owned(),
        ));
    }
    validate_supervision_timeout(value.supervision_timeout, control.opcode_name())?;
    Ok(value)
}

fn parse_subrate_ind(control: ControlPdu<'_>) -> Result<SubrateInd> {
    require_length(control, 10)?;
    let value = SubrateInd {
        factor: le_u16(control.parameters, 0),
        base_event: le_u16(control.parameters, 2),
        latency: le_u16(control.parameters, 4),
        continuation_number: le_u16(control.parameters, 6),
        supervision_timeout: le_u16(control.parameters, 8),
    };
    validate_subrate_factor(value.factor, control.opcode_name())?;
    if u32::from(value.factor) * (u32::from(value.latency) + 1) > 500 {
        return Err(Error::InvalidInput(
            "LL_SUBRATE_IND factor and latency exceed 500".to_owned(),
        ));
    }
    if value.continuation_number >= value.factor {
        return Err(Error::InvalidInput(
            "LL_SUBRATE_IND continuation number is not below the factor".to_owned(),
        ));
    }
    validate_supervision_timeout(value.supervision_timeout, control.opcode_name())?;
    Ok(value)
}

fn parse_channel_reporting(control: ControlPdu<'_>) -> Result<ChannelReportingInd> {
    require_length(control, 3)?;
    if control.parameters[0] > 1 {
        return Err(Error::InvalidInput(format!(
            "LL_CHANNEL_REPORTING_IND Enable value {} is reserved",
            control.parameters[0]
        )));
    }
    let minimum_spacing = control.parameters[1];
    let maximum_delay = control.parameters[2];
    if !(5..=150).contains(&minimum_spacing) {
        return Err(Error::InvalidInput(format!(
            "LL_CHANNEL_REPORTING_IND minimum spacing {minimum_spacing} is outside 5..=150"
        )));
    }
    if !(5..=150).contains(&maximum_delay) || maximum_delay < minimum_spacing {
        return Err(Error::InvalidInput(format!(
            "LL_CHANNEL_REPORTING_IND maximum delay {maximum_delay} is invalid for minimum spacing {minimum_spacing}"
        )));
    }
    Ok(ChannelReportingInd {
        enabled: control.parameters[0] != 0,
        minimum_spacing,
        maximum_delay,
    })
}

fn parse_channel_status(control: ControlPdu<'_>) -> Result<ChannelStatusInd> {
    require_length(control, 10)?;
    if control.parameters[9] & 0xfc != 0 {
        return Err(Error::InvalidInput(
            "LL_CHANNEL_STATUS_IND sets reserved trailing bits".to_owned(),
        ));
    }
    let mut classifications = [ChannelClassification::Unknown; 37];
    for (channel, classification) in classifications.iter_mut().enumerate() {
        let raw = (control.parameters[channel / 4] >> ((channel % 4) * 2)) & 0x03;
        *classification = match raw {
            0 => ChannelClassification::Unknown,
            1 => ChannelClassification::Good,
            2 => {
                return Err(Error::InvalidInput(format!(
                    "LL_CHANNEL_STATUS_IND channel {channel} uses reserved classification 2"
                )));
            }
            3 => ChannelClassification::Bad,
            _ => unreachable!(),
        };
    }
    Ok(ChannelStatusInd { classifications })
}

fn parse_periodic_sync_wr(control: ControlPdu<'_>) -> Result<PeriodicSyncWrInd> {
    require_length(control, 42)?;
    Ok(PeriodicSyncWrInd {
        periodic_sync: parse_periodic_sync_ind(&control.parameters[..34])?,
        response_access_address: u32::from_le_bytes(array(control.parameters, 34)),
        subevent_count: control.parameters[38],
        subevent_interval: control.parameters[39],
        response_slot_delay: control.parameters[40],
        response_slot_spacing: control.parameters[41],
    })
}

fn parse_feature_page(control: ControlPdu<'_>) -> Result<FeaturePagePdu> {
    require_length(control, 26)?;
    let value = FeaturePagePdu {
        maximum_page: control.parameters[0],
        page_number: control.parameters[1],
        feature_page: array(control.parameters, 2),
    };
    if value.maximum_page > 0x0a {
        return Err(Error::InvalidInput(format!(
            "{} maximum page {} exceeds 10",
            control.opcode_name(),
            value.maximum_page
        )));
    }
    if !(1..=0x0a).contains(&value.page_number) {
        return Err(Error::InvalidInput(format!(
            "{} page number {} is outside 1..=10",
            control.opcode_name(),
            value.page_number
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control(opcode: u8, parameters: &[u8]) -> ControlPdu<'_> {
        ControlPdu { opcode, parameters }
    }

    #[test]
    fn decodes_fixed_and_cryptographic_control_pdus() {
        assert_eq!(
            control(0x02, &[0x13]).decode().unwrap(),
            DecodedControlPdu::TerminateInd(ErrorIndication { error_code: 0x13 })
        );
        let encryption_request: Vec<u8> = (0..22).collect();
        assert_eq!(
            control(0x03, &encryption_request).decode().unwrap(),
            DecodedControlPdu::EncryptionRequest(EncryptionRequest {
                random_number: [0, 1, 2, 3, 4, 5, 6, 7],
                encrypted_diversifier: 0x0908,
                central_session_key_diversifier: [10, 11, 12, 13, 14, 15, 16, 17],
                central_initialization_vector: [18, 19, 20, 21],
            })
        );
        assert!(matches!(
            control(0x05, &[]).decode().unwrap(),
            DecodedControlPdu::StartEncryptionRequest
        ));
        assert!(matches!(
            control(0x12, &[]).decode().unwrap(),
            DecodedControlPdu::PingRequest
        ));
    }

    #[test]
    fn decodes_connection_parameters_and_rejects_invalid_offsets() {
        let parameters = [
            24, 0, 40, 0, 0, 0, 200, 0, 20, 0x34, 0x12, 1, 0, 2, 0, 3, 0, 4, 0, 0xff, 0xff, 0xff,
            0xff,
        ];
        let DecodedControlPdu::ConnectionParameterRequest(value) =
            control(0x0f, &parameters).decode().unwrap()
        else {
            panic!("unexpected decoded command");
        };
        assert_eq!(value.interval_min, 24);
        assert_eq!(value.interval_max, 40);
        assert_eq!(value.offsets, [1, 2, 3, 4, u16::MAX, u16::MAX]);

        let mut duplicate = parameters;
        duplicate[15] = 2;
        duplicate[16] = 0;
        assert!(control(0x0f, &duplicate).decode().is_err());

        let mut valid_after_invalid = parameters;
        valid_after_invalid[13] = 0xff;
        valid_after_invalid[14] = 0xff;
        assert!(control(0x10, &valid_after_invalid).decode().is_err());
    }

    #[test]
    fn validates_length_phy_cte_and_clock_fields() {
        let length = [251, 0, 0x48, 0x08, 27, 0, 0x48, 0x08];
        assert!(matches!(
            control(0x14, &length).decode().unwrap(),
            DecodedControlPdu::LengthRequest(_)
        ));
        let mut short_time = length;
        short_time[2..4].copy_from_slice(&327u16.to_le_bytes());
        assert!(control(0x14, &short_time).decode().is_err());
        assert!(control(0x16, &[0x03, 0x04]).decode().is_ok());
        assert!(control(0x16, &[0x00, 0x04]).decode().is_err());
        assert!(control(0x18, &[0, 0, 0, 0]).decode().is_ok());
        assert!(control(0x18, &[0, 0, 1, 0]).decode().is_err());
        assert!(control(0x19, &[0x07, 37]).decode().is_ok());
        assert!(control(0x1a, &[0x8a]).decode().is_ok());
        assert!(control(0x1a, &[0xaa]).decode().is_err());
        assert!(control(0x1d, &[7]).decode().is_ok());
        assert!(control(0x1d, &[8]).decode().is_err());
    }

    #[test]
    fn decodes_periodic_sync_and_feature_pages() {
        let mut parameters = vec![0u8; 34];
        parameters[0..2].copy_from_slice(&0x1234u16.to_le_bytes());
        parameters[2..4].copy_from_slice(&3u16.to_le_bytes());
        parameters[4..6].copy_from_slice(&24u16.to_le_bytes());
        parameters[6..11].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x1f]);
        parameters[11..15].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        parameters[15..18].copy_from_slice(&[0xef, 0xcd, 0xab]);
        parameters[18..20].copy_from_slice(&9u16.to_le_bytes());
        parameters[20..22].copy_from_slice(&10u16.to_le_bytes());
        parameters[22..24].copy_from_slice(&9u16.to_le_bytes());
        parameters[24] = 0xc5;
        parameters[25] = 0x02;
        parameters[26..32].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        parameters[32..34].copy_from_slice(&8u16.to_le_bytes());
        let DecodedControlPdu::PeriodicSyncInd(value) =
            control(0x1c, &parameters).decode().unwrap()
        else {
            panic!("unexpected decoded command");
        };
        assert_eq!(value.sync_info.packet_window_offset_us(), 90);
        assert_eq!(value.sync_info.access_address, 0x1234_5678);
        assert_eq!(value.advertising_sid, 5);
        assert_eq!(value.sender_sleep_clock_accuracy.raw(), 6);

        let mut wrong_offset_units = parameters.clone();
        wrong_offset_units[2..4].copy_from_slice(&0x2003u16.to_le_bytes());
        assert!(control(0x1c, &wrong_offset_units).decode().is_err());

        let mut with_response = parameters;
        with_response.extend_from_slice(&0x1020_3040u32.to_le_bytes());
        with_response.extend_from_slice(&[4, 8, 2, 3]);
        assert!(matches!(
            control(0x2a, &with_response).decode().unwrap(),
            DecodedControlPdu::PeriodicSyncWrInd(_)
        ));

        let mut feature_page = vec![0x0a, 0x03];
        feature_page.extend(0u8..24);
        let DecodedControlPdu::FeatureExtendedRequest(page) =
            control(0x2b, &feature_page).decode().unwrap()
        else {
            panic!("unexpected decoded command");
        };
        assert_eq!(page.feature_page[23], 23);
    }

    #[test]
    fn validates_cis_layout_including_core_61_framing_mode() {
        let mut parameters = vec![0u8; 35];
        parameters[0..4].copy_from_slice(&[1, 2, 1, 2]);
        parameters[4..6].copy_from_slice(&0xc064u16.to_le_bytes());
        parameters[6..8].copy_from_slice(&80u16.to_le_bytes());
        parameters[8..11].copy_from_slice(&[0xe8, 0x03, 0]);
        parameters[11..14].copy_from_slice(&[0xd0, 0x07, 0]);
        parameters[14..16].copy_from_slice(&100u16.to_le_bytes());
        parameters[16..18].copy_from_slice(&80u16.to_le_bytes());
        parameters[18] = 2;
        parameters[19..22].copy_from_slice(&[0xf4, 0x01, 0]);
        parameters[22] = 0x11;
        parameters[23] = 2;
        parameters[24] = 3;
        parameters[25..27].copy_from_slice(&8u16.to_le_bytes());
        parameters[27..30].copy_from_slice(&[0xf4, 0x01, 0]);
        parameters[30..33].copy_from_slice(&[0x58, 0x02, 0]);
        parameters[33..35].copy_from_slice(&7u16.to_le_bytes());
        let DecodedControlPdu::CisRequest(value) = control(0x1f, &parameters).decode().unwrap()
        else {
            panic!("unexpected decoded command");
        };
        assert!(value.framed);
        assert!(value.framing_mode_unsegmented);
        assert_eq!(value.maximum_central_sdu, 100);

        parameters[4..6].copy_from_slice(&0x4064u16.to_le_bytes());
        assert!(control(0x1f, &parameters).decode().is_err());
    }

    #[test]
    fn validates_power_subrate_and_channel_reporting_pdus() {
        assert!(control(0x23, &[0x08, 0xff, 0x7f]).decode().is_ok());
        assert!(control(0x23, &[0x03, 0, 0]).decode().is_err());
        assert!(control(0x24, &[0x03, 0, 126, 0xff]).decode().is_ok());
        assert!(control(0x25, &[0x0f, 0, 1, 2]).decode().is_ok());

        let subrate_request = [2, 0, 5, 0, 9, 0, 1, 0, 200, 0];
        assert!(control(0x26, &subrate_request).decode().is_ok());
        let subrate_ind = [5, 0, 9, 0, 9, 0, 4, 0, 200, 0];
        assert!(control(0x27, &subrate_ind).decode().is_ok());
        assert!(control(0x28, &[1, 5, 150]).decode().is_ok());
        assert!(control(0x28, &[1, 10, 9]).decode().is_err());

        let mut classifications = [0u8; 10];
        classifications[0] = 0b1101_0001;
        let DecodedControlPdu::ChannelStatusInd(status) =
            control(0x29, &classifications).decode().unwrap()
        else {
            panic!("unexpected decoded command");
        };
        assert_eq!(status.classifications[0], ChannelClassification::Good);
        assert_eq!(status.classifications[1], ChannelClassification::Unknown);
        assert_eq!(status.classifications[2], ChannelClassification::Good);
        assert_eq!(status.classifications[3], ChannelClassification::Bad);
    }

    #[test]
    fn matches_independent_scapy_control_vectors() {
        let vectors: &[(u8, &[u8])] = &[
            (
                0x03,
                &[
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
                ],
            ),
            (0x0c, &[0x0d, 0x34, 0x12, 0x78, 0x56]),
            (0x14, &[0xfb, 0x00, 0x48, 0x08, 0x1b, 0x00, 0x48, 0x08]),
            (0x1a, &[0x8a]),
            (0x23, &[0x08, 0xff, 0x7f]),
            (0x24, &[0x03, 0xfe, 0x7e, 0xff]),
            (0x25, &[0x0f, 0x01, 0x01, 0x02]),
            (
                0x26,
                &[0x02, 0x00, 0x05, 0x00, 0x09, 0x00, 0x01, 0x00, 0xc8, 0x00],
            ),
            (
                0x27,
                &[0x05, 0x00, 0x09, 0x00, 0x09, 0x00, 0x04, 0x00, 0xc8, 0x00],
            ),
            (0x28, &[0x01, 0x05, 0x96]),
        ];
        for &(opcode, parameters) in vectors {
            assert!(
                control(opcode, parameters).decode().is_ok(),
                "Scapy vector for opcode 0x{opcode:02x} failed"
            );
        }
    }

    #[test]
    fn rejects_every_known_short_and_long_parameter_layout() {
        let lengths = [
            11, 7, 1, 22, 12, 0, 0, 1, 8, 8, 0, 0, 5, 1, 8, 23, 23, 2, 0, 0, 8, 8, 2, 2, 4, 2, 1,
            0, 34, 1, 1, 35, 8, 15, 3, 3, 4, 4, 10, 10, 3, 10, 42, 26, 26, 20, 25, 25, 27, 1, 28,
            21, 18, 4, 0, 72, 12, 20, 4, 7, 5,
        ];
        for (opcode, expected) in (0u8..=0x3c).zip(lengths) {
            if expected > 0 {
                assert!(
                    control(opcode, &vec![0; expected - 1]).decode().is_err(),
                    "opcode 0x{opcode:02x} accepted a short layout"
                );
            }
            assert!(
                control(opcode, &vec![0; expected + 1]).decode().is_err(),
                "opcode 0x{opcode:02x} accepted a long layout"
            );
        }
    }

    #[test]
    fn decodes_newly_assigned_and_preserves_future_opcodes_losslessly() {
        assert_eq!(control(0x2d, &[]).opcode_name(), "LL_CS_SEC_RSP");
        assert_eq!(control(0x39, &[]).opcode_name(), "LL_CS_SEC_REQ");
        assert_eq!(control(0x3c, &[]).opcode_name(), "LL_FRAME_SPACE_RSP");
        assert!(matches!(
            control(0x36, &[]).decode().unwrap(),
            DecodedControlPdu::CsFaeRequest
        ));
        assert_eq!(
            control(0xee, &[4, 5]).decode().unwrap(),
            DecodedControlPdu::Raw {
                opcode: 0xee,
                parameters: &[4, 5],
            }
        );
    }

    #[test]
    fn bounded_arbitrary_inputs_never_panic() {
        for opcode in 0u8..=u8::MAX {
            for length in 0..=80 {
                let parameters: Vec<u8> = (0..length)
                    .map(|index| opcode.wrapping_add(index as u8))
                    .collect();
                let _ = control(opcode, &parameters).decode();
            }
        }
    }
}
