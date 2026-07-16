use crate::ble::{BleChannel, LeFrameConfig, LePdu, decode_le_frames};
use crate::{Error, Result};
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LogicalLinkId {
    Reserved = 0,
    ContinuationOrEmpty = 1,
    StartOrComplete = 2,
    Control = 3,
}

impl LogicalLinkId {
    pub const fn from_header(header: u8) -> Self {
        match header & 0x03 {
            0 => Self::Reserved,
            1 => Self::ContinuationOrEmpty,
            2 => Self::StartOrComplete,
            3 => Self::Control,
            _ => unreachable!(),
        }
    }
}

impl Display for LogicalLinkId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reserved => formatter.write_str("reserved"),
            Self::ContinuationOrEmpty => formatter.write_str("continuation-or-empty"),
            Self::StartOrComplete => formatter.write_str("start-or-complete"),
            Self::Control => formatter.write_str("control"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataChannelPdu {
    pub channel: BleChannel,
    pub access_address: u32,
    pub bit_offset: usize,
    pub inverted: bool,
    pub access_address_errors: u8,
    pub header: [u8; 2],
    pub cte_info: Option<u8>,
    /// Payload and optional MIC, as counted by the data header Length octet.
    pub payload: Vec<u8>,
    pub crc: [u8; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstantToneExtensionInfo(u8);

impl ConstantToneExtensionInfo {
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn time_units(self) -> u8 {
        self.0 & 0x1f
    }

    pub const fn duration_us(self) -> u16 {
        self.time_units() as u16 * 8
    }

    pub const fn rfu(self) -> bool {
        self.0 & 0x20 != 0
    }

    pub const fn cte_type(self) -> u8 {
        self.0 >> 6
    }

    pub const fn cte_type_name(self) -> &'static str {
        match self.cte_type() {
            0 => "AoA",
            1 => "AoD-1us",
            2 => "AoD-2us",
            _ => "reserved",
        }
    }

    pub const fn has_reserved_value(self) -> bool {
        self.rfu() || self.cte_type() == 3 || self.time_units() < 2 || self.time_units() > 20
    }
}

impl DataChannelPdu {
    pub const fn llid(&self) -> LogicalLinkId {
        LogicalLinkId::from_header(self.header[0])
    }

    pub const fn next_expected_sequence_number(&self) -> bool {
        self.header[0] & 0x04 != 0
    }

    pub const fn sequence_number(&self) -> bool {
        self.header[0] & 0x08 != 0
    }

    pub const fn more_data(&self) -> bool {
        self.header[0] & 0x10 != 0
    }

    pub const fn constant_tone_extension_present(&self) -> bool {
        self.header[0] & 0x20 != 0
    }

    pub const fn reserved_header_bits(&self) -> u8 {
        self.header[0] >> 6
    }

    pub const fn declared_payload_length(&self) -> u8 {
        self.header[1]
    }

    pub const fn constant_tone_extension_info(&self) -> Option<ConstantToneExtensionInfo> {
        match self.cte_info {
            Some(raw) => Some(ConstantToneExtensionInfo::from_raw(raw)),
            None => None,
        }
    }

    pub fn link_layer_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            4 + 2 + usize::from(self.cte_info.is_some()) + self.payload.len() + 3,
        );
        bytes.extend_from_slice(&self.access_address.to_le_bytes());
        bytes.extend_from_slice(&self.header);
        if let Some(cte_info) = self.cte_info {
            bytes.push(cte_info);
        }
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&self.crc);
        bytes
    }

    pub fn l2cap_start(&self) -> Result<Option<L2capStart<'_>>> {
        if self.llid() != LogicalLinkId::StartOrComplete {
            return Ok(None);
        }
        if self.payload.len() < 4 {
            return Err(Error::InvalidInput(format!(
                "LLID start-or-complete PDU requires a 4-octet L2CAP header, received {} octets",
                self.payload.len()
            )));
        }
        Ok(Some(L2capStart {
            payload_length: u16::from_le_bytes([self.payload[0], self.payload[1]]),
            channel_id: u16::from_le_bytes([self.payload[2], self.payload[3]]),
            fragment: &self.payload[4..],
        }))
    }

    pub fn control(&self) -> Result<Option<ControlPdu<'_>>> {
        if self.llid() != LogicalLinkId::Control {
            return Ok(None);
        }
        let Some((&opcode, parameters)) = self.payload.split_first() else {
            return Err(Error::InvalidInput(
                "LL control PDU is missing its opcode".to_owned(),
            ));
        };
        Ok(Some(ControlPdu { opcode, parameters }))
    }
}

