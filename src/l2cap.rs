use crate::att::AttPdu;
use crate::link_layer::{
    DecodedL2capSignalingCommand, L2capDisconnection, L2capEnhancedCreditBasedConnectionRequest,
    L2capEnhancedCreditBasedConnectionResponse, L2capEnhancedCreditBasedReconfigureRequest,
    L2capLeCreditBasedConnectionResponse, L2capPdu, LE_SIGNALING_CHANNEL_ID, LinkDirection,
};
use crate::{Error, Result};

pub const L2CAP_LE_DYNAMIC_CID_START: u16 = 0x0040;
pub const L2CAP_LE_DYNAMIC_CID_END: u16 = 0x007f;
pub const L2CAP_LE_SPSM_START: u16 = 0x0001;
pub const L2CAP_LE_SPSM_END: u16 = 0x00ff;
pub const L2CAP_LE_CREDIT_BASED_MINIMUM_MTU: u16 = 23;
pub const L2CAP_LE_CREDIT_BASED_MINIMUM_MPS: u16 = 23;
pub const L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MTU: u16 = 64;
pub const L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MPS: u16 = 64;
pub const L2CAP_CREDIT_BASED_MAXIMUM_MPS: u16 = 65_533;
pub const EATT_SPSM: u16 = 0x0027;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum L2capCreditBasedMode {
    LeCreditBased,
    EnhancedCreditBased,
}

