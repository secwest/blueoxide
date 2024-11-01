use anyhow::{Context, Result};
use lime_sdr::{Device, SampleFormat};
use num_complex::Complex;
use crate::SDRDevice;

pub struct LimeSDR {
    device: Device,
}

impl LimeSDR {
    pub fn new() -> Result<Self> {
        let device = Device::open().context("Failed to open LimeSDR device")?;
        Ok(LimeSDR { device })
    }
}

impl SDRDevice for LimeSDR {
    fn init(&self) -> Result<()> {
        self.device.enable_rx_channel(0).context("Failed to enable RX channel on LimeSDR")?;
        Ok(())
    }

    fn set_frequency(&self, freq: u64) -> Result<()> {
        self.device.set_rx_frequency(0, freq).context("Failed to set frequency on LimeSDR")
    }

    fn set_sample_rate(&self, rate: u32) -> Result<()> {
        self.device.set_rx_sample_rate(0, rate).context("Failed to set sample rate on LimeSDR")
    }

    fn read_iq_samples(&self, buffer: &mut [Complex<f32>]) -> Result<usize> {
        let samples = self.device.receive::<SampleFormat::I16>(buffer.len()).context("Failed to read samples from LimeSDR")?;
        buffer.copy_from_slice(&samples);
        Ok(samples.len())
    }

    fn get_bandwidth(&self) -> u32 {
        61_440_000 // LimeSDR can support up to 61.44 MHz bandwidth
    }
}