impl From<LePdu> for DataChannelPdu {
    fn from(packet: LePdu) -> Self {
        Self {
            channel: packet.channel,
            access_address: packet.access_address,
            bit_offset: packet.bit_offset,
            inverted: packet.inverted,
            access_address_errors: packet.access_address_errors,
            header: packet.header,
            cte_info: packet.cte_info,
            payload: packet.payload,
            crc: packet.crc,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct L2capStart<'a> {
    pub payload_length: u16,
    pub channel_id: u16,
    pub fragment: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlPdu<'a> {
    pub opcode: u8,
    pub parameters: &'a [u8],
}

impl ControlPdu<'_> {
    pub const fn opcode_name(self) -> &'static str {
        match self.opcode {
            0x00 => "LL_CONNECTION_UPDATE_IND",
            0x01 => "LL_CHANNEL_MAP_IND",
            0x02 => "LL_TERMINATE_IND",
            0x03 => "LL_ENC_REQ",
            0x04 => "LL_ENC_RSP",
            0x05 => "LL_START_ENC_REQ",
            0x06 => "LL_START_ENC_RSP",
            0x07 => "LL_UNKNOWN_RSP",
            0x08 => "LL_FEATURE_REQ",
            0x09 => "LL_FEATURE_RSP",
            0x0a => "LL_PAUSE_ENC_REQ",
            0x0b => "LL_PAUSE_ENC_RSP",
            0x0c => "LL_VERSION_IND",
            0x0d => "LL_REJECT_IND",
            0x0e => "LL_PERIPHERAL_FEATURE_REQ",
            0x0f => "LL_CONNECTION_PARAM_REQ",
            0x10 => "LL_CONNECTION_PARAM_RSP",
            0x11 => "LL_REJECT_EXT_IND",
            0x12 => "LL_PING_REQ",
            0x13 => "LL_PING_RSP",
            0x14 => "LL_LENGTH_REQ",
            0x15 => "LL_LENGTH_RSP",
            0x16 => "LL_PHY_REQ",
            0x17 => "LL_PHY_RSP",
            0x18 => "LL_PHY_UPDATE_IND",
            0x19 => "LL_MIN_USED_CHANNELS_IND",
            0x1a => "LL_CTE_REQ",
            0x1b => "LL_CTE_RSP",
            0x1c => "LL_PERIODIC_SYNC_IND",
            0x1d => "LL_CLOCK_ACCURACY_REQ",
            0x1e => "LL_CLOCK_ACCURACY_RSP",
            0x1f => "LL_CIS_REQ",
            0x20 => "LL_CIS_RSP",
            0x21 => "LL_CIS_IND",
            0x22 => "LL_CIS_TERMINATE_IND",
            0x23 => "LL_POWER_CONTROL_REQ",
            0x24 => "LL_POWER_CONTROL_RSP",
            0x25 => "LL_POWER_CHANGE_IND",
            0x26 => "LL_SUBRATE_REQ",
            0x27 => "LL_SUBRATE_IND",
            0x28 => "LL_CHANNEL_REPORTING_IND",
            0x29 => "LL_CHANNEL_STATUS_IND",
            0x2a => "LL_PERIODIC_SYNC_WR_IND",
            0x2b => "LL_FEATURE_EXT_REQ",
            0x2c => "LL_FEATURE_EXT_RSP",
            _ => "LL_UNKNOWN_OPCODE",
        }
    }
}

