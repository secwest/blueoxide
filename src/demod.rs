use crate::ble::{AdvertisingPdu, BleChannel, decode_primary_advertising};
use crate::complex::Complex32;
use crate::{Error, Result};

pub const LE_1M_SYMBOL_RATE: u32 = 1_000_000;
const MAX_PRIMARY_ADV_BODY_BITS: usize = (2 + 37 + 3) * 8;
const STREAM_THRESHOLD_CONTEXT_SYMBOLS: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct Le1mDemodConfig {
    pub sample_rate_hz: u32,
    pub max_access_address_errors: u8,
}

#[derive(Clone, Debug)]
pub struct ReceivedAdvertisingPdu {
    pub pdu: AdvertisingPdu,
    /// Sample index at the beginning of the detected access address.
    pub access_address_sample: u64,
    pub symbol_phase: usize,
    pub estimated_carrier_offset_hz: f32,
    pub estimated_deviation_hz: f32,
    pub discriminator_separation: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleDiscontinuity {
    pub expected_first_sample: u64,
    pub observed_first_sample: u64,
}

#[derive(Clone, Debug, Default)]
pub struct StreamDecodeBatch {
    pub packets: Vec<ReceivedAdvertisingPdu>,
    pub discontinuity: Option<SampleDiscontinuity>,
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

#[derive(Clone, Copy, Debug)]
struct SliceLevels {
    threshold: f32,
    low: f32,
    high: f32,
}

fn robust_threshold(symbols: &[f32]) -> Option<SliceLevels> {
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
        Some(SliceLevels {
            threshold: (low + high) * 0.5,
            low,
            high,
        })
    }
}

fn same_observation(
    left: &ReceivedAdvertisingPdu,
    right: &ReceivedAdvertisingPdu,
    samples_per_symbol: u64,
) -> bool {
    left.access_address_sample
        .abs_diff(right.access_address_sample)
        <= samples_per_symbol
        && left.pdu.header == right.pdu.header
        && left.pdu.payload == right.pdu.payload
        && left.pdu.crc == right.pdu.crc
}

fn packet_slice_levels(
    symbols: &[f32],
    sliced_bits: &[bool],
    start: usize,
    length: usize,
) -> Option<SliceLevels> {
    let end = start.checked_add(length)?;
    if end > symbols.len() || end > sliced_bits.len() {
        return None;
    }
    let mut low_sum = 0.0f64;
    let mut low_count = 0usize;
    let mut high_sum = 0.0f64;
    let mut high_count = 0usize;
    for (value, high) in symbols[start..end].iter().zip(&sliced_bits[start..end]) {
        if !value.is_finite() {
            continue;
        }
        if *high {
            high_sum += *value as f64;
            high_count += 1;
        } else {
            low_sum += *value as f64;
            low_count += 1;
        }
    }
    if low_count == 0 || high_count == 0 {
        return None;
    }
    let low = (low_sum / low_count as f64) as f32;
    let high = (high_sum / high_count as f64) as f32;
    Some(SliceLevels {
        threshold: (low + high) * 0.5,
        low,
        high,
    })
}

/// Demodulates detailed LE 1M primary advertising observations.
///
/// Every integer symbol phase is evaluated. Spectrum inversion is handled by
/// the packet decoder, and only CRC-valid PDUs are returned.
pub fn decode_le_1m_advertising_detailed(
    samples: &[Complex32],
    channel: BleChannel,
    config: Le1mDemodConfig,
) -> Result<Vec<ReceivedAdvertisingPdu>> {
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
        let Some(levels) = robust_threshold(&symbols) else {
            continue;
        };
        let bits: Vec<bool> = symbols
            .iter()
            .map(|value| *value >= levels.threshold)
            .collect();
        for packet in decode_primary_advertising(&bits, channel, config.max_access_address_errors)?
        {
            let packet_bits = 32 + (2 + packet.payload.len() + 3) * 8;
            let packet_levels =
                packet_slice_levels(&symbols, &bits, packet.bit_offset, packet_bits)
                    .unwrap_or(levels);
            let observation = ReceivedAdvertisingPdu {
                access_address_sample: (phase + 1 + packet.bit_offset * samples_per_symbol) as u64,
                symbol_phase: phase,
                estimated_carrier_offset_hz: packet_levels.threshold * config.sample_rate_hz as f32
                    / std::f32::consts::TAU,
                estimated_deviation_hz: (packet_levels.high - packet_levels.low)
                    * config.sample_rate_hz as f32
                    / (2.0 * std::f32::consts::TAU),
                discriminator_separation: packet_levels.high - packet_levels.low,
                pdu: packet,
            };
            if let Some(existing) = packets.iter_mut().find(|existing| {
                same_observation(existing, &observation, samples_per_symbol as u64)
            }) {
                if observation.discriminator_separation > existing.discriminator_separation {
                    *existing = observation;
                }
            } else {
                packets.push(observation);
            }
        }
    }
    packets.sort_unstable_by_key(|packet| packet.access_address_sample);
    Ok(packets)
}

