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


# More Function Detail

**Device Management**

#### **1\. `xtrx_open`**

* **Purpose**: Opens a single XTRX device, establishing a connection to the hardware.  
* **Arguments**:  
  * `device`: Path to the XTRX device file, usually in `/dev/` (e.g., `/dev/xtrx0`).  
  * `flags`: Bitwise flags for logging levels and reset operations.  
    * `XTRX_O_LOGLVL_MASK`: Logging level control, default `0`.  
    * `XTRX_O_RESET`: Resets the device upon opening.  
  * `dev`: Output pointer to the `xtrx_dev` struct representing the device handle.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Use when initializing a connection to a single XTRX device for the first time in an application.

#### **2\. `xtrx_close`**

* **Purpose**: Closes a previously opened XTRX device, releasing associated resources.  
* **Arguments**:  
  * `dev`: Device handle to close.  
* **Return**: `void`.  
* **Typical Usage**: Called at the end of a session or before reinitializing a device.

#### **3\. `xtrx_open_multi`**

* **Purpose**: Opens multiple XTRX devices simultaneously, useful in multi-device setups.  
* **Arguments**:  
  * `dinfo`: A pointer to `xtrx_open_multi_info_t`, containing device paths, count, log level, and any extended flags.  
  * `dev`: Output pointer to an array of `xtrx_dev` handles, one for each device.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: For multi-device synchronization, like MIMO, to handle devices as a cohesive unit.

#### **4\. `xtrx_open_string`**

* **Purpose**: Opens one or multiple XTRX devices based on a formatted string, allowing dynamic selection and configuration.  
* **Arguments**:  
  * `paramstring`: A string defining devices and configuration options, separated by semicolons. Example: `"usb3380;;loglevel=7"`.  
  * `dev`: Output pointer to an array of `xtrx_dev` handles.  
* **Return**: Number of devices opened or an error code.  
* **Typical Usage**: Provides flexible initialization without hardcoding paths or parameters.

#### **5\. `xtrx_discovery`**

* **Purpose**: Discovers available XTRX devices connected to the system.  
* **Arguments**:  
  * `devs`: Pointer to an array of `xtrx_device_info_t` structs for storing device information.  
  * `maxbuf`: Maximum number of devices the buffer can store.  
* **Return**: Number of devices found or negative error code.  
* **Typical Usage**: Use to dynamically detect devices, especially useful in systems with multiple XTRX units.

### **Clock and Sample Rate Configuration**

#### **6\. `xtrx_set_ref_clk`**

* **Purpose**: Configures the reference clock source and frequency for an XTRX device, essential for achieving precise timing and synchronization.  
* **Arguments**:  
  * `dev`: Device handle for the XTRX device.  
  * `refclkhz`: Frequency of the reference clock in Hertz (Hz). Common values are `10,000,000` Hz or `40,000,000` Hz.  
  * `clksrc`: Clock source selection as an enum of type `xtrx_clock_source_t`:  
    * `XTRX_CLKSRC_INT`: Internal oscillator.  
    * `XTRX_CLKSRC_EXT`: External reference clock.  
    * `XTRX_CLKSRC_EXT_W1PPS_SYNC`: External clock synchronized with a 1PPS signal.  
* **Return**: `0` on success; negative error code otherwise.  
* **Typical Usage**: Called at startup or when switching clock sources; needed to set the desired timing source before sampling.

#### **7\. `xtrx_set_samplerate`**

* **Purpose**: Sets the sample rates for RX and TX paths, configuring internal clocks to achieve these rates.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `cgen_rate`: Target CGEN clock rate in Hz, used to drive RX/TX rates (0 for auto).  
  * `rxrate`: RX sample rate in Hz, post-decimation (0 to disable RX).  
  * `txrate`: TX sample rate in Hz, pre-interpolation (0 to disable TX).  
  * `flags`: Bitwise flags (`xtrx_samplerate_flags_t`) for debugging, enforcing, or bypassing clock constraints.  
  * `actualcgen`, `actualrx`, `actualtx`: Output pointers to store the actual configured rates for CGEN, RX, and TX.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Adjusted when initializing the device or changing RX/TX data rates for different applications.

### **Tuning and Bandwidth Control**

#### **8\. `xtrx_tune`**

* **Purpose**: Tunes the XTRX device to a specified frequency, typically for RX or TX, setting the LO (local oscillator) frequency.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `type`: Enum of type `xtrx_tune_t`, determining tuning mode:  
    * `XTRX_TUNE_RX_FDD`: RX frequency for FDD (Frequency Division Duplex).  
    * `XTRX_TUNE_TX_FDD`: TX frequency for FDD.  
    * `XTRX_TUNE_TX_AND_RX_TDD`: For TDD (Time Division Duplex), tunes both RX and TX.  
    * `XTRX_TUNE_BB_RX`: Baseband RX tuning (DSP-based).  
    * `XTRX_TUNE_BB_TX`: Baseband TX tuning (DSP-based).  
  * `freq`: Desired tuning frequency in Hz.  
  * `actualfreq`: Output pointer for the actual tuned frequency.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Called when setting the target center frequency, typically done at device initialization or to switch frequencies during operation.

