Here's an interface summary based on the key `libxtrx` functions available for use in your Rust code through FFI.

1. **Device Management**  
   * `xtrx_open`, `xtrx_close`, `xtrx_open_multi`, `xtrx_open_string`, `xtrx_discovery`  
2. **Clock and Sample Rate Configuration**  
   * `xtrx_set_ref_clk`, `xtrx_set_samplerate`  
3. **Tuning and Bandwidth**  
   * `xtrx_tune`, `xtrx_tune_ex`, `xtrx_tune_tx_bandwidth`, `xtrx_tune_rx_bandwidth`  
4. **Gain and Antenna Control**  
   * `xtrx_set_gain`, `xtrx_set_antenna`, `xtrx_set_antenna_ex`  
5. **Data Transmission and Reception**  
   * `xtrx_run`, `xtrx_run_ex`, `xtrx_stop`  
   * Synchronous burst: `xtrx_send_burst_sync`, `xtrx_recv_burst_sync`  
   * Extended synchronous streaming: `xtrx_send_sync_ex`, `xtrx_recv_sync_ex`  
   * Stream buffer handling: `xtrx_run_params_init`  
6. **GPIO and Timing**  
   * GPIO setup and control: `xtrx_gpio_configure`, `xtrx_gpio_out`, `xtrx_gpio_clear_set`, `xtrx_gpio_in`  
   * Timing functions: `xtrx_gtime_op`  
7. **Calibration and Register Access**  
   * `xtrx_val_set`, `xtrx_val_get` for low-level RF calibration and register access, plus `xtrx_calibrate` for hardware calibration.  
8. **Debugging and Diagnostics**  
   * `xtrx_debug_init`, `xtrx_debug_free`, logging: `xtrx_log_setfunc`, `xtrx_log_setlevel`  
9. **Miscellaneous Functions**  
   * Data formatting and scaling: `xtrx_host_format_get_iq_size`, `xtrx_wire_format_get_iq_size`  
   * Device reset: `xtrx_reset`  
   * Gain setting in dB: `xtrx_set_gain_db`

With these functions, you have access to a full suite of operations for managing and utilizing XTRX SDR devices at multiple abstraction levels, from high-level streaming to granular, low-level device control.  
**Device Management**

* **`xtrx_open`**: Opens an XTRX device by specifying the device path and flags.  
  **Args**: `device` (C string), `flags` (unsigned int), `dev` (pointer to device handle).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_close`**: Closes an XTRX device. **Args**: `dev` (pointer to device handle).  
  **Returns**: `void`.

### **Clock and Sampling Configuration**

* **`xtrx_set_ref_clk`**: Sets the reference clock frequency and source.  
  **Args**: `dev` (pointer to device handle), `refclkhz` (unsigned, reference frequency in Hz), `clksrc` (enum for clock source).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_set_samplerate`**: Configures the device's sample rate for RX/TX.  
  **Args**: `dev`, `cgen_rate` (double), `rxrate` (double), `txrate` (double), `flags` (unsigned), `actualcgen`, `actualrx`, `actualtx` (all pointers to doubles for returning actual rates).  
  **Returns**: `0` on success, error code otherwise.

### **Tuning**

* **`xtrx_tune`**: Tunes the XTRX device to a specific frequency for TX or RX.  
  **Args**: `dev`, `type` (enum for tuning type), `freq` (double), `actualfreq` (pointer to double).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_tune_tx_bandwidth` / `xtrx_tune_rx_bandwidth`**: Tunes the device's bandwidth for TX/RX channels.  
  **Args**: `dev`, `ch` (enum for channel), `bw` (double), `actualbw` (pointer to double for actual bandwidth).  
  **Returns**: `0` on success, error code otherwise.

### **Gain and Antenna Control**

* **`xtrx_set_gain`**: Adjusts the gain for a specific channel and gain type.  
  **Args**: `dev`, `ch` (enum for channel), `gt` (enum for gain type), `gain` (double), `actualgain` (pointer to double for actual gain).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_set_antenna`**: Sets the active antenna for RX or TX.  
  **Args**: `dev`, `antenna` (enum for antenna selection).  
  **Returns**: `0` on success, error code otherwise.

### **Data Transmission**

