use crate::ble::{AdvertisingPdu, BleChannel, decode_primary_advertising};
use crate::complex::Complex32;
use crate::{Error, Result};

pub const LE_1M_SYMBOL_RATE: u32 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct Le1mDemodConfig {
    pub sample_rate_hz: u32,
    pub max_access_address_errors: u8,
}

impl Le1mDemodConfig {
    pub fn validate(self) -> Result<usize> {
        if !self.sample_rate_hz.is_multiple_of(LE_1M_SYMBOL_RATE) {
            return Err(Error::InvalidConfiguration(format!(
                "LE 1M currently requires a sample rate that is an integer multiple of {LE_1M_SYMBOL_RATE} Hz"
            )));
        }
        let samples_per_symbol = (self.sample_rate_hz / LE_1M_SYMBOL_RATE) as usize;
        if !(2..=64).contains(&samples_per_symbol) {
            return Err(Error::InvalidConfiguration(
                "LE 1M samples per symbol must be in 2..=64".to_owned(),
            ));
        }
        if self.max_access_address_errors > 8 {
            return Err(Error::InvalidConfiguration(
                "access-address error tolerance must be 0..=8".to_owned(),
            ));
        }
        Ok(samples_per_symbol)
    }
}

/// Converts complex baseband samples into instantaneous phase differences.
pub fn quadrature_discriminator(samples: &[Complex32]) -> Vec<f32> {
    samples
        .windows(2)
        .map(|pair| pair[0].phase_difference(pair[1]))
        .collect()
}

fn symbol_averages(discriminator: &[f32], phase: usize, samples_per_symbol: usize) -> Vec<f32> {
    discriminator[phase..]
        .chunks_exact(samples_per_symbol)
        .map(|symbol| symbol.iter().copied().sum::<f32>() / samples_per_symbol as f32)
        .collect()
}

fn robust_threshold(symbols: &[f32]) -> Option<f32> {
    let mut finite: Vec<f32> = symbols
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if finite.len() < 32 {
        return None;
    }
    finite.sort_unstable_by(f32::total_cmp);
    let low = finite[finite.len() / 5];
    let high = finite[finite.len() * 4 / 5];
    if high - low < 1.0e-6 {
        None
    } else {
        Some((low + high) * 0.5)
    }
}

/// Demodulates LE 1M primary advertising packets from oversampled complex I/Q.
///
/// Every integer symbol phase is evaluated. Spectrum inversion is handled by
/// the packet decoder, and only CRC-valid PDUs are returned.
pub fn decode_le_1m_advertising(
    samples: &[Complex32],
    channel: BleChannel,
    config: Le1mDemodConfig,
) -> Result<Vec<AdvertisingPdu>> {
    let samples_per_symbol = config.validate()?;
    if !channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "LE 1M advertising decoder requires channel 37, 38, or 39; got {}",
            channel.index()
        )));
    }
    if samples.len() < samples_per_symbol * 10 {
        return Ok(Vec::new());
    }

    let discriminator = quadrature_discriminator(samples);
    let mut packets = Vec::new();

    for phase in 0..samples_per_symbol {
        if phase >= discriminator.len() {
            break;
        }
        let symbols = symbol_averages(&discriminator, phase, samples_per_symbol);
        let Some(threshold) = robust_threshold(&symbols) else {
            continue;
        };
        let bits: Vec<bool> = symbols.iter().map(|value| *value >= threshold).collect();
        for packet in decode_primary_advertising(&bits, channel, config.max_access_address_errors)?
        {
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
    use crate::ble::{
        LE_ADV_ACCESS_ADDRESS, LE_ADV_CRC_INIT, bytes_to_bits_lsb, crc24_bytes, whiten_bits,
    };
    use std::f32::consts::TAU;

    fn modulate(bits: &[bool], samples_per_symbol: usize, offset_hz: f32) -> Vec<Complex32> {
        let sample_rate = samples_per_symbol as f32 * LE_1M_SYMBOL_RATE as f32;
        let mut phase = 0.0f32;
        let mut samples = vec![Complex32::new(1.0, 0.0); 7];
        for bit in bits {
            let deviation = if *bit { 250_000.0 } else { -250_000.0 };
            let step = TAU * (deviation + offset_hz) / sample_rate;
            for _ in 0..samples_per_symbol {
                phase += step;
                samples.push(Complex32::new(phase.cos(), phase.sin()));
            }
        }
        samples
    }

    #[test]
    fn demodulates_oversampled_advertisement_with_offset() {
        let channel = BleChannel::new(38).unwrap();
        let payload = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x02, 0x01, 0x06];
        let mut pdu = vec![0x00, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);

        let mut bits = bytes_to_bits_lsb(&[0xaa]);
        bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
        bits.extend(body);
        let samples = modulate(&bits, 4, 35_000.0);

        let packets = decode_le_1m_advertising(
            &samples,
            channel,
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
        )
        .unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload, payload);
    }

    #[test]
    fn rejects_non_integral_oversampling() {
        let error = Le1mDemodConfig {
            sample_rate_hz: 2_400_000,
            max_access_address_errors: 0,
        }
        .validate()
        .unwrap_err();
        assert!(error.to_string().contains("integer multiple"));
    }
}