#### **9\. `xtrx_tune_ex`**

* **Purpose**: Extended version of `xtrx_tune`, allowing finer control over which channel is tuned.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `type`: Same as `xtrx_tune`.  
  * `ch`: Enum of type `xtrx_channel_t` indicating the channel to tune:  
    * `XTRX_CH_A`: Channel A.  
    * `XTRX_CH_B`: Channel B.  
    * `XTRX_CH_AB`: Both channels (when possible).  
    * `XTRX_CH_ALL`: All available channels.  
  * `freq`: Desired frequency in Hz.  
  * `actualfreq`: Output pointer to the actual tuned frequency.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used in multi-channel or multi-device scenarios to configure each channel individually or ensure synchronization.

#### **10\. `xtrx_tune_tx_bandwidth` and `xtrx_tune_rx_bandwidth`**

* **Purpose**: Sets the bandwidth (BW) of the TX or RX filters, optimizing signal filtering for specific bandwidths.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `ch`: Channel selector, using `xtrx_channel_t`.  
  * `bw`: Desired bandwidth in Hz.  
  * `actualbw`: Output pointer for the actual bandwidth achieved.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Applied after setting the sample rate and frequency, often during initialization to match signal bandwidth requirements for a specific application.

### **Gain and Antenna Control**

#### **11\. `xtrx_set_gain`**

* **Purpose**: Sets the gain for the RX or TX path on a specific channel and gain type, which can be essential in optimizing signal quality.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `ch`: Channel selection, specified with `xtrx_channel_t` enum (e.g., `XTRX_CH_A`, `XTRX_CH_B`).  
  * `gt`: Gain type specified by the `xtrx_gain_type_t` enum:  
    * `XTRX_RX_LNA_GAIN`: Low-Noise Amplifier (LNA) gain for RX.  
    * `XTRX_RX_TIA_GAIN`: Trans-Impedance Amplifier (TIA) gain for RX.  
    * `XTRX_RX_PGA_GAIN`: Programmable Gain Amplifier (PGA) gain for RX.  
    * `XTRX_TX_PAD_GAIN`: Power Amplifier (PAD) gain for TX.  
  * `gain`: Desired gain level, in dB for RX and a value proportional to signal power for TX.  
  * `actualgain`: Output pointer to store the actual applied gain.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Typically called when initializing the device or when adjusting gain based on environmental factors.

#### **12\. `xtrx_set_antenna`**

* **Purpose**: Selects the active antenna for RX or TX paths, enabling control over the physical RF interface used for the signal.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `antenna`: Antenna selection, specified with the `xtrx_antenna_t` enum:  
    * `XTRX_RX_L`, `XTRX_RX_H`, `XTRX_RX_W`: Different RX antenna selections.  
    * `XTRX_TX_H`, `XTRX_TX_W`: TX antenna selections.  
    * `XTRX_RX_AUTO`, `XTRX_TX_AUTO`: Automatic selection.  
    * `XTRX_RX_ADC_EXT`: External ADC input.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Set during device initialization or in applications where different antennas are used based on the frequency or environment.

#### **13\. `xtrx_set_antenna_ex`**

* **Purpose**: Extended version of `xtrx_set_antenna` that allows per-channel control over the antenna.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `ch`: Channel selection (`xtrx_channel_t` enum), such as `XTRX_CH_A`, `XTRX_CH_B`, or `XTRX_CH_ALL`.  
  * `antenna`: Antenna selection, as in `xtrx_set_antenna`.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Useful in multi-channel configurations to specify different antennas for each channel.

### **Data Transmission and Reception**

#### **14\. `xtrx_run`**

* **Purpose**: Starts data streaming for RX and/or TX using simplified configuration parameters.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `params`: Pointer to `xtrx_run_params_t` struct, where only a subset of fields (like `dir` for direction and `flags` for settings) is required.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Use in basic streaming applications when you do not need extensive control over data paths, buffering, or timing.

#### **15\. `xtrx_run_ex`**

* **Purpose**: Starts RX and/or TX data streams with full control over stream parameters, including channel configuration, data format, and buffering.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `params`: Pointer to a fully populated `xtrx_run_params_t` struct, which contains detailed stream parameters for RX and TX paths.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: This function is ideal for complex applications requiring control over the data format, packet size, multi-channel configuration, and hardware timing settings.

#### **16\. `xtrx_stop`**

