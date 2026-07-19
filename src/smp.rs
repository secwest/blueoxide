use crate::link_layer::L2capPdu;
use crate::{Error, Result};
use std::fmt::{Display, Formatter};

pub const LE_SMP_FIXED_CHANNEL_ID: u16 = 0x0006;
pub const SMP_KEY_LENGTH: usize = 16;
pub const SMP_PUBLIC_KEY_COORDINATE_LENGTH: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpPdu<'a> {
    pub code: u8,
    pub parameters: &'a [u8],
}

impl<'a> SmpPdu<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let Some((&code, parameters)) = payload.split_first() else {
            return Err(Error::InvalidInput(
                "SMP PDU is missing its command code".to_owned(),
            ));
        };
        Ok(Self { code, parameters })
    }

    pub const fn code_name(self) -> &'static str {
        match self.code {
            0x01 => "pairing-request",
            0x02 => "pairing-response",
            0x03 => "pairing-confirm",
            0x04 => "pairing-random",
            0x05 => "pairing-failed",
            0x06 => "encryption-information",
            0x07 => "central-identification",
            0x08 => "identity-information",
            0x09 => "identity-address-information",
            0x0a => "signing-information",
            0x0b => "security-request",
            0x0c => "pairing-public-key",
            0x0d => "pairing-dhkey-check",
            0x0e => "keypress-notification",
            _ => "unknown",
        }
    }

    pub fn decode(self) -> Result<DecodedSmpPdu<'a>> {
        match self.code {
            0x01 | 0x02 => {
                require_smp_parameter_length(self, 6)?;
                let features = SmpPairingFeatures {
                    io_capability: SmpIoCapability::parse(self.parameters[0])?,
                    oob_data_present: match self.parameters[1] {
                        0 => false,
                        1 => true,
                        value => {
                            return Err(Error::InvalidInput(format!(
                                "SMP {} OOB data flag 0x{value:02x} is reserved",
                                self.code_name()
                            )));
                        }
                    },
                    authentication: SmpAuthenticationRequirements::parse(self.parameters[2], true)?,
                    maximum_encryption_key_size: parse_encryption_key_size(self.parameters[3])?,
                    initiator_key_distribution: SmpKeyDistribution::parse(self.parameters[4])?,
                    responder_key_distribution: SmpKeyDistribution::parse(self.parameters[5])?,
                };
                if self.code == 0x01 {
                    Ok(DecodedSmpPdu::PairingRequest(features))
                } else {
                    Ok(DecodedSmpPdu::PairingResponse(features))
                }
            }
            0x03 => Ok(DecodedSmpPdu::PairingConfirm(smp_array(self)?)),
            0x04 => Ok(DecodedSmpPdu::PairingRandom(smp_array(self)?)),
            0x05 => {
                require_smp_parameter_length(self, 1)?;
                let reason = self.parameters[0];
                if !(0x01..=0x10).contains(&reason) {
                    return Err(Error::InvalidInput(format!(
                        "SMP Pairing Failed reason 0x{reason:02x} is reserved"
                    )));
                }
                Ok(DecodedSmpPdu::PairingFailed(SmpPairingFailure { reason }))
            }
            0x06 => Ok(DecodedSmpPdu::EncryptionInformation(smp_array(self)?)),
            0x07 => {
                require_smp_parameter_length(self, 10)?;
                Ok(DecodedSmpPdu::CentralIdentification(
                    SmpCentralIdentification {
                        encrypted_diversifier: u16::from_le_bytes([
                            self.parameters[0],
                            self.parameters[1],
                        ]),
                        random: self.parameters[2..].try_into().map_err(|_| {
                            Error::InvalidInput(
                                "SMP Central Identification random value has an invalid width"
                                    .to_owned(),
                            )
                        })?,
                    },
                ))
            }
            0x08 => Ok(DecodedSmpPdu::IdentityInformation(smp_array(self)?)),
            0x09 => {
                require_smp_parameter_length(self, 7)?;
                let address_type = SmpIdentityAddressType::parse(self.parameters[0])?;
                let address: [u8; 6] = self.parameters[1..].try_into().map_err(|_| {
                    Error::InvalidInput(
                        "SMP Identity Address Information address has an invalid width".to_owned(),
                    )
                })?;
                if address_type == SmpIdentityAddressType::StaticRandom {
                    validate_static_random_address(address)?;
                }
                Ok(DecodedSmpPdu::IdentityAddressInformation(
                    SmpIdentityAddress {
                        address_type,
                        address,
                    },
                ))
            }
            0x0a => Ok(DecodedSmpPdu::SigningInformation(smp_array(self)?)),
            0x0b => {
                require_smp_parameter_length(self, 1)?;
                Ok(DecodedSmpPdu::SecurityRequest(
                    SmpAuthenticationRequirements::parse(self.parameters[0], false)?,
                ))
            }
            0x0c => {
                require_smp_parameter_length(self, 64)?;
                Ok(DecodedSmpPdu::PairingPublicKey(SmpPublicKey {
                    x: self.parameters[..SMP_PUBLIC_KEY_COORDINATE_LENGTH]
                        .try_into()
                        .map_err(|_| {
                            Error::InvalidInput(
                                "SMP public-key X coordinate has an invalid width".to_owned(),
                            )
                        })?,
                    y: self.parameters[SMP_PUBLIC_KEY_COORDINATE_LENGTH..]
                        .try_into()
                        .map_err(|_| {
                            Error::InvalidInput(
                                "SMP public-key Y coordinate has an invalid width".to_owned(),
                            )
                        })?,
                }))
            }
            0x0d => Ok(DecodedSmpPdu::PairingDhKeyCheck(smp_array(self)?)),
            0x0e => {
                require_smp_parameter_length(self, 1)?;
                Ok(DecodedSmpPdu::KeypressNotification(
                    SmpKeypressNotificationType::parse(self.parameters[0])?,
                ))
            }
            _ => Ok(DecodedSmpPdu::Unknown {
                code: self.code,
                parameters: self.parameters,
            }),
        }
    }
}

