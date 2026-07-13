use crate::complex::Complex32;
use crate::{Error, Result};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdrKind {
    LimeSdr,
    BladeRf,
    Xtrx,
}

#[derive(Clone, Copy, Debug)]
pub struct SdrCapabilities {
    pub minimum_frequency_hz: u64,
    pub maximum_frequency_hz: u64,
    pub maximum_sample_rate_hz: u32,
    pub receive_channels: u8,
}

#[derive(Clone, Debug)]
pub struct SdrConfig {
    pub center_frequency_hz: u64,
    pub sample_rate_hz: u32,
    pub bandwidth_hz: u32,
    pub gain_db: f32,
    pub channel: u8,
}

impl SdrConfig {
    pub fn validate(&self, capabilities: SdrCapabilities) -> Result<()> {
        if !(capabilities.minimum_frequency_hz..=capabilities.maximum_frequency_hz)
            .contains(&self.center_frequency_hz)
        {
            return Err(Error::InvalidConfiguration(format!(
                "center frequency {} Hz is outside device range {}..={} Hz",
                self.center_frequency_hz,
                capabilities.minimum_frequency_hz,
                capabilities.maximum_frequency_hz
            )));
        }
        if self.sample_rate_hz < 2_000_000
            || self.sample_rate_hz > capabilities.maximum_sample_rate_hz
        {
            return Err(Error::InvalidConfiguration(format!(
                "sample rate {} Hz is outside supported capture range 2000000..={} Hz",
                self.sample_rate_hz, capabilities.maximum_sample_rate_hz
            )));
        }
        if self.bandwidth_hz == 0 || self.bandwidth_hz > self.sample_rate_hz {
            return Err(Error::InvalidConfiguration(
                "bandwidth must be non-zero and no greater than the sample rate".to_owned(),
            ));
        }
        if self.channel >= capabilities.receive_channels {
            return Err(Error::InvalidConfiguration(format!(
                "receive channel {} is unavailable; device exposes {} channel(s)",
                self.channel, capabilities.receive_channels
            )));
        }
        if !self.gain_db.is_finite() {
            return Err(Error::InvalidConfiguration(
                "gain must be finite".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReadMetadata {
    pub first_sample_index: u64,
    pub dropped_samples_before: u64,
    pub overrun: bool,
}

/// Hardware-neutral receive interface used by capture and decoder pipelines.
///
/// Backends must report dropped samples and overruns instead of silently
/// returning a discontinuous stream.
pub trait IqSource {
    fn kind(&self) -> SdrKind;
    fn capabilities(&self) -> SdrCapabilities;
    fn configure(&mut self, config: &SdrConfig) -> Result<()>;
    fn start(&mut self) -> Result<()>;
    fn read(
        &mut self,
        output: &mut [Complex32],
        timeout: Duration,
    ) -> Result<(usize, ReadMetadata)>;
    fn stop(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> SdrCapabilities {
        SdrCapabilities {
            minimum_frequency_hz: 30_000_000,
            maximum_frequency_hz: 3_800_000_000,
            maximum_sample_rate_hz: 61_440_000,
            receive_channels: 2,
        }
    }

    #[test]
    fn accepts_valid_bluetooth_capture_configuration() {
        SdrConfig {
            center_frequency_hz: 2_426_000_000,
            sample_rate_hz: 4_000_000,
            bandwidth_hz: 2_000_000,
            gain_db: 32.0,
            channel: 0,
        }
        .validate(capabilities())
        .unwrap();
    }

    #[test]
    fn rejects_bandwidth_above_sample_rate() {
        let error = SdrConfig {
            center_frequency_hz: 2_426_000_000,
            sample_rate_hz: 4_000_000,
            bandwidth_hz: 5_000_000,
            gain_db: 32.0,
            channel: 0,
        }
        .validate(capabilities())
        .unwrap_err();
        assert!(error.to_string().contains("bandwidth"));
    }
}
