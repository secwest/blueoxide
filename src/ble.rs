use crate::{Error, Result};

pub const LE_ADV_ACCESS_ADDRESS: u32 = 0x8e89_bed6;
pub const LE_ADV_CRC_INIT: u32 = 0x55_55_55;
pub const LE_PRIMARY_ADV_MAX_PAYLOAD: usize = 37;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BleChannel(u8);

impl BleChannel {
    pub fn new(index: u8) -> Result<Self> {
        if index <= 39 {
            Ok(Self(index))
        } else {
            Err(Error::InvalidChannel(index))
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub const fn is_primary_advertising(self) -> bool {
        matches!(self.0, 37..=39)
    }

    pub const fn center_frequency_hz(self) -> u64 {
        let mhz = match self.0 {
            37 => 2402,
            0..=10 => 2404 + self.0 as u64 * 2,
            38 => 2426,
            11..=36 => 2428 + (self.0 as u64 - 11) * 2,
            39 => 2480,
            _ => unreachable!(),
        };
        mhz * 1_000_000
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvertisingPdu {
    pub channel: BleChannel,
    pub bit_offset: usize,
    pub inverted: bool,
    pub access_address_errors: u8,
    pub header: [u8; 2],
    pub payload: Vec<u8>,
    pub crc: [u8; 3],
}

impl AdvertisingPdu {
    pub fn pdu_type(&self) -> u8 {
        self.header[0] & 0x0f
    }

    pub fn tx_add_random(&self) -> bool {
        self.header[0] & 0x40 != 0
    }

    pub fn rx_add_random(&self) -> bool {
        self.header[0] & 0x80 != 0
    }
}

#[derive(Clone, Copy, Debug)]
struct Whitening {
    state: [bool; 7],
}

impl Whitening {
    fn new(channel: BleChannel) -> Self {
        let channel = channel.index();
        Self {
            state: [
                true,
                channel & 0x20 != 0,
                channel & 0x10 != 0,
                channel & 0x08 != 0,
                channel & 0x04 != 0,
                channel & 0x02 != 0,
                channel & 0x01 != 0,
            ],
        }
    }

    fn apply(&mut self, bit: bool) -> bool {
        let feedback = self.state[6];
        let output = bit ^ feedback;
        self.state = [
            feedback,
            self.state[0],
            self.state[1],
            self.state[2],
            self.state[3] ^ feedback,
            self.state[4],
            self.state[5],
        ];
        output
    }
}

pub fn whiten_bits(bits: &mut [bool], channel: BleChannel) {
    let mut whitening = Whitening::new(channel);
    for bit in bits {
        *bit = whitening.apply(*bit);
    }
}

/// Calculates the Bluetooth LE 24-bit CRC state.
///
/// Bytes are consumed least-significant bit first, matching their over-the-air
/// order. `crc_init` is accepted in the conventional hexadecimal byte order.
pub fn crc24_state(bytes: &[u8], crc_init: u32) -> u32 {
    let init = crc_init & 0x00ff_ffff;
    let mut state = ((init & 0xff) << 16) | (init & 0xff00) | ((init >> 16) & 0xff);

    for byte in bytes {
        for bit_index in 0..8 {
            let input = (byte >> bit_index) & 1;
            let feedback = ((state >> 23) as u8 & 1) ^ input;
            state = (state << 1) & 0x00ff_ffff;
            if feedback != 0 {
                state ^= 0x0000_065b;
            }
        }
    }
    state
}

/// Returns CRC octets in their transmitted order.
pub fn crc24_bytes(bytes: &[u8], crc_init: u32) -> [u8; 3] {
    let state = crc24_state(bytes, crc_init);
    [
        ((state >> 16) as u8).reverse_bits(),
        ((state >> 8) as u8).reverse_bits(),
        (state as u8).reverse_bits(),
    ]
}

pub fn bytes_to_bits_lsb(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for shift in 0..8 {
            bits.push((byte >> shift) & 1 != 0);
        }
    }
    bits
}

pub fn bits_to_bytes_lsb(bits: &[bool]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(bits.len().div_ceil(8));
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (shift, bit) in chunk.iter().enumerate() {
            if *bit {
                byte |= 1 << shift;
            }
        }
        bytes.push(byte);
    }
    bytes
}

/// Finds CRC-valid primary advertising packets in a hard-decision bit stream.
///
/// Both normal and inverted spectra are checked. Access-address errors are
/// tolerated only up to `max_access_address_errors`; CRC validation remains
/// mandatory before a packet is returned.
pub fn decode_primary_advertising(
    bits: &[bool],
    channel: BleChannel,
    max_access_address_errors: u8,
) -> Result<Vec<AdvertisingPdu>> {
    if !channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "channel {} is not a primary advertising channel",
            channel.index()
        )));
    }
    if max_access_address_errors > 8 {
        return Err(Error::InvalidConfiguration(
            "access-address error tolerance must be 0..=8".to_owned(),
        ));
    }

    let access_bits = bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes());
    let minimum_body_bits = (2 + 3) * 8;
    if bits.len() < access_bits.len() + minimum_body_bits {
        return Ok(Vec::new());
    }

    let mut packets = Vec::new();
    for inverted in [false, true] {
        for offset in 0..=bits.len() - access_bits.len() - minimum_body_bits {
            let errors = access_bits
                .iter()
                .enumerate()
                .filter(|(index, expected)| (bits[offset + index] ^ inverted) != **expected)
                .count() as u8;
            if errors > max_access_address_errors {
                continue;
            }

            let body_start = offset + access_bits.len();
            let available = &bits[body_start..];
            let maximum_body_bits = (2 + LE_PRIMARY_ADV_MAX_PAYLOAD + 3) * 8;
            let mut body_bits: Vec<bool> = available
                .iter()
                .take(maximum_body_bits)
                .map(|bit| *bit ^ inverted)
                .collect();
            whiten_bits(&mut body_bits, channel);
            let body = bits_to_bytes_lsb(&body_bits);
            if body.len() < 5 {
                continue;
            }

            let payload_length = (body[1] & 0x3f) as usize;
            if payload_length > LE_PRIMARY_ADV_MAX_PAYLOAD {
                continue;
            }
            let pdu_length = 2 + payload_length;
            let total_length = pdu_length + 3;
            if body.len() < total_length {
                continue;
            }

            let received_crc = [body[pdu_length], body[pdu_length + 1], body[pdu_length + 2]];
            if crc24_bytes(&body[..pdu_length], LE_ADV_CRC_INIT) != received_crc {
                continue;
            }

            let packet = AdvertisingPdu {
                channel,
                bit_offset: offset,
                inverted,
                access_address_errors: errors,
                header: [body[0], body[1]],
                payload: body[2..pdu_length].to_vec(),
                crc: received_crc,
            };
            if !packets.iter().any(|existing: &AdvertisingPdu| {
                existing.header == packet.header
                    && existing.payload == packet.payload
                    && existing.crc == packet.crc
            }) {
                packets.push(packet);
            }
        }
    }
    Ok(packets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_frequency_map_covers_band_edges() {
        assert_eq!(
            BleChannel::new(37).unwrap().center_frequency_hz(),
            2_402_000_000
        );
        assert_eq!(
            BleChannel::new(0).unwrap().center_frequency_hz(),
            2_404_000_000
        );
        assert_eq!(
            BleChannel::new(10).unwrap().center_frequency_hz(),
            2_424_000_000
        );
        assert_eq!(
            BleChannel::new(38).unwrap().center_frequency_hz(),
            2_426_000_000
        );
        assert_eq!(
            BleChannel::new(11).unwrap().center_frequency_hz(),
            2_428_000_000
        );
        assert_eq!(
            BleChannel::new(36).unwrap().center_frequency_hz(),
            2_478_000_000
        );
        assert_eq!(
            BleChannel::new(39).unwrap().center_frequency_hz(),
            2_480_000_000
        );
    }

    #[test]
    fn whitening_is_self_inverse() {
        let channel = BleChannel::new(38).unwrap();
        let original = bytes_to_bits_lsb(&[0x42, 0x19, 0xaa, 0x00, 0xff]);
        let mut transformed = original.clone();
        whiten_bits(&mut transformed, channel);
        assert_ne!(transformed, original);
        whiten_bits(&mut transformed, channel);
        assert_eq!(transformed, original);
    }

    #[test]
    fn crc_matches_independent_lfsr_vector() {
        assert_eq!(crc24_state(&[0x00, 0x00], LE_ADV_CRC_INIT), 0xb8ad1c);
        assert_eq!(
            crc24_bytes(&[0x00, 0x00], LE_ADV_CRC_INIT),
            [0x1d, 0xb5, 0x38]
        );
    }

    #[test]
    fn advertising_decoder_requires_valid_crc() {
        let channel = BleChannel::new(37).unwrap();
        let mut pdu = vec![0x00, 0x06, 1, 2, 3, 4, 5, 6];
        pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);

        let mut bits = vec![false, true, false];
        bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
        bits.extend(body);

        let packets = decode_primary_advertising(&bits, channel, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload, [1, 2, 3, 4, 5, 6]);

        let last = bits.len() - 1;
        bits[last] = !bits[last];
        assert!(
            decode_primary_advertising(&bits, channel, 0)
                .unwrap()
                .is_empty()
        );
    }
}
