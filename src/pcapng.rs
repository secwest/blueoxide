use crate::ble::LE_ADV_ACCESS_ADDRESS;
use crate::demod::{LeUncodedPhy, ReceivedAdvertisingPdu, ReceivedLePdu};
use crate::{Error, Result};
use std::io::Write;

const SECTION_HEADER_BLOCK: u32 = 0x0a0d_0d0a;
const INTERFACE_DESCRIPTION_BLOCK: u32 = 0x0000_0001;
const ENHANCED_PACKET_BLOCK: u32 = 0x0000_0006;
const BYTE_ORDER_MAGIC: u32 = 0x1a2b_3c4d;
const LINKTYPE_BLUETOOTH_LE_LL_WITH_PHDR: u16 = 256;
const PCAPNG_NANOSECOND_RESOLUTION: u8 = 9;

const BLE_DEWHITENED: u16 = 0x0001;
const BLE_REFERENCE_ACCESS_ADDRESS_VALID: u16 = 0x0010;
const BLE_ACCESS_ADDRESS_OFFENSES_VALID: u16 = 0x0020;
const BLE_CRC_CHECKED: u16 = 0x0400;
const BLE_CRC_VALID: u16 = 0x0800;
const BLE_PHY_LE_2M: u16 = 0x4000;

pub struct PcapNgWriter<W: Write> {
    writer: W,
}

impl<W: Write> PcapNgWriter<W> {
    pub fn new(mut writer: W) -> Result<Self> {
        let mut section = Vec::with_capacity(16);
        section.extend_from_slice(&BYTE_ORDER_MAGIC.to_le_bytes());
        section.extend_from_slice(&1u16.to_le_bytes());
        section.extend_from_slice(&0u16.to_le_bytes());
        section.extend_from_slice(&u64::MAX.to_le_bytes());
        write_block(&mut writer, SECTION_HEADER_BLOCK, &section)?;

        let mut interface = Vec::new();
        interface.extend_from_slice(&LINKTYPE_BLUETOOTH_LE_LL_WITH_PHDR.to_le_bytes());
        interface.extend_from_slice(&0u16.to_le_bytes());
        interface.extend_from_slice(&65_535u32.to_le_bytes());
        interface.extend_from_slice(&9u16.to_le_bytes());
        interface.extend_from_slice(&1u16.to_le_bytes());
        interface.push(PCAPNG_NANOSECOND_RESOLUTION);
        interface.extend_from_slice(&[0; 3]);
        interface.extend_from_slice(&0u16.to_le_bytes());
        interface.extend_from_slice(&0u16.to_le_bytes());
        write_block(&mut writer, INTERFACE_DESCRIPTION_BLOCK, &interface)?;

        Ok(Self { writer })
    }

    pub fn write_advertising(
        &mut self,
        packet: &ReceivedAdvertisingPdu,
        timestamp_ns: u64,
    ) -> Result<()> {
        let phy_flags = match packet.phy {
            LeUncodedPhy::Le1M => 0,
            LeUncodedPhy::Le2M => BLE_PHY_LE_2M,
        };
        self.write_packet(
            packet.pdu.channel.index(),
            packet.pdu.access_address_errors,
            LE_ADV_ACCESS_ADDRESS,
            &packet.pdu.link_layer_bytes(),
            phy_flags,
            timestamp_ns,
        )
    }

    pub fn write_le(&mut self, packet: &ReceivedLePdu, timestamp_ns: u64) -> Result<()> {
        let phy_flags = match packet.phy {
            LeUncodedPhy::Le1M => 0,
            LeUncodedPhy::Le2M => BLE_PHY_LE_2M,
        };
        self.write_packet(
            packet.pdu.channel.index(),
            packet.pdu.access_address_errors,
            packet.pdu.access_address,
            &packet.pdu.link_layer_bytes(),
            phy_flags,
            timestamp_ns,
        )
    }

