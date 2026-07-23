use crate::{Error, Result};

pub const LE_ADV_ACCESS_ADDRESS: u32 = 0x8e89_bed6;
pub const LE_ADV_CRC_INIT: u32 = 0x55_55_55;
pub const LE_PRIMARY_ADV_MAX_PAYLOAD: usize = 37;
pub const LE_SECONDARY_ADV_MAX_PAYLOAD: usize = 255;
pub const LE_DATA_MAX_PAYLOAD: usize = 255;

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
    pub access_address: u32,
    pub bit_offset: usize,
    pub inverted: bool,
    pub access_address_errors: u8,
    pub header: [u8; 2],
    pub payload: Vec<u8>,
    pub crc: [u8; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LePduLayout {
    Advertising,
    SecondaryAdvertising,
    Data,
}

impl LePduLayout {
    const fn maximum_payload_length(self) -> usize {
        match self {
            Self::Advertising => LE_PRIMARY_ADV_MAX_PAYLOAD,
            Self::SecondaryAdvertising => LE_SECONDARY_ADV_MAX_PAYLOAD,
            Self::Data => LE_DATA_MAX_PAYLOAD,
        }
    }

    const fn additional_header_length(self, header: [u8; 2]) -> usize {
        match self {
            Self::Advertising | Self::SecondaryAdvertising => 0,
            Self::Data => {
                if header[0] & 0x20 != 0 {
                    1
                } else {
                    0
                }
            }
        }
    }

    const fn maximum_additional_header_length(self) -> usize {
        match self {
            Self::Advertising | Self::SecondaryAdvertising => 0,
            Self::Data => 1,
        }
    }

    const fn payload_length(self, header: [u8; 2]) -> usize {
        match self {
            Self::Advertising => (header[1] & 0x3f) as usize,
            Self::SecondaryAdvertising | Self::Data => header[1] as usize,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeFrameConfig {
    pub access_address: u32,
    pub crc_init: u32,
    pub layout: LePduLayout,
}

impl LeFrameConfig {
    pub const fn advertising() -> Self {
        Self {
            access_address: LE_ADV_ACCESS_ADDRESS,
            crc_init: LE_ADV_CRC_INIT,
            layout: LePduLayout::Advertising,
        }
    }

    pub const fn secondary_advertising() -> Self {
        Self {
            access_address: LE_ADV_ACCESS_ADDRESS,
            crc_init: LE_ADV_CRC_INIT,
            layout: LePduLayout::SecondaryAdvertising,
        }
    }

    pub fn periodic_advertising(access_address: u32, crc_init: u32) -> Result<Self> {
        let config = Self {
            access_address,
            crc_init,
            layout: LePduLayout::SecondaryAdvertising,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn data(access_address: u32, crc_init: u32) -> Result<Self> {
        let config = Self {
            access_address,
            crc_init,
            layout: LePduLayout::Data,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(self) -> Result<()> {
        if self.crc_init > 0x00ff_ffff {
            return Err(Error::InvalidConfiguration(format!(
                "LE CRC initialization 0x{:x} exceeds 24 bits",
                self.crc_init
            )));
        }
        Ok(())
    }

    pub const fn maximum_frame_bits(self) -> usize {
        32 + (2
            + self.layout.maximum_additional_header_length()
            + self.layout.maximum_payload_length()
            + 3)
            * 8
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LePdu {
    pub channel: BleChannel,
    pub access_address: u32,
    pub bit_offset: usize,
    pub inverted: bool,
    pub access_address_errors: u8,
    pub header: [u8; 2],
    /// Raw data-channel CTEInfo octet when the header's CP bit is set.
    pub cte_info: Option<u8>,
    /// Data-channel Payload and optional MIC, as counted by the Length octet.
    pub payload: Vec<u8>,
    pub crc: [u8; 3],
}

impl LePdu {
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

    pub const fn frame_bit_length(&self) -> usize {
        let cte_info_length = if self.cte_info.is_some() { 1 } else { 0 };
        32 + (2 + cte_info_length + self.payload.len() + 3) * 8
    }
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

    pub fn link_layer_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + 2 + self.payload.len() + 3);
        bytes.extend_from_slice(&self.access_address.to_le_bytes());
        bytes.extend_from_slice(&self.header);
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&self.crc);
        bytes
    }
}

impl TryFrom<LePdu> for AdvertisingPdu {
    type Error = Error;

    fn try_from(packet: LePdu) -> Result<Self> {
        if packet.access_address != LE_ADV_ACCESS_ADDRESS {
            return Err(Error::InvalidInput(format!(
                "advertising PDU requires access address 0x{LE_ADV_ACCESS_ADDRESS:08x}, received 0x{:08x}",
                packet.access_address
            )));
        }
        Self::from_le_pdu(packet)
    }
}

impl AdvertisingPdu {
    pub fn from_le_pdu(packet: LePdu) -> Result<Self> {
        if packet.cte_info.is_some() {
            return Err(Error::InvalidInput(
                "advertising PDU cannot contain a data-channel CTEInfo header".to_owned(),
            ));
        }
        Ok(Self {
            channel: packet.channel,
            access_address: packet.access_address,
            bit_offset: packet.bit_offset,
            inverted: packet.inverted,
            access_address_errors: packet.access_address_errors,
            header: packet.header,
            payload: packet.payload,
            crc: packet.crc,
        })
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

/// Finds CRC-valid LE packets in a hard-decision bit stream.
///
/// The caller selects advertising or data-channel header length semantics.
/// Both normal and inverted spectra are checked. Access-address errors may be
/// tolerated, but every returned packet has a valid CRC.
pub fn decode_le_frames(
    bits: &[bool],
    channel: BleChannel,
    frame_config: LeFrameConfig,
    max_access_address_errors: u8,
) -> Result<Vec<LePdu>> {
    frame_config.validate()?;
    if max_access_address_errors > 8 {
        return Err(Error::InvalidConfiguration(
            "access-address error tolerance must be 0..=8".to_owned(),
        ));
    }

    let access_bits = bytes_to_bits_lsb(&frame_config.access_address.to_le_bytes());
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
            let maximum_body_bits = (2
                + frame_config.layout.maximum_additional_header_length()
                + frame_config.layout.maximum_payload_length()
                + 3)
                * 8;
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

            let header = [body[0], body[1]];
            let payload_length = frame_config.layout.payload_length(header);
            if payload_length > frame_config.layout.maximum_payload_length() {
                continue;
            }
            let additional_header_length = frame_config.layout.additional_header_length(header);
            let payload_start = 2 + additional_header_length;
            let pdu_length = payload_start + payload_length;
            let total_length = pdu_length + 3;
            if body.len() < total_length {
                continue;
            }

            let received_crc = [body[pdu_length], body[pdu_length + 1], body[pdu_length + 2]];
            if crc24_bytes(&body[..pdu_length], frame_config.crc_init) != received_crc {
                continue;
            }

            packets.push(LePdu {
                channel,
                access_address: frame_config.access_address,
                bit_offset: offset,
                inverted,
                access_address_errors: errors,
                header,
                cte_info: (additional_header_length != 0).then(|| body[2]),
                payload: body[payload_start..pdu_length].to_vec(),
                crc: received_crc,
            });
        }
    }
    Ok(packets)
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
    Ok(decode_le_frames(
        bits,
        channel,
        LeFrameConfig::advertising(),
        max_access_address_errors,
    )?
    .into_iter()
    .map(|packet| AdvertisingPdu {
        channel: packet.channel,
        access_address: packet.access_address,
        bit_offset: packet.bit_offset,
        inverted: packet.inverted,
        access_address_errors: packet.access_address_errors,
        header: packet.header,
        payload: {
            debug_assert!(packet.cte_info.is_none());
            packet.payload
        },
        crc: packet.crc,
    })
    .collect())
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
        assert_eq!(
            crc24_bytes(&[0x00, 0x06, 1, 2, 3, 4, 5, 6], LE_ADV_CRC_INIT),
            [0x42, 0xf5, 0xf2]
        );
        assert_eq!(
            crc24_bytes(
                &[
                    0x05, 0x00, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
                    0xdd, 0xee, 0xff
                ],
                LE_ADV_CRC_INIT
            ),
            [0x0c, 0xc8, 0x32]
        );
    }

    #[test]
    fn whitening_matches_independent_phy_vectors() {
        let input = [
            0x00, 0x06, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x1d, 0xb5, 0x38,
        ];
        for (channel, expected) in [
            (
                0,
                [
                    0x40, 0xb4, 0xbd, 0xc1, 0x1c, 0x33, 0x4f, 0x59, 0x98, 0x43, 0xa4,
                ],
            ),
            (
                37,
                [
                    0x8d, 0xd4, 0x56, 0xa3, 0x3e, 0xa3, 0x63, 0xb6, 0x68, 0x84, 0x29,
                ],
            ),
            (
                38,
                [
                    0xd6, 0xc3, 0x45, 0x22, 0x5a, 0xda, 0xe4, 0x89, 0x06, 0x10, 0x97,
                ],
            ),
            (
                39,
                [
                    0x1f, 0x31, 0x4b, 0x5d, 0x86, 0xf2, 0x99, 0x9c, 0xdc, 0x63, 0xfd,
                ],
            ),
        ] {
            let mut bits = bytes_to_bits_lsb(&input);
            whiten_bits(&mut bits, BleChannel::new(channel).unwrap());
            assert_eq!(bits_to_bytes_lsb(&bits), expected, "channel {channel}");
        }
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

    #[test]
    fn generic_decoder_uses_full_data_length_octet() {
        let channel = BleChannel::new(12).unwrap();
        let config = LeFrameConfig::data(0x1234_5678, 0x00ab_cdef).unwrap();
        let payload: Vec<u8> = (0..200).map(|value| value as u8).collect();
        let mut pdu = vec![0x1d, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, config.crc_init));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);

        let mut bits = vec![true, false, true, false];
        bits.extend(bytes_to_bits_lsb(&config.access_address.to_le_bytes()));
        bits.extend(body);

        let packets = decode_le_frames(&bits, channel, config, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].header, [0x1d, 200]);
        assert_eq!(packets[0].payload, payload);
        assert_eq!(packets[0].bit_offset, 4);
    }

    #[test]
    fn secondary_advertising_decoder_uses_full_length_octet() {
        let channel = BleChannel::new(20).unwrap();
        let config = LeFrameConfig::secondary_advertising();
        let payload: Vec<u8> = (0..200).map(|value| value as u8).collect();
        let mut pdu = vec![0x07, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, config.crc_init));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);

        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);

        let packets = decode_le_frames(&bits, channel, config, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].header, [0x07, 200]);
        assert_eq!(packets[0].payload, payload);
        assert_eq!(
            AdvertisingPdu::try_from(packets[0].clone())
                .unwrap()
                .payload,
            payload
        );
        assert!(
            decode_le_frames(&bits, channel, LeFrameConfig::advertising(), 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn advertising_conversion_rejects_data_framing() {
        let packet = LePdu {
            channel: BleChannel::new(0).unwrap(),
            access_address: 0x1234_5678,
            bit_offset: 0,
            inverted: false,
            access_address_errors: 0,
            header: [0x02, 0],
            cte_info: None,
            payload: Vec::new(),
            crc: [0; 3],
        };
        assert!(AdvertisingPdu::try_from(packet).is_err());
    }

    #[test]
    fn periodic_advertising_conversion_retains_custom_access_address() {
        let packet = LePdu {
            channel: BleChannel::new(27).unwrap(),
            access_address: 0x1234_5678,
            bit_offset: 32,
            inverted: true,
            access_address_errors: 1,
            header: [0x07, 2],
            cte_info: None,
            payload: vec![0xaa, 0xbb],
            crc: [0xef, 0xcd, 0xab],
        };
        assert!(AdvertisingPdu::try_from(packet.clone()).is_err());

        let advertising = AdvertisingPdu::from_le_pdu(packet).unwrap();
        assert_eq!(advertising.access_address, 0x1234_5678);
        assert_eq!(
            advertising.link_layer_bytes(),
            [
                0x78, 0x56, 0x34, 0x12, 0x07, 0x02, 0xaa, 0xbb, 0xef, 0xcd, 0xab
            ]
        );
    }

    #[test]
    fn generic_decoder_handles_inverted_data_packet() {
        let channel = BleChannel::new(0).unwrap();
        let config = LeFrameConfig::data(0x89ab_cdef, 0x0012_3456).unwrap();
        let mut pdu = vec![0x02, 3, 0xaa, 0x55, 0x19];
        pdu.extend_from_slice(&crc24_bytes(&pdu, config.crc_init));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);
        for bit in &mut bits {
            *bit = !*bit;
        }

        let packets = decode_le_frames(&bits, channel, config, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert!(packets[0].inverted);
        assert_eq!(packets[0].payload, [0xaa, 0x55, 0x19]);
    }

    #[test]
    fn data_decoder_separates_cte_info_from_declared_payload() {
        let channel = BleChannel::new(9).unwrap();
        let config = LeFrameConfig::data(0x1234_5678, 0x00ab_cdef).unwrap();
        let payload = [0x11, 0x22, 0x33];
        let cte_info = 0x85;
        let mut pdu = vec![0x22, payload.len() as u8, cte_info];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&[0x27, 0xe2, 0xcf]);
        let expected_link_layer_bytes = [
            config.access_address.to_le_bytes().as_slice(),
            pdu.as_slice(),
        ]
        .concat();
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);

        let packets = decode_le_frames(&bits, channel, config, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].header, [0x22, 3]);
        assert_eq!(packets[0].cte_info, Some(cte_info));
        assert_eq!(packets[0].payload, payload);
        assert_eq!(packets[0].link_layer_bytes(), expected_link_layer_bytes);
        assert_eq!(packets[0].frame_bit_length(), (4 + pdu.len()) * 8);
    }

    #[test]
    fn data_decoder_accepts_cte_info_with_zero_declared_payload() {
        let channel = BleChannel::new(3).unwrap();
        let config = LeFrameConfig::data(0x89ab_cdef, 0x0012_3456).unwrap();
        let mut pdu = vec![0x21, 0, 0x42];
        pdu.extend_from_slice(&[0x7f, 0xd4, 0x6c]);
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);

        let packets = decode_le_frames(&bits, channel, config, 0).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].cte_info, Some(0x42));
        assert!(packets[0].payload.is_empty());
    }

    #[test]
    fn data_decoder_crc_covers_cte_info() {
        let channel = BleChannel::new(7).unwrap();
        let config = LeFrameConfig::data(0x1020_3040, 0x0055_aa33).unwrap();
        let mut pdu = vec![0x21, 1, 0x85, 0x19];
        let wrong_crc = crc24_bytes(&[0x21, 1, 0x19], config.crc_init);
        pdu.extend_from_slice(&wrong_crc);
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);

        assert!(
            decode_le_frames(&bits, channel, config, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn data_decoder_rejects_missing_cte_info_octet() {
        let channel = BleChannel::new(16).unwrap();
        let config = LeFrameConfig::data(0x1357_9bdf, 0x0024_68ac).unwrap();
        let mut truncated_body = vec![0x21, 0];
        truncated_body.extend_from_slice(&crc24_bytes(&truncated_body, config.crc_init));
        let mut body = bytes_to_bits_lsb(&truncated_body);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&config.access_address.to_le_bytes());
        bits.extend(body);

        assert!(
            decode_le_frames(&bits, channel, config, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn frame_config_rejects_wide_crc_init() {
        assert!(LeFrameConfig::data(1, 0x0100_0000).is_err());
    }
}
