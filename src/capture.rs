use crate::ble::BleChannel;
use crate::complex::Complex32;
use crate::demod::{
    Le1mDemodConfig, Le1mStreamDecoder, ReceivedAdvertisingPdu, SampleDiscontinuity,
};
use crate::sdr::{IqSource, SdrConfig};
use crate::{Error, Result};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    pub maximum_samples: Option<u64>,
    pub maximum_duration: Option<Duration>,
    pub read_timeout: Duration,
    pub block_samples: usize,
}

impl CaptureLimits {
    fn validate(self) -> Result<()> {
        if self.maximum_samples.is_none() && self.maximum_duration.is_none() {
            return Err(Error::InvalidConfiguration(
                "capture requires a sample or duration limit".to_owned(),
            ));
        }
        if self.maximum_samples == Some(0) {
            return Err(Error::InvalidConfiguration(
                "capture maximum_samples must be greater than zero".to_owned(),
            ));
        }
        if self.maximum_duration == Some(Duration::ZERO) {
            return Err(Error::InvalidConfiguration(
                "capture maximum_duration must be greater than zero".to_owned(),
            ));
        }
        if self.read_timeout == Duration::ZERO {
            return Err(Error::InvalidConfiguration(
                "capture read_timeout must be greater than zero".to_owned(),
            ));
        }
        if self.block_samples == 0 {
            return Err(Error::InvalidConfiguration(
                "capture block_samples must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CapturedAdvertisingPdu {
    pub observation: ReceivedAdvertisingPdu,
    pub relative_sample_index: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CaptureStats {
    pub samples_received: u64,
    pub packets_decoded: u64,
    pub dropped_samples: u64,
    pub overruns: u64,
    pub discontinuities: u64,
    pub first_hardware_sample: Option<u64>,
    pub last_hardware_sample: Option<u64>,
}

pub fn capture_primary_advertising<S, F>(
    source: &mut S,
    radio_config: &SdrConfig,
    ble_channel: BleChannel,
    demod_config: Le1mDemodConfig,
    limits: CaptureLimits,
    mut on_packet: F,
) -> Result<CaptureStats>
where
    S: IqSource,
    F: FnMut(&CapturedAdvertisingPdu) -> Result<()>,
{
    limits.validate()?;
    if radio_config.sample_rate_hz != demod_config.sample_rate_hz {
        return Err(Error::InvalidConfiguration(format!(
            "radio sample rate {} does not match demodulator sample rate {}",
            radio_config.sample_rate_hz, demod_config.sample_rate_hz
        )));
    }
    let decoder = Le1mStreamDecoder::new(ble_channel, demod_config)?;
    source.configure(radio_config)?;
    if let Some(applied_sample_rate_hz) = source.applied_sample_rate_hz()
        && applied_sample_rate_hz != demod_config.sample_rate_hz
    {
        return Err(Error::InvalidConfiguration(format!(
            "SDR applied sample rate {applied_sample_rate_hz} Hz does not match demodulator sample rate {} Hz",
            demod_config.sample_rate_hz
        )));
    }
    source.start()?;

    let capture_result = capture_loop(source, decoder, limits, &mut on_packet);
    let stop_result = source.stop();
    match (capture_result, stop_result) {
        (Ok(stats), Ok(())) => Ok(stats),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn capture_loop<S, F>(
    source: &mut S,
    mut decoder: Le1mStreamDecoder,
    limits: CaptureLimits,
    on_packet: &mut F,
) -> Result<CaptureStats>
where
    S: IqSource,
    F: FnMut(&CapturedAdvertisingPdu) -> Result<()>,
{
    let mut buffer = vec![Complex32::ZERO; limits.block_samples];
    let started = Instant::now();
    let mut stats = CaptureStats::default();

    loop {
        if limits
            .maximum_duration
            .is_some_and(|duration| started.elapsed() >= duration)
        {
            break;
        }
        let remaining = limits
            .maximum_samples
            .map(|maximum| maximum.saturating_sub(stats.samples_received))
            .unwrap_or(u64::MAX);
        if remaining == 0 {
            break;
        }
        let requested = buffer.len().min(remaining.min(usize::MAX as u64) as usize);
        let (count, metadata) = source.read(&mut buffer[..requested], limits.read_timeout)?;
        if count > requested {
            return Err(Error::InvalidInput(format!(
                "SDR backend returned {count} samples for a {requested}-sample buffer"
            )));
        }
        if count == 0 {
            continue;
        }

        stats.samples_received = stats
            .samples_received
            .checked_add(count as u64)
            .ok_or_else(|| Error::InvalidInput("capture sample count overflow".to_owned()))?;
        stats.dropped_samples = stats
            .dropped_samples
            .checked_add(metadata.dropped_samples_before)
            .ok_or_else(|| {
                Error::InvalidInput("capture dropped-sample count overflow".to_owned())
            })?;
        if metadata.overrun {
            stats.overruns += 1;
        }
        let first_hardware_sample = *stats
            .first_hardware_sample
            .get_or_insert(metadata.first_sample_index);
        stats.last_hardware_sample = metadata
            .first_sample_index
            .checked_add(count as u64)
            .and_then(|value| value.checked_sub(1));

        let batch = decoder.push(metadata.first_sample_index, &buffer[..count])?;
        if batch.discontinuity.is_some() {
            stats.discontinuities += 1;
        }
        for observation in batch.packets {
            let relative_sample_index = observation
                .access_address_sample
                .checked_sub(first_hardware_sample)
                .ok_or_else(|| {
                    Error::InvalidInput(format!(
                        "hardware sample counter moved before capture origin: {} < {}",
                        observation.access_address_sample, first_hardware_sample
                    ))
                })?;
            on_packet(&CapturedAdvertisingPdu {
                observation,
                relative_sample_index,
            })?;
            stats.packets_decoded += 1;
        }
    }
    Ok(stats)
}

pub fn describe_discontinuity(discontinuity: SampleDiscontinuity) -> String {
    if discontinuity.observed_first_sample >= discontinuity.expected_first_sample {
        format!(
            "{} sample(s) missing before hardware sample {}",
            discontinuity.observed_first_sample - discontinuity.expected_first_sample,
            discontinuity.observed_first_sample
        )
    } else {
        format!(
            "hardware sample counter moved backward from expected {} to {}",
            discontinuity.expected_first_sample, discontinuity.observed_first_sample
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ble::{
        LE_ADV_ACCESS_ADDRESS, LE_ADV_CRC_INIT, bytes_to_bits_lsb, crc24_bytes, whiten_bits,
    };
    use crate::sdr::{ReadMetadata, SdrCapabilities, SdrKind};
    use std::collections::VecDeque;
    use std::f32::consts::TAU;

    struct MockSource {
        blocks: VecDeque<(u64, Vec<Complex32>, ReadMetadata)>,
        configured: bool,
        running: bool,
        stopped: bool,
        applied_sample_rate_hz: Option<u32>,
    }

    impl IqSource for MockSource {
        fn kind(&self) -> SdrKind {
            SdrKind::BladeRf
        }

        fn capabilities(&self) -> SdrCapabilities {
            SdrCapabilities {
                minimum_frequency_hz: 1,
                maximum_frequency_hz: u64::MAX,
                maximum_sample_rate_hz: u32::MAX,
                receive_channels: 1,
            }
        }

        fn configure(&mut self, _config: &SdrConfig) -> Result<()> {
            self.configured = true;
            Ok(())
        }

        fn applied_sample_rate_hz(&self) -> Option<u32> {
            self.applied_sample_rate_hz
        }

        fn start(&mut self) -> Result<()> {
            assert!(self.configured);
            self.running = true;
            Ok(())
        }

        fn read(
            &mut self,
            output: &mut [Complex32],
            _timeout: Duration,
        ) -> Result<(usize, ReadMetadata)> {
            assert!(self.running);
            let (_, samples, metadata) = self.blocks.pop_front().ok_or_else(|| {
                Error::InvalidInput("mock source exhausted unexpectedly".to_owned())
            })?;
            let count = samples.len();
            output[..count].copy_from_slice(&samples);
            Ok((count, metadata))
        }

        fn stop(&mut self) -> Result<()> {
            self.running = false;
            self.stopped = true;
            Ok(())
        }
    }

    fn modulated_advertisement() -> Vec<Complex32> {
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

        let mut phase = 0.0f32;
        let mut samples = vec![Complex32::new(1.0, 0.0); 7];
        for bit in bits {
            let frequency = if bit { 250_000.0 } else { -250_000.0 };
            let step = TAU * frequency / 4_000_000.0;
            for _ in 0..4 {
                phase += step;
                samples.push(Complex32::new(phase.cos(), phase.sin()));
            }
        }
        samples
    }

    #[test]
    fn captures_across_backend_blocks_and_always_stops() {
        let samples = modulated_advertisement();
        let split = samples.len() / 3;
        let chunks = [
            samples[..split].to_vec(),
            samples[split..split * 2].to_vec(),
            samples[split * 2..].to_vec(),
        ];
        let mut next = 50_000u64;
        let mut blocks = VecDeque::new();
        for chunk in chunks {
            let metadata = ReadMetadata {
                first_sample_index: next,
                dropped_samples_before: 0,
                overrun: false,
            };
            next += chunk.len() as u64;
            blocks.push_back((metadata.first_sample_index, chunk, metadata));
        }
        let total_samples = samples.len() as u64;
        let mut source = MockSource {
            blocks,
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: None,
        };
        let mut packets = Vec::new();
        let stats = capture_primary_advertising(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_402_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(37).unwrap(),
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(total_samples),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: samples.len(),
            },
            |packet| {
                packets.push(packet.clone());
                Ok(())
            },
        )
        .unwrap();
        assert!(source.stopped);
        assert_eq!(stats.samples_received, total_samples);
        assert_eq!(stats.packets_decoded, 1);
        assert_eq!(packets.len(), 1);
        assert!(packets[0].relative_sample_index < total_samples);
    }

    #[test]
    fn stops_source_when_packet_callback_fails() {
        let samples = modulated_advertisement();
        let total_samples = samples.len() as u64;
        let mut blocks = VecDeque::new();
        blocks.push_back((
            0,
            samples,
            ReadMetadata {
                first_sample_index: 0,
                dropped_samples_before: 0,
                overrun: false,
            },
        ));
        let mut source = MockSource {
            blocks,
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: None,
        };
        let result = capture_primary_advertising(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_402_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(37).unwrap(),
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(total_samples),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: total_samples as usize,
            },
            |_| Err(Error::InvalidInput("callback failure".to_owned())),
        );
        assert!(result.unwrap_err().to_string().contains("callback failure"));
        assert!(source.stopped);
    }

    #[test]
    fn validates_decoder_before_configuring_source() {
        let mut source = MockSource {
            blocks: VecDeque::new(),
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: None,
        };
        let result = capture_primary_advertising(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_404_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(0).unwrap(),
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(1),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: 1,
            },
            |_| Ok(()),
        );
        assert!(result.unwrap_err().to_string().contains("37, 38, or 39"));
        assert!(!source.configured);
        assert!(!source.running);
    }

    #[test]
    fn rejects_applied_sample_rate_mismatch_before_start() {
        let mut source = MockSource {
            blocks: VecDeque::new(),
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: Some(3_999_999),
        };
        let result = capture_primary_advertising(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_402_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(37).unwrap(),
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(1),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: 1,
            },
            |_| Ok(()),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("applied sample rate")
        );
        assert!(source.configured);
        assert!(!source.running);
    }

    #[test]
    fn stops_source_when_read_fails() {
        let mut source = MockSource {
            blocks: VecDeque::new(),
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: None,
        };
        let result = capture_primary_advertising(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_402_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(37).unwrap(),
            Le1mDemodConfig {
                sample_rate_hz: 4_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(1),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: 1,
            },
            |_| Ok(()),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("mock source exhausted")
        );
        assert!(source.stopped);
    }
}
