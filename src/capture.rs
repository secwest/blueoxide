use crate::ble::{BleChannel, LeFrameConfig, LePduLayout};
use crate::complex::Complex32;
use crate::demod::{
    Le1mDemodConfig, Le1mStreamDecoder, LeUncodedDemodConfig, LeUncodedPacketStreamDecoder,
    ReceivedAdvertisingPdu, ReceivedLePdu, SampleDiscontinuity,
};
use crate::link_layer::{
    ConnectionObservation, ConnectionTracker, ConnectionTrackerConfig, SampleTimingError,
    SleepClockAccuracy,
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

#[derive(Clone, Debug)]
pub struct CapturedDataChannelPdu {
    pub observation: ReceivedLePdu,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedChannelCentralObservationConfig {
    pub tracker: ConnectionTrackerConfig,
    pub first_event_counter: u16,
    pub peer_clock_accuracy: SleepClockAccuracy,
    pub receiver_clock_accuracy_ppm: u32,
    pub maximum_event_advance: u16,
}

impl FixedChannelCentralObservationConfig {
    pub fn validate(&self, capture_channel: BleChannel) -> Result<()> {
        if capture_channel.index() > 36 {
            return Err(Error::InvalidConfiguration(format!(
                "fixed-channel connection observations require a data channel in 0..=36; got {}",
                capture_channel.index()
            )));
        }
        let probe = ConnectionTracker::new(self.tracker.clone(), self.first_event_counter, 0)?;
        let first_event = probe.current_event()?;
        if first_event.channel != capture_channel {
            return Err(Error::InvalidConfiguration(format!(
                "asserted first central event {} uses channel {}, not tuned channel {}",
                self.first_event_counter,
                first_event.channel.index(),
                capture_channel.index()
            )));
        }
        probe
            .current_timing_window(self.peer_clock_accuracy, self.receiver_clock_accuracy_ppm)?
            .ok_or_else(|| {
                Error::InvalidConfiguration(
                    "fixed-channel central observation tracker requires a known timing anchor"
                        .to_owned(),
                )
            })?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FixedChannelCentralObservationTracker {
    capture_channel: BleChannel,
    config: FixedChannelCentralObservationConfig,
    tracker: Option<ConnectionTracker>,
}

impl FixedChannelCentralObservationTracker {
    pub fn new(
        capture_channel: BleChannel,
        config: FixedChannelCentralObservationConfig,
    ) -> Result<Self> {
        config.validate(capture_channel)?;
        Ok(Self {
            capture_channel,
            config,
            tracker: None,
        })
    }

    pub const fn is_anchored(&self) -> bool {
        self.tracker.is_some()
    }

    /// Associates a caller-asserted central transmission with a connection event.
    ///
    /// A fixed-channel receiver cannot infer packet direction. Callers must not
    /// pass peripheral responses as central observations.
    pub fn observe_central(&mut self, access_address_sample: u64) -> Result<ConnectionObservation> {
        if let Some(tracker) = &mut self.tracker {
            return tracker.synchronize_observation(
                self.capture_channel,
                access_address_sample,
                self.config.peer_clock_accuracy,
                self.config.receiver_clock_accuracy_ppm,
                self.config.maximum_event_advance,
            );
        }

        let tracker = ConnectionTracker::new(
            self.config.tracker.clone(),
            self.config.first_event_counter,
            access_address_sample,
        )?;
        let event = tracker.current_event()?;
        let timing_window = tracker
            .current_timing_window(
                self.config.peer_clock_accuracy,
                self.config.receiver_clock_accuracy_ppm,
            )?
            .ok_or_else(|| {
                Error::InvalidState(
                    "new fixed-channel central observation tracker has no timing anchor".to_owned(),
                )
            })?;
        let observation = ConnectionObservation {
            event,
            timing_window,
            advanced_events: 0,
            timing_error: SampleTimingError::OnTime,
        };
        self.tracker = Some(tracker);
        Ok(observation)
    }
}

trait CaptureObservation {
    fn access_address_sample(&self) -> u64;
}

impl CaptureObservation for ReceivedAdvertisingPdu {
    fn access_address_sample(&self) -> u64 {
        self.access_address_sample
    }
}

impl CaptureObservation for ReceivedLePdu {
    fn access_address_sample(&self) -> u64 {
        self.access_address_sample
    }
}

struct CaptureDecodeBatch<T> {
    packets: Vec<T>,
    discontinuity: Option<SampleDiscontinuity>,
}

trait CaptureStreamDecoder {
    type Observation: CaptureObservation;

    fn push_capture(
        &mut self,
        first_sample_index: u64,
        input: &[Complex32],
    ) -> Result<CaptureDecodeBatch<Self::Observation>>;
}

impl CaptureStreamDecoder for Le1mStreamDecoder {
    type Observation = ReceivedAdvertisingPdu;

    fn push_capture(
        &mut self,
        first_sample_index: u64,
        input: &[Complex32],
    ) -> Result<CaptureDecodeBatch<Self::Observation>> {
        let batch = self.push(first_sample_index, input)?;
        Ok(CaptureDecodeBatch {
            packets: batch.packets,
            discontinuity: batch.discontinuity,
        })
    }
}

impl CaptureStreamDecoder for LeUncodedPacketStreamDecoder {
    type Observation = ReceivedLePdu;

    fn push_capture(
        &mut self,
        first_sample_index: u64,
        input: &[Complex32],
    ) -> Result<CaptureDecodeBatch<Self::Observation>> {
        let batch = self.push(first_sample_index, input)?;
        Ok(CaptureDecodeBatch {
            packets: batch.packets,
            discontinuity: batch.discontinuity,
        })
    }
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
    let decoder = Le1mStreamDecoder::new(ble_channel, demod_config)?;
    capture_with_decoder(
        source,
        radio_config,
        demod_config.sample_rate_hz,
        decoder,
        limits,
        |observation, relative_sample_index| {
            on_packet(&CapturedAdvertisingPdu {
                observation,
                relative_sample_index,
            })
        },
    )
}

pub fn capture_data_channel<S, F>(
    source: &mut S,
    radio_config: &SdrConfig,
    ble_channel: BleChannel,
    frame_config: LeFrameConfig,
    demod_config: LeUncodedDemodConfig,
    limits: CaptureLimits,
    mut on_packet: F,
) -> Result<CaptureStats>
where
    S: IqSource,
    F: FnMut(&CapturedDataChannelPdu) -> Result<()>,
{
    if ble_channel.index() > 36 {
        return Err(Error::InvalidConfiguration(format!(
            "data capture requires a channel in 0..=36; got {}",
            ble_channel.index()
        )));
    }
    if frame_config.layout != LePduLayout::Data {
        return Err(Error::InvalidConfiguration(
            "data capture requires LE data-channel frame configuration".to_owned(),
        ));
    }
    let decoder = LeUncodedPacketStreamDecoder::new(ble_channel, frame_config, demod_config)?;
    capture_with_decoder(
        source,
        radio_config,
        demod_config.sample_rate_hz,
        decoder,
        limits,
        |observation, relative_sample_index| {
            on_packet(&CapturedDataChannelPdu {
                observation,
                relative_sample_index,
            })
        },
    )
}

fn capture_with_decoder<S, D, F>(
    source: &mut S,
    radio_config: &SdrConfig,
    demodulator_sample_rate_hz: u32,
    decoder: D,
    limits: CaptureLimits,
    mut on_packet: F,
) -> Result<CaptureStats>
where
    S: IqSource,
    D: CaptureStreamDecoder,
    F: FnMut(D::Observation, u64) -> Result<()>,
{
    limits.validate()?;
    if radio_config.sample_rate_hz != demodulator_sample_rate_hz {
        return Err(Error::InvalidConfiguration(format!(
            "radio sample rate {} does not match demodulator sample rate {demodulator_sample_rate_hz}",
            radio_config.sample_rate_hz
        )));
    }
    source.configure(radio_config)?;
    if let Some(applied_sample_rate_hz) = source.applied_sample_rate_hz()
        && applied_sample_rate_hz != demodulator_sample_rate_hz
    {
        return Err(Error::InvalidConfiguration(format!(
            "SDR applied sample rate {applied_sample_rate_hz} Hz does not match demodulator sample rate {demodulator_sample_rate_hz} Hz"
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

fn capture_loop<S, D, F>(
    source: &mut S,
    mut decoder: D,
    limits: CaptureLimits,
    on_packet: &mut F,
) -> Result<CaptureStats>
where
    S: IqSource,
    D: CaptureStreamDecoder,
    F: FnMut(D::Observation, u64) -> Result<()>,
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

        let batch = decoder.push_capture(metadata.first_sample_index, &buffer[..count])?;
        if batch.discontinuity.is_some() {
            stats.discontinuities += 1;
        }
        for observation in batch.packets {
            let access_address_sample = observation.access_address_sample();
            let relative_sample_index = access_address_sample
                .checked_sub(first_hardware_sample)
                .ok_or_else(|| {
                    Error::InvalidInput(format!(
                        "hardware sample counter moved before capture origin: {} < {}",
                        access_address_sample, first_hardware_sample
                    ))
                })?;
            on_packet(observation, relative_sample_index)?;
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
    use crate::link_layer::{ChannelSelectionAlgorithm, ConnectionParameters, DataChannelMap};
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

        modulate_uncoded(&bits, 4, 250_000.0, 4_000_000.0)
    }

    fn modulated_data_channel_packet() -> Vec<Complex32> {
        let channel = BleChannel::new(12).unwrap();
        // Scapy commit de3399269bad8c9a6bfb1dc181c3876340c198b8
        // independently produced CRC 421893 for this CTE-bearing body.
        let mut pdu = vec![
            0x3e, 0x09, 0x85, 0x05, 0x00, 0x04, 0x00, 0x0a, 0x01, 0x00, 0x02, 0x00,
        ];
        pdu.extend_from_slice(&[0x42, 0x18, 0x93]);
        let mut body = bytes_to_bits_lsb(&pdu);
        whiten_bits(&mut body, channel);
        let mut bits = bytes_to_bits_lsb(&[0xaa]);
        bits.extend(bytes_to_bits_lsb(&0x1234_5678u32.to_le_bytes()));
        bits.extend(body);
        modulate_uncoded(&bits, 4, 250_000.0, 4_000_000.0)
    }

    fn modulated_le_2m_data_channel_packet() -> Vec<Complex32> {
        // BTLE commit 85401861e8f4b04b90cbaa0394c0f9d45ed02f18
        // independently generated these complete channel-12 over-air bytes.
        let over_air = [
            0xaa, 0xaa, 0x78, 0x56, 0x34, 0x12, 0x2e, 0xe8, 0xf3, 0xc7, 0x89, 0xd2, 0x5d, 0xa0,
            0x3d, 0x55, 0xe5, 0x3c,
        ];
        modulate_uncoded(&bytes_to_bits_lsb(&over_air), 4, 500_000.0, 8_000_000.0)
    }

    fn modulate_uncoded(
        bits: &[bool],
        samples_per_symbol: usize,
        deviation_hz: f32,
        sample_rate_hz: f32,
    ) -> Vec<Complex32> {
        let mut phase = 0.0f32;
        let mut samples = vec![Complex32::new(1.0, 0.0); 7];
        for bit in bits {
            let frequency = if *bit { deviation_hz } else { -deviation_hz };
            let step = TAU * frequency / sample_rate_hz;
            for _ in 0..samples_per_symbol {
                phase += step;
                samples.push(Complex32::new(phase.cos(), phase.sin()));
            }
        }
        samples
    }

    fn fixed_channel_tracking_config() -> FixedChannelCentralObservationConfig {
        FixedChannelCentralObservationConfig {
            tracker: ConnectionTrackerConfig {
                access_address: 0x1234_5678,
                channel_selection_algorithm: ChannelSelectionAlgorithm::Csa2,
                hop_increment: 5,
                channel_map: DataChannelMap::new([0xff, 0xff, 0xff, 0xff, 0x1f]).unwrap(),
                parameters: ConnectionParameters::new(24, 0, 100).unwrap(),
                sample_rate_hz: 4_000_000,
            },
            first_event_counter: 0,
            peer_clock_accuracy: SleepClockAccuracy::new(0).unwrap(),
            receiver_clock_accuracy_ppm: 20,
            maximum_event_advance: 32,
        }
    }

    #[test]
    fn fixed_channel_central_observations_anchor_and_recover_missed_events() {
        let mut tracker = FixedChannelCentralObservationTracker::new(
            BleChannel::new(31).unwrap(),
            fixed_channel_tracking_config(),
        )
        .unwrap();
        assert!(!tracker.is_anchored());

        let first = tracker.observe_central(1_000).unwrap();
        assert!(tracker.is_anchored());
        assert_eq!(first.event.event_counter, 0);
        assert_eq!(first.event.channel, BleChannel::new(31).unwrap());
        assert_eq!(first.advanced_events, 0);
        assert_eq!(first.timing_error, SampleTimingError::OnTime);

        assert!(tracker.observe_central(1_600).is_err());
        let recovered = tracker.observe_central(841_050).unwrap();
        assert_eq!(recovered.event.event_counter, 7);
        assert_eq!(recovered.event.channel, BleChannel::new(31).unwrap());
        assert_eq!(recovered.advanced_events, 7);
        assert_eq!(recovered.timing_error, SampleTimingError::Late(50));
    }

    #[test]
    fn fixed_channel_central_observations_require_the_first_selected_channel() {
        let error = FixedChannelCentralObservationTracker::new(
            BleChannel::new(20).unwrap(),
            fixed_channel_tracking_config(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("event 0 uses channel 31"));

        let mut invalid_clock = fixed_channel_tracking_config();
        invalid_clock.receiver_clock_accuracy_ppm = 1_000_001;
        assert!(
            FixedChannelCentralObservationTracker::new(BleChannel::new(31).unwrap(), invalid_clock)
                .is_err()
        );
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
    fn captures_cte_bearing_data_packet_across_backend_blocks() {
        let samples = modulated_data_channel_packet();
        let split = samples.len() / 4;
        let chunks = [
            samples[..split].to_vec(),
            samples[split..split * 2].to_vec(),
            samples[split * 2..split * 3].to_vec(),
            samples[split * 3..].to_vec(),
        ];
        let mut next = 70_000u64;
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
        let stats = capture_data_channel(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_430_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(12).unwrap(),
            LeFrameConfig::data(0x1234_5678, 0x00ab_cdef).unwrap(),
            LeUncodedDemodConfig {
                phy: crate::demod::LeUncodedPhy::Le1M,
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
        assert_eq!(packets[0].observation.pdu.header, [0x3e, 0x09]);
        assert_eq!(packets[0].observation.pdu.cte_info, Some(0x85));
        assert_eq!(
            packets[0].observation.pdu.payload,
            [0x05, 0x00, 0x04, 0x00, 0x0a, 0x01, 0x00, 0x02, 0x00]
        );
        assert_eq!(packets[0].observation.pdu.crc, [0x42, 0x18, 0x93]);
        assert!(packets[0].relative_sample_index < total_samples);
    }

    #[test]
    fn captures_independent_le_2m_data_vector() {
        let samples = modulated_le_2m_data_channel_packet();
        let total_samples = samples.len() as u64;
        let mut blocks = VecDeque::new();
        blocks.push_back((
            90_000,
            samples,
            ReadMetadata {
                first_sample_index: 90_000,
                dropped_samples_before: 0,
                overrun: false,
            },
        ));
        let mut source = MockSource {
            blocks,
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: Some(8_000_000),
        };
        let mut packets = Vec::new();
        let stats = capture_data_channel(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_430_000_000,
                sample_rate_hz: 8_000_000,
                bandwidth_hz: 4_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(12).unwrap(),
            LeFrameConfig::data(0x1234_5678, 0x00ab_cdef).unwrap(),
            LeUncodedDemodConfig {
                phy: crate::demod::LeUncodedPhy::Le2M,
                sample_rate_hz: 8_000_000,
                max_access_address_errors: 0,
            },
            CaptureLimits {
                maximum_samples: Some(total_samples),
                maximum_duration: None,
                read_timeout: Duration::from_millis(100),
                block_samples: total_samples as usize,
            },
            |packet| {
                packets.push(packet.clone());
                Ok(())
            },
        )
        .unwrap();

        assert!(source.stopped);
        assert_eq!(stats.packets_decoded, 1);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].observation.phy, crate::demod::LeUncodedPhy::Le2M);
        assert_eq!(packets[0].observation.pdu.header, [0x02, 0x07]);
        assert_eq!(
            packets[0].observation.pdu.payload,
            [0x03, 0x00, 0x04, 0x00, 0x0a, 0x01, 0x00]
        );
        assert_eq!(packets[0].observation.pdu.crc, [0xf2, 0x83, 0x8c]);
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
    fn validates_data_channel_before_configuring_source() {
        let mut source = MockSource {
            blocks: VecDeque::new(),
            configured: false,
            running: false,
            stopped: false,
            applied_sample_rate_hz: None,
        };
        let result = capture_data_channel(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_402_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(37).unwrap(),
            LeFrameConfig::data(0x1234_5678, 0x00ab_cdef).unwrap(),
            LeUncodedDemodConfig {
                phy: crate::demod::LeUncodedPhy::Le1M,
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
        assert!(result.unwrap_err().to_string().contains("0..=36"));
        assert!(!source.configured);
        assert!(!source.running);

        let result = capture_data_channel(
            &mut source,
            &SdrConfig {
                center_frequency_hz: 2_430_000_000,
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
                gain_db: 20.0,
                channel: 0,
            },
            BleChannel::new(12).unwrap(),
            LeFrameConfig::advertising(),
            LeUncodedDemodConfig {
                phy: crate::demod::LeUncodedPhy::Le1M,
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
                .contains("data-channel frame configuration")
        );
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
