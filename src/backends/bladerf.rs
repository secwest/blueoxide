use crate::complex::Complex32;
use crate::native::DynamicLibrary;
use crate::sdr::{IqSource, ReadMetadata, SdrCapabilities, SdrConfig, SdrKind};
use crate::{Error, Result};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

const BACKEND: &str = "bladeRF";
const BLADERF_CHANNEL_RX_BASE: c_int = 0;
const BLADERF_RX_X1: c_int = 0;
const BLADERF_FORMAT_SC16_Q11_META: c_int = 2;
const BLADERF_META_STATUS_OVERRUN: u32 = 1;
const BLADERF_META_FLAG_RX_NOW: u32 = 1 << 31;
const BLADERF_ERR_TIMEOUT: c_int = -6;
const Q11_SCALE: f32 = 1.0 / 2048.0;

#[derive(Clone, Copy, Debug)]
pub struct BladeRfOptions {
    pub num_buffers: u32,
    pub buffer_size: u32,
    pub num_transfers: u32,
    pub stream_timeout_ms: u32,
}

impl Default for BladeRfOptions {
    fn default() -> Self {
        Self {
            num_buffers: 16,
            buffer_size: 8_192,
            num_transfers: 8,
            stream_timeout_ms: 1_000,
        }
    }
}