* **Purpose**: Stops RX and/or TX data streams, halting ongoing data transfers.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `dir`: Direction enum `xtrx_direction_t` for stopping RX (`XTRX_RX`), TX (`XTRX_TX`), or both (`XTRX_TRX`).  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used to halt data transfers during shutdown, reconfiguration, or in the event of an error.

### **GPIO and Timing**

#### **17\. `xtrx_gpio_configure`**

* **Purpose**: Configures a GPIO pin on the XTRX device to act as an input, output, or special-purpose pin, such as a Pulse Per Second (PPS) signal.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `devno`: Device number (index of the device if working with multiple devices).  
  * `gpio_num`: GPIO pin number to configure.  
  * `function`: Enum `xtrx_gpio_func_t` specifying the pin function:  
    * `XTRX_GPIO_FUNC_IN`: Configure as input.  
    * `XTRX_GPIO_FUNC_OUT`: Configure as output.  
    * `XTRX_GPIO_FUNC_PPS_O` / `XTRX_GPIO_FUNC_PPS_I`: Configure as PPS output/input.  
    * `XTRX_GPIO_FUNC_ALT0`, `XTRX_GPIO_FUNC_ALT1`, `XTRX_GPIO_FUNC_ALT2`: Alternative functions.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Called when initializing GPIOs for time synchronization, control signals, or triggering events.

#### **18\. `xtrx_gpio_out`**

* **Purpose**: Sets the state of one or more GPIO output pins.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `devno`: Device number (index in multi-device setups).  
  * `out`: Bitmask specifying which GPIO pins to set (1 to set high, 0 to set low).  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used for setting control signals or triggering events on external hardware.

#### **19\. `xtrx_gpio_clear_set`**

* **Purpose**: Atomically clears and sets specific GPIO pins, useful for controlling multiple GPIOs in a single operation.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `devno`: Device number.  
  * `clear_msk`: Bitmask indicating which GPIO pins to clear.  
  * `set_msk`: Bitmask indicating which GPIO pins to set.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Ideal for toggling multiple GPIOs simultaneously for precise timing or atomic updates to GPIO state.

#### **20\. `xtrx_gpio_in`**

* **Purpose**: Reads the state of GPIO input pins.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `devno`: Device number.  
  * `in`: Pointer to an unsigned integer to store the state of GPIO inputs.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used to monitor input signals, such as external triggers or PPS pulses.

#### **21\. `xtrx_gtime_op`**

* **Purpose**: Manages general-purpose timing operations (gtime) for functions like enabling/disabling timing or retrieving current time.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `devno`: Device number.  
  * `cmd`: Enum `xtrx_gtime_cmd_t` specifying the timing operation, such as:  
    * `XTRX_GTIME_ENABLE_INT`: Enable internal timing.  
    * `XTRX_GTIME_ENABLE_EXT`: Enable external timing.  
    * `XTRX_GTIME_GET_CUR`: Get current time.  
    * Other options for enabling/disabling external timing, setting resolutions, or corrections.  
  * `in`: Input structure `gtime_data_t` for command-related data.  
  * `out`: Output structure `gtime_data_t` to store result data (e.g., current time).  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used for time-stamping, synchronization, and coordinating precise timing signals in SDR applications.

### **Calibration, Register Access, and Low-Level Control**

#### **22\. `xtrx_val_set`**

* **Purpose**: Sets a low-level parameter or register value, allowing direct manipulation of hardware calibration or control registers.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `dir`: Enum `xtrx_direction_t`, specifies the direction (e.g., `XTRX_RX`, `XTRX_TX`).  
  * `chan`: Enum `xtrx_channel_t` for the channel (e.g., `XTRX_CH_A`, `XTRX_CH_B`).  
  * `type`: Enum `xtrx_val_t`, representing the parameter type (e.g., `XTRX_RFIC_CORR_DC_EN` for DC correction or `XTRX_LMS7_RSSI` for RSSI).  
  * `val`: Value to set for the specified parameter.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used for advanced control of parameters like RFIC correction values or DSP data rates, often necessary during calibration or custom tuning.

#### **23\. `xtrx_val_get`**

* **Purpose**: Retrieves the current value of a low-level parameter or register, useful for diagnostics, calibration, or monitoring hardware state.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `dir`: Direction (`XTRX_RX`, `XTRX_TX`).  
  * `chan`: Channel (`XTRX_CH_A`, `XTRX_CH_B`).  
  * `type`: Parameter type (e.g., `XTRX_RFIC_CORR_DC_EN`).  
  * `val`: Output pointer to store the current parameter value.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Commonly used in calibration and diagnostics, such as verifying DSP or RFIC parameter settings after applying a configuration.

#### **24\. `xtrx_calibrate`**

* **Purpose**: Performs a device calibration procedure to optimize signal paths and correct specific RFIC parameters.  
* **Arguments**: Typically takes device handle and additional calibration-specific settings (these can vary by implementation).  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Essential during device initialization to ensure proper RF performance, such as temperature compensation and LO alignment.