impl std::fmt::Display for L2capCreditBasedMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LeCreditBased => formatter.write_str("le-credit-based"),
            Self::EnhancedCreditBased => formatter.write_str("enhanced-credit-based"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum L2capCreditBasedChannelStatus {
    Open,
    Disconnecting,
}

impl std::fmt::Display for L2capCreditBasedChannelStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => formatter.write_str("open"),
            Self::Disconnecting => formatter.write_str("disconnecting"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct L2capCreditBasedEndpoint {
    pub channel_id: u16,
    pub mtu: u16,
    pub mps: u16,
    pub credits: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L2capCreditBasedChannel {
    pub mode: L2capCreditBasedMode,
    pub spsm: u16,
    pub central: L2capCreditBasedEndpoint,
    pub peripheral: L2capCreditBasedEndpoint,
    pub status: L2capCreditBasedChannelStatus,
}

impl L2capCreditBasedChannel {
    pub const fn is_eatt(&self) -> bool {
        self.spsm == EATT_SPSM
    }

    pub const fn destination(&self, direction: LinkDirection) -> L2capCreditBasedEndpoint {
        match direction {
            LinkDirection::CentralToPeripheral => self.peripheral,
            LinkDirection::PeripheralToCentral => self.central,
        }
    }

    pub const fn owner(&self, direction: LinkDirection) -> L2capCreditBasedEndpoint {
        match direction {
            LinkDirection::CentralToPeripheral => self.central,
            LinkDirection::PeripheralToCentral => self.peripheral,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncompleteL2capCreditBasedSdu {
    pub direction: LinkDirection,
    pub channel_id: u16,
    pub spsm: u16,
    pub expected_octets: usize,
    pub received_octets: usize,
    pub segment_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L2capCreditBasedSdu {
    pub direction: LinkDirection,
    pub channel_id: u16,
    pub spsm: u16,
    pub payload: Vec<u8>,
    pub segment_count: u32,
}

impl L2capCreditBasedSdu {
    pub const fn is_eatt(&self) -> bool {
        self.spsm == EATT_SPSM
    }

    pub fn att_pdu(&self) -> Result<Option<AttPdu<'_>>> {
        if !self.is_eatt() {
            return Ok(None);
        }
        Ok(Some(AttPdu::parse(&self.payload)?))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum L2capCreditBasedEvent {
    Ignored,
    ConnectionRequestPending {
        mode: L2capCreditBasedMode,
        identifier: u8,
        spsm: u16,
        channel_count: usize,
    },
    ConnectionRejected {
        mode: L2capCreditBasedMode,
        identifier: u8,
        result: u16,
    },
    ChannelsOpened(Vec<L2capCreditBasedChannel>),
    CreditsAdded {
        owner: LinkDirection,
        channel_id: u16,
        added: u16,
        total: u16,
    },
    SduInProgress(IncompleteL2capCreditBasedSdu),
    SduComplete(L2capCreditBasedSdu),
    ReconfigurePending {
        identifier: u8,
        owner: LinkDirection,
        channel_ids: Vec<u16>,
        mtu: u16,
        mps: u16,
    },
    ReconfigureRejected {
        identifier: u8,
        result: u16,
    },
    Reconfigured {
        owner: LinkDirection,
        channel_ids: Vec<u16>,
        mtu: u16,
        mps: u16,
    },
    DisconnectPending {
        identifier: u8,
        central_channel_id: u16,
        peripheral_channel_id: u16,
    },
    Disconnected(L2capCreditBasedChannel),
    CommandRejected {
        identifier: u8,
        removed_pending_procedure: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingSdu {
    expected_octets: usize,
    payload: Vec<u8>,
    segment_count: u32,
}

impl PendingSdu {
    fn incomplete(
        &self,
        direction: LinkDirection,
        channel_id: u16,
        spsm: u16,
    ) -> IncompleteL2capCreditBasedSdu {
        IncompleteL2capCreditBasedSdu {
            direction,
            channel_id,
            spsm,
            expected_octets: self.expected_octets,
            received_octets: self.payload.len(),
            segment_count: self.segment_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TrackedChannel {
    public: L2capCreditBasedChannel,
    central_to_peripheral_sdu: Option<PendingSdu>,
    peripheral_to_central_sdu: Option<PendingSdu>,
}

impl TrackedChannel {
    fn pending_sdu(&self, direction: LinkDirection) -> &Option<PendingSdu> {
        match direction {
            LinkDirection::CentralToPeripheral => &self.central_to_peripheral_sdu,
            LinkDirection::PeripheralToCentral => &self.peripheral_to_central_sdu,
        }
    }

    fn pending_sdu_mut(&mut self, direction: LinkDirection) -> &mut Option<PendingSdu> {
        match direction {
            LinkDirection::CentralToPeripheral => &mut self.central_to_peripheral_sdu,
            LinkDirection::PeripheralToCentral => &mut self.peripheral_to_central_sdu,
        }
    }

    fn endpoint_mut(&mut self, owner: LinkDirection) -> &mut L2capCreditBasedEndpoint {
        match owner {
            LinkDirection::CentralToPeripheral => &mut self.public.central,
            LinkDirection::PeripheralToCentral => &mut self.public.peripheral,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingProcedure {
    Connection {
        direction: LinkDirection,
        identifier: u8,
        mode: L2capCreditBasedMode,
        spsm: u16,
        mtu: u16,
        mps: u16,
        credits: u16,
        source_channel_ids: Vec<u16>,
    },
    Reconfigure {
        direction: LinkDirection,
        identifier: u8,
        mtu: u16,
        mps: u16,
        channel_ids: Vec<u16>,
    },
    Disconnect {
        direction: LinkDirection,
        identifier: u8,
        channel_index: usize,
        disconnection: L2capDisconnection,
    },
}

impl PendingProcedure {
    const fn direction(&self) -> LinkDirection {
        match self {
            Self::Connection { direction, .. }
            | Self::Reconfigure { direction, .. }
            | Self::Disconnect { direction, .. } => *direction,
        }
    }

    const fn identifier(&self) -> u8 {
        match self {
            Self::Connection { identifier, .. }
            | Self::Reconfigure { identifier, .. }
            | Self::Disconnect { identifier, .. } => *identifier,
        }
    }
}

/// Tracks LE credit-based dynamic channels from ordered, direction-tagged L2CAP PDUs.
///
/// Inputs must be complete plaintext L2CAP PDUs, normally produced by
/// [`crate::link_layer::L2capReassembler`]. The tracker correlates signaling
/// requests and responses, maps each peer's dynamic CID, accounts for
/// direction-specific credits, reassembles credit-based SDUs, and recognizes
/// EATT only from the assigned SPSM. Every observation is transactional.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct L2capCreditBasedChannelTracker {
    channels: Vec<TrackedChannel>,
    pending: Vec<PendingProcedure>,
}

impl L2capCreditBasedChannelTracker {
    pub fn channels(&self) -> impl Iterator<Item = &L2capCreditBasedChannel> {
        self.channels.iter().map(|channel| &channel.public)
    }

    pub fn channel_for_data(
        &self,
        direction: LinkDirection,
        channel_id: u16,
    ) -> Option<&L2capCreditBasedChannel> {
        self.channel_index_for_data(direction, channel_id)
            .map(|index| &self.channels[index].public)
    }

    pub fn pending_sdu(
        &self,
        direction: LinkDirection,
        channel_id: u16,
    ) -> Option<IncompleteL2capCreditBasedSdu> {
        let channel = &self.channels[self.channel_index_for_data(direction, channel_id)?];
        channel
            .pending_sdu(direction)
            .as_ref()
            .map(|pending| pending.incomplete(direction, channel_id, channel.public.spsm))
    }

    pub fn reset(&mut self) {
        self.channels.clear();
        self.pending.clear();
    }

    pub fn observe(&mut self, pdu: &L2capPdu) -> Result<L2capCreditBasedEvent> {
        let mut candidate = self.clone();
        let event = candidate.process(pdu)?;
        *self = candidate;
        Ok(event)
    }

    fn process(&mut self, pdu: &L2capPdu) -> Result<L2capCreditBasedEvent> {
        if pdu.channel_id == LE_SIGNALING_CHANNEL_ID {
            return self.process_signaling(pdu);
        }
        if is_dynamic_channel_id(pdu.channel_id) {
            return self.process_data(pdu);
        }
        Ok(L2capCreditBasedEvent::Ignored)
    }

    fn process_signaling(&mut self, pdu: &L2capPdu) -> Result<L2capCreditBasedEvent> {
        let command = pdu.le_signaling_command()?.ok_or_else(|| {
            Error::InvalidState("LE signaling CID did not produce a signaling command".to_owned())
        })?;
        match command.decode()? {
            DecodedL2capSignalingCommand::LeCreditBasedConnectionRequest(request) => self
                .connection_request(
                    pdu.direction,
                    command.identifier,
                    L2capCreditBasedMode::LeCreditBased,
                    request.spsm,
                    request.mtu,
                    request.mps,
                    request.initial_credits,
                    vec![request.source_channel_id],
                ),
            DecodedL2capSignalingCommand::LeCreditBasedConnectionResponse(response) => {
                self.le_connection_response(pdu.direction, command.identifier, response)
            }
            DecodedL2capSignalingCommand::EnhancedCreditBasedConnectionRequest(request) => {
                self.enhanced_connection_request(pdu.direction, command.identifier, request)
            }
            DecodedL2capSignalingCommand::EnhancedCreditBasedConnectionResponse(response) => {
                self.enhanced_connection_response(pdu.direction, command.identifier, response)
            }
            DecodedL2capSignalingCommand::FlowControlCredit(credit) => {
                self.add_credits(pdu.direction, credit.channel_id, credit.credits)
            }
            DecodedL2capSignalingCommand::EnhancedCreditBasedReconfigureRequest(request) => {
                self.reconfigure_request(pdu.direction, command.identifier, request)
            }
            DecodedL2capSignalingCommand::EnhancedCreditBasedReconfigureResponse(response) => {
                self.reconfigure_response(pdu.direction, command.identifier, response.result)
            }
            DecodedL2capSignalingCommand::DisconnectionRequest(disconnection) => {
                self.disconnect_request(pdu.direction, command.identifier, disconnection)
            }
            DecodedL2capSignalingCommand::DisconnectionResponse(disconnection) => {
                self.disconnect_response(pdu.direction, command.identifier, disconnection)
            }
            DecodedL2capSignalingCommand::CommandReject(_) => {
                Ok(self.command_rejected(pdu.direction, command.identifier))
            }
            DecodedL2capSignalingCommand::ConnectionParameterUpdateRequest(_)
            | DecodedL2capSignalingCommand::ConnectionParameterUpdateResponse(_)
            | DecodedL2capSignalingCommand::Unknown { .. } => Ok(L2capCreditBasedEvent::Ignored),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn connection_request(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        mode: L2capCreditBasedMode,
        spsm: u16,
        mtu: u16,
        mps: u16,
        credits: u16,
        source_channel_ids: Vec<u16>,
    ) -> Result<L2capCreditBasedEvent> {
        validate_spsm(spsm)?;
        validate_endpoint_parameters(mode, mtu, mps, credits)?;
        validate_channel_id_list(&source_channel_ids, false)?;
        let pending = PendingProcedure::Connection {
            direction,
            identifier,
            mode,
            spsm,
            mtu,
            mps,
            credits,
            source_channel_ids: source_channel_ids.clone(),
        };
        if let Some(existing) = self.pending_for_request(direction, identifier) {
            if existing == &pending {
                let PendingProcedure::Connection {
                    mode,
                    spsm,
                    source_channel_ids,
                    ..
                } = existing
                else {
                    unreachable!()
                };
                return Ok(L2capCreditBasedEvent::ConnectionRequestPending {
                    mode: *mode,
                    identifier,
                    spsm: *spsm,
                    channel_count: source_channel_ids.len(),
                });
            }
            return Err(Error::InvalidState(format!(
                "{} signaling identifier {identifier} already has a pending procedure",
                endpoint_owner_name(direction)
            )));
        }
        for &channel_id in &source_channel_ids {
            if self.owner_channel_id_in_use(direction, channel_id) {
                return Err(Error::InvalidState(format!(
                    "{} endpoint CID 0x{channel_id:04x} is already in use",
                    endpoint_owner_name(direction)
                )));
            }
        }
        let (mode, spsm, channel_count) = match &pending {
            PendingProcedure::Connection {
                mode,
                spsm,
                source_channel_ids,
                ..
            } => (*mode, *spsm, source_channel_ids.len()),
            _ => unreachable!(),
        };
        self.pending.push(pending);
        Ok(L2capCreditBasedEvent::ConnectionRequestPending {
            mode,
            identifier,
            spsm,
            channel_count,
        })
    }

    fn enhanced_connection_request(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        request: L2capEnhancedCreditBasedConnectionRequest,
    ) -> Result<L2capCreditBasedEvent> {
        self.connection_request(
            direction,
            identifier,
            L2capCreditBasedMode::EnhancedCreditBased,
            request.spsm,
            request.mtu,
            request.mps,
            request.initial_credits,
            request.source_channel_ids.as_slice().to_vec(),
        )
    }

    fn le_connection_response(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        response: L2capLeCreditBasedConnectionResponse,
    ) -> Result<L2capCreditBasedEvent> {
        let pending_index = self.pending_connection_index(
            direction,
            identifier,
            L2capCreditBasedMode::LeCreditBased,
        )?;
        let PendingProcedure::Connection {
            direction: request_direction,
            spsm,
            mtu,
            mps,
            credits,
            source_channel_ids,
            ..
        } = self.pending[pending_index].clone()
        else {
            unreachable!()
        };
        if response.result != 0 {
            self.pending.remove(pending_index);
            return Ok(L2capCreditBasedEvent::ConnectionRejected {
                mode: L2capCreditBasedMode::LeCreditBased,
                identifier,
                result: response.result,
            });
        }
        validate_endpoint_parameters(
            L2capCreditBasedMode::LeCreditBased,
            response.mtu,
            response.mps,
            response.initial_credits,
        )?;
        validate_channel_id_list(&[response.destination_channel_id], false)?;
        if self.owner_channel_id_in_use(direction, response.destination_channel_id) {
            return Err(Error::InvalidState(format!(
                "{} endpoint CID 0x{:04x} is already in use",
                endpoint_owner_name(direction),
                response.destination_channel_id
            )));
        }
        let channel = build_channel(
            L2capCreditBasedMode::LeCreditBased,
            spsm,
            request_direction,
            L2capCreditBasedEndpoint {
                channel_id: source_channel_ids[0],
                mtu,
                mps,
                credits,
            },
            L2capCreditBasedEndpoint {
                channel_id: response.destination_channel_id,
                mtu: response.mtu,
                mps: response.mps,
                credits: response.initial_credits,
            },
        );
        self.pending.remove(pending_index);
        self.channels.push(TrackedChannel {
            public: channel.clone(),
            central_to_peripheral_sdu: None,
            peripheral_to_central_sdu: None,
        });
        Ok(L2capCreditBasedEvent::ChannelsOpened(vec![channel]))
    }

    fn enhanced_connection_response(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        response: L2capEnhancedCreditBasedConnectionResponse,
    ) -> Result<L2capCreditBasedEvent> {
        let pending_index = self.pending_connection_index(
            direction,
            identifier,
            L2capCreditBasedMode::EnhancedCreditBased,
        )?;
        let PendingProcedure::Connection {
            direction: request_direction,
            spsm,
            mtu,
            mps,
            credits,
            source_channel_ids,
            ..
        } = self.pending[pending_index].clone()
        else {
            unreachable!()
        };
        let destination_channel_ids = response.destination_channel_ids.as_slice();
        if destination_channel_ids.len() != source_channel_ids.len() {
            return Err(Error::InvalidInput(format!(
                "enhanced credit response carries {} destination CIDs for {} requested channels",
                destination_channel_ids.len(),
                source_channel_ids.len()
            )));
        }
        let partial_result = matches!(response.result, 0x0004 | 0x0009 | 0x000a);
        if response.result != 0 && !partial_result {
            if destination_channel_ids
                .iter()
                .any(|channel_id| *channel_id != 0)
            {
                return Err(Error::InvalidInput(format!(
                    "enhanced credit response result 0x{:04x} must not establish destination CIDs",
                    response.result
                )));
            }
            self.pending.remove(pending_index);
            return Ok(L2capCreditBasedEvent::ConnectionRejected {
                mode: L2capCreditBasedMode::EnhancedCreditBased,
                identifier,
                result: response.result,
            });
        }
        if response.result != 0
            && destination_channel_ids
                .iter()
                .all(|channel_id| *channel_id == 0)
        {
            self.pending.remove(pending_index);
            return Ok(L2capCreditBasedEvent::ConnectionRejected {
                mode: L2capCreditBasedMode::EnhancedCreditBased,
                identifier,
                result: response.result,
            });
        }
        validate_endpoint_parameters(
            L2capCreditBasedMode::EnhancedCreditBased,
            response.mtu,
            response.mps,
            response.initial_credits,
        )?;
        if response.result == 0 && destination_channel_ids.contains(&0) {
            return Err(Error::InvalidInput(
                "successful enhanced credit response contains a zero destination CID".to_owned(),
            ));
        }
        validate_channel_id_list(destination_channel_ids, partial_result)?;
        let mut nonzero_destination_ids = Vec::new();
        for &channel_id in destination_channel_ids {
            if channel_id == 0 {
                continue;
            }
            if nonzero_destination_ids.contains(&channel_id) {
                return Err(Error::InvalidInput(format!(
                    "enhanced credit response repeats destination CID 0x{channel_id:04x}"
                )));
            }
            if self.owner_channel_id_in_use(direction, channel_id) {
                return Err(Error::InvalidState(format!(
                    "{} endpoint CID 0x{channel_id:04x} is already in use",
                    endpoint_owner_name(direction)
                )));
            }
            nonzero_destination_ids.push(channel_id);
        }
        let mut opened = Vec::new();
        for (&source_channel_id, &destination_channel_id) in
            source_channel_ids.iter().zip(destination_channel_ids)
        {
            if destination_channel_id == 0 {
                continue;
            }
            let channel = build_channel(
                L2capCreditBasedMode::EnhancedCreditBased,
                spsm,
                request_direction,
                L2capCreditBasedEndpoint {
                    channel_id: source_channel_id,
                    mtu,
                    mps,
                    credits,
                },
                L2capCreditBasedEndpoint {
                    channel_id: destination_channel_id,
                    mtu: response.mtu,
                    mps: response.mps,
                    credits: response.initial_credits,
                },
            );
            self.channels.push(TrackedChannel {
                public: channel.clone(),
                central_to_peripheral_sdu: None,
                peripheral_to_central_sdu: None,
            });
            opened.push(channel);
        }
        self.pending.remove(pending_index);
        if opened.is_empty() {
            Ok(L2capCreditBasedEvent::ConnectionRejected {
                mode: L2capCreditBasedMode::EnhancedCreditBased,
                identifier,
                result: response.result,
            })
        } else {
            Ok(L2capCreditBasedEvent::ChannelsOpened(opened))
        }
    }

    fn add_credits(
        &mut self,
        owner: LinkDirection,
        channel_id: u16,
        credits: u16,
    ) -> Result<L2capCreditBasedEvent> {
        if credits == 0 {
            return Err(Error::InvalidInput(
                "flow-control credit indication must add at least one credit".to_owned(),
            ));
        }
        let channel_index = self
            .channel_index_for_owner(owner, channel_id)
            .ok_or_else(|| {
                Error::InvalidState(format!(
                    "no {} endpoint uses CID 0x{channel_id:04x}",
                    endpoint_owner_name(owner)
                ))
            })?;
        if self.channels[channel_index].public.status != L2capCreditBasedChannelStatus::Open {
            return Err(Error::InvalidState(format!(
                "dynamic CID 0x{channel_id:04x} is {}",
                self.channels[channel_index].public.status
            )));
        }
        let endpoint = self.channels[channel_index].endpoint_mut(owner);
        endpoint.credits = endpoint.credits.checked_add(credits).ok_or_else(|| {
            Error::InvalidState(format!(
                "flow-control credits overflow for CID 0x{channel_id:04x}"
            ))
        })?;
        Ok(L2capCreditBasedEvent::CreditsAdded {
            owner,
            channel_id,
            added: credits,
            total: endpoint.credits,
        })
    }

    fn process_data(&mut self, pdu: &L2capPdu) -> Result<L2capCreditBasedEvent> {
        let channel_index = self
            .channel_index_for_data(pdu.direction, pdu.channel_id)
            .ok_or_else(|| {
                Error::InvalidState(format!(
                    "dynamic CID 0x{:04x} is not open for {} data",
                    pdu.channel_id, pdu.direction
                ))
            })?;
        let channel = &mut self.channels[channel_index];
        if channel.public.status != L2capCreditBasedChannelStatus::Open {
            return Err(Error::InvalidState(format!(
                "dynamic CID 0x{:04x} is {}",
                pdu.channel_id, channel.public.status
            )));
        }
        let endpoint = match pdu.direction {
            LinkDirection::CentralToPeripheral => &mut channel.public.peripheral,
            LinkDirection::PeripheralToCentral => &mut channel.public.central,
        };
        if pdu.payload.len() > usize::from(endpoint.mps) {
            return Err(Error::InvalidInput(format!(
                "credit-based PDU on CID 0x{:04x} carries {} octets, exceeding MPS {}",
                pdu.channel_id,
                pdu.payload.len(),
                endpoint.mps
            )));
        }
        if endpoint.credits == 0 {
            return Err(Error::InvalidState(format!(
                "credit-based PDU on CID 0x{:04x} arrived with no remaining credits",
                pdu.channel_id
            )));
        }
        endpoint.credits -= 1;
        let spsm = channel.public.spsm;
        let mtu = endpoint.mtu;
        let pending_slot = channel.pending_sdu_mut(pdu.direction);
        if let Some(mut pending) = pending_slot.take() {
            let remaining = pending.expected_octets - pending.payload.len();
            if pdu.payload.len() > remaining {
                return Err(Error::InvalidInput(format!(
                    "credit-based SDU continuation on CID 0x{:04x} carries {} octets with {remaining} remaining",
                    pdu.channel_id,
                    pdu.payload.len()
                )));
            }
            pending.payload.extend_from_slice(&pdu.payload);
            pending.segment_count = pending.segment_count.checked_add(1).ok_or_else(|| {
                Error::InvalidState("credit-based SDU segment count overflow".to_owned())
            })?;
            if pending.payload.len() == pending.expected_octets {
                return Ok(L2capCreditBasedEvent::SduComplete(L2capCreditBasedSdu {
                    direction: pdu.direction,
                    channel_id: pdu.channel_id,
                    spsm,
                    payload: pending.payload,
                    segment_count: pending.segment_count,
                }));
            }
            let incomplete = pending.incomplete(pdu.direction, pdu.channel_id, spsm);
            *pending_slot = Some(pending);
            return Ok(L2capCreditBasedEvent::SduInProgress(incomplete));
        }

        if pdu.payload.len() < 2 {
            return Err(Error::InvalidInput(format!(
                "first credit-based PDU on CID 0x{:04x} is missing the two-octet SDU length",
                pdu.channel_id
            )));
        }
        let expected_octets = usize::from(u16::from_le_bytes([pdu.payload[0], pdu.payload[1]]));
        if expected_octets > usize::from(mtu) {
            return Err(Error::InvalidInput(format!(
                "credit-based SDU on CID 0x{:04x} declares {expected_octets} octets, exceeding MTU {mtu}",
                pdu.channel_id
            )));
        }
        let fragment = &pdu.payload[2..];
        if fragment.len() > expected_octets {
            return Err(Error::InvalidInput(format!(
                "first credit-based PDU on CID 0x{:04x} carries {} SDU octets, exceeding declared length {expected_octets}",
                pdu.channel_id,
                fragment.len()
            )));
        }
        if fragment.len() == expected_octets {
            return Ok(L2capCreditBasedEvent::SduComplete(L2capCreditBasedSdu {
                direction: pdu.direction,
                channel_id: pdu.channel_id,
                spsm,
                payload: fragment.to_vec(),
                segment_count: 1,
            }));
        }
        let pending = PendingSdu {
            expected_octets,
            payload: fragment.to_vec(),
            segment_count: 1,
        };
        let incomplete = pending.incomplete(pdu.direction, pdu.channel_id, spsm);
        *pending_slot = Some(pending);
        Ok(L2capCreditBasedEvent::SduInProgress(incomplete))
    }

    fn reconfigure_request(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        request: L2capEnhancedCreditBasedReconfigureRequest,
    ) -> Result<L2capCreditBasedEvent> {
        if request.mtu < L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MTU {
            return Err(Error::InvalidInput(format!(
                "enhanced credit reconfiguration MTU {} is below {}",
                request.mtu, L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MTU
            )));
        }
        if !(L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MPS..=L2CAP_CREDIT_BASED_MAXIMUM_MPS)
            .contains(&request.mps)
        {
            return Err(Error::InvalidInput(format!(
                "enhanced credit reconfiguration MPS {} is outside {}..={}",
                request.mps,
                L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MPS,
                L2CAP_CREDIT_BASED_MAXIMUM_MPS
            )));
        }
        let channel_ids = request.channel_ids.as_slice().to_vec();
        validate_channel_id_list(&channel_ids, false)?;
        let mut old_mps = Vec::new();
        for &channel_id in &channel_ids {
            let index = self
                .channel_index_for_owner(direction, channel_id)
                .ok_or_else(|| {
                    Error::InvalidState(format!(
                        "no {} endpoint uses reconfiguration CID 0x{channel_id:04x}",
                        endpoint_owner_name(direction)
                    ))
                })?;
            let channel = &self.channels[index].public;
            if channel.mode != L2capCreditBasedMode::EnhancedCreditBased {
                return Err(Error::InvalidInput(format!(
                    "CID 0x{channel_id:04x} is not an Enhanced Credit Based channel"
                )));
            }
            let endpoint = channel.owner(direction);
            if request.mtu < endpoint.mtu {
                return Err(Error::InvalidInput(format!(
                    "reconfiguration cannot decrease CID 0x{channel_id:04x} MTU from {} to {}",
                    endpoint.mtu, request.mtu
                )));
            }
            old_mps.push(endpoint.mps);
        }
        if channel_ids.len() > 1 && old_mps.iter().any(|mps| request.mps < *mps) {
            return Err(Error::InvalidInput(
                "reconfiguration cannot decrease MPS when multiple channels are specified"
                    .to_owned(),
            ));
        }
        let pending = PendingProcedure::Reconfigure {
            direction,
            identifier,
            mtu: request.mtu,
            mps: request.mps,
            channel_ids: channel_ids.clone(),
        };
        if let Some(existing) = self.pending_for_request(direction, identifier) {
            if existing == &pending {
                return Ok(L2capCreditBasedEvent::ReconfigurePending {
                    identifier,
                    owner: direction,
                    channel_ids,
                    mtu: request.mtu,
                    mps: request.mps,
                });
            }
            return Err(Error::InvalidState(format!(
                "{} signaling identifier {identifier} already has a pending procedure",
                endpoint_owner_name(direction)
            )));
        }
        for &channel_id in &channel_ids {
            let channel_index = self
                .channel_index_for_owner(direction, channel_id)
                .expect("validated reconfiguration channel remains present");
            if self.channels[channel_index].public.status != L2capCreditBasedChannelStatus::Open
                || self.pending_targets_channel(channel_index)
            {
                return Err(Error::InvalidState(format!(
                    "CID 0x{channel_id:04x} already has a pending channel procedure"
                )));
            }
        }
        self.pending.push(pending);
        Ok(L2capCreditBasedEvent::ReconfigurePending {
            identifier,
            owner: direction,
            channel_ids,
            mtu: request.mtu,
            mps: request.mps,
        })
    }

    fn reconfigure_response(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        result: u16,
    ) -> Result<L2capCreditBasedEvent> {
        let pending_index = self.pending_response_index(direction, identifier, |pending| {
            matches!(pending, PendingProcedure::Reconfigure { .. })
        })?;
        let PendingProcedure::Reconfigure {
            direction: owner,
            mtu,
            mps,
            channel_ids,
            ..
        } = self.pending[pending_index].clone()
        else {
            unreachable!()
        };
        self.pending.remove(pending_index);
        if result != 0 {
            return Ok(L2capCreditBasedEvent::ReconfigureRejected { identifier, result });
        }
        for &channel_id in &channel_ids {
            let channel_index = self
                .channel_index_for_owner(owner, channel_id)
                .expect("validated reconfiguration channel remains present");
            let endpoint = self.channels[channel_index].endpoint_mut(owner);
            endpoint.mtu = mtu;
            endpoint.mps = mps;
        }
        Ok(L2capCreditBasedEvent::Reconfigured {
            owner,
            channel_ids,
            mtu,
            mps,
        })
    }

    fn disconnect_request(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        disconnection: L2capDisconnection,
    ) -> Result<L2capCreditBasedEvent> {
        let channel_index = self
            .channel_index_for_pair(direction, disconnection)
            .ok_or_else(|| {
                Error::InvalidState(format!(
                    "disconnection pair dcid=0x{:04x} scid=0x{:04x} is not open",
                    disconnection.destination_channel_id, disconnection.source_channel_id
                ))
            })?;
        let pending = PendingProcedure::Disconnect {
            direction,
            identifier,
            channel_index,
            disconnection,
        };
        if let Some(existing) = self.pending_for_request(direction, identifier) {
            if existing == &pending {
                let channel = &self.channels[channel_index].public;
                return Ok(L2capCreditBasedEvent::DisconnectPending {
                    identifier,
                    central_channel_id: channel.central.channel_id,
                    peripheral_channel_id: channel.peripheral.channel_id,
                });
            }
            return Err(Error::InvalidState(format!(
                "{} signaling identifier {identifier} already has a pending procedure",
                endpoint_owner_name(direction)
            )));
        }
        if self.pending_targets_channel(channel_index) {
            return Err(Error::InvalidState(format!(
                "channel central=0x{:04x} peripheral=0x{:04x} already has a pending procedure",
                self.channels[channel_index].public.central.channel_id,
                self.channels[channel_index].public.peripheral.channel_id
            )));
        }
        let channel = &mut self.channels[channel_index].public;
        if channel.status != L2capCreditBasedChannelStatus::Open {
            return Err(Error::InvalidState(format!(
                "channel central=0x{:04x} peripheral=0x{:04x} is already {}",
                channel.central.channel_id, channel.peripheral.channel_id, channel.status
            )));
        }
        channel.status = L2capCreditBasedChannelStatus::Disconnecting;
        let event = L2capCreditBasedEvent::DisconnectPending {
            identifier,
            central_channel_id: channel.central.channel_id,
            peripheral_channel_id: channel.peripheral.channel_id,
        };
        self.pending.push(pending);
        Ok(event)
    }

    fn disconnect_response(
        &mut self,
        direction: LinkDirection,
        identifier: u8,
        disconnection: L2capDisconnection,
    ) -> Result<L2capCreditBasedEvent> {
        let pending_index = self.pending_response_index(direction, identifier, |pending| {
            matches!(pending, PendingProcedure::Disconnect { .. })
        })?;
        let PendingProcedure::Disconnect {
            channel_index,
            disconnection: request,
            ..
        } = self.pending[pending_index]
        else {
            unreachable!()
        };
        if request != disconnection {
            return Err(Error::InvalidInput(format!(
                "disconnection response dcid=0x{:04x} scid=0x{:04x} does not match the request",
                disconnection.destination_channel_id, disconnection.source_channel_id
            )));
        }
        self.pending.remove(pending_index);
        self.adjust_pending_channel_indices_after_removal(channel_index);
        let channel = self.channels.remove(channel_index).public;
        Ok(L2capCreditBasedEvent::Disconnected(channel))
    }

    fn command_rejected(
        &mut self,
        response_direction: LinkDirection,
        identifier: u8,
    ) -> L2capCreditBasedEvent {
        let pending_index = self.pending.iter().position(|pending| {
            pending.identifier() == identifier && pending.direction() != response_direction
        });
        let Some(pending_index) = pending_index else {
            return L2capCreditBasedEvent::CommandRejected {
                identifier,
                removed_pending_procedure: false,
            };
        };
        if let PendingProcedure::Disconnect { channel_index, .. } = self.pending[pending_index]
            && let Some(channel) = self.channels.get_mut(channel_index)
        {
            channel.public.status = L2capCreditBasedChannelStatus::Open;
        }
        self.pending.remove(pending_index);
        L2capCreditBasedEvent::CommandRejected {
            identifier,
            removed_pending_procedure: true,
        }
    }

    fn pending_for_request(
        &self,
        direction: LinkDirection,
        identifier: u8,
    ) -> Option<&PendingProcedure> {
        self.pending
            .iter()
            .find(|pending| pending.direction() == direction && pending.identifier() == identifier)
    }

    fn pending_connection_index(
        &self,
        response_direction: LinkDirection,
        identifier: u8,
        mode: L2capCreditBasedMode,
    ) -> Result<usize> {
        self.pending_response_index(response_direction, identifier, |pending| {
            matches!(
                pending,
                PendingProcedure::Connection {
                    mode: pending_mode,
                    ..
                } if *pending_mode == mode
            )
        })
    }

    fn pending_response_index<F>(
        &self,
        response_direction: LinkDirection,
        identifier: u8,
        expected: F,
    ) -> Result<usize>
    where
        F: Fn(&PendingProcedure) -> bool,
    {
        self.pending
            .iter()
            .position(|pending| {
                pending.direction() != response_direction
                    && pending.identifier() == identifier
                    && expected(pending)
            })
            .ok_or_else(|| {
                Error::InvalidState(format!(
                    "no matching request precedes {} signaling response identifier {identifier}",
                    endpoint_owner_name(response_direction)
                ))
            })
    }

    fn owner_channel_id_in_use(&self, owner: LinkDirection, channel_id: u16) -> bool {
        self.channel_index_for_owner(owner, channel_id).is_some()
            || self.pending.iter().any(|pending| match pending {
                PendingProcedure::Connection {
                    direction,
                    source_channel_ids,
                    ..
                } => *direction == owner && source_channel_ids.contains(&channel_id),
                PendingProcedure::Reconfigure { .. } | PendingProcedure::Disconnect { .. } => false,
            })
    }

    fn channel_index_for_owner(&self, owner: LinkDirection, channel_id: u16) -> Option<usize> {
        self.channels
            .iter()
            .position(|channel| channel.public.owner(owner).channel_id == channel_id)
    }

    fn channel_index_for_data(&self, direction: LinkDirection, channel_id: u16) -> Option<usize> {
        self.channels
            .iter()
            .position(|channel| channel.public.destination(direction).channel_id == channel_id)
    }

    fn channel_index_for_pair(
        &self,
        direction: LinkDirection,
        disconnection: L2capDisconnection,
    ) -> Option<usize> {
        self.channels.iter().position(|channel| {
            channel.public.owner(direction).channel_id == disconnection.source_channel_id
                && channel.public.destination(direction).channel_id
                    == disconnection.destination_channel_id
        })
    }

    fn adjust_pending_channel_indices_after_removal(&mut self, removed: usize) {
        for pending in &mut self.pending {
            if let PendingProcedure::Disconnect { channel_index, .. } = pending
                && *channel_index > removed
            {
                *channel_index -= 1;
            }
        }
    }

    fn pending_targets_channel(&self, channel_index: usize) -> bool {
        let Some(channel) = self.channels.get(channel_index) else {
            return false;
        };
        self.pending.iter().any(|pending| match pending {
            PendingProcedure::Connection { .. } => false,
            PendingProcedure::Reconfigure {
                direction,
                channel_ids,
                ..
            } => channel_ids.contains(&channel.public.owner(*direction).channel_id),
            PendingProcedure::Disconnect {
                channel_index: pending_index,
                ..
            } => *pending_index == channel_index,
        })
    }
}

fn is_dynamic_channel_id(channel_id: u16) -> bool {
    (L2CAP_LE_DYNAMIC_CID_START..=L2CAP_LE_DYNAMIC_CID_END).contains(&channel_id)
}

fn validate_spsm(spsm: u16) -> Result<()> {
    if !(L2CAP_LE_SPSM_START..=L2CAP_LE_SPSM_END).contains(&spsm) {
        return Err(Error::InvalidInput(format!(
            "LE SPSM 0x{spsm:04x} is outside 0x0001..=0x00ff"
        )));
    }
    Ok(())
}

fn validate_endpoint_parameters(
    mode: L2capCreditBasedMode,
    mtu: u16,
    mps: u16,
    credits: u16,
) -> Result<()> {
    let (minimum_mtu, minimum_mps) = match mode {
        L2capCreditBasedMode::LeCreditBased => (
            L2CAP_LE_CREDIT_BASED_MINIMUM_MTU,
            L2CAP_LE_CREDIT_BASED_MINIMUM_MPS,
        ),
        L2capCreditBasedMode::EnhancedCreditBased => (
            L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MTU,
            L2CAP_ENHANCED_CREDIT_BASED_MINIMUM_MPS,
        ),
    };
    if mtu < minimum_mtu {
        return Err(Error::InvalidInput(format!(
            "{mode} MTU {mtu} is below {minimum_mtu}"
        )));
    }
    if !(minimum_mps..=L2CAP_CREDIT_BASED_MAXIMUM_MPS).contains(&mps) {
        return Err(Error::InvalidInput(format!(
            "{mode} MPS {mps} is outside {minimum_mps}..={L2CAP_CREDIT_BASED_MAXIMUM_MPS}"
        )));
    }
    if mode == L2capCreditBasedMode::EnhancedCreditBased && credits == 0 {
        return Err(Error::InvalidInput(
            "enhanced-credit-based initial credits must be nonzero".to_owned(),
        ));
    }
    Ok(())
}

fn validate_channel_id_list(channel_ids: &[u16], allow_zero: bool) -> Result<()> {
    for (index, &channel_id) in channel_ids.iter().enumerate() {
        if channel_id == 0 && allow_zero {
            continue;
        }
        if !is_dynamic_channel_id(channel_id) {
            return Err(Error::InvalidInput(format!(
                "dynamic channel ID 0x{channel_id:04x} at index {index} is outside 0x0040..=0x007f"
            )));
        }
        if channel_ids[..index].contains(&channel_id) {
            return Err(Error::InvalidInput(format!(
                "dynamic channel ID 0x{channel_id:04x} is repeated"
            )));
        }
    }
    Ok(())
}

fn build_channel(
    mode: L2capCreditBasedMode,
    spsm: u16,
    request_direction: LinkDirection,
    initiator: L2capCreditBasedEndpoint,
    responder: L2capCreditBasedEndpoint,
) -> L2capCreditBasedChannel {
    let (central, peripheral) = match request_direction {
        LinkDirection::CentralToPeripheral => (initiator, responder),
        LinkDirection::PeripheralToCentral => (responder, initiator),
    };
    L2capCreditBasedChannel {
        mode,
        spsm,
        central,
        peripheral,
        status: L2capCreditBasedChannelStatus::Open,
    }
}

fn endpoint_owner_name(direction: LinkDirection) -> &'static str {
    match direction {
        LinkDirection::CentralToPeripheral => "central",
        LinkDirection::PeripheralToCentral => "peripheral",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdu(direction: LinkDirection, channel_id: u16, payload: &[u8]) -> L2capPdu {
        L2capPdu {
            direction,
            channel_id,
            payload: payload.to_vec(),
            fragment_count: 1,
        }
    }

    fn signaling(direction: LinkDirection, payload: &[u8]) -> L2capPdu {
        pdu(direction, LE_SIGNALING_CHANNEL_ID, payload)
    }

    fn open_two_eatt_channels() -> L2capCreditBasedChannelTracker {
        let mut tracker = L2capCreditBasedChannelTracker::default();
        let request = signaling(
            LinkDirection::CentralToPeripheral,
            &[
                0x17, 0x01, 0x0c, 0x00, 0x27, 0x00, 0x40, 0x00, 0x40, 0x00, 0x03, 0x00, 0x40, 0x00,
                0x41, 0x00,
            ],
        );
        tracker.observe(&request).unwrap();
        assert!(matches!(
            tracker.observe(&request).unwrap(),
            L2capCreditBasedEvent::ConnectionRequestPending {
                channel_count: 2,
                ..
            }
        ));
        let opened = tracker
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[
                    0x18, 0x01, 0x0c, 0x00, 0x40, 0x00, 0x40, 0x00, 0x02, 0x00, 0x00, 0x00, 0x42,
                    0x00, 0x43, 0x00,
                ],
            ))
            .unwrap();
        let L2capCreditBasedEvent::ChannelsOpened(channels) = opened else {
            panic!("unexpected event");
        };
        assert_eq!(channels.len(), 2);
        tracker
    }

    #[test]
    fn opens_enhanced_channels_and_reassembles_bidirectional_eatt() {
        let mut tracker = open_two_eatt_channels();
        let channels: Vec<_> = tracker.channels().cloned().collect();
        assert_eq!(channels[0].central.channel_id, 0x0040);
        assert_eq!(channels[0].peripheral.channel_id, 0x0042);
        assert!(channels[0].is_eatt());

        let complete = tracker
            .observe(&pdu(
                LinkDirection::CentralToPeripheral,
                0x0042,
                &[0x03, 0x00, 0x0a, 0x01, 0x00],
            ))
            .unwrap();
        let L2capCreditBasedEvent::SduComplete(sdu) = complete else {
            panic!("unexpected event");
        };
        assert_eq!(sdu.payload, [0x0a, 0x01, 0x00]);
        assert_eq!(sdu.att_pdu().unwrap().unwrap().opcode, 0x0a);
        assert_eq!(
            tracker
                .channel_for_data(LinkDirection::CentralToPeripheral, 0x0042)
                .unwrap()
                .peripheral
                .credits,
            1
        );

        let complete = tracker
            .observe(&pdu(
                LinkDirection::PeripheralToCentral,
                0x0040,
                &[0x01, 0x00, 0x13],
            ))
            .unwrap();
        let L2capCreditBasedEvent::SduComplete(sdu) = complete else {
            panic!("unexpected event");
        };
        assert_eq!(sdu.payload, [0x13]);
        assert_eq!(
            tracker
                .channel_for_data(LinkDirection::PeripheralToCentral, 0x0040)
                .unwrap()
                .central
                .credits,
            2
        );
    }

    #[test]
    fn tracks_segmentation_credit_exhaustion_and_credit_addition_transactionally() {
        let mut tracker = open_two_eatt_channels();
        let first = tracker
            .observe(&pdu(
                LinkDirection::CentralToPeripheral,
                0x0042,
                &[0x05, 0x00, 0x1b, 0x01],
            ))
            .unwrap();
        assert!(matches!(
            first,
            L2capCreditBasedEvent::SduInProgress(IncompleteL2capCreditBasedSdu {
                expected_octets: 5,
                received_octets: 2,
                segment_count: 1,
                ..
            })
        ));
        let complete = tracker
            .observe(&pdu(
                LinkDirection::CentralToPeripheral,
                0x0042,
                &[0x00, 0xaa, 0xbb],
            ))
            .unwrap();
        assert!(matches!(
            complete,
            L2capCreditBasedEvent::SduComplete(L2capCreditBasedSdu {
                segment_count: 2,
                ..
            })
        ));
        let error = tracker
            .observe(&pdu(
                LinkDirection::CentralToPeripheral,
                0x0042,
                &[0x01, 0x00, 0x13],
            ))
            .unwrap_err();
        assert!(error.to_string().contains("no remaining credits"));
        assert_eq!(
            tracker
                .channel_for_data(LinkDirection::CentralToPeripheral, 0x0042)
                .unwrap()
                .peripheral
                .credits,
            0
        );

        let added = tracker
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[0x16, 0x02, 0x04, 0x00, 0x42, 0x00, 0x04, 0x00],
            ))
            .unwrap();
        assert!(matches!(
            added,
            L2capCreditBasedEvent::CreditsAdded {
                owner: LinkDirection::PeripheralToCentral,
                channel_id: 0x0042,
                total: 4,
                ..
            }
        ));
    }

    #[test]
    fn applies_successful_reconfiguration_and_disconnects_exact_channel_pair() {
        let mut tracker = open_two_eatt_channels();
        tracker
            .observe(&signaling(
                LinkDirection::CentralToPeripheral,
                &[
                    0x19, 0x03, 0x08, 0x00, 0x80, 0x00, 0x50, 0x00, 0x40, 0x00, 0x41, 0x00,
                ],
            ))
            .unwrap();
        assert!(
            tracker
                .observe(&signaling(
                    LinkDirection::CentralToPeripheral,
                    &[0x06, 0x04, 0x04, 0x00, 0x42, 0x00, 0x40, 0x00],
                ))
                .is_err()
        );
        tracker
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[0x1a, 0x03, 0x02, 0x00, 0x00, 0x00],
            ))
            .unwrap();
        let channel = tracker
            .channel_for_data(LinkDirection::PeripheralToCentral, 0x0040)
            .unwrap();
        assert_eq!(channel.central.mtu, 128);
        assert_eq!(channel.central.mps, 80);

        let pending = tracker
            .observe(&signaling(
                LinkDirection::CentralToPeripheral,
                &[0x06, 0x04, 0x04, 0x00, 0x42, 0x00, 0x40, 0x00],
            ))
            .unwrap();
        assert!(matches!(
            pending,
            L2capCreditBasedEvent::DisconnectPending {
                central_channel_id: 0x0040,
                peripheral_channel_id: 0x0042,
                ..
            }
        ));
        assert!(
            tracker
                .observe(&pdu(
                    LinkDirection::CentralToPeripheral,
                    0x0042,
                    &[0x01, 0x00, 0x13],
                ))
                .is_err()
        );
        let disconnected = tracker
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[0x07, 0x04, 0x04, 0x00, 0x42, 0x00, 0x40, 0x00],
            ))
            .unwrap();
        assert!(matches!(
            disconnected,
            L2capCreditBasedEvent::Disconnected(L2capCreditBasedChannel {
                spsm: EATT_SPSM,
                ..
            })
        ));
        assert_eq!(tracker.channels().count(), 1);
    }

    #[test]
    fn tracks_legacy_channel_and_rejected_or_partial_enhanced_responses() {
        let mut tracker = L2capCreditBasedChannelTracker::default();
        tracker
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[
                    0x14, 0x09, 0x0a, 0x00, 0x80, 0x00, 0x44, 0x00, 0x17, 0x00, 0x17, 0x00, 0x00,
                    0x00,
                ],
            ))
            .unwrap();
        tracker
            .observe(&signaling(
                LinkDirection::CentralToPeripheral,
                &[
                    0x15, 0x09, 0x0a, 0x00, 0x45, 0x00, 0x40, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00,
                    0x00,
                ],
            ))
            .unwrap();
        let channel = tracker
            .channel_for_data(LinkDirection::CentralToPeripheral, 0x0044)
            .unwrap();
        assert_eq!(channel.central.channel_id, 0x0045);
        assert_eq!(channel.peripheral.channel_id, 0x0044);
        assert_eq!(channel.mode, L2capCreditBasedMode::LeCreditBased);

        let mut partial = L2capCreditBasedChannelTracker::default();
        partial
            .observe(&signaling(
                LinkDirection::CentralToPeripheral,
                &[
                    0x17, 0x01, 0x0c, 0x00, 0x27, 0x00, 0x40, 0x00, 0x40, 0x00, 0x03, 0x00, 0x40,
                    0x00, 0x41, 0x00,
                ],
            ))
            .unwrap();
        let opened = partial
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[
                    0x18, 0x01, 0x0c, 0x00, 0x40, 0x00, 0x40, 0x00, 0x02, 0x00, 0x04, 0x00, 0x42,
                    0x00, 0x00, 0x00,
                ],
            ))
            .unwrap();
        assert!(matches!(
            opened,
            L2capCreditBasedEvent::ChannelsOpened(ref channels) if channels.len() == 1
        ));

        let mut refused = L2capCreditBasedChannelTracker::default();
        refused
            .observe(&signaling(
                LinkDirection::CentralToPeripheral,
                &[
                    0x17, 0x01, 0x0c, 0x00, 0x27, 0x00, 0x40, 0x00, 0x40, 0x00, 0x03, 0x00, 0x40,
                    0x00, 0x41, 0x00,
                ],
            ))
            .unwrap();
        let rejected = refused
            .observe(&signaling(
                LinkDirection::PeripheralToCentral,
                &[
                    0x18, 0x01, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
            ))
            .unwrap();
        assert!(matches!(
            rejected,
            L2capCreditBasedEvent::ConnectionRejected { result: 4, .. }
        ));
    }

    #[test]
    fn invalid_observations_preserve_channel_credit_and_pending_sdu_state() {
        let mut tracker = open_two_eatt_channels();
        tracker
            .observe(&pdu(
                LinkDirection::CentralToPeripheral,
                0x0042,
                &[0x05, 0x00, 0x1b, 0x01],
            ))
            .unwrap();
        let before = tracker.clone();
        assert!(
            tracker
                .observe(&pdu(
                    LinkDirection::CentralToPeripheral,
                    0x0042,
                    &[0x00, 0xaa, 0xbb, 0xcc],
                ))
                .is_err()
        );
        assert_eq!(tracker, before);
        assert!(
            tracker
                .observe(&signaling(
                    LinkDirection::PeripheralToCentral,
                    &[0x16, 0x02, 0x04, 0x00, 0x42, 0x00, 0x00, 0x00],
                ))
                .is_err()
        );
        assert_eq!(tracker, before);
    }

    #[test]
    fn bounded_arbitrary_l2cap_input_never_panics() {
        for direction in [
            LinkDirection::CentralToPeripheral,
            LinkDirection::PeripheralToCentral,
        ] {
            for channel_id in [0x0004, 0x0005, 0x003f, 0x0040, 0x007f, 0x0080] {
                for length in 0..=24usize {
                    let payload = vec![(length as u8).wrapping_mul(17); length];
                    let mut tracker = L2capCreditBasedChannelTracker::default();
                    let _ = tracker.observe(&pdu(direction, channel_id, &payload));
                }
            }
        }
    }
}