impl BladeRfOptions {
    fn validate(self) -> Result<()> {
        if self.num_buffers < 2 {
            return Err(Error::InvalidConfiguration(
                "bladeRF num_buffers must be at least 2".to_owned(),
            ));
        }
        if self.num_transfers == 0 || self.num_transfers >= self.num_buffers {
            return Err(Error::InvalidConfiguration(
                "bladeRF num_transfers must be non-zero and less than num_buffers".to_owned(),
            ));
        }
        if self.buffer_size == 0 {
            return Err(Error::InvalidConfiguration(
                "bladeRF buffer_size must be non-zero".to_owned(),
            ));
        }
        if !self.buffer_size.is_multiple_of(2_048) {
            return Err(Error::InvalidConfiguration(
                "bladeRF buffer_size must be a multiple of 2048 samples".to_owned(),
            ));
        }
        if self.stream_timeout_ms == 0 {
            return Err(Error::InvalidConfiguration(
                "bladeRF stream_timeout_ms must be non-zero".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppliedBladeRfConfig {
    pub sample_rate_hz: u32,
    pub bandwidth_hz: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct BladeRfMetadata {
    timestamp: u64,
    flags: u32,
    status: u32,
    actual_count: u32,
    reserved: [u8; 32],
}

trait BladeRfApi {
    fn library_name(&self) -> &str;
    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void);
    fn close(&self, device: NonNull<c_void>);
    fn set_frequency(&self, device: NonNull<c_void>, channel: c_int, value: u64) -> c_int;
    fn set_sample_rate(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        value: u32,
        actual: &mut u32,
    ) -> c_int;
    fn set_bandwidth(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        value: u32,
        actual: &mut u32,
    ) -> c_int;
    fn set_gain(&self, device: NonNull<c_void>, channel: c_int, value: c_int) -> c_int;
    fn sync_config(&self, device: NonNull<c_void>, options: BladeRfOptions) -> c_int;
    fn enable_module(&self, device: NonNull<c_void>, channel: c_int, enable: bool) -> c_int;
    fn sync_rx(
        &self,
        device: NonNull<c_void>,
        samples: &mut [i16],
        sample_count: u32,
        metadata: &mut BladeRfMetadata,
        timeout_ms: u32,
    ) -> c_int;
    fn error_string(&self, code: c_int) -> String;
}

type OpenFn = unsafe extern "C" fn(*mut *mut c_void, *const c_char) -> c_int;
type CloseFn = unsafe extern "C" fn(*mut c_void);
type SetFrequencyFn = unsafe extern "C" fn(*mut c_void, c_int, u64) -> c_int;
type SetSampleRateFn = unsafe extern "C" fn(*mut c_void, c_int, u32, *mut u32) -> c_int;
type SetBandwidthFn = unsafe extern "C" fn(*mut c_void, c_int, u32, *mut u32) -> c_int;
type SetGainFn = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type SyncConfigFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, u32, u32, u32, u32) -> c_int;
type EnableModuleFn = unsafe extern "C" fn(*mut c_void, c_int, bool) -> c_int;
type SyncRxFn =
    unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut BladeRfMetadata, u32) -> c_int;
type StrErrorFn = unsafe extern "C" fn(c_int) -> *const c_char;

struct DynamicBladeRfApi {
    _library: DynamicLibrary,
    library_name: String,
    open: OpenFn,
    close: CloseFn,
    set_frequency: SetFrequencyFn,
    set_sample_rate: SetSampleRateFn,
    set_bandwidth: SetBandwidthFn,
    set_gain: SetGainFn,
    sync_config: SyncConfigFn,
    enable_module: EnableModuleFn,
    sync_rx: SyncRxFn,
    strerror: StrErrorFn,
}

impl DynamicBladeRfApi {
    fn load() -> Result<Self> {
        let names = match std::env::var_os("BLUEOXIDE_BLADERF_LIBRARY") {
            Some(override_name) if override_name.is_empty() => {
                return Err(Error::InvalidConfiguration(
                    "BLUEOXIDE_BLADERF_LIBRARY must not be empty".to_owned(),
                ));
            }
            Some(override_name) => vec![override_name.to_string_lossy().into_owned()],
            None => default_library_names()
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        };
        let references: Vec<&str> = names.iter().map(String::as_str).collect();
        let library = DynamicLibrary::open_candidates(&references)?;
        let library_name = library.name().to_owned();

        // SAFETY: Function signatures are copied from libbladeRF.h at the
        // pinned revision documented in Verification.md.
        unsafe {
            Ok(Self {
                open: library.symbol("bladerf_open")?,
                close: library.symbol("bladerf_close")?,
                set_frequency: library.symbol("bladerf_set_frequency")?,
                set_sample_rate: library.symbol("bladerf_set_sample_rate")?,
                set_bandwidth: library.symbol("bladerf_set_bandwidth")?,
                set_gain: library.symbol("bladerf_set_gain")?,
                sync_config: library.symbol("bladerf_sync_config")?,
                enable_module: library.symbol("bladerf_enable_module")?,
                sync_rx: library.symbol("bladerf_sync_rx")?,
                strerror: library.symbol("bladerf_strerror")?,
                _library: library,
                library_name,
            })
        }
    }
}

impl BladeRfApi for DynamicBladeRfApi {
    fn library_name(&self) -> &str {
        &self.library_name
    }

    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void) {
        let mut device = std::ptr::null_mut();
        // SAFETY: Function pointer was loaded with the exact ABI. The output
        // pointer is valid and identifier is either NULL or NUL-terminated.
        let status = unsafe {
            (self.open)(
                &mut device,
                identifier.map_or(std::ptr::null(), CStr::as_ptr),
            )
        };
        (status, device)
    }

    fn close(&self, device: NonNull<c_void>) {
        // SAFETY: device was returned by bladerf_open and is owned by driver.
        unsafe { (self.close)(device.as_ptr()) };
    }

    fn set_frequency(&self, device: NonNull<c_void>, channel: c_int, value: u64) -> c_int {
        // SAFETY: device is open and arguments match libbladeRF ABI.
        unsafe { (self.set_frequency)(device.as_ptr(), channel, value) }
    }

    fn set_sample_rate(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        value: u32,
        actual: &mut u32,
    ) -> c_int {
        // SAFETY: actual is writable and device is open.
        unsafe { (self.set_sample_rate)(device.as_ptr(), channel, value, actual) }
    }

    fn set_bandwidth(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        value: u32,
        actual: &mut u32,
    ) -> c_int {
        // SAFETY: actual is writable and device is open.
        unsafe { (self.set_bandwidth)(device.as_ptr(), channel, value, actual) }
    }

    fn set_gain(&self, device: NonNull<c_void>, channel: c_int, value: c_int) -> c_int {
        // SAFETY: device is open and arguments match libbladeRF ABI.
        unsafe { (self.set_gain)(device.as_ptr(), channel, value) }
    }

    fn sync_config(&self, device: NonNull<c_void>, options: BladeRfOptions) -> c_int {
        // SAFETY: device is open and constants match libbladeRF enums.
        unsafe {
            (self.sync_config)(
                device.as_ptr(),
                BLADERF_RX_X1,
                BLADERF_FORMAT_SC16_Q11_META,
                options.num_buffers,
                options.buffer_size,
                options.num_transfers,
                options.stream_timeout_ms,
            )
        }
    }

    fn enable_module(&self, device: NonNull<c_void>, channel: c_int, enable: bool) -> c_int {
        // SAFETY: device is open and channel is a validated RX channel.
        unsafe { (self.enable_module)(device.as_ptr(), channel, enable) }
    }

    fn sync_rx(
        &self,
        device: NonNull<c_void>,
        samples: &mut [i16],
        sample_count: u32,
        metadata: &mut BladeRfMetadata,
        timeout_ms: u32,
    ) -> c_int {
        // SAFETY: samples has two i16 values per requested complex sample,
        // metadata is writable, and device is a running RX stream.
        unsafe {
            (self.sync_rx)(
                device.as_ptr(),
                samples.as_mut_ptr().cast(),
                sample_count,
                metadata,
                timeout_ms,
            )
        }
    }

    fn error_string(&self, code: c_int) -> String {
        // SAFETY: strerror accepts any native result code.
        let pointer = unsafe { (self.strerror)(code) };
        if pointer.is_null() {
            "unknown libbladeRF error".to_owned()
        } else {
            // SAFETY: libbladeRF returns a static NUL-terminated error string.
            unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(windows)]
fn default_library_names() -> &'static [&'static str] {
    &["bladeRF.dll", "libbladeRF.dll"]
}

#[cfg(target_os = "macos")]
fn default_library_names() -> &'static [&'static str] {
    &["libbladeRF.dylib"]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_library_names() -> &'static [&'static str] {
    &["libbladeRF.so.2", "libbladeRF.so"]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverState {
    Open,
    Configured,
    Running,
}

struct BladeRfDriver<A: BladeRfApi> {
    api: Arc<A>,
    device: NonNull<c_void>,
    options: BladeRfOptions,
    state: DriverState,
    rx_channel: c_int,
    native_samples: Vec<i16>,
    expected_next_sample: Option<u64>,
    applied: Option<AppliedBladeRfConfig>,
}

impl<A: BladeRfApi> BladeRfDriver<A> {
    fn open(api: Arc<A>, identifier: Option<&str>, options: BladeRfOptions) -> Result<Self> {
        options.validate()?;
        let identifier = identifier
            .map(|value| {
                CString::new(value).map_err(|_| {
                    Error::InvalidConfiguration(
                        "bladeRF device identifier contains a NUL octet".to_owned(),
                    )
                })
            })
            .transpose()?;
        let (status, device) = api.open(identifier.as_deref());
        if status != 0 {
            return Err(native_error(api.as_ref(), "open", status));
        }
        let Some(device) = NonNull::new(device) else {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "open",
                code: 0,
                message: "libbladeRF returned success with a NULL device".to_owned(),
            });
        };
        Ok(Self {
            api,
            device,
            options,
            state: DriverState::Open,
            rx_channel: BLADERF_CHANNEL_RX_BASE,
            native_samples: Vec::new(),
            expected_next_sample: None,
            applied: None,
        })
    }

    fn applied_config(&self) -> Option<AppliedBladeRfConfig> {
        self.applied
    }

    fn configure(&mut self, config: &SdrConfig) -> Result<()> {
        if self.state == DriverState::Running {
            return Err(Error::InvalidState(
                "bladeRF cannot be configured while streaming".to_owned(),
            ));
        }
        config.validate(bladerf_capabilities())?;
        let channel = (config.channel as c_int) << 1;
        let gain = config.gain_db.round();
        if gain < c_int::MIN as f32 || gain > c_int::MAX as f32 {
            return Err(Error::InvalidConfiguration(
                "bladeRF gain is outside native integer range".to_owned(),
            ));
        }

        let mut actual_sample_rate = 0u32;
        check(
            self.api.as_ref(),
            "set_sample_rate",
            self.api.set_sample_rate(
                self.device,
                channel,
                config.sample_rate_hz,
                &mut actual_sample_rate,
            ),
        )?;
        if actual_sample_rate == 0 {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "set_sample_rate",
                code: 0,
                message: "libbladeRF returned a zero applied sample rate".to_owned(),
            });
        }
        let mut actual_bandwidth = 0u32;
        check(
            self.api.as_ref(),
            "set_bandwidth",
            self.api.set_bandwidth(
                self.device,
                channel,
                config.bandwidth_hz,
                &mut actual_bandwidth,
            ),
        )?;
        if actual_bandwidth == 0 {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "set_bandwidth",
                code: 0,
                message: "libbladeRF returned a zero applied bandwidth".to_owned(),
            });
        }
        check(
            self.api.as_ref(),
            "set_frequency",
            self.api
                .set_frequency(self.device, channel, config.center_frequency_hz),
        )?;
        check(
            self.api.as_ref(),
            "set_gain",
            self.api.set_gain(self.device, channel, gain as c_int),
        )?;
        check(
            self.api.as_ref(),
            "sync_config",
            self.api.sync_config(self.device, self.options),
        )?;

        self.rx_channel = channel;
        self.expected_next_sample = None;
        self.applied = Some(AppliedBladeRfConfig {
            sample_rate_hz: actual_sample_rate,
            bandwidth_hz: actual_bandwidth,
        });
        self.state = DriverState::Configured;
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        match self.state {
            DriverState::Open => Err(Error::InvalidState(
                "bladeRF must be configured before start".to_owned(),
            )),
            DriverState::Configured => {
                check(
                    self.api.as_ref(),
                    "enable_module",
                    self.api.enable_module(self.device, self.rx_channel, true),
                )?;
                self.expected_next_sample = None;
                self.state = DriverState::Running;
                Ok(())
            }
            DriverState::Running => Ok(()),
        }
    }

    fn read(
        &mut self,
        output: &mut [Complex32],
        timeout: Duration,
    ) -> Result<(usize, ReadMetadata)> {
        if self.state != DriverState::Running {
            return Err(Error::InvalidState(
                "bladeRF read requires a running stream".to_owned(),
            ));
        }
        if output.is_empty() {
            return Ok((0, ReadMetadata::default()));
        }
        let sample_count = u32::try_from(output.len()).map_err(|_| {
            Error::InvalidConfiguration("bladeRF read buffer exceeds u32 samples".to_owned())
        })?;
        let scalar_count = output.len().checked_mul(2).ok_or_else(|| {
            Error::InvalidConfiguration("bladeRF native buffer size overflow".to_owned())
        })?;
        self.native_samples.resize(scalar_count, 0);
        let timeout_ms = duration_to_timeout_ms(timeout)?;
        let mut metadata = BladeRfMetadata {
            flags: BLADERF_META_FLAG_RX_NOW,
            ..BladeRfMetadata::default()
        };
        let status = self.api.sync_rx(
            self.device,
            &mut self.native_samples,
            sample_count,
            &mut metadata,
            timeout_ms,
        );
        if status == BLADERF_ERR_TIMEOUT {
            return Ok((0, ReadMetadata::default()));
        }
        check(self.api.as_ref(), "sync_rx", status)?;
        let actual_count = metadata.actual_count as usize;
        if actual_count > output.len() {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "sync_rx",
                code: 0,
                message: format!(
                    "libbladeRF reported {} samples for a {}-sample buffer",
                    actual_count,
                    output.len()
                ),
            });
        }
        for (destination, iq) in output[..actual_count]
            .iter_mut()
            .zip(self.native_samples[..actual_count * 2].chunks_exact(2))
        {
            *destination = Complex32::new(iq[0] as f32 * Q11_SCALE, iq[1] as f32 * Q11_SCALE);
        }

        let first_sample_index = metadata.timestamp;
        let dropped_samples_before = self
            .expected_next_sample
            .map(|expected| first_sample_index.saturating_sub(expected))
            .unwrap_or(0);
        let overrun = metadata.status & BLADERF_META_STATUS_OVERRUN != 0
            || self
                .expected_next_sample
                .is_some_and(|expected| expected != first_sample_index);
        self.expected_next_sample = Some(
            first_sample_index
                .checked_add(actual_count as u64)
                .ok_or_else(|| Error::NativeCall {
                    backend: BACKEND,
                    operation: "sync_rx",
                    code: 0,
                    message: "libbladeRF sample timestamp overflow".to_owned(),
                })?,
        );

        Ok((
            actual_count,
            ReadMetadata {
                first_sample_index,
                dropped_samples_before,
                overrun,
            },
        ))
    }

    fn stop(&mut self) -> Result<()> {
        if self.state == DriverState::Running {
            check(
                self.api.as_ref(),
                "disable_module",
                self.api.enable_module(self.device, self.rx_channel, false),
            )?;
            self.state = DriverState::Configured;
        }
        Ok(())
    }
}

