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
pub enum LinkDirection {
    CentralToPeripheral,
    PeripheralToCentral,
}

impl Display for LinkDirection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CentralToPeripheral => formatter.write_str("central-to-peripheral"),
            Self::PeripheralToCentral => formatter.write_str("peripheral-to-central"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L2capPdu {
    pub direction: LinkDirection,
    pub channel_id: u16,
    pub payload: Vec<u8>,
    pub fragment_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncompleteL2capPdu {
    pub direction: LinkDirection,
    pub channel_id: u16,
    pub expected_payload_length: usize,
    pub received_payload_length: usize,
    pub fragment_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum L2capReassemblyOutcome {
    Ignored,
    Duplicate,
    OrphanedContinuation { fragment_octets: usize },
    InProgress(IncompleteL2capPdu),
    Complete(L2capPdu),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L2capReassemblyUpdate {
    pub replaced: Option<IncompleteL2capPdu>,
    pub outcome: L2capReassemblyOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct L2capFragmentFingerprint {
    llid: LogicalLinkId,
    sequence_number: bool,
    cte_info: Option<u8>,
    payload: Vec<u8>,
}

impl L2capFragmentFingerprint {
    fn from_packet(packet: &DataChannelPdu) -> Self {
        Self {
            llid: packet.llid(),
            sequence_number: packet.sequence_number(),
            cte_info: packet.cte_info,
            payload: packet.payload.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingL2capPdu {
    channel_id: u16,
    expected_payload_length: usize,
    payload: Vec<u8>,
    fragment_count: u32,
}

impl PendingL2capPdu {
    fn incomplete(&self, direction: LinkDirection) -> IncompleteL2capPdu {
        IncompleteL2capPdu {
            direction,
            channel_id: self.channel_id,
            expected_payload_length: self.expected_payload_length,
            received_payload_length: self.payload.len(),
            fragment_count: self.fragment_count,
        }
    }

    fn complete(self, direction: LinkDirection) -> L2capPdu {
        L2capPdu {
            direction,
            channel_id: self.channel_id,
            payload: self.payload,
            fragment_count: self.fragment_count,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DirectionalL2capState {
    pending: Option<PendingL2capPdu>,
    last_fragment: Option<L2capFragmentFingerprint>,
}

/// Reassembles plaintext LE-U L2CAP PDUs from direction-tagged link-layer PDUs.
///
/// Packet direction and encryption state cannot be inferred from an isolated
/// data-channel PDU. Callers must supply the actual transmitter direction and
/// must only feed plaintext or already-decrypted packets. A capture gap must be
/// reported through [`Self::reset`] or [`Self::reset_all`] before more packets
/// are supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L2capReassembler {
    maximum_payload_length: usize,
    central_to_peripheral: DirectionalL2capState,
    peripheral_to_central: DirectionalL2capState,
}

impl Default for L2capReassembler {
    fn default() -> Self {
        Self {
            maximum_payload_length: usize::from(u16::MAX),
            central_to_peripheral: DirectionalL2capState::default(),
            peripheral_to_central: DirectionalL2capState::default(),
        }
    }
}

impl L2capReassembler {
    pub fn new(maximum_payload_length: usize) -> Result<Self> {
        if maximum_payload_length > usize::from(u16::MAX) {
            return Err(Error::InvalidConfiguration(format!(
                "maximum L2CAP payload length {maximum_payload_length} exceeds 65535"
            )));
        }
        Ok(Self {
            maximum_payload_length,
            ..Self::default()
        })
    }

    pub const fn maximum_payload_length(&self) -> usize {
        self.maximum_payload_length
    }

    pub fn pending(&self, direction: LinkDirection) -> Option<IncompleteL2capPdu> {
        self.state(direction)
            .pending
            .as_ref()
            .map(|pending| pending.incomplete(direction))
    }

    pub fn reset(&mut self, direction: LinkDirection) -> Option<IncompleteL2capPdu> {
        let state = self.state_mut(direction);
        state.last_fragment = None;
        state
            .pending
            .take()
            .map(|pending| pending.incomplete(direction))
    }

    pub fn reset_all(&mut self) -> Vec<IncompleteL2capPdu> {
        [
            LinkDirection::CentralToPeripheral,
            LinkDirection::PeripheralToCentral,
        ]
        .into_iter()
        .filter_map(|direction| self.reset(direction))
        .collect()
    }

    /// Adds one ordered link-layer PDU.
    ///
    /// A malformed data-header invariant or L2CAP fragment length discards the
    /// pending PDU for that direction before returning an error.
    pub fn push(
        &mut self,
        direction: LinkDirection,
        packet: &DataChannelPdu,
    ) -> Result<L2capReassemblyUpdate> {
        if usize::from(packet.declared_payload_length()) != packet.payload.len() {
            self.reset(direction);
            return Err(Error::InvalidInput(format!(
                "data PDU declares {} payload octets but carries {}",
                packet.declared_payload_length(),
                packet.payload.len()
            )));
        }
        if packet.constant_tone_extension_present() != packet.cte_info.is_some() {
            self.reset(direction);
            return Err(Error::InvalidInput(
                "data PDU CP bit and CTEInfo presence disagree".to_owned(),
            ));
        }
        match packet.llid() {
            LogicalLinkId::Control | LogicalLinkId::Reserved => {
                self.state_mut(direction).last_fragment = None;
                return Ok(L2capReassemblyUpdate {
                    replaced: None,
                    outcome: L2capReassemblyOutcome::Ignored,
                });
            }
            LogicalLinkId::ContinuationOrEmpty if packet.payload.is_empty() => {
                self.state_mut(direction).last_fragment = None;
                return Ok(L2capReassemblyUpdate {
                    replaced: None,
                    outcome: L2capReassemblyOutcome::Ignored,
                });
            }
            LogicalLinkId::ContinuationOrEmpty | LogicalLinkId::StartOrComplete => {}
        }

        let fingerprint = L2capFragmentFingerprint::from_packet(packet);
        let state = self.state_mut(direction);
        if state.last_fragment.as_ref() == Some(&fingerprint) {
            return Ok(L2capReassemblyUpdate {
                replaced: None,
                outcome: L2capReassemblyOutcome::Duplicate,
            });
        }
        state.last_fragment = Some(fingerprint);

        match packet.llid() {
            LogicalLinkId::StartOrComplete => self.push_start(direction, packet),
            LogicalLinkId::ContinuationOrEmpty => self.push_continuation(direction, packet),
            LogicalLinkId::Control | LogicalLinkId::Reserved => unreachable!(),
        }
    }

    fn push_start(
        &mut self,
        direction: LinkDirection,
        packet: &DataChannelPdu,
    ) -> Result<L2capReassemblyUpdate> {
        let start = match packet.l2cap_start() {
            Ok(Some(start)) => start,
            Ok(None) => unreachable!(),
            Err(error) => {
                self.state_mut(direction).pending = None;
                return Err(error);
            }
        };
        let expected_payload_length = usize::from(start.payload_length);
        if expected_payload_length > self.maximum_payload_length {
            self.state_mut(direction).pending = None;
            return Err(Error::InvalidInput(format!(
                "L2CAP payload length {expected_payload_length} exceeds configured maximum {}",
                self.maximum_payload_length
            )));
        }
        if start.fragment.len() > expected_payload_length {
            self.state_mut(direction).pending = None;
            return Err(Error::InvalidInput(format!(
                "L2CAP start contains {} payload octets, exceeding declared length {expected_payload_length}",
                start.fragment.len()
            )));
        }

        let state = self.state_mut(direction);
        let replaced = state
            .pending
            .take()
            .map(|pending| pending.incomplete(direction));
        let pending = PendingL2capPdu {
            channel_id: start.channel_id,
            expected_payload_length,
            payload: start.fragment.to_vec(),
            fragment_count: 1,
        };
        let outcome = if pending.payload.len() == pending.expected_payload_length {
            L2capReassemblyOutcome::Complete(pending.complete(direction))
        } else {
            let progress = pending.incomplete(direction);
            state.pending = Some(pending);
            L2capReassemblyOutcome::InProgress(progress)
        };
        Ok(L2capReassemblyUpdate { replaced, outcome })
    }

    fn push_continuation(
        &mut self,
        direction: LinkDirection,
        packet: &DataChannelPdu,
    ) -> Result<L2capReassemblyUpdate> {
        let state = self.state_mut(direction);
        let Some(mut pending) = state.pending.take() else {
            return Ok(L2capReassemblyUpdate {
                replaced: None,
                outcome: L2capReassemblyOutcome::OrphanedContinuation {
                    fragment_octets: packet.payload.len(),
                },
            });
        };
        let remaining = pending.expected_payload_length - pending.payload.len();
        if packet.payload.len() > remaining {
            return Err(Error::InvalidInput(format!(
                "L2CAP continuation contains {} octets with only {remaining} remaining",
                packet.payload.len()
            )));
        }
        pending.payload.extend_from_slice(&packet.payload);
        pending.fragment_count = pending
            .fragment_count
            .checked_add(1)
            .ok_or_else(|| Error::InvalidState("L2CAP fragment count overflow".to_owned()))?;
        let outcome = if pending.payload.len() == pending.expected_payload_length {
            L2capReassemblyOutcome::Complete(pending.complete(direction))
        } else {
            let progress = pending.incomplete(direction);
            state.pending = Some(pending);
            L2capReassemblyOutcome::InProgress(progress)
        };
        Ok(L2capReassemblyUpdate {
            replaced: None,
            outcome,
        })
    }

    fn state(&self, direction: LinkDirection) -> &DirectionalL2capState {
        match direction {
            LinkDirection::CentralToPeripheral => &self.central_to_peripheral,
            LinkDirection::PeripheralToCentral => &self.peripheral_to_central,
        }
    }

    fn state_mut(&mut self, direction: LinkDirection) -> &mut DirectionalL2capState {
        match direction {
            LinkDirection::CentralToPeripheral => &mut self.central_to_peripheral,
            LinkDirection::PeripheralToCentral => &mut self.peripheral_to_central,
        }
    }
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

    pub fn connection_update_ind(self) -> Result<Option<ConnectionUpdateInd>> {
        if self.opcode != 0x00 {
            return Ok(None);
        }
        if self.parameters.len() != 11 {
            return Err(Error::InvalidInput(format!(
                "LL_CONNECTION_UPDATE_IND requires 11 parameter octets, received {}",
                self.parameters.len()
            )));
        }
        Ok(Some(ConnectionUpdateInd::new(
            self.parameters[0],
            u16::from_le_bytes([self.parameters[1], self.parameters[2]]),
            u16::from_le_bytes([self.parameters[3], self.parameters[4]]),
            u16::from_le_bytes([self.parameters[5], self.parameters[6]]),
            u16::from_le_bytes([self.parameters[7], self.parameters[8]]),
            u16::from_le_bytes([self.parameters[9], self.parameters[10]]),
        )?))
    }

    pub fn channel_map_ind(self) -> Result<Option<ChannelMapInd>> {
        if self.opcode != 0x01 {
            return Ok(None);
        }
        if self.parameters.len() != 7 {
            return Err(Error::InvalidInput(format!(
                "LL_CHANNEL_MAP_IND requires 7 parameter octets, received {}",
                self.parameters.len()
            )));
        }
        Ok(Some(ChannelMapInd {
            channel_map: DataChannelMap::new(self.parameters[..5].try_into().map_err(|_| {
                Error::InvalidInput("LL_CHANNEL_MAP_IND channel map is truncated".to_owned())
            })?)?,
            instant: u16::from_le_bytes([self.parameters[5], self.parameters[6]]),
        }))
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
pub struct ConnectionParameters {
    pub interval: u16,
    pub latency: u16,
    pub supervision_timeout: u16,
}

impl ConnectionParameters {
    pub fn new(interval: u16, latency: u16, supervision_timeout: u16) -> Result<Self> {
        let parameters = Self {
            interval,
            latency,
            supervision_timeout,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    pub fn validate(self) -> Result<()> {
        if !(6..=3_200).contains(&self.interval) {
            return Err(Error::InvalidInput(format!(
                "connection interval {} is outside 6..=3200",
                self.interval
            )));
        }
        if self.latency > 499 {
            return Err(Error::InvalidInput(format!(
                "connection latency {} exceeds 499",
                self.latency
            )));
        }
        if !(10..=3_200).contains(&self.supervision_timeout) {
            return Err(Error::InvalidInput(format!(
                "connection supervision timeout {} is outside 10..=3200",
                self.supervision_timeout
            )));
        }
        let minimum_timeout_us =
            2u64 * (u64::from(self.latency) + 1) * u64::from(self.interval_us());
        if u64::from(self.supervision_timeout_us()) <= minimum_timeout_us {
            return Err(Error::InvalidInput(format!(
                "connection supervision timeout {} us must exceed {} us for interval and latency",
                self.supervision_timeout_us(),
                minimum_timeout_us
            )));
        }
        Ok(())
    }

    pub const fn interval_us(self) -> u32 {
        self.interval as u32 * 1_250
    }

    pub const fn supervision_timeout_us(self) -> u32 {
        self.supervision_timeout as u32 * 10_000
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionUpdateInd {
    pub window_size: u8,
    pub window_offset: u16,
    pub parameters: ConnectionParameters,
    pub instant: u16,
}

impl ConnectionUpdateInd {
    pub fn new(
        window_size: u8,
        window_offset: u16,
        interval: u16,
        latency: u16,
        supervision_timeout: u16,
        instant: u16,
    ) -> Result<Self> {
        let update = Self {
            window_size,
            window_offset,
            parameters: ConnectionParameters::new(interval, latency, supervision_timeout)?,
            instant,
        };
        update.validate()?;
        Ok(update)
    }

    pub fn validate(self) -> Result<()> {
        if !(1..=16).contains(&self.window_size) {
            return Err(Error::InvalidInput(format!(
                "connection-update window size {} is outside 1..=16",
                self.window_size
            )));
        }
        self.parameters.validate()?;
        if self.window_offset > self.parameters.interval {
            return Err(Error::InvalidInput(format!(
                "connection-update window offset {} exceeds interval {}",
                self.window_offset, self.parameters.interval
            )));
        }
        Ok(())
    }

    pub const fn window_offset_us(self) -> u32 {
        self.window_offset as u32 * 1_250
    }

    pub const fn window_size_us(self) -> u32 {
        self.window_size as u32 * 1_250
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelMapInd {
    pub channel_map: DataChannelMap,
    pub instant: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstantRelation {
    Future(u16),
    Reached,
    Passed(u16),
    Ambiguous,
}

pub const fn instant_relation(current_event_counter: u16, instant: u16) -> InstantRelation {
    let future = instant.wrapping_sub(current_event_counter);
    if future == 0 {
        InstantRelation::Reached
    } else if future < 0x7fff {
        InstantRelation::Future(future)
    } else if future <= 0x8000 {
        InstantRelation::Ambiguous
    } else {
        InstantRelation::Passed(current_event_counter.wrapping_sub(instant))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepClockAccuracy(u8);

impl SleepClockAccuracy {
    pub fn new(raw: u8) -> Result<Self> {
        if raw > 7 {
            return Err(Error::InvalidInput(format!(
                "sleep clock accuracy {raw} is outside 0..=7"
            )));
        }
        Ok(Self(raw))
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn maximum_ppm(self) -> u16 {
        [500, 250, 150, 100, 75, 50, 30, 20][self.0 as usize]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionTrackerConfig {
    pub access_address: u32,
    pub channel_selection_algorithm: ChannelSelectionAlgorithm,
    pub hop_increment: u8,
    pub channel_map: DataChannelMap,
    pub parameters: ConnectionParameters,
    pub sample_rate_hz: u32,
}

impl ConnectionTrackerConfig {
    pub fn validate(&self) -> Result<()> {
        self.parameters.validate()?;
        if self.sample_rate_hz == 0 {
            return Err(Error::InvalidConfiguration(
                "connection tracker sample rate must be greater than zero".to_owned(),
            ));
        }
        ConnectionChannelSelector::new(
            self.channel_selection_algorithm,
            self.channel_map.clone(),
            self.access_address,
            self.hop_increment,
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionEventTiming {
    Expected { access_address_sample: u64 },
    AnchorObservationRequired { window_offset: u16, window_size: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionEvent {
    pub event_counter: u16,
    pub channel: BleChannel,
    pub timing: ConnectionEventTiming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionTimingWindow {
    pub expected_sample: u64,
    pub earliest_sample: u64,
    pub latest_sample: u64,
    pub widening_samples: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleTimingError {
    Early(u64),
    OnTime,
    Late(u64),
}

impl SampleTimingError {
    pub const fn absolute_samples(self) -> u64 {
        match self {
            Self::Early(samples) | Self::Late(samples) => samples,
            Self::OnTime => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionObservation {
    pub event: ConnectionEvent,
    pub timing_window: ConnectionTimingWindow,
    pub advanced_events: u16,
    pub timing_error: SampleTimingError,
}

#[derive(Clone, Debug)]
enum PendingConnectionUpdate {
    ChannelMap {
        target_event: u64,
        update: ChannelMapInd,
    },
    Parameters {
        target_event: u64,
        update: ConnectionUpdateInd,
    },
}

#[derive(Clone, Copy, Debug)]
enum TrackerTiming {
    Anchored {
        event_index: u64,
        access_address_sample: u64,
    },
    AwaitingAnchor {
        update: ConnectionUpdateInd,
    },
}

#[derive(Clone, Debug)]
pub struct ConnectionTracker {
    config: ConnectionTrackerConfig,
    selector: ConnectionChannelSelector,
    event_counter: u16,
    event_index: u64,
    timing: TrackerTiming,
    pending: Option<PendingConnectionUpdate>,
}

impl ConnectionTracker {
    pub fn new(
        config: ConnectionTrackerConfig,
        observed_event_counter: u16,
        observed_access_address_sample: u64,
    ) -> Result<Self> {
        config.validate()?;
        let selector = ConnectionChannelSelector::new(
            config.channel_selection_algorithm,
            config.channel_map.clone(),
            config.access_address,
            config.hop_increment,
        )?;
        Ok(Self {
            config,
            selector,
            event_counter: observed_event_counter,
            event_index: 0,
            timing: TrackerTiming::Anchored {
                event_index: 0,
                access_address_sample: observed_access_address_sample,
            },
            pending: None,
        })
    }

    pub const fn event_counter(&self) -> u16 {
        self.event_counter
    }

    pub const fn parameters(&self) -> ConnectionParameters {
        self.config.parameters
    }

    pub fn channel_map(&self) -> &DataChannelMap {
        &self.config.channel_map
    }

    pub fn current_event(&self) -> Result<ConnectionEvent> {
        Ok(ConnectionEvent {
            event_counter: self.event_counter,
            channel: self.selector.channel_for_event(self.event_counter),
            timing: self.current_timing()?,
        })
    }

    pub fn current_timing_window(
        &self,
        peer_clock_accuracy: SleepClockAccuracy,
        receiver_clock_accuracy_ppm: u32,
    ) -> Result<Option<ConnectionTimingWindow>> {
        if receiver_clock_accuracy_ppm > 1_000_000 {
            return Err(Error::InvalidConfiguration(format!(
                "receiver clock accuracy {receiver_clock_accuracy_ppm} ppm exceeds 1000000"
            )));
        }
        let ConnectionEventTiming::Expected {
            access_address_sample: expected_sample,
        } = self.current_timing()?
        else {
            return Ok(None);
        };
        let TrackerTiming::Anchored { event_index, .. } = self.timing else {
            return Ok(None);
        };
        let elapsed_events = self.event_index.checked_sub(event_index).ok_or_else(|| {
            Error::InvalidState(
                "connection tracker event index precedes its timing anchor".to_owned(),
            )
        })?;
        let elapsed_us = u128::from(elapsed_events)
            .checked_mul(u128::from(self.config.parameters.interval_us()))
            .ok_or_else(|| {
                Error::InvalidState("connection timing elapsed-time overflow".to_owned())
            })?;
        let combined_ppm = u128::from(receiver_clock_accuracy_ppm)
            .checked_add(u128::from(peer_clock_accuracy.maximum_ppm()))
            .ok_or_else(|| Error::InvalidState("connection clock-accuracy overflow".to_owned()))?;
        let widening_numerator = elapsed_us
            .checked_mul(combined_ppm)
            .and_then(|value| value.checked_mul(u128::from(self.config.sample_rate_hz)))
            .ok_or_else(|| {
                Error::InvalidState("connection timing-window arithmetic overflow".to_owned())
            })?;
        let widening_samples = divide_round_up(widening_numerator, 1_000_000_000_000)?;
        let maximum_widening_us = u128::from(self.config.parameters.interval_us() / 2 - 150);
        let maximum_widening_samples = divide_round_up(
            maximum_widening_us
                .checked_mul(u128::from(self.config.sample_rate_hz))
                .ok_or_else(|| {
                    Error::InvalidState(
                        "connection maximum timing-window arithmetic overflow".to_owned(),
                    )
                })?,
            1_000_000,
        )?;
        let widening_samples = widening_samples.min(maximum_widening_samples);
        Ok(Some(ConnectionTimingWindow {
            expected_sample,
            earliest_sample: expected_sample.saturating_sub(widening_samples),
            latest_sample: expected_sample
                .checked_add(widening_samples)
                .ok_or_else(|| {
                    Error::InvalidState("connection timing window exceeds u64".to_owned())
                })?,
            widening_samples,
        }))
    }

    pub fn advance(&mut self) -> Result<ConnectionEvent> {
        if matches!(self.timing, TrackerTiming::AwaitingAnchor { .. }) {
            return Err(Error::InvalidState(
                "connection tracker requires an observed anchor before it can advance".to_owned(),
            ));
        }
        self.event_index = self.event_index.checked_add(1).ok_or_else(|| {
            Error::InvalidState("connection tracker event index overflow".to_owned())
        })?;
        self.event_counter = self.event_counter.wrapping_add(1);

        if self.pending_target_event() == Some(self.event_index) {
            match self.pending.take().expect("pending target was present") {
                PendingConnectionUpdate::ChannelMap { update, .. } => {
                    self.config.channel_map = update.channel_map;
                    self.selector = ConnectionChannelSelector::new(
                        self.config.channel_selection_algorithm,
                        self.config.channel_map.clone(),
                        self.config.access_address,
                        self.config.hop_increment,
                    )?;
                }
                PendingConnectionUpdate::Parameters { update, .. } => {
                    self.config.parameters = update.parameters;
                    self.timing = TrackerTiming::AwaitingAnchor { update };
                }
            }
        }

        self.current_event()
    }

    pub fn observe_anchor(&mut self, access_address_sample: u64) -> Result<ConnectionEvent> {
        if !matches!(self.timing, TrackerTiming::AwaitingAnchor { .. }) {
            return Err(Error::InvalidState(
                "connection tracker is not waiting for an anchor observation".to_owned(),
            ));
        }
        self.timing = TrackerTiming::Anchored {
            event_index: self.event_index,
            access_address_sample,
        };
        self.current_event()
    }

    pub fn synchronize_observation(
        &mut self,
        channel: BleChannel,
        access_address_sample: u64,
        peer_clock_accuracy: SleepClockAccuracy,
        receiver_clock_accuracy_ppm: u32,
        maximum_event_advance: u16,
    ) -> Result<ConnectionObservation> {
        if channel.index() > 36 {
            return Err(Error::InvalidInput(format!(
                "connection observation requires a data channel in 0..=36; got {}",
                channel.index()
            )));
        }

        let mut candidate = self.clone();
        let mut best: Option<(Self, ConnectionObservation)> = None;
        let mut tied = false;
        for advanced_events in 0..=maximum_event_advance {
            let event = candidate.current_event()?;
            let Some(timing_window) = candidate
                .current_timing_window(peer_clock_accuracy, receiver_clock_accuracy_ppm)?
            else {
                break;
            };
            if event.channel == channel
                && (timing_window.earliest_sample..=timing_window.latest_sample)
                    .contains(&access_address_sample)
            {
                let timing_error = if access_address_sample < timing_window.expected_sample {
                    SampleTimingError::Early(timing_window.expected_sample - access_address_sample)
                } else if access_address_sample > timing_window.expected_sample {
                    SampleTimingError::Late(access_address_sample - timing_window.expected_sample)
                } else {
                    SampleTimingError::OnTime
                };
                let observation = ConnectionObservation {
                    event,
                    timing_window,
                    advanced_events,
                    timing_error,
                };
                let replace = best.as_ref().is_none_or(|(_, existing)| {
                    timing_error.absolute_samples() < existing.timing_error.absolute_samples()
                });
                if replace {
                    let mut matched = candidate.clone();
                    matched.timing = TrackerTiming::Anchored {
                        event_index: matched.event_index,
                        access_address_sample,
                    };
                    best = Some((matched, observation));
                    tied = false;
                } else if best.as_ref().is_some_and(|(_, existing)| {
                    timing_error.absolute_samples() == existing.timing_error.absolute_samples()
                }) {
                    tied = true;
                }
            }
            if advanced_events != maximum_event_advance {
                candidate.advance()?;
            }
        }

        if tied {
            return Err(Error::InvalidInput(
                "connection observation matches multiple events equally".to_owned(),
            ));
        }
        let Some((matched, observation)) = best else {
            return Err(Error::InvalidInput(format!(
                "connection observation on channel {} at sample {} did not match the next {} event(s)",
                channel.index(),
                access_address_sample,
                u32::from(maximum_event_advance) + 1
            )));
        };
        *self = matched;
        Ok(observation)
    }

    pub fn schedule_channel_map(&mut self, update: ChannelMapInd) -> Result<()> {
        let target_event = self.validate_new_instant(update.instant)?;
        self.pending = Some(PendingConnectionUpdate::ChannelMap {
            target_event,
            update,
        });
        Ok(())
    }

    pub fn schedule_connection_update(&mut self, update: ConnectionUpdateInd) -> Result<()> {
        update.validate()?;
        let target_event = self.validate_new_instant(update.instant)?;
        self.pending = Some(PendingConnectionUpdate::Parameters {
            target_event,
            update,
        });
        Ok(())
    }

    pub fn schedule_control(&mut self, control: ControlPdu<'_>) -> Result<bool> {
        if let Some(update) = control.connection_update_ind()? {
            self.schedule_connection_update(update)?;
            return Ok(true);
        }
        if let Some(update) = control.channel_map_ind()? {
            self.schedule_channel_map(update)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn validate_new_instant(&self, instant: u16) -> Result<u64> {
        if matches!(self.timing, TrackerTiming::AwaitingAnchor { .. }) {
            return Err(Error::InvalidState(
                "connection tracker requires an observed anchor before scheduling another update"
                    .to_owned(),
            ));
        }
        if self.pending.is_some() {
            return Err(Error::InvalidState(
                "an instant-based connection update is already pending".to_owned(),
            ));
        }
        let InstantRelation::Future(events) = instant_relation(self.event_counter, instant) else {
            return Err(Error::InvalidInput(format!(
                "instant {instant} is not unambiguously in the future of event {}",
                self.event_counter
            )));
        };
        self.event_index
            .checked_add(u64::from(events))
            .ok_or_else(|| Error::InvalidState("connection update event index overflow".to_owned()))
    }

    fn pending_target_event(&self) -> Option<u64> {
        match &self.pending {
            Some(PendingConnectionUpdate::ChannelMap { target_event, .. })
            | Some(PendingConnectionUpdate::Parameters { target_event, .. }) => Some(*target_event),
            None => None,
        }
    }

    fn current_timing(&self) -> Result<ConnectionEventTiming> {
        match self.timing {
            TrackerTiming::Anchored {
                event_index,
                access_address_sample,
            } => {
                let elapsed_events =
                    self.event_index.checked_sub(event_index).ok_or_else(|| {
                        Error::InvalidState(
                            "connection tracker event index precedes its timing anchor".to_owned(),
                        )
                    })?;
                let numerator = u128::from(elapsed_events)
                    .checked_mul(u128::from(self.config.parameters.interval_us()))
                    .and_then(|value| value.checked_mul(u128::from(self.config.sample_rate_hz)))
                    .ok_or_else(|| {
                        Error::InvalidState(
                            "connection tracker sample-offset arithmetic overflow".to_owned(),
                        )
                    })?;
                let rounded_samples = numerator.checked_add(500_000).ok_or_else(|| {
                    Error::InvalidState("connection tracker sample rounding overflow".to_owned())
                })? / 1_000_000;
                let rounded_samples = u64::try_from(rounded_samples).map_err(|_| {
                    Error::InvalidState("connection tracker sample offset exceeds u64".to_owned())
                })?;
                Ok(ConnectionEventTiming::Expected {
                    access_address_sample: access_address_sample
                        .checked_add(rounded_samples)
                        .ok_or_else(|| {
                            Error::InvalidState(
                                "connection tracker expected sample exceeds u64".to_owned(),
                            )
                        })?,
                })
            }
            TrackerTiming::AwaitingAnchor { update } => {
                Ok(ConnectionEventTiming::AnchorObservationRequired {
                    window_offset: update.window_offset,
                    window_size: update.window_size,
                })
            }
        }
    }
}

fn divide_round_up(numerator: u128, denominator: u128) -> Result<u64> {
    let rounded = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| Error::InvalidState("integer ceiling arithmetic overflow".to_owned()))?
        / denominator;
    u64::try_from(rounded)
        .map_err(|_| Error::InvalidState("integer ceiling result exceeds u64".to_owned()))
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

    fn tracker_config(
        channel_selection_algorithm: ChannelSelectionAlgorithm,
    ) -> ConnectionTrackerConfig {
        ConnectionTrackerConfig {
            access_address: 0x1234_5678,
            channel_selection_algorithm,
            hop_increment: 5,
            channel_map: all_channels(),
            parameters: ConnectionParameters::new(24, 0, 100).unwrap(),
            sample_rate_hz: 4_000_000,
        }
    }

    fn data_packet(header: u8, payload: &[u8]) -> DataChannelPdu {
        DataChannelPdu {
            channel: BleChannel::new(7).unwrap(),
            access_address: 0x1234_5678,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [header, payload.len() as u8],
            cte_info: None,
            payload: payload.to_vec(),
            crc: [0; 3],
        }
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
    fn l2cap_reassembler_completes_single_and_fragmented_pdus() {
        let direction = LinkDirection::CentralToPeripheral;
        let mut reassembler = L2capReassembler::default();
        let single = reassembler
            .push(direction, &data_packet(0x02, &[3, 0, 4, 0, 1, 2, 3]))
            .unwrap();
        assert_eq!(
            single.outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction,
                channel_id: 4,
                payload: vec![1, 2, 3],
                fragment_count: 1,
            })
        );

        let start_packet = data_packet(0x02, &[5, 0, 4, 0, 0x0a, 1]);
        let started = reassembler.push(direction, &start_packet).unwrap();
        assert_eq!(
            started.outcome,
            L2capReassemblyOutcome::InProgress(IncompleteL2capPdu {
                direction,
                channel_id: 4,
                expected_payload_length: 5,
                received_payload_length: 2,
                fragment_count: 1,
            })
        );
        assert_eq!(
            reassembler.push(direction, &start_packet).unwrap().outcome,
            L2capReassemblyOutcome::Duplicate
        );
        let complete = reassembler
            .push(direction, &data_packet(0x09, &[0, 2, 0]))
            .unwrap();
        assert_eq!(
            complete.outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction,
                channel_id: 4,
                payload: vec![0x0a, 1, 0, 2, 0],
                fragment_count: 2,
            })
        );
        assert_eq!(reassembler.pending(direction), None);
    }

    #[test]
    fn l2cap_reassembler_keeps_directions_independent() {
        let mut reassembler = L2capReassembler::default();
        let central = LinkDirection::CentralToPeripheral;
        let peripheral = LinkDirection::PeripheralToCentral;
        reassembler
            .push(central, &data_packet(0x02, &[3, 0, 4, 0, 1]))
            .unwrap();
        reassembler
            .push(peripheral, &data_packet(0x0a, &[2, 0, 6, 0, 7]))
            .unwrap();

        let peripheral_complete = reassembler
            .push(peripheral, &data_packet(0x01, &[8]))
            .unwrap();
        assert_eq!(
            peripheral_complete.outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction: peripheral,
                channel_id: 6,
                payload: vec![7, 8],
                fragment_count: 2,
            })
        );
        let central_complete = reassembler
            .push(central, &data_packet(0x09, &[2, 3]))
            .unwrap();
        assert_eq!(
            central_complete.outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction: central,
                channel_id: 4,
                payload: vec![1, 2, 3],
                fragment_count: 2,
            })
        );
    }

    #[test]
    fn l2cap_reassembler_reports_replacement_orphan_and_reset() {
        let direction = LinkDirection::CentralToPeripheral;
        let mut reassembler = L2capReassembler::default();
        assert_eq!(
            reassembler
                .push(direction, &data_packet(0x01, &[9, 8]))
                .unwrap()
                .outcome,
            L2capReassemblyOutcome::OrphanedContinuation { fragment_octets: 2 }
        );
        reassembler
            .push(direction, &data_packet(0x02, &[4, 0, 4, 0, 1]))
            .unwrap();
        let replacement = reassembler
            .push(direction, &data_packet(0x0a, &[2, 0, 6, 0, 7]))
            .unwrap();
        assert_eq!(
            replacement.replaced,
            Some(IncompleteL2capPdu {
                direction,
                channel_id: 4,
                expected_payload_length: 4,
                received_payload_length: 1,
                fragment_count: 1,
            })
        );
        assert_eq!(
            reassembler.reset(direction),
            Some(IncompleteL2capPdu {
                direction,
                channel_id: 6,
                expected_payload_length: 2,
                received_payload_length: 1,
                fragment_count: 1,
            })
        );
        assert_eq!(reassembler.pending(direction), None);
    }

    #[test]
    fn l2cap_reassembler_rejects_malformed_lengths_and_recovers() {
        let direction = LinkDirection::CentralToPeripheral;
        let mut reassembler = L2capReassembler::new(5).unwrap();
        assert!(
            reassembler
                .push(direction, &data_packet(0x02, &[6, 0, 4, 0]))
                .is_err()
        );
        assert!(
            reassembler
                .push(direction, &data_packet(0x0a, &[1, 0, 4, 0, 1, 2]))
                .is_err()
        );
        reassembler
            .push(direction, &data_packet(0x02, &[3, 0, 4, 0, 1]))
            .unwrap();
        assert!(
            reassembler
                .push(direction, &data_packet(0x09, &[2, 3, 4]))
                .is_err()
        );
        assert_eq!(reassembler.pending(direction), None);
        assert!(
            reassembler
                .push(direction, &data_packet(0x02, &[3, 0, 4]))
                .is_err()
        );
        assert_eq!(
            reassembler
                .push(direction, &data_packet(0x0a, &[1, 0, 4, 0, 9]))
                .unwrap()
                .outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction,
                channel_id: 4,
                payload: vec![9],
                fragment_count: 1,
            })
        );
        assert!(L2capReassembler::new(65_536).is_err());
    }

    #[test]
    fn l2cap_reassembler_ignores_control_and_empty_pdus() {
        let direction = LinkDirection::PeripheralToCentral;
        let mut reassembler = L2capReassembler::default();
        for packet in [data_packet(0x03, &[0x14]), data_packet(0x01, &[])] {
            assert_eq!(
                reassembler.push(direction, &packet).unwrap().outcome,
                L2capReassemblyOutcome::Ignored
            );
        }
        assert!(reassembler.reset_all().is_empty());
        assert_eq!(
            LinkDirection::CentralToPeripheral.to_string(),
            "central-to-peripheral"
        );
    }

    #[test]
    fn l2cap_reassembler_validates_public_data_pdu_invariants() {
        let direction = LinkDirection::CentralToPeripheral;
        let mut reassembler = L2capReassembler::default();
        let mut packet = data_packet(0x02, &[1, 0, 4, 0, 9]);
        packet.header[1] = 4;
        assert!(reassembler.push(direction, &packet).is_err());

        packet.header[1] = 5;
        packet.header[0] |= 0x20;
        assert!(reassembler.push(direction, &packet).is_err());
        packet.cte_info = Some(0x85);
        assert_eq!(
            reassembler.push(direction, &packet).unwrap().outcome,
            L2capReassemblyOutcome::Complete(L2capPdu {
                direction,
                channel_id: 4,
                payload: vec![9],
                fragment_count: 1,
            })
        );
    }

    #[test]
    fn l2cap_reassembler_handles_arbitrary_bounded_data_pdus_without_panicking() {
        let mut reassembler = L2capReassembler::default();
        for header in 0u8..=u8::MAX {
            for payload_length in [0usize, 1, 2, 3, 4, 5, 31, 255] {
                let direction = if header & 1 == 0 {
                    LinkDirection::CentralToPeripheral
                } else {
                    LinkDirection::PeripheralToCentral
                };
                let mut packet = data_packet(header, &vec![header; payload_length]);
                if packet.constant_tone_extension_present() {
                    packet.cte_info = Some(0x85);
                }
                let _ = reassembler.push(direction, &packet);
                reassembler.reset(direction);
            }
        }
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

    #[test]
    fn parses_connection_update_and_channel_map_control_pdus() {
        let connection_update = ControlPdu {
            opcode: 0x00,
            parameters: &[2, 3, 0, 24, 0, 4, 0, 100, 0, 0x34, 0x12],
        }
        .connection_update_ind()
        .unwrap()
        .unwrap();
        assert_eq!(connection_update.window_size, 2);
        assert_eq!(connection_update.window_offset, 3);
        assert_eq!(connection_update.parameters.interval, 24);
        assert_eq!(connection_update.parameters.latency, 4);
        assert_eq!(connection_update.parameters.supervision_timeout, 100);
        assert_eq!(connection_update.instant, 0x1234);
        assert_eq!(connection_update.window_offset_us(), 3_750);
        assert_eq!(connection_update.window_size_us(), 2_500);

        let channel_map = ControlPdu {
            opcode: 0x01,
            parameters: &[0x06, 0, 0, 0, 0, 2, 0],
        }
        .channel_map_ind()
        .unwrap()
        .unwrap();
        assert_eq!(channel_map.channel_map.used_channels(), [1, 2]);
        assert_eq!(channel_map.instant, 2);

        assert!(
            ControlPdu {
                opcode: 0x02,
                parameters: &[]
            }
            .connection_update_ind()
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn rejects_malformed_instant_control_pdus() {
        let short_update = ControlPdu {
            opcode: 0x00,
            parameters: &[1; 10],
        };
        assert!(short_update.connection_update_ind().is_err());

        let invalid_window = ControlPdu {
            opcode: 0x00,
            parameters: &[0, 0, 0, 24, 0, 0, 0, 100, 0, 6, 0],
        };
        assert!(invalid_window.connection_update_ind().is_err());

        let reserved_map = ControlPdu {
            opcode: 0x01,
            parameters: &[0x03, 0, 0, 0, 0x20, 6, 0],
        };
        assert!(reserved_map.channel_map_ind().is_err());

        let short_map = ControlPdu {
            opcode: 0x01,
            parameters: &[0x03, 0, 0, 0, 0, 6],
        };
        assert!(short_map.channel_map_ind().is_err());
    }

    #[test]
    fn classifies_instants_across_event_counter_wrap() {
        assert_eq!(instant_relation(65_532, 2), InstantRelation::Future(6));
        assert_eq!(instant_relation(2, 65_532), InstantRelation::Passed(6));
        assert_eq!(instant_relation(44, 44), InstantRelation::Reached);
        assert_eq!(instant_relation(0, 0x7fff), InstantRelation::Ambiguous);
        assert_eq!(instant_relation(0, 0x8000), InstantRelation::Ambiguous);
    }

    #[test]
    fn tracker_applies_channel_map_before_selecting_instant_event() {
        let mut tracker = ConnectionTracker::new(
            tracker_config(ChannelSelectionAlgorithm::Csa2),
            65_532,
            1_000,
        )
        .unwrap();
        assert!(
            tracker
                .schedule_control(ControlPdu {
                    opcode: 0x01,
                    parameters: &[0x06, 0, 0, 0, 0, 2, 0],
                })
                .unwrap()
        );

        let mut event = tracker.current_event().unwrap();
        for _ in 0..6 {
            event = tracker.advance().unwrap();
        }
        assert_eq!(event.event_counter, 2);
        assert!(matches!(event.channel.index(), 1 | 2));
        assert_eq!(tracker.channel_map().used_channels(), [1, 2]);
        assert_eq!(
            event.timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 721_000
            }
        );
    }

    #[test]
    fn tracker_requires_a_new_anchor_at_connection_update_instant() {
        let mut tracker =
            ConnectionTracker::new(tracker_config(ChannelSelectionAlgorithm::Csa1), 10, 1_000)
                .unwrap();
        tracker
            .schedule_connection_update(ConnectionUpdateInd::new(2, 3, 40, 0, 100, 12).unwrap())
            .unwrap();

        assert_eq!(
            tracker.advance().unwrap().timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 121_000
            }
        );
        let update_event = tracker.advance().unwrap();
        assert_eq!(update_event.event_counter, 12);
        assert_eq!(
            update_event.timing,
            ConnectionEventTiming::AnchorObservationRequired {
                window_offset: 3,
                window_size: 2
            }
        );
        assert_eq!(tracker.parameters().interval, 40);
        assert!(tracker.advance().is_err());

        assert_eq!(
            tracker.observe_anchor(500_000).unwrap().timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 500_000
            }
        );
        assert_eq!(
            tracker.advance().unwrap().timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 700_000
            }
        );
    }

    #[test]
    fn tracker_rejects_nonfuture_and_overlapping_updates() {
        let mut tracker =
            ConnectionTracker::new(tracker_config(ChannelSelectionAlgorithm::Csa2), 100, 0)
                .unwrap();
        assert!(
            tracker
                .schedule_connection_update(ConnectionUpdateInd {
                    window_size: 0,
                    window_offset: 0,
                    parameters: ConnectionParameters {
                        interval: 24,
                        latency: 0,
                        supervision_timeout: 100,
                    },
                    instant: 101,
                })
                .is_err()
        );
        assert!(
            tracker
                .schedule_channel_map(ChannelMapInd {
                    channel_map: all_channels(),
                    instant: 100,
                })
                .is_err()
        );
        assert!(
            tracker
                .schedule_channel_map(ChannelMapInd {
                    channel_map: all_channels(),
                    instant: 99,
                })
                .is_err()
        );
        assert!(
            tracker
                .schedule_channel_map(ChannelMapInd {
                    channel_map: all_channels(),
                    instant: 100u16.wrapping_add(0x8000),
                })
                .is_err()
        );

        tracker
            .schedule_channel_map(ChannelMapInd {
                channel_map: all_channels(),
                instant: 101,
            })
            .unwrap();
        assert!(
            tracker
                .schedule_connection_update(
                    ConnectionUpdateInd::new(1, 0, 24, 0, 100, 102).unwrap()
                )
                .is_err()
        );
    }

    #[test]
    fn tracker_uses_anchor_relative_rounding_without_cumulative_drift() {
        let mut config = tracker_config(ChannelSelectionAlgorithm::Csa2);
        config.parameters = ConnectionParameters::new(6, 0, 100).unwrap();
        config.sample_rate_hz = 2_000_001;
        let mut tracker = ConnectionTracker::new(config, 0, 0).unwrap();

        let mut event = tracker.current_event().unwrap();
        for _ in 0..100 {
            event = tracker.advance().unwrap();
        }
        assert_eq!(event.event_counter, 100);
        assert_eq!(
            event.timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 1_500_001
            }
        );
    }

    #[test]
    fn tracker_rejects_anchor_observation_when_timing_is_already_known() {
        let mut tracker =
            ConnectionTracker::new(tracker_config(ChannelSelectionAlgorithm::Csa2), 0, 0).unwrap();
        assert!(tracker.observe_anchor(10).is_err());
        assert!(
            !tracker
                .schedule_control(ControlPdu {
                    opcode: 0x14,
                    parameters: &[0; 8],
                })
                .unwrap()
        );
    }

    #[test]
    fn sleep_clock_accuracy_matches_core_ranges() {
        let ppm: Vec<u16> = (0..=7)
            .map(|raw| SleepClockAccuracy::new(raw).unwrap().maximum_ppm())
            .collect();
        assert_eq!(ppm, [500, 250, 150, 100, 75, 50, 30, 20]);
        assert!(SleepClockAccuracy::new(8).is_err());
    }

    #[test]
    fn timing_windows_accumulate_combined_clock_uncertainty() {
        let mut tracker =
            ConnectionTracker::new(tracker_config(ChannelSelectionAlgorithm::Csa2), 0, 1_000)
                .unwrap();
        let sca = SleepClockAccuracy::new(0).unwrap();
        assert_eq!(
            tracker.current_timing_window(sca, 20).unwrap().unwrap(),
            ConnectionTimingWindow {
                expected_sample: 1_000,
                earliest_sample: 1_000,
                latest_sample: 1_000,
                widening_samples: 0,
            }
        );
        tracker.advance().unwrap();
        assert_eq!(
            tracker.current_timing_window(sca, 20).unwrap().unwrap(),
            ConnectionTimingWindow {
                expected_sample: 121_000,
                earliest_sample: 120_937,
                latest_sample: 121_063,
                widening_samples: 63,
            }
        );
        assert!(tracker.current_timing_window(sca, 1_000_001).is_err());
    }

    #[test]
    fn timing_window_is_capped_before_adjacent_events_overlap() {
        let mut config = tracker_config(ChannelSelectionAlgorithm::Csa2);
        config.parameters = ConnectionParameters::new(6, 0, 100).unwrap();
        let mut tracker = ConnectionTracker::new(config, 0, 20_000).unwrap();
        tracker.advance().unwrap();
        let window = tracker
            .current_timing_window(SleepClockAccuracy::new(0).unwrap(), 1_000_000)
            .unwrap()
            .unwrap();
        assert_eq!(window.widening_samples, 14_400);
    }

    #[test]
    fn tracker_matches_missed_event_and_reanchors_to_observation() {
        let mut config = tracker_config(ChannelSelectionAlgorithm::Csa2);
        config.access_address = 0x8e89_bed6;
        let mut tracker = ConnectionTracker::new(config, 0, 1_000).unwrap();
        let observation = tracker
            .synchronize_observation(
                BleChannel::new(21).unwrap(),
                361_050,
                SleepClockAccuracy::new(0).unwrap(),
                20,
                5,
            )
            .unwrap();
        assert_eq!(observation.event.event_counter, 3);
        assert_eq!(observation.advanced_events, 3);
        assert_eq!(observation.timing_error, SampleTimingError::Late(50));
        assert_eq!(observation.timing_window.widening_samples, 188);
        assert_eq!(tracker.event_counter(), 3);
        assert_eq!(
            tracker.current_event().unwrap().timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 361_050
            }
        );
        assert_eq!(
            tracker.advance().unwrap().timing,
            ConnectionEventTiming::Expected {
                access_address_sample: 481_050
            }
        );
        assert_eq!(
            tracker
                .current_timing_window(SleepClockAccuracy::new(0).unwrap(), 20)
                .unwrap()
                .unwrap()
                .widening_samples,
            63
        );
    }

    #[test]
    fn tracker_rejects_observation_outside_channel_or_timing_window() {
        let mut config = tracker_config(ChannelSelectionAlgorithm::Csa2);
        config.access_address = 0x8e89_bed6;
        let mut tracker = ConnectionTracker::new(config, 0, 1_000).unwrap();
        assert!(
            tracker
                .synchronize_observation(
                    BleChannel::new(20).unwrap(),
                    361_050,
                    SleepClockAccuracy::new(0).unwrap(),
                    20,
                    5,
                )
                .is_err()
        );
        assert!(
            tracker
                .synchronize_observation(
                    BleChannel::new(21).unwrap(),
                    362_000,
                    SleepClockAccuracy::new(0).unwrap(),
                    20,
                    5,
                )
                .is_err()
        );
        assert_eq!(tracker.event_counter(), 0);
    }

    #[test]
    fn observation_search_stops_at_unknown_connection_update_anchor() {
        let mut config = tracker_config(ChannelSelectionAlgorithm::Csa2);
        config.access_address = 0x8e89_bed6;
        let mut tracker = ConnectionTracker::new(config, 0, 1_000).unwrap();
        tracker
            .schedule_connection_update(ConnectionUpdateInd::new(1, 0, 24, 0, 100, 2).unwrap())
            .unwrap();
        assert!(
            tracker
                .synchronize_observation(
                    BleChannel::new(21).unwrap(),
                    361_000,
                    SleepClockAccuracy::new(0).unwrap(),
                    20,
                    5,
                )
                .is_err()
        );
        assert_eq!(tracker.event_counter(), 0);
    }
}