    fn write_packet(
        &mut self,
        channel: u8,
        access_address_errors: u8,
        access_address: u32,
        link_layer_bytes: &[u8],
        phy_flags: u16,
        timestamp_ns: u64,
    ) -> Result<()> {
        let mut captured = Vec::with_capacity(10 + link_layer_bytes.len());
        captured.push(channel);
        captured.push(0);
        captured.push(0);
        captured.push(access_address_errors);
        captured.extend_from_slice(&access_address.to_le_bytes());
        let flags = BLE_DEWHITENED
            | BLE_REFERENCE_ACCESS_ADDRESS_VALID
            | BLE_ACCESS_ADDRESS_OFFENSES_VALID
            | BLE_CRC_CHECKED
            | BLE_CRC_VALID
            | phy_flags;
        captured.extend_from_slice(&flags.to_le_bytes());
        captured.extend_from_slice(link_layer_bytes);

        let mut body = Vec::with_capacity(20 + captured.len() + 3);
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&((timestamp_ns >> 32) as u32).to_le_bytes());
        body.extend_from_slice(&(timestamp_ns as u32).to_le_bytes());
        body.extend_from_slice(&(captured.len() as u32).to_le_bytes());
        body.extend_from_slice(&(captured.len() as u32).to_le_bytes());
        body.extend_from_slice(&captured);
        body.resize(body.len().next_multiple_of(4), 0);
        write_block(&mut self.writer, ENHANCED_PACKET_BLOCK, &body)
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

pub fn sample_timestamp_ns(
    capture_start_ns: u64,
    sample_index: u64,
    sample_rate_hz: u32,
) -> Result<u64> {
    if sample_rate_hz == 0 {
        return Err(Error::InvalidConfiguration(
            "sample rate must be greater than zero".to_owned(),
        ));
    }
    let elapsed = (sample_index as u128)
        .checked_mul(1_000_000_000)
        .ok_or_else(|| Error::InvalidInput("sample timestamp overflow".to_owned()))?
        / sample_rate_hz as u128;
    let timestamp = capture_start_ns as u128 + elapsed;
    u64::try_from(timestamp)
        .map_err(|_| Error::InvalidInput("sample timestamp overflow".to_owned()))
}

fn write_block(writer: &mut impl Write, block_type: u32, body: &[u8]) -> Result<()> {
    if !body.len().is_multiple_of(4) {
        return Err(Error::InvalidInput(
            "PCAPNG block body is not 32-bit aligned".to_owned(),
        ));
    }
    let length = body
        .len()
        .checked_add(12)
        .ok_or_else(|| Error::InvalidInput("PCAPNG block length overflow".to_owned()))?;
    let total_length = u32::try_from(length)
        .map_err(|_| Error::InvalidInput("PCAPNG block exceeds 4 GiB".to_owned()))?;
    writer.write_all(&block_type.to_le_bytes())?;
    writer.write_all(&total_length.to_le_bytes())?;
    writer.write_all(body)?;
    writer.write_all(&total_length.to_le_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::{AdvertisingPdu, BleChannel};

    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn writes_standard_ble_pcapng_blocks() {
        let packet = ReceivedAdvertisingPdu {
            pdu: AdvertisingPdu {
                channel: BleChannel::new(37).unwrap(),
                bit_offset: 8,
                inverted: false,
                access_address_errors: 0,
                header: [0x00, 0x02],
                payload: vec![0xaa, 0xbb],
                crc: [1, 2, 3],
            },
            phy: LeUncodedPhy::Le1M,
            access_address_sample: 400,
            symbol_phase: 0,
            estimated_carrier_offset_hz: 0.0,
            estimated_deviation_hz: 250_000.0,
            discriminator_separation: 1.0,
        };
        let mut writer = PcapNgWriter::new(Vec::new()).unwrap();
        writer.write_advertising(&packet, 123_456_789).unwrap();
        let bytes = writer.into_inner();

        assert_eq!(read_u32(&bytes, 0), SECTION_HEADER_BLOCK);
        let shb_length = read_u32(&bytes, 4) as usize;
        assert_eq!(read_u32(&bytes, shb_length), INTERFACE_DESCRIPTION_BLOCK);
        assert_eq!(read_u16(&bytes, shb_length + 8), 256);
        let idb_length = read_u32(&bytes, shb_length + 4) as usize;
        let epb = shb_length + idb_length;
        assert_eq!(read_u32(&bytes, epb), ENHANCED_PACKET_BLOCK);
        let epb_length = read_u32(&bytes, epb + 4) as usize;
        assert_eq!(read_u32(&bytes, epb + epb_length - 4) as usize, epb_length);

        let captured = epb + 28;
        assert_eq!(bytes[captured], 37);
        assert_eq!(read_u32(&bytes, captured + 4), LE_ADV_ACCESS_ADDRESS);
        assert_eq!(
            read_u16(&bytes, captured + 8),
            BLE_DEWHITENED
                | BLE_REFERENCE_ACCESS_ADDRESS_VALID
                | BLE_ACCESS_ADDRESS_OFFENSES_VALID
                | BLE_CRC_CHECKED
                | BLE_CRC_VALID
        );
        assert_eq!(
            &bytes[captured + 10..captured + 14],
            &LE_ADV_ACCESS_ADDRESS.to_le_bytes()
        );
    }

    #[test]
    fn converts_sample_index_to_nanoseconds_without_float_rounding() {
        assert_eq!(sample_timestamp_ns(10, 4, 4_000_000).unwrap(), 1_010);
        assert!(sample_timestamp_ns(0, 1, 0).is_err());
    }
}