impl<A: BladeRfApi> Drop for BladeRfDriver<A> {
    fn drop(&mut self) {
        if self.state == DriverState::Running {
            let _ = self.api.enable_module(self.device, self.rx_channel, false);
        }
        self.api.close(self.device);
    }
}

pub struct BladeRfSource {
    driver: BladeRfDriver<DynamicBladeRfApi>,
}

impl BladeRfSource {
    pub fn open(identifier: Option<&str>, options: BladeRfOptions) -> Result<Self> {
        let api = Arc::new(DynamicBladeRfApi::load()?);
        Ok(Self {
            driver: BladeRfDriver::open(api, identifier, options)?,
        })
    }

    pub fn probe_library() -> Result<String> {
        let api = DynamicBladeRfApi::load()?;
        Ok(api.library_name().to_owned())
    }

    pub fn applied_config(&self) -> Option<AppliedBladeRfConfig> {
        self.driver.applied_config()
    }
}

impl IqSource for BladeRfSource {
    fn kind(&self) -> SdrKind {
        SdrKind::BladeRf
    }

    fn capabilities(&self) -> SdrCapabilities {
        bladerf_capabilities()
    }

    fn configure(&mut self, config: &SdrConfig) -> Result<()> {
        self.driver.configure(config)
    }

    fn applied_sample_rate_hz(&self) -> Option<u32> {
        self.driver
            .applied_config()
            .map(|config| config.sample_rate_hz)
    }

