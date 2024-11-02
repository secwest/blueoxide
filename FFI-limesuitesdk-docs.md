### **Major Data Types**

1. **`TestCallback`**: Function type `int(int testID, int event, const char* msg)`, used for callback during tests.  
2. **`RFTestData`**: Struct containing fields for test configurations:  
   * `float rxfreq`, `float txfreq`, `float peakval`, `float peakfreq`, `int ch`.  
3. **`SoapySDR::Kwargs`**: A map-like structure representing key-value pairs for SDR configuration.  
4. **`lms7Device`**: Pointer to `LMS7_Device`, handles low-level device management.  
5. **`Channel` (struct)**: Holds channel parameters like `freq`, `bw`, `rf_bw`.

### **Common Argument Descriptions**

1. **Direction**: Integer, 0 for RX, 1 for TX.  
2. **Channel**: Size\_t indicating the channel number.  
3. **Frequency, Gain, Bandwidth**: Floating-point values for the respective parameters.  
4. **Device**: Pointer to `lime::LMS7_Device` or related types.  
   * 

---

### **Key Data Structures**

1. **`RFTestData`**: Holds data for RF testing with fields:  
   * `float rxfreq`: Receive frequency.  
   * `float txfreq`: Transmit frequency.  
   * `float peakval`: Peak signal value detected.  
   * `float peakfreq`: Frequency at peak signal.  
   * `int ch`: Channel used for the test.

**`TestCallback`**: Type definition for callback functions, signature:

`typedef std::function<int(int testID, int event, const char* msg)> TestCallback;`

2.   
   * **Purpose**: Allows customized handling of events within tests.  
3. **`SoapySDR::Kwargs`**: Key-value argument dictionary for SDR configurations, commonly used for passing device parameters.

---

### **Identification and Device Control API**

This API provides functions to identify the LimeSDR device, open/close connections, and manage device information.

---

1. **`LMS_GetDeviceList(lms_info_str_t *dev_list)`**  
   * **Description**: Retrieves a list of LimeSDR devices currently connected to the system.  
   * **Arguments**:  
     * `lms_info_str_t *dev_list`: An array of `lms_info_str_t` structures. This output array is populated with details about each connected device.  
   * **Returns**: `int` \- The number of connected devices, or `-1` if an error occurs.  
   * **Usage**: Use this function to scan for available LimeSDR devices and retrieve information on each, such as serial numbers, which can be useful for selecting a specific device.  
2. **`LMS_Open(lms_device_t **device, const lms_info_str_t info, void* args = nullptr)`**  
   * **Description**: Opens a connection to a LimeSDR device, creating a device handle for further communication.  
   * **Arguments**:  
     * `lms_device_t **device`: A pointer to a device handle. This handle is allocated and set upon successful connection.  
     * `const lms_info_str_t info`: A device information string obtained from `LMS_GetDeviceList`. If set to `nullptr`, it opens the first available device.  
     * `void* args`: Optional arguments for custom configurations (usually left as `nullptr`).  
   * **Returns**: `int` \- `0` if successful, `-1` if an error occurs.  
   * **Usage**: Use this function as the first step in communicating with a LimeSDR device. Pass the resulting `device` handle to other functions for further operations.  
3. **`LMS_Close(lms_device_t *device)`**  
   * **Description**: Closes an open LimeSDR device and frees associated resources.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle to be closed.  
   * **Usage**: This function should always be called to properly disconnect and free resources when you are done using the LimeSDR device.  
4. **`LMS_GetDeviceInfo(lms_device_t *device)`**  
   * **Description**: Retrieves detailed information about a connected LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle.  
   * **Returns**: `lms_dev_info_t*` \- A pointer to a structure containing device information, including fields such as:  
     * `deviceName`: The name of the device model.  
     * `expansionName`: The name of any connected expansion.  
     * `firmwareVersion`: The device’s firmware version.  
     * `hardwareVersion`: The hardware version.  
     * `protocolVersion`: Communication protocol version.  
     * `gatewareVersion`: FPGA gateware version.  
     * `gatewareRevision`: Revision number of the gateware.  
     * `boardSerialNumber`: Serial number of the board.  
   * **Usage**: Useful for verifying compatibility and tracking device specifics, such as firmware or gateware version.  
5. **`GetDeviceName(eLMS_DEV device)`**  
   * **Description**: Returns the name of a specific LimeSDR device type.  
   * **Arguments**:  
     * `eLMS_DEV device`: Enum specifying the type of LimeSDR device (e.g., `LMS_DEV_LIMESDR`, `LMS_DEV_LIMESDRMINI`).  
   * **Returns**: `std::string` \- The name of the specified device type.  
   * **Usage**: Helps in distinguishing among various supported LimeSDR device types when multiple models are used.  
6. **`EnableCache(bool enable)`**  
   * **Description**: Enables or disables caching of register values in memory, reducing redundant register reads.  
   * **Arguments**:  
     * `bool enable`: If `true`, enables caching; if `false`, disables it.  
   * **Usage**: Enabling caching can improve performance in applications with frequent register reads. However, caching should be disabled in scenarios requiring real-time register updates.  
7. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables register caching for a specific device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool enable`: If `true`, enables caching; if `false`, disables it.  
   * **Usage**: Similar to `EnableCache`, but allows finer control by enabling or disabling cache settings on a per-device basis.

### **Stream and Buffer Management API**

1. **`GetBuffersCount() const`**  
   * **Description**: Retrieves the number of buffers currently allocated for active data streams.  
   * **Arguments**: None.  
   * **Returns**: `int` \- The number of allocated buffers.  
   * **Usage**: Useful for monitoring and debugging buffer allocation, especially when optimizing memory usage or troubleshooting buffer overflow issues during high-throughput streaming.  
2. **`CheckStreamSize(int size) const`**  
   * **Description**: Checks whether a given buffer size is compatible with the device’s streaming requirements.  
   * **Arguments**:  
     * `int size`: The desired buffer size in bytes.  
   * **Returns**: `bool` \- `true` if the specified size is valid; `false` if it is invalid.  
   * **Usage**: Ensures that the buffer size aligns with device constraints before setting up a stream, helping avoid data transfer errors due to incompatible buffer sizes.  
3. **`ResetStreamBuffers()`**  
   * **Description**: Clears all data from the buffers used by active streams, effectively resetting the stream state.  
   * **Arguments**: None.  
   * **Usage**: This function is particularly helpful after pausing or restarting a stream, as it prevents any stale or corrupted data from persisting in the buffers. It’s commonly used when recovering from streaming errors or interruptions.  
4. **`LMS_SetupStream(lms_device_t *device, lms_stream_t *stream, bool is_tx, size_t channel, lms_stream_fmt_t fmt, double sample_rate, size_t fifo_size)`**  
   * **Description**: Configures a data stream with specified parameters, preparing it for activation.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_stream_t *stream`: Pointer to a stream object to be set up.  
     * `bool is_tx`: Set to `true` for a transmit stream, `false` for receive.  
     * `size_t channel`: Channel number (typically 0 or 1).  
     * `lms_stream_fmt_t fmt`: Stream format, such as `LMS_FMT_F32`, `LMS_FMT_I16`, or `LMS_FMT_I12`.  
     * `double sample_rate`: Desired sample rate for the stream in Hz.  
     * `size_t fifo_size`: Size of the FIFO buffer in samples.  
   * **Returns**: `int` \- `0` on success, `-1` if setup fails.  
   * **Usage**: This function initializes a stream with specific settings and must be called before activating the stream. It allows configuration of essential parameters like sample rate, stream direction, and buffer size.  
5. **`LMS_StartStream(lms_stream_t *stream)`**  
   * **Description**: Starts a previously set up stream, beginning data transfer.  
   * **Arguments**:  
     * `lms_stream_t *stream`: The stream object to be activated.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: After setting up a stream, use this function to start data transmission (TX) or reception (RX). Once activated, the stream continuously transfers data until stopped.  
6. **`LMS_StopStream(lms_stream_t *stream)`**  
   * **Description**: Stops an active stream, halting data transfer.  
   * **Arguments**:  
     * `lms_stream_t *stream`: The stream object to be deactivated.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: Use this function to stop data flow in a stream without closing the stream configuration. This is typically used when pausing data transfer.  