pub fn decode_le_1m_advertising(
    samples: &[Complex32],
    channel: BleChannel,
    config: Le1mDemodConfig,
) -> Result<Vec<AdvertisingPdu>> {
    Ok(decode_le_1m_advertising_detailed(samples, channel, config)?
        .into_iter()
        .map(|packet| packet.pdu)
        .collect())
}

/// Bounded, discontinuity-aware wrapper around the LE 1M block demodulator.
///
/// Incoming sample indices must be monotonically contiguous. A gap or overlap
/// resets DSP history and is reported with the returned batch, preventing
/// packets from being assembled across a discontinuous radio stream.
pub struct Le1mStreamDecoder {
    channel: BleChannel,
    config: Le1mDemodConfig,
    samples_per_symbol: usize,
    samples: Vec<Complex32>,
    buffer_first_sample: Option<u64>,
    expected_next_sample: Option<u64>,
    recent_packets: Vec<ReceivedAdvertisingPdu>,
    maximum_buffer_samples: usize,
}

impl Le1mStreamDecoder {
    pub fn new(channel: BleChannel, config: Le1mDemodConfig) -> Result<Self> {
        let samples_per_symbol = config.validate()?;
        if !channel.is_primary_advertising() {
            return Err(Error::InvalidConfiguration(format!(
                "LE 1M stream decoder requires channel 37, 38, or 39; got {}",
                channel.index()
            )));
        }
        let maximum_buffer_samples =
            (32 + MAX_PRIMARY_ADV_BODY_BITS + STREAM_THRESHOLD_CONTEXT_SYMBOLS)
                * samples_per_symbol;
        Ok(Self {
            channel,
            config,
            samples_per_symbol,
            samples: Vec::with_capacity(maximum_buffer_samples),
            buffer_first_sample: None,
            expected_next_sample: None,
            recent_packets: Vec::new(),
            maximum_buffer_samples,
        })
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.buffer_first_sample = None;
        self.expected_next_sample = None;
        self.recent_packets.clear();
    }

    pub fn push(
        &mut self,
        first_sample_index: u64,
        input: &[Complex32],
    ) -> Result<StreamDecodeBatch> {
        let mut batch = StreamDecodeBatch::default();
        if input.is_empty() {
            return Ok(batch);
        }

        if let Some(expected) = self.expected_next_sample
            && expected != first_sample_index
        {
            batch.discontinuity = Some(SampleDiscontinuity {
                expected_first_sample: expected,
                observed_first_sample: first_sample_index,
            });
            self.samples.clear();
            self.recent_packets.clear();
            self.buffer_first_sample = None;
        }

        let final_sample = first_sample_index
            .checked_add(input.len() as u64)
            .ok_or_else(|| Error::InvalidInput("sample index overflow".to_owned()))?;
        let processing_stride = (self.maximum_buffer_samples / 2).max(1);
        let mut consumed = 0usize;

        for chunk in input.chunks(processing_stride) {
            let chunk_first = first_sample_index + consumed as u64;
            if self.samples.is_empty() {
                self.buffer_first_sample = Some(chunk_first);
            }
            self.samples.extend_from_slice(chunk);
            self.decode_buffer(&mut batch.packets)?;
            self.trim_buffer();
            consumed += chunk.len();
        }
        self.expected_next_sample = Some(final_sample);
        Ok(batch)
    }

    fn decode_buffer(&mut self, output: &mut Vec<ReceivedAdvertisingPdu>) -> Result<()> {
        let buffer_first = self.buffer_first_sample.ok_or_else(|| {
            Error::InvalidInput("stream decoder lost its buffer sample index".to_owned())
        })?;
        for mut observation in
            decode_le_1m_advertising_detailed(&self.samples, self.channel, self.config)?
        {
            observation.access_address_sample = observation
                .access_address_sample
                .checked_add(buffer_first)
                .ok_or_else(|| Error::InvalidInput("sample index overflow".to_owned()))?;
            if self.recent_packets.iter().any(|existing| {
                same_observation(existing, &observation, self.samples_per_symbol as u64)
            }) {
                continue;
            }
            self.recent_packets.push(observation.clone());
            output.push(observation);
        }
        Ok(())
    }