* **`xtrx_run_ex`**: Starts the data transmission/reception based on specified parameters.  
  **Args**: `dev`, `params` (pointer to a struct containing stream and transmission parameters).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_stop`**: Stops data transmission/reception for the specified direction.  
  **Args**: `dev`, `dir` (enum indicating RX, TX, or both).  
  **Returns**: `0` on success, error code otherwise.

### **Device Discovery and Logging**

* **`xtrx_discovery`**: Lists available XTRX devices connected to the system.  
  **Args**: `devs` (pointer to an array of `xtrx_device_info_t` structs), `maxbuf` (size of the array).  
  **Returns**: Number of devices found or a negative error code.  
* **`xtrx_log_setfunc`**: Sets a custom logging function to handle log messages from the XTRX library.  
  **Args**: `func` (pointer to a logging function with a specific signature).  
  **Returns**: `void`.  
* **`xtrx_log_setlevel`**: Adjusts the logging level for the XTRX library to control verbosity.  
  **Args**: `severity` (integer log level), `subsystem` (4-character string identifying the subsystem).  
  **Returns**: `void`.

### **GPIO and General Purpose Timing**

* **`xtrx_gpio_configure`**: Configures a GPIO pin on the XTRX device.  
  **Args**: `dev`, `devno` (device index), `gpio_num` (pin number), `function` (enum defining function as input/output or specific GPIO function).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_gpio_out` / `xtrx_gpio_in`**: Sets or gets the state of GPIO pins.  
  **Args**: For `xtrx_gpio_out`, `dev`, `devno` (device index), and `out` (output value). For `xtrx_gpio_in`, `dev`, `devno`, and a pointer to `in` for receiving the input value.  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_gtime_op`**: Controls the general-purpose timing (gtime) operations like enabling, disabling, or querying current time.  
  **Args**: `dev`, `devno`, `cmd` (gtime operation command), `in` (input `gtime_data_t` struct), and `out` (output `gtime_data_t` struct for results).  
  **Returns**: `0` on success, error code otherwise.

### **Advanced Sampling Control**

* **`xtrx_set_antenna_ex`**: Allows setting antenna on a specific channel.  
  **Args**: `dev`, `ch` (channel identifier), `antenna` (enum for antenna selection).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_send_sync_ex`**: Extended synchronous transmission function, which provides more control over sample buffers and timing for TX operations.  
  **Args**: `dev`, `info` (pointer to `xtrx_send_ex_info_t` struct with detailed buffer and timing settings).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_recv_sync_ex`**: Extended synchronous reception function, supporting additional options for buffer management and timing in RX operations.  
  **Args**: `dev`, `info` (pointer to `xtrx_recv_ex_info_t` struct with reception configuration).  
  **Returns**: `0` on success, error code otherwise.

### **Value Manipulation and Direct Register Access**

* **`xtrx_val_set` / `xtrx_val_get`**: Allows direct manipulation of various low-level parameters or registers for advanced tuning or configuration.  
  **Args**: For `xtrx_val_set`, `dev`, `dir` (direction), `chan` (channel), `type` (parameter type, e.g., calibration or RFIC-specific), and `val` (value to set). For `xtrx_val_get`, similar arguments but with an output pointer for `val`.  
  **Returns**: `0` on success, error code otherwise.

### **Debugging and Diagnostics**

* **`xtrx_debug_init`**: Initializes the debugging interface, which allows for low-level control and diagnostics. This is used for advanced users needing to read/write internal registers or debug the RF chain.  
  **Args**: `params` (debug parameters as a string), `ops` (pointer to a struct defining debug operations), `obj` (object pointer for debugging context), `octx` (pointer to debug context).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_debug_free`**: Frees the debug context created by `xtrx_debug_init`.  
  **Args**: `ctx` (debug context pointer).  
  **Returns**: `void`.

### **Multi-Device and Reset Operations**

* **`xtrx_open_multi`**: Opens multiple XTRX devices at once, which can be useful for MIMO or phased-array applications.  
  **Args**: `dinfo` (pointer to an `xtrx_open_multi_info_t` struct containing device paths, log level, etc.), `dev` (pointer to device handle array).  
  **Returns**: `0` on success, error code otherwise.  