7. **`LMS_DestroyStream(lms_device_t *device, lms_stream_t *stream)`**  
   * **Description**: Destroys a configured stream, freeing associated resources.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_stream_t *stream`: The stream object to be destroyed.  
   * **Usage**: This function should be called after stopping a stream to clean up resources. It removes the stream configuration and clears any remaining buffers.  
8. **`LMS_RecvStream(lms_stream_t *stream, void *samples, size_t sample_count, lms_stream_meta_t *meta, unsigned timeout_ms)`**  
   * **Description**: Receives data from an active RX stream.  
   * **Arguments**:  
     * `lms_stream_t *stream`: The RX stream object.  
     * `void *samples`: Buffer to store received samples.  
     * `size_t sample_count`: Number of samples to read.  
     * `lms_stream_meta_t *meta`: Metadata for the received samples (e.g., timestamp).  
     * `unsigned timeout_ms`: Timeout in milliseconds.  
   * **Returns**: `int` \- Number of samples received, `-1` on error.  
   * **Usage**: Call this function in a loop to read data from an RX stream. The metadata parameter allows access to additional information, such as timestamps, which is useful for synchronized data acquisition.  
9. **`LMS_SendStream(lms_stream_t *stream, const void *samples, size_t sample_count, const lms_stream_meta_t *meta, unsigned timeout_ms)`**  
   * **Description**: Sends data through an active TX stream.  
   * **Arguments**:  
     * `lms_stream_t *stream`: The TX stream object.  
     * `const void *samples`: Buffer containing samples to transmit.  
     * `size_t sample_count`: Number of samples to transmit.  
     * `const lms_stream_meta_t *meta`: Metadata for the transmitted samples.  
     * `unsigned timeout_ms`: Timeout in milliseconds.  
   * **Returns**: `int` \- Number of samples transmitted, `-1` on error.  
   * **Usage**: Use this function to write data to a TX stream for transmission. Metadata can be used for timestamping, which is particularly useful in applications that require precise timing.  
10. **`LMS_GetStreamStatus(lms_stream_t *stream, lms_stream_status_t *status)`**  
    * **Description**: Retrieves the current status of an active stream, including information on the FIFO buffer, overflows, and underflows.  
    * **Arguments**:  
      * `lms_stream_t *stream`: The stream object whose status is being queried.  
      * `lms_stream_status_t *status`: Pointer to a structure where stream status is stored.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: This function is valuable for real-time monitoring of stream health. It provides insights into buffer levels, underflow/overflow events, and data rates, aiding in troubleshooting data flow issues.

### **Advanced Programming and GPIO API**

1. **`ProgramWrite(const char *buffer, size_t length, int programmingMode, int index, ProgrammingCallback callback)`**  
   * **Description**: Writes firmware or other binary data to the device memory or flash storage.  
   * **Arguments**:  
     * `const char *buffer`: Pointer to the buffer containing the binary data to be programmed.  
     * `size_t length`: Length of the data buffer in bytes.  
     * `int programmingMode`: Mode for programming, such as writing to RAM, EEPROM, or FLASH. Accepted values include:  
       * `PROGRAM_MODE_RAM` – Temporary programming to volatile RAM.  
       * `PROGRAM_MODE_FLASH` – Permanent programming to non-volatile flash.  
       * `PROGRAM_MODE_EEPROM` – Permanent programming to EEPROM memory.  
     * `int index`: Target device index, allowing for multiple devices to be programmed if connected.  
     * `ProgrammingCallback callback`: Optional callback function to monitor programming progress.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function is used to upload firmware updates or custom binaries to the device. The `programmingMode` parameter determines if the update is temporary (in RAM) or permanent (in FLASH or EEPROM).  
2. **`ProgramMCU(lms_device_t *device, const uint8_t *data, size_t length, bool boot, bool erase)`**  
   * **Description**: Programs the device’s microcontroller unit (MCU) with a firmware image.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `const uint8_t *data`: Pointer to the firmware data buffer.  
     * `size_t length`: Length of the data buffer in bytes.  
     * `bool boot`: If `true`, the MCU will boot from the programmed data.  
     * `bool erase`: If `true`, erases the previous firmware before writing the new one.  
   * **Returns**: `int` \- `0` if successful, `-1` if an error occurs.  
   * **Usage**: This function is used to program and boot the device’s internal MCU, typically as part of a firmware update or custom MCU programming process.  
3. **`ProgramUpdate(bool download, bool force, ProgrammingCallback callback)`**  
   * **Description**: Manages firmware updates, including downloading and flashing new versions if available.  
   * **Arguments**:  
     * `bool download`: If `true`, downloads the latest firmware from a designated source.  
     * `bool force`: If `true`, forces an update even if the device has the latest version.  
     * `ProgrammingCallback callback`: Optional callback for tracking update progress.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function provides a streamlined way to ensure the device firmware is up to date. It can check for and download new versions, and the `force` option allows for reinstallation if needed.  
4. **`LMS_GPIOWrite(lms_device_t *device, const uint8_t *buffer, size_t len)`**  
   * **Description**: Writes specified values to the device’s GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint8_t *buffer`: Pointer to the data buffer containing GPIO values (each bit in the buffer corresponds to a GPIO pin).  
     * `size_t len`: Length of the data buffer in bytes.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: This function allows direct control of the GPIO pins on the device. Each bit in the `buffer` corresponds to the state (high or low) of a GPIO pin, enabling custom control of external components.  
5. **`LMS_GPIORead(lms_device_t *device, uint8_t *buffer, size_t len)`**  
   * **Description**: Reads the current state of the device’s GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *buffer`: Buffer where the GPIO states will be stored (each bit in the buffer represents a GPIO pin state).  
     * `size_t len`: Length of the buffer in bytes.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to read the current state of the GPIO pins, which can be useful for monitoring external signals or device status.  
6. **`LMS_GPIODirWrite(lms_device_t *device, const uint8_t *dir_buffer, size_t len)`**  
   * **Description**: Configures the direction of GPIO pins (input or output).  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint8_t *dir_buffer`: Buffer specifying the direction of each GPIO pin (each bit represents a pin, with `1` for output and `0` for input).  
     * `size_t len`: Length of the direction buffer in bytes.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: This function enables configuration of each GPIO pin as either an input or output. It’s often used in setup routines where pins are allocated specific roles.  