### **Debugging and Diagnostics**

#### **25\. `xtrx_debug_init`**

* **Purpose**: Initializes the debugging interface, allowing access to diagnostic and control functions.  
* **Arguments**:  
  * `params`: Optional parameters as a string for initialization.  
  * `ops`: Pointer to `xtrx_debug_ops_t` struct, defining specific debug operations.  
  * `obj`: Pointer to an object providing context for debugging.  
  * `octx`: Output pointer to the `xtrx_debug_ctx_t` debug context.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Used in environments requiring direct access to RFIC or board registers, especially for testing or debugging custom applications.

#### **26\. `xtrx_debug_free`**

* **Purpose**: Frees the debug context created by `xtrx_debug_init`.  
* **Arguments**:  
  * `ctx`: Debug context pointer.  
* **Return**: `void`.  
* **Typical Usage**: Ensures the clean release of debug resources, typically called at the end of a debug session.

#### **27\. `xtrx_log_setfunc`**

* **Purpose**: Sets a custom logging function to handle log messages from the XTRX library.  
* **Arguments**:  
  * `func`: Pointer to a logging function that follows the `xtrx_logfunc_t` signature, allowing custom log handling.  
* **Return**: `void`.  
* **Typical Usage**: Use in applications where custom logging is needed for debugging, monitoring, or integrating with larger systems.

#### **28\. `xtrx_log_setlevel`**

* **Purpose**: Sets the logging level for XTRX library messages, adjusting the verbosity of log output.  
* **Arguments**:  
  * `severity`: Logging level, with higher values for more detailed logs.  
  * `subsystem`: 4-character string specifying a subsystem to target (e.g., "XTRX" or "RXTX").  
* **Return**: `void`.  
* **Typical Usage**: Called during setup to control log verbosity, especially useful when debugging or performing detailed diagnostics.

### **Miscellaneous Functions**

#### **29\. `xtrx_host_format_get_iq_size`**

* **Purpose**: Determines the size of an IQ sample in bytes for the specified host format.  
* **Arguments**:  
  * `fmt`: Host data format, given as `xtrx_host_format_t` (e.g., `XTRX_IQ_FLOAT32` for 32-bit floating-point IQ samples).  
* **Return**: Size of one IQ sample in bytes for the given format.  
* **Typical Usage**: Useful when calculating buffer sizes or performing data conversions, especially if multiple host formats are used in an application.

#### **30\. `xtrx_wire_format_get_iq_size`**

* **Purpose**: Retrieves the size of an IQ sample in bytes for the specified wire format.  
* **Arguments**:  
  * `fmt`: Wire format specified as `xtrx_wire_format_t` (e.g., `XTRX_WF_16` for 16-bit wire format).  
* **Return**: Number of bytes in one IQ sample.  
* **Typical Usage**: Often used when managing memory buffers for transmitting or receiving data over PCIe or USB, as different wire formats may be required depending on performance and bandwidth constraints.

#### **31\. `xtrx_reset`**

* **Purpose**: Resets the XTRX device, restoring it to a default state and clearing all configurations.  
* **Arguments**:  
  * `dev`: Device handle.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Useful for reinitialization during fault recovery, or if a reset is needed after modifying low-level configurations.

#### **32\. `xtrx_set_gain_db`**

* **Purpose**: Sets the gain on a specific channel in decibels (dB), providing a more standardized unit for gain control.  
* **Arguments**:  
  * `dev`: Device handle.  
  * `channel`: Enum `xtrx_channel_t` specifying the channel (`XTRX_CH_A`, `XTRX_CH_B`, etc.).  
  * `gain_dB`: Gain value in decibels.  
* **Return**: `0` on success; error code otherwise.  
* **Typical Usage**: Common in applications requiring precise gain control, as it allows gain adjustments in a universally recognized unit (dB), especially useful in testing or calibration.

# Key Data Types and Structures

#### **1\. Device Handle and Contexts**

* **`struct xtrx_dev`**: The primary device handle structure for XTRX. This is used as a reference for most functions to perform operations on a specific XTRX device. It’s created by `xtrx_open` and `xtrx_open_multi` and used throughout the API.  
* **`xtrx_debug_ctx_t`**: Context structure used for debugging, created by `xtrx_debug_init` and released by `xtrx_debug_free`. Allows low-level access to registers and diagnostics.

#### **2\. Sample and Stream Parameters**

