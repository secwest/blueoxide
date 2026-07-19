use crate::link_layer::L2capPdu;
use crate::{Error, Result};
use std::fmt::{Display, Formatter};

pub const ATT_FIXED_CHANNEL_ID: u16 = 0x0004;
pub const ATT_DEFAULT_LE_MTU: u16 = 23;
pub const ATT_SIGNATURE_LENGTH: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttPdu<'a> {
    pub opcode: u8,
    pub parameters: &'a [u8],
}

impl<'a> AttPdu<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let Some((&opcode, parameters)) = payload.split_first() else {
            return Err(Error::InvalidInput(
                "ATT PDU is missing its opcode".to_owned(),
            ));
        };
        Ok(Self { opcode, parameters })
    }

    pub const fn opcode_name(self) -> &'static str {
        match self.opcode {
            0x01 => "error-response",
            0x02 => "exchange-mtu-request",
            0x03 => "exchange-mtu-response",
            0x04 => "find-information-request",
            0x05 => "find-information-response",
            0x06 => "find-by-type-value-request",
            0x07 => "find-by-type-value-response",
            0x08 => "read-by-type-request",
            0x09 => "read-by-type-response",
            0x0a => "read-request",
            0x0b => "read-response",
            0x0c => "read-blob-request",
            0x0d => "read-blob-response",
            0x0e => "read-multiple-request",
            0x0f => "read-multiple-response",
            0x10 => "read-by-group-type-request",
            0x11 => "read-by-group-type-response",
            0x12 => "write-request",
            0x13 => "write-response",
            0x16 => "prepare-write-request",
            0x17 => "prepare-write-response",
            0x18 => "execute-write-request",
            0x19 => "execute-write-response",
            0x1b => "handle-value-notification",
            0x1d => "handle-value-indication",
            0x1e => "handle-value-confirmation",
            0x20 => "read-multiple-variable-request",
            0x21 => "read-multiple-variable-response",
            0x23 => "multiple-handle-value-notification",
            0x52 => "write-command",
            0xd2 => "signed-write-command",
            _ => "unknown",
        }
    }

    pub const fn pdu_type(self) -> AttPduType {
        match self.opcode {
            0x02 | 0x04 | 0x06 | 0x08 | 0x0a | 0x0c | 0x0e | 0x10 | 0x12 | 0x16 | 0x18 | 0x20 => {
                AttPduType::Request
            }
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f | 0x11 | 0x13 | 0x17 | 0x19
            | 0x21 => AttPduType::Response,
            0x1b | 0x23 => AttPduType::Notification,
            0x1d => AttPduType::Indication,
            0x1e => AttPduType::Confirmation,
            0x52 | 0xd2 => AttPduType::Command,
            opcode if opcode & 0x40 != 0 => AttPduType::Command,
            _ => AttPduType::Unknown,
        }
    }

    pub fn decode(self) -> Result<DecodedAttPdu<'a>> {
        match self.opcode {
            0x01 => {
                require_att_parameter_length(self, 4)?;
                let error_code = self.parameters[3];
                if error_code == 0 {
                    return Err(Error::InvalidInput(
                        "ATT Error Response error code must be nonzero".to_owned(),
                    ));
                }
                Ok(DecodedAttPdu::ErrorResponse(AttErrorResponse {
                    request_opcode: self.parameters[0],
                    handle: att_parameter_u16(self, 1, "handle")?,
                    error_code,
                }))
            }
            0x02 | 0x03 => {
                require_att_parameter_length(self, 2)?;
                let mtu = att_parameter_u16(self, 0, "MTU")?;
                if mtu < ATT_DEFAULT_LE_MTU {
                    return Err(Error::InvalidInput(format!(
                        "ATT MTU {mtu} is below the default MTU of {ATT_DEFAULT_LE_MTU}"
                    )));
                }
                let exchange = AttExchangeMtu { mtu };
                if self.opcode == 0x02 {
                    Ok(DecodedAttPdu::ExchangeMtuRequest(exchange))
                } else {
                    Ok(DecodedAttPdu::ExchangeMtuResponse(exchange))
                }
            }
            0x04 => {
                require_att_parameter_length(self, 4)?;
                Ok(DecodedAttPdu::FindInformationRequest(
                    decode_att_handle_range(self, 0)?,
                ))
            }
            0x05 => Ok(DecodedAttPdu::FindInformationResponse(
                decode_att_find_information_response(self)?,
            )),
            0x06 => {
                require_minimum_att_parameter_length(self, 6)?;
                Ok(DecodedAttPdu::FindByTypeValueRequest(
                    AttFindByTypeValueRequest {
                        range: decode_att_handle_range(self, 0)?,
                        attribute_type: att_parameter_u16(self, 4, "attribute type")?,
                        value: &self.parameters[6..],
                    },
                ))
            }
            0x07 => Ok(DecodedAttPdu::FindByTypeValueResponse(
                decode_att_handle_ranges(self)?,
            )),
            0x08 | 0x10 => {
                require_minimum_att_parameter_length(self, 6)?;
                let request = AttReadByTypeRequest {
                    range: decode_att_handle_range(self, 0)?,
                    attribute_type: decode_att_uuid(&self.parameters[4..])?,
                };
                if self.opcode == 0x08 {
                    Ok(DecodedAttPdu::ReadByTypeRequest(request))
                } else {
                    Ok(DecodedAttPdu::ReadByGroupTypeRequest(request))
                }
            }
            0x09 => Ok(DecodedAttPdu::ReadByTypeResponse(
                decode_att_attribute_data_response(self)?,
            )),
            0x0a => {
                require_att_parameter_length(self, 2)?;
                Ok(DecodedAttPdu::ReadRequest(AttHandle {
                    handle: decode_att_handle(self, 0, "handle")?,
                }))
            }
            0x0b => Ok(DecodedAttPdu::ReadResponse(self.parameters)),
            0x0c => {
                require_att_parameter_length(self, 4)?;
                Ok(DecodedAttPdu::ReadBlobRequest(AttReadBlobRequest {
                    handle: decode_att_handle(self, 0, "handle")?,
                    offset: att_parameter_u16(self, 2, "offset")?,
                }))
            }
            0x0d => Ok(DecodedAttPdu::ReadBlobResponse(self.parameters)),
            0x0e | 0x20 => {
                let handles = decode_att_handle_list(self, 2)?;
                if self.opcode == 0x0e {
                    Ok(DecodedAttPdu::ReadMultipleRequest(handles))
                } else {
                    Ok(DecodedAttPdu::ReadMultipleVariableRequest(handles))
                }
            }
            0x0f => Ok(DecodedAttPdu::ReadMultipleResponse(self.parameters)),
            0x11 => Ok(DecodedAttPdu::ReadByGroupTypeResponse(
                decode_att_group_data_response(self)?,
            )),
            0x12 | 0x1b | 0x1d | 0x52 => {
                let value = decode_att_handle_value(self)?;
                match self.opcode {
                    0x12 => Ok(DecodedAttPdu::WriteRequest(value)),
                    0x1b => Ok(DecodedAttPdu::HandleValueNotification(value)),
                    0x1d => Ok(DecodedAttPdu::HandleValueIndication(value)),
                    0x52 => Ok(DecodedAttPdu::WriteCommand(value)),
                    _ => unreachable!(),
                }
            }
            0x13 | 0x19 | 0x1e => {
                require_att_parameter_length(self, 0)?;
                match self.opcode {
                    0x13 => Ok(DecodedAttPdu::WriteResponse),
                    0x19 => Ok(DecodedAttPdu::ExecuteWriteResponse),
                    0x1e => Ok(DecodedAttPdu::HandleValueConfirmation),
                    _ => unreachable!(),
                }
            }
            0x16 | 0x17 => {
                let write = decode_att_prepare_write(self)?;
                if self.opcode == 0x16 {
                    Ok(DecodedAttPdu::PrepareWriteRequest(write))
                } else {
                    Ok(DecodedAttPdu::PrepareWriteResponse(write))
                }
            }
            0x18 => {
                require_att_parameter_length(self, 1)?;
                let flags = match self.parameters[0] {
                    0 => AttExecuteWriteFlags::Cancel,
                    1 => AttExecuteWriteFlags::Execute,
                    flags => {
                        return Err(Error::InvalidInput(format!(
                            "ATT Execute Write Request flags 0x{flags:02x} are invalid"
                        )));
                    }
                };
                Ok(DecodedAttPdu::ExecuteWriteRequest(flags))
            }
            0x21 => Ok(DecodedAttPdu::ReadMultipleVariableResponse(
                decode_att_length_value_list(self)?,
            )),
            0x23 => Ok(DecodedAttPdu::MultipleHandleValueNotification(
                decode_att_multiple_handle_values(self)?,
            )),
            0xd2 => Ok(DecodedAttPdu::SignedWriteCommand(decode_att_signed_write(
                self,
            )?)),
            _ => Ok(DecodedAttPdu::Unknown {
                opcode: self.opcode,
                parameters: self.parameters,
            }),
        }
    }
}

