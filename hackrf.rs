use anyhow::{Context, Result};
use hackrf::{HackRF, HackRFDevice};
use num_complex::Complex;
use crate::SDRDevice;

pub struct HackRFSdr {
    device: HackRFDevice,
}

impl HackRFSdr {
    pub fn new() -> Result<Self> {
        let device = HackRF::open().context("Failed to open HackRF device")?;
        Ok(HackRFSdr { device })
    }
}

impl SDRDevice for HackRFSdr {
    fn init(&self) -> Result<()> {
        self.device.set_antenna_enable(true)?;
        self.device.set_lna_gain(16)?;
        self.device.set_vga_gain(20)?;
        Ok(())
    }

    fn set_frequency(&self, freq: u64) -> Result<()> {
        self.device.set_freq(freq).context("Failed to set frequency on HackRF")
    }

    fn set_sample_rate(&self, rate: u32) -> Result<()> {
        self.device.set_sample_rate(rate as f64).context("Failed to set sample rate on HackRF")
    }

    fn read_iq_samples(&self, buffer: &mut [Complex<f32>]) -> Result<usize> {
        let samples = self.device.receive(buffer)?;
        Ok(samples)
    }

    fn get_bandwidth(&self) -> u32 {
        20_000_000 // HackRF has a maximum bandwidth of 20 MHz
    }
}