* **`xtrx_run_params_t`**: Core structure for configuring data streams (TX and RX). Includes fields for setting data direction, format, channel configuration, and streaming options.  
  * **`dir`**: Enum `xtrx_direction_t`, specifies direction (e.g., `XTRX_RX`, `XTRX_TX`, or `XTRX_TRX` for both).  
  * **`rx` / `tx`**: `xtrx_run_stream_params_t` sub-structures that define RX and TX parameters.  
    * **`wfmt`**: Wire format (`xtrx_wire_format_t`), specifies how samples are encoded for transmission.  
    * **`hfmt`**: Host format (`xtrx_host_format_t`), specifies the format in which samples are provided to or read from the host.  
    * **`chs`**: Channel mask (`xtrx_channel_t`) to define channels (e.g., `XTRX_CH_A`, `XTRX_CH_B`).  
    * **`paketsize`**: Packet size in samples.  
    * **`flags`**: Stream configuration flags (`xtrx_run_sp_flags_t`), defines options like loopback mode, test signals, scaling, etc.  
    * **`scale`**: Float value, scales floating-point data in `XTRX_IQ_FLOAT32` format, with the default of `[-1,1]`.  
  * **`rx_stream_start` / `tx_repeat_buf`**: Timestamp for stream start and optional repeat buffer for constant transmission.

#### **3\. Clock and Timing**

* **`xtrx_clock_source_t`**: Enum for selecting the clock source.  
  * **Values**: `XTRX_CLKSRC_INT`, `XTRX_CLKSRC_EXT`, `XTRX_CLKSRC_EXT_W1PPS_SYNC`  
* **`gtime_data_t`**: Structure to hold general-purpose timing data.  
  * **`sec`**: Seconds.  
  * **`nsec`**: Nanoseconds.

#### **4\. Channel and Stream Format Definitions**

* **`xtrx_channel_t`**: Enum for specifying channels in RX/TX paths.  
  * **Values**: `XTRX_CH_A`, `XTRX_CH_B`, `XTRX_CH_AB`, `XTRX_CH_ALL`  
* **`xtrx_direction_t`**: Enum for direction, specifying RX, TX, or both.  
  * **Values**: `XTRX_RX`, `XTRX_TX`, `XTRX_TRX`  
* **`xtrx_wire_format_t`**: Enum defining wire format, specifying the bit depth of IQ pairs on the wire.  
  * **Values**: `XTRX_WF_8` (8 bits), `XTRX_WF_12` (12 bits), `XTRX_WF_16` (16 bits)  
* **`xtrx_host_format_t`**: Enum defining the host data format, specifying how data is formatted for host processing.  
  * **Values**: `XTRX_IQ_FLOAT32` (32-bit float), `XTRX_IQ_INT16` (16-bit integer), `XTRX_IQ_INT8` (8-bit integer)

#### **5\. Tuning and Gain Control**

* **`xtrx_tune_t`**: Enum for tuning modes.  
  * **Values**: `XTRX_TUNE_RX_FDD`, `XTRX_TUNE_TX_FDD`, `XTRX_TUNE_TX_AND_RX_TDD`, `XTRX_TUNE_BB_RX`, `XTRX_TUNE_BB_TX`, `XTRX_TUNE_EXT_FE`  
* **`xtrx_gain_type_t`**: Enum defining gain stages.  
  * **Values**:  
    * `XTRX_RX_LNA_GAIN`: Low Noise Amplifier (LNA).  
    * `XTRX_RX_TIA_GAIN`: Trans-Impedance Amplifier (TIA).  
    * `XTRX_RX_PGA_GAIN`: Programmable Gain Amplifier (PGA).  
    * `XTRX_TX_PAD_GAIN`: Power Amplifier (PAD) for TX.

#### **6\. GPIO Control**

* **`xtrx_gpio_func_t`**: Enum for GPIO functionality.  
  * **Values**: `XTRX_GPIO_FUNC_IN`, `XTRX_GPIO_FUNC_OUT`, `XTRX_GPIO_FUNC_PPS_O`, `XTRX_GPIO_FUNC_PPS_I`, plus alternate functions `XTRX_GPIO_FUNC_ALT0`, `XTRX_GPIO_FUNC_ALT1`, and `XTRX_GPIO_FUNC_ALT2`.

#### **7\. Other Enums and Flags**

* **`xtrx_samplerate_flags_t`**: Flags for setting sample rate with debugging or constraint adjustments.  
  * **Values** include options like `XTRX_SAMPLERATE_FORCE_UPDATE` (update samplerate at runtime) and `XTRX_SAMPLERATE_AUTO_DECIM` (automatic decimation).  
* **`xtrx_run_params_flags_t`**: Flags for configuring run parameters.  
  * **Values** include options for enabling loopback mode (`XTRX_RUN_DIGLOOPBACK`), RX LFSR (`XTRX_RUN_RXLFSR`), and general timing (`XTRX_RUN_GTIME`).  