impl L2capPdu {
    pub fn smp_pdu(&self) -> Result<Option<SmpPdu<'_>>> {
        if self.channel_id != LE_SMP_FIXED_CHANNEL_ID {
            return Ok(None);
        }
        Ok(Some(SmpPdu::parse(&self.payload)?))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedSmpPdu<'a> {
    PairingRequest(SmpPairingFeatures),
    PairingResponse(SmpPairingFeatures),
    PairingConfirm([u8; SMP_KEY_LENGTH]),
    PairingRandom([u8; SMP_KEY_LENGTH]),
    PairingFailed(SmpPairingFailure),
    EncryptionInformation([u8; SMP_KEY_LENGTH]),
    CentralIdentification(SmpCentralIdentification),
    IdentityInformation([u8; SMP_KEY_LENGTH]),
    IdentityAddressInformation(SmpIdentityAddress),
    SigningInformation([u8; SMP_KEY_LENGTH]),
    SecurityRequest(SmpAuthenticationRequirements),
    PairingPublicKey(SmpPublicKey),
    PairingDhKeyCheck([u8; SMP_KEY_LENGTH]),
    KeypressNotification(SmpKeypressNotificationType),
    Unknown { code: u8, parameters: &'a [u8] },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpPairingFeatures {
    pub io_capability: SmpIoCapability,
    pub oob_data_present: bool,
    pub authentication: SmpAuthenticationRequirements,
    pub maximum_encryption_key_size: u8,
    pub initiator_key_distribution: SmpKeyDistribution,
    pub responder_key_distribution: SmpKeyDistribution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmpIoCapability {
    DisplayOnly,
    DisplayYesNo,
    KeyboardOnly,
    NoInputNoOutput,
    KeyboardDisplay,
}

impl SmpIoCapability {
    fn parse(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::DisplayOnly),
            0x01 => Ok(Self::DisplayYesNo),
            0x02 => Ok(Self::KeyboardOnly),
            0x03 => Ok(Self::NoInputNoOutput),
            0x04 => Ok(Self::KeyboardDisplay),
            _ => Err(Error::InvalidInput(format!(
                "SMP IO capability 0x{value:02x} is reserved"
            ))),
        }
    }
}

