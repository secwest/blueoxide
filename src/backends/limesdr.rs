use crate::complex::Complex32;
use crate::native::DynamicLibrary;
use crate::sdr::{IqSource, ReadMetadata, SdrCapabilities, SdrConfig, SdrKind};
use crate::{Error, Result};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

const BACKEND: &str = "LimeSDR";
const LMS_CH_RX: bool = false;
const LMS_FMT_F32: c_int = 0;
const LMS_LINK_FMT_DEFAULT: c_int = 0;
const MAX_GAIN_DB: f32 = 73.0;
const MIN_CALIBRATION_BANDWIDTH_HZ: f64 = 2_500_000.0;

#[derive(Clone, Copy, Debug)]
pub struct LimeSdrOptions {
    pub fifo_size: u32,
    pub throughput_vs_latency: f32,
    pub oversample: usize,
    pub calibrate: bool,
}

impl Default for LimeSdrOptions {
    fn default() -> Self {
        Self {
            fifo_size: 1_048_576,
            throughput_vs_latency: 1.0,
            oversample: 0,
            calibrate: true,
        }
    }
}

impl LimeSdrOptions {
    fn validate(self) -> Result<()> {
        if self.fifo_size == 0 {
            return Err(Error::InvalidConfiguration(
                "LimeSDR fifo_size must be greater than zero".to_owned(),
            ));
        }
        if !self.throughput_vs_latency.is_finite()
            || !(0.0..=1.0).contains(&self.throughput_vs_latency)
        {
            return Err(Error::InvalidConfiguration(
                "LimeSDR throughput_vs_latency must be finite and in 0..=1".to_owned(),
            ));
        }
        if !matches!(self.oversample, 0 | 1 | 2 | 4 | 8 | 16 | 32) {
            return Err(Error::InvalidConfiguration(
                "LimeSDR oversample must be one of 0, 1, 2, 4, 8, 16, or 32".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppliedLimeSdrConfig {
    pub sample_rate_hz: u32,
    pub bandwidth_hz: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct LmsRange {
    min: f64,
    max: f64,
    step: f64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct LmsStreamMetadata {
    timestamp: u64,
    wait_for_timestamp: bool,
    flush_partial_packet: bool,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct LmsStream {
    handle: usize,
    is_tx: bool,
    channel: u32,
    fifo_size: u32,
    throughput_vs_latency: f32,
    data_fmt: c_int,
    link_fmt: c_int,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct LmsStreamStatus {
    active: bool,
    fifo_filled_count: u32,
    fifo_size: u32,
    underrun: u32,
    overrun: u32,
    dropped_packets: u32,
    sample_rate: f64,
    link_rate: f64,
    timestamp: u64,
}

trait LimeApi {
    fn library_name(&self) -> &str;
    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void);
    fn close(&self, device: NonNull<c_void>) -> c_int;
    fn init(&self, device: NonNull<c_void>) -> c_int;
    fn get_num_channels(&self, device: NonNull<c_void>) -> c_int;
    fn get_frequency_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int;
    fn get_sample_rate_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int;
    fn get_lpf_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int;
    fn enable_channel(&self, device: NonNull<c_void>, channel: usize, enabled: bool) -> c_int;
    fn set_sample_rate(&self, device: NonNull<c_void>, rate_hz: f64, oversample: usize) -> c_int;
    fn get_sample_rate(
        &self,
        device: NonNull<c_void>,
        channel: usize,
        host_hz: &mut f64,
        rf_hz: &mut f64,
    ) -> c_int;
    fn set_frequency(&self, device: NonNull<c_void>, channel: usize, frequency_hz: f64) -> c_int;
    fn set_lpf(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int;
    fn get_lpf(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: &mut f64) -> c_int;
    fn set_gain(&self, device: NonNull<c_void>, channel: usize, gain_db: u32) -> c_int;
    fn calibrate(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int;
    fn setup_stream(&self, device: NonNull<c_void>, stream: &mut LmsStream) -> c_int;
    fn destroy_stream(&self, device: NonNull<c_void>, stream: &mut LmsStream) -> c_int;
    fn start_stream(&self, stream: &mut LmsStream) -> c_int;
    fn stop_stream(&self, stream: &mut LmsStream) -> c_int;
    fn recv_stream(
        &self,
        stream: &mut LmsStream,
        samples: &mut [f32],
        sample_count: usize,
        metadata: &mut LmsStreamMetadata,
        timeout_ms: u32,
    ) -> c_int;
    fn get_stream_status(&self, stream: &mut LmsStream, status: &mut LmsStreamStatus) -> c_int;
    fn error_string(&self) -> String;
}

type OpenFn = unsafe extern "C" fn(*mut *mut c_void, *const c_char, *mut c_void) -> c_int;
type CloseFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type InitFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type GetNumChannelsFn = unsafe extern "C" fn(*mut c_void, bool) -> c_int;
type GetRangeFn = unsafe extern "C" fn(*mut c_void, bool, *mut LmsRange) -> c_int;
type EnableChannelFn = unsafe extern "C" fn(*mut c_void, bool, usize, bool) -> c_int;
type SetSampleRateFn = unsafe extern "C" fn(*mut c_void, f64, usize) -> c_int;
type GetSampleRateFn = unsafe extern "C" fn(*mut c_void, bool, usize, *mut f64, *mut f64) -> c_int;
type SetFrequencyFn = unsafe extern "C" fn(*mut c_void, bool, usize, f64) -> c_int;
type SetLpfFn = unsafe extern "C" fn(*mut c_void, bool, usize, f64) -> c_int;
type GetLpfFn = unsafe extern "C" fn(*mut c_void, bool, usize, *mut f64) -> c_int;
type SetGainFn = unsafe extern "C" fn(*mut c_void, bool, usize, c_uint) -> c_int;
type CalibrateFn = unsafe extern "C" fn(*mut c_void, bool, usize, f64, c_uint) -> c_int;
type SetupStreamFn = unsafe extern "C" fn(*mut c_void, *mut LmsStream) -> c_int;
type DestroyStreamFn = unsafe extern "C" fn(*mut c_void, *mut LmsStream) -> c_int;
type StreamControlFn = unsafe extern "C" fn(*mut LmsStream) -> c_int;
type RecvStreamFn = unsafe extern "C" fn(
    *mut LmsStream,
    *mut c_void,
    usize,
    *mut LmsStreamMetadata,
    c_uint,
) -> c_int;
type GetStreamStatusFn = unsafe extern "C" fn(*mut LmsStream, *mut LmsStreamStatus) -> c_int;
type GetLastErrorFn = unsafe extern "C" fn() -> *const c_char;

struct DynamicLimeApi {
    _library: DynamicLibrary,
    library_name: String,
    open: OpenFn,
    close: CloseFn,
    init: InitFn,
    get_num_channels: GetNumChannelsFn,
    get_frequency_range: GetRangeFn,
    get_sample_rate_range: GetRangeFn,
    get_lpf_range: GetRangeFn,
    enable_channel: EnableChannelFn,
    set_sample_rate: SetSampleRateFn,
    get_sample_rate: GetSampleRateFn,
    set_frequency: SetFrequencyFn,
    set_lpf: SetLpfFn,
    get_lpf: GetLpfFn,
    set_gain: SetGainFn,
    calibrate: CalibrateFn,
    setup_stream: SetupStreamFn,
    destroy_stream: DestroyStreamFn,
    start_stream: StreamControlFn,
    stop_stream: StreamControlFn,
    recv_stream: RecvStreamFn,
    get_stream_status: GetStreamStatusFn,
    get_last_error: GetLastErrorFn,
}

impl DynamicLimeApi {
    fn load() -> Result<Self> {
        let names = match std::env::var_os("BLUEOXIDE_LIMESUITE_LIBRARY") {
            Some(override_name) if override_name.is_empty() => {
                return Err(Error::InvalidConfiguration(
                    "BLUEOXIDE_LIMESUITE_LIBRARY must not be empty".to_owned(),
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

        // SAFETY: Function signatures and structures are transcribed from
        // LimeSuite.h at the pinned revision documented in Verification.md.
        unsafe {
            Ok(Self {
                open: library.symbol("LMS_Open")?,
                close: library.symbol("LMS_Close")?,
                init: library.symbol("LMS_Init")?,
                get_num_channels: library.symbol("LMS_GetNumChannels")?,
                get_frequency_range: library.symbol("LMS_GetLOFrequencyRange")?,
                get_sample_rate_range: library.symbol("LMS_GetSampleRateRange")?,
                get_lpf_range: library.symbol("LMS_GetLPFBWRange")?,
                enable_channel: library.symbol("LMS_EnableChannel")?,
                set_sample_rate: library.symbol("LMS_SetSampleRate")?,
                get_sample_rate: library.symbol("LMS_GetSampleRate")?,
                set_frequency: library.symbol("LMS_SetLOFrequency")?,
                set_lpf: library.symbol("LMS_SetLPFBW")?,
                get_lpf: library.symbol("LMS_GetLPFBW")?,
                set_gain: library.symbol("LMS_SetGaindB")?,
                calibrate: library.symbol("LMS_Calibrate")?,
                setup_stream: library.symbol("LMS_SetupStream")?,
                destroy_stream: library.symbol("LMS_DestroyStream")?,
                start_stream: library.symbol("LMS_StartStream")?,
                stop_stream: library.symbol("LMS_StopStream")?,
                recv_stream: library.symbol("LMS_RecvStream")?,
                get_stream_status: library.symbol("LMS_GetStreamStatus")?,
                get_last_error: library.symbol("LMS_GetLastErrorMessage")?,
                _library: library,
                library_name,
            })
        }
    }
}

impl LimeApi for DynamicLimeApi {
    fn library_name(&self) -> &str {
        &self.library_name
    }

    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void) {
        let mut device = std::ptr::null_mut();
        // SAFETY: The function pointer has the reviewed ABI, device is writable,
        // and identifier is NULL or a NUL-terminated LimeSuite device string.
        let status = unsafe {
            (self.open)(
                &mut device,
                identifier.map_or(std::ptr::null(), CStr::as_ptr),
                std::ptr::null_mut(),
            )
        };
        (status, device)
    }

    fn close(&self, device: NonNull<c_void>) -> c_int {
        // SAFETY: device was returned by LMS_Open and remains owned here.
        unsafe { (self.close)(device.as_ptr()) }
    }

    fn init(&self, device: NonNull<c_void>) -> c_int {
        // SAFETY: device is open.
        unsafe { (self.init)(device.as_ptr()) }
    }

    fn get_num_channels(&self, device: NonNull<c_void>) -> c_int {
        // SAFETY: device is initialized and false selects RX.
        unsafe { (self.get_num_channels)(device.as_ptr(), LMS_CH_RX) }
    }

    fn get_frequency_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
        // SAFETY: range is writable and false selects RX.
        unsafe { (self.get_frequency_range)(device.as_ptr(), LMS_CH_RX, range) }
    }

    fn get_sample_rate_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
        // SAFETY: range is writable and false selects RX.
        unsafe { (self.get_sample_rate_range)(device.as_ptr(), LMS_CH_RX, range) }
    }

    fn get_lpf_range(&self, device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
        // SAFETY: range is writable and false selects RX.
        unsafe { (self.get_lpf_range)(device.as_ptr(), LMS_CH_RX, range) }
    }

    fn enable_channel(&self, device: NonNull<c_void>, channel: usize, enabled: bool) -> c_int {
        // SAFETY: device is initialized and channel was capability-validated.
        unsafe { (self.enable_channel)(device.as_ptr(), LMS_CH_RX, channel, enabled) }
    }

    fn set_sample_rate(&self, device: NonNull<c_void>, rate_hz: f64, oversample: usize) -> c_int {
        // SAFETY: device is initialized and values were validated.
        unsafe { (self.set_sample_rate)(device.as_ptr(), rate_hz, oversample) }
    }

    fn get_sample_rate(
        &self,
        device: NonNull<c_void>,
        channel: usize,
        host_hz: &mut f64,
        rf_hz: &mut f64,
    ) -> c_int {
        // SAFETY: output pointers are writable and channel is valid.
        unsafe { (self.get_sample_rate)(device.as_ptr(), LMS_CH_RX, channel, host_hz, rf_hz) }
    }

    fn set_frequency(&self, device: NonNull<c_void>, channel: usize, frequency_hz: f64) -> c_int {
        // SAFETY: device and channel are valid and frequency is finite.
        unsafe { (self.set_frequency)(device.as_ptr(), LMS_CH_RX, channel, frequency_hz) }
    }

    fn set_lpf(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int {
        // SAFETY: device and channel are valid and bandwidth is finite.
        unsafe { (self.set_lpf)(device.as_ptr(), LMS_CH_RX, channel, bandwidth_hz) }
    }

    fn get_lpf(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: &mut f64) -> c_int {
        // SAFETY: output pointer is writable and channel is valid.
        unsafe { (self.get_lpf)(device.as_ptr(), LMS_CH_RX, channel, bandwidth_hz) }
    }

    fn set_gain(&self, device: NonNull<c_void>, channel: usize, gain_db: u32) -> c_int {
        // SAFETY: device and channel are valid and gain is in the API range.
        unsafe { (self.set_gain)(device.as_ptr(), LMS_CH_RX, channel, gain_db) }
    }

    fn calibrate(&self, device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int {
        // SAFETY: device and channel are fully configured for RX calibration.
        unsafe { (self.calibrate)(device.as_ptr(), LMS_CH_RX, channel, bandwidth_hz, 0) }
    }

    fn setup_stream(&self, device: NonNull<c_void>, stream: &mut LmsStream) -> c_int {
        // SAFETY: stream has the reviewed ABI and is writable.
        unsafe { (self.setup_stream)(device.as_ptr(), stream) }
    }

    fn destroy_stream(&self, device: NonNull<c_void>, stream: &mut LmsStream) -> c_int {
        // SAFETY: stream was initialized by LMS_SetupStream for this device.
        unsafe { (self.destroy_stream)(device.as_ptr(), stream) }
    }

    fn start_stream(&self, stream: &mut LmsStream) -> c_int {
        // SAFETY: stream has a live LimeSuite handle.
        unsafe { (self.start_stream)(stream) }
    }

    fn stop_stream(&self, stream: &mut LmsStream) -> c_int {
        // SAFETY: stream has a live LimeSuite handle.
        unsafe { (self.stop_stream)(stream) }
    }

    fn recv_stream(
        &self,
        stream: &mut LmsStream,
        samples: &mut [f32],
        sample_count: usize,
        metadata: &mut LmsStreamMetadata,
        timeout_ms: u32,
    ) -> c_int {
        // SAFETY: samples contains two f32 scalars per requested complex
        // sample, stream is running, and metadata is writable.
        unsafe {
            (self.recv_stream)(
                stream,
                samples.as_mut_ptr().cast(),
                sample_count,
                metadata,
                timeout_ms,
            )
        }
    }

    fn get_stream_status(&self, stream: &mut LmsStream, status: &mut LmsStreamStatus) -> c_int {
        // SAFETY: stream is live and status is writable.
        unsafe { (self.get_stream_status)(stream, status) }
    }

    fn error_string(&self) -> String {
        // SAFETY: LMS_GetLastErrorMessage has no arguments.
        let pointer = unsafe { (self.get_last_error)() };
        if pointer.is_null() {
            "unknown LimeSuite error".to_owned()
        } else {
            // SAFETY: LimeSuite returns a library-owned NUL-terminated string.
            unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(windows)]
fn default_library_names() -> &'static [&'static str] {
    &["LimeSuite.dll", "libLimeSuite.dll"]
}

#[cfg(target_os = "macos")]
fn default_library_names() -> &'static [&'static str] {
    &["libLimeSuite.dylib"]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_library_names() -> &'static [&'static str] {
    &["libLimeSuite.so"]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverState {
    Open,
    Configured,
    Running,
}

struct LimeDriver<A: LimeApi> {
    api: Arc<A>,
    device: NonNull<c_void>,
    options: LimeSdrOptions,
    state: DriverState,
    capabilities: SdrCapabilities,
    lpf_range: LmsRange,
    enabled_channel: Option<usize>,
    stream: Option<LmsStream>,
    native_samples: Vec<f32>,
    expected_next_sample: Option<u64>,
    applied: Option<AppliedLimeSdrConfig>,
}

impl<A: LimeApi> LimeDriver<A> {
    fn open(api: Arc<A>, identifier: Option<&str>, options: LimeSdrOptions) -> Result<Self> {
        options.validate()?;
        let identifier = identifier
            .map(|value| {
                CString::new(value).map_err(|_| {
                    Error::InvalidConfiguration(
                        "LimeSuite device identifier contains a NUL octet".to_owned(),
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
                message: "LimeSuite returned success with a NULL device".to_owned(),
            });
        };
        if let Err(error) = check(api.as_ref(), "init", api.init(device)) {
            let _ = api.close(device);
            return Err(error);
        }

        let capabilities = match query_capabilities(api.as_ref(), device) {
            Ok(capabilities) => capabilities,
            Err(error) => {
                let _ = api.close(device);
                return Err(error);
            }
        };
        let mut lpf_range = LmsRange::default();
        if let Err(error) = check(
            api.as_ref(),
            "get_lpf_range",
            api.get_lpf_range(device, &mut lpf_range),
        ) {
            let _ = api.close(device);
            return Err(error);
        }
        if let Err(error) = validate_range("LimeSDR LPF", lpf_range) {
            let _ = api.close(device);
            return Err(error);
        }

        Ok(Self {
            api,
            device,
            options,
            state: DriverState::Open,
            capabilities,
            lpf_range,
            enabled_channel: None,
            stream: None,
            native_samples: Vec::new(),
            expected_next_sample: None,
            applied: None,
        })
    }

    fn capabilities(&self) -> SdrCapabilities {
        self.capabilities
    }

    fn applied_config(&self) -> Option<AppliedLimeSdrConfig> {
        self.applied
    }

    fn configure(&mut self, config: &SdrConfig) -> Result<()> {
        if self.state == DriverState::Running {
            return Err(Error::InvalidState(
                "LimeSDR cannot be configured while streaming".to_owned(),
            ));
        }
        if self.stream.is_some() || self.enabled_channel.is_some() {
            self.release_configuration()?;
        }
        config.validate(self.capabilities)?;
        let bandwidth_hz = config.bandwidth_hz as f64;
        if bandwidth_hz < self.lpf_range.min || bandwidth_hz > self.lpf_range.max {
            return Err(Error::InvalidConfiguration(format!(
                "LimeSDR bandwidth {} Hz is outside device LPF range {:.0}..={:.0} Hz",
                config.bandwidth_hz, self.lpf_range.min, self.lpf_range.max
            )));
        }
        if !(0.0..=MAX_GAIN_DB).contains(&config.gain_db) {
            return Err(Error::InvalidConfiguration(format!(
                "LimeSDR gain {} dB is outside supported range 0..={MAX_GAIN_DB} dB",
                config.gain_db
            )));
        }

        let channel = config.channel as usize;
        check(
            self.api.as_ref(),
            "enable_channel",
            self.api.enable_channel(self.device, channel, true),
        )?;
        self.enabled_channel = Some(channel);

        let configuration_result = self.configure_enabled_channel(config, channel);
        if configuration_result.is_err()
            && self.api.enable_channel(self.device, channel, false) == 0
        {
            self.enabled_channel = None;
        }
        configuration_result
    }

    fn configure_enabled_channel(&mut self, config: &SdrConfig, channel: usize) -> Result<()> {
        check(
            self.api.as_ref(),
            "set_sample_rate",
            self.api.set_sample_rate(
                self.device,
                config.sample_rate_hz as f64,
                self.options.oversample,
            ),
        )?;
        let mut actual_sample_rate = 0.0;
        let mut rf_sample_rate = 0.0;
        check(
            self.api.as_ref(),
            "get_sample_rate",
            self.api.get_sample_rate(
                self.device,
                channel,
                &mut actual_sample_rate,
                &mut rf_sample_rate,
            ),
        )?;
        let actual_sample_rate = exact_u32_hz("LimeSDR applied sample rate", actual_sample_rate)?;

        check(
            self.api.as_ref(),
            "set_frequency",
            self.api
                .set_frequency(self.device, channel, config.center_frequency_hz as f64),
        )?;
        check(
            self.api.as_ref(),
            "set_lpf",
            self.api
                .set_lpf(self.device, channel, config.bandwidth_hz as f64),
        )?;
        let mut actual_bandwidth = 0.0;
        check(
            self.api.as_ref(),
            "get_lpf",
            self.api
                .get_lpf(self.device, channel, &mut actual_bandwidth),
        )?;
        let actual_bandwidth = exact_u32_hz("LimeSDR applied bandwidth", actual_bandwidth)?;

        let gain = config.gain_db.round() as u32;
        check(
            self.api.as_ref(),
            "set_gain",
            self.api.set_gain(self.device, channel, gain),
        )?;
        if self.options.calibrate {
            let calibration_bandwidth =
                (config.bandwidth_hz as f64).max(MIN_CALIBRATION_BANDWIDTH_HZ);
            check(
                self.api.as_ref(),
                "calibrate",
                self.api
                    .calibrate(self.device, channel, calibration_bandwidth),
            )?;
        }

        let mut stream = LmsStream {
            handle: 0,
            is_tx: false,
            channel: config.channel as u32,
            fifo_size: self.options.fifo_size,
            throughput_vs_latency: self.options.throughput_vs_latency,
            data_fmt: LMS_FMT_F32,
            link_fmt: LMS_LINK_FMT_DEFAULT,
        };
        let setup_status = self.api.setup_stream(self.device, &mut stream);
        if setup_status != 0 {
            if stream.handle != 0 {
                let _ = self.api.destroy_stream(self.device, &mut stream);
            }
            return Err(native_error(
                self.api.as_ref(),
                "setup_stream",
                setup_status,
            ));
        }
        if stream.handle == 0 {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "setup_stream",
                code: 0,
                message: "LimeSuite returned success with a zero stream handle".to_owned(),
            });
        }

        self.stream = Some(stream);
        self.expected_next_sample = None;
        self.applied = Some(AppliedLimeSdrConfig {
            sample_rate_hz: actual_sample_rate,
            bandwidth_hz: actual_bandwidth,
        });
        self.state = DriverState::Configured;
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        match self.state {
            DriverState::Open => Err(Error::InvalidState(
                "LimeSDR must be configured before start".to_owned(),
            )),
            DriverState::Configured => {
                let stream = self.stream.as_mut().ok_or_else(|| {
                    Error::InvalidState("LimeSDR configured without a stream".to_owned())
                })?;
                check(
                    self.api.as_ref(),
                    "start_stream",
                    self.api.start_stream(stream),
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
                "LimeSDR read requires a running stream".to_owned(),
            ));
        }
        if output.is_empty() {
            return Ok((0, ReadMetadata::default()));
        }
        if output.len() > c_int::MAX as usize {
            return Err(Error::InvalidConfiguration(
                "LimeSDR read buffer exceeds native signed sample count".to_owned(),
            ));
        }
        let scalar_count = output.len().checked_mul(2).ok_or_else(|| {
            Error::InvalidConfiguration("LimeSDR native buffer size overflow".to_owned())
        })?;
        self.native_samples.resize(scalar_count, 0.0);
        let timeout_ms = duration_to_timeout_ms(timeout)?;
        let mut metadata = LmsStreamMetadata::default();
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::InvalidState("LimeSDR running without a stream".to_owned()))?;
        let count = self.api.recv_stream(
            stream,
            &mut self.native_samples,
            output.len(),
            &mut metadata,
            timeout_ms,
        );
        if count < 0 {
            return Err(native_error(self.api.as_ref(), "recv_stream", count));
        }
        let count = count as usize;
        if count == 0 {
            return Ok((0, ReadMetadata::default()));
        }
        if count > output.len() {
            return Err(Error::NativeCall {
                backend: BACKEND,
                operation: "recv_stream",
                code: 0,
                message: format!(
                    "LimeSuite reported {count} samples for a {}-sample buffer",
                    output.len()
                ),
            });
        }
        for (index, (destination, iq)) in output[..count]
            .iter_mut()
            .zip(self.native_samples[..count * 2].chunks_exact(2))
            .enumerate()
        {
            if !iq[0].is_finite() || !iq[1].is_finite() {
                return Err(Error::NativeCall {
                    backend: BACKEND,
                    operation: "recv_stream",
                    code: 0,
                    message: format!("LimeSuite returned non-finite I/Q at sample {index}"),
                });
            }
            *destination = Complex32::new(iq[0], iq[1]);
        }

        let mut status = LmsStreamStatus::default();
        check(
            self.api.as_ref(),
            "get_stream_status",
            self.api.get_stream_status(stream, &mut status),
        )?;

        let first_sample_index = metadata.timestamp;
        let dropped_samples_before = self
            .expected_next_sample
            .map(|expected| first_sample_index.saturating_sub(expected))
            .unwrap_or(0);
        let overrun = status.overrun != 0
            || status.dropped_packets != 0
            || self
                .expected_next_sample
                .is_some_and(|expected| expected != first_sample_index);
        self.expected_next_sample = Some(first_sample_index.checked_add(count as u64).ok_or_else(
            || Error::NativeCall {
                backend: BACKEND,
                operation: "recv_stream",
                code: 0,
                message: "LimeSuite sample timestamp overflow".to_owned(),
            },
        )?);

        Ok((
            count,
            ReadMetadata {
                first_sample_index,
                dropped_samples_before,
                overrun,
            },
        ))
    }

    fn stop(&mut self) -> Result<()> {
        if self.state == DriverState::Running {
            let stream = self.stream.as_mut().ok_or_else(|| {
                Error::InvalidState("LimeSDR running without a stream".to_owned())
            })?;
            check(
                self.api.as_ref(),
                "stop_stream",
                self.api.stop_stream(stream),
            )?;
            self.state = DriverState::Configured;
        }
        Ok(())
    }

    fn release_configuration(&mut self) -> Result<()> {
        if self.state == DriverState::Running {
            return Err(Error::InvalidState(
                "LimeSDR cannot release a running stream".to_owned(),
            ));
        }
        if let Some(mut stream) = self.stream.take() {
            let status = self.api.destroy_stream(self.device, &mut stream);
            if status != 0 {
                self.stream = Some(stream);
                return Err(native_error(self.api.as_ref(), "destroy_stream", status));
            }
        }
        if let Some(channel) = self.enabled_channel {
            let status = self.api.enable_channel(self.device, channel, false);
            if status != 0 {
                return Err(native_error(self.api.as_ref(), "disable_channel", status));
            }
            self.enabled_channel = None;
        }
        self.applied = None;
        self.expected_next_sample = None;
        self.state = DriverState::Open;
        Ok(())
    }
}

impl<A: LimeApi> Drop for LimeDriver<A> {
    fn drop(&mut self) {
        if self.state == DriverState::Running
            && let Some(stream) = &mut self.stream
        {
            let _ = self.api.stop_stream(stream);
        }
        if let Some(mut stream) = self.stream.take() {
            let _ = self.api.destroy_stream(self.device, &mut stream);
        }
        if let Some(channel) = self.enabled_channel.take() {
            let _ = self.api.enable_channel(self.device, channel, false);
        }
        let _ = self.api.close(self.device);
    }
}

pub struct LimeSdrSource {
    driver: LimeDriver<DynamicLimeApi>,
}

impl LimeSdrSource {
    pub fn open(identifier: Option<&str>, options: LimeSdrOptions) -> Result<Self> {
        let api = Arc::new(DynamicLimeApi::load()?);
        Ok(Self {
            driver: LimeDriver::open(api, identifier, options)?,
        })
    }

    pub fn probe_library() -> Result<String> {
        let api = DynamicLimeApi::load()?;
        Ok(api.library_name().to_owned())
    }

    pub fn applied_config(&self) -> Option<AppliedLimeSdrConfig> {
        self.driver.applied_config()
    }
}

impl IqSource for LimeSdrSource {
    fn kind(&self) -> SdrKind {
        SdrKind::LimeSdr
    }

    fn capabilities(&self) -> SdrCapabilities {
        self.driver.capabilities()
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

fn query_capabilities(api: &impl LimeApi, device: NonNull<c_void>) -> Result<SdrCapabilities> {
    let channel_count = api.get_num_channels(device);
    if channel_count <= 0 || channel_count > u8::MAX as c_int {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "get_num_channels",
            code: channel_count,
            message: if channel_count < 0 {
                api.error_string()
            } else {
                format!("LimeSuite returned invalid RX channel count {channel_count}")
            },
        });
    }
    let mut frequency_range = LmsRange::default();
    check(
        api,
        "get_frequency_range",
        api.get_frequency_range(device, &mut frequency_range),
    )?;
    validate_range("LimeSDR frequency", frequency_range)?;
    let mut sample_rate_range = LmsRange::default();
    check(
        api,
        "get_sample_rate_range",
        api.get_sample_rate_range(device, &mut sample_rate_range),
    )?;
    validate_range("LimeSDR sample rate", sample_rate_range)?;

    let minimum_frequency_hz = ceil_u64("LimeSDR minimum frequency", frequency_range.min)?;
    let maximum_frequency_hz = floor_u64("LimeSDR maximum frequency", frequency_range.max)?;
    let maximum_sample_rate_hz = floor_u32("LimeSDR maximum sample rate", sample_rate_range.max)?;
    if minimum_frequency_hz > maximum_frequency_hz {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "get_frequency_range",
            code: 0,
            message: "LimeSuite returned an empty integer frequency range".to_owned(),
        });
    }
    Ok(SdrCapabilities {
        minimum_frequency_hz,
        maximum_frequency_hz,
        maximum_sample_rate_hz,
        receive_channels: channel_count as u8,
    })
}

fn validate_range(name: &str, range: LmsRange) -> Result<()> {
    if !range.min.is_finite()
        || !range.max.is_finite()
        || !range.step.is_finite()
        || range.min < 0.0
        || range.max < range.min
    {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "query_range",
            code: 0,
            message: format!(
                "{name} range is invalid: min={}, max={}, step={}",
                range.min, range.max, range.step
            ),
        });
    }
    Ok(())
}

fn exact_u32_hz(name: &str, value: f64) -> Result<u32> {
    if !value.is_finite() || value <= 0.0 || value > u32::MAX as f64 {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "read_applied_value",
            code: 0,
            message: format!("{name} is invalid: {value}"),
        });
    }
    let rounded = value.round();
    if (value - rounded).abs() > 1.0e-6 {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "read_applied_value",
            code: 0,
            message: format!("{name} is not an integer number of hertz: {value}"),
        });
    }
    Ok(rounded as u32)
}

fn ceil_u64(name: &str, value: f64) -> Result<u64> {
    if !value.is_finite() || value < 0.0 || value > u64::MAX as f64 {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "query_range",
            code: 0,
            message: format!("{name} is invalid: {value}"),
        });
    }
    Ok(value.ceil() as u64)
}

fn floor_u64(name: &str, value: f64) -> Result<u64> {
    if !value.is_finite() || value < 0.0 || value > u64::MAX as f64 {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "query_range",
            code: 0,
            message: format!("{name} is invalid: {value}"),
        });
    }
    Ok(value.floor() as u64)
}

fn floor_u32(name: &str, value: f64) -> Result<u32> {
    if !value.is_finite() || value < 0.0 || value > u32::MAX as f64 {
        return Err(Error::NativeCall {
            backend: BACKEND,
            operation: "query_range",
            code: 0,
            message: format!("{name} is invalid: {value}"),
        });
    }
    Ok(value.floor() as u32)
}

fn native_error(api: &impl LimeApi, operation: &'static str, code: c_int) -> Error {
    Error::NativeCall {
        backend: BACKEND,
        operation,
        code,
        message: api.error_string(),
    }
}

fn check(api: &impl LimeApi, operation: &'static str, status: c_int) -> Result<()> {
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
    u32::try_from(millis)
        .map_err(|_| Error::InvalidConfiguration("LimeSDR timeout exceeds u32 milliseconds".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct MockRx {
        native_result: c_int,
        timestamp: u64,
        iq: Vec<f32>,
        overrun: u32,
        dropped_packets: u32,
    }

    struct MockState {
        calls: Vec<String>,
        receives: VecDeque<MockRx>,
        pending_status: Option<(u32, u32)>,
        requested_sample_rate: f64,
        requested_bandwidth: f64,
        failure: Option<&'static str>,
    }

    impl Default for MockState {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                receives: VecDeque::new(),
                pending_status: None,
                requested_sample_rate: 0.0,
                requested_bandwidth: 0.0,
                failure: None,
            }
        }
    }

    #[derive(Default)]
    struct MockApi {
        state: Mutex<MockState>,
    }

    impl MockApi {
        fn with_failure(operation: &'static str) -> Self {
            let api = Self::default();
            api.state.lock().unwrap().failure = Some(operation);
            api
        }

        fn push_rx(&self, receive: MockRx) {
            self.state.lock().unwrap().receives.push_back(receive);
        }

        fn calls(&self) -> Vec<String> {
            self.state.lock().unwrap().calls.clone()
        }

        fn result(&self, operation: &'static str) -> c_int {
            if self.state.lock().unwrap().failure == Some(operation) {
                -1
            } else {
                0
            }
        }
    }

    impl LimeApi for MockApi {
        fn library_name(&self) -> &str {
            "mock"
        }

        fn open(&self, _identifier: Option<&CStr>) -> (c_int, *mut c_void) {
            self.state.lock().unwrap().calls.push("open".to_owned());
            (
                self.result("open"),
                if self.result("open") == 0 {
                    std::ptr::dangling_mut::<c_void>()
                } else {
                    std::ptr::null_mut()
                },
            )
        }

        fn close(&self, _device: NonNull<c_void>) -> c_int {
            self.state.lock().unwrap().calls.push("close".to_owned());
            self.result("close")
        }

        fn init(&self, _device: NonNull<c_void>) -> c_int {
            self.state.lock().unwrap().calls.push("init".to_owned());
            self.result("init")
        }

        fn get_num_channels(&self, _device: NonNull<c_void>) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("get_num_channels".to_owned());
            if self.result("get_num_channels") == 0 {
                2
            } else {
                -1
            }
        }

        fn get_frequency_range(&self, _device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
            *range = LmsRange {
                min: 100_000.0,
                max: 3_800_000_000.0,
                step: 1.0,
            };
            self.state
                .lock()
                .unwrap()
                .calls
                .push("get_frequency_range".to_owned());
            self.result("get_frequency_range")
        }

        fn get_sample_rate_range(&self, _device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
            *range = LmsRange {
                min: 100_000.0,
                max: 61_440_000.0,
                step: 1.0,
            };
            self.state
                .lock()
                .unwrap()
                .calls
                .push("get_sample_rate_range".to_owned());
            self.result("get_sample_rate_range")
        }

        fn get_lpf_range(&self, _device: NonNull<c_void>, range: &mut LmsRange) -> c_int {
            *range = LmsRange {
                min: 1_400_000.0,
                max: 130_000_000.0,
                step: 1.0,
            };
            self.state
                .lock()
                .unwrap()
                .calls
                .push("get_lpf_range".to_owned());
            self.result("get_lpf_range")
        }

        fn enable_channel(&self, _device: NonNull<c_void>, channel: usize, enabled: bool) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("enable:{channel}:{enabled}"));
            self.result(if enabled {
                "enable_channel"
            } else {
                "disable_channel"
            })
        }

        fn set_sample_rate(
            &self,
            _device: NonNull<c_void>,
            rate_hz: f64,
            oversample: usize,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.requested_sample_rate = rate_hz;
            state
                .calls
                .push(format!("sample_rate:{rate_hz:.0}:{oversample}"));
            drop(state);
            self.result("set_sample_rate")
        }

        fn get_sample_rate(
            &self,
            _device: NonNull<c_void>,
            _channel: usize,
            host_hz: &mut f64,
            rf_hz: &mut f64,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            *host_hz = state.requested_sample_rate;
            *rf_hz = state.requested_sample_rate * 2.0;
            state.calls.push("get_sample_rate".to_owned());
            drop(state);
            self.result("get_sample_rate")
        }

        fn set_frequency(
            &self,
            _device: NonNull<c_void>,
            channel: usize,
            frequency_hz: f64,
        ) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("frequency:{channel}:{frequency_hz:.0}"));
            self.result("set_frequency")
        }

        fn set_lpf(&self, _device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.requested_bandwidth = bandwidth_hz;
            state
                .calls
                .push(format!("bandwidth:{channel}:{bandwidth_hz:.0}"));
            drop(state);
            self.result("set_lpf")
        }

        fn get_lpf(
            &self,
            _device: NonNull<c_void>,
            _channel: usize,
            bandwidth_hz: &mut f64,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            *bandwidth_hz = state.requested_bandwidth;
            state.calls.push("get_lpf".to_owned());
            drop(state);
            self.result("get_lpf")
        }

        fn set_gain(&self, _device: NonNull<c_void>, channel: usize, gain_db: u32) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("gain:{channel}:{gain_db}"));
            self.result("set_gain")
        }

        fn calibrate(&self, _device: NonNull<c_void>, channel: usize, bandwidth_hz: f64) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("calibrate:{channel}:{bandwidth_hz:.0}"));
            self.result("calibrate")
        }

        fn setup_stream(&self, _device: NonNull<c_void>, stream: &mut LmsStream) -> c_int {
            assert_eq!(stream.handle, 0);
            assert!(!stream.is_tx);
            assert!(stream.channel <= 1);
            assert_eq!(stream.fifo_size, 1_048_576);
            assert_eq!(stream.throughput_vs_latency, 1.0);
            assert_eq!(stream.data_fmt, LMS_FMT_F32);
            assert_eq!(stream.link_fmt, LMS_LINK_FMT_DEFAULT);
            self.state
                .lock()
                .unwrap()
                .calls
                .push("setup_stream".to_owned());
            if self.result("setup_stream") == 0 {
                stream.handle = 0x1234;
                0
            } else {
                -1
            }
        }

        fn destroy_stream(&self, _device: NonNull<c_void>, stream: &mut LmsStream) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("destroy_stream".to_owned());
            let result = self.result("destroy_stream");
            if result == 0 {
                stream.handle = 0;
            }
            result
        }

        fn start_stream(&self, _stream: &mut LmsStream) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("start_stream".to_owned());
            self.result("start_stream")
        }

        fn stop_stream(&self, _stream: &mut LmsStream) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("stop_stream".to_owned());
            self.result("stop_stream")
        }

        fn recv_stream(
            &self,
            _stream: &mut LmsStream,
            samples: &mut [f32],
            _sample_count: usize,
            metadata: &mut LmsStreamMetadata,
            _timeout_ms: u32,
        ) -> c_int {
            assert!(!metadata.wait_for_timestamp);
            assert!(!metadata.flush_partial_packet);
            let mut state = self.state.lock().unwrap();
            state.calls.push("recv_stream".to_owned());
            let receive = state.receives.pop_front().unwrap();
            metadata.timestamp = receive.timestamp;
            state.pending_status = Some((receive.overrun, receive.dropped_packets));
            if receive.native_result >= 0 {
                samples[..receive.iq.len()].copy_from_slice(&receive.iq);
            }
            receive.native_result
        }

        fn get_stream_status(
            &self,
            _stream: &mut LmsStream,
            status: &mut LmsStreamStatus,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.calls.push("get_stream_status".to_owned());
            if let Some((overrun, dropped_packets)) = state.pending_status.take() {
                status.active = true;
                status.overrun = overrun;
                status.dropped_packets = dropped_packets;
            }
            drop(state);
            self.result("get_stream_status")
        }

        fn error_string(&self) -> String {
            "mock LimeSuite failure".to_owned()
        }
    }

    fn config() -> SdrConfig {
        SdrConfig {
            center_frequency_hz: 2_426_000_000,
            sample_rate_hz: 4_000_000,
            bandwidth_hz: 2_000_000,
            gain_db: 30.4,
            channel: 0,
        }
    }

    #[test]
    fn lifecycle_converts_f32_and_reports_timestamp() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: 2,
            timestamp: 5_000,
            iq: vec![0.25, -0.5, 1.0, -1.0],
            overrun: 0,
            dropped_packets: 0,
        });
        let mut driver =
            LimeDriver::open(api.clone(), Some("serial=mock"), LimeSdrOptions::default()).unwrap();
        assert_eq!(driver.capabilities().receive_channels, 2);
        driver.configure(&config()).unwrap();
        assert_eq!(
            driver.applied_config(),
            Some(AppliedLimeSdrConfig {
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
            })
        );
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 4];
        let (count, metadata) = driver.read(&mut output, Duration::from_millis(20)).unwrap();
        assert_eq!(count, 2);
        assert_eq!(metadata.first_sample_index, 5_000);
        assert_eq!(output[0], Complex32::new(0.25, -0.5));
        assert_eq!(output[1], Complex32::new(1.0, -1.0));
        driver.stop().unwrap();
        drop(driver);

        let calls = api.calls();
        assert_eq!(
            calls,
            [
                "open",
                "init",
                "get_num_channels",
                "get_frequency_range",
                "get_sample_rate_range",
                "get_lpf_range",
                "enable:0:true",
                "sample_rate:4000000:0",
                "get_sample_rate",
                "frequency:0:2426000000",
                "bandwidth:0:2000000",
                "get_lpf",
                "gain:0:30",
                "calibrate:0:2500000",
                "setup_stream",
                "start_stream",
                "recv_stream",
                "get_stream_status",
                "stop_stream",
                "destroy_stream",
                "enable:0:false",
                "close",
            ]
        );
    }

    #[test]
    fn timestamp_gap_and_native_status_report_overrun() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: 2,
            timestamp: 100,
            iq: vec![0.0; 4],
            overrun: 0,
            dropped_packets: 0,
        });
        api.push_rx(MockRx {
            native_result: 1,
            timestamp: 110,
            iq: vec![0.0; 2],
            overrun: 1,
            dropped_packets: 2,
        });
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
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
            native_result: 2,
            timestamp: 100,
            iq: vec![0.0; 4],
            overrun: 0,
            dropped_packets: 0,
        });
        api.push_rx(MockRx {
            native_result: 1,
            timestamp: 90,
            iq: vec![0.0; 2],
            overrun: 0,
            dropped_packets: 0,
        });
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
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
    fn timeout_is_empty_and_does_not_consume_status_counters() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: 0,
            timestamp: 0,
            iq: Vec::new(),
            overrun: 1,
            dropped_packets: 1,
        });
        let mut driver = LimeDriver::open(api.clone(), None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        let (count, _) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(count, 0);
        assert_eq!(
            api.calls()
                .iter()
                .filter(|call| call.as_str() == "get_stream_status")
                .count(),
            0
        );
    }

    #[test]
    fn rejects_non_finite_native_samples() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: 1,
            timestamp: 0,
            iq: vec![f32::NAN, 0.0],
            overrun: 0,
            dropped_packets: 0,
        });
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert!(
            driver
                .read(&mut output, Duration::from_millis(1))
                .unwrap_err()
                .to_string()
                .contains("non-finite")
        );
    }

    #[test]
    fn native_receive_error_preserves_operation_and_message() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: -1,
            timestamp: 0,
            iq: Vec::new(),
            overrun: 0,
            dropped_packets: 0,
        });
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        let error = driver
            .read(&mut output, Duration::from_millis(1))
            .unwrap_err()
            .to_string();
        assert!(error.contains("recv_stream"));
        assert!(error.contains("mock LimeSuite failure"));
    }

    #[test]
    fn rejects_timestamp_overflow() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            native_result: 1,
            timestamp: u64::MAX,
            iq: vec![0.0; 2],
            overrun: 0,
            dropped_packets: 0,
        });
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert!(
            driver
                .read(&mut output, Duration::from_millis(1))
                .unwrap_err()
                .to_string()
                .contains("timestamp overflow")
        );
    }

    #[test]
    fn initialization_failure_closes_device() {
        let api = Arc::new(MockApi::with_failure("init"));
        let error = match LimeDriver::open(api.clone(), None, LimeSdrOptions::default()) {
            Ok(_) => panic!("initialization failure must reject the device"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("init"));
        assert_eq!(api.calls(), ["open", "init", "close"]);
    }

    #[test]
    fn configuration_failure_disables_channel() {
        let api = Arc::new(MockApi::with_failure("set_frequency"));
        let mut driver = LimeDriver::open(api.clone(), None, LimeSdrOptions::default()).unwrap();
        assert!(driver.configure(&config()).is_err());
        assert!(api.calls().ends_with(&[
            "get_sample_rate".to_owned(),
            "frequency:0:2426000000".to_owned(),
            "enable:0:false".to_owned(),
        ]));
    }

    #[test]
    fn rejects_invalid_lifecycle_and_configuration() {
        let api = Arc::new(MockApi::default());
        let mut driver = LimeDriver::open(api, None, LimeSdrOptions::default()).unwrap();
        assert!(driver.start().is_err());
        let mut invalid = config();
        invalid.gain_db = -1.0;
        assert!(driver.configure(&invalid).is_err());
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        assert!(driver.configure(&config()).is_err());
        driver.stop().unwrap();
        driver.stop().unwrap();
    }

    #[test]
    fn reconfigure_releases_previous_stream_and_channel() {
        let api = Arc::new(MockApi::default());
        let mut driver = LimeDriver::open(api.clone(), None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        let mut second = config();
        second.channel = 1;
        driver.configure(&second).unwrap();
        let calls = api.calls();
        let reconfigure = [
            "destroy_stream",
            "enable:0:false",
            "enable:1:true",
            "sample_rate:4000000:0",
        ];
        assert!(
            calls
                .windows(reconfigure.len())
                .any(|window| window == reconfigure)
        );
    }

    #[test]
    fn validates_options_and_abi_layouts() {
        assert!(
            LimeSdrOptions {
                throughput_vs_latency: 1.1,
                ..LimeSdrOptions::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            LimeSdrOptions {
                oversample: 3,
                ..LimeSdrOptions::default()
            }
            .validate()
            .is_err()
        );
        if usize::BITS == 64 {
            assert_eq!(std::mem::size_of::<LmsStreamMetadata>(), 16);
            assert_eq!(std::mem::align_of::<LmsStreamMetadata>(), 8);
            assert_eq!(std::mem::size_of::<LmsStream>(), 32);
            assert_eq!(std::mem::align_of::<LmsStream>(), 8);
            assert_eq!(std::mem::size_of::<LmsStreamStatus>(), 48);
            assert_eq!(std::mem::align_of::<LmsStreamStatus>(), 8);
        }
    }

    #[test]
    fn drop_stops_destroys_disables_and_closes() {
        let api = Arc::new(MockApi::default());
        let mut driver = LimeDriver::open(api.clone(), None, LimeSdrOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        drop(driver);
        let calls = api.calls();
        assert_eq!(
            &calls[calls.len() - 4..],
            ["stop_stream", "destroy_stream", "enable:0:false", "close"]
        );
    }
}
