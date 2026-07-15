use crate::complex::Complex32;
use crate::native::DynamicLibrary;
use crate::sdr::{IqSource, ReadMetadata, SdrCapabilities, SdrConfig, SdrKind};
use crate::{Error, Result};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr::NonNull;
use std::sync::Arc;
use std::time::Duration;

const BACKEND: &str = "XTRX";
const XTRX_TUNE_RX_FDD: c_int = 0;
const XTRX_RX_LNA_GAIN: c_int = 0;
const XTRX_RX_AUTO: c_int = 7;
const XTRX_CH_A: c_int = 1;
const XTRX_CH_B: c_int = 2;
const XTRX_CH_AB: c_int = XTRX_CH_A | XTRX_CH_B;
const XTRX_RX: c_int = 1;
const XTRX_WF_16: c_int = 3;
const XTRX_IQ_INT16: c_int = 2;
const XTRX_RSP_SWAP_AB: u32 = 8;
const XTRX_RSP_SISO_MODE: u32 = 32;
const RCVEX_DONT_INSER_ZEROS: u32 = 4;
const RCVEX_DROP_OLD_ON_OVERFLOW: u32 = 8;
const RCVEX_TIMOUT: u32 = 32;
const RCVEX_EVENT_OVERFLOW: u32 = 1;
const RCVEX_EVENT_FILLED_ZERO: u32 = 2;
const Q11_SCALE: f32 = 1.0 / 2048.0;
const MIN_BANDWIDTH_HZ: u32 = 1_000_000;
const MAX_BANDWIDTH_HZ: u32 = 60_000_000;
const MIN_GAIN_DB: f32 = 0.0;
const MAX_GAIN_DB: f32 = 30.0;

#[derive(Clone, Copy, Debug)]
pub struct XtrxOptions {
    pub rx_stream_start_samples: u64,
    pub packet_size: u32,
}

impl Default for XtrxOptions {
    fn default() -> Self {
        Self {
            rx_stream_start_samples: 32_768,
            packet_size: 0,
        }
    }
}