7. **`LMS_GPIODirRead(lms_device_t *device, uint8_t *dir_buffer, size_t len)`**  
   * **Description**: Reads the current direction configuration of the GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *dir_buffer`: Buffer where each bit represents the direction of a GPIO pin (`1` for output, `0` for input).  
     * `size_t len`: Length of the direction buffer in bytes.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: This function is used to confirm or inspect the GPIO pin directions, allowing verification that the pins are configured as intended before usage.

### **Device Enumeration and Hardware Management API**

1. **`LMS_GetDeviceList(lms_info_str_t *dev_list)`**  
   * **Description**: Enumerates all LimeSDR devices currently connected to the system, populating an array with information about each device.  
   * **Arguments**:  
     * `lms_info_str_t *dev_list`: An output array of `lms_info_str_t` structures, which will be populated with device information for each connected device. This struct includes fields such as the device name and serial number.  
   * **Returns**: `int` \- The number of connected devices on success, or `-1` if an error occurs.  
   * **Usage**: This function is used to detect all LimeSDR devices connected to the system, allowing the application to choose a specific device based on criteria like the serial number or model name.  
2. **`LMS_Open(lms_device_t **device, const lms_info_str_t info = nullptr, void* args = nullptr)`**  
   * **Description**: Opens a connection to a LimeSDR device, providing a device handle for further operations.  
   * **Arguments**:  
     * `lms_device_t **device`: Pointer to a device handle that will be created and set if the device opens successfully.  
     * `const lms_info_str_t info`: Information string for selecting a specific device (optional). If set to `nullptr`, the function will open the first available device.  
     * `void* args`: Optional pointer for additional configuration parameters, usually left as `nullptr`.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: This function must be called to establish communication with a LimeSDR device before performing any other operations. It provides the application with a handle to the device, which is required for subsequent function calls.  
3. **`LMS_Close(lms_device_t *device)`**  
   * **Description**: Closes an open LimeSDR device, releasing any resources associated with the device handle.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle to be closed.  
   * **Usage**: This function should be called when the device is no longer needed to properly release resources. Failure to close the device may result in resource leaks or issues when trying to open the device again.  
4. **`SetConnection(IConnection *port, size_t devIndex = 0)`**  
   * **Description**: Establishes a connection to a LimeSDR device through a specified communication port, allowing for multiple device management.  
   * **Arguments**:  
     * `IConnection *port`: Pointer to an `IConnection` object representing the communication port for the device (e.g., USB or network).  
     * `size_t devIndex`: Optional index to specify the device number, especially useful when multiple devices are connected. Default is `0` (first device).  
   * **Returns**: `bool` \- `true` if the connection is successfully established; `false` otherwise.  
   * **Usage**: This function is useful when working in environments with multiple LimeSDR devices connected, enabling the user to specify a particular device for operations.  
5. **`LMS_Reset(lms_device_t *device)`**  
   * **Description**: Resets the LimeSDR device, reinitializing it to its default state.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle for the device to be reset.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: This function is helpful for troubleshooting or reinitializing the device without disconnecting and reconnecting it. It clears any custom configurations and returns the device to a known baseline state.  
6. **`GetDeviceName(eLMS_DEV device)`**  
   * **Description**: Returns the name of a specific LimeSDR device type.  
   * **Arguments**:  
     * `eLMS_DEV device`: Enum specifying the LimeSDR device type (e.g., `LMS_DEV_LIMESDR`, `LMS_DEV_LIMESDRMINI`).  
   * **Returns**: `std::string` \- A string representing the name of the specified device type.  
   * **Usage**: This function is useful when supporting multiple LimeSDR device models, allowing the application to retrieve the name associated with a particular device type for display or logging purposes.  
7. **`EnableCache(bool enable)`**  
   * **Description**: Enables or disables caching of register values to minimize redundant reads.  
   * **Arguments**:  
     * `bool enable`: Set to `true` to enable caching or `false` to disable it.  
   * **Usage**: Enabling caching is beneficial for applications with frequent register accesses, as it improves performance by reducing I/O. However, caching should be disabled in situations requiring real-time register data, as cached values may not reflect immediate changes.  
8. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables register caching for a specific device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle for the target LimeSDR device.  
     * `bool enable`: Set to `true` to enable caching, or `false` to disable it.  
   * **Usage**: Provides control over register caching on a per-device basis, which can be advantageous in applications managing multiple LimeSDR devices. Enabling cache for a specific device optimizes repeated access patterns but can be disabled for applications that require real-time register updates.

### **Specialized Signal and Calibration API**

1. **`LMS_SetTestSignal(lms_device_t *device, bool dir_tx, size_t chan, lms_testsig_t sig, int16_t dc_i, int16_t dc_q)`**  
   * **Description**: Configures a test signal for debugging and calibration, allowing for specific signal characteristics like DC, NCO-based, or tone generation.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Set to `true` to apply the test signal to a transmit (TX) channel, or `false` for a receive (RX) channel.  
     * `size_t chan`: Channel index, typically `0` or `1`.  
     * `lms_testsig_t sig`: Type of test signal to configure. Options include:  
       * `LMS_TESTSIG_NONE` – No test signal.  
       * `LMS_TESTSIG_DC` – DC offset signal.  
       * `LMS_TESTSIG_NCODIV4` – NCO signal with frequency division by 4\.  
       * `LMS_TESTSIG_NCODIV8` – NCO signal with frequency division by 8\.  
       * `LMS_TESTSIG_TONE` – Single tone generation.  
     * `int16_t dc_i`: DC offset for the I (in-phase) component of the signal, used if `sig` is `LMS_TESTSIG_DC`.  
     * `int16_t dc_q`: DC offset for the Q (quadrature) component, also used if `sig` is `LMS_TESTSIG_DC`.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: This function is used during testing and calibration to introduce known signal characteristics, which can be useful for verifying signal path performance or isolating issues. For example, setting a DC signal allows you to check for DC offset correction.  
2. **`LMS_Calibrate(lms_device_t *device, bool dir_tx, size_t chan, double bw, unsigned flags)`**  
   * **Description**: Performs automatic calibration of the specified RX or TX channel to optimize performance within the specified bandwidth.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` to calibrate a transmit (TX) channel, `false` for receive (RX).  
     * `size_t chan`: Channel index.  
     * `double bw`: Bandwidth for calibration in Hz (e.g., 10e6 for 10 MHz).  
     * `unsigned flags`: Flags for calibration options, typically `0` for default behavior.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: This function calibrates the RX or TX channel to improve the linearity and signal quality within the specified bandwidth. Calibration may include adjustments to gain, phase, and other parameters to optimize RF performance.  
3. **`SetDCOffset(const int direction, const size_t channel, const std::complex<double> &offset)`**  
   * **Description**: Manually sets the DC offset for a specified RX or TX channel.  
   * **Arguments**:  
     * `int direction`: Direction of the channel, `0` for RX and `1` for TX.  
     * `size_t channel`: Channel index.  
     * `std::complex<double> &offset`: Complex offset value, with real and imaginary parts representing the I and Q DC offsets, respectively.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Manually setting the DC offset is useful for custom calibration scenarios where specific offset values are required, especially in controlled testing environments.  
4. **`LMS_SetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type freq)`**  
   * **Description**: Sets the Local Oscillator (LO) frequency for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX LO frequency, `false` for RX LO frequency.  
     * `size_t chan`: Channel index.  
     * `float_type freq`: Desired LO frequency in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjusting the LO frequency is essential for tuning the center frequency of the signal. This function is commonly used in applications requiring dynamic frequency adjustment or frequency hopping.  
5. **`CalibrateRx(lms_device_t *device, double bandwidth, size_t channel)`**  
   * **Description**: Calibrates the RX path on the specified channel to achieve optimal performance over the given bandwidth.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `double bandwidth`: Bandwidth for calibration in Hz.  
     * `size_t channel`: Channel index (e.g., 0 or 1).  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: RX path calibration improves the reception quality by aligning the gain and other parameters to the desired bandwidth. This is particularly useful in wideband applications where bandwidth-specific tuning is critical.  
6. **`CalibrateTx(lms_device_t *device, double bandwidth, size_t channel)`**  
   * **Description**: Calibrates the TX path on the specified channel to achieve optimal signal transmission over the specified bandwidth.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `double bandwidth`: Target bandwidth in Hz.  
     * `size_t channel`: Channel index (typically 0 or 1).  
   * **Returns**: `int` \- `0` if calibration is successful, `-1` on error.  
   * **Usage**: TX path calibration enhances transmission performance by adjusting the gain and other TX-specific parameters based on the desired bandwidth. This is especially important in applications demanding high signal fidelity within a certain frequency range.  
7. **`TuneTxFilter(lms_device_t *device, double bandwidth, bool cache_values)`**  
   * **Description**: Tunes the TX filter to the desired bandwidth, optionally using cached values for faster setup.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `double bandwidth`: Bandwidth in Hz for filter tuning.  
     * `bool cache_values`: If `true`, uses previously cached values for faster tuning.  
   * **Returns**: `int` \- `0` if successful, `-1` on failure.  
   * **Usage**: Tuning the TX filter allows for efficient bandwidth optimization, and the option to use cached values accelerates the process when the device is retuned frequently to the same bandwidth.  
8. **`TuneRxFilter(lms_device_t *device, double bandwidth, bool cache_values)`**  
   * **Description**: Tunes the RX filter to the specified bandwidth, with an option to use cached values.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `double bandwidth`: Target filter bandwidth in Hz.  
     * `bool cache_values`: If `true`, enables faster tuning with cached values.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Similar to `TuneTxFilter`, this function optimizes the RX filter to match the desired bandwidth, enhancing reception quality for specific frequency ranges.  
9. **`LMS_GetChipTemperature(lms_device_t *device, size_t chan)`**  
   * **Description**: Reads the internal temperature of the LMS7002M chip for the specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `size_t chan`: Channel index (e.g., 0 or 1).  
   * **Returns**: `float` \- The temperature in degrees Celsius.  
   * **Usage**: Monitoring chip temperature is important for thermal management and performance optimization, especially in high-power or extended operation scenarios.