* **`xtrx_run_sp_flags_t`**: Stream flags that configure specific options for data streaming.  
  * **Values**:  
    * `XTRX_RSP_TEST_SIGNAL_A` / `XTRX_RSP_TEST_SIGNAL_B`: Injects test signals into channels A and B.  
    * `XTRX_RSP_SWAP_AB`: Swaps channels A and B.  
    * `XTRX_RSP_SISO_MODE`: Single Input, Single Output (SISO) mode.  
    * `XTRX_RSP_SCALE`: Enables scaling for `XTRX_IQ_FLOAT32`.

### **1\. Device Handle and Context Structures**

#### **`struct xtrx_dev`**

* This is an opaque handle returned by functions like `xtrx_open` and `xtrx_open_multi`. It represents the connection to an XTRX device and is the primary reference for almost all API calls. It holds internal state and configuration but doesn’t directly expose fields for modification.

#### **`xtrx_debug_ctx_t`**

* Used when initializing debug functionality with `xtrx_debug_init`, this context allows access to registers and diagnostics.  
* Common fields and usage include setting up custom diagnostic functions or inspecting internal RFIC register values.

### **2\. Stream Parameter Structure**

#### **`xtrx_run_params_t`**

* This structure configures RX and TX streaming, holding substructures for each direction (`rx` and `tx`) that define stream-specific settings.  
* Important fields:  
  * `dir`: Specifies the streaming direction, set using `xtrx_direction_t` (e.g., `XTRX_RX`, `XTRX_TX`, `XTRX_TRX` for both).  
  * **RX and TX Substructure (xtrx\_run\_stream\_params\_t)**:  
    * **`wfmt` (xtrx\_wire\_format\_t)**: Determines the format in which data is transferred over the wire:  
      * `XTRX_WF_8`: 8-bit IQ samples, packed tightly.  
      * `XTRX_WF_12`: 12-bit IQ samples, optimized for balancing fidelity and bandwidth.  
      * `XTRX_WF_16`: 16-bit IQ samples, high-fidelity with greater bandwidth usage.  
    * **`hfmt` (xtrx\_host\_format\_t)**: Specifies data format for the host:  
      * `XTRX_IQ_FLOAT32`: 32-bit floating point, `[-1, 1]` normalized range.  
      * `XTRX_IQ_INT16`: 16-bit signed integer.  
      * `XTRX_IQ_INT8`: 8-bit signed integer, compact for low-resolution data.  
    * **`chs` (xtrx\_channel\_t)**: Bitmask defining which channels are used (e.g., `XTRX_CH_A`, `XTRX_CH_B`, `XTRX_CH_AB` for both).  
    * **`packetsize`**: Number of samples per packet (e.g., 1024 or 2048), essential for buffer management and controlling data chunk size.  
    * **`flags` (xtrx\_run\_sp\_flags\_t)**: Flags for stream configuration:  
      * `XTRX_RSP_TEST_SIGNAL_A` / `XTRX_RSP_TEST_SIGNAL_B`: Inject test signals in specified channels.  
      * `XTRX_RSP_SWAP_AB`: Swap channels A and B.  
      * `XTRX_RSP_SCALE`: Scale IQ data for normalized floating-point formats.  
      * **Other Fields**:  
        * `scale`: Only applicable for `XTRX_IQ_FLOAT32`, allows custom scaling of samples.  
        * `rx_stream_start`: Starting timestamp for RX, helps control timing for burst RX streams.  
        * `tx_repeat_buf`: Specifies buffer for TX repeat mode, enabling periodic signal transmission.

### **3\. Clock and Timing Types**

#### **`xtrx_clock_source_t`**

* Specifies the clock source, allowing you to use an internal or external clock and sync with PPS.  
* Common values:  
  * `XTRX_CLKSRC_INT`: Internal clock, used when standalone operation is required.  
  * `XTRX_CLKSRC_EXT`: External clock, often used to synchronize with other devices.  
  * `XTRX_CLKSRC_EXT_W1PPS_SYNC`: External clock synced with 1-Pulse-Per-Second (1PPS), ideal for precise timing applications.

#### **`gtime_data_t`**

* Structure used for general-purpose timing, often populated in time-sensitive operations:  
  * **`sec`**: Integer field for seconds.  
  * **`nsec`**: Integer field for nanoseconds.  
* This structure helps manage timestamps, timing delays, and synchronizing events with precise timing control.

### **4\. Channel, Stream Format, and Direction Enums**

#### **`xtrx_channel_t`**

* Specifies which channel(s) to use, essential in multi-channel SDR applications:  
  * `XTRX_CH_A`, `XTRX_CH_B`: Single-channel modes.  
  * `XTRX_CH_AB`: Both channels active, used in MIMO or diversity applications.  
  * `XTRX_CH_ALL`: All available channels.

#### **`xtrx_direction_t`**

