use anyhow::{Context, Result};
use libbladeRF::{BladeRF, Channel, SampleFormat};
use num_complex::Complex;
use crate::SDRDevice;

#[derive(Debug, Clone, Copy)]
enum BladeRFModel {
    Original,
    XA4,
    XA9,
    XA16,
}

pub struct BladeRFSDR {
    device: BladeRF,
    model: BladeRFModel,
    max_bandwidth: u32,
}

impl BladeRFSDR {
    pub fn new() -> Result<Self> {
        let device = BladeRF::open().context("Failed to open BladeRF device")?;

        // Detect the model and configure max bandwidth
        let model = match device.get_fpga_version()?.to_string().as_str() {
            "40" => BladeRFModel::Original,
            "XA4" => BladeRFModel::XA4,
            "XA9" => BladeRFModel::XA9,
            "XA16" => BladeRFModel::XA16,
            _ => BladeRFModel::Original,
        };

        let max_bandwidth = match model {
            BladeRFModel::Original => 28_000_000, // BladeRF x40
            BladeRFModel::XA4 => 38_000_000,      // BladeRF xA4
            BladeRFModel::XA9 | BladeRFModel::XA16 => 61_000_000, // BladeRF xA9 and xA16
        };

        Ok(BladeRFSDR {
            device,
            model,
            max_bandwidth,
        })
    }

    fn get_model_name(&self) -> &'static str {
        match self.model {
            BladeRFModel::Original => "BladeRF x40",
            BladeRFModel::XA4 => "BladeRF xA4",
            BladeRFModel::XA9 => "BladeRF xA9",
            BladeRFModel::XA16 => "BladeRF xA16",
        }
    }
}

impl SDRDevice for BladeRFSDR {
    fn init(&self) -> Result<()> {
        self.device
            .enable_rx(Channel::RX1, true)
            .context(format!("Failed to enable RX on {}", self.get_model_name()))?;
        Ok(())
    }

    fn set_frequency(&self, freq: u64) -> Result<()> {
        self.device
            .set_rx_frequency(Channel::RX1, freq)
            .context(format!("Failed to set frequency on {}", self.get_model_name()))
    }

    fn set_sample_rate(&self, rate: u32) -> Result<()> {
        if rate > self.max_bandwidth {
            anyhow::bail!("Requested sample rate exceeds max bandwidth for {}", self.get_model_name());
        }
        self.device
            .set_rx_sample_rate(Channel::RX1, rate)
            .context(format!("Failed to set sample rate on {}", self.get_model_name()))
    }

    fn read_iq_samples(&self, buffer: &mut [Complex<f32>]) -> Result<usize> {
        let samples = self
            .device
            .receive(Channel::RX1, buffer, SampleFormat::I16)
            .context(format!("Failed to read samples from {}", self.get_model_name()))?;
        Ok(samples)
    }

    fn get_bandwidth(&self) -> u32 {
        self.max_bandwidth
    }
}