### **Configuration and Utility API**

1. **`SaveConfig(const char *filename)`**  
   * **Description**: Saves the current device configuration to a specified file, allowing the configuration to be restored later.  
   * **Arguments**:  
     * `const char *filename`: Path to the file where the configuration will be saved.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: This function is useful for saving custom configurations that may need to be reloaded across sessions. For instance, a user might save specific tuning, calibration, or filter settings and reload them as needed.  
2. **`LoadConfig(const char *filename)`**  
   * **Description**: Loads a previously saved device configuration from a file.  
   * **Arguments**:  
     * `const char *filename`: Path to the file containing the saved configuration.  
   * **Returns**: `int` \- `0` if successful, `-1` if the configuration load fails.  
   * **Usage**: Used to restore the device state to a known configuration, which can be helpful for quick setup in scenarios where multiple devices need to be consistently configured.  
3. **`LMS7002M::SoftReset()`**  
   * **Description**: Performs a software reset of the LMS7002M chip, reinitializing its state without fully powering down or disconnecting the device.  
   * **Arguments**: None.  
   * **Returns**: `int` \- `0` on success, `-1` if the reset fails.  
   * **Usage**: Useful for resetting the chip to a known state without a complete hardware reset. This function is often used to clear any temporary settings or errors that may have occurred during testing or streaming.  
4. **`listAntennas(int direction, size_t channel) const`**  
   * **Description**: Lists all available antennas for a specified RX or TX channel.  
   * **Arguments**:  
     * `int direction`: `0` for RX, `1` for TX.  
     * `size_t channel`: Channel index (e.g., 0 or 1).  
   * **Returns**: `std::vector<std::string>` \- A vector of strings, each representing a possible antenna option for the specified channel and direction.  
   * **Usage**: Antenna selection may be necessary in applications that need to change reception or transmission characteristics, allowing the user to programmatically choose the best antenna for a given frequency or environmental condition.  
5. **`LMS_GetClockLockedStatus(lms_device_t *device, size_t chan)`**  
   * **Description**: Checks if the LO (Local Oscillator) or CGEN (Clock Generator) clock is locked for the specified channel, confirming that the frequency settings are stable.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `size_t chan`: Channel index (e.g., 0 or 1).  
   * **Returns**: `bool` \- `true` if the clock is locked; `false` if not.  
   * **Usage**: Clock locking is critical in applications requiring high-frequency stability. This function helps verify that the LO or CGEN is properly tuned and stable before streaming or testing.  
6. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables register caching specifically for the provided device, allowing for reduced I/O operations during repeated register accesses.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool enable`: Set to `true` to enable caching or `false` to disable it.  
   * **Returns**: `int` \- `0` if successful, `-1` on failure.  
   * **Usage**: Caching improves performance by reducing the need for redundant register reads. It should be enabled in scenarios where register values do not need to be immediately updated, and disabled when real-time data is critical.  
7. **`LMS_Reset(lms_device_t *device)`**  
   * **Description**: Resets the entire LimeSDR device, returning it to its default state.  
   * **Arguments**:  
     * `lms_device_t *device`: The handle of the device to reset.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: A complete hardware reset is useful when a device needs to be cleared of all configurations, as it reinitializes the LimeSDR to its default setup. This function is typically used to recover from errors or prepare the device for a new configuration.

### **Sensor and Status Functions API**

1. **`LMS_GetChipTemperature(lms_device_t *device, size_t chan)`**  
   * **Description**: Retrieves the internal temperature of the LMS7002M chip for a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `size_t chan`: Channel index (e.g., 0 or 1).  
   * **Returns**: `float` \- The temperature of the chip in degrees Celsius.  
   * **Usage**: This function is essential for thermal management, allowing applications to monitor device temperature and prevent overheating. It is particularly useful in scenarios with continuous high-power operation.  
2. **`LMS_GetClockLockedStatus(lms_device_t *device, size_t chan)`**  
   * **Description**: Checks the lock status of the Local Oscillator (LO) or Clock Generator (CGEN) for a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `size_t chan`: Channel index (e.g., 0 or 1).  
   * **Returns**: `bool` \- `true` if the clock is locked, indicating stable frequency; `false` if not locked.  
   * **Usage**: Useful in frequency-sensitive applications, this function confirms that the clock is stable and locked at the desired frequency before streaming or testing.  
3. **`LMS_GetStreamStatus(lms_stream_t *stream, lms_stream_status_t *status)`**  
   * **Description**: Retrieves the current status of an active data stream, including information about the FIFO buffer, data rates, and any underflow or overflow conditions.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Handle to the data stream.  
     * `lms_stream_status_t *status`: Pointer to a structure where the stream’s status is stored.  
   * **Structure Fields**:  
     * `fifoFilledCount`: Number of elements currently in the FIFO buffer.  
     * `fifoSize`: Total size of the FIFO buffer.  
     * `underrun`: Counter for RX buffer underrun events.  
     * `overrun`: Counter for TX buffer overrun events.  
     * `droppedPackets`: Count of packets dropped.  
     * `sampleRate`: Current sample rate in samples per second.  
   * **Returns**: `int` \- `0` if successful, `-1` if an error occurs.  
   * **Usage**: This function provides insights into the health and performance of the stream, useful for real-time monitoring, particularly in applications that require uninterrupted data flow.  
4. **`LMS_GetGPIOStatus(lms_device_t *device, uint8_t *buffer, size_t len)`**  
   * **Description**: Reads the status of the GPIO pins on the device, storing the result in the provided buffer.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *buffer`: Buffer where the GPIO states will be stored (each bit represents a GPIO pin state).  
     * `size_t len`: Length of the buffer in bytes.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: This function is used to monitor the current state of GPIO pins, which may indicate device status or be used to interface with external components.  
5. **`listSensors(int direction, size_t channel)`**  
   * **Description**: Lists the available sensors for a specified RX or TX channel, providing identifiers that can be used to query individual sensor values.  
   * **Arguments**:  
     * `int direction`: `0` for RX, `1` for TX.  
     * `size_t channel`: Channel index (e.g., 0 or 1).  
   * **Returns**: `std::vector<std::string>` \- A vector containing names of available sensors for the specified channel and direction.  
   * **Usage**: This function helps identify the sensors available for a channel, allowing for more granular monitoring and diagnostic options in applications that require specific environmental or operational metrics.  
6. **`getSensorInfo(const std::string &name)`**  
   * **Description**: Provides detailed information about a specified sensor, including its range, resolution, and units.  
   * **Arguments**:  
     * `const std::string &name`: Name of the sensor (obtained from `listSensors`).  
   * **Returns**: `SoapySDR::ArgInfo` \- A structure with fields such as `name`, `description`, `units`, `range`, and `resolution`.  
   * **Usage**: Use this function to retrieve metadata for a sensor, such as its operational limits or unit of measurement, which can be useful for interpreting sensor data correctly.  
7. **`readSensor(const std::string &name)`**  
   * **Description**: Reads the current value of a specified sensor.  
   * **Arguments**:  
     * `const std::string &name`: Name of the sensor to read (obtained from `listSensors`).  
   * **Returns**: `std::string` \- The sensor’s current value, returned as a string to accommodate various data types.  
   * **Usage**: This function is commonly used in real-time monitoring applications to obtain sensor values, such as LO lock status, chip temperature, or environmental conditions.

### **Register Access API**

1. **`LMS_WriteFPGAReg(lms_device_t *device, uint32_t address, uint32_t value)`**  
   * **Description**: Writes a 32-bit value to a specified FPGA register on the LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `uint32_t address`: Address of the FPGA register.  
     * `uint32_t value`: The 32-bit value to write to the register.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function provides direct access to the FPGA’s registers, allowing for fine-tuning of device parameters. It is typically used in low-level hardware testing, debugging, or custom modifications.  
