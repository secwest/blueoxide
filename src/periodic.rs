use crate::advertising::PeriodicAdvertisingSyncInfo;
use crate::ble::BleChannel;
use crate::link_layer::{ChannelSelectionAlgorithm2, LePhy, SampleTimingError};
use crate::{Error, Result};

const EVENT_IFS_US: u128 = 150;
const OFFSET_ADJUST_US: u32 = 2_457_600;
const MINIMUM_300_US_OFFSET: u32 = 245_700;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicAdvertisingTrackerConfig {
    pub sample_rate_hz: u32,
    pub receiver_clock_accuracy_ppm: u32,
}

impl PeriodicAdvertisingTrackerConfig {
    pub fn validate(self) -> Result<()> {
        if self.sample_rate_hz == 0 {
            return Err(Error::InvalidConfiguration(
                "periodic advertising tracker sample rate must be greater than zero".to_owned(),
            ));
        }
        if self.receiver_clock_accuracy_ppm > 1_000_000 {
            return Err(Error::InvalidConfiguration(format!(
                "receiver clock accuracy {} ppm exceeds 1000000",
                self.receiver_clock_accuracy_ppm
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicAdvertisingTimingWindow {
    pub represented_earliest_sample: u64,
    pub represented_latest_sample: u64,
    pub earliest_sample: u64,
    pub latest_sample: u64,
    pub quantization_width_samples: u64,
    pub widening_samples: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicAdvertisingEvent {
    pub event_counter: u16,
    pub channel: BleChannel,
    pub phy: LePhy,
    pub timing_window: PeriodicAdvertisingTimingWindow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicAdvertisingObservation {
    pub event: PeriodicAdvertisingEvent,
    pub advanced_events: u16,
    pub timing_error: SampleTimingError,
}

#[derive(Clone, Copy, Debug)]
enum PeriodicAdvertisingTiming {
    SyncInfo {
        packet_access_address_sample: u64,
    },
    Anchored {
        event_index: u64,
        access_address_sample: u64,
    },
}

#[derive(Clone, Debug)]
pub struct PeriodicAdvertisingTracker {
    sync_info: PeriodicAdvertisingSyncInfo,
    selector: ChannelSelectionAlgorithm2,
    phy: LePhy,
    config: PeriodicAdvertisingTrackerConfig,
    event_counter: u16,
    event_index: u64,
    timing: PeriodicAdvertisingTiming,
}

impl PeriodicAdvertisingTracker {
    pub fn new(
        sync_info: PeriodicAdvertisingSyncInfo,
        phy: LePhy,
        sync_packet_access_address_sample: u64,
        config: PeriodicAdvertisingTrackerConfig,
    ) -> Result<Self> {
        config.validate()?;
        validate_sync_info(&sync_info)?;
        let selector = ChannelSelectionAlgorithm2::new(
            sync_info.channel_map.clone(),
            sync_info.access_address,
        );
        let event_counter = sync_info.event_counter;
        Ok(Self {
            sync_info,
            selector,
            phy,
            config,
            event_counter,
            event_index: 0,
            timing: PeriodicAdvertisingTiming::SyncInfo {
                packet_access_address_sample: sync_packet_access_address_sample,
            },
        })
    }

    pub const fn event_counter(&self) -> u16 {
        self.event_counter
    }

    pub const fn phy(&self) -> LePhy {
        self.phy
    }

    pub const fn access_address(&self) -> u32 {
        self.sync_info.access_address
    }

    pub const fn crc_init(&self) -> u32 {
        self.sync_info.crc_init
    }

    pub const fn interval_us(&self) -> u32 {
        self.sync_info.interval_us()
    }

    pub fn channel_map(&self) -> &crate::link_layer::DataChannelMap {
        &self.sync_info.channel_map
    }

    pub fn reset(&mut self, sync_packet_access_address_sample: u64) {
        self.event_counter = self.sync_info.event_counter;
        self.event_index = 0;
        self.timing = PeriodicAdvertisingTiming::SyncInfo {
            packet_access_address_sample: sync_packet_access_address_sample,
        };
    }

    pub fn current_event(&self) -> Result<PeriodicAdvertisingEvent> {
        Ok(PeriodicAdvertisingEvent {
            event_counter: self.event_counter,
            channel: self.selector.channel_for_event(self.event_counter),
            phy: self.phy,
            timing_window: self.current_timing_window()?,
        })
    }

    pub fn advance(&mut self) -> Result<PeriodicAdvertisingEvent> {
        self.event_index = self.event_index.checked_add(1).ok_or_else(|| {
            Error::InvalidState("periodic advertising event index overflow".to_owned())
        })?;
        self.event_counter = self.event_counter.wrapping_add(1);
        self.current_event()
    }

    pub fn synchronize_observation(
        &mut self,
        channel: BleChannel,
        phy: LePhy,
        access_address_sample: u64,
        maximum_event_advance: u16,
    ) -> Result<PeriodicAdvertisingObservation> {
        if channel.index() > 36 {
            return Err(Error::InvalidInput(format!(
                "periodic advertising observation requires channel 0..=36; got {}",
                channel.index()
            )));
        }
        if phy != self.phy {
            return Err(Error::InvalidInput(format!(
                "periodic advertising observation expected PHY {}, received {phy}",
                self.phy
            )));
        }

        let mut candidate = self.clone();
        let mut best: Option<(Self, PeriodicAdvertisingObservation)> = None;
        let mut tied = false;
        for advanced_events in 0..=maximum_event_advance {
            let event = candidate.current_event()?;
            let window = event.timing_window;
            if event.channel == channel
                && (window.earliest_sample..=window.latest_sample).contains(&access_address_sample)
            {
                let timing_error = timing_error(window, access_address_sample);
                let observation = PeriodicAdvertisingObservation {
                    event,
                    advanced_events,
                    timing_error,
                };
                let replace = best.as_ref().is_none_or(|(_, existing)| {
                    timing_error.absolute_samples() < existing.timing_error.absolute_samples()
                });
                if replace {
                    let mut matched = candidate.clone();
                    matched.timing = PeriodicAdvertisingTiming::Anchored {
                        event_index: matched.event_index,
                        access_address_sample,
                    };
                    best = Some((matched, observation));
                    tied = false;
                } else if best.as_ref().is_some_and(|(_, existing)| {
                    timing_error.absolute_samples() == existing.timing_error.absolute_samples()
                }) {
                    tied = true;
                }
            }
            if advanced_events != maximum_event_advance {
                candidate.advance()?;
            }
        }

        if tied {
            return Err(Error::InvalidInput(
                "periodic advertising observation matches multiple events equally".to_owned(),
            ));
        }
        let Some((matched, observation)) = best else {
            return Err(Error::InvalidInput(format!(
                "periodic advertising observation on channel {} at sample {} did not match the next {} event(s)",
                channel.index(),
                access_address_sample,
                u32::from(maximum_event_advance) + 1
            )));
        };
        *self = matched;
        Ok(observation)
    }

    fn current_timing_window(&self) -> Result<PeriodicAdvertisingTimingWindow> {
        let interval_us = u128::from(self.sync_info.interval_us());
        let maximum_widening_us = interval_us
            .checked_div(2)
            .and_then(|value| value.checked_sub(EVENT_IFS_US))
            .ok_or_else(|| {
                Error::InvalidState("periodic advertising widening cap underflow".to_owned())
            })?;
        let maximum_widening_samples =
            microseconds_to_samples_ceil(maximum_widening_us, self.config.sample_rate_hz)?;

        match self.timing {
            PeriodicAdvertisingTiming::SyncInfo {
                packet_access_address_sample,
            } => {
                let elapsed_us = u128::from(self.sync_info.packet_offset_us())
                    .checked_add(
                        u128::from(self.event_index)
                            .checked_mul(interval_us)
                            .ok_or_else(|| {
                                Error::InvalidState(
                                    "periodic advertising elapsed-time overflow".to_owned(),
                                )
                            })?,
                    )
                    .ok_or_else(|| {
                        Error::InvalidState("periodic advertising elapsed-time overflow".to_owned())
                    })?;
                let represented_latest_us = elapsed_us
                    .checked_add(u128::from(self.sync_info.offset_units_us))
                    .ok_or_else(|| {
                        Error::InvalidState("periodic advertising quantization overflow".to_owned())
                    })?;
                let represented_earliest_sample = packet_access_address_sample
                    .checked_add(microseconds_to_samples_floor(
                        elapsed_us,
                        self.config.sample_rate_hz,
                    )?)
                    .ok_or_else(|| {
                        Error::InvalidState(
                            "periodic advertising earliest sample exceeds u64".to_owned(),
                        )
                    })?;
                let represented_latest_sample = packet_access_address_sample
                    .checked_add(microseconds_to_samples_ceil(
                        represented_latest_us,
                        self.config.sample_rate_hz,
                    )?)
                    .ok_or_else(|| {
                        Error::InvalidState(
                            "periodic advertising latest sample exceeds u64".to_owned(),
                        )
                    })?;
                let widening_samples = self
                    .widening_samples(elapsed_us)?
                    .min(maximum_widening_samples);
                Ok(PeriodicAdvertisingTimingWindow {
                    represented_earliest_sample,
                    represented_latest_sample,
                    earliest_sample: represented_earliest_sample.saturating_sub(widening_samples),
                    latest_sample: represented_latest_sample
                        .checked_add(widening_samples)
                        .ok_or_else(|| {
                            Error::InvalidState(
                                "periodic advertising widened sample exceeds u64".to_owned(),
                            )
                        })?,
                    quantization_width_samples: microseconds_to_samples_ceil(
                        u128::from(self.sync_info.offset_units_us),
                        self.config.sample_rate_hz,
                    )?,
                    widening_samples,
                })
            }
            PeriodicAdvertisingTiming::Anchored {
                event_index,
                access_address_sample,
            } => {
                let elapsed_events =
                    self.event_index.checked_sub(event_index).ok_or_else(|| {
                        Error::InvalidState(
                            "periodic advertising event index precedes its anchor".to_owned(),
                        )
                    })?;
                let elapsed_us = u128::from(elapsed_events)
                    .checked_mul(interval_us)
                    .ok_or_else(|| {
                        Error::InvalidState("periodic advertising elapsed-time overflow".to_owned())
                    })?;
                let expected_sample = access_address_sample
                    .checked_add(microseconds_to_samples_round(
                        elapsed_us,
                        self.config.sample_rate_hz,
                    )?)
                    .ok_or_else(|| {
                        Error::InvalidState(
                            "periodic advertising expected sample exceeds u64".to_owned(),
                        )
                    })?;
                let widening_samples = self
                    .widening_samples(elapsed_us)?
                    .min(maximum_widening_samples);
                Ok(PeriodicAdvertisingTimingWindow {
                    represented_earliest_sample: expected_sample,
                    represented_latest_sample: expected_sample,
                    earliest_sample: expected_sample.saturating_sub(widening_samples),
                    latest_sample: expected_sample.checked_add(widening_samples).ok_or_else(
                        || {
                            Error::InvalidState(
                                "periodic advertising widened sample exceeds u64".to_owned(),
                            )
                        },
                    )?,
                    quantization_width_samples: 0,
                    widening_samples,
                })
            }
        }
    }

    fn widening_samples(&self, elapsed_us: u128) -> Result<u64> {
        let combined_ppm = u128::from(self.config.receiver_clock_accuracy_ppm)
            .checked_add(u128::from(
                self.sync_info.sleep_clock_accuracy.maximum_ppm(),
            ))
            .ok_or_else(|| {
                Error::InvalidState("periodic advertising clock-accuracy overflow".to_owned())
            })?;
        let numerator = elapsed_us
            .checked_mul(combined_ppm)
            .and_then(|value| value.checked_mul(u128::from(self.config.sample_rate_hz)))
            .ok_or_else(|| {
                Error::InvalidState(
                    "periodic advertising timing-window arithmetic overflow".to_owned(),
                )
            })?;
        divide_round_up(numerator, 1_000_000_000_000)
    }
}

fn validate_sync_info(sync_info: &PeriodicAdvertisingSyncInfo) -> Result<()> {
    if sync_info.packet_offset == 0 {
        return Err(Error::InvalidInput(
            "periodic SyncInfo offset zero cannot schedule a periodic packet".to_owned(),
        ));
    }
    if sync_info.offset_adjust && sync_info.offset_units_us != 300 {
        return Err(Error::InvalidInput(
            "periodic SyncInfo sets Offset Adjust with 30 us units".to_owned(),
        ));
    }
    if sync_info.offset_units_us == 300
        && !sync_info.offset_adjust
        && u32::from(sync_info.packet_offset) * 300 < MINIMUM_300_US_OFFSET
    {
        return Err(Error::InvalidInput(format!(
            "periodic SyncInfo uses 300 us units for an offset below {MINIMUM_300_US_OFFSET} us"
        )));
    }
    if !matches!(sync_info.offset_units_us, 30 | 300) {
        return Err(Error::InvalidInput(format!(
            "periodic SyncInfo offset unit {} is not 30 or 300 us",
            sync_info.offset_units_us
        )));
    }
    if sync_info.offset_adjust && sync_info.packet_offset_us() < OFFSET_ADJUST_US {
        return Err(Error::InvalidInput(
            "periodic SyncInfo adjusted offset underflow".to_owned(),
        ));
    }
    if sync_info.interval < 6 {
        return Err(Error::InvalidInput(format!(
            "periodic SyncInfo interval {} is below 6",
            sync_info.interval
        )));
    }
    if sync_info.crc_init > 0x00ff_ffff {
        return Err(Error::InvalidInput(format!(
            "periodic SyncInfo CRC initialization 0x{:x} exceeds 24 bits",
            sync_info.crc_init
        )));
    }
    Ok(())
}

fn timing_error(
    window: PeriodicAdvertisingTimingWindow,
    observed_sample: u64,
) -> SampleTimingError {
    if observed_sample < window.represented_earliest_sample {
        SampleTimingError::Early(window.represented_earliest_sample - observed_sample)
    } else if observed_sample > window.represented_latest_sample {
        SampleTimingError::Late(observed_sample - window.represented_latest_sample)
    } else {
        SampleTimingError::OnTime
    }
}

fn microseconds_to_samples_floor(microseconds: u128, sample_rate_hz: u32) -> Result<u64> {
    let samples = microseconds
        .checked_mul(u128::from(sample_rate_hz))
        .ok_or_else(|| Error::InvalidState("sample-time multiplication overflow".to_owned()))?
        / 1_000_000;
    u64::try_from(samples)
        .map_err(|_| Error::InvalidState("sample-time result exceeds u64".to_owned()))
}

fn microseconds_to_samples_ceil(microseconds: u128, sample_rate_hz: u32) -> Result<u64> {
    let numerator = microseconds
        .checked_mul(u128::from(sample_rate_hz))
        .ok_or_else(|| Error::InvalidState("sample-time multiplication overflow".to_owned()))?;
    divide_round_up(numerator, 1_000_000)
}

fn microseconds_to_samples_round(microseconds: u128, sample_rate_hz: u32) -> Result<u64> {
    let numerator = microseconds
        .checked_mul(u128::from(sample_rate_hz))
        .and_then(|value| value.checked_add(500_000))
        .ok_or_else(|| Error::InvalidState("sample-time rounding overflow".to_owned()))?;
    u64::try_from(numerator / 1_000_000)
        .map_err(|_| Error::InvalidState("sample-time result exceeds u64".to_owned()))
}

fn divide_round_up(numerator: u128, denominator: u128) -> Result<u64> {
    let rounded = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| Error::InvalidState("integer ceiling arithmetic overflow".to_owned()))?
        / denominator;
    u64::try_from(rounded)
        .map_err(|_| Error::InvalidState("integer ceiling result exceeds u64".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_layer::{DataChannelMap, SleepClockAccuracy};

    fn sync_info() -> PeriodicAdvertisingSyncInfo {
        PeriodicAdvertisingSyncInfo {
            packet_offset: 0x0321,
            offset_units_us: 300,
            offset_adjust: true,
            interval: 0x0020,
            channel_map: DataChannelMap::new([0xff, 0xff, 0xff, 0xff, 0x1f]).unwrap(),
            sleep_clock_accuracy: SleepClockAccuracy::new(3).unwrap(),
            access_address: 0x1234_5678,
            crc_init: 0x00ab_cdef,
            event_counter: 0x4567,
        }
    }

    fn tracker() -> PeriodicAdvertisingTracker {
        PeriodicAdvertisingTracker::new(
            sync_info(),
            LePhy::Le2M,
            1_000,
            PeriodicAdvertisingTrackerConfig {
                sample_rate_hz: 4_000_000,
                receiver_clock_accuracy_ppm: 20,
            },
        )
        .unwrap()
    }

    #[test]
    fn schedules_first_periodic_event_from_sync_info() {
        let tracker = tracker();
        let event = tracker.current_event().unwrap();
        assert_eq!(event.event_counter, 0x4567);
        assert_eq!(event.channel.index(), 27);
        assert_eq!(event.phy, LePhy::Le2M);
        assert_eq!(
            event.timing_window,
            PeriodicAdvertisingTimingWindow {
                represented_earliest_sample: 10_792_600,
                represented_latest_sample: 10_793_800,
                earliest_sample: 10_791_305,
                latest_sample: 10_795_095,
                quantization_width_samples: 1_200,
                widening_samples: 1_295,
            }
        );
        assert_eq!(tracker.access_address(), 0x1234_5678);
        assert_eq!(tracker.crc_init(), 0x00ab_cdef);
        assert_eq!(tracker.interval_us(), 40_000);
    }

    #[test]
    fn advances_with_csa2_and_accumulated_unanchored_uncertainty() {
        let mut tracker = tracker();
        let channels: Vec<u8> = (0..6)
            .map(|index| {
                let event = if index == 0 {
                    tracker.current_event().unwrap()
                } else {
                    tracker.advance().unwrap()
                };
                event.channel.index()
            })
            .collect();
        assert_eq!(channels, [27, 14, 32, 5, 13, 34]);
        let event = tracker.current_event().unwrap();
        assert_eq!(event.event_counter, 0x456c);
        assert_eq!(event.timing_window.represented_earliest_sample, 11_592_600);
        assert_eq!(event.timing_window.widening_samples, 1_391);
    }

    #[test]
    fn observation_reanchors_and_recovers_missed_events() {
        let mut tracker = tracker();
        let first = tracker
            .synchronize_observation(BleChannel::new(27).unwrap(), LePhy::Le2M, 10_792_700, 0)
            .unwrap();
        assert_eq!(first.event.event_counter, 0x4567);
        assert_eq!(first.timing_error, SampleTimingError::OnTime);
        assert_eq!(
            tracker.current_event().unwrap().timing_window,
            PeriodicAdvertisingTimingWindow {
                represented_earliest_sample: 10_792_700,
                represented_latest_sample: 10_792_700,
                earliest_sample: 10_792_700,
                latest_sample: 10_792_700,
                quantization_width_samples: 0,
                widening_samples: 0,
            }
        );

        let recovered = tracker
            .synchronize_observation(BleChannel::new(32).unwrap(), LePhy::Le2M, 11_112_705, 3)
            .unwrap();
        assert_eq!(recovered.event.event_counter, 0x4569);
        assert_eq!(recovered.advanced_events, 2);
        assert_eq!(recovered.timing_error, SampleTimingError::Late(5));
        assert_eq!(recovered.event.timing_window.widening_samples, 39);
        assert_eq!(
            tracker
                .current_event()
                .unwrap()
                .timing_window
                .widening_samples,
            0
        );
    }

    #[test]
    fn rejects_mismatch_without_mutating_state() {
        let mut tracker = tracker();
        let before = tracker.current_event().unwrap();
        assert!(
            tracker
                .synchronize_observation(BleChannel::new(26).unwrap(), LePhy::Le2M, 10_792_600, 0,)
                .is_err()
        );
        assert_eq!(tracker.current_event().unwrap(), before);
        assert!(
            tracker
                .synchronize_observation(BleChannel::new(27).unwrap(), LePhy::Le1M, 10_792_600, 0,)
                .is_err()
        );
        assert_eq!(tracker.current_event().unwrap(), before);
        assert!(
            tracker
                .synchronize_observation(BleChannel::new(27).unwrap(), LePhy::Le2M, 20_000_000, 0,)
                .is_err()
        );
        assert_eq!(tracker.current_event().unwrap(), before);
    }

    #[test]
    fn validates_sync_info_offset_encoding() {
        let mut value = sync_info();
        value.packet_offset = 0;
        assert!(
            PeriodicAdvertisingTracker::new(
                value.clone(),
                LePhy::Le1M,
                0,
                PeriodicAdvertisingTrackerConfig {
                    sample_rate_hz: 4_000_000,
                    receiver_clock_accuracy_ppm: 20,
                },
            )
            .unwrap_err()
            .to_string()
            .contains("offset zero")
        );
        value.packet_offset = 1;
        value.offset_units_us = 30;
        assert!(
            PeriodicAdvertisingTracker::new(
                value.clone(),
                LePhy::Le1M,
                0,
                PeriodicAdvertisingTrackerConfig {
                    sample_rate_hz: 4_000_000,
                    receiver_clock_accuracy_ppm: 20,
                },
            )
            .unwrap_err()
            .to_string()
            .contains("Offset Adjust")
        );
        value.offset_units_us = 300;
        value.offset_adjust = false;
        assert!(
            PeriodicAdvertisingTracker::new(
                value,
                LePhy::Le1M,
                0,
                PeriodicAdvertisingTrackerConfig {
                    sample_rate_hz: 4_000_000,
                    receiver_clock_accuracy_ppm: 20,
                },
            )
            .unwrap_err()
            .to_string()
            .contains("below 245700")
        );
    }

    #[test]
    fn reset_restores_sync_info_schedule() {
        let mut tracker = tracker();
        tracker.advance().unwrap();
        tracker.reset(2_000);
        assert_eq!(tracker.event_counter(), 0x4567);
        assert_eq!(
            tracker
                .current_event()
                .unwrap()
                .timing_window
                .represented_earliest_sample,
            10_793_600
        );
    }

    #[test]
    fn event_counter_wraps_without_losing_monotonic_timing() {
        let mut value = sync_info();
        value.event_counter = u16::MAX;
        let mut tracker = PeriodicAdvertisingTracker::new(
            value,
            LePhy::Le2M,
            1_000,
            PeriodicAdvertisingTrackerConfig {
                sample_rate_hz: 4_000_000,
                receiver_clock_accuracy_ppm: 20,
            },
        )
        .unwrap();
        let before = tracker.current_event().unwrap();
        let after = tracker.advance().unwrap();
        assert_eq!(before.event_counter, u16::MAX);
        assert_eq!(after.event_counter, 0);
        assert_eq!(
            after.timing_window.represented_earliest_sample
                - before.timing_window.represented_earliest_sample,
            160_000
        );
    }

    #[test]
    fn clock_widening_is_capped_before_adjacent_events_overlap() {
        let mut tracker = tracker();
        tracker.event_index = 10_000;
        tracker.event_counter = tracker
            .sync_info
            .event_counter
            .wrapping_add(tracker.event_index as u16);
        let event = tracker.current_event().unwrap();
        assert_eq!(event.timing_window.widening_samples, 79_400);
        assert_eq!(
            event.timing_window.latest_sample - event.timing_window.earliest_sample,
            160_000
        );
    }
}
