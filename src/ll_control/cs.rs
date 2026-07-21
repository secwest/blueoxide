use super::{array, le_u16, le_u24, require_length};
use crate::link_layer::ControlPdu;
use crate::{Error, Result};

pub const CS_CHANNEL_MAP_OCTETS: usize = 10;
pub const CS_FAE_CHANNELS: usize = 72;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsSecurityParameters {
    pub initialization_vector: [u8; 8],
    pub nonce: [u8; 4],
    pub personalization_vector: [u8; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsCapabilities {
    pub mode_types: u8,
    pub rtt_capability: u8,
    pub rtt_aa_only_n: u8,
    pub rtt_sounding_n: u8,
    pub rtt_random_sequence_n: u8,
    pub nadm_sounding_capability: u16,
    pub nadm_random_capability: u16,
    pub cs_sync_phy_capability: u8,
    pub antenna_count: u8,
    pub maximum_antenna_paths: u8,
    pub roles: u8,
    pub no_fae: bool,
    pub channel_selection_3c: bool,
    pub sounding_pct_estimate: bool,
    pub configuration_count: u8,
    pub maximum_procedures_supported: u16,
    pub antenna_switch_time_us: u8,
    pub t_ip1_capability: u16,
    pub t_ip2_capability: u16,
    pub t_fcs_capability: u16,
    pub t_pm_capability: u16,
    /// Bit 0 represents SNR output index 0 after removing the wire RFU bit.
    pub tx_snr_capability: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsChannelMap {
    bytes: [u8; CS_CHANNEL_MAP_OCTETS],
}

impl CsChannelMap {
    pub const fn bytes(&self) -> [u8; CS_CHANNEL_MAP_OCTETS] {
        self.bytes
    }

    pub fn used_count(&self) -> u32 {
        self.bytes.iter().map(|byte| byte.count_ones()).sum()
    }

    pub fn is_used(&self, channel: u8) -> bool {
        channel < 79 && self.bytes[channel as usize / 8] & (1 << (channel % 8)) != 0
    }

    fn from_valid_bytes(bytes: [u8; CS_CHANNEL_MAP_OCTETS]) -> Self {
        Self { bytes }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CsConfigAction {
    Remove,
    Create,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsConfigRequest {
    pub config_id: u8,
    pub action: CsConfigAction,
    pub channel_map: CsChannelMap,
    pub channel_map_repetition: u8,
    pub main_mode: u8,
    pub sub_mode: u8,
    pub main_mode_min_steps: u8,
    pub main_mode_max_steps: u8,
    pub main_mode_repetition: u8,
    pub mode_0_steps: u8,
    pub cs_sync_phy: u8,
    pub rtt_type: u8,
    pub role: u8,
    pub channel_selection: u8,
    pub channel_selection_3c_shape: u8,
    pub channel_selection_3c_jump: u8,
    pub t_ip1: u8,
    pub t_ip2: u8,
    pub t_fcs: u8,
    pub t_pm: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsConfigResponse {
    pub config_id: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsProcedureRequest {
    pub config_id: u8,
    pub connection_event_count: u16,
    pub offset_min_us: u32,
    pub offset_max_us: u32,
    pub maximum_procedure_length_units: u16,
    pub event_interval_connection_events: u16,
    pub subevents_per_event: u8,
    pub subevent_interval_units: u16,
    pub subevent_length_us: u32,
    pub procedure_interval_connection_events: u16,
    pub procedure_count: u16,
    pub antenna_configuration_index: u8,
    pub preferred_peer_antennas: u8,
    pub phy: u8,
    pub power_delta_db: i8,
    pub initiator_snr_index: u8,
    pub reflector_snr_index: u8,
}

impl CsProcedureRequest {
    pub const fn maximum_procedure_length_us(self) -> u32 {
        self.maximum_procedure_length_units as u32 * 625
    }

    pub const fn subevent_interval_us(self) -> u32 {
        self.subevent_interval_units as u32 * 625
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsProcedureResponse {
    pub config_id: u8,
    pub connection_event_count: u16,
    pub offset_min_us: u32,
    pub offset_max_us: u32,
    pub event_interval_connection_events: u16,
    pub subevents_per_event: u8,
    pub subevent_interval_units: u16,
    pub subevent_length_us: u32,
    pub antenna_configuration_index: u8,
    pub phy: u8,
    pub power_delta_db: i8,
}

impl CsProcedureResponse {
    pub const fn subevent_interval_us(self) -> u32 {
        self.subevent_interval_units as u32 * 625
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsProcedureIndication {
    pub config_id: u8,
    pub connection_event_count: u16,
    pub offset_us: u32,
    pub event_interval_connection_events: u16,
    pub subevents_per_event: u8,
    pub subevent_interval_units: u16,
    pub subevent_length_us: u32,
    pub antenna_configuration_index: u8,
    pub phy: u8,
    pub power_delta_db: i8,
}

impl CsProcedureIndication {
    pub const fn subevent_interval_us(self) -> u32 {
        self.subevent_interval_units as u32 * 625
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsTermination {
    pub config_id: u8,
    pub procedure_count: u16,
    pub error_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsFaeTable {
    pub values: [i8; CS_FAE_CHANNELS],
}

impl CsFaeTable {
    pub fn ppm(&self, index: usize) -> Option<f32> {
        self.values.get(index).map(|value| f32::from(*value) / 32.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsChannelMapInd {
    pub channel_map: CsChannelMap,
    pub instant: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameSpaceRequest {
    pub minimum_us: u16,
    pub maximum_us: u16,
    pub phys: u8,
    pub spacing_types: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameSpaceResponse {
    pub frame_space_us: u16,
    pub phys: u8,
    pub spacing_types: u16,
}

pub(super) fn parse_cs_security(control: ControlPdu<'_>) -> Result<CsSecurityParameters> {
    require_length(control, 20)?;
    Ok(CsSecurityParameters {
        initialization_vector: array(control.parameters, 0),
        nonce: array(control.parameters, 8),
        personalization_vector: array(control.parameters, 12),
    })
}

pub(super) fn parse_cs_capabilities(control: ControlPdu<'_>) -> Result<CsCapabilities> {
    require_length(control, 25)?;
    let p = control.parameters;
    if p[0] & !0x01 != 0 {
        return invalid(control, "Mode_Types sets reserved bits");
    }
    if p[1] & !0x07 != 0 {
        return invalid(control, "RTT_Capability sets reserved bits");
    }
    if p[2] == 0 {
        return invalid(control, "RTT_AA_Only_N must be nonzero");
    }
    for (bit, n, label) in [
        (0x01, p[2], "RTT_AA_Only_N"),
        (0x02, p[3], "RTT_Sounding_N"),
        (0x04, p[4], "RTT_Random_Sequence_N"),
    ] {
        if n == 0 && p[1] & bit != 0 {
            return invalid(
                control,
                &format!("RTT_Capability selects {label} while its N value is zero"),
            );
        }
    }
    let nadm_sounding_capability = le_u16(p, 5);
    let nadm_random_capability = le_u16(p, 7);
    if nadm_sounding_capability & !0x0001 != 0 || nadm_random_capability & !0x0001 != 0 {
        return invalid(control, "NADM capability sets reserved bits");
    }
    if p[9] & !0x06 != 0 {
        return invalid(control, "CS_SYNC_PHY_Capability sets reserved bits");
    }
    let antenna_count = p[10] & 0x0f;
    let maximum_antenna_paths = p[10] >> 4;
    if !(1..=4).contains(&antenna_count) {
        return invalid(control, "Num_Ant is outside 1..=4");
    }
    if !(antenna_count..=4).contains(&maximum_antenna_paths) {
        return invalid(control, "Max_Ant_Path is outside Num_Ant..=4");
    }
    if p[11] & 0xc4 != 0 {
        return invalid(control, "role and subfeature octet sets reserved bits");
    }
    let roles = p[11] & 0x03;
    if roles == 0 {
        return invalid(control, "Role must select at least one CS role");
    }
    let sounding_pct_estimate = p[11] & 0x20 != 0;
    if sounding_pct_estimate && p[3] == 0 {
        return invalid(
            control,
            "Sounding_PCT_Estimate requires nonzero RTT_Sounding_N",
        );
    }
    if !(1..=4).contains(&p[12]) {
        return invalid(control, "Num_Configs is outside 1..=4");
    }
    if !matches!(p[15], 0 | 1 | 2 | 4 | 10) {
        return invalid(control, "T_SW is not 0, 1, 2, 4, or 10 us");
    }
    let t_ip1_capability = le_u16(p, 16);
    let t_ip2_capability = le_u16(p, 18);
    let t_fcs_capability = le_u16(p, 20);
    let t_pm_capability = le_u16(p, 22);
    if t_ip1_capability & !0x007f != 0 || t_ip2_capability & !0x007f != 0 {
        return invalid(control, "T_IP capability sets reserved bits");
    }
    if t_fcs_capability & !0x01ff != 0 {
        return invalid(control, "T_FCS_Capability sets reserved bits");
    }
    if t_pm_capability & !0x0003 != 0 {
        return invalid(control, "T_PM_Capability sets reserved bits");
    }
    if p[24] & !0x3e != 0 {
        return invalid(control, "TX_SNR_Capability sets RFU or reserved bits");
    }
    Ok(CsCapabilities {
        mode_types: p[0],
        rtt_capability: p[1],
        rtt_aa_only_n: p[2],
        rtt_sounding_n: p[3],
        rtt_random_sequence_n: p[4],
        nadm_sounding_capability,
        nadm_random_capability,
        cs_sync_phy_capability: p[9],
        antenna_count,
        maximum_antenna_paths,
        roles,
        no_fae: p[11] & 0x08 != 0,
        channel_selection_3c: p[11] & 0x10 != 0,
        sounding_pct_estimate,
        configuration_count: p[12],
        maximum_procedures_supported: le_u16(p, 13),
        antenna_switch_time_us: p[15],
        t_ip1_capability,
        t_ip2_capability,
        t_fcs_capability,
        t_pm_capability,
        tx_snr_capability: p[24] >> 1,
    })
}

pub(super) fn parse_cs_config_request(control: ControlPdu<'_>) -> Result<CsConfigRequest> {
    require_length(control, 27)?;
    let p = control.parameters;
    let config_id = p[0] & 0x3f;
    if config_id > 3 {
        return invalid(control, "Config_ID exceeds 3");
    }
    let action = match p[0] >> 6 {
        0 => CsConfigAction::Remove,
        1 => CsConfigAction::Create,
        _ => return invalid(control, "Action is reserved"),
    };
    if action == CsConfigAction::Remove {
        if p[1..].iter().any(|byte| *byte != 0) {
            return invalid(control, "remove action has nonzero RFU fields");
        }
        return Ok(CsConfigRequest {
            config_id,
            action,
            channel_map: CsChannelMap::from_valid_bytes([0; CS_CHANNEL_MAP_OCTETS]),
            channel_map_repetition: 0,
            main_mode: 0,
            sub_mode: 0,
            main_mode_min_steps: 0,
            main_mode_max_steps: 0,
            main_mode_repetition: 0,
            mode_0_steps: 0,
            cs_sync_phy: 0,
            rtt_type: 0,
            role: 0,
            channel_selection: 0,
            channel_selection_3c_shape: 0,
            channel_selection_3c_jump: 0,
            t_ip1: 0,
            t_ip2: 0,
            t_fcs: 0,
            t_pm: 0,
        });
    }

    let channel_map = parse_channel_map(array(p, 1), control)?;
    if p[11] == 0 {
        return invalid(control, "ChM_Repetition must be nonzero");
    }
    let main_mode = p[12];
    let sub_mode = p[13];
    if !matches!(
        (main_mode, sub_mode),
        (1, 0xff) | (2, 0xff) | (3, 0xff) | (2, 1) | (2, 3) | (3, 2)
    ) {
        return invalid(control, "Main_Mode and Sub_Mode combination is reserved");
    }
    if sub_mode == 0xff {
        if p[14] != 0 || p[15] != 0 {
            return invalid(control, "None Sub_Mode has nonzero RFU step limits");
        }
    } else if p[14] == 0 || p[15] < p[14] {
        return invalid(control, "main-mode step range is invalid");
    }
    if p[16] > 3 {
        return invalid(control, "Main_Mode_Repetition exceeds 3");
    }
    if !(1..=3).contains(&p[17]) {
        return invalid(control, "Mode_0_Steps is outside 1..=3");
    }
    validate_cs_phy(p[18], control)?;
    let rtt_type = p[19] & 0x0f;
    let role = (p[19] >> 4) & 0x03;
    if p[19] & 0xc0 != 0 {
        return invalid(control, "RTT_Type and Role octet sets reserved bits");
    }
    if rtt_type > 6 {
        return invalid(control, "RTT_Type is reserved");
    }
    if role > 1 {
        return invalid(control, "Role is reserved");
    }
    let channel_selection = p[20] & 0x0f;
    let channel_selection_3c_shape = p[20] >> 4;
    let channel_selection_3c_jump = p[21];
    match channel_selection {
        0 => {
            if channel_selection_3c_shape != 0 || channel_selection_3c_jump != 0 {
                return invalid(control, "algorithm #3b has nonzero #3c RFU fields");
            }
        }
        1 => {
            if channel_selection_3c_shape > 1 {
                return invalid(control, "Ch3cShape is reserved");
            }
            if !(2..=8).contains(&channel_selection_3c_jump) {
                return invalid(control, "Ch3cJump is outside 2..=8");
            }
            let maximum_repetitions = match channel_selection_3c_jump {
                2 | 3 => 1,
                4 | 5 => 2,
                6..=8 => 3,
                _ => unreachable!(),
            };
            if p[11] > maximum_repetitions {
                return invalid(
                    control,
                    "ChM_Repetition exceeds the selected #3c jump limit",
                );
            }
        }
        _ => return invalid(control, "ChSel is reserved"),
    }
    if p[22] > 7 || p[23] > 7 {
        return invalid(control, "T_IP1 or T_IP2 index exceeds 7");
    }
    if p[24] > 9 {
        return invalid(control, "T_FCS index exceeds 9");
    }
    if p[25] > 2 {
        return invalid(control, "T_PM index exceeds 2");
    }
    if p[26] != 0 {
        return invalid(control, "trailing RFU octet is nonzero");
    }
    Ok(CsConfigRequest {
        config_id,
        action,
        channel_map,
        channel_map_repetition: p[11],
        main_mode,
        sub_mode,
        main_mode_min_steps: p[14],
        main_mode_max_steps: p[15],
        main_mode_repetition: p[16],
        mode_0_steps: p[17],
        cs_sync_phy: p[18],
        rtt_type,
        role,
        channel_selection,
        channel_selection_3c_shape,
        channel_selection_3c_jump,
        t_ip1: p[22],
        t_ip2: p[23],
        t_fcs: p[24],
        t_pm: p[25],
    })
}

pub(super) fn parse_cs_config_response(control: ControlPdu<'_>) -> Result<CsConfigResponse> {
    require_length(control, 1)?;
    Ok(CsConfigResponse {
        config_id: validate_config_id(control.parameters[0], control)?,
    })
}

pub(super) fn parse_cs_procedure_request(control: ControlPdu<'_>) -> Result<CsProcedureRequest> {
    require_length(control, 28)?;
    let p = control.parameters;
    let value = CsProcedureRequest {
        config_id: validate_config_id(p[0], control)?,
        connection_event_count: le_u16(p, 1),
        offset_min_us: le_u24(p, 3),
        offset_max_us: le_u24(p, 6),
        maximum_procedure_length_units: le_u16(p, 9),
        event_interval_connection_events: le_u16(p, 11),
        subevents_per_event: p[13],
        subevent_interval_units: le_u16(p, 14),
        subevent_length_us: le_u24(p, 16),
        procedure_interval_connection_events: le_u16(p, 19),
        procedure_count: le_u16(p, 21),
        antenna_configuration_index: p[23],
        preferred_peer_antennas: p[24],
        phy: p[25],
        power_delta_db: p[26] as i8,
        initiator_snr_index: p[27] & 0x0f,
        reflector_snr_index: p[27] >> 4,
    };
    validate_offsets(value.offset_min_us, value.offset_max_us, control)?;
    validate_procedure_common(
        value.event_interval_connection_events,
        value.subevents_per_event,
        value.subevent_interval_units,
        value.subevent_length_us,
        value.antenna_configuration_index,
        value.phy,
        control,
    )?;
    if value.preferred_peer_antennas & !0x0f != 0 {
        return invalid(control, "Preferred_Peer_Ant sets reserved bits");
    }
    if value.procedure_count == 1 && value.procedure_interval_connection_events != 0 {
        return invalid(
            control,
            "Procedure_Interval must be zero when Procedure_Count is one",
        );
    }
    validate_snr_index(value.initiator_snr_index, "TX_SNR_I", control)?;
    validate_snr_index(value.reflector_snr_index, "TX_SNR_R", control)?;
    Ok(value)
}

pub(super) fn parse_cs_procedure_response(control: ControlPdu<'_>) -> Result<CsProcedureResponse> {
    require_length(control, 21)?;
    let p = control.parameters;
    if p[20] != 0 {
        return invalid(control, "trailing RFU octet is nonzero");
    }
    let value = CsProcedureResponse {
        config_id: validate_config_id(p[0], control)?,
        connection_event_count: le_u16(p, 1),
        offset_min_us: le_u24(p, 3),
        offset_max_us: le_u24(p, 6),
        event_interval_connection_events: le_u16(p, 9),
        subevents_per_event: p[11],
        subevent_interval_units: le_u16(p, 12),
        subevent_length_us: le_u24(p, 14),
        antenna_configuration_index: p[17],
        phy: p[18],
        power_delta_db: p[19] as i8,
    };
    validate_offsets(value.offset_min_us, value.offset_max_us, control)?;
    validate_procedure_common(
        value.event_interval_connection_events,
        value.subevents_per_event,
        value.subevent_interval_units,
        value.subevent_length_us,
        value.antenna_configuration_index,
        value.phy,
        control,
    )?;
    Ok(value)
}

pub(super) fn parse_cs_procedure_indication(
    control: ControlPdu<'_>,
) -> Result<CsProcedureIndication> {
    require_length(control, 18)?;
    let p = control.parameters;
    if p[17] != 0 {
        return invalid(control, "trailing RFU octet is nonzero");
    }
    let value = CsProcedureIndication {
        config_id: validate_config_id(p[0], control)?,
        connection_event_count: le_u16(p, 1),
        offset_us: le_u24(p, 3),
        event_interval_connection_events: le_u16(p, 6),
        subevents_per_event: p[8],
        subevent_interval_units: le_u16(p, 9),
        subevent_length_us: le_u24(p, 11),
        antenna_configuration_index: p[14],
        phy: p[15],
        power_delta_db: p[16] as i8,
    };
    validate_procedure_common(
        value.event_interval_connection_events,
        value.subevents_per_event,
        value.subevent_interval_units,
        value.subevent_length_us,
        value.antenna_configuration_index,
        value.phy,
        control,
    )?;
    Ok(value)
}

pub(super) fn parse_cs_termination(control: ControlPdu<'_>) -> Result<CsTermination> {
    require_length(control, 4)?;
    Ok(CsTermination {
        config_id: validate_config_id(control.parameters[0], control)?,
        procedure_count: le_u16(control.parameters, 1),
        error_code: control.parameters[3],
    })
}

pub(super) fn parse_cs_fae_response(control: ControlPdu<'_>) -> Result<CsFaeTable> {
    require_length(control, CS_FAE_CHANNELS)?;
    Ok(CsFaeTable {
        values: std::array::from_fn(|index| control.parameters[index] as i8),
    })
}

pub(super) fn parse_cs_channel_map_ind(control: ControlPdu<'_>) -> Result<CsChannelMapInd> {
    require_length(control, 12)?;
    Ok(CsChannelMapInd {
        channel_map: parse_channel_map(array(control.parameters, 0), control)?,
        instant: le_u16(control.parameters, 10),
    })
}

pub(super) fn parse_frame_space_request(control: ControlPdu<'_>) -> Result<FrameSpaceRequest> {
    require_length(control, 7)?;
    let value = FrameSpaceRequest {
        minimum_us: le_u16(control.parameters, 0),
        maximum_us: le_u16(control.parameters, 2),
        phys: control.parameters[4],
        spacing_types: le_u16(control.parameters, 5),
    };
    if value.maximum_us < value.minimum_us || value.maximum_us > 10_000 {
        return invalid(control, "frame-space range is invalid");
    }
    validate_frame_space_masks(value.phys, value.spacing_types, false, control)?;
    Ok(value)
}

pub(super) fn parse_frame_space_response(control: ControlPdu<'_>) -> Result<FrameSpaceResponse> {
    require_length(control, 5)?;
    let value = FrameSpaceResponse {
        frame_space_us: le_u16(control.parameters, 0),
        phys: control.parameters[2],
        spacing_types: le_u16(control.parameters, 3),
    };
    if value.frame_space_us > 10_000 {
        return invalid(control, "FS exceeds 10000 us");
    }
    validate_frame_space_masks(value.phys, value.spacing_types, true, control)?;
    Ok(value)
}

fn invalid<T>(control: ControlPdu<'_>, message: &str) -> Result<T> {
    Err(Error::InvalidInput(format!(
        "{} {message}",
        control.opcode_name()
    )))
}

fn validate_config_id(raw: u8, control: ControlPdu<'_>) -> Result<u8> {
    if raw & 0xc0 != 0 {
        return invalid(control, "Config_ID octet sets reserved bits");
    }
    let config_id = raw & 0x3f;
    if config_id > 3 {
        return invalid(control, "Config_ID exceeds 3");
    }
    Ok(config_id)
}

fn parse_channel_map(
    bytes: [u8; CS_CHANNEL_MAP_OCTETS],
    control: ControlPdu<'_>,
) -> Result<CsChannelMap> {
    if bytes[9] & 0x80 != 0 {
        return invalid(control, "channel map sets RFU bit 79");
    }
    let map = CsChannelMap::from_valid_bytes(bytes);
    for channel in [0, 1, 23, 24, 25, 77, 78] {
        if map.is_used(channel) {
            return invalid(
                control,
                &format!("channel map selects excluded channel {channel}"),
            );
        }
    }
    if map.used_count() < 15 {
        return invalid(control, "channel map selects fewer than 15 channels");
    }
    Ok(map)
}

fn validate_cs_phy(phy: u8, control: ControlPdu<'_>) -> Result<()> {
    if phy & !0x0f != 0 || phy.count_ones() != 1 {
        return invalid(
            control,
            &format!("PHY mask 0x{phy:02x} must select exactly one CS PHY"),
        );
    }
    Ok(())
}

fn validate_offsets(minimum: u32, maximum: u32, control: ControlPdu<'_>) -> Result<()> {
    if !(500..4_000_000).contains(&minimum) {
        return invalid(control, "Offset_Min is outside 500..4000000 us");
    }
    if maximum < minimum || maximum >= 4_000_000 {
        return invalid(control, "Offset_Max is invalid for Offset_Min");
    }
    Ok(())
}

fn validate_procedure_common(
    event_interval: u16,
    subevents_per_event: u8,
    subevent_interval: u16,
    subevent_length_us: u32,
    antenna_configuration_index: u8,
    phy: u8,
    control: ControlPdu<'_>,
) -> Result<()> {
    if event_interval == 0 {
        return invalid(control, "Event_Interval must be nonzero");
    }
    if !(1..=32).contains(&subevents_per_event) {
        return invalid(control, "Subevents_Per_Event is outside 1..=32");
    }
    if subevents_per_event == 1 && subevent_interval != 0 {
        return invalid(control, "Subevent_Interval must be zero for one subevent");
    }
    if !(1_250..4_000_000).contains(&subevent_length_us) {
        return invalid(control, "Subevent_Len is outside 1250..4000000 us");
    }
    if antenna_configuration_index > 7 {
        return invalid(control, "ACI exceeds 7");
    }
    validate_cs_phy(phy, control)
}

fn validate_snr_index(index: u8, label: &str, control: ControlPdu<'_>) -> Result<()> {
    if index > 4 && index != 0x0f {
        return invalid(control, &format!("{label} index is reserved"));
    }
    Ok(())
}

fn validate_frame_space_masks(
    phys: u8,
    spacing_types: u16,
    allow_zero: bool,
    control: ControlPdu<'_>,
) -> Result<()> {
    if phys & !0x07 != 0 {
        return invalid(control, "PHYS sets reserved bits");
    }
    if spacing_types & !0x001f != 0 {
        return invalid(control, "Spacing_Types sets reserved bits");
    }
    if !allow_zero && (phys == 0 || spacing_types == 0) {
        return invalid(
            control,
            "PHYS and Spacing_Types must each select at least one value",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control(opcode: u8, parameters: &[u8]) -> ControlPdu<'_> {
        ControlPdu { opcode, parameters }
    }

    fn valid_channel_map() -> [u8; CS_CHANNEL_MAP_OCTETS] {
        [0xfc, 0xff, 0x7f, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1f]
    }

    fn valid_capabilities() -> [u8; 25] {
        [
            0x01, 0x07, 1, 2, 3, 1, 0, 1, 0, 0x06, 0x42, 0x3b, 4, 0x34, 0x12, 10, 0x7f, 0, 0x7f, 0,
            0xff, 1, 3, 0, 0x3e,
        ]
    }

    fn valid_config() -> [u8; 27] {
        let mut parameters = [0u8; 27];
        parameters[0] = 0x42;
        parameters[1..11].copy_from_slice(&valid_channel_map());
        parameters[11..22].copy_from_slice(&[1, 2, 3, 1, 4, 2, 3, 0x08, 0x14, 0x11, 2]);
        parameters[22..26].copy_from_slice(&[7, 7, 9, 2]);
        parameters
    }

    fn valid_request() -> [u8; 28] {
        let mut parameters = [0u8; 28];
        parameters[0] = 2;
        parameters[1..3].copy_from_slice(&0x1234u16.to_le_bytes());
        parameters[3..6].copy_from_slice(&[0xf4, 0x01, 0]);
        parameters[6..9].copy_from_slice(&[0xe8, 0x03, 0]);
        parameters[9..11].copy_from_slice(&8u16.to_le_bytes());
        parameters[11..13].copy_from_slice(&2u16.to_le_bytes());
        parameters[13] = 2;
        parameters[14..16].copy_from_slice(&4u16.to_le_bytes());
        parameters[16..19].copy_from_slice(&[0xe2, 0x04, 0]);
        parameters[19..21].copy_from_slice(&10u16.to_le_bytes());
        parameters[21..23].copy_from_slice(&3u16.to_le_bytes());
        parameters[23..28].copy_from_slice(&[7, 0x0f, 0x08, 0xfe, 0xf4]);
        parameters
    }

    #[test]
    fn decodes_security_capabilities_and_fae_layouts() {
        let security: Vec<u8> = (0..20).collect();
        assert_eq!(
            parse_cs_security(control(0x39, &security)).unwrap(),
            CsSecurityParameters {
                initialization_vector: [0, 1, 2, 3, 4, 5, 6, 7],
                nonce: [8, 9, 10, 11],
                personalization_vector: [12, 13, 14, 15, 16, 17, 18, 19],
            }
        );

        let capabilities = parse_cs_capabilities(control(0x2e, &valid_capabilities())).unwrap();
        assert_eq!(capabilities.antenna_count, 2);
        assert_eq!(capabilities.maximum_antenna_paths, 4);
        assert_eq!(capabilities.tx_snr_capability, 0x1f);

        let mut fae: Vec<u8> = (0..CS_FAE_CHANNELS as u8).collect();
        fae[0] = 0x80;
        fae[1] = 0xff;
        let table = parse_cs_fae_response(control(0x37, &fae)).unwrap();
        assert_eq!(table.values[0], -128);
        assert_eq!(table.values[1], -1);
        assert_eq!(table.values[71], 71);
        assert_eq!(table.ppm(0), Some(-4.0));
        assert_eq!(table.ppm(64), Some(2.0));
    }

    #[test]
    fn rejects_invalid_capability_masks_and_relationships() {
        let base = valid_capabilities();
        for (index, value) in [
            (0, 0x02),
            (1, 0x08),
            (5, 0x02),
            (9, 0x01),
            (11, 0x04),
            (12, 0),
            (15, 3),
            (17, 1),
            (21, 2),
            (23, 4),
            (24, 1),
        ] {
            let mut invalid = base;
            invalid[index] = value;
            assert!(
                parse_cs_capabilities(control(0x2e, &invalid)).is_err(),
                "capability octet {index} accepted 0x{value:02x}"
            );
        }
        let mut no_aa_only = base;
        no_aa_only[2] = 0;
        assert!(parse_cs_capabilities(control(0x2e, &no_aa_only)).is_err());
        let mut missing_sounding = base;
        missing_sounding[3] = 0;
        assert!(parse_cs_capabilities(control(0x2e, &missing_sounding)).is_err());
        let mut bad_antennas = base;
        bad_antennas[10] = 0x24;
        assert!(parse_cs_capabilities(control(0x2f, &bad_antennas)).is_err());
        let mut no_role = base;
        no_role[11] &= !0x03;
        assert!(parse_cs_capabilities(control(0x2f, &no_role)).is_err());
    }

    #[test]
    fn validates_config_create_remove_and_channel_selection() {
        let value = parse_cs_config_request(control(0x30, &valid_config())).unwrap();
        assert_eq!(value.config_id, 2);
        assert_eq!(value.action, CsConfigAction::Create);
        assert_eq!(value.channel_map.used_count(), 72);
        assert_eq!(value.main_mode, 2);
        assert_eq!(value.sub_mode, 3);

        let mut remove = [0u8; 27];
        remove[0] = 3;
        assert_eq!(
            parse_cs_config_request(control(0x30, &remove))
                .unwrap()
                .action,
            CsConfigAction::Remove
        );
        remove[26] = 1;
        assert!(parse_cs_config_request(control(0x30, &remove)).is_err());

        let mut invalid = valid_config();
        invalid[13] = 2;
        assert!(parse_cs_config_request(control(0x30, &invalid)).is_err());
        invalid = valid_config();
        invalid[20] = 0;
        assert!(parse_cs_config_request(control(0x30, &invalid)).is_err());
        invalid = valid_config();
        invalid[11] = 2;
        invalid[21] = 2;
        assert!(parse_cs_config_request(control(0x30, &invalid)).is_err());
        invalid = valid_config();
        invalid[22] = 8;
        assert!(parse_cs_config_request(control(0x30, &invalid)).is_err());
        for (index, value) in [
            (14, 0),
            (16, 4),
            (17, 0),
            (18, 3),
            (19, 7),
            (20, 0x21),
            (21, 9),
            (24, 10),
            (25, 3),
            (26, 1),
        ] {
            invalid = valid_config();
            invalid[index] = value;
            assert!(
                parse_cs_config_request(control(0x30, &invalid)).is_err(),
                "configuration octet {index} accepted 0x{value:02x}"
            );
        }
        invalid = valid_config();
        invalid[19] = 0x20;
        assert!(parse_cs_config_request(control(0x30, &invalid)).is_err());
    }

    #[test]
    fn validates_channel_maps_and_config_responses() {
        let mut parameters = [0u8; 12];
        parameters[..10].copy_from_slice(&valid_channel_map());
        parameters[10..].copy_from_slice(&0x1234u16.to_le_bytes());
        let indication = parse_cs_channel_map_ind(control(0x38, &parameters)).unwrap();
        assert_eq!(indication.channel_map.used_count(), 72);
        assert_eq!(indication.instant, 0x1234);

        for channel in [0u8, 1, 23, 24, 25, 77, 78, 79] {
            let mut invalid = parameters;
            invalid[channel as usize / 8] |= 1 << (channel % 8);
            assert!(
                parse_cs_channel_map_ind(control(0x38, &invalid)).is_err(),
                "excluded or RFU channel {channel} was accepted"
            );
        }
        parameters[..10].fill(0);
        assert!(parse_cs_channel_map_ind(control(0x38, &parameters)).is_err());
        assert_eq!(
            parse_cs_config_response(control(0x31, &[3])).unwrap(),
            CsConfigResponse { config_id: 3 }
        );
        assert!(parse_cs_config_response(control(0x31, &[4])).is_err());
        assert!(parse_cs_config_response(control(0x31, &[0x40])).is_err());
    }

    #[test]
    fn decodes_request_response_and_indication_layouts() {
        let request = parse_cs_procedure_request(control(0x32, &valid_request())).unwrap();
        assert_eq!(request.connection_event_count, 0x1234);
        assert_eq!(request.offset_min_us, 500);
        assert_eq!(request.offset_max_us, 1000);
        assert_eq!(request.maximum_procedure_length_us(), 5000);
        assert_eq!(request.subevent_interval_us(), 2500);
        assert_eq!(request.power_delta_db, -2);
        assert_eq!(request.initiator_snr_index, 4);
        assert_eq!(request.reflector_snr_index, 0x0f);

        let mut response = [0u8; 21];
        response[0..2].copy_from_slice(&[2, 0x34]);
        response[2] = 0x12;
        response[3..9].copy_from_slice(&[0xf4, 1, 0, 0xe8, 3, 0]);
        response[9..12].copy_from_slice(&[2, 0, 2]);
        response[12..14].copy_from_slice(&4u16.to_le_bytes());
        response[14..17].copy_from_slice(&[0xe2, 4, 0]);
        response[17..20].copy_from_slice(&[7, 8, 0xfe]);
        let response = parse_cs_procedure_response(control(0x33, &response)).unwrap();
        assert_eq!(response.offset_max_us, 1000);
        assert_eq!(response.power_delta_db, -2);

        let mut indication = [0u8; 18];
        indication[0..3].copy_from_slice(&[2, 0x34, 0x12]);
        indication[3..6].copy_from_slice(&[0, 0, 0]);
        indication[6..9].copy_from_slice(&[2, 0, 1]);
        indication[9..11].copy_from_slice(&0u16.to_le_bytes());
        indication[11..14].copy_from_slice(&[0xe2, 4, 0]);
        indication[14..17].copy_from_slice(&[7, 8, 0xfe]);
        let indication = parse_cs_procedure_indication(control(0x34, &indication)).unwrap();
        assert_eq!(indication.offset_us, 0);
        assert_eq!(indication.subevent_interval_us(), 0);
    }

    #[test]
    fn rejects_invalid_procedure_fields() {
        let base = valid_request();
        for mutate in [
            |p: &mut [u8; 28]| p[0] = 4,
            |p: &mut [u8; 28]| p[3..6].copy_from_slice(&[0xf3, 1, 0]),
            |p: &mut [u8; 28]| p[6..9].copy_from_slice(&[0xf3, 1, 0]),
            |p: &mut [u8; 28]| p[11..13].copy_from_slice(&0u16.to_le_bytes()),
            |p: &mut [u8; 28]| p[13] = 33,
            |p: &mut [u8; 28]| {
                p[13] = 1;
                p[14] = 1;
            },
            |p: &mut [u8; 28]| p[16..19].copy_from_slice(&[0xe1, 4, 0]),
            |p: &mut [u8; 28]| p[23] = 8,
            |p: &mut [u8; 28]| p[24] = 0x10,
            |p: &mut [u8; 28]| p[25] = 3,
            |p: &mut [u8; 28]| p[27] = 0x55,
            |p: &mut [u8; 28]| {
                p[21..23].copy_from_slice(&1u16.to_le_bytes());
                p[19..21].copy_from_slice(&1u16.to_le_bytes());
            },
        ] {
            let mut invalid = base;
            mutate(&mut invalid);
            assert!(parse_cs_procedure_request(control(0x32, &invalid)).is_err());
        }

        let mut response = [0u8; 21];
        response[0..3].copy_from_slice(&[2, 0x34, 0x12]);
        response[3..9].copy_from_slice(&[0xf4, 1, 0, 0xe8, 3, 0]);
        response[9..12].copy_from_slice(&[2, 0, 1]);
        response[14..17].copy_from_slice(&[0xe2, 4, 0]);
        response[17..20].copy_from_slice(&[7, 8, 0xfe]);
        for (index, value) in [(0, 0x40), (3, 0xf3), (11, 0), (17, 8), (18, 3), (20, 1)] {
            let mut invalid = response;
            invalid[index] = value;
            assert!(
                parse_cs_procedure_response(control(0x33, &invalid)).is_err(),
                "response octet {index} accepted 0x{value:02x}"
            );
        }

        let mut indication = [0u8; 18];
        indication[0..3].copy_from_slice(&[2, 0x34, 0x12]);
        indication[6..9].copy_from_slice(&[2, 0, 1]);
        indication[11..14].copy_from_slice(&[0xe2, 4, 0]);
        indication[14..17].copy_from_slice(&[7, 8, 0xfe]);
        for (index, value) in [(0, 0x40), (6, 0), (8, 0), (14, 8), (15, 3), (17, 1)] {
            let mut invalid = indication;
            invalid[index] = value;
            assert!(
                parse_cs_procedure_indication(control(0x34, &invalid)).is_err(),
                "indication octet {index} accepted 0x{value:02x}"
            );
        }
    }

    #[test]
    fn decodes_termination_and_frame_space_layouts() {
        assert_eq!(
            parse_cs_termination(control(0x35, &[2, 0x34, 0x12, 0x13])).unwrap(),
            CsTermination {
                config_id: 2,
                procedure_count: 0x1234,
                error_code: 0x13,
            }
        );
        assert_eq!(
            parse_frame_space_request(control(0x3b, &[100, 0, 200, 0, 3, 5, 0])).unwrap(),
            FrameSpaceRequest {
                minimum_us: 100,
                maximum_us: 200,
                phys: 3,
                spacing_types: 5,
            }
        );
        assert_eq!(
            parse_frame_space_response(control(0x3c, &[150, 0, 0, 0, 0])).unwrap(),
            FrameSpaceResponse {
                frame_space_us: 150,
                phys: 0,
                spacing_types: 0,
            }
        );
        assert!(parse_frame_space_request(control(0x3b, &[201, 0, 200, 0, 1, 1, 0])).is_err());
        assert!(parse_frame_space_request(control(0x3b, &[0, 0, 0x11, 0x27, 0, 1, 0])).is_err());
        assert!(parse_frame_space_request(control(0x3b, &[0, 0, 1, 0, 1, 0x20, 0])).is_err());
        assert!(parse_frame_space_response(control(0x3c, &[0x11, 0x27, 8, 1, 0])).is_err());
        assert!(parse_frame_space_response(control(0x3c, &[0, 0, 1, 0x20, 0])).is_err());
    }

    #[test]
    fn decodes_every_core_61_cs_and_frame_space_opcode() {
        let security: Vec<u8> = (0..20).collect();
        let capabilities = valid_capabilities();
        let config = valid_config();
        let request = valid_request();

        let mut response = [0u8; 21];
        response[0..3].copy_from_slice(&[2, 0x34, 0x12]);
        response[3..9].copy_from_slice(&[0xf4, 1, 0, 0xe8, 3, 0]);
        response[9..12].copy_from_slice(&[2, 0, 1]);
        response[14..17].copy_from_slice(&[0xe2, 4, 0]);
        response[17..20].copy_from_slice(&[7, 8, 0xfe]);

        let mut indication = [0u8; 18];
        indication[0..3].copy_from_slice(&[2, 0x34, 0x12]);
        indication[6..9].copy_from_slice(&[2, 0, 1]);
        indication[11..14].copy_from_slice(&[0xe2, 4, 0]);
        indication[14..17].copy_from_slice(&[7, 8, 0xfe]);

        let mut channel_map = [0u8; 12];
        channel_map[..10].copy_from_slice(&valid_channel_map());
        channel_map[10..].copy_from_slice(&0x1234u16.to_le_bytes());
        let fae = [0u8; CS_FAE_CHANNELS];
        let cases: &[(u8, &[u8])] = &[
            (0x2d, &security),
            (0x2e, &capabilities),
            (0x2f, &capabilities),
            (0x30, &config),
            (0x31, &[2]),
            (0x32, &request),
            (0x33, &response),
            (0x34, &indication),
            (0x35, &[2, 0x34, 0x12, 0x13]),
            (0x36, &[]),
            (0x37, &fae),
            (0x38, &channel_map),
            (0x39, &security),
            (0x3a, &[2, 0x34, 0x12, 0x13]),
            (0x3b, &[100, 0, 200, 0, 3, 5, 0]),
            (0x3c, &[150, 0, 0, 0, 0]),
        ];
        for &(opcode, parameters) in cases {
            let decoded = control(opcode, parameters).decode().unwrap();
            assert!(
                !matches!(decoded, super::super::DecodedControlPdu::Raw { .. }),
                "opcode 0x{opcode:02x} remained raw"
            );
        }
    }

    #[test]
    fn matches_pinned_rootcanal_and_ti_layout_vectors() {
        // RootCanal LL/CS/CEN/INI/BV-01-C configuration fields.
        let mut rootcanal = [0u8; 27];
        rootcanal[0] = 0x42;
        rootcanal[1..11].copy_from_slice(&valid_channel_map());
        rootcanal[11..22].copy_from_slice(&[1, 1, 0xff, 0, 0, 0, 3, 1, 0, 1, 2]);
        let config = parse_cs_config_request(control(0x30, &rootcanal)).unwrap();
        assert_eq!(config.config_id, 2);
        assert_eq!(config.channel_map.bytes(), valid_channel_map());
        assert_eq!(config.main_mode, 1);
        assert_eq!(config.sub_mode, 0xff);
        assert_eq!(config.cs_sync_phy, 1);
        assert_eq!(config.channel_selection_3c_jump, 2);

        // TI's packed csReq_t ordering, with its fields serialized little-endian.
        let ti = valid_request();
        let request = parse_cs_procedure_request(control(0x32, &ti)).unwrap();
        assert_eq!(request.offset_min_us, 500);
        assert_eq!(request.offset_max_us, 1000);
        assert_eq!(request.antenna_configuration_index, 7);
        assert_eq!(request.power_delta_db, -2);
        assert_eq!(request.reflector_snr_index, 0x0f);
    }
}