2. **`LMS_ReadFPGAReg(lms_device_t *device, uint32_t address, uint32_t *value)`**  
   * **Description**: Reads a 32-bit value from a specified FPGA register.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint32_t address`: Address of the FPGA register.  
     * `uint32_t *value`: Pointer to store the 32-bit value read from the register.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function is used to retrieve values from FPGA registers, enabling validation of register settings or state checks for debugging purposes.  
3. **`LMS_WriteParam(lms_device_t *device, const LMS7Parameter param, int value)`**  
   * **Description**: Writes a specified value to a defined parameter on the LMS7002M chip.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const LMS7Parameter param`: A predefined parameter identifier for the LMS7002M chip.  
     * `int value`: Value to write to the parameter.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: This function is useful for configuring LMS7002M chip settings, especially when dealing with specific parameters that control RF performance, gain, or filter settings.  
4. **`LMS_ReadParam(lms_device_t *device, const LMS7Parameter param)`**  
   * **Description**: Reads the current value of a specified parameter on the LMS7002M chip.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const LMS7Parameter param`: A predefined parameter identifier for the LMS7002M chip.  
   * **Returns**: `int` \- The value of the specified parameter, or `-1` on error.  
   * **Usage**: Use this function to retrieve the current value of a parameter, which is useful for verifying configuration settings or for monitoring adjustments to LMS7002M parameters during operation.  
5. **`WriteRegisters(lms_device_t *device, const uint32_t *addresses, const uint32_t *values, size_t count)`**  
   * **Description**: Writes multiple values to a set of FPGA registers in a batch operation.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `const uint32_t *addresses`: Array of register addresses to write to.  
     * `const uint32_t *values`: Array of values corresponding to each register address.  
     * `size_t count`: Number of registers to write.  
   * **Returns**: `int` \- `0` if successful, `-1` on failure.  
   * **Usage**: This function is efficient for applying multiple register settings at once, reducing the overhead of individual register writes. It is commonly used in initialization routines or when configuring multiple parameters in bulk.  
6. **`ReadRegisters(lms_device_t *device, const uint32_t *addresses, uint32_t *values, size_t count)`**  
   * **Description**: Reads values from a set of FPGA registers in a batch operation.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint32_t *addresses`: Array of register addresses to read from.  
     * `uint32_t *values`: Array to store the values read from each register.  
     * `size_t count`: Number of registers to read.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: Useful for retrieving multiple register values in a single call, this function reduces the overhead associated with repeated read calls, which is beneficial in performance-critical applications.  
7. **`CustomParameterWrite(const std::vector<uint32_t> &ids, const std::vector<uint32_t> &values, const std::string &units = "")`**  
   * **Description**: Writes values to custom parameters on the device, specified by an array of IDs.  
   * **Arguments**:  
     * `const std::vector<uint32_t> &ids`: A vector of custom parameter IDs.  
     * `const std::vector<uint32_t> &values`: A vector of values to write to each parameter ID.  
     * `const std::string &units`: Optional string specifying the units (e.g., V, A) associated with the parameter values.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function allows for writing custom configuration parameters that are not covered by standard register settings, enabling advanced users to apply specialized settings for experimental purposes or specific tuning.

### **High-Level Control Functions API**

1. **`LMS_SetSampleRate(lms_device_t *device, float_type rate, size_t oversample)`**  
   * **Description**: Sets the sample rate for the device, applying an optional oversampling factor.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `float_type rate`: Desired sample rate in samples per second.  
     * `size_t oversample`: Optional oversampling factor (e.g., 1, 2, 4).  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Adjusting the sample rate is fundamental for applications with different bandwidth requirements. Oversampling can improve the signal quality, but also increases processing demands.  
2. **`LMS_SetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type freq)`**  
   * **Description**: Configures the Local Oscillator (LO) frequency for a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` to set the TX LO frequency, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `float_type freq`: Desired frequency in Hz.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: Setting the LO frequency is essential for tuning the center frequency of transmitted or received signals, especially in frequency-agile applications.  
3. **`LMS_SetGaindB(lms_device_t *device, bool dir_tx, size_t chan, unsigned gain)`**  
   * **Description**: Sets the gain level for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX gain, `false` for RX gain.  
     * `size_t chan`: Channel index.  
     * `unsigned gain`: Desired gain in dB.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: Gain adjustment is critical in controlling signal strength and quality. Higher gain can improve signal reception, but may also increase noise, so careful tuning is often required.  
4. **`LMS_SetLPFBW(lms_device_t *device, bool dir_tx, size_t chan, float_type bandwidth)`**  
   * **Description**: Sets the bandwidth of the low-pass filter (LPF) for a given RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX filter, `false` for RX filter.  
     * `size_t chan`: Channel index.  
     * `float_type bandwidth`: Desired LPF bandwidth in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Adjusting the LPF bandwidth allows control over the signal bandwidth, which can help in reducing out-of-band noise while optimizing the desired frequency range.  
5. **`LMS_EnableChannel(lms_device_t *device, bool dir_tx, size_t chan, bool enable)`**  
   * **Description**: Enables or disables a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `bool enable`: `true` to enable, `false` to disable.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Used to selectively enable or disable channels based on the application’s requirements, which can save power and reduce processing overhead for unused channels.  
6. **`LMS_SetAntenna(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t antenna)`**  
   * **Description**: Selects an antenna for a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_name_t antenna`: Antenna name or index (obtained from `listAntennas`).  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Antenna selection can be optimized for different frequency ranges or environmental conditions, allowing the device to switch to the most appropriate antenna as needed.

### **Communicaiton Routines** 

1. **`IConnection::DeviceReset(int ind)`**  
   * **Description**: Attempts to reset the connected device at the specified index.  
   * **Arguments**:  
     * `int ind`: Index of the device to reset.  
   * **Returns**: `int` \- `0` on success, error code if unsupported.  
   * **Usage**: Useful for reinitializing a device in a multi-device setup when connectivity or stability issues ariseonnection::ReceiveData(char\* buffer, int length, int epIndex, int timeout)\`\*\*  
   * **Description**: Receives data from the specified endpoint.  
   * **Arguments**:  
     * `char* buffer`: Buffer to store incoming data.  
     * `int length`: Maximum number of bytes to read.  
     * `int epIndex`: Index of the endpoint.  
     * `int timeout`: Timeout for receiving data.  
   * **Returns**: `int` \- Number of bytes received, `-1` on error.  
   * **Usage**: Useful for custom data transfer applications requiring low-level endpoint access .  
2. **`ISendData(const char* buffer, int length, int epIndex, int timeout)`**  
   * **Description**: Sends data to a specified endpoint.  
   * **Arguments**:  
     * `const char* buffer`: Buffer containing the data to send.  
     * `int length`: Number of bytes to send.  
     * `int epIndex`: Index of the endpoint.  
     * `int timeout`: Timeout for sending data.  
   * **Returns**: `int` \- Number of bytes sent, `-1` on error.  
   * **Usage**: Allows direct communication with hardware endpoints, useful in high-performance streaming applications .  
3. **`IConnection:nding(const char* buffer, uint32_t length, int ep)`**  
   * **Description**: Begins asynchronous data transmission on a specified endpoint.  
   * **Arguments**:  
     * `const char* buffer`: Data buffer to transmit.  
     * `uint32_t length`: Number of bytes to send.  
     * `int ep`: Endpoint index for the transfer.  
   * **Returns**: `int` \- Context handle for tracking the send operation.  
   * **Usage**: Enables non-blocking data transmission, improving performance in latency-sensitive applications .  
4. **`IConnection::WaitForSentextHandle, uint32_t timeout_ms)`**  
   * **Description**: Waits for an asynchronous send operation to complete.  
   * **Arguments**:  
     * `int contextHandle`: Handle returned by `BeginDataSending`.  
     * `uint32_t timeout_ms`: Timeout for the operation.  
   * **Returns**: `bool` \- `true` if successful, `false` on timeout or error.  
   * **Usage**: Ensures completion of non-blocking data transfers .  
5. **`IConnection::FinishDataSending(confer, uint32_t length, int contextHandle)`**  
   * **Description**: Completes an asynchronous data send operation.  
   * **Arguments**:  
     * `const char* buffer`: Data buffer to send.  
     * `uint32_t length`: Number of bytes sent.  
     * `int contextHandle`: Context handle for the send operation.  
   * **Usage**: Completes a previously initiated async transfer, critical for high-throughput applications .  
6. **`IConnection::BeginDataReading(char* buffer, uth, int ep)`**  
   * **Description**: Begins asynchronous data reception from an endpoint.  
   * **Arguments**:  
     * `char* buffer`: Buffer to store incoming data.  
     * `uint32_t length`: Length of the buffer.  
     * `int ep`: Endpoint index for data reception.  
   * **Usage**: Useful for continuous data acquisition applications requiring asynchronous data handling .  
7. **`IConnection::WaitForReading(int contextHandle, unsigned ms)`**  
   * **Description**: Waits for an asynchronous read operation to complete.  
   * **Arguments**:  
     * `int contextHandle`: Handle from `BeginDataReading`.  
     * `unsigned int timeout_ms`: Timeout in milliseconds.  
   * **Returns**: `bool` \- `true` if successful, `false` otherwise.  
   * **Usage**: Useful for non-blocking data reads, improving responsiveness in real-time applications .  
8. **`IConnection::FinishDataReading(char* buffer, uint32_t length, int c)`**  
   * **Description**: Completes an asynchronous data read operation.  
   * **Arguments**:  
     * `char* buffer`: Buffer to store the received data.  
     * `uint32_t length`: Number of bytes to read.  
     * `int contextHandle`: Context handle for the read operation.  
   * **Usage**: Finalizes non-blocking data reading, commonly used in high-speed streaming or low-latency applications .  
9. **`IConnection::SetDataLogCallback(std::function<void(bool, const unsigned char*gned int)> callback)`**  
   * **Description**: Sets a callback function for logging data send/receive events.  
   * **Arguments**:  
     * `std::function<void(bool, const unsigned char*, const unsigned int)> callback`: A function for logging, with parameters indicating direction (`true` for TX, `false` for RX), data pointer, and data size.  
   * **Usage**: Allows applications to monitor data flow for debugging or logging purposes, providing real-time insights into data exchange events .

### 

#### **Clock and Frequency Control**

1. **`LMS_GetLPFBWRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Retrieves the allowed range of low-pass filter (LPF) bandwidths for a specified RX or TX path.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `lms_range_t *range`: Pointer to an `lms_range_t` structure to store the min, max, and step values.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: This function is critical for applications needing to adjust the LPF within the supported bandwidth range, ensuring optimal filtering for specific frequency ranges.  
2. **`LMS_VCTCXOWrite(lms_device_t *device, uint16_t val)`**  
   * **Description**: Writes a trim value to the VCTCXO DAC for fine-tuning the reference clock frequency.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t val`: DAC value to set.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Used during clock calibration to stabilize or adjust the reference oscillator. Ensures high precision in frequency-dependent applications.  
3. **`LMS_VCTCXORead(lms_device_t *device, uint16_t *val)`**  
   * **Description**: Reads the VCTCXO DAC trim value from non-volatile storage.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t *val`: Pointer to store the current DAC value.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for checking and verifying the reference oscillator calibration, especially after power cycling or firmware updates.  
