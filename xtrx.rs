use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int, c_uint};
use std::ptr;
use anyhow::{Context, Result};
use num_complex::Complex;
use crate::SDRDevice;

#[repr(C)]
struct XtrxDevice; // Placeholder for the device handle type

// FFI function bindings to libxtrx
extern "C" {
    fn xtrx_open(devname: *const c_char, flags: c_uint, dev: *mut *mut XtrxDevice) -> c_int;
    fn xtrx_close(dev: *mut XtrxDevice);
    fn xtrx_set_samplerate(dev: *mut XtrxDevice, cgen_rate: c_double, rxrate: c_double, txrate: c_double,
                           flags: c_uint, actualcgen: *mut c_double, actualrx: *mut c_double, actualtx: *mut c_double) -> c_int;
    fn xtrx_tune(dev: *mut XtrxDevice, is_tx: c_int, freq: c_double, actualfreq: *mut c_double) -> c_int;
    fn xtrx_tune_rx_bandwidth(dev: *mut XtrxDevice, bw: c_double, actualbw: *mut c_double) -> c_int;
    fn xtrx_set_gain(dev: *mut XtrxDevice, is_tx: c_int, gain: c_double, actualgain: *mut c_double) -> c_int;
    fn xtrx_recv_burst_sync(dev: *mut XtrxDevice, buffer: *mut Complex<f32>, samples: c_int, timeout: c_uint) -> c_int;
}

// XTRXSdr struct representing the XTRX SDR device
pub struct XTRXSdr {
    device: *mut XtrxDevice,
}

impl XTRXSdr {
    /// Creates a new instance of XTRXSdr by opening the XTRX device
    pub fn new() -> Result<Self> {
        let dev_name = CString::new("xtrx").expect("CString creation failed");
        let mut device: *mut XtrxDevice = ptr::null_mut();

        // Open device with xtrx_open
        let res = unsafe { xtrx_open(dev_name.as_ptr(), 0, &mut device) };
        if res != 0 {
            return Err(anyhow::anyhow!("Failed to open XTRX device, error code: {}", res));
        }

        Ok(XTRXSdr { device })
    }

    /// Sets the RX bandwidth in Hz and returns the actual bandwidth set
    pub fn set_rx_bandwidth(&self, bandwidth: f64) -> Result<f64> {
        let mut actual_bw: c_double = 0.0;
        let res = unsafe { xtrx_tune_rx_bandwidth(self.device, bandwidth, &mut actual_bw) };
        if res != 0 {
            return Err(anyhow::anyhow!("Failed to set RX bandwidth, error code: {}", res));
        }
        Ok(actual_bw as f64)
    }

    /// Sets the RX gain in dB and returns the actual gain set
    pub fn set_gain(&self, gain: f64) -> Result<f64> {
        let mut actual_gain: c_double = 0.0;
        let res = unsafe { xtrx_set_gain(self.device, 0, gain, &mut actual_gain) };
        if res != 0 {
            return Err(anyhow::anyhow!("Failed to set RX gain, error code: {}", res));
        }
        Ok(actual_gain as f64)
    }
}

// Implement Drop for automatic resource cleanup
impl Drop for XTRXSdr {
    fn drop(&mut self) {
        unsafe { xtrx_close(self.device) };
    }
}

// Implement SDRDevice trait for XTRXSdr
impl SDRDevice for XTRXSdr {
    fn init(&self) -> Result<()> {
        // Initialization is handled in new() for now
        Ok(())
    }

    /// Sets the center frequency for RX in Hz
    fn set_frequency(&self, freq: u64) -> Result<()> {
        let mut actual_freq: c_double = 0.0;
        let res = unsafe { xtrx_tune(self.device, 0, freq as c_double, &mut actual_freq) };
        if res != 0 {
            return Err(anyhow::anyhow!("Failed to set frequency, error code: {}", res));
        }
        Ok(())
    }

    /// Sets the sample rate in Hz for RX and TX
    fn set_sample_rate(&self, rate: u32) -> Result<()> {
        let mut actual_cgen: c_double = 0.0;
        let mut actual_rx: c_double = 0.0;
        let mut actual_tx: c_double = 0.0;
        let res = unsafe {
            xtrx_set_samplerate(
                self.device,
                rate as c_double,
                rate as c_double,
                rate as c_double,
                0,
                &mut actual_cgen,
                &mut actual_rx,
                &mut actual_tx,
            )
        };
        if res != 0 {
            return Err(anyhow::anyhow!("Failed to set sample rate, error code: {}", res));
        }
        Ok(())
    }

    /// Receives samples synchronously in a burst and writes to the buffer
    fn read_iq_samples(&self, buffer: &mut [Complex<f32>]) -> Result<usize> {
        let samples_to_read = buffer.len() as c_int;
        let timeout_ms: c_uint = 1000;
        let samples_received = unsafe {
            xtrx_recv_burst_sync(self.device, buffer.as_mut_ptr(), samples_to_read, timeout_ms)
        };
        if samples_received < 0 {
            return Err(anyhow::anyhow!("Failed to receive samples, error code: {}", samples_received));
        }
        Ok(samples_received as usize)
    }

    /// Returns the maximum bandwidth supported by the XTRX device
    fn get_bandwidth(&self) -> u32 {
        120_000_000 // XTRX supports up to 120 MHz bandwidth
    }
}