    fn trim_buffer(&mut self) {
        if self.samples.len() <= self.maximum_buffer_samples {
            return;
        }
        let remove = self.samples.len() - self.maximum_buffer_samples;
        self.samples.drain(..remove);
        if let Some(first) = &mut self.buffer_first_sample {
            *first += remove as u64;
            let retained_from = *first;
            self.recent_packets.retain(|packet| {
                packet
                    .access_address_sample
                    .saturating_add(self.maximum_buffer_samples as u64)
                    >= retained_from
            });
        }
    }
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

        let detailed = decode_le_1m_advertising_detailed(
            &samples,
            channel,
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
        )
        .unwrap();
        assert!((detailed[0].estimated_carrier_offset_hz - 35_000.0).abs() < 5_000.0);
        assert!((detailed[0].estimated_deviation_hz - 250_000.0).abs() < 5_000.0);
    }

    #[test]
    fn preserves_repeated_identical_advertisements() {
        let channel = BleChannel::new(37).unwrap();
        let payload = [1, 2, 3, 4, 5, 6];
        let mut pdu = vec![0x00, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);

        let mut packet_bits = bytes_to_bits_lsb(&[0xaa]);
        packet_bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
        packet_bits.extend(body);
        let mut bits = packet_bits.clone();
        bits.extend([false; 80]);
        bits.extend(packet_bits);
        let samples = modulate(&bits, 4, 0.0);

        let packets = decode_le_1m_advertising_detailed(
            &samples,
            channel,
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
        )
        .unwrap();
        assert_eq!(packets.len(), 2);
        assert!(packets[1].access_address_sample > packets[0].access_address_sample + 4);
    }

    #[test]
    fn stream_decoder_handles_packet_split_and_reports_gap() {
        let channel = BleChannel::new(39).unwrap();
        let payload = [6, 5, 4, 3, 2, 1, 2, 1, 6];
        let mut pdu = vec![0x00, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&[0xaa]);
        bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
        bits.extend(body);
        let samples = modulate(&bits, 4, 20_000.0);

        let config = Le1mDemodConfig {
            sample_rate_hz: 4_000_000,
            max_access_address_errors: 0,
        };
        let mut decoder = Le1mStreamDecoder::new(channel, config).unwrap();
        let split = samples.len() / 2;
        let first = decoder.push(10_000, &samples[..split]).unwrap();
        assert!(first.packets.is_empty());
        let second = decoder
            .push(10_000 + split as u64, &samples[split..])
            .unwrap();
        assert_eq!(second.packets.len(), 1);
        assert!(second.packets[0].access_address_sample >= 10_000);

        let gap = decoder.push(99_000, &[Complex32::ZERO; 256]).unwrap();
        assert_eq!(
            gap.discontinuity,
            Some(SampleDiscontinuity {
                expected_first_sample: 10_000 + samples.len() as u64,
                observed_first_sample: 99_000,
            })
        );
        assert!(gap.packets.is_empty());
    }

    #[test]
    fn demodulates_rate_offset_and_spectrum_inversion_matrix() {
        let channel = BleChannel::new(37).unwrap();
        let payload = [1, 2, 3, 4, 5, 6, 2, 1, 6];
        let mut pdu = vec![0x00, payload.len() as u8];
        pdu.extend_from_slice(&payload);
        pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&[0xaa]);
        bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
        bits.extend(body);

        for samples_per_symbol in [2, 4, 8, 16] {
            for offset_hz in [-100_000.0, 0.0, 100_000.0] {
                for inverted in [false, true] {
                    let mut samples = modulate(&bits, samples_per_symbol, offset_hz);
                    if inverted {
                        for sample in &mut samples {
                            sample.im = -sample.im;
                        }
                    }
                    let packets = decode_le_1m_advertising_detailed(
                        &samples,
                        channel,
                        Le1mDemodConfig {
                            sample_rate_hz: samples_per_symbol as u32 * LE_1M_SYMBOL_RATE,
                            max_access_address_errors: 0,
                        },
                    )
                    .unwrap();
                    assert_eq!(
                        packets.len(),
                        1,
                        "sps={samples_per_symbol} offset={offset_hz} inverted={inverted}"
                    );
                    assert_eq!(packets[0].pdu.payload, payload);
                    assert_eq!(packets[0].pdu.inverted, inverted);
                    let expected_offset = if inverted { -offset_hz } else { offset_hz };
                    assert!(
                        (packets[0].estimated_carrier_offset_hz - expected_offset).abs() < 10_000.0
                    );
                }
            }
        }
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