4. **`LMS_SetNCOFrequency(lms_device_t *device, bool dir_tx, size_t chan, const float_type *freq, float_type pho)`**  
   * **Description**: Configures the NCO (Numerically Controlled Oscillator) with a set of predefined frequencies and a phase offset, allowing rapid switching between up to 16 frequencies.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `const float_type *freq`: Array of NCO frequencies (must contain at least 16 values).  
     * `float_type pho`: Phase offset in degrees.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Essential for applications requiring frequency hopping or multi-tone transmission, as it enables rapid frequency changes without reconfiguration.

#### **Logging and Error Handling**

1. **`LMS_GetLibraryVersion(void)`**  
   * **Description**: Returns the version of the LimeSuite API library currently in use.  
   * **Returns**: `const char*` \- A string indicating the version.  
   * **Usage**: Important for compatibility checks, especially when developing software that may need specific library versions.  
2. **`LMS_GetLastErrorMessage(void)`**  
   * **Description**: Retrieves the last error message, providing details on why the most recent API function call failed.  
   * **Returns**: `const char*` \- String containing the last error message.  
   * **Usage**: Useful in debugging, particularly in cases where more context is required to understand and address errors.  
3. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
   * **Description**: Registers a custom log handler, redirecting API log messages (errors, warnings, info) to a user-defined function.  
   * **Arguments**:  
     * `LMS_LogHandler handler`: Pointer to a logging function that will handle messages based on log level.  
   * **Usage**: Useful for integrating LimeSDR logging with external logging systems, enabling custom message handling and filtering.

#### **Synchronization**

1. **`LMS_Synchronize(lms_device_t *device, bool toChip)`**  
   * **Description**: Synchronizes register values between the API cache and the actual LimeSDR chip.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool toChip`: If `true`, copies values from the API cache to the chip; if `false`, the cache is updated from the chip.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Important for ensuring configuration consistency, particularly in scenarios where external changes may affect register values.

#### **Clock and Frequency Configuration**

1. **`LMS_GetSampleRateRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Provides the supported range of sample rates for a specified RX or TX path.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `lms_range_t *range`: Pointer to a structure that stores the minimum, maximum, and step values of the sample rate.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Useful for determining valid sample rates prior to configuration, ensuring that settings fall within the device’s operational limits\_GetNumChannels(lms\_device\_t *device, bool dir\_tx)\`*\*  
   * **Description**: Retrieves the number of available channels for either RX or TX.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX channels, `false` for RX channels.  
   * **Returns**: `int` \- Number of channels on success, `-1` on failure.  
   * **Usage**: Used to check the available channel count, which is essential in multi-channel applications to dynamically adjust the configuration .

#### **MCUoller Unit) Control**

1. **`MCU_BD::Read_IRAM()`**  
   * **Description**: Reads the contents of the internal RAM (IRAM) in the microcontroller.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Reading IRAM can be useful in debugging or retrieving specific MCU states for more detailed diagnostics in complex configurations .  
2. \*\*\`MCU\_BD::Erase- **Description**: Clears the IRAM in the microcontroller, setting all values to zero.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function is useful for ensuring a clean slate on the MCU, which can be helpful in memory-intensive applications or when troubleshooting memory corruption .  
3. **`MCU_BD::Read_SFR()`** \-\*\*: Reads special function registers (SFR) in the microcontroller.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Access to SFR values is crucial for low-level debugging and tuning specific MCU operations within the LimeSDR .

#### **Board-Specific Configuration and Loggingit(lms\_device\_t *device)\`*\***

* **Description**: Initializes the device with settings to prepare it for operation.  
* **Arguments**:  
  * `lms_device_t *device`: Device handle.  
* **Returns**: `int` \- `0` on success, `-1` on error.  
* **Usage**: This function is fundamental for initializing the LimeSDR to its default operational state, a prerequisite for reliable data transmission and reception 【14:14†source】.  
2. **`LMS_ReadCustomBoardParam(lms_device_t :contentReference[oaicite:0]{index=0} float_type *val, lms_name_t units)`**  
   * **Description**: Reads a custom board parameter by its identifier.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t param_id`: ID of the parameter to read.  
     * `float_type *val`: Pointer to store the value read.  
     * `lms_name_t units`: Units associated with the parameter.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: This function allows reading unique board parameters that may be specific to custom configurations or experimental setups【14:14†source】.  
3. **`LMS_WriteCustomBoardParam(lms_device_t *device, ui&#8203;:contentReference[oaicite:1]{index=1}, float_type val, const lms_name_t units)`**  
   * **Description**: Writes a value to a specified custom board parameter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t param_id`: ID of the parameter to set.  
     * `float_type val`: Value to write.  
     * `const lms_name_t units`: Units associated with the parameter.  
   * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
   * **Usage**: Useful in custom applications needing control over experimental or board-specific parameters for configuration beyond standard API limits【14:14†source】.

### **Additional Functions**

1. **`LMS_GetLOFrequencyRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Retrieves the permissible range of LO frequencies for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `lms_range_t *range`: Structure to store the minimum, maximum, and step values for the LO frequency.  
   * **Usage**: This function is crucial for applications requiring dynamic tuning within the hardware's frequency capabilities.  