* **`xtrx_open_string`**: Parses a string of device parameters and opens the specified XTRX devices. This function provides flexibility for dynamic device selection.  
  **Args**: `paramstring` (string of device parameters), `dev` (pointer to device handle array).  
  **Returns**: Number of devices opened on success, error code otherwise.  
* **`xtrx_stop`**: Stops RX, TX, or both operations, clearing device buffers and halting the stream.  
  **Args**: `dev`, `dir` (direction to stop, e.g., RX or TX).  
  **Returns**: `0` on success, error code otherwise.

### **Advanced Transmission and Reception Parameters**

* **`xtrx_run_params_init`**: Initializes the `xtrx_run_params_t` structure with default values, ensuring that all relevant fields are set for transmission or reception.  
  **Args**: `params` (pointer to `xtrx_run_params_t` struct).  
  **Returns**: `void`.

### **Host Format and Wire Format Converters**

* **`xtrx_host_format_get_iq_size`**: Returns the size in bytes of an IQ sample in the host format. Useful when working with raw buffers or converting between formats.  
  **Args**: `fmt` (host format enumeration, e.g., `XTRX_IQ_FLOAT32`).  
  **Returns**: Size in bytes.  
* **`xtrx_wire_format_get_iq_size`**: Returns the size in bytes of an IQ sample in the wire format, also helpful for buffer management during transmission and reception.  
  **Args**: `fmt` (wire format enumeration, e.g., `XTRX_WF_16`).  
  **Returns**: Size in bytes.

### **Stream Handling and Data Buffering**

* **`xtrx_run`**: Starts the RX and TX data streams using simplified parameters. This function operates with fewer customization options compared to `xtrx_run_ex`.  
  **Args**: `dev` (device handle), `params` (pointer to `xtrx_run_params_t` struct, but with fewer required fields).  
  **Returns**: `0` on success, error code otherwise.  
  *Note*: Useful when basic configuration is sufficient and fine-grained control isn’t necessary.  
* **`xtrx_send_burst_sync`**: Sends a burst of samples synchronously. This is beneficial in real-time applications where precise timing or a single burst transmission is required.  
  **Args**: `dev`, `buffer` (pointer to data buffer), `samples` (number of samples to send), `timeout` (timeout for operation).  
  **Returns**: Number of samples sent on success, error code otherwise.  
* **`xtrx_recv_burst_sync`**: Receives a burst of samples synchronously. Suitable for applications that need to capture data in discrete bursts.  
  **Args**: `dev`, `buffer` (pointer to data buffer), `samples` (number of samples to receive), `timeout` (timeout for operation).  
  **Returns**: Number of samples received on success, error code otherwise.

### **GPIO and Event Handling**

* **`xtrx_gpio_clear_set`**: Allows atomic clear-and-set operations on GPIO pins, useful for precise timing or toggling pins within one operation.  
  **Args**: `dev`, `devno` (device index), `clear_msk` (mask to clear pins), `set_msk` (mask to set pins).  
  **Returns**: `0` on success, error code otherwise.

### **Calibration, Register Access, and Low-Level Control**

* **`xtrx_val_set` / `xtrx_val_get` (Extended)**: In addition to setting values, this function provides access to hardware-specific calibration values, internal registers, and low-level configurations.  
  **Args**: `type` (e.g., `XTRX_LMS7_XSP_SIGNAL`, `XTRX_RFIC_CORR_DC_EN`), enabling control over RFIC calibration and low-level RF tuning beyond standard APIs.  
* **`xtrx_calibrate`**: An internal calibration routine, potentially useful if custom calibration is necessary in production or field-testing setups. This function may involve specific hardware-dependent parameters not exposed in standard usage.  
  **Args**: Typically device-specific settings (undocumented, requires testing).  
  **Returns**: `0` on success, error otherwise.

### **Miscellaneous**

* **`xtrx_set_gain_db`**: Sets the gain in decibels (dB) for a specific channel, useful in cases where absolute gain values are required.  
  **Args**: `dev`, `channel`, `gain_dB` (float).  
  **Returns**: `0` on success, error otherwise.  
* **`xtrx_reset`**: Resets the XTRX device, clearing all configurations and returning it to its default state. This is particularly helpful in scenarios where reinitialization or fault recovery is required.  
  **Args**: `dev` (device handle).  
  **Returns**: `0` on success, error otherwise.