impl L2capPdu {
    pub fn att_pdu(&self) -> Result<Option<AttPdu<'_>>> {
        if self.channel_id != ATT_FIXED_CHANNEL_ID {
            return Ok(None);
        }
        Ok(Some(AttPdu::parse(&self.payload)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttPduType {
    Request,
    Response,
    Command,
    Notification,
    Indication,
    Confirmation,
    Unknown,
}

impl Display for AttPduType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request => formatter.write_str("request"),
            Self::Response => formatter.write_str("response"),
            Self::Command => formatter.write_str("command"),
            Self::Notification => formatter.write_str("notification"),
            Self::Indication => formatter.write_str("indication"),
            Self::Confirmation => formatter.write_str("confirmation"),
            Self::Unknown => formatter.write_str("unknown"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedAttPdu<'a> {
    ErrorResponse(AttErrorResponse),
    ExchangeMtuRequest(AttExchangeMtu),
    ExchangeMtuResponse(AttExchangeMtu),
    FindInformationRequest(AttHandleRange),
    FindInformationResponse(AttFindInformationResponse),
    FindByTypeValueRequest(AttFindByTypeValueRequest<'a>),
    FindByTypeValueResponse(Vec<AttHandleRange>),
    ReadByTypeRequest(AttReadByTypeRequest),
    ReadByTypeResponse(AttAttributeDataResponse<'a>),
    ReadRequest(AttHandle),
    ReadResponse(&'a [u8]),
    ReadBlobRequest(AttReadBlobRequest),
    ReadBlobResponse(&'a [u8]),
    ReadMultipleRequest(Vec<u16>),
    ReadMultipleResponse(&'a [u8]),
    ReadByGroupTypeRequest(AttReadByTypeRequest),
    ReadByGroupTypeResponse(AttGroupDataResponse<'a>),
    WriteRequest(AttHandleValue<'a>),
    WriteResponse,
    PrepareWriteRequest(AttPrepareWrite<'a>),
    PrepareWriteResponse(AttPrepareWrite<'a>),
    ExecuteWriteRequest(AttExecuteWriteFlags),
    ExecuteWriteResponse,
    HandleValueNotification(AttHandleValue<'a>),
    HandleValueIndication(AttHandleValue<'a>),
    HandleValueConfirmation,
    ReadMultipleVariableRequest(Vec<u16>),
    ReadMultipleVariableResponse(AttLengthValueList<'a>),
    MultipleHandleValueNotification(Vec<AttHandleValue<'a>>),
    WriteCommand(AttHandleValue<'a>),
    SignedWriteCommand(AttSignedWrite<'a>),
    Unknown { opcode: u8, parameters: &'a [u8] },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttErrorResponse {
    pub request_opcode: u8,
    pub handle: u16,
    pub error_code: u8,
}

impl AttErrorResponse {
    pub const fn error_name(self) -> &'static str {
        att_error_name(self.error_code)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttExchangeMtu {
    pub mtu: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttHandle {
    pub handle: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttHandleRange {
    pub start_handle: u16,
    pub end_handle: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttUuid {
    Uuid16(u16),
    Uuid128([u8; 16]),
}

impl AttUuid {
    pub const fn width(self) -> usize {
        match self {
            Self::Uuid16(_) => 2,
            Self::Uuid128(_) => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttFindInformationEntry {
    pub handle: u16,
    pub uuid: AttUuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttFindInformationResponse {
    pub entries: Vec<AttFindInformationEntry>,
}

impl AttFindInformationResponse {
    pub fn uuid_width(&self) -> usize {
        self.entries
            .first()
            .map(|entry| entry.uuid.width())
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttFindByTypeValueRequest<'a> {
    pub range: AttHandleRange,
    pub attribute_type: u16,
    pub value: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttReadByTypeRequest {
    pub range: AttHandleRange,
    pub attribute_type: AttUuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttHandleValue<'a> {
    pub handle: u16,
    pub value: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttAttributeDataResponse<'a> {
    pub entry_length: u8,
    pub entries: Vec<AttHandleValue<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttReadBlobRequest {
    pub handle: u16,
    pub offset: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttGroupValue<'a> {
    pub range: AttHandleRange,
    pub value: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttGroupDataResponse<'a> {
    pub entry_length: u8,
    pub entries: Vec<AttGroupValue<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttPrepareWrite<'a> {
    pub handle: u16,
    pub offset: u16,
    pub value: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttExecuteWriteFlags {
    Cancel,
    Execute,
}

impl Display for AttExecuteWriteFlags {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancel => formatter.write_str("cancel"),
            Self::Execute => formatter.write_str("execute"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttLengthValueList<'a> {
    pub values: Vec<AttLengthValue<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttLengthValue<'a> {
    pub declared_length: u16,
    pub value: &'a [u8],
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttSignedWrite<'a> {
    pub handle: u16,
    pub value: &'a [u8],
    pub signature: [u8; ATT_SIGNATURE_LENGTH],
}

pub const fn att_error_name(error_code: u8) -> &'static str {
    match error_code {
        0x01 => "invalid-handle",
        0x02 => "read-not-permitted",
        0x03 => "write-not-permitted",
        0x04 => "invalid-pdu",
        0x05 => "insufficient-authentication",
        0x06 => "request-not-supported",
        0x07 => "invalid-offset",
        0x08 => "insufficient-authorization",
        0x09 => "prepare-queue-full",
        0x0a => "attribute-not-found",
        0x0b => "attribute-not-long",
        0x0c => "insufficient-encryption-key-size",
        0x0d => "invalid-attribute-value-length",
        0x0e => "unlikely-error",
        0x0f => "insufficient-encryption",
        0x10 => "unsupported-group-type",
        0x11 => "insufficient-resources",
        0x12 => "database-out-of-sync",
        0x13 => "value-not-allowed",
        0xfc => "write-request-rejected",
        0xfd => "ccc-improperly-configured",
        0xfe => "procedure-already-in-progress",
        0xff => "out-of-range",
        _ => "unknown",
    }
}

fn require_att_parameter_length(pdu: AttPdu<'_>, expected: usize) -> Result<()> {
    if pdu.parameters.len() != expected {
        return Err(Error::InvalidInput(format!(
            "ATT {} (0x{:02x}) requires {expected} parameter octets, received {}",
            pdu.opcode_name(),
            pdu.opcode,
            pdu.parameters.len()
        )));
    }
    Ok(())
}

fn require_minimum_att_parameter_length(pdu: AttPdu<'_>, minimum: usize) -> Result<()> {
    if pdu.parameters.len() < minimum {
        return Err(Error::InvalidInput(format!(
            "ATT {} (0x{:02x}) requires at least {minimum} parameter octets, received {}",
            pdu.opcode_name(),
            pdu.opcode,
            pdu.parameters.len()
        )));
    }
    Ok(())
}

fn att_parameter_u16(pdu: AttPdu<'_>, offset: usize, field: &str) -> Result<u16> {
    let bytes = pdu
        .parameters
        .get(offset..offset.saturating_add(2))
        .ok_or_else(|| {
            Error::InvalidInput(format!("ATT {} has a truncated {field}", pdu.opcode_name()))
        })?;
    let bytes: [u8; 2] = bytes.try_into().map_err(|_| {
        Error::InvalidInput(format!("ATT {} has an invalid {field}", pdu.opcode_name()))
    })?;
    Ok(u16::from_le_bytes(bytes))
}

fn decode_att_handle(pdu: AttPdu<'_>, offset: usize, field: &str) -> Result<u16> {
    let handle = att_parameter_u16(pdu, offset, field)?;
    if handle == 0 {
        return Err(Error::InvalidInput(format!(
            "ATT {} {field} must be nonzero",
            pdu.opcode_name()
        )));
    }
    Ok(handle)
}

fn decode_att_handle_range(pdu: AttPdu<'_>, offset: usize) -> Result<AttHandleRange> {
    let range = AttHandleRange {
        start_handle: decode_att_handle(pdu, offset, "start handle")?,
        end_handle: decode_att_handle(pdu, offset + 2, "end handle")?,
    };
    if range.start_handle > range.end_handle {
        return Err(Error::InvalidInput(format!(
            "ATT {} start handle 0x{:04x} exceeds end handle 0x{:04x}",
            pdu.opcode_name(),
            range.start_handle,
            range.end_handle
        )));
    }
    Ok(range)
}

fn decode_att_uuid(bytes: &[u8]) -> Result<AttUuid> {
    match bytes {
        [low, high] => Ok(AttUuid::Uuid16(u16::from_le_bytes([*low, *high]))),
        bytes if bytes.len() == 16 => {
            let uuid: [u8; 16] = bytes.try_into().map_err(|_| {
                Error::InvalidInput("ATT 128-bit UUID has an invalid width".to_owned())
            })?;
            Ok(AttUuid::Uuid128(uuid))
        }
        _ => Err(Error::InvalidInput(format!(
            "ATT UUID requires 2 or 16 octets, received {}",
            bytes.len()
        ))),
    }
}

fn decode_att_find_information_response(pdu: AttPdu<'_>) -> Result<AttFindInformationResponse> {
    require_minimum_att_parameter_length(pdu, 1)?;
    let (uuid_width, entry_length) = match pdu.parameters[0] {
        1 => (2, 4),
        2 => (16, 18),
        format => {
            return Err(Error::InvalidInput(format!(
                "ATT Find Information Response format 0x{format:02x} is invalid"
            )));
        }
    };
    let bytes = &pdu.parameters[1..];
    if bytes.is_empty() || !bytes.len().is_multiple_of(entry_length) {
        return Err(Error::InvalidInput(format!(
            "ATT Find Information Response requires a nonempty list of {entry_length}-octet entries, received {} octets",
            bytes.len()
        )));
    }

    let mut entries = Vec::with_capacity(bytes.len() / entry_length);
    for entry in bytes.chunks_exact(entry_length) {
        let handle = u16::from_le_bytes([entry[0], entry[1]]);
        if handle == 0 {
            return Err(Error::InvalidInput(
                "ATT Find Information Response contains handle zero".to_owned(),
            ));
        }
        let uuid = decode_att_uuid(&entry[2..2 + uuid_width])?;
        entries.push(AttFindInformationEntry { handle, uuid });
    }
    Ok(AttFindInformationResponse { entries })
}

fn decode_att_handle_ranges(pdu: AttPdu<'_>) -> Result<Vec<AttHandleRange>> {
    if pdu.parameters.is_empty() || !pdu.parameters.len().is_multiple_of(4) {
        return Err(Error::InvalidInput(format!(
            "ATT {} requires a nonempty list of 4-octet handle ranges, received {} octets",
            pdu.opcode_name(),
            pdu.parameters.len()
        )));
    }
    let mut ranges = Vec::with_capacity(pdu.parameters.len() / 4);
    for offset in (0..pdu.parameters.len()).step_by(4) {
        ranges.push(decode_att_handle_range(pdu, offset)?);
    }
    Ok(ranges)
}

fn decode_att_attribute_data_response(pdu: AttPdu<'_>) -> Result<AttAttributeDataResponse<'_>> {
    require_minimum_att_parameter_length(pdu, 1)?;
    let entry_length = usize::from(pdu.parameters[0]);
    let bytes = &pdu.parameters[1..];
    if entry_length < 2 || bytes.is_empty() || !bytes.len().is_multiple_of(entry_length) {
        return Err(Error::InvalidInput(format!(
            "ATT Read By Type Response entry length {entry_length} does not frame {} data octets",
            bytes.len()
        )));
    }
    let mut entries = Vec::with_capacity(bytes.len() / entry_length);
    for entry in bytes.chunks_exact(entry_length) {
        let handle = u16::from_le_bytes([entry[0], entry[1]]);
        if handle == 0 {
            return Err(Error::InvalidInput(
                "ATT Read By Type Response contains handle zero".to_owned(),
            ));
        }
        entries.push(AttHandleValue {
            handle,
            value: &entry[2..],
        });
    }
    Ok(AttAttributeDataResponse {
        entry_length: entry_length as u8,
        entries,
    })
}

fn decode_att_handle_list(pdu: AttPdu<'_>, minimum_handles: usize) -> Result<Vec<u16>> {
    let minimum_length = minimum_handles.saturating_mul(2);
    if pdu.parameters.len() < minimum_length || !pdu.parameters.len().is_multiple_of(2) {
        return Err(Error::InvalidInput(format!(
            "ATT {} requires at least {minimum_handles} complete handles, received {} octets",
            pdu.opcode_name(),
            pdu.parameters.len()
        )));
    }
    let mut handles = Vec::with_capacity(pdu.parameters.len() / 2);
    for pair in pdu.parameters.chunks_exact(2) {
        let handle = u16::from_le_bytes([pair[0], pair[1]]);
        if handle == 0 {
            return Err(Error::InvalidInput(format!(
                "ATT {} contains handle zero",
                pdu.opcode_name()
            )));
        }
        handles.push(handle);
    }
    Ok(handles)
}

fn decode_att_group_data_response(pdu: AttPdu<'_>) -> Result<AttGroupDataResponse<'_>> {
    require_minimum_att_parameter_length(pdu, 1)?;
    let entry_length = usize::from(pdu.parameters[0]);
    let bytes = &pdu.parameters[1..];
    if entry_length < 4 || bytes.is_empty() || !bytes.len().is_multiple_of(entry_length) {
        return Err(Error::InvalidInput(format!(
            "ATT Read By Group Type Response entry length {entry_length} does not frame {} data octets",
            bytes.len()
        )));
    }
    let mut entries = Vec::with_capacity(bytes.len() / entry_length);
    for entry in bytes.chunks_exact(entry_length) {
        let range_pdu = AttPdu {
            opcode: pdu.opcode,
            parameters: entry,
        };
        entries.push(AttGroupValue {
            range: decode_att_handle_range(range_pdu, 0)?,
            value: &entry[4..],
        });
    }
    Ok(AttGroupDataResponse {
        entry_length: entry_length as u8,
        entries,
    })
}

fn decode_att_handle_value(pdu: AttPdu<'_>) -> Result<AttHandleValue<'_>> {
    require_minimum_att_parameter_length(pdu, 2)?;
    Ok(AttHandleValue {
        handle: decode_att_handle(pdu, 0, "handle")?,
        value: &pdu.parameters[2..],
    })
}

fn decode_att_prepare_write(pdu: AttPdu<'_>) -> Result<AttPrepareWrite<'_>> {
    require_minimum_att_parameter_length(pdu, 4)?;
    Ok(AttPrepareWrite {
        handle: decode_att_handle(pdu, 0, "handle")?,
        offset: att_parameter_u16(pdu, 2, "offset")?,
        value: &pdu.parameters[4..],
    })
}

fn decode_att_length_value_list(pdu: AttPdu<'_>) -> Result<AttLengthValueList<'_>> {
    require_minimum_att_parameter_length(pdu, 4)?;
    let mut bytes = pdu.parameters;
    let mut values = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 2 {
            return Err(Error::InvalidInput(
                "ATT Read Multiple Variable Length Response truncates a Value Length field"
                    .to_owned(),
            ));
        }
        let declared_length = u16::from_le_bytes([bytes[0], bytes[1]]);
        let length = usize::from(declared_length);
        bytes = &bytes[2..];
        if bytes.len() < length {
            values.push(AttLengthValue {
                declared_length,
                value: bytes,
                truncated: true,
            });
            bytes = &[];
        } else {
            values.push(AttLengthValue {
                declared_length,
                value: &bytes[..length],
                truncated: false,
            });
            bytes = &bytes[length..];
        }
    }
    let final_value_is_truncated = values.last().is_some_and(|value| value.truncated);
    if final_value_is_truncated && pdu.parameters.len() + 1 < usize::from(ATT_DEFAULT_LE_MTU) {
        return Err(Error::InvalidInput(format!(
            "ATT Read Multiple Variable Length Response truncates a value in a {}-octet PDU below the default MTU of {ATT_DEFAULT_LE_MTU}",
            pdu.parameters.len() + 1
        )));
    }
    if values.len() < 2 && !final_value_is_truncated {
        return Err(Error::InvalidInput(
            "ATT Read Multiple Variable Length Response requires at least two tuples unless its first value is truncated"
                .to_owned(),
        ));
    }
    Ok(AttLengthValueList { values })
}

fn decode_att_multiple_handle_values(pdu: AttPdu<'_>) -> Result<Vec<AttHandleValue<'_>>> {
    require_minimum_att_parameter_length(pdu, 8)?;
    let mut bytes = pdu.parameters;
    let mut values = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 4 {
            return Err(Error::InvalidInput(format!(
                "ATT Multiple Handle Value Notification has a truncated tuple header of {} octets",
                bytes.len()
            )));
        }
        let handle = u16::from_le_bytes([bytes[0], bytes[1]]);
        if handle == 0 {
            return Err(Error::InvalidInput(
                "ATT Multiple Handle Value Notification contains handle zero".to_owned(),
            ));
        }
        let length = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
        bytes = &bytes[4..];
        if bytes.len() < length {
            return Err(Error::InvalidInput(format!(
                "ATT Multiple Handle Value Notification declares a {length}-octet value with only {} octets remaining",
                bytes.len()
            )));
        }
        values.push(AttHandleValue {
            handle,
            value: &bytes[..length],
        });
        bytes = &bytes[length..];
    }
    if values.len() < 2 {
        return Err(Error::InvalidInput(
            "ATT Multiple Handle Value Notification requires at least two tuples".to_owned(),
        ));
    }
    Ok(values)
}

fn decode_att_signed_write(pdu: AttPdu<'_>) -> Result<AttSignedWrite<'_>> {
    require_minimum_att_parameter_length(pdu, 2 + ATT_SIGNATURE_LENGTH)?;
    let signature_offset = pdu.parameters.len() - ATT_SIGNATURE_LENGTH;
    let signature: [u8; ATT_SIGNATURE_LENGTH] = pdu.parameters[signature_offset..]
        .try_into()
        .map_err(|_| Error::InvalidInput("ATT signature has an invalid width".to_owned()))?;
    Ok(AttSignedWrite {
        handle: decode_att_handle(pdu, 0, "handle")?,
        value: &pdu.parameters[2..signature_offset],
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_layer::LinkDirection;

    fn decode(opcode: u8, parameters: &[u8]) -> Result<DecodedAttPdu<'_>> {
        AttPdu { opcode, parameters }.decode()
    }

    fn u16_bytes(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    #[test]
    fn fixed_att_channel_parser_preserves_unknown_pdus() {
        let mut l2cap = L2capPdu {
            direction: LinkDirection::CentralToPeripheral,
            channel_id: 5,
            payload: Vec::new(),
            fragment_count: 1,
        };
        assert_eq!(l2cap.att_pdu().unwrap(), None);
        l2cap.channel_id = ATT_FIXED_CHANNEL_ID;
        assert!(l2cap.att_pdu().is_err());
        l2cap.payload = vec![0x3f, 1, 2, 3];
        let pdu = l2cap.att_pdu().unwrap().unwrap();
        assert_eq!(pdu.opcode_name(), "unknown");
        assert_eq!(pdu.pdu_type(), AttPduType::Unknown);
        assert_eq!(
            pdu.decode().unwrap(),
            DecodedAttPdu::Unknown {
                opcode: 0x3f,
                parameters: &[1, 2, 3],
            }
        );
        assert_eq!(
            AttPdu {
                opcode: 0x40,
                parameters: &[]
            }
            .pdu_type(),
            AttPduType::Command
        );
    }

    #[test]
    fn decodes_fixed_att_pdu_layouts() {
        assert_eq!(
            decode(0x01, &[0x0a, 1, 0, 0x0a]).unwrap(),
            DecodedAttPdu::ErrorResponse(AttErrorResponse {
                request_opcode: 0x0a,
                handle: 1,
                error_code: 0x0a,
            })
        );
        assert_eq!(
            decode(0x02, &[0x40, 0]).unwrap(),
            DecodedAttPdu::ExchangeMtuRequest(AttExchangeMtu { mtu: 64 })
        );
        assert_eq!(
            decode(0x03, &[0x17, 0]).unwrap(),
            DecodedAttPdu::ExchangeMtuResponse(AttExchangeMtu { mtu: 23 })
        );
        assert_eq!(
            decode(0x04, &[1, 0, 0xff, 0xff]).unwrap(),
            DecodedAttPdu::FindInformationRequest(AttHandleRange {
                start_handle: 1,
                end_handle: 0xffff,
            })
        );
        assert_eq!(
            decode(0x0a, &[1, 0]).unwrap(),
            DecodedAttPdu::ReadRequest(AttHandle { handle: 1 })
        );
        assert_eq!(
            decode(0x0c, &[1, 0, 2, 0]).unwrap(),
            DecodedAttPdu::ReadBlobRequest(AttReadBlobRequest {
                handle: 1,
                offset: 2,
            })
        );
        assert_eq!(
            decode(0x18, &[1]).unwrap(),
            DecodedAttPdu::ExecuteWriteRequest(AttExecuteWriteFlags::Execute)
        );
        for opcode in [0x13, 0x19, 0x1e] {
            assert!(decode(opcode, &[]).is_ok());
        }
    }

    #[test]
    fn decodes_att_uuid_and_fixed_record_responses() {
        let find_info_16 = decode(0x05, &[1, 1, 0, 0x00, 0x28, 2, 0, 0x03, 0x28]).unwrap();
        let DecodedAttPdu::FindInformationResponse(response) = find_info_16 else {
            panic!("unexpected PDU");
        };
        assert_eq!(response.uuid_width(), 2);
        assert_eq!(
            response.entries,
            [
                AttFindInformationEntry {
                    handle: 1,
                    uuid: AttUuid::Uuid16(0x2800),
                },
                AttFindInformationEntry {
                    handle: 2,
                    uuid: AttUuid::Uuid16(0x2803),
                },
            ]
        );

        let mut find_info_128 = vec![2, 3, 0];
        find_info_128.extend(0u8..16);
        let DecodedAttPdu::FindInformationResponse(response) =
            decode(0x05, &find_info_128).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(response.uuid_width(), 16);
        assert_eq!(response.entries[0].handle, 3);

        let DecodedAttPdu::FindByTypeValueResponse(ranges) =
            decode(0x07, &u16_bytes(&[1, 5, 8, 12])).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(
            ranges,
            [
                AttHandleRange {
                    start_handle: 1,
                    end_handle: 5,
                },
                AttHandleRange {
                    start_handle: 8,
                    end_handle: 12,
                },
            ]
        );
    }

    #[test]
    fn decodes_att_type_requests_and_data_responses() {
        let request = decode(0x08, &[1, 0, 0xff, 0xff, 0x03, 0x28]).unwrap();
        assert_eq!(
            request,
            DecodedAttPdu::ReadByTypeRequest(AttReadByTypeRequest {
                range: AttHandleRange {
                    start_handle: 1,
                    end_handle: 0xffff,
                },
                attribute_type: AttUuid::Uuid16(0x2803),
            })
        );

        let mut request_128 = vec![1, 0, 0xff, 0xff];
        request_128.extend(0u8..16);
        let DecodedAttPdu::ReadByGroupTypeRequest(request) = decode(0x10, &request_128).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(request.attribute_type.width(), 16);

        let DecodedAttPdu::ReadByTypeResponse(response) =
            decode(0x09, &[4, 1, 0, 0xaa, 0xbb, 2, 0, 0xcc, 0xdd]).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(response.entry_length, 4);
        assert_eq!(
            response.entries,
            [
                AttHandleValue {
                    handle: 1,
                    value: &[0xaa, 0xbb],
                },
                AttHandleValue {
                    handle: 2,
                    value: &[0xcc, 0xdd],
                },
            ]
        );

        let DecodedAttPdu::ReadByGroupTypeResponse(response) =
            decode(0x11, &[6, 1, 0, 5, 0, 0x00, 0x18]).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(response.entry_length, 6);
        assert_eq!(response.entries[0].range.end_handle, 5);
        assert_eq!(response.entries[0].value, [0x00, 0x18]);
    }

    #[test]
    fn decodes_att_values_writes_and_handle_lists_losslessly() {
        assert_eq!(
            decode(0x06, &[1, 0, 0xff, 0xff, 0x00, 0x28, 0x0f, 0x18]).unwrap(),
            DecodedAttPdu::FindByTypeValueRequest(AttFindByTypeValueRequest {
                range: AttHandleRange {
                    start_handle: 1,
                    end_handle: 0xffff,
                },
                attribute_type: 0x2800,
                value: &[0x0f, 0x18],
            })
        );
        assert_eq!(
            decode(0x12, &[3, 0, 0xaa, 0xbb]).unwrap(),
            DecodedAttPdu::WriteRequest(AttHandleValue {
                handle: 3,
                value: &[0xaa, 0xbb],
            })
        );
        assert_eq!(
            decode(0x16, &[4, 0, 2, 0, 0xcc]).unwrap(),
            DecodedAttPdu::PrepareWriteRequest(AttPrepareWrite {
                handle: 4,
                offset: 2,
                value: &[0xcc],
            })
        );
        assert_eq!(
            decode(0x1b, &[5, 0]).unwrap(),
            DecodedAttPdu::HandleValueNotification(AttHandleValue {
                handle: 5,
                value: &[],
            })
        );
        assert_eq!(
            decode(0x0e, &u16_bytes(&[1, 2, 3])).unwrap(),
            DecodedAttPdu::ReadMultipleRequest(vec![1, 2, 3])
        );
        assert_eq!(
            decode(0x20, &u16_bytes(&[4, 5])).unwrap(),
            DecodedAttPdu::ReadMultipleVariableRequest(vec![4, 5])
        );
        assert_eq!(
            decode(0x0b, &[1, 2, 3]).unwrap(),
            DecodedAttPdu::ReadResponse(&[1, 2, 3])
        );
        assert_eq!(
            decode(0x0f, &[4, 5]).unwrap(),
            DecodedAttPdu::ReadMultipleResponse(&[4, 5])
        );
    }

    #[test]
    fn decodes_variable_length_values_notifications_and_signatures() {
        let mut response = vec![2, 0, 0xaa, 0xbb, 32, 0];
        response.extend(0..16);
        let DecodedAttPdu::ReadMultipleVariableResponse(list) = decode(0x21, &response).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(
            list.values,
            [
                AttLengthValue {
                    declared_length: 2,
                    value: &[0xaa, 0xbb],
                    truncated: false,
                },
                AttLengthValue {
                    declared_length: 32,
                    value: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                    truncated: true,
                },
            ]
        );

        let DecodedAttPdu::MultipleHandleValueNotification(values) =
            decode(0x23, &[1, 0, 2, 0, 0xaa, 0xbb, 2, 0, 1, 0, 0xcc]).unwrap()
        else {
            panic!("unexpected PDU");
        };
        assert_eq!(
            values,
            [
                AttHandleValue {
                    handle: 1,
                    value: &[0xaa, 0xbb],
                },
                AttHandleValue {
                    handle: 2,
                    value: &[0xcc],
                },
            ]
        );

        let mut signed = vec![3, 0, 0x11, 0x22];
        signed.extend(0u8..ATT_SIGNATURE_LENGTH as u8);
        let DecodedAttPdu::SignedWriteCommand(write) = decode(0xd2, &signed).unwrap() else {
            panic!("unexpected PDU");
        };
        assert_eq!(write.handle, 3);
        assert_eq!(write.value, [0x11, 0x22]);
        assert_eq!(write.signature, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn rejects_invalid_att_lengths_handles_ranges_and_records() {
        for (opcode, expected) in [
            (0x01, 4),
            (0x02, 2),
            (0x03, 2),
            (0x04, 4),
            (0x0a, 2),
            (0x0c, 4),
            (0x18, 1),
        ] {
            assert!(decode(opcode, &vec![0xff; expected - 1]).is_err());
            assert!(decode(opcode, &vec![0xff; expected + 1]).is_err());
        }
        for opcode in [0x13, 0x19, 0x1e] {
            assert!(decode(opcode, &[0]).is_err());
        }
        assert!(decode(0x01, &[0x0a, 0, 0, 0]).is_err());
        assert!(decode(0x02, &[22, 0]).is_err());
        assert_eq!(
            decode(0x02, &u16::MAX.to_le_bytes()).unwrap(),
            DecodedAttPdu::ExchangeMtuRequest(AttExchangeMtu { mtu: u16::MAX })
        );
        assert!(decode(0x04, &[0, 0, 1, 0]).is_err());
        assert!(decode(0x04, &[2, 0, 1, 0]).is_err());
        assert!(decode(0x08, &[1, 0, 2, 0, 1, 2, 3]).is_err());
        assert!(decode(0x0e, &u16_bytes(&[1])).is_err());
        assert!(decode(0x0e, &[1, 0, 2, 0, 3]).is_err());
        assert!(decode(0x18, &[2]).is_err());
    }

    #[test]
    fn rejects_malformed_att_variable_record_lists() {
        assert!(decode(0x05, &[0]).is_err());
        assert!(decode(0x05, &[1]).is_err());
        assert!(decode(0x05, &[1, 1, 0, 0]).is_err());
        assert!(decode(0x05, &[2, 1, 0, 0]).is_err());
        assert!(decode(0x07, &[]).is_err());
        assert!(decode(0x07, &[1, 0, 2]).is_err());
        assert!(decode(0x09, &[1, 1]).is_err());
        assert!(decode(0x09, &[3, 1, 0, 2, 0]).is_err());
        assert!(decode(0x11, &[3, 1, 0, 2]).is_err());
        assert!(decode(0x11, &[4, 1, 0, 0, 0]).is_err());
        assert!(decode(0x21, &[0, 0, 0]).is_err());
        assert!(decode(0x21, &[0, 0, 0, 0, 0]).is_err());
        assert!(decode(0x21, &[2, 0, 1, 2]).is_err());
        assert!(decode(0x21, &[3, 0, 1, 2]).is_err());
        assert!(decode(0x23, &[1, 0, 3, 0, 1, 2]).is_err());
        assert!(decode(0x23, &[1, 0, 4, 0, 1, 2, 3, 4]).is_err());
        assert!(decode(0x23, &[0, 0, 0, 0]).is_err());
        assert!(decode(0xd2, &[1, 0, 1, 2, 3]).is_err());
    }

    #[test]
    fn att_parser_handles_arbitrary_bounded_pdus_without_panicking() {
        for length in 0usize..=64 {
            let payload: Vec<u8> = (0..length).map(|index| index as u8).collect();
            if let Ok(pdu) = AttPdu::parse(&payload) {
                let _ = pdu.decode();
            }
        }
    }

    #[test]
    fn reports_att_names_types_and_error_names() {
        assert_eq!(
            AttPdu {
                opcode: 0x1d,
                parameters: &[]
            }
            .pdu_type()
            .to_string(),
            "indication"
        );
        assert_eq!(att_error_name(0x12), "database-out-of-sync");
        assert_eq!(att_error_name(0x80), "unknown");
        assert_eq!(
            AttErrorResponse {
                request_opcode: 0x0a,
                handle: 1,
                error_code: 0x0a,
            }
            .error_name(),
            "attribute-not-found"
        );
    }
}