2. **`LMS_GetAntennaList(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t *list)`**  
   * **Description**: Returns a list of available antennas for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_name_t *list`: Buffer to store antenna names.  
   * **Returns**: Number of antennas available, or `-1` on error.  
   * **Usage**: Useful for antenna selection when switching between frequency bands or different signal conditions.  
3. **`LMS_SetSampleRateDir(lms_device_t *device, bool dir_tx, float_type rate, size_t oversample)`**  
   * **Description**: Sets the sample rate with direction-specific control, allowing different sample rates for RX and TX.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `float_type rate`: Desired sample rate in Hz.  
     * `size_t oversample`: Oversampling ratio.  
   * **Usage**: Allows distinct sample rate configurations for RX and TX, supporting applications that benefit from varying rates for input and outputS\_SetNCOFrequency(lms\_device\_t *device, bool dir\_tx, size\_t chan, const float\_type freq, float\_type pho)\`*  
   * **Description**: Configures the NCO for frequency hopping with up to 16 pre-set frequencies.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `const float_type *freq`: Array of frequencies.  
     * `float_type pho`: Phase offset in degrees.  
   * **Usage**: Enables rapid switching between frequencies in applications requiring agile frequency hopping .  
4. **`Rt char* serialport, lms_device_t *dev)`**  
   * **Description**: Opens a LimeRFE device either directly via USB or through an SDR connection.  
   * **Arguments**:  
     * `const char* serialport`: Serial port name if controlled by USB.  
     * `lms_device_t *dev`: SDR device handle.  
   * **Usage**: Initializes the LimeRFE configuration, supporting multi-device setups when pairing LimeRFE with LimeSDR .  
5. **`RFE_GetInfo(fe, unsigned char* cinfo)`**  
   * **Description**: Retrieves firmware and hardware version information for LimeRFE.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: Handle to the RFE device.  
     * `unsigned char* cinfo`: Buffer to store version and status bytes.  
   * **Usage**: Useful for device verification and version control in applications with multi-generation LimeRFE devices .

### **Advanced Control and Configuration**

1. **`LMS_GetNCOIndex(lms_device_t *device, bool dir_tx, size_t chan)`**  
   * **Description**: Retrieves the current NCO (Numerically Controlled Oscillator) frequency/phase offset index for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
   * **Returns**: `int` \- Current NCO index on success, `-1` on failure.  
   * **Usage**: This is useful for applications that require tracking or modifying the active NCO index, especially in frequency-agile applications like frequency hoppingLMS\_SetGFIRCoeff(lms\_device\_t *device, bool dir\_tx, size\_t chan, lms\_gfir\_t filt, const float\_type coef, size\_t count)\`*  
   * **Description**: Configures the LMS GFIR (General Finite Impulse Response) filter with specified filter coefficients.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_gfir_t filt`: GFIR filter type (`GFIR1`, `GFIR2`, or `GFIR3`).  
     * `const float_type *coef`: Array of filter coefficients (range: \-1.0 to 1.0).  
     * `size_t count`: Number of coefficients.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Essential for custom filter design and configuration, supporting up to 40 coefficients for `GFIR1` and `GFIR2`, and 120 for `GFIR3` .  
2. \*\*IRCoeff(lms\_device\_t *device, bool dir\_tx, size\_t chan, lms\_gfir\_t filt, float\_type coef)\`*  
   * **Description**: Retrieves the current coefficients set for a specified GFIR filter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_gfir_t filt`: GFIR filter type.  
     * `float_type *coef`: Array to store retrieved coefficients.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Useful for reading and verifying filter configurations, allowing dynamic adjustments in real-time applications .  
3. **`LMS_SetGvice_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, bool enabled)`**  
   * **Description**: Enables or disables a specific GFIR filter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_gfir_t filt`: GFIR filter type.  
     * `bool enabled`: `true` to enable, `false` to disable.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Controls filter activation, essential for dynamic filtering scenarios and optimized signal processing pipelines .  
4. **`LMS_EnableCache(lt *device, bool enable)`**  
   * **Description**: Toggles caching of LMS7 and FPGA register values for the specified device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool enable`: `true` to enable caching, `false` to disable.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Helps improve performance by reducing redundant register reads, beneficial in applications requiring frequent access to register values .

### **Additional Functions for Frequency Configuration and Waveform Handling**

1. **`LMS_GetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *freq)`**  
   * **Description**: Retrieves the current Local Oscillator (LO) frequency for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `float_type *freq`: Pointer to store the frequency in Hz.  
   * **Usage**: This function is critical for verifying the LO frequency, particularly in applications requiring specific frequency settings​(alllimesdk).  
2. **`LMS_UploadWFM(lms_device_t *device, const void **samples, uint8_t chCount, size_t sample_count, int format)`**  
   * **Description**: Uploads a custom waveform to the onboard memory for playback.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const void **samples`: Array of sample buffers (one per channel).  
     * `uint8_t chCount`: Number of channels (1 or 2).  
     * `size_t sample_count`: Number of samples per channel (multiple of 4).  
     * `int format`: Format for the waveform (`0` for int16, `1` for int16, `2` for float).  
   * **Usage**: Enables custom waveform generation, suitable for applications requiring repeated signal patterns (alllimesdk).  
3. **`LMS_EnableTxWFM(lms_device_t *device, unsigned chan, bool active)`**  
   * **Description**: Enables or disables the transmission of the uploaded waveform on a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `unsigned chan`: Channel index.  
     * `bool active`: Set to `true` to enable, `false` to disable.  
   * **Usage**: Useful for controlled transmission testing or demonstration modes​(alllimesdk).

### **Additional LimeRFE-Specific Functions**

1. **`RFE_Configure(rfe_dev_t* rfe, int in_port, int out_port, int mode, int notch, int attenuation)`**  
   * **Description**: Configures the LimeRFE module with specific settings.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: Handle to the LimeRFE device.  
     * `int in_port`, `int out_port`: Input/output port settings.  
     * `int mode`: Operational mode (e.g., RX, TX).  
     * `int notch`: Notch filter setting.  
     * `int attenuation`: Attenuation level.  
   * **Usage**: Essential for applications using LimeRFE for front-end configuration, adjusting frequency ranges, filters, and transmission settings​(alllimesdk).  
2. **`RFE_Reset(rfe_dev_t* rfe)`**  
   * **Description**: Resets the LimeRFE device to its default settings.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE handle.  
   * **Usage**: Useful for clearing configurations and reinitializing LimeRFE​(alllimesdk).  
3. **`RFE_GetInfo(rfe_dev_t* rfe, unsigned char* cinfo)`**  
   * **Description**: Retrieves firmware and hardware version information for the LimeRFE device.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE handle.  
     * `unsigned char* cinfo`: Buffer to store firmware, hardware version, and status.  
   * **Usage**: Important for compatibility and version control in LimeRFE setups​(alllimesdk).

### **Additional Functions**

1. **`LMS_GetProgramModes(lms_device_t *device, lms_name_t *list)`**  
   * **Description**: Retrieves a list of available programming modes supported by the LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_name_t *list`: Buffer to store programming mode names.  
   * **Returns**: `int` \- Number of available modes, `-1` on failure.  
   * **Usage**: Useful for firmware and gateware updates, allowing dynamic selection of the appropriate programming mode​(alllimesdk).  
2. **`LMS_Program(lms_device_t *device, const char *data, size_t size, const lms_name_t mode, lms_prog_callback_t callback)`**  
   * **Description**: Programs the device using the specified mode, supporting firmware or bitstream updates.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const char *data`: Pointer to the firmware/bitstream data.  
     * `size_t size`: Size of the data.  
     * `const lms_name_t mode`: Programming mode, obtained from `LMS_GetProgramModes`.  
     * `lms_prog_callback_t callback`: Optional callback to monitor programming progress.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Enables firmware and gateware updates, critical for maintaining compatibility with newer LimeSuite versions​(alllimesdk).  
3. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
   * **Description**: Registers a custom log handler to capture and process LimeSDR log messages.  
   * **Arguments**:  
     * `LMS_LogHandler handler`: Custom function to handle log messages.  
   * **Usage**: Allows redirecting log messages to a custom handler for more advanced debugging and logging setups​(alllimesdk).  
4. **`LMS_GetLastErrorMessage(void)`**  
   * **Description**: Retrieves the most recent error message from the LimeSDR API, detailing the cause of the last failure.  
   * **Returns**: `const char*` \- String containing the last error message.  
   * **Usage**: Helpful for debugging by providing immediate feedback on API errors​(alllimesdk).  
5. **`LMS_UploadWFM(lms_device_t *device, const void **samples, uint8_t chCount, size_t sample_count, int format)`**  
   * **Description**: Uploads a waveform for repeated playback on specified channels.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const void **samples`: Array of sample buffers.  
     * `uint8_t chCount`: Number of channels (1 or 2).  
     * `size_t sample_count`: Sample count per channel.  
     * `int format`: Format of the waveform data.  
   * **Usage**: Ideal for testing transmission with custom waveforms in controlled environments​(alllimesdk).