impl XtrxOptions {
    fn validate(self) -> Result<()> {
        if self.packet_size > 32_767 {
            return Err(Error::InvalidConfiguration(
                "XTRX packet_size must be zero (automatic) or no greater than 32767".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppliedXtrxConfig {
    pub sample_rate_hz: u32,
    pub bandwidth_hz: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct XtrxGtimeData {
    sec: u32,
    nsec: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct XtrxRunStreamParams {
    wire_format: c_int,
    host_format: c_int,
    channels: c_int,
    packet_size: u32,
    flags: u32,
    scale: f32,
    reserved: [u32; 6],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct XtrxRunParams {
    direction: c_int,
    flags: c_uint,
    tx: XtrxRunStreamParams,
    rx: XtrxRunStreamParams,
    rx_stream_start: u64,
    tx_repeat_buffer: *mut c_void,
    gtime: XtrxGtimeData,
    reserved: [u32; 8],
}

impl Default for XtrxRunParams {
    fn default() -> Self {
        Self {
            direction: 0,
            flags: 0,
            tx: XtrxRunStreamParams::default(),
            rx: XtrxRunStreamParams::default(),
            rx_stream_start: 0,
            tx_repeat_buffer: std::ptr::null_mut(),
            gtime: XtrxGtimeData::default(),
            reserved: [0; 8],
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct XtrxRecvInfo {
    samples: c_uint,
    buffer_count: c_uint,
    buffers: *const *mut c_void,
    flags: c_uint,
    timeout: c_uint,
    out_samples: c_uint,
    out_events: c_uint,
    out_first_sample: u64,
    out_overrun_at: u64,
    out_resumed_at: u64,
}

impl Default for XtrxRecvInfo {
    fn default() -> Self {
        Self {
            samples: 0,
            buffer_count: 0,
            buffers: std::ptr::null(),
            flags: 0,
            timeout: 0,
            out_samples: 0,
            out_events: 0,
            out_first_sample: 0,
            out_overrun_at: 0,
            out_resumed_at: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct XtrxRecvOutput {
    samples: u32,
    events: u32,
    first_sample: u64,
    overrun_at: u64,
    resumed_at: u64,
}

trait XtrxApi {
    fn library_name(&self) -> &str;
    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void);
    fn close(&self, device: NonNull<c_void>);
    fn set_sample_rate(
        &self,
        device: NonNull<c_void>,
        requested_hz: f64,
        actual_cgen_hz: &mut f64,
        actual_rx_hz: &mut f64,
        actual_tx_hz: &mut f64,
    ) -> c_int;
    fn tune_rx(&self, device: NonNull<c_void>, frequency_hz: f64, actual_hz: &mut f64) -> c_int;
    fn tune_rx_bandwidth(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        bandwidth_hz: f64,
        actual_hz: &mut f64,
    ) -> c_int;
    fn set_lna_gain(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        gain_db: f64,
        actual_db: &mut f64,
    ) -> c_int;
    fn set_rx_antenna(&self, device: NonNull<c_void>, channel: c_int) -> c_int;
    fn init_run_params(&self, params: &mut XtrxRunParams);
    fn run(&self, device: NonNull<c_void>, params: &XtrxRunParams) -> c_int;
    fn recv(
        &self,
        device: NonNull<c_void>,
        samples: &mut [i16],
        sample_count: u32,
        flags: u32,
        timeout_ms: u32,
    ) -> (c_int, XtrxRecvOutput);
    fn stop_rx(&self, device: NonNull<c_void>) -> c_int;
    fn error_string(&self, code: c_int) -> String;
}

type OpenFn = unsafe extern "C" fn(*const c_char, c_uint, *mut *mut c_void) -> c_int;
type CloseFn = unsafe extern "C" fn(*mut c_void);
type SetSampleRateFn =
    unsafe extern "C" fn(*mut c_void, f64, f64, f64, c_uint, *mut f64, *mut f64, *mut f64) -> c_int;
type TuneFn = unsafe extern "C" fn(*mut c_void, c_int, f64, *mut f64) -> c_int;
type TuneBandwidthFn = unsafe extern "C" fn(*mut c_void, c_int, f64, *mut f64) -> c_int;
type SetGainFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, f64, *mut f64) -> c_int;
type SetAntennaFn = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type RunParamsInitFn = unsafe extern "C" fn(*mut XtrxRunParams);
type RunFn = unsafe extern "C" fn(*mut c_void, *const XtrxRunParams) -> c_int;
type RecvFn = unsafe extern "C" fn(*mut c_void, *mut XtrxRecvInfo) -> c_int;
type StopFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;

struct DynamicXtrxApi {
    _library: DynamicLibrary,
    library_name: String,
    open: OpenFn,
    close: CloseFn,
    set_sample_rate: SetSampleRateFn,
    tune: TuneFn,
    tune_rx_bandwidth: TuneBandwidthFn,
    set_gain: SetGainFn,
    set_antenna: SetAntennaFn,
    run_params_init: RunParamsInitFn,
    run: RunFn,
    recv: RecvFn,
    stop: StopFn,
}

impl DynamicXtrxApi {
    fn load() -> Result<Self> {
        let names = match std::env::var_os("BLUEOXIDE_XTRX_LIBRARY") {
            Some(override_name) if override_name.is_empty() => {
                return Err(Error::InvalidConfiguration(
                    "BLUEOXIDE_XTRX_LIBRARY must not be empty".to_owned(),
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

        // SAFETY: Signatures and structures are transcribed from xtrx_api.h at
        // the pinned libxtrx revision documented in Verification.md.
        unsafe {
            Ok(Self {
                open: library.symbol("xtrx_open")?,
                close: library.symbol("xtrx_close")?,
                set_sample_rate: library.symbol("xtrx_set_samplerate")?,
                tune: library.symbol("xtrx_tune")?,
                tune_rx_bandwidth: library.symbol("xtrx_tune_rx_bandwidth")?,
                set_gain: library.symbol("xtrx_set_gain")?,
                set_antenna: library.symbol("xtrx_set_antenna_ex")?,
                run_params_init: library.symbol("xtrx_run_params_init")?,
                run: library.symbol("xtrx_run_ex")?,
                recv: library.symbol("xtrx_recv_sync_ex")?,
                stop: library.symbol("xtrx_stop")?,
                _library: library,
                library_name,
            })
        }
    }
}

impl XtrxApi for DynamicXtrxApi {
    fn library_name(&self) -> &str {
        &self.library_name
    }

    fn open(&self, identifier: Option<&CStr>) -> (c_int, *mut c_void) {
        let mut device = std::ptr::null_mut();
        // SAFETY: The function pointer has the reviewed ABI, device is
        // writable, and identifier is NULL or a NUL-terminated path.
        let status = unsafe {
            (self.open)(
                identifier.map_or(std::ptr::null(), CStr::as_ptr),
                0,
                &mut device,
            )
        };
        (status, device)
    }

    fn close(&self, device: NonNull<c_void>) {
        // SAFETY: device was returned by xtrx_open and remains owned here.
        unsafe { (self.close)(device.as_ptr()) };
    }

    fn set_sample_rate(
        &self,
        device: NonNull<c_void>,
        requested_hz: f64,
        actual_cgen_hz: &mut f64,
        actual_rx_hz: &mut f64,
        actual_tx_hz: &mut f64,
    ) -> c_int {
        // SAFETY: device is open and all output pointers are writable.
        unsafe {
            (self.set_sample_rate)(
                device.as_ptr(),
                0.0,
                requested_hz,
                0.0,
                0,
                actual_cgen_hz,
                actual_rx_hz,
                actual_tx_hz,
            )
        }
    }

    fn tune_rx(&self, device: NonNull<c_void>, frequency_hz: f64, actual_hz: &mut f64) -> c_int {
        // SAFETY: device is configured and frequency/output are valid.
        unsafe { (self.tune)(device.as_ptr(), XTRX_TUNE_RX_FDD, frequency_hz, actual_hz) }
    }

    fn tune_rx_bandwidth(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        bandwidth_hz: f64,
        actual_hz: &mut f64,
    ) -> c_int {
        // SAFETY: device and channel are valid and output is writable.
        unsafe { (self.tune_rx_bandwidth)(device.as_ptr(), channel, bandwidth_hz, actual_hz) }
    }

    fn set_lna_gain(
        &self,
        device: NonNull<c_void>,
        channel: c_int,
        gain_db: f64,
        actual_db: &mut f64,
    ) -> c_int {
        // SAFETY: device/channel are valid and output is writable.
        unsafe {
            (self.set_gain)(
                device.as_ptr(),
                channel,
                XTRX_RX_LNA_GAIN,
                gain_db,
                actual_db,
            )
        }
    }

    fn set_rx_antenna(&self, device: NonNull<c_void>, channel: c_int) -> c_int {
        // SAFETY: device/channel are valid and XTRX_RX_AUTO is an ABI constant.
        unsafe { (self.set_antenna)(device.as_ptr(), channel, XTRX_RX_AUTO) }
    }

    fn init_run_params(&self, params: &mut XtrxRunParams) {
        // SAFETY: params has the reviewed ABI and is writable.
        unsafe { (self.run_params_init)(params) };
    }

    fn run(&self, device: NonNull<c_void>, params: &XtrxRunParams) -> c_int {
        // SAFETY: device is configured and params remains valid for the call.
        unsafe { (self.run)(device.as_ptr(), params) }
    }

    fn recv(
        &self,
        device: NonNull<c_void>,
        samples: &mut [i16],
        sample_count: u32,
        flags: u32,
        timeout_ms: u32,
    ) -> (c_int, XtrxRecvOutput) {
        let mut buffer = samples.as_mut_ptr().cast::<c_void>();
        let mut info = XtrxRecvInfo {
            samples: sample_count,
            buffer_count: 1,
            buffers: &mut buffer,
            flags,
            timeout: timeout_ms,
            ..XtrxRecvInfo::default()
        };
        // SAFETY: info points at one writable interleaved I/Q buffer with two
        // i16 scalars per requested sample.
        let status = unsafe { (self.recv)(device.as_ptr(), &mut info) };
        (
            status,
            XtrxRecvOutput {
                samples: info.out_samples,
                events: info.out_events,
                first_sample: info.out_first_sample,
                overrun_at: info.out_overrun_at,
                resumed_at: info.out_resumed_at,
            },
        )
    }

    fn stop_rx(&self, device: NonNull<c_void>) -> c_int {
        // SAFETY: device is open and XTRX_RX selects receive only.
        unsafe { (self.stop)(device.as_ptr(), XTRX_RX) }
    }

    fn error_string(&self, code: c_int) -> String {
        errno_message(code)
    }
}

#[cfg(windows)]
fn default_library_names() -> &'static [&'static str] {
    &["xtrx.dll", "libxtrx.dll"]
}

#[cfg(target_os = "macos")]
fn default_library_names() -> &'static [&'static str] {
    &["libxtrx.0.dylib", "libxtrx.dylib"]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_library_names() -> &'static [&'static str] {
    &["libxtrx.so.0", "libxtrx.so"]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverState {
    Open,
    Configured,
    Running,
}

struct XtrxDriver<A: XtrxApi> {
    api: Arc<A>,
    device: NonNull<c_void>,
    options: XtrxOptions,
    state: DriverState,
    configured_channel: u8,
    native_samples: Vec<i16>,
    expected_next_sample: Option<u64>,
    pending_dropped_samples: u64,
    pending_overrun: bool,
    applied: Option<AppliedXtrxConfig>,
}

impl<A: XtrxApi> XtrxDriver<A> {
    fn open(api: Arc<A>, identifier: Option<&str>, options: XtrxOptions) -> Result<Self> {
        options.validate()?;
        let identifier = identifier
            .map(|value| {
                CString::new(value).map_err(|_| {
                    Error::InvalidConfiguration(
                        "XTRX device identifier contains a NUL octet".to_owned(),
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
                message: "libxtrx returned success with a NULL device".to_owned(),
            });
        };
        Ok(Self {
            api,
            device,
            options,
            state: DriverState::Open,
            configured_channel: 0,
            native_samples: Vec::new(),
            expected_next_sample: None,
            pending_dropped_samples: 0,
            pending_overrun: false,
            applied: None,
        })
    }

    fn applied_config(&self) -> Option<AppliedXtrxConfig> {
        self.applied
    }

    fn configure(&mut self, config: &SdrConfig) -> Result<()> {
        if self.state == DriverState::Running {
            return Err(Error::InvalidState(
                "XTRX cannot be configured while streaming".to_owned(),
            ));
        }
        config.validate(xtrx_capabilities())?;
        validate_sample_rate(config.sample_rate_hz)?;
        if !(MIN_BANDWIDTH_HZ..=MAX_BANDWIDTH_HZ).contains(&config.bandwidth_hz) {
            return Err(Error::InvalidConfiguration(format!(
                "XTRX bandwidth {} Hz is outside supported range {}..={} Hz",
                config.bandwidth_hz, MIN_BANDWIDTH_HZ, MAX_BANDWIDTH_HZ
            )));
        }
        if !(MIN_GAIN_DB..=MAX_GAIN_DB).contains(&config.gain_db) {
            return Err(Error::InvalidConfiguration(format!(
                "XTRX LNA gain {} dB is outside supported range {}..={} dB",
                config.gain_db, MIN_GAIN_DB, MAX_GAIN_DB
            )));
        }

        self.applied = None;
        self.expected_next_sample = None;
        self.pending_dropped_samples = 0;
        self.pending_overrun = false;
        self.state = DriverState::Open;
        let channel = channel_mask(config.channel)?;

        let mut actual_cgen_hz = 0.0;
        let mut actual_rx_hz = 0.0;
        let mut actual_tx_hz = 0.0;
        check(
            self.api.as_ref(),
            "set_sample_rate",
            self.api.set_sample_rate(
                self.device,
                config.sample_rate_hz as f64,
                &mut actual_cgen_hz,
                &mut actual_rx_hz,
                &mut actual_tx_hz,
            ),
        )?;
        let actual_sample_rate = exact_u32_hz("XTRX applied sample rate", actual_rx_hz)?;
        if !actual_cgen_hz.is_finite() || actual_cgen_hz <= 0.0 {
            return Err(native_contract(
                "set_sample_rate",
                format!("libxtrx returned invalid CGEN rate {actual_cgen_hz}"),
            ));
        }

        let mut actual_frequency_hz = 0.0;
        check(
            self.api.as_ref(),
            "tune_rx",
            self.api.tune_rx(
                self.device,
                config.center_frequency_hz as f64,
                &mut actual_frequency_hz,
            ),
        )?;
        validate_frequency(actual_frequency_hz)?;

        let mut actual_bandwidth_hz = 0.0;
        check(
            self.api.as_ref(),
            "tune_rx_bandwidth",
            self.api.tune_rx_bandwidth(
                self.device,
                channel,
                config.bandwidth_hz as f64,
                &mut actual_bandwidth_hz,
            ),
        )?;
        let actual_bandwidth = exact_u32_hz("XTRX applied RX bandwidth", actual_bandwidth_hz)?;

        let mut actual_gain_db = 0.0;
        check(
            self.api.as_ref(),
            "set_lna_gain",
            self.api.set_lna_gain(
                self.device,
                channel,
                config.gain_db as f64,
                &mut actual_gain_db,
            ),
        )?;
        if !actual_gain_db.is_finite() {
            return Err(native_contract(
                "set_lna_gain",
                format!("libxtrx returned non-finite applied gain {actual_gain_db}"),
            ));
        }
        check(
            self.api.as_ref(),
            "set_rx_antenna",
            self.api.set_rx_antenna(self.device, channel),
        )?;

        self.configured_channel = config.channel;
        self.applied = Some(AppliedXtrxConfig {
            sample_rate_hz: actual_sample_rate,
            bandwidth_hz: actual_bandwidth,
        });
        self.state = DriverState::Configured;
        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        match self.state {
            DriverState::Open => Err(Error::InvalidState(
                "XTRX must be configured before start".to_owned(),
            )),
            DriverState::Configured => {
                let mut params = XtrxRunParams::default();
                self.api.init_run_params(&mut params);
                params.direction = XTRX_RX;
                params.flags = 0;
                params.rx.wire_format = XTRX_WF_16;
                params.rx.host_format = XTRX_IQ_INT16;
                params.rx.channels = XTRX_CH_AB;
                params.rx.packet_size = self.options.packet_size;
                params.rx.flags = XTRX_RSP_SISO_MODE
                    | if self.configured_channel == 1 {
                        XTRX_RSP_SWAP_AB
                    } else {
                        0
                    };
                params.rx_stream_start = self.options.rx_stream_start_samples;
                params.tx_repeat_buffer = std::ptr::null_mut();
                check(self.api.as_ref(), "run", self.api.run(self.device, &params))?;
                self.expected_next_sample = None;
                self.pending_dropped_samples = 0;
                self.pending_overrun = false;
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
                "XTRX read requires a running stream".to_owned(),
            ));
        }
        if output.is_empty() {
            return Ok((0, ReadMetadata::default()));
        }
        let sample_count = u32::try_from(output.len()).map_err(|_| {
            Error::InvalidConfiguration("XTRX read buffer exceeds u32 samples".to_owned())
        })?;
        let scalar_count = output.len().checked_mul(2).ok_or_else(|| {
            Error::InvalidConfiguration("XTRX native buffer size overflow".to_owned())
        })?;
        self.native_samples.resize(scalar_count, 0);
        let timeout_ms = duration_to_timeout_ms(timeout)?;
        let receive_flags = RCVEX_DONT_INSER_ZEROS | RCVEX_DROP_OLD_ON_OVERFLOW | RCVEX_TIMOUT;
        let (status, native) = self.api.recv(
            self.device,
            &mut self.native_samples,
            sample_count,
            receive_flags,
            timeout_ms,
        );
        if is_timeout_code(status) {
            return Ok((0, ReadMetadata::default()));
        }
        check(self.api.as_ref(), "recv", status)?;

        let count = native.samples as usize;
        if count > output.len() {
            return Err(native_contract(
                "recv",
                format!(
                    "libxtrx reported {count} samples for a {}-sample buffer",
                    output.len()
                ),
            ));
        }
        let native_overflow_gap =
            if native.events & RCVEX_EVENT_OVERFLOW != 0 {
                native.resumed_at.checked_sub(native.overrun_at).ok_or_else(|| {
                native_contract(
                    "recv",
                    format!(
                        "libxtrx overflow resume timestamp {} precedes overrun timestamp {}",
                        native.resumed_at, native.overrun_at
                    ),
                )
            })?
            } else {
                0
            };
        let known_native_event =
            native.events & (RCVEX_EVENT_OVERFLOW | RCVEX_EVENT_FILLED_ZERO) != 0;
        let unknown_native_event =
            native.events & !(RCVEX_EVENT_OVERFLOW | RCVEX_EVENT_FILLED_ZERO) != 0;
        if count == 0 {
            self.pending_dropped_samples = self.pending_dropped_samples.max(native_overflow_gap);
            self.pending_overrun |= known_native_event || unknown_native_event;
            return Ok((0, ReadMetadata::default()));
        }

        for (destination, iq) in output[..count]
            .iter_mut()
            .zip(self.native_samples[..count * 2].chunks_exact(2))
        {
            *destination = Complex32::new(iq[0] as f32 * Q11_SCALE, iq[1] as f32 * Q11_SCALE);
        }

        let first_sample_index = native.first_sample;
        let timestamp_gap = self
            .expected_next_sample
            .map(|expected| first_sample_index.saturating_sub(expected))
            .unwrap_or(0);
        let dropped_samples_before = timestamp_gap
            .max(native_overflow_gap)
            .max(self.pending_dropped_samples);
        let overrun = self.pending_overrun
            || known_native_event
            || unknown_native_event
            || self
                .expected_next_sample
                .is_some_and(|expected| expected != first_sample_index);
        let next_sample = first_sample_index
            .checked_add(count as u64)
            .ok_or_else(|| {
                native_contract("recv", "libxtrx sample timestamp overflow".to_owned())
            })?;
        self.expected_next_sample = Some(next_sample);
        self.pending_dropped_samples = 0;
        self.pending_overrun = false;

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
            check(self.api.as_ref(), "stop", self.api.stop_rx(self.device))?;
            self.state = DriverState::Configured;
        }
        Ok(())
    }
}

impl<A: XtrxApi> Drop for XtrxDriver<A> {
    fn drop(&mut self) {
        if self.state == DriverState::Running {
            let _ = self.api.stop_rx(self.device);
        }
        self.api.close(self.device);
    }
}

pub struct XtrxSource {
    driver: XtrxDriver<DynamicXtrxApi>,
}

impl XtrxSource {
    pub fn open(identifier: Option<&str>, options: XtrxOptions) -> Result<Self> {
        let api = Arc::new(DynamicXtrxApi::load()?);
        Ok(Self {
            driver: XtrxDriver::open(api, identifier, options)?,
        })
    }

    pub fn probe_library() -> Result<String> {
        let api = DynamicXtrxApi::load()?;
        Ok(api.library_name().to_owned())
    }

    pub fn applied_config(&self) -> Option<AppliedXtrxConfig> {
        self.driver.applied_config()
    }
}

impl IqSource for XtrxSource {
    fn kind(&self) -> SdrKind {
        SdrKind::Xtrx
    }

    fn capabilities(&self) -> SdrCapabilities {
        xtrx_capabilities()
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

fn xtrx_capabilities() -> SdrCapabilities {
    SdrCapabilities {
        minimum_frequency_hz: 30_000_000,
        maximum_frequency_hz: 3_800_000_000,
        maximum_sample_rate_hz: 80_000_000,
        receive_channels: 2,
    }
}

fn validate_sample_rate(sample_rate_hz: u32) -> Result<()> {
    if sample_rate_hz <= 56_250_000 || sample_rate_hz >= 61_437_500 {
        Ok(())
    } else {
        Err(Error::InvalidConfiguration(format!(
            "XTRX sample rate {sample_rate_hz} Hz falls in unsupported range 56250001..=61437499 Hz"
        )))
    }
}

fn channel_mask(channel: u8) -> Result<c_int> {
    match channel {
        0 => Ok(XTRX_CH_A),
        1 => Ok(XTRX_CH_B),
        _ => Err(Error::InvalidConfiguration(format!(
            "XTRX receive channel {channel} is unavailable; expected 0 or 1"
        ))),
    }
}

fn native_error(api: &impl XtrxApi, operation: &'static str, code: c_int) -> Error {
    Error::NativeCall {
        backend: BACKEND,
        operation,
        code,
        message: api.error_string(code),
    }
}

fn native_contract(operation: &'static str, message: String) -> Error {
    Error::NativeCall {
        backend: BACKEND,
        operation,
        code: 0,
        message,
    }
}

fn check(api: &impl XtrxApi, operation: &'static str, status: c_int) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(native_error(api, operation, status))
    }
}

fn errno_message(code: c_int) -> String {
    let errno = if code < 0 {
        code.checked_neg().unwrap_or(c_int::MAX)
    } else {
        code
    };
    let description = std::io::Error::from_raw_os_error(errno).to_string();
    format!("errno {errno}: {description}")
}

fn is_timeout_code(code: c_int) -> bool {
    // libxtrx returns negative errno values. These cover Linux, BSD/macOS,
    // Microsoft CRT, and Winsock timeout values without a libc dependency.
    matches!(code, -110 | -60 | -138 | -10060)
}

fn exact_u32_hz(name: &str, value: f64) -> Result<u32> {
    if !value.is_finite() || value <= 0.0 || value > u32::MAX as f64 {
        return Err(native_contract(
            "read_applied_value",
            format!("{name} is invalid: {value}"),
        ));
    }
    let rounded = value.round();
    if (value - rounded).abs() > 1.0e-6 {
        return Err(native_contract(
            "read_applied_value",
            format!("{name} is not an integer number of hertz: {value}"),
        ));
    }
    Ok(rounded as u32)
}

fn validate_frequency(value: f64) -> Result<()> {
    let capabilities = xtrx_capabilities();
    if !value.is_finite()
        || value < capabilities.minimum_frequency_hz as f64
        || value > capabilities.maximum_frequency_hz as f64
    {
        return Err(native_contract(
            "read_applied_value",
            format!("XTRX applied RX frequency is invalid: {value}"),
        ));
    }
    Ok(())
}

fn duration_to_timeout_ms(timeout: Duration) -> Result<u32> {
    let millis = timeout.as_millis();
    if millis == 0 {
        return Ok(1);
    }
    u32::try_from(millis).map_err(|_| {
        Error::InvalidConfiguration("XTRX timeout exceeds u32 milliseconds".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct MockRx {
        status: c_int,
        iq: Vec<i16>,
        events: u32,
        first_sample: u64,
        overrun_at: u64,
        resumed_at: u64,
    }

    #[derive(Default)]
    struct MockRunParams {
        direction: c_int,
        rx_wire_format: c_int,
        rx_host_format: c_int,
        rx_channels: c_int,
        rx_flags: u32,
        rx_stream_start: u64,
    }

    #[derive(Default)]
    struct MockState {
        calls: Vec<String>,
        failure: Option<&'static str>,
        receives: VecDeque<MockRx>,
        requested_sample_rate: f64,
        last_run: Option<MockRunParams>,
        last_receive_flags: u32,
        last_timeout_ms: u32,
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

        fn result(&self, operation: &'static str) -> c_int {
            if self.state.lock().unwrap().failure == Some(operation) {
                -5
            } else {
                0
            }
        }

        fn push_rx(&self, receive: MockRx) {
            self.state.lock().unwrap().receives.push_back(receive);
        }

        fn calls(&self) -> Vec<String> {
            self.state.lock().unwrap().calls.clone()
        }

        fn run_params(&self) -> MockRunParams {
            self.state.lock().unwrap().last_run.take().unwrap()
        }
    }

    impl XtrxApi for MockApi {
        fn library_name(&self) -> &str {
            "mock"
        }

        fn open(&self, _identifier: Option<&CStr>) -> (c_int, *mut c_void) {
            self.state.lock().unwrap().calls.push("open".to_owned());
            let status = self.result("open");
            (
                status,
                if status == 0 {
                    std::ptr::dangling_mut::<c_void>()
                } else {
                    std::ptr::null_mut()
                },
            )
        }

        fn close(&self, _device: NonNull<c_void>) {
            self.state.lock().unwrap().calls.push("close".to_owned());
        }

        fn set_sample_rate(
            &self,
            _device: NonNull<c_void>,
            requested_hz: f64,
            actual_cgen_hz: &mut f64,
            actual_rx_hz: &mut f64,
            actual_tx_hz: &mut f64,
        ) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.requested_sample_rate = requested_hz;
            state.calls.push(format!("sample_rate:{requested_hz:.0}"));
            *actual_cgen_hz = requested_hz * 4.0;
            *actual_rx_hz = requested_hz;
            *actual_tx_hz = 0.0;
            drop(state);
            self.result("set_sample_rate")
        }

        fn tune_rx(
            &self,
            _device: NonNull<c_void>,
            frequency_hz: f64,
            actual_hz: &mut f64,
        ) -> c_int {
            *actual_hz = frequency_hz;
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("frequency:{frequency_hz:.0}"));
            self.result("tune_rx")
        }

        fn tune_rx_bandwidth(
            &self,
            _device: NonNull<c_void>,
            channel: c_int,
            bandwidth_hz: f64,
            actual_hz: &mut f64,
        ) -> c_int {
            *actual_hz = bandwidth_hz;
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("bandwidth:{channel}:{bandwidth_hz:.0}"));
            self.result("tune_rx_bandwidth")
        }

        fn set_lna_gain(
            &self,
            _device: NonNull<c_void>,
            channel: c_int,
            gain_db: f64,
            actual_db: &mut f64,
        ) -> c_int {
            *actual_db = gain_db;
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("gain:{channel}:{gain_db:.1}"));
            self.result("set_lna_gain")
        }

        fn set_rx_antenna(&self, _device: NonNull<c_void>, channel: c_int) -> c_int {
            self.state
                .lock()
                .unwrap()
                .calls
                .push(format!("antenna:{channel}"));
            self.result("set_rx_antenna")
        }

        fn init_run_params(&self, params: &mut XtrxRunParams) {
            self.state
                .lock()
                .unwrap()
                .calls
                .push("run_params_init".to_owned());
            params.direction = 3;
            params.rx.wire_format = XTRX_WF_16;
            params.rx.host_format = 1;
            params.rx.channels = XTRX_CH_AB;
        }

        fn run(&self, _device: NonNull<c_void>, params: &XtrxRunParams) -> c_int {
            let mut state = self.state.lock().unwrap();
            state.calls.push("run".to_owned());
            state.last_run = Some(MockRunParams {
                direction: params.direction,
                rx_wire_format: params.rx.wire_format,
                rx_host_format: params.rx.host_format,
                rx_channels: params.rx.channels,
                rx_flags: params.rx.flags,
                rx_stream_start: params.rx_stream_start,
            });
            drop(state);
            self.result("run")
        }

        fn recv(
            &self,
            _device: NonNull<c_void>,
            samples: &mut [i16],
            sample_count: u32,
            flags: u32,
            timeout_ms: u32,
        ) -> (c_int, XtrxRecvOutput) {
            let mut state = self.state.lock().unwrap();
            state.calls.push("recv".to_owned());
            state.last_receive_flags = flags;
            state.last_timeout_ms = timeout_ms;
            let receive = state.receives.pop_front().unwrap();
            assert!(receive.iq.len() <= sample_count as usize * 2);
            if receive.status == 0 {
                samples[..receive.iq.len()].copy_from_slice(&receive.iq);
            }
            (
                receive.status,
                XtrxRecvOutput {
                    samples: (receive.iq.len() / 2) as u32,
                    events: receive.events,
                    first_sample: receive.first_sample,
                    overrun_at: receive.overrun_at,
                    resumed_at: receive.resumed_at,
                },
            )
        }

        fn stop_rx(&self, _device: NonNull<c_void>) -> c_int {
            self.state.lock().unwrap().calls.push("stop".to_owned());
            self.result("stop")
        }

        fn error_string(&self, code: c_int) -> String {
            format!("mock libxtrx error {code}")
        }
    }

    fn config() -> SdrConfig {
        SdrConfig {
            center_frequency_hz: 2_426_000_000,
            sample_rate_hz: 4_000_000,
            bandwidth_hz: 2_000_000,
            gain_db: 24.5,
            channel: 0,
        }
    }

    #[test]
    fn lifecycle_converts_q11_and_preserves_timestamp() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![2047, -2048, 1024, -1024],
            events: 0,
            first_sample: 40_000,
            overrun_at: 0,
            resumed_at: 0,
        });
        let mut driver =
            XtrxDriver::open(api.clone(), Some("/dev/xtrx0"), XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        assert_eq!(
            driver.applied_config(),
            Some(AppliedXtrxConfig {
                sample_rate_hz: 4_000_000,
                bandwidth_hz: 2_000_000,
            })
        );
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 4];
        let (count, metadata) = driver.read(&mut output, Duration::from_millis(50)).unwrap();
        assert_eq!(count, 2);
        assert_eq!(metadata.first_sample_index, 40_000);
        assert_eq!(output[0], Complex32::new(2047.0 / 2048.0, -1.0));
        assert_eq!(output[1], Complex32::new(0.5, -0.5));
        driver.stop().unwrap();
        drop(driver);

        assert_eq!(
            api.calls(),
            [
                "open",
                "sample_rate:4000000",
                "frequency:2426000000",
                "bandwidth:1:2000000",
                "gain:1:24.5",
                "antenna:1",
                "run_params_init",
                "run",
                "recv",
                "stop",
                "close",
            ]
        );
        let run = api.run_params();
        assert_eq!(run.direction, XTRX_RX);
        assert_eq!(run.rx_wire_format, XTRX_WF_16);
        assert_eq!(run.rx_host_format, XTRX_IQ_INT16);
        assert_eq!(run.rx_channels, XTRX_CH_AB);
        assert_eq!(run.rx_flags, XTRX_RSP_SISO_MODE);
        assert_eq!(run.rx_stream_start, 32_768);
    }

    #[test]
    fn channel_b_uses_swap_ab_siso_mode() {
        let api = Arc::new(MockApi::default());
        let mut driver = XtrxDriver::open(api.clone(), None, XtrxOptions::default()).unwrap();
        let mut channel_b = config();
        channel_b.channel = 1;
        driver.configure(&channel_b).unwrap();
        driver.start().unwrap();
        let run = api.run_params();
        assert_eq!(run.rx_flags, XTRX_RSP_SISO_MODE | XTRX_RSP_SWAP_AB);
        assert!(api.calls().contains(&"bandwidth:2:2000000".to_owned()));
    }

    #[test]
    fn timestamp_and_native_overflow_gaps_are_reported() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 4],
            events: 0,
            first_sample: 100,
            overrun_at: 0,
            resumed_at: 0,
        });
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: RCVEX_EVENT_OVERFLOW,
            first_sample: 120,
            overrun_at: 102,
            resumed_at: 120,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        driver.read(&mut output, Duration::from_millis(1)).unwrap();
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.dropped_samples_before, 18);
        assert!(metadata.overrun);
    }

    #[test]
    fn first_read_overflow_uses_native_gap() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: RCVEX_EVENT_OVERFLOW,
            first_sample: 500,
            overrun_at: 480,
            resumed_at: 500,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.dropped_samples_before, 20);
        assert!(metadata.overrun);
    }

    #[test]
    fn zero_sample_overflow_is_carried_to_next_read() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: Vec::new(),
            events: RCVEX_EVENT_OVERFLOW,
            first_sample: 0,
            overrun_at: 100,
            resumed_at: 120,
        });
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: 0,
            first_sample: 120,
            overrun_at: 0,
            resumed_at: 0,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert_eq!(
            driver
                .read(&mut output, Duration::from_millis(1))
                .unwrap()
                .0,
            0
        );
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.dropped_samples_before, 20);
        assert!(metadata.overrun);
    }

    #[test]
    fn backward_timestamp_does_not_wrap_drop_count() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 4],
            events: 0,
            first_sample: 100,
            overrun_at: 0,
            resumed_at: 0,
        });
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: 0,
            first_sample: 90,
            overrun_at: 0,
            resumed_at: 0,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 2];
        driver.read(&mut output, Duration::from_millis(1)).unwrap();
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert_eq!(metadata.dropped_samples_before, 0);
        assert!(metadata.overrun);
    }

    #[test]
    fn timeout_is_a_recoverable_empty_read() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: -110,
            iq: Vec::new(),
            events: 0,
            first_sample: 0,
            overrun_at: 0,
            resumed_at: 0,
        });
        let mut driver = XtrxDriver::open(api.clone(), None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        let (count, _) = driver.read(&mut output, Duration::ZERO).unwrap();
        assert_eq!(count, 0);
        let state = api.state.lock().unwrap();
        assert_eq!(
            state.last_receive_flags,
            RCVEX_DONT_INSER_ZEROS | RCVEX_DROP_OLD_ON_OVERFLOW | RCVEX_TIMOUT
        );
        assert_eq!(state.last_timeout_ms, 1);
    }

    #[test]
    fn filled_zero_event_marks_discontinuity() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: RCVEX_EVENT_FILLED_ZERO,
            first_sample: 5,
            overrun_at: 0,
            resumed_at: 0,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        let (_, metadata) = driver.read(&mut output, Duration::from_millis(1)).unwrap();
        assert!(metadata.overrun);
    }

    #[test]
    fn invalid_native_overflow_timestamps_are_rejected() {
        let api = Arc::new(MockApi::default());
        api.push_rx(MockRx {
            status: 0,
            iq: vec![0; 2],
            events: RCVEX_EVENT_OVERFLOW,
            first_sample: 100,
            overrun_at: 110,
            resumed_at: 100,
        });
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        let mut output = [Complex32::ZERO; 1];
        assert!(
            driver
                .read(&mut output, Duration::from_millis(1))
                .unwrap_err()
                .to_string()
                .contains("precedes")
        );
    }

    #[test]
    fn configuration_and_lifecycle_failures_are_preserved() {
        let api = Arc::new(MockApi::with_failure("tune_rx"));
        let mut driver = XtrxDriver::open(api.clone(), None, XtrxOptions::default()).unwrap();
        let error = driver.configure(&config()).unwrap_err().to_string();
        assert!(error.contains("tune_rx"));
        assert!(error.contains("mock libxtrx error -5"));
        drop(driver);
        assert_eq!(api.calls().last().map(String::as_str), Some("close"));

        let api = Arc::new(MockApi::default());
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        assert!(driver.start().is_err());
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        assert!(driver.configure(&config()).is_err());
        driver.stop().unwrap();
        driver.stop().unwrap();
    }

    #[test]
    fn validates_backend_specific_ranges() {
        let api = Arc::new(MockApi::default());
        let mut driver = XtrxDriver::open(api, None, XtrxOptions::default()).unwrap();
        let mut invalid = config();
        invalid.sample_rate_hz = 60_000_000;
        assert!(driver.configure(&invalid).is_err());
        invalid = config();
        invalid.bandwidth_hz = 500_000;
        assert!(driver.configure(&invalid).is_err());
        invalid = config();
        invalid.gain_db = 31.0;
        assert!(driver.configure(&invalid).is_err());
        assert!(validate_frequency(2_426_000_000.25).is_ok());
    }

    #[test]
    fn validates_options_timeout_and_abi_layouts() {
        assert!(
            XtrxOptions {
                packet_size: 32_768,
                ..XtrxOptions::default()
            }
            .validate()
            .is_err()
        );
        assert_eq!(duration_to_timeout_ms(Duration::from_micros(1)).unwrap(), 1);
        if usize::BITS == 64 {
            assert_eq!(std::mem::size_of::<XtrxRunStreamParams>(), 48);
            assert_eq!(std::mem::align_of::<XtrxRunStreamParams>(), 4);
            assert_eq!(std::mem::size_of::<XtrxRunParams>(), 160);
            assert_eq!(std::mem::align_of::<XtrxRunParams>(), 8);
            assert_eq!(std::mem::size_of::<XtrxRecvInfo>(), 56);
            assert_eq!(std::mem::align_of::<XtrxRecvInfo>(), 8);
        }
    }

    #[test]
    fn drop_stops_before_close() {
        let api = Arc::new(MockApi::default());
        let mut driver = XtrxDriver::open(api.clone(), None, XtrxOptions::default()).unwrap();
        driver.configure(&config()).unwrap();
        driver.start().unwrap();
        drop(driver);
        let calls = api.calls();
        assert_eq!(&calls[calls.len() - 2..], ["stop", "close"]);
    }
}