pub fn decode_data_channel(
    bits: &[bool],
    channel: BleChannel,
    access_address: u32,
    crc_init: u32,
    max_access_address_errors: u8,
) -> Result<Vec<DataChannelPdu>> {
    let config = LeFrameConfig::data(access_address, crc_init)?;
    Ok(
        decode_le_frames(bits, channel, config, max_access_address_errors)?
            .into_iter()
            .map(DataChannelPdu::from)
            .collect(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataChannelMap {
    bytes: [u8; 5],
    used_channels: [u8; 37],
    used_count: u8,
}

impl DataChannelMap {
    pub fn new(bytes: [u8; 5]) -> Result<Self> {
        if bytes[4] & 0xe0 != 0 {
            return Err(Error::InvalidInput(
                "LE data channel map sets reserved bits 37..39".to_owned(),
            ));
        }

        let mut used_channels = [0u8; 37];
        let mut used_count = 0usize;
        for channel in 0u8..=36 {
            if bytes[channel as usize / 8] & (1 << (channel % 8)) != 0 {
                used_channels[used_count] = channel;
                used_count += 1;
            }
        }
        if used_count < 2 {
            return Err(Error::InvalidInput(
                "LE data channel map enables fewer than two channels".to_owned(),
            ));
        }

        Ok(Self {
            bytes,
            used_channels,
            used_count: used_count as u8,
        })
    }

    pub const fn bytes(&self) -> [u8; 5] {
        self.bytes
    }

    pub const fn used_count(&self) -> u8 {
        self.used_count
    }

    pub fn contains(&self, channel: u8) -> bool {
        channel <= 36 && self.bytes[channel as usize / 8] & (1 << (channel % 8)) != 0
    }

    pub fn used_channels(&self) -> &[u8] {
        &self.used_channels[..self.used_count as usize]
    }

    fn remap(&self, index: u8) -> BleChannel {
        let channel = self.used_channels[index as usize];
        BleChannel::new(channel).expect("validated data channel map contains only channels 0..=36")
    }
}

impl TryFrom<[u8; 5]> for DataChannelMap {
    type Error = Error;

    fn try_from(bytes: [u8; 5]) -> Result<Self> {
        Self::new(bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelSelectionAlgorithm {
    Csa1,
    Csa2,
}

#[derive(Clone, Debug)]
pub enum ConnectionChannelSelector {
    Csa1(ChannelSelectionAlgorithm1),
    Csa2(ChannelSelectionAlgorithm2),
}

impl ConnectionChannelSelector {
    pub fn new(
        algorithm: ChannelSelectionAlgorithm,
        channel_map: DataChannelMap,
        access_address: u32,
        hop_increment: u8,
    ) -> Result<Self> {
        match algorithm {
            ChannelSelectionAlgorithm::Csa1 => Ok(Self::Csa1(ChannelSelectionAlgorithm1::new(
                channel_map,
                hop_increment,
            )?)),
            ChannelSelectionAlgorithm::Csa2 => Ok(Self::Csa2(ChannelSelectionAlgorithm2::new(
                channel_map,
                access_address,
            ))),
        }
    }

    pub fn channel_for_event(&self, event_counter: u16) -> BleChannel {
        match self {
            Self::Csa1(selector) => selector.channel_for_event(event_counter),
            Self::Csa2(selector) => selector.channel_for_event(event_counter),
        }
    }
}

impl Display for ChannelSelectionAlgorithm {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Csa1 => formatter.write_str("CSA#1"),
            Self::Csa2 => formatter.write_str("CSA#2"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChannelSelectionAlgorithm1 {
    channel_map: DataChannelMap,
    hop_increment: u8,
}

impl ChannelSelectionAlgorithm1 {
    pub fn new(channel_map: DataChannelMap, hop_increment: u8) -> Result<Self> {
        if !(5..=16).contains(&hop_increment) {
            return Err(Error::InvalidConfiguration(format!(
                "CSA#1 hop increment {hop_increment} is outside 5..=16"
            )));
        }
        Ok(Self {
            channel_map,
            hop_increment,
        })
    }

    pub fn channel_for_event(&self, event_counter: u16) -> BleChannel {
        let unmapped = (((event_counter as u32 + 1) * self.hop_increment as u32) % 37) as u8;
        if self.channel_map.contains(unmapped) {
            BleChannel::new(unmapped).expect("CSA#1 produces a data channel")
        } else {
            self.channel_map
                .remap(unmapped % self.channel_map.used_count())
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChannelSelectionAlgorithm2 {
    channel_map: DataChannelMap,
    channel_identifier: u16,
}

impl ChannelSelectionAlgorithm2 {
    pub fn new(channel_map: DataChannelMap, access_address: u32) -> Self {
        Self {
            channel_map,
            channel_identifier: channel_identifier(access_address),
        }
    }

    pub const fn channel_identifier(&self) -> u16 {
        self.channel_identifier
    }

    pub fn channel_for_event(&self, event_counter: u16) -> BleChannel {
        let pseudo_random_number = csa2_pseudo_random(event_counter, self.channel_identifier);
        let unmapped = (pseudo_random_number % 37) as u8;
        if self.channel_map.contains(unmapped) {
            BleChannel::new(unmapped).expect("CSA#2 produces a data channel")
        } else {
            let remapping_index =
                ((self.channel_map.used_count() as u32 * pseudo_random_number as u32) >> 16) as u8;
            self.channel_map.remap(remapping_index)
        }
    }
}

pub const fn channel_identifier(access_address: u32) -> u16 {
    (access_address as u16) ^ (access_address >> 16) as u16
}

const fn csa2_multiply_add_modulo(value: u16, channel_identifier: u16) -> u16 {
    value.wrapping_mul(17).wrapping_add(channel_identifier)
}

const fn csa2_permute(mut value: u16) -> u16 {
    value = ((value & 0xaaaa) >> 1) | ((value & 0x5555) << 1);
    value = ((value & 0xcccc) >> 2) | ((value & 0x3333) << 2);
    ((value & 0xf0f0) >> 4) | ((value & 0x0f0f) << 4)
}

pub const fn csa2_pseudo_random(event_counter: u16, channel_identifier: u16) -> u16 {
    let mut value = event_counter ^ channel_identifier;
    value = csa2_multiply_add_modulo(csa2_permute(value), channel_identifier);
    value = csa2_multiply_add_modulo(csa2_permute(value), channel_identifier);
    value = csa2_multiply_add_modulo(csa2_permute(value), channel_identifier);
    value ^ channel_identifier
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::{bytes_to_bits_lsb, crc24_bytes, whiten_bits};

    fn all_channels() -> DataChannelMap {
        DataChannelMap::new([0xff, 0xff, 0xff, 0xff, 0x1f]).unwrap()
    }

    #[test]
    fn parses_data_header_l2cap_and_control_fields() {
        let l2cap = DataChannelPdu {
            channel: BleChannel::new(7).unwrap(),
            access_address: 0x1234_5678,
            bit_offset: 9,
            inverted: false,
            access_address_errors: 0,
            header: [0x3e, 7],
            cte_info: Some(0x85),
            payload: vec![3, 0, 4, 0, 0xaa, 0xbb, 0xcc],
            crc: [1, 2, 3],
        };
        assert_eq!(l2cap.llid(), LogicalLinkId::StartOrComplete);
        assert!(l2cap.next_expected_sequence_number());
        assert!(l2cap.sequence_number());
        assert!(l2cap.more_data());
        assert!(l2cap.constant_tone_extension_present());
        let cte_info = l2cap.constant_tone_extension_info().unwrap();
        assert_eq!(cte_info.raw(), 0x85);
        assert_eq!(cte_info.time_units(), 5);
        assert_eq!(cte_info.duration_us(), 40);
        assert_eq!(cte_info.cte_type_name(), "AoD-2us");
        assert!(!cte_info.rfu());
        assert!(!cte_info.has_reserved_value());
        let start = l2cap.l2cap_start().unwrap().unwrap();
        assert_eq!(start.payload_length, 3);
        assert_eq!(start.channel_id, 4);
        assert_eq!(start.fragment, [0xaa, 0xbb, 0xcc]);

        let mut control = l2cap.clone();
        control.header[0] = 0x03;
        control.payload = vec![0x14, 0xfb, 0x00];
        let decoded = control.control().unwrap().unwrap();
        assert_eq!(decoded.opcode_name(), "LL_LENGTH_REQ");
        assert_eq!(decoded.parameters, [0xfb, 0x00]);
    }

    #[test]
    fn cte_info_reports_reserved_values_without_losing_raw_octet() {
        let info = ConstantToneExtensionInfo::from_raw(0xe1);
        assert_eq!(info.raw(), 0xe1);
        assert_eq!(info.time_units(), 1);
        assert_eq!(info.duration_us(), 8);
        assert!(info.rfu());
        assert_eq!(info.cte_type_name(), "reserved");
        assert!(info.has_reserved_value());
    }

    #[test]
    fn data_decoder_preserves_full_payload_and_header() {
        let channel = BleChannel::new(21).unwrap();
        let access_address = 0x1234_5678u32;
        let crc_init = 0x00ab_cdef;
        let payload = [5, 0, 4, 0, 1, 2, 3, 4, 5];
        let mut pdu = vec![0x1e, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, crc_init));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&access_address.to_le_bytes());
        bits.extend(body);

        let packets = decode_data_channel(&bits, channel, access_address, crc_init, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].header, [0x1e, 9]);
        assert_eq!(packets[0].cte_info, None);
        assert_eq!(packets[0].payload, payload);
        assert_eq!(packets[0].llid(), LogicalLinkId::StartOrComplete);
    }

    #[test]
    fn channel_map_validates_and_orders_used_channels() {
        let map = DataChannelMap::new([0b0010_1010, 0, 0, 0, 0x10]).unwrap();
        assert_eq!(map.used_channels(), [1, 3, 5, 36]);
        assert!(map.contains(36));
        assert!(!map.contains(37));
        assert!(DataChannelMap::new([1, 0, 0, 0, 0]).is_err());
        assert!(DataChannelMap::new([3, 0, 0, 0, 0x20]).is_err());
    }

    #[test]
    fn csa1_all_channel_sequence_starts_with_hop_increment() {
        let selector = ChannelSelectionAlgorithm1::new(all_channels(), 5).unwrap();
        let channels: Vec<u8> = (0..8)
            .map(|event| selector.channel_for_event(event).index())
            .collect();
        assert_eq!(channels, [5, 10, 15, 20, 25, 30, 35, 3]);
    }

    #[test]
    fn csa1_remaps_by_unmapped_channel_modulo_used_count() {
        let map = DataChannelMap::new([0b0010_1010, 0, 0, 0, 0x10]).unwrap();
        let selector = ChannelSelectionAlgorithm1::new(map, 5).unwrap();
        assert_eq!(selector.channel_for_event(0).index(), 5);
        assert_eq!(selector.channel_for_event(1).index(), 5);
        assert_eq!(selector.channel_for_event(6).index(), 36);
    }

    #[test]
    fn csa2_matches_core_sequence_for_advertising_access_address() {
        let selector = ChannelSelectionAlgorithm2::new(all_channels(), 0x8e89_bed6);
        assert_eq!(selector.channel_identifier(), 0x305f);
        let channels: Vec<u8> = (0..8)
            .map(|event| selector.channel_for_event(event).index())
            .collect();
        assert_eq!(channels, [25, 20, 6, 21, 34, 36, 23, 14]);
    }

    #[test]
    fn csa2_matches_core_remapping_vectors() {
        let nine_channel_map = DataChannelMap::new([0x00, 0x06, 0xe0, 0x00, 0x1e]).unwrap();
        let selector = ChannelSelectionAlgorithm2::new(nine_channel_map, 0x8e89_bed6);
        assert_eq!(selector.channel_for_event(6).index(), 23);
        assert_eq!(selector.channel_for_event(7).index(), 9);
        assert_eq!(selector.channel_for_event(8).index(), 34);

        let two_channel_map = DataChannelMap::new([0x06, 0, 0, 0, 0]).unwrap();
        let selector = ChannelSelectionAlgorithm2::new(two_channel_map, 0x8e89_bed6);
        assert_eq!(selector.channel_for_event(11).index(), 1);
        assert_eq!(selector.channel_for_event(12).index(), 2);
        assert_eq!(selector.channel_for_event(13).index(), 1);
    }

    #[test]
    fn channel_selection_wraps_event_counter_without_panicking() {
        let csa1 = ChannelSelectionAlgorithm1::new(all_channels(), 16).unwrap();
        let csa2 = ChannelSelectionAlgorithm2::new(all_channels(), 0xffff_ffff);
        assert!(csa1.channel_for_event(u16::MAX).index() <= 36);
        assert!(csa2.channel_for_event(u16::MAX).index() <= 36);
    }
}