impl Display for SmpIoCapability {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayOnly => formatter.write_str("display-only"),
            Self::DisplayYesNo => formatter.write_str("display-yes-no"),
            Self::KeyboardOnly => formatter.write_str("keyboard-only"),
            Self::NoInputNoOutput => formatter.write_str("no-input-no-output"),
            Self::KeyboardDisplay => formatter.write_str("keyboard-display"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpAuthenticationRequirements {
    pub raw: u8,
}

impl SmpAuthenticationRequirements {
    fn parse(raw: u8, allow_ct2: bool) -> Result<Self> {
        let allowed_mask = if allow_ct2 { 0x3f } else { 0x1f };
        if raw & !allowed_mask != 0 {
            return Err(Error::InvalidInput(format!(
                "SMP authentication requirements 0x{raw:02x} set reserved bits"
            )));
        }
        if raw & 0x03 > 1 {
            return Err(Error::InvalidInput(format!(
                "SMP authentication requirements 0x{raw:02x} use a reserved bonding value"
            )));
        }
        Ok(Self { raw })
    }

    pub const fn bonding(self) -> bool {
        self.raw & 0x01 != 0
    }

    pub const fn mitm(self) -> bool {
        self.raw & 0x04 != 0
    }

    pub const fn secure_connections(self) -> bool {
        self.raw & 0x08 != 0
    }

    pub const fn keypress(self) -> bool {
        self.raw & 0x10 != 0
    }

    pub const fn ct2(self) -> bool {
        self.raw & 0x20 != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpKeyDistribution {
    pub raw: u8,
}

impl SmpKeyDistribution {
    fn parse(raw: u8) -> Result<Self> {
        if raw & 0xf0 != 0 {
            return Err(Error::InvalidInput(format!(
                "SMP key distribution 0x{raw:02x} sets reserved bits"
            )));
        }
        Ok(Self { raw })
    }

    pub const fn encryption_key(self) -> bool {
        self.raw & 0x01 != 0
    }

    pub const fn identity_key(self) -> bool {
        self.raw & 0x02 != 0
    }

    pub const fn signing_key(self) -> bool {
        self.raw & 0x04 != 0
    }

    pub const fn link_key(self) -> bool {
        self.raw & 0x08 != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpPairingFailure {
    pub reason: u8,
}

impl SmpPairingFailure {
    pub const fn reason_name(self) -> &'static str {
        smp_pairing_failure_name(self.reason)
    }
}

pub const fn smp_pairing_failure_name(reason: u8) -> &'static str {
    match reason {
        0x01 => "passkey-entry-failed",
        0x02 => "oob-not-available",
        0x03 => "authentication-requirements",
        0x04 => "confirm-value-failed",
        0x05 => "pairing-not-supported",
        0x06 => "encryption-key-size",
        0x07 => "command-not-supported",
        0x08 => "unspecified-reason",
        0x09 => "repeated-attempts",
        0x0a => "invalid-parameters",
        0x0b => "dhkey-check-failed",
        0x0c => "numeric-comparison-failed",
        0x0d => "bredr-pairing-in-progress",
        0x0e => "cross-transport-key-derivation-not-allowed",
        0x0f => "key-rejected",
        0x10 => "busy",
        _ => "unknown",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpCentralIdentification {
    pub encrypted_diversifier: u16,
    pub random: [u8; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmpIdentityAddressType {
    Public,
    StaticRandom,
}

impl SmpIdentityAddressType {
    fn parse(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Public),
            1 => Ok(Self::StaticRandom),
            _ => Err(Error::InvalidInput(format!(
                "SMP identity address type 0x{value:02x} is reserved"
            ))),
        }
    }
}

impl Display for SmpIdentityAddressType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => formatter.write_str("public"),
            Self::StaticRandom => formatter.write_str("static-random"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpIdentityAddress {
    pub address_type: SmpIdentityAddressType,
    pub address: [u8; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmpPublicKey {
    pub x: [u8; SMP_PUBLIC_KEY_COORDINATE_LENGTH],
    pub y: [u8; SMP_PUBLIC_KEY_COORDINATE_LENGTH],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmpKeypressNotificationType {
    PasskeyEntryStarted,
    PasskeyDigitEntered,
    PasskeyDigitErased,
    PasskeyCleared,
    PasskeyEntryCompleted,
}

impl SmpKeypressNotificationType {
    fn parse(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::PasskeyEntryStarted),
            1 => Ok(Self::PasskeyDigitEntered),
            2 => Ok(Self::PasskeyDigitErased),
            3 => Ok(Self::PasskeyCleared),
            4 => Ok(Self::PasskeyEntryCompleted),
            _ => Err(Error::InvalidInput(format!(
                "SMP keypress notification type 0x{value:02x} is reserved"
            ))),
        }
    }
}

impl Display for SmpKeypressNotificationType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PasskeyEntryStarted => formatter.write_str("passkey-entry-started"),
            Self::PasskeyDigitEntered => formatter.write_str("passkey-digit-entered"),
            Self::PasskeyDigitErased => formatter.write_str("passkey-digit-erased"),
            Self::PasskeyCleared => formatter.write_str("passkey-cleared"),
            Self::PasskeyEntryCompleted => formatter.write_str("passkey-entry-completed"),
        }
    }
}

fn require_smp_parameter_length(pdu: SmpPdu<'_>, expected: usize) -> Result<()> {
    if pdu.parameters.len() != expected {
        return Err(Error::InvalidInput(format!(
            "SMP {} (0x{:02x}) requires {expected} parameter octets, received {}",
            pdu.code_name(),
            pdu.code,
            pdu.parameters.len()
        )));
    }
    Ok(())
}

fn smp_array<const N: usize>(pdu: SmpPdu<'_>) -> Result<[u8; N]> {
    require_smp_parameter_length(pdu, N)?;
    pdu.parameters.try_into().map_err(|_| {
        Error::InvalidInput(format!(
            "SMP {} has an invalid parameter width",
            pdu.code_name()
        ))
    })
}

fn parse_encryption_key_size(value: u8) -> Result<u8> {
    if !(7..=16).contains(&value) {
        return Err(Error::InvalidInput(format!(
            "SMP maximum encryption key size {value} is outside 7..=16"
        )));
    }
    Ok(value)
}

fn validate_static_random_address(address: [u8; 6]) -> Result<()> {
    if address[5] & 0xc0 != 0xc0 {
        return Err(Error::InvalidInput(
            "SMP static random identity address does not set its two most significant bits"
                .to_owned(),
        ));
    }
    let random_part_all_zero = address[..5].iter().all(|byte| *byte == 0) && address[5] & 0x3f == 0;
    let random_part_all_one =
        address[..5].iter().all(|byte| *byte == 0xff) && address[5] & 0x3f == 0x3f;
    if random_part_all_zero || random_part_all_one {
        return Err(Error::InvalidInput(
            "SMP static random identity address has a degenerate random part".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_layer::LinkDirection;

    fn decode(code: u8, parameters: &[u8]) -> Result<DecodedSmpPdu<'_>> {
        SmpPdu { code, parameters }.decode()
    }

    #[test]
    fn fixed_smp_channel_parser_preserves_unknown_commands() {
        let mut l2cap = L2capPdu {
            direction: LinkDirection::CentralToPeripheral,
            channel_id: 4,
            payload: Vec::new(),
            fragment_count: 1,
        };
        assert_eq!(l2cap.smp_pdu().unwrap(), None);
        l2cap.channel_id = LE_SMP_FIXED_CHANNEL_ID;
        assert!(l2cap.smp_pdu().is_err());
        l2cap.payload = vec![0x7f, 1, 2, 3];
        let pdu = l2cap.smp_pdu().unwrap().unwrap();
        assert_eq!(pdu.code_name(), "unknown");
        assert_eq!(
            pdu.decode().unwrap(),
            DecodedSmpPdu::Unknown {
                code: 0x7f,
                parameters: &[1, 2, 3],
            }
        );
    }

    #[test]
    fn decodes_pairing_features_and_authentication_flags() {
        let DecodedSmpPdu::PairingRequest(features) =
            decode(0x01, &[0x04, 0x01, 0x3d, 16, 0x0f, 0x06]).unwrap()
        else {
            panic!("unexpected command");
        };
        assert_eq!(features.io_capability, SmpIoCapability::KeyboardDisplay);
        assert!(features.oob_data_present);
        assert!(features.authentication.bonding());
        assert!(features.authentication.mitm());
        assert!(features.authentication.secure_connections());
        assert!(features.authentication.keypress());
        assert!(features.authentication.ct2());
        assert_eq!(features.maximum_encryption_key_size, 16);
        assert!(features.initiator_key_distribution.encryption_key());
        assert!(features.initiator_key_distribution.identity_key());
        assert!(features.initiator_key_distribution.signing_key());
        assert!(features.initiator_key_distribution.link_key());
        assert_eq!(
            features.responder_key_distribution,
            SmpKeyDistribution { raw: 0x06 }
        );
        assert_eq!(features.io_capability.to_string(), "keyboard-display");
    }

    #[test]
    fn rejects_invalid_pairing_features() {
        assert!(decode(0x01, &[3, 0, 0, 16, 0, 0, 0]).is_err());
        assert!(decode(0x01, &[5, 0, 0, 16, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 2, 0, 16, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 0, 2, 16, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 0, 0x40, 16, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 0, 0, 6, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 0, 0, 17, 0, 0]).is_err());
        assert!(decode(0x01, &[3, 0, 0, 16, 0x10, 0]).is_err());
        assert!(decode(0x02, &[3, 0, 0, 16, 0, 0x80]).is_err());
    }

    #[test]
    fn decodes_confirm_random_keys_and_central_identification() {
        let value: Vec<u8> = (0..16).collect();
        assert_eq!(
            decode(0x03, &value).unwrap(),
            DecodedSmpPdu::PairingConfirm(value.clone().try_into().unwrap())
        );
        assert_eq!(
            decode(0x04, &value).unwrap(),
            DecodedSmpPdu::PairingRandom(value.clone().try_into().unwrap())
        );
        assert_eq!(
            decode(0x06, &value).unwrap(),
            DecodedSmpPdu::EncryptionInformation(value.clone().try_into().unwrap())
        );
        assert_eq!(
            decode(0x08, &value).unwrap(),
            DecodedSmpPdu::IdentityInformation(value.clone().try_into().unwrap())
        );
        assert_eq!(
            decode(0x0a, &value).unwrap(),
            DecodedSmpPdu::SigningInformation(value.clone().try_into().unwrap())
        );
        assert_eq!(
            decode(0x07, &[0x34, 0x12, 0, 1, 2, 3, 4, 5, 6, 7]).unwrap(),
            DecodedSmpPdu::CentralIdentification(SmpCentralIdentification {
                encrypted_diversifier: 0x1234,
                random: [0, 1, 2, 3, 4, 5, 6, 7],
            })
        );
    }

    #[test]
    fn decodes_and_validates_identity_addresses() {
        assert_eq!(
            decode(0x09, &[0, 1, 2, 3, 4, 5, 6]).unwrap(),
            DecodedSmpPdu::IdentityAddressInformation(SmpIdentityAddress {
                address_type: SmpIdentityAddressType::Public,
                address: [1, 2, 3, 4, 5, 6],
            })
        );
        assert!(decode(0x09, &[1, 1, 2, 3, 4, 5, 0xc6]).is_ok());
        assert!(decode(0x09, &[2, 1, 2, 3, 4, 5, 6]).is_err());
        assert!(decode(0x09, &[1, 1, 2, 3, 4, 5, 0x86]).is_err());
        assert!(decode(0x09, &[1, 0, 0, 0, 0, 0, 0xc0]).is_err());
        assert!(decode(0x09, &[1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).is_err());
        assert_eq!(
            SmpIdentityAddressType::StaticRandom.to_string(),
            "static-random"
        );
    }

    #[test]
    fn decodes_pairing_failures_security_requests_and_keypress() {
        let DecodedSmpPdu::PairingFailed(failure) = decode(0x05, &[0x10]).unwrap() else {
            panic!("unexpected command");
        };
        assert_eq!(failure.reason_name(), "busy");
        assert_eq!(smp_pairing_failure_name(0x0f), "key-rejected");
        assert!(decode(0x05, &[0]).is_err());
        assert!(decode(0x05, &[0x11]).is_err());

        let DecodedSmpPdu::SecurityRequest(authentication) = decode(0x0b, &[0x1d]).unwrap() else {
            panic!("unexpected command");
        };
        assert!(authentication.bonding());
        assert!(authentication.mitm());
        assert!(authentication.secure_connections());
        assert!(authentication.keypress());
        assert!(!authentication.ct2());
        assert!(decode(0x0b, &[0x20]).is_err());

        assert_eq!(
            decode(0x0e, &[4]).unwrap(),
            DecodedSmpPdu::KeypressNotification(SmpKeypressNotificationType::PasskeyEntryCompleted)
        );
        assert!(decode(0x0e, &[5]).is_err());
    }

    #[test]
    fn decodes_public_key_and_dhkey_check_losslessly() {
        let public_key: Vec<u8> = (0..64).collect();
        let DecodedSmpPdu::PairingPublicKey(key) = decode(0x0c, &public_key).unwrap() else {
            panic!("unexpected command");
        };
        assert_eq!(key.x, public_key[..32]);
        assert_eq!(key.y, public_key[32..]);

        let check: Vec<u8> = (0x80..0x90).collect();
        assert_eq!(
            decode(0x0d, &check).unwrap(),
            DecodedSmpPdu::PairingDhKeyCheck(check.clone().try_into().unwrap())
        );
    }

    #[test]
    fn rejects_all_known_command_length_mismatches() {
        for (code, expected) in [
            (0x01, 6),
            (0x02, 6),
            (0x03, 16),
            (0x04, 16),
            (0x05, 1),
            (0x06, 16),
            (0x07, 10),
            (0x08, 16),
            (0x09, 7),
            (0x0a, 16),
            (0x0b, 1),
            (0x0c, 64),
            (0x0d, 16),
            (0x0e, 1),
        ] {
            let expected: usize = expected;
            assert!(decode(code, &vec![0; expected.saturating_sub(1)]).is_err());
            assert!(decode(code, &vec![0; expected + 1]).is_err());
        }
    }

    #[test]
    fn reports_all_assigned_command_names() {
        let names = [
            "pairing-request",
            "pairing-response",
            "pairing-confirm",
            "pairing-random",
            "pairing-failed",
            "encryption-information",
            "central-identification",
            "identity-information",
            "identity-address-information",
            "signing-information",
            "security-request",
            "pairing-public-key",
            "pairing-dhkey-check",
            "keypress-notification",
        ];
        for (index, name) in names.iter().enumerate() {
            assert_eq!(
                SmpPdu {
                    code: index as u8 + 1,
                    parameters: &[],
                }
                .code_name(),
                *name
            );
        }
    }

    #[test]
    fn smp_parser_handles_arbitrary_bounded_pdus_without_panicking() {
        for length in 0usize..=80 {
            let payload: Vec<u8> = (0..length).map(|index| index as u8).collect();
            if let Ok(pdu) = SmpPdu::parse(&payload) {
                let _ = pdu.decode();
            }
        }
    }
}