### **Lesser but Useful Functions**

1. **`LMS_GetLibraryVersion(void)`**  
   * **Description**: Returns the version of the LimeSuite API library currently in use.  
   * **Returns**: `const char*` \- A string indicating the version.  
   * **Usage**: Important for compatibility checks, especially when developing software that requires specific library versions​(alllimesdk).  
2. **`LMS_GetLastErrorMessage(void)`**  
   * **Description**: Retrieves the last error message generated by the LimeSDR API, useful for understanding the cause of the last failure.  
   * **Returns**: `const char*` \- A string containing the last error message.  
   * **Usage**: Useful in debugging to get immediate feedback on why an error occurred, especially when troubleshooting SDK calls​(alllimesdk).  
3. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
   * **Description**: Registers a custom log handler to capture and process LimeSDR log messages.  
   * **Arguments**:  
     * `LMS_LogHandler handler`: Pointer to a custom function that processes log messages.  
   * **Usage**: Allows applications to redirect log messages to a custom handler, enabling more detailed logging and debugging​(alllimesdk).  
4. **`LMS_SaveConfig(lms_device_t *device, const char *filename)`**  
   * **Description**: Saves the current configuration of the LMS7002M chip to a file.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const char *filename`: Path to the file where configuration is saved.  
   * **Usage**: Facilitates saving and reloading configurations, allowing users to quickly switch between setups​(alllimesdk).  
5. **`LMS_LoadConfig(lms_device_t *device, const char *filename)`**  
   * **Description**: Loads a saved LMS7002M chip configuration from a file.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const char *filename`: Path to the configuration file.  
   * **Usage**: Useful for restoring specific configurations or testing multiple settings across sessions​(alllimesdk).  
6. **`LMS_SetTestSignal(lms_device_t *device, bool dir_tx, size_t chan, lms_testsig_t sig, int16_t dc_i, int16_t dc_q)`**  
   * **Description**: Configures a test signal for diagnostics.  
   * **Arguments**:  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_testsig_t sig`: Type of test signal to enable (e.g., DC, NCO).  
     * `int16_t dc_i`: DC offset I for DC test signals.  
     * `int16_t dc_q`: DC offset Q for DC test signals.  
   * **Usage**: Useful for signal integrity testing and diagnostics​(alllimesdk).

---

### **Additional Utility and Device-Specific Configuration Functions**

1. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables caching of register values for performance improvement.  
   * **Arguments**:  
     * `lms_device_t *device`: LimeSDR device handle.  
     * `bool enable`: `true` to enable caching, `false` to disable it.  
   * **Usage**: This function is especially useful when frequent register reads slow down performance. By caching values, repeated access becomes more efficient, though it should be disabled when real-time values are critical​(alllimesdk).  
2. **`LMS_GetDeviceInfo(lms_device_t *device, lms_info_str_t *info)`**  
   * **Description**: Retrieves detailed device information, including platform-specific details such as PCIe configurations.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_info_str_t *info`: Pointer to store device information in a structured format.  
   * **Usage**: This is particularly important when managing multiple devices or platforms, especially with PCIe-based LimeSDR configurations​(alllimesdk).  
3. **`ConnectionXillybus::deviceConfigs`**  
   * **Description**: Configuration set for Xillybus device connections used in PCIe platforms, detailing read/write paths and endpoint setups.  
   * **Details**:  
     * Supports multiple connection definitions for LimeSDR-PCIe and LimeSDR-QPCIe, with paths set depending on the OS and PCIe slots.  
   * **Usage**: Critical for applications that use LimeSDR in a PCIe configuration, as it manages different Xillybus setups depending on the LimeSDR model and OS environment​(alllimesdk).  
4. **`lms7002_mainPanel` functions**: GUI-based functions for handling device settings like calibration, MIMO configuration, and logging.  
   * **Functions include**:  
     * `OnReadTemperature`, `OnCalibrateInternalADC`, `OnSwitchToChannelA/B`, and others for handling channel switching, ADC calibration, and real-time temperature logging.  
   * **Usage**: Primarily for GUI applications but relevant in environments requiring interactive or automated device management​(alllimesdk).  
5. **`GenerateWindowCoefficients(int func, int N, std::vector<float> &windowFcoefs, float amplitudeCorrection)`**  
   * **Description**: Generates window coefficients for signal processing, supporting types like Blackman-Harris, Hamming, and Hanning.  
   * **Arguments**:  
     * `int func`: Function type (e.g., Blackman-Harris, Hamming).  
     * `int N`: Number of coefficients.  
     * `std::vector<float> &windowFcoefs`: Coefficients vector to populate.  
     * `float amplitudeCorrection`: Correction factor for amplitude.  
   * **Usage**: Supports DSP applications where signal windowing is necessary, enhancing filtering and waveform shaping for precision signal analysis​(alllimesdk).

### **Clock Functions**

1. **`getMasterClockRate()`**  
   * **Description**: Retrieves the current master clock rate of the device.  
   * **Returns**: `double` \- Master clock rate in Hz.  
   * **Usage**: Useful for verifying clock rate settings and ensuring they align with the sample rate requirements for both RX and TX operations​(alllimesdk).  
2. **`listClockSources()`**  
   * **Description**: Lists available clock sources for the LimeSDR.  
   * **Returns**: `std::vector<std::string>` \- Available clock sources, such as “internal” or “external.”  
   * **Usage**: Important when selecting clock sources for applications requiring precise timing control​(alllimesdk).  
3. **`setClockSource(const std::string &source)`**  
   * **Description**: Sets the clock source for the LimeSDR device.  
   * **Arguments**:  
     * `source`: Name of the clock source to set (e.g., "internal" or "external").  
   * **Usage**: Used for switching between internal and external clocks, necessary in applications that sync to external timing sources​(alllimesdk).  
4. **`getHardwareTime(const std::string &what = "")`**  
   * **Description**: Retrieves the current hardware time in nanoseconds.  
   * **Returns**: `long long` \- The current hardware time in nanoseconds.  
   * **Usage**: Useful for time-synchronized operations and applications that require precise timing or timestamping of data​(alllimesdk).  
5. **`setHardwareTime(const long long timeNs, const std::string &what = "")`**  
   * **Description**: Sets the hardware time on the LimeSDR device.  
   * **Arguments**:  
     * `timeNs`: Time in nanoseconds to set.  
     * `what`: Optional descriptor for specifying time domain.  
   * **Usage**: Used to synchronize or adjust the internal time of the LimeSDR device, crucial for applications needing precise timing alignment​(alllimesdk).  
6. **`listAntennas(const int direction, const size_t channel) const`**  
   * **Description**: Lists available antenna options for a specified channel.  
   * **Arguments**:  
     * `direction`: RX or TX direction.  
     * `channel`: Channel index.  
   * **Returns**: `std::vector<std::string>` \- List of available antennas.  
   * **Usage**: Important for selecting antennas in frequency-specific configurations​(alllimesdk).  
7. **`setAntenna(const int direction, const size_t channel, const std::string &name)`**  
   * **Description**: Sets a specific antenna for RX or TX.  
   * **Arguments**:  
     * `direction`: RX or TX.  
     * `channel`: Channel index.  
     * `name`: Antenna name to select.  
   * **Usage**: Allows for dynamically switching antennas, aiding applications that require different antennas for specific frequency bands​(alllimesdk).  
8. **`setSampleRate(const int direction, const size_t channel, const double rate)`**  
   * **Description**: Sets the sample rate for a specified channel.  
   * **Arguments**:  
     * `direction`: RX or TX.  
     * `channel`: Channel index.  
     * `rate`: Desired sample rate in Hz.  
   * **Usage**: This is a core function for configuring sample rate, essential for bandwidth and data throughput requirements​(alllimesdk).