* Defines the streaming direction, ensuring data paths are configured as required:  
  * `XTRX_RX`: Receive-only mode.  
  * `XTRX_TX`: Transmit-only mode.  
  * `XTRX_TRX`: Both RX and TX enabled, typical in full-duplex or TDD systems.

#### **`xtrx_wire_format_t`**

* Specifies wire format for encoding IQ samples in data transfers:  
  * `XTRX_WF_8`, `XTRX_WF_12`, `XTRX_WF_16`: Different formats offer trade-offs in fidelity and bandwidth use.

#### **`xtrx_host_format_t`**

* Defines the format for host-side data processing:  
  * `XTRX_IQ_FLOAT32`: 32-bit floating point, commonly used when high accuracy is needed.  
  * `XTRX_IQ_INT16` and `XTRX_IQ_INT8`: Integer formats, suitable for memory-limited applications.

### **5\. Tuning and Gain Control Types**

#### **`xtrx_tune_t`**

* Used to select tuning modes, critical for frequency and LO configurations:  
  * `XTRX_TUNE_RX_FDD`, `XTRX_TUNE_TX_FDD`: Frequency Division Duplex (FDD) RX/TX modes.  
  * `XTRX_TUNE_TX_AND_RX_TDD`: Time Division Duplex (TDD) for both RX and TX.  
  * `XTRX_TUNE_BB_RX`, `XTRX_TUNE_BB_TX`: Baseband tuning for DSP-level adjustments.

#### **`xtrx_gain_type_t`**

* Allows setting gain at different stages in the RF chain, enabling precise control over amplification:  
  * `XTRX_RX_LNA_GAIN`: Low Noise Amplifier, boosts weak signals.  
  * `XTRX_RX_TIA_GAIN`: Trans-Impedance Amplifier, amplifies converted baseband signal.  
  * `XTRX_RX_PGA_GAIN`: Programmable Gain Amplifier, final gain stage for RX.  
  * `XTRX_TX_PAD_GAIN`: Power Amplifier Driver, controls TX signal power.

### **6\. GPIO Control Types**

#### **`xtrx_gpio_func_t`**

* Controls GPIO configuration, assigning pins for general I/O or specific timing:  
  * `XTRX_GPIO_FUNC_IN`, `XTRX_GPIO_FUNC_OUT`: Basic input/output.  
  * `XTRX_GPIO_FUNC_PPS_O` / `XTRX_GPIO_FUNC_PPS_I`: Pulse Per Second sync input/output, often for precise timing or sync with other devices.  
  * Alternate modes (`XTRX_GPIO_FUNC_ALT0`, `XTRX_GPIO_FUNC_ALT1`, `XTRX_GPIO_FUNC_ALT2`): Used for custom functionality or board-specific I/O needs.

### **7\. Other Enums and Flags**

#### **`xtrx_samplerate_flags_t`**

* Flags that allow tuning sample rate behavior, handling various operational constraints:  
  * `XTRX_SAMPLERATE_FORCE_UPDATE`: Forces update of sample rates at runtime.  
  * `XTRX_SAMPLERATE_AUTO_DECIM`: Enables automatic decimation adjustment based on RX/TX rates.

#### **`xtrx_run_params_flags_t`**

* General flags for the `xtrx_run_params_t` configuration:  
  * `XTRX_RUN_DIGLOOPBACK`: Enables loopback for testing data paths.  
  * `XTRX_RUN_RXLFSR`: Enables RX Linear Feedback Shift Register for signal testing.  
  * `XTRX_RUN_GTIME`: Enables time-based start control, using a timestamp.

#### **`xtrx_run_sp_flags_t`**

* Stream-specific flags for finer control:  
  * `XTRX_RSP_TEST_SIGNAL_A` / `XTRX_RSP_TEST_SIGNAL_B`: Inserts test signals for diagnostics.  
  * `XTRX_RSP_SWAP_AB`: Swaps channels A and B.  
  * `XTRX_RSP_SISO_MODE`: Configures for SISO (Single Input, Single Output) operation, disables channel pairing.  
  * `XTRX_RSP_SCALE`: Enables scaling, especially useful for floating-point IQ data.

### **8\. Device State and Error Codes**

#### **Device State Variables and Status Functions**

While there aren’t direct state variables exposed, `libxtrx` provides implicit device state control and error handling mechanisms that help manage the state of XTRX devices:

* **Error Handling**:  
  * Functions generally return `0` for success and negative values for errors. Typical errors include:  
    * **Device Errors** (e.g., `XTRX_EBUSY` if the device is already in use).  
    * **Configuration Errors** (e.g., `XTRX_EINVAL` for invalid parameters).  
    * **Streaming Errors** (e.g., `XTRX_ETIMEOUT` if there are timing mismatches).  
  * It’s good practice to check these return codes and implement error logging or state checks.  