    fn start(&mut self) -> Result<()> {
        self.driver.start()
    }

    fn read(
        &mut self,
        output: &mut [Complex32],
        timeout: Duration,
    ) -> Result<(usize, ReadMetadata)> {
        self.driver.read(output, timeout)
    }

    fn stop(&mut self) -> Result<()> {
        self.driver.stop()
    }
}

fn bladerf_capabilities() -> SdrCapabilities {
    SdrCapabilities {
        minimum_frequency_hz: 47_000_000,
        maximum_frequency_hz: 6_000_000_000,
        maximum_sample_rate_hz: 61_440_000,
        // BLADERF_RX_X1 maps to RX0. RX1 requires an X2 stream and explicit
        // deinterleaving, which this single-channel backend does not yet expose.
        receive_channels: 1,
    }
}

fn native_error(api: &impl BladeRfApi, operation: &'static str, code: c_int) -> Error {
    Error::NativeCall {
        backend: BACKEND,
        operation,
        code,
        message: api.error_string(code),
    }
}

fn check(api: &impl BladeRfApi, operation: &'static str, status: c_int) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(native_error(api, operation, status))
    }
}

fn duration_to_timeout_ms(timeout: Duration) -> Result<u32> {
    let millis = timeout.as_millis();
    if millis == 0 {
        return Ok(1);
    }
    u32::try_from(millis).map_err(|_| {
        Error::InvalidConfiguration("bladeRF timeout exceeds u32 milliseconds".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct MockRx {
        native_status: c_int,
        timestamp: u64,
        status: u32,
        iq: Vec<i16>,
    }

    #[derive(Default)]
    struct MockState {
        calls: Vec<String>,
        receives: VecDeque<MockRx>,
        closed: bool,
        last_rx_flags: Option<u32>,
    }

    #[derive(Default)]
    struct MockApi {
        state: Mutex<MockState>,
    }

    impl MockApi {
        fn push_rx(&self, receive: MockRx) {
            self.state.lock().unwrap().receives.push_back(receive);
        }

        fn calls(&self) -> Vec<String> {
            self.state.lock().unwrap().calls.clone()
        }

        fn last_rx_flags(&self) -> Option<u32> {
            self.state.lock().unwrap().last_rx_flags
        }
    }

    impl BladeRfApi for MockApi {
        fn library_name(&self) -> &str {
            "mock"
        }

        fn open(&self, _identifier: Option<&CStr>) -> (c_int, *mut c_void) {
            self.state.lock().unwrap().calls.push("open".to_owned());
            (0, std::ptr::dangling_mut::<c_void>())
        }

        fn close(&self, _device: NonNull<c_void>) {
            let mut state = self.state.lock().unwrap();
            state.calls.push("close".to_owned());
            state.closed = true;
        }

        fn set_frequency(&self, _device: NonNull<c_void>, _channel: c_int, value: u64) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("frequency:{value}"));
            0
        }

        fn set_sample_rate(
            &self,
            _device: NonNull<c_void>,
            _channel: c_int,
            value: u32,
            actual: &mut u32,
        ) -> c_int {
            *actual = value - 1;
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("sample_rate:{value}"));
            0
        }

        fn set_bandwidth(
            &self,
            _device: NonNull<c_void>,
            _channel: c_int,
            value: u32,
            actual: &mut u32,
        ) -> c_int {
            *actual = value;
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("bandwidth:{value}"));
            0
        }

        fn set_gain(&self, _device: NonNull<c_void>, _channel: c_int, value: c_int) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("gain:{value}"));
            0
        }

        fn sync_config(&self, _device: NonNull<c_void>, _options: BladeRfOptions) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("sync_config".to_owned());
            0
        }

        fn enable_module(&self, _device: NonNull<c_void>, _channel: c_int, enable: bool) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("enable:{enable}"));
            0
        }

        fn sync_rx(
            &self,
            _device: NonNull<c_void>,
            samples: &mut [i16],
            _sample_count: u32,
            metadata: &mut BladeRfMetadata,
            _timeout_ms: u32,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.calls.push("sync_rx".to_owned());
            let receive = state.receives.pop_front().unwrap();
            state.last_rx_flags = Some(metadata.flags);
            if receive.native_status != 0 {
                return receive.native_status;
            }
            samples[..receive.iq.len()].copy_from_slice(&receive.iq);
            metadata.timestamp = receive.timestamp;
            metadata.status = receive.status;
            metadata.actual_count = (receive.iq.len() / 2) as u32;
            0
        }

        fn error_string(&self, code: c_int) -> String {
            format!("mock error {code}")
        }
    }

    fn config() -> SdrConfig {
        SdrConfig {
            center_frequency_hz: 2_426_000_000,
            sample_rate_hz: 4_000_000,
            bandwidth_hz: 2_000_000,
            gain_db: 27.6,
            channel: 0,
        }
    }

    #[test]
    fn lifecycle_converts_q11_and_reports_native_timestamp() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: 1_000,
            status: 0,
            iq: vec![2047, -2048, 1024, -1024],
        });
        let mut driver = BladeRfDriver::open(
            api.clone(),
            Some("*:serial=test"),
            BladeRfOptions::default(),
        )
        .unwrap();
        driver.configure(&config()).unwrap();
        assert_eq!(
            driver.applied_config(),
            Some(AppliedBladeRfConfig {
                sample_rate_hz: 3_999_999,
                bandwidth_hz: 2_000_000,
            })
        );
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 4];
        let (count, metadata) = driver
            .read(&mut output, Duration::from_millis(250))
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(metadata.first_sample_index, 1_000);
        assert_eq!(metadata.dropped_samples_before, 0);
        assert!(!metadata.overrun);
        assert_eq!(api.last_rx_flags(), Some(BLADERF_META_FLAG_RX_NOW));
        assert!((output[0].re - 2047.0 / 2048.0).abs() < 1.0e-6);
        assert_eq!(output[0].im, -1.0);
        assert_eq!(output[1], Complex32::new(0.5, -0.5));
        driver.stop().unwrap();
        drop(driver);

        let calls = api.calls();
        assert_eq!(
            calls,
            [
                "open",
                "sample_rate:4000000",
                "bandwidth:2000000",
                "frequency:2426000000",
                "gain:28",
                "sync_config",
                "enable:true",
                "sync_rx",
                "enable:false",
                "close",
            ]
        );
    }

    #[test]
    fn timestamp_gap_and_native_status_report_overrun() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: 100,
            status: 0,
            iq: vec![0, 0, 0, 0],
        });
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: 110,
            status: BLADERF_META_STATUS_OVERRUN,
            iq: vec![0, 0],
        });
        let mut driver = BladeRfDriver::open(api, None, BladeRfOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        driver.read(&mut output, Duration::from_millis(1)).unwrap();
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.first_sample_index, 110);
        assert_eq!(metadata.dropped_samples_before, 8);
        assert!(metadata.overrun);
    }

    #[test]
    fn backward_timestamp_is_reported_without_wrapping_drop_count() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: 100,
            status: 0,
            iq: vec![0, 0, 0, 0],
        });
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: 90,
            status: 0,
            iq: vec![0, 0],
        });
        let mut driver = BladeRfDriver::open(api, None, BladeRfOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        driver.read(&mut output, Duration::from_millis(1)).unwrap();
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.first_sample_index, 90);
        assert_eq!(metadata.dropped_samples_before, 0);
        assert!(metadata.overrun);
    }

    #[test]
    fn native_timeout_is_a_recoverable_empty_read() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_status: BLADERF_ERR_TIMEOUT,
            timestamp: 0,
            status: 0,
            iq: Vec::new(),
        });
        let mut driver = BladeRfDriver::open(api.clone(), None, BladeRfOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        let (count, metadata) = driver.read(&mut output, Duration::from_millis(5)).unwrap();
        assert_eq!(count, 0);
        assert_eq!(metadata.first_sample_index, 0);
        assert_eq!(api.last_rx_flags(), Some(BLADERF_META_FLAG_RX_NOW));
    }

    #[test]
    fn rejects_timestamp_overflow_from_native_metadata() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_status: 0,
            timestamp: u64::MAX,
            status: 0,
            iq: vec![0, 0],
        });
        let mut driver = BladeRfDriver::open(api, None, BladeRfOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert!(
            driver
                .read(&mut output, Duration::from_millis(5))
                .unwrap_err()
                .to_string()
                .contains("timestamp overflow")
        );
    }

    #[test]
    fn rejects_invalid_lifecycle_transitions() {
        let api = Arc::new(MockApi::default());
        let mut driver = BladeRfDriver::open(api, None, BladeRfOptions::default()).unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert!(
            driver
                .read(&mut output, Duration::from_millis(1))
                .unwrap_err()
                .to_string()
                .contains("running")
        );
        assert!(
            driver
                .start()
                .unwrap_err()
                .to_string()
                .contains("configured")
        );
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        assert!(
            driver
                .configure(&config())
                .unwrap_err()
                .to_string()
                .contains("while streaming")
        );
        driver.stop().unwrap();
        driver.stop().unwrap();
    }

    #[test]
    fn validates_stream_options_and_timeout() {
        assert!(
            BladeRfOptions {
                num_buffers: 4,
                num_transfers: 4,
                ..BladeRfOptions::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            BladeRfOptions {
                buffer_size: 2_049,
                ..BladeRfOptions::default()
            }
            .validate()
            .is_err()
        );
        assert_eq!(duration_to_timeout_ms(Duration::ZERO).unwrap(), 1);
        assert_eq!(duration_to_timeout_ms(Duration::from_micros(1)).unwrap(), 1);
    }

    #[test]
    fn single_channel_stream_rejects_rx1() {
        let api = Arc::new(MockApi::default());
        let mut driver = BladeRfDriver::open(api, None, BladeRfOptions::default()).unwrap();
        let mut rx1 = config();
        rx1.channel = 1;
        assert!(
            driver
                .configure(&rx1)
                .unwrap_err()
                .to_string()
                .contains("unavailable")
        );
    }

    #[test]
    fn drop_disables_running_stream_before_close() {
        let api = Arc::new(MockApi::default());
        let mut driver = BladeRfDriver::open(api.clone(), None, BladeRfOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        drop(driver);
        let calls = api.calls();
        assert_eq!(
            &calls[calls.len() - 3..],
            ["enable:true", "enable:false", "close"]
        );
    }

    #[test]
    fn metadata_layout_matches_current_64_bit_vendor_abi() {
        assert_eq!(std::mem::size_of::<BladeRfMetadata>(), 56);
        assert_eq!(std::mem::align_of::<BladeRfMetadata>(), 8);
    }
}
