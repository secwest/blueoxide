use crate::ble::AdvertisingPdu;
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
}

impl ConnectRequest {
    pub fn interval_us(&self) -> u32 {
        self.interval as u32 * 1_250
    }

    pub fn supervision_timeout_us(&self) -> u32 {
        self.supervision_timeout as u32 * 10_000
    }

    pub fn enabled_data_channels(&self) -> Vec<u8> {
        (0..=36)
            .filter(|channel| self.channel_map[*channel as usize / 8] & (1 << (*channel % 8)) != 0)
            .collect()
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
                "CONNECT_IND initiator={initiator} initiator_type={initiator_kind} advertiser={advertiser} advertiser_type={advertiser_kind} access_address={:08x} interval_us={} latency={} timeout_us={} hop={} channels={}",
                request.access_address,
                request.interval_us(),
                request.latency,
                request.supervision_timeout_us(),
                request.hop_increment,
                request.enabled_data_channels().len()
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
            if request.channel_map[4] & 0xe0 != 0 {
                return Err(Error::InvalidInput(
                    "CONNECT_IND channel map sets reserved bits 37..39".to_owned(),
                ));
            }
            if request.enabled_data_channels().len() < 2 {
                return Err(Error::InvalidInput(
                    "CONNECT_IND channel map enables fewer than two data channels".to_owned(),
                ));
            }
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