* **Status Querying**:  
  * XTRX’s functions rely on implicit state maintained in `xtrx_dev`, which tracks configuration settings (like RX/TX active states) and data flow.  
  * The `xtrx_debug_init` and associated diagnostic functions can help verify device state by reading internal RFIC or baseband registers, especially useful in debugging.

#### **Important Debugging and Logging Settings**

* **Logging Levels**:  
  * The library uses log levels to control verbosity, configured through `xtrx_log_setlevel`:  
    * **Critical**: Errors causing operation failure.  
    * **Error**: Major issues, but not always fatal.  
    * **Warning**: Potentially problematic settings or situations.  
    * **Info**: Standard information about device actions.  
    * **Debug**: Detailed output, suitable for diagnostics.

---

### **9\. Advanced Timing Control and Synchronization**

#### **General-Purpose Timing (`gtime`)**

* **Usage of `xtrx_gtime_op`**:  
  * Controls precise timing for RX and TX start/stop actions.  
  * Common commands include `XTRX_GTIME_ENABLE_INT` (enable internal timing) and `XTRX_GTIME_ENABLE_EXT` (enable external timing).  
  * **Timing Resolution**: Controlled in `gtime_data_t` fields (`sec`, `nsec`) to achieve high-precision control of RX/TX synchronization.  
* **1PPS and External Clock Sync**:  
  * GPIO configuration with `XTRX_GPIO_FUNC_PPS_I` / `XTRX_GPIO_FUNC_PPS_O` allows synchronization with an external PPS signal.  
  * This is essential for applications where multiple devices must stay in sync, like phased-array systems or time-based RF experiments.

---

### **10\. Application-Specific Usage and Examples**

#### **Multi-Channel and MIMO Operations**

* In MIMO setups, `xtrx_open_multi` can open multiple devices simultaneously, providing a common handle for synchronized control.  
* Use `xtrx_run_ex` with channel masks set to `XTRX_CH_AB` to operate channels A and B simultaneously.  
* Multi-device synchronization can be achieved by configuring the same reference clock and using 1PPS sync or other external timing inputs for precise alignment.

#### **Common Configurations and Scaling**

* **Gain and Signal Level Adjustments**:  
  * For SDR applications requiring manual control, `xtrx_set_gain` allows configuration of each gain stage independently (e.g., LNA, TIA, PGA, PAD).  
  * Gain settings may vary across frequency ranges, so using `xtrx_val_get` to verify applied settings helps ensure signal quality and strength.  
* **Data Format Considerations**:  
  * The combination of `wfmt` (wire format) and `hfmt` (host format) helps optimize between data fidelity and transfer rates.  
  * For real-time applications with limited bandwidth, `XTRX_WF_8` and `XTRX_IQ_INT8` reduce transfer load, while `XTRX_WF_16` and `XTRX_IQ_FLOAT32` ensure high precision.

#### **Data Transmission Patterns**

* **Burst Transmission and Reception**:  
  * Functions like `xtrx_send_burst_sync` and `xtrx_recv_burst_sync` allow precise control over discrete data bursts.  
  * Useful for applications that require capturing short signal pulses or controlled RF emissions.

---

### **11\. Custom Extension with Debug Functions**

#### **Debug Extensions and Register-Level Access**

* **Low-Level Register Access**:  
  * Through `xtrx_debug_init`, custom diagnostics can be implemented to read and modify internal registers for RFIC and baseband processors, which is particularly useful for firmware development and custom signal processing implementations.  
* **Custom Logging and Callback Functions**:  
  * The logging interface can be extended with `xtrx_log_setfunc` to use custom logging functions, enabling integration with existing monitoring systems or enhanced debugging capabilities for complex applications.

---

### **Example Scenario**

To tie all of this together, here’s an example configuration for a multi-device phased-array setup with specific settings:

1. **Initialization**:  
   * Call `xtrx_discovery` to list available devices.  
   * Use `xtrx_open_multi` to open and initialize multiple devices.  
2. **Synchronization**:  
   * Set the reference clock to `XTRX_CLKSRC_EXT_W1PPS_SYNC` on each device to ensure timing precision.  
   * Configure GPIO pins as `XTRX_GPIO_FUNC_PPS_I` on each device to receive a common PPS pulse.  
3. **Channel and Stream Configuration**:  
   * Configure RX channels using `xtrx_run_ex` with `XTRX_CH_AB` and enable scaling.  
   * Use `XTRX_WF_16` and `XTRX_IQ_FLOAT32` for maximum signal fidelity.  
   * Adjust LNA and TIA gain using `xtrx_set_gain` to handle varying signal strength across channels.  
4. **Transmission and Data Collection**:  
   * Use `xtrx_send_burst_sync` for discrete signal transmission, especially in pulse-based systems.  
   * Synchronize start times across devices using `xtrx_gtime_op` to ensure all devices align with the 1PPS signal.
