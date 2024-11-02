# **LimeSuite SDK API Documentation**

## **Introduction**

The **LimeSuite SDK** is a comprehensive software library for interfacing with LimeSDR devices. It provides a rich set of functions for device identification, control, data streaming, calibration, and more. This documentation serves as a detailed reference to all functions available in the LimeSuite SDK, organized logically into categories for ease of use.

---

## **Table of Contents**

* Major Data Types  
* Common Argument Descriptions  
* API Reference  
  * Identification and Device Control API  
  * Stream and Buffer Management API  
  * Advanced Programming and GPIO API  
  * Specialized Signal and Calibration API  
  * Configuration and Utility API  
  * Sensor and Status Functions API  
  * Register Access API  
  * High-Level Control Functions API  
  * Communication Routines  
  * Clock and Frequency Control  
  * Logging and Error Handling  
  * Synchronization  
  * Additional Functions  
  * Advanced Control and Configuration  
  * LimeRFE-Specific Functions  
  * Utility and Device-Specific Configuration Functions  
  * Clock Functions  
* Conclusion

---

## **Major Data Types**

1. **`lms_device_t`**: Handle representing a connected LimeSDR device.  
2. **`lms_info_str_t`**: String type used for device information (e.g., serial numbers, device names).  
3. **`lms_stream_t`**: Structure representing a data stream configuration for RX or TX operations.  
4. **`lms_stream_meta_t`**: Contains metadata for streamed samples (e.g., timestamps).  
5. **`lms_stream_status_t`**: Holds status information about a data stream (e.g., FIFO levels, sample rates).  
6. **`lms_range_t`**: Represents a numeric range with minimum, maximum, and step values.  
7. **`lms_dev_info_t`**: Provides detailed information about a connected device, including firmware and hardware versions.  
8. **`lms_testsig_t`**: Enumerated type for selecting test signals (e.g., `LMS_TESTSIG_NONE`, `LMS_TESTSIG_DC`).  
9. **`lms_gfir_t`**: Enumerated type for selecting GFIR filters (`LMS_GFIR1`, `LMS_GFIR2`, `LMS_GFIR3`).  
10. **`lms_name_t`**: String type used for naming elements like antennas or clock sources.  
11. **`lms_prog_callback_t`**: Function pointer type for monitoring programming progress.  
12. **`LMS_LogHandler`**: Function pointer type for custom log handling.  
13. **`float_type`**: Floating-point type, typically `double`, used for frequency and bandwidth values.

---

## **Common Argument Descriptions**

* **`device`**: Pointer to an `lms_device_t`, representing the target LimeSDR device.  
* **`dir_tx`**: Boolean indicating direction; `true` for TX (transmit), `false` for RX (receive).  
* **`chan`**: Channel index, usually `0` or `1`.  
* **`freq`**: Frequency value in Hz, typically of type `float_type`.  
* **`bandwidth` / `bw`**: Bandwidth in Hz, of type `float_type`.  
* **`gain`**: Gain value in dB, represented as an unsigned integer.  
* **`enable`**: Boolean to enable (`true`) or disable (`false`) a feature or component.  
* **`samples`**: Pointer to a buffer containing sample data.  
* **`sample_count`**: Number of samples to process.  
* **`timeout_ms`**: Timeout value in milliseconds.  
* **`stream`**: Pointer to an `lms_stream_t` structure.  
* **`meta`**: Pointer to an `lms_stream_meta_t` structure.  
* **`buffer`**: Pointer to a data buffer for read/write operations.  
* **`length` / `len`**: Size of data in bytes.  
* **`address`**: Register address for read/write operations.  
* **`value`**: Data value for read/write operations.  
* **`callback`**: Function pointer for callbacks (e.g., progress updates).  
* **`param_id`**: Identifier for custom board parameters.  
* **`args`**: Optional arguments, usually a pointer to additional configurations.

---

## **API Reference**

### **Identification and Device Control API**

This API provides functions to identify connected LimeSDR devices, open and close connections, retrieve device information, and manage device settings.

---

1. **`LMS_GetDeviceList(lms_info_str_t *dev_list)`**  
   * **Description**: Retrieves a list of LimeSDR devices currently connected to the system.  
   * **Arguments**:  
     * `lms_info_str_t *dev_list`: An array to store device information. Each entry contains details such as device name and serial number.  
   * **Returns**: `int` \- The number of devices found, or `-1` if an error occurs.  
   * **Usage**: Use this function to enumerate all connected LimeSDR devices before opening a specific one.  
2. **`LMS_Open(lms_device_t **device, const lms_info_str_t info = nullptr, void* args = nullptr)`**  
   * **Description**: Opens a connection to a LimeSDR device, providing a device handle for further operations.  
   * **Arguments**:  
     * `lms_device_t **device`: Pointer to a variable where the device handle will be stored upon successful connection.  
     * `const lms_info_str_t info`: Device information string to select a specific device (optional). If `nullptr`, the first available device is opened.  
     * `void* args`: Optional arguments for advanced configurations (usually `nullptr`).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Must be called before any other operations are performed on the device.  
3. **`LMS_Close(lms_device_t *device)`**  
   * **Description**: Closes an open LimeSDR device and frees associated resources.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle to be closed.  
   * **Usage**: Should be called when the device is no longer needed to release resources properly.  
4. **`LMS_GetDeviceInfo(lms_device_t *device)`**  
   * **Description**: Retrieves detailed information about a connected LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle.  
   * **Returns**: `const lms_dev_info_t*` \- Pointer to a structure containing device information, including:  
     * `deviceName`: Name of the device model.  
     * `expansionName`: Name of any connected expansion board.  
     * `firmwareVersion`: Device's firmware version.  
     * `hardwareVersion`: Hardware version.  
     * `protocolVersion`: Communication protocol version.  
     * `gatewareVersion`: FPGA gateware version.  
     * `gatewareRevision`: Revision number of the gateware.  
     * `boardSerialNumber`: Serial number of the board.  
   * **Usage**: Useful for verifying device details, checking firmware versions, and ensuring compatibility.  
5. **`LMS_Reset(lms_device_t *device)`**  
   * **Description**: Resets the LimeSDR device, reinitializing it to its default state.  
   * **Arguments**:  
     * `lms_device_t *device`: The device handle.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for reinitializing the device without disconnecting it physically, especially after configuration changes.  
6. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables caching of register values to improve performance by reducing redundant reads.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool enable`: Set to `true` to enable caching, `false` to disable.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Enable caching when performing multiple read operations to improve performance; disable when real-time register values are required.  
7. **`LMS_GetNumChannels(lms_device_t *device, bool dir_tx)`**  
   * **Description**: Retrieves the number of available channels for RX or TX.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Set to `true` for TX channels, `false` for RX channels.  
   * **Returns**: `int` \- Number of channels, or `-1` on error.  
   * **Usage**: Determine the number of channels supported by the device for configuration purposes.  
8. **`LMS_GetSampleRateRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Retrieves the supported range of sample rates for RX or TX paths.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `lms_range_t *range`: Pointer to a structure to receive the range information (minimum, maximum, and step values).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to determine valid sample rates before setting them.  
9. **`LMS_GetAntennaList(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t *list)`**  
   * **Description**: Retrieves a list of available antennas for a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_name_t *list`: Buffer to store antenna names.  
   * **Returns**: `int` \- Number of antennas, or `-1` on error.  
   * **Usage**: Use to list antenna options before selecting one.  
10. **`LMS_SetClockSource(lms_device_t *device, const lms_name_t source)`**  
    * **Description**: Sets the clock source for the device (e.g., internal or external).  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `const lms_name_t source`: Name of the clock source.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Configure the device to use a specific clock source.  
11. **`LMS_GetClockSource(lms_device_t *device, lms_name_t *source)`**  
    * **Description**: Retrieves the current clock source used by the device.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `lms_name_t *source`: Buffer to store the clock source name.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Check which clock source is currently in use.  
12. **`LMS_VCTCXORead(lms_device_t *device, uint16_t *val)`**  
    * **Description**: Reads the current VCTCXO DAC trim value.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `uint16_t *val`: Pointer to store the DAC value.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Useful for calibrating the reference clock.  
13. **`LMS_VCTCXOWrite(lms_device_t *device, uint16_t val)`**  
    * **Description**: Writes a trim value to the VCTCXO DAC to adjust the reference clock frequency.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `uint16_t val`: DAC value to set.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Calibrate the reference clock for precise frequency control.  
14. **`LMS_GetLibraryVersion(void)`**  
    * **Description**: Returns the version of the LimeSuite library.  
    * **Returns**: `const char*` \- Version string.  
    * **Usage**: Check library version for compatibility.  
15. **`LMS_GetLastErrorMessage(void)`**  
    * **Description**: Retrieves the last error message generated by the API.  
    * **Returns**: `const char*` \- Error message string.  
    * **Usage**: Useful for debugging and error handling.  
16. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
    * **Description**: Registers a custom log handler to capture API log messages.  
    * **Arguments**:  
      * `LMS_LogHandler handler`: Pointer to a custom log handling function.  
    * **Usage**: Integrate LimeSuite logging with your application's logging system.  
17. **`LMS_Synchronize(lms_device_t *device, bool toChip)`**  
    * **Description**: Synchronizes register values between the software cache and the hardware.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool toChip`: `true` to write cache to hardware, `false` to update cache from hardware.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Ensure that software and hardware register values are consistent.  
    * 

---

### **Stream and Buffer Management API**

This section covers functions related to setting up, managing, and controlling data streams and buffers for transmitting (TX) and receiving (RX) data with LimeSDR devices.

---

1. **`LMS_SetupStream(lms_device_t *device, lms_stream_t *stream)`**  
   * **Description**: Configures a data stream for either RX or TX operations with specified parameters.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle obtained from `LMS_Open`.  
     * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure where stream parameters are set:  
       * `bool isTx`: Set to `true` for TX (transmit) stream, `false` for RX (receive) stream.  
       * `size_t channel`: Channel index (usually `0` or `1`).  
       * `float_type fifoSize`: Size of the FIFO buffer in samples (optional; default is 1 MB).  
       * `lms_stream_sample_format_t dataFmt`: Data format of the samples, e.g., `LMS_FMT_F32` (32-bit float), `LMS_FMT_I16` (16-bit integers), `LMS_FMT_I12` (12-bit integers packed in 16 bits).  
       * `float throughputVsLatency`: A parameter between 0 and 1 to balance throughput and latency (optional).  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Must be called before starting a data stream. Configures the stream parameters according to the application's needs.  
2. **`LMS_StartStream(lms_stream_t *stream)`**  
   * **Description**: Starts the specified data stream, enabling data transfer.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure previously set up by `LMS_SetupStream`.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Call this function after `LMS_SetupStream` to begin data transmission or reception.  
3. **`LMS_StopStream(lms_stream_t *stream)`**  
   * **Description**: Stops an active data stream, halting data transfer.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Pointer to an active `lms_stream_t` structure.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Use this function to stop data flow without destroying the stream configuration.  
4. **`LMS_DestroyStream(lms_device_t *device, lms_stream_t *stream)`**  
   * **Description**: Destroys a configured data stream and releases associated resources.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure to be destroyed.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Call this function when the stream is no longer needed to free up resources.  
5. **`LMS_RecvStream(lms_stream_t *stream, void *samples, size_t sample_count, lms_stream_meta_t *meta, unsigned timeout_ms)`**  
   * **Description**: Receives data from an active RX stream.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Pointer to an active RX `lms_stream_t` structure.  
     * `void *samples`: Buffer to store received samples.  
     * `size_t sample_count`: Number of samples to receive.  
     * `lms_stream_meta_t *meta`: Pointer to an `lms_stream_meta_t` structure for metadata (can be `nullptr` if not used).  
     * `unsigned timeout_ms`: Timeout in milliseconds for the operation.  
   * **Returns**: `int` \- Number of samples received, `-1` on error.  
   * **Usage**: Call repeatedly to read incoming data. The function blocks until the specified number of samples are received or the timeout expires.  
6. **`LMS_SendStream(lms_stream_t *stream, const void *samples, size_t sample_count, const lms_stream_meta_t *meta, unsigned timeout_ms)`**  
   * **Description**: Sends data through an active TX stream.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Pointer to an active TX `lms_stream_t` structure.  
     * `const void *samples`: Buffer containing samples to transmit.  
     * `size_t sample_count`: Number of samples to send.  
     * `const lms_stream_meta_t *meta`: Pointer to an `lms_stream_meta_t` structure for metadata (can be `nullptr` if not used).  
     * `unsigned timeout_ms`: Timeout in milliseconds for the operation.  
   * **Returns**: `int` \- Number of samples sent, `-1` on error.  
   * **Usage**: Call repeatedly to send outgoing data. The function blocks until the samples are sent or the timeout expires.  
7. **`LMS_GetStreamStatus(lms_stream_t *stream, lms_stream_status_t *status)`**  
   * **Description**: Retrieves the current status of an active data stream.  
   * **Arguments**:  
     * `lms_stream_t *stream`: Pointer to an active `lms_stream_t` structure.  
     * `lms_stream_status_t *status`: Pointer to an `lms_stream_status_t` structure to receive status information.  
       * Fields include:  
         * `float_type fifoFilledCount`: Number of samples currently in the FIFO buffer.  
         * `float_type fifoSize`: Total size of the FIFO buffer in samples.  
         * `uint32_t underrun`: Number of RX buffer underrun events.  
         * `uint32_t overrun`: Number of TX buffer overrun events.  
         * `uint32_t droppedPackets`: Number of dropped packets.  
         * `float_type sampleRate`: Current sample rate in samples per second.  
         * `uint32_t active`: Stream active status (`0` for inactive, `1` for active).  
         * `uint32_t linkRate`: Data transfer rate in bytes per second.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Use this function to monitor the health and performance of the stream, such as buffer levels and data rates.  
8. **`LMS_UploadWFM(lms_device_t *device, const void **samples, uint8_t chCount, size_t sample_count, int format)`**  
   * **Description**: Uploads a custom waveform to the LimeSDR's onboard memory for transmission via the Lime Suite API.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const void **samples`: Array of pointers to sample buffers for each channel.  
     * `uint8_t chCount`: Number of channels (1 or 2).  
     * `size_t sample_count`: Number of samples per channel (must be a multiple of 4).  
     * `int format`: Data format of the samples:  
       * `LMS_FMT_F32` (0): 32-bit floating-point.  
       * `LMS_FMT_I16` (1): 16-bit integers.  
       * `LMS_FMT_I12` (2): 12-bit integers packed in 16 bits.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Use to upload waveforms for continuous transmission without host intervention.  
9. **`LMS_EnableTxWFM(lms_device_t *device, unsigned chan, bool active)`**  
   * **Description**: Enables or disables the transmission of the uploaded waveform on a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `unsigned chan`: Channel index.  
     * `bool active`: `true` to enable waveform transmission, `false` to disable.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: After uploading a waveform with `LMS_UploadWFM`, use this function to start or stop its transmission.  
10. **`LMS_SetupTRX(lms_device_t *device, bool dir_tx, size_t chan, float_type sample_rate, float_type bandwidth, float_type frequency, lms_stream_t *stream)`**  
    * **Description**: Simplifies the setup of a transmit or receive path, configuring sample rate, bandwidth, frequency, and stream parameters.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `size_t chan`: Channel index.  
      * `float_type sample_rate`: Desired sample rate in samples per second.  
      * `float_type bandwidth`: Desired bandwidth in Hz.  
      * `float_type frequency`: Desired center frequency in Hz.  
      * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure to set up the stream (can be `nullptr` if stream setup is not required).  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: A convenience function to quickly configure common settings for RX or TX operations.  
11. **`LMS_GetNumChannels(lms_device_t *device, bool dir_tx)`**  
    * **Description**: Retrieves the number of available channels for RX or TX.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX channels, `false` for RX channels.  
    * **Returns**: `int` \- Number of channels, or `-1` on error.  
    * **Usage**: Determine the number of channels supported by the device for stream setup.  
12. **`LMS_SetupFIFO(lms_device_t *device, size_t size)`**  
    * **Description**: Sets the size of the FIFO buffer for data streams.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `size_t size`: Desired FIFO size in samples.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Adjust the FIFO size to balance between latency and data throughput requirements.  
13. **`LMS_SetupThreadPool(lms_device_t *device, unsigned threadCount)`**  
    * **Description**: Configures the number of threads used for handling data streams.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `unsigned threadCount`: Number of threads to allocate for streaming operations.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Optimize performance by adjusting the number of threads based on system capabilities and streaming requirements.  
14. **`LMS_SetupStreamMode(lms_stream_t *stream, lms_stream_mode_t mode)`**  
    * **Description**: Sets the mode of operation for a data stream.  
    * **Arguments**:  
      * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure.  
      * `lms_stream_mode_t mode`: Stream mode:  
        * `LMS_STREAM_MODE_BLOCKING`: Stream operates in blocking mode.  
        * `LMS_STREAM_MODE_NONBLOCKING`: Stream operates in non-blocking mode.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Choose between blocking and non-blocking modes based on application needs.  
15. **`LMS_FlushFIFO(lms_stream_t *stream)`**  
    * **Description**: Clears the FIFO buffer of a specified data stream.  
    * **Arguments**:  
      * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Use to discard any buffered data, often used when restarting a stream or recovering from an error.  
    * 

---

### **Advanced Programming and GPIO API**

Functions for programming the device and controlling GPIO pins.

---

1. **`LMS_Program(lms_device_t *device, const char *data, size_t size, const lms_name_t mode, lms_prog_callback_t callback)`**  
   * **Description**: Programs the device's firmware or gateware.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const char *data`: Data buffer containing the firmware or gateware.  
     * `size_t size`: Size of the data buffer.  
     * `const lms_name_t mode`: Programming mode (e.g., "FLASH", "RAM").  
     * `lms_prog_callback_t callback`: Callback function for progress updates.  
   * **Usage**: Update the device firmware or FPGA gateware.  
2. **`LMS_GPIOWrite(lms_device_t *device, const uint8_t *buffer, size_t len)`**  
   * **Description**: Writes values to GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint8_t *buffer`: Data to write to GPIO pins.  
     * `size_t len`: Length of the buffer.  
   * **Usage**: Control external hardware through GPIO.  
3. **`LMS_GPIORead(lms_device_t *device, uint8_t *buffer, size_t len)`**  
   * **Description**: Reads the current state of GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *buffer`: Buffer to receive GPIO states.  
     * `size_t len`: Length of the buffer.  
   * **Usage**: Monitor external signals connected to GPIO pins.  
4. **`LMS_GPIODirWrite(lms_device_t *device, const uint8_t *dir_buffer, size_t len)`**  
   * **Description**: Sets the direction (input/output) of GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint8_t *dir_buffer`: Direction configuration.  
     * `size_t len`: Length of the buffer.  
   * **Usage**: Configure GPIO pins before use.  
5. **`LMS_GPIODirRead(lms_device_t *device, uint8_t *dir_buffer, size_t len)`**  
   * **Description**: Reads the direction configuration of GPIO pins.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *dir_buffer`: Buffer to receive direction information.  
     * `size_t len`: Length of the buffer.  
   * **Usage**: Verify GPIO pin configurations.

---

### **Specialized Signal and Calibration API**

This section covers functions related to setting frequencies, gains, performing calibrations, and generating test signals. These functions are essential for optimizing the performance of the LimeSDR device and ensuring signal integrity.

---

1. **`LMS_SetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type freq)`**  
   * **Description**: Sets the Local Oscillator (LO) frequency for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle obtained from `LMS_Open`.  
     * `bool dir_tx`: Direction flag; set to `true` for TX (transmit), `false` for RX (receive).  
     * `size_t chan`: Channel index (usually `0` or `1`).  
     * `float_type freq`: Desired LO frequency in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to tune the device to a specific RF frequency. It's crucial for applications that require frequency agility or precise tuning.  
2. **`LMS_GetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *freq)`**  
   * **Description**: Retrieves the current Local Oscillator (LO) frequency for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type *freq`: Pointer to a variable where the current LO frequency will be stored.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use to verify the LO frequency currently set on the device.  
3. **`LMS_SetSampleRate(lms_device_t *device, float_type rate, size_t oversample)`**  
   * **Description**: Sets the sample rate for both RX and TX paths of the device, along with an optional oversampling factor.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `float_type rate`: Desired sample rate in samples per second (Hz).  
     * `size_t oversample`: Optional oversampling factor (e.g., 1, 2, 4). If set to 0, the API will select the best oversampling value.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjust the sample rate to match the bandwidth requirements of your application. Oversampling can improve signal quality but may increase computational load.  
4. **`LMS_GetSampleRate(lms_device_t *device, bool dir_tx, size_t chan, float_type *host_Hz, float_type *rf_Hz)`**  
   * **Description**: Retrieves the current sample rates for the specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type *host_Hz`: Pointer to store the sample rate between the device and the host (in Hz).  
     * `float_type *rf_Hz`: Pointer to store the sample rate between the RF frontend and the device's DSP core (in Hz).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use to confirm the actual sample rates being used, which may differ slightly from requested rates due to hardware limitations.  
5. **`LMS_SetGaindB(lms_device_t *device, bool dir_tx, size_t chan, unsigned gain)`**  
   * **Description**: Sets the gain for a specific RX or TX channel in decibels (dB).  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `unsigned gain`: Desired gain in dB.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjust gain to control signal amplitude, improve signal-to-noise ratio, or prevent saturation.  
6. **`LMS_GetGaindB(lms_device_t *device, bool dir_tx, size_t chan, unsigned *gain)`**  
   * **Description**: Retrieves the current gain setting for a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `unsigned *gain`: Pointer to store the gain in dB.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Verify the gain setting applied to the channel.  
7. **`LMS_SetLPFBW(lms_device_t *device, bool dir_tx, size_t chan, float_type bandwidth)`**  
   * **Description**: Sets the bandwidth of the low-pass filter (LPF) for a given RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type bandwidth`: Desired LPF bandwidth in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjust the LPF to match the signal bandwidth, reducing out-of-band noise and interference.  
8. **`LMS_GetLPFBW(lms_device_t *device, bool dir_tx, size_t chan, float_type *bandwidth)`**  
   * **Description**: Retrieves the current bandwidth setting of the low-pass filter for a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type *bandwidth`: Pointer to store the LPF bandwidth in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Verify the LPF bandwidth currently in use.  
9. **`LMS_Calibrate(lms_device_t *device, bool dir_tx, size_t chan, double bw, unsigned flags)`**  
   * **Description**: Performs automatic calibration of the specified RX or TX channel to optimize performance within the specified bandwidth.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `double bw`: Bandwidth for calibration in Hz.  
     * `unsigned flags`: Calibration options (set to `0` for default behavior).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Improves signal quality by calibrating the device's internal components, such as filters and gains.  
10. **`LMS_SetTestSignal(lms_device_t *device, bool dir_tx, size_t chan, lms_testsig_t sig, int16_t dc_i, int16_t dc_q)`**  
    * **Description**: Configures a test signal for debugging and calibration purposes.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_testsig_t sig`: Type of test signal to generate:  
        * `LMS_TESTSIG_NONE`: No test signal.  
        * `LMS_TESTSIG_DC`: DC offset signal.  
        * `LMS_TESTSIG_NCODIV4`: NCO signal with frequency division by 4\.  
        * `LMS_TESTSIG_NCODIV8`: NCO signal with frequency division by 8\.  
        * `LMS_TESTSIG_TONE`: Single tone generation.  
      * `int16_t dc_i`: DC offset for the I (in-phase) component (used if `sig` is `LMS_TESTSIG_DC`).  
      * `int16_t dc_q`: DC offset for the Q (quadrature) component (used if `sig` is `LMS_TESTSIG_DC`).  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use to inject known signals into the signal path for testing and calibration.  
11. **`LMS_GetTestSignal(lms_device_t *device, bool dir_tx, size_t chan, lms_testsig_t *sig)`**  
    * **Description**: Retrieves the current test signal configuration for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_testsig_t *sig`: Pointer to store the test signal type.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the test signal settings applied to the channel.  
12. **`LMS_GetChipTemperature(lms_device_t *device, size_t chan)`**  
    * **Description**: Reads the internal temperature of the LMS7002M chip for the specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `size_t chan`: Channel index.  
    * **Returns**: `float` \- Temperature in degrees Celsius.  
    * **Usage**: Monitor device temperature to prevent overheating and ensure reliable operation.  
13. **`LMS_CalibrateRx(lms_device_t *device, size_t chan, double bw, unsigned flags)`**  
    * **Description**: Calibrates the RX path on the specified channel for optimal performance over the given bandwidth.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `size_t chan`: Channel index.  
      * `double bw`: Bandwidth in Hz.  
      * `unsigned flags`: Calibration options.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use when precise calibration of the RX path is required, such as in sensitive reception applications.  
14. **`LMS_CalibrateTx(lms_device_t *device, size_t chan, double bw, unsigned flags)`**  
    * **Description**: Calibrates the TX path on the specified channel for optimal signal transmission over the specified bandwidth.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `size_t chan`: Channel index.  
      * `double bw`: Bandwidth in Hz.  
      * `unsigned flags`: Calibration options.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use to optimize the TX path for applications requiring high signal fidelity.  
15. **`LMS_SetNCOFrequency(lms_device_t *device, bool dir_tx, size_t chan, const float_type *freq, float_type pho)`**  
    * **Description**: Configures the Numerically Controlled Oscillator (NCO) with up to 16 predefined frequencies and a phase offset, enabling rapid frequency changes.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `const float_type *freq`: Array of NCO frequencies (must contain at least 16 values).  
      * `float_type pho`: Phase offset in degrees.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Essential for applications requiring frequency hopping or multi-tone generation.  
16. **`LMS_GetNCOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *freq, float_type *pho)`**  
    * **Description**: Retrieves the NCO frequency and phase offset settings for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *freq`: Array to store NCO frequencies.  
      * `float_type *pho`: Pointer to store the phase offset.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify NCO configurations for debugging or monitoring purposes.  
17. **`LMS_EnableChannel(lms_device_t *device, bool dir_tx, size_t chan, bool enable)`**  
    * **Description**: Enables or disables a specific RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `bool enable`: Set to `true` to enable the channel, `false` to disable.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Control which channels are active, useful for power saving or when only certain channels are needed.  
18. **`LMS_SetAntenna(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t antenna)`**  
    * **Description**: Selects an antenna for a specific RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_name_t antenna`: Antenna name or index.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Switch between different antenna options based on frequency bands or application requirements.  
19. **`LMS_GetAntenna(lms_device_t *device, bool dir_tx, size_t chan)`**  
    * **Description**: Retrieves the currently selected antenna for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
    * **Returns**: `int` \- Antenna index, or `-1` on error.  
    * **Usage**: Verify which antenna is currently in use.  
20. **`LMS_SetGFIRCoeff(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, const float_type *coef, size_t count)`**  
    * **Description**: Sets the coefficients for a General Finite Impulse Response (GFIR) filter.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_gfir_t filt`: GFIR filter type (`LMS_GFIR1`, `LMS_GFIR2`, `LMS_GFIR3`).  
      * `const float_type *coef`: Array of filter coefficients.  
      * `size_t count`: Number of coefficients.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Customize filter responses for specific signal processing requirements.  
21. **`LMS_EnableGFIR(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, bool enabled)`**  
    * **Description**: Enables or disables a specific GFIR filter.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_gfir_t filt`: GFIR filter type.  
      * `bool enabled`: Set to `true` to enable the filter, `false` to disable.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Control filter activation to optimize signal processing pipelines.  
22. **`LMS_SetGFIR(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, const float_type *coef, size_t count, bool enabled)`**  
    * **Description**: Combines setting coefficients and enabling/disabling a GFIR filter in one function.  
    * **Arguments**:  
      * Same as `LMS_SetGFIRCoeff` and `LMS_EnableGFIR`, with `enabled` flag.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Simplifies GFIR configuration by combining coefficient setting and filter enabling.  
    * 

---

### **Configuration and Utility API**

**This section covers functions related to saving and loading device configurations, performing device resets, synchronizing settings, and other utility functions that help manage the LimeSDR device's settings and states.**

---

1. **`LMS_SaveConfig(lms_device_t *device, const char *filename)`**  
   * **Description: Saves the current configuration of the LMS7002M chip to a file, allowing the configuration to be restored later.**  
   * **Arguments:**  
     * **`lms_device_t *device`: Device handle obtained from `LMS_Open`.**  
     * **`const char *filename`: Path to the file where the configuration will be saved.**  
   * **Returns: `int` \- `0` on success, `-1` if an error occurs.**  
   * **Usage: Useful for saving custom configurations that may need to be reloaded across sessions. For instance, a user might save specific tuning, calibration, or filter settings and reload them as needed.**  
2. **`LMS_LoadConfig(lms_device_t *device, const char *filename)`**  
   * **Description: Loads a previously saved LMS7002M chip configuration from a file.**  
   * **Arguments:**  
     * **`lms_device_t *device`: Device handle.**  
     * **`const char *filename`: Path to the file containing the saved configuration.**  
   * **Returns: `int` \- `0` if successful, `-1` if the configuration load fails.**  
   * **Usage: Used to restore the device state to a known configuration, which can be helpful for quick setup in scenarios where multiple devices need to be consistently configured.**  
3. **`LMS_Reset(lms_device_t *device)`**  
   * **Description: Resets the entire LimeSDR device, returning it to its default state.**  
   * **Arguments:**  
     * **`lms_device_t *device`: The handle of the device to reset.**  
   * **Returns: `int` \- `0` on success, `-1` if an error occurs.**  
   * **Usage: A complete hardware reset is useful when a device needs to be cleared of all configurations, as it reinitializes the LimeSDR to its default setup. This function is typically used to recover from errors or prepare the device for a new configuration.**  
4. **`LMS_Synchronize(lms_device_t *device, bool toChip)`**  
   * **Description: Synchronizes the register values between the software cache and the hardware registers of the LimeSDR device.**  
   * **Arguments:**  
     * **`lms_device_t *device`: Device handle.**  
     * **`bool toChip`: If `true`, copies the values from the software cache to the hardware registers; if `false`, updates the cache with the values from the hardware.**  
   * **Returns: `int` \- `0` on success, `-1` if an error occurs.**  
   * **Usage: Important for ensuring configuration consistency, particularly in scenarios where external changes may affect register values or when multiple software components may interact with the device.**  
5. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description: Enables or disables caching of LMS7002M and FPGA register values for the specified device.**  
   * **Arguments:**  
     * **`lms_device_t *device`: Device handle.**  
     * **`bool enable`: Set to `true` to enable caching, `false` to disable.**  
   * **Returns: `int` \- `0` if successful, `-1` on failure.**  
   * **Usage: Helps improve performance by reducing redundant register reads, beneficial in applications requiring frequent access to register values. Caching should be disabled when real-time register updates are critical.**  
6. **`LMS7002M::SoftReset()`**  
   * **Description: Performs a software reset of the LMS7002M chip, reinitializing its state without fully powering down or disconnecting the device.**  
   * **Arguments: None.**  
   * **Returns: `int` \- `0` on success, `-1` if the reset fails.**  
   * **Usage: Useful for resetting the chip to a known state without a complete hardware reset. This function is often used to clear any temporary settings or errors that may have occurred during testing or streaming.**  
7. **`LMS_GetDeviceInfo(lms_device_t *device, lms_dev_info_t **info)`**  
   * **Description: Retrieves detailed information about the connected LimeSDR device, including platform-specific details such as PCIe configurations.**  
   * **Arguments:**  
     * **`lms_device_t *device`: Device handle.**  
     * **`lms_dev_info_t **info`: Pointer to a pointer where the device information structure will be stored.**  
   * **Returns: `int` \- `0` on success, `-1` on error.**  
   * **Usage: Important when managing multiple devices or platforms, especially with PCIe-based LimeSDR configurations.**  
8. **`LMS_GetLibraryVersion(void)`**  
   * **Description: Returns the version of the LimeSuite API library currently in use.**  
   * **Returns: `const char*` \- A string indicating the version.**  
   * **Usage: Important for compatibility checks, especially when developing software that requires specific library versions.**  
9. **`LMS_GetLastErrorMessage(void)`**  
   * **Description: Retrieves the last error message generated by the LimeSuite API, providing details on why the most recent API function call failed.**  
   * **Returns: `const char*` \- String containing the last error message.**  
   * **Usage: Useful in debugging, particularly in cases where more context is required to understand and address errors.**  
10. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
    * **Description: Registers a custom log handler, redirecting API log messages (errors, warnings, info) to a user-defined function.**  
    * **Arguments:**  
      * **`LMS_LogHandler handler`: Pointer to a logging function that will handle messages based on log level.**  
    * **Usage: Useful for integrating LimeSDR logging with external logging systems, enabling custom message handling and filtering.**  
11. **`LMS_GetDeviceList(lms_info_str_t *dev_list)`**  
    * **Description: Retrieves a list of connected LimeSDR devices.**  
    * **Arguments:**  
      * **`lms_info_str_t *dev_list`: Array to store device information.**  
    * **Returns: `int` \- Number of devices found, or `-1` on error.**  
    * **Usage: Scan for available devices and select one for use. Useful in applications that may need to handle multiple devices.**  
12. **`LMS_Init(lms_device_t *device)`**  
    * **Description: Initializes the device with settings to prepare it for operation.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
    * **Returns: `int` \- `0` on success, `-1` on error.**  
    * **Usage: This function is fundamental for initializing the LimeSDR to its default operational state, a prerequisite for reliable data transmission and reception.**  
13. **`LMS_ReadCustomBoardParam(lms_device_t *device, uint8_t param_id, float_type *val, lms_name_t units)`**  
    * **Description: Reads a custom board parameter by its identifier.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`uint8_t param_id`: ID of the parameter to read.**  
      * **`float_type *val`: Pointer to store the value read.**  
      * **`lms_name_t units`: Buffer to store units associated with the parameter.**  
    * **Returns: `int` \- `0` on success, `-1` on failure.**  
    * **Usage: Allows reading unique board parameters that may be specific to custom configurations or experimental setups.**  
14. **`LMS_WriteCustomBoardParam(lms_device_t *device, uint8_t param_id, float_type val, const lms_name_t units)`**  
    * **Description: Writes a value to a specified custom board parameter.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`uint8_t param_id`: ID of the parameter to set.**  
      * **`float_type val`: Value to write.**  
      * **`const lms_name_t units`: Units associated with the parameter.**  
    * **Returns: `int` \- `0` on success, `-1` if an error occurs.**  
    * **Usage: Useful in custom applications needing control over experimental or board-specific parameters for configuration beyond standard API limits.**  
15. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
    * **Description: Enables or disables register caching for a specific device.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`bool enable`: Set to `true` to enable caching, `false` to disable.**  
    * **Returns: `int` \- `0` if successful, `-1` on failure.**  
    * **Usage: Caching improves performance by reducing the need for redundant register reads. It should be enabled in scenarios where register values do not need to be immediately updated, and disabled when real-time data is critical.**  
16. **`LMS_GetClockLockedStatus(lms_device_t *device, size_t chan)`**  
    * **Description: Checks if the LO (Local Oscillator) or CGEN (Clock Generator) clock is locked for the specified channel, confirming that the frequency settings are stable.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`size_t chan`: Channel index (e.g., 0 or 1).**  
    * **Returns: `bool` \- `true` if the clock is locked; `false` if not.**  
    * **Usage: Clock locking is critical in applications requiring high-frequency stability. This function helps verify that the LO or CGEN is properly tuned and stable before streaming or testing.**  
17. **`LMS_GetNumChannels(lms_device_t *device, bool dir_tx)`**  
    * **Description: Retrieves the number of available channels for either RX or TX.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`bool dir_tx`: `true` for TX channels, `false` for RX channels.**  
    * **Returns: `int` \- Number of channels on success, `-1` on failure.**  
    * **Usage: Used to check the available channel count, which is essential in multi-channel applications to dynamically adjust the configuration.**  
18. **`LMS_GetSampleRateRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description: Provides the supported range of sample rates for a specified RX or TX path.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`bool dir_tx`: `true` for TX, `false` for RX.**  
      * **`lms_range_t *range`: Pointer to a structure that stores the minimum, maximum, and step values of the sample rate.**  
    * **Returns: `int` \- `0` on success, `-1` if an error occurs.**  
    * **Usage: Useful for determining valid sample rates prior to configuration, ensuring that settings fall within the device’s operational limits.**  
19. **`LMS_GetLOFrequencyRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description: Retrieves the permissible range of LO frequencies for a specified RX or TX channel.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`bool dir_tx`: `true` for TX, `false` for RX.**  
      * **`lms_range_t *range`: Structure to store the minimum, maximum, and step values for the LO frequency.**  
    * **Returns: `int` \- `0` on success, `-1` on error.**  
    * **Usage: This function is crucial for applications requiring dynamic tuning within the hardware's frequency capabilities.**  
20. **`LMS_GetLPFBWRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description: Retrieves the allowed range of low-pass filter (LPF) bandwidths for a specified RX or TX path.**  
    * **Arguments:**  
      * **`lms_device_t *device`: Device handle.**  
      * **`bool dir_tx`: `true` for TX, `false` for RX.**  
      * **`lms_range_t *range`: Pointer to an `lms_range_t` structure to store the min, max, and step values.**  
    * **Returns: `int` \- `0` on success, `-1` on error.**  
    * **Usage: This function is critical for applications needing to adjust the LPF within the supported bandwidth range, ensuring optimal filtering for specific frequency ranges.**  
    * 

---

### **Sensor and Status Functions API**

This section covers functions that allow you to retrieve various sensor readings and status information from the LimeSDR device, including temperatures, clock status, stream status, and GPIO pin states. These functions are essential for monitoring the device's operational health and for debugging purposes.

---

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
     * `float_type fifoFilledCount`: Number of samples currently in the FIFO buffer.  
     * `float_type fifoSize`: Total size of the FIFO buffer in samples.  
     * `uint32_t underrun`: Counter for RX buffer underrun events.  
     * `uint32_t overrun`: Counter for TX buffer overrun events.  
     * `uint32_t droppedPackets`: Count of packets dropped.  
     * `float_type sampleRate`: Current sample rate in samples per second.  
     * `uint32_t active`: Stream active status (`0` for inactive, `1` for active).  
     * `uint32_t linkRate`: Data transfer rate in bytes per second.  
   * **Returns**: `int` \- `0` if successful, `-1` if an error occurs.  
   * **Usage**: This function provides insights into the health and performance of the stream, useful for real-time monitoring, particularly in applications that require uninterrupted data flow.  
4. **`LMS_GPIORead(lms_device_t *device, uint8_t *buffer, size_t len)`**  
   * **Description**: Reads the status of the GPIO pins on the device, storing the result in the provided buffer.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *buffer`: Buffer where the GPIO states will be stored (each bit represents a GPIO pin state).  
     * `size_t len`: Length of the buffer in bytes.  
   * **Returns**: `int` \- `0` if successful, `-1` on error.  
   * **Usage**: This function is used to monitor the current state of GPIO pins, which may indicate device status or be used to interface with external components.  
5. **`LMS_GPIODirRead(lms_device_t *device, uint8_t *dir_buffer, size_t len)`**  
   * **Description**: Reads the direction configuration of the GPIO pins (input or output).  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint8_t *dir_buffer`: Buffer to store the direction of each GPIO pin (1 for output, 0 for input).  
     * `size_t len`: Length of the buffer in bytes.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for verifying GPIO pin configurations, especially when working with custom hardware interfaces.  
6. **`LMS_GetDeviceInfo(lms_device_t *device)`**  
   * **Description**: Retrieves detailed information about the connected LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
   * **Returns**: `const lms_dev_info_t*` \- Pointer to a structure containing device information, including firmware version, hardware version, serial number, etc.  
   * **Usage**: Useful for logging device information or verifying compatibility.  
7. **`LMS_GetProgramModes(lms_device_t *device, lms_name_t *list)`**  
   * **Description**: Retrieves a list of available programming modes supported by the LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_name_t *list`: Buffer to store programming mode names.  
   * **Returns**: `int` \- Number of available modes, `-1` on failure.  
   * **Usage**: Useful for firmware and gateware updates, allowing dynamic selection of the appropriate programming mode.  
8. **`LMS_GetNumChannels(lms_device_t *device, bool dir_tx)`**  
   * **Description**: Retrieves the number of available channels for either RX or TX.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX channels, `false` for RX channels.  
   * **Returns**: `int` \- Number of channels on success, `-1` on failure.  
   * **Usage**: Useful for determining the channel capabilities of the device.  
9. **`LMS_GetAntennaList(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t *list)`**  
   * **Description**: Returns a list of available antennas for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: `true` for TX, `false` for RX.  
     * `size_t chan`: Channel index.  
     * `lms_name_t *list`: Buffer to store antenna names.  
   * **Returns**: `int` \- Number of antennas available, or `-1` on error.  
   * **Usage**: Useful for antenna selection when switching between frequency bands or different signal conditions.  
10. **`LMS_GetSampleRateRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description**: Provides the supported range of sample rates for a specified RX or TX path.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `lms_range_t *range`: Pointer to a structure that stores the minimum, maximum, and step values of the sample rate.  
    * **Returns**: `int` \- `0` on success, `-1` if an error occurs.  
    * **Usage**: Useful for determining valid sample rates prior to configuration, ensuring that settings fall within the device’s operational limits.  
11. **`LMS_GetLOFrequencyRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description**: Retrieves the permissible range of LO frequencies for a specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `lms_range_t *range`: Structure to store the minimum, maximum, and step values for the LO frequency.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: This function is crucial for applications requiring dynamic tuning within the hardware's frequency capabilities.  
12. **`LMS_GetLPFBWRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
    * **Description**: Retrieves the allowed range of low-pass filter (LPF) bandwidths for a specified RX or TX path.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `lms_range_t *range`: Pointer to an `lms_range_t` structure to store the min, max, and step values.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: This function is critical for applications needing to adjust the LPF within the supported bandwidth range, ensuring optimal filtering for specific frequency ranges.  
13. **`LMS_GetGaindBRange(lms_device_t *device, bool dir_tx, size_t chan, lms_range_t *range)`**  
    * **Description**: Retrieves the gain range in decibels (dB) for the specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `size_t chan`: Channel index.  
      * `lms_range_t *range`: Pointer to store the gain range.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Useful for understanding the valid gain settings before configuring gain levels.  
14. **`LMS_GetSampleRate(lms_device_t *device, bool dir_tx, size_t chan, float_type *host_Hz, float_type *rf_Hz)`**  
    * **Description**: Retrieves the current sample rates for the specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *host_Hz`: Pointer to store the sample rate between the device and the host (in Hz).  
      * `float_type *rf_Hz`: Pointer to store the sample rate between the RF frontend and the device's DSP core (in Hz).  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use to confirm the actual sample rates being used, which may differ slightly from requested rates due to hardware limitations.  
15. **`LMS_GetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *freq)`**  
    * **Description**: Retrieves the current Local Oscillator (LO) frequency for a specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *freq`: Pointer to store the frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Useful for verifying the LO frequency, particularly in applications requiring specific frequency settings.  
16. **`LMS_GetAntenna(lms_device_t *device, bool dir_tx, size_t chan)`**  
    * **Description**: Retrieves the currently selected antenna for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
    * **Returns**: `int` \- Antenna index, or `-1` on error.  
    * **Usage**: Verify which antenna is currently in use.  
17. **`LMS_GetGFIRCoeff(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, float_type *coef)`**  
    * **Description**: Retrieves the current coefficients set for a specified GFIR filter.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: `true` for TX, `false` for RX.  
      * `size_t chan`: Channel index.  
      * `lms_gfir_t filt`: GFIR filter type (`LMS_GFIR1`, `LMS_GFIR2`, `LMS_GFIR3`).  
      * `float_type *coef`: Array to store retrieved coefficients.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Useful for reading and verifying filter configurations, allowing dynamic adjustments in real-time applications.  
    * 

---

### **Register Access API**

This section covers functions that allow direct access to the registers of the LMS7002M chip and the FPGA on the LimeSDR device. These functions are intended for advanced users who need fine-grained control over the device's internal settings beyond what is provided by the higher-level APIs.

---

1. **`LMS_WriteFPGAReg(lms_device_t *device, uint32_t address, uint32_t value)`**  
   * **Description**: Writes a 32-bit value to a specified FPGA register.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `uint32_t address`: The address of the FPGA register to write to.  
     * `uint32_t value`: The 32-bit value to write to the register.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to configure FPGA settings that are not accessible via higher-level APIs. Requires knowledge of the FPGA register map.  
2. **`LMS_ReadFPGAReg(lms_device_t *device, uint32_t address, uint32_t *value)`**  
   * **Description**: Reads a 32-bit value from a specified FPGA register.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint32_t address`: The address of the FPGA register to read from.  
     * `uint32_t *value`: Pointer to a variable where the read value will be stored.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for retrieving FPGA status or configuration settings not exposed via higher-level APIs.  
3. **`LMS_WriteLMSReg(lms_device_t *device, uint32_t address, uint16_t value)`**  
   * **Description**: Writes a 16-bit value to a specified LMS7002M register.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint32_t address`: The address of the LMS7002M register to write to.  
     * `uint16_t value`: The 16-bit value to write to the register.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Allows for low-level configuration of the LMS7002M chip. Knowledge of the LMS7002M register map is required.  
4. **`LMS_ReadLMSReg(lms_device_t *device, uint32_t address, uint16_t *value)`**  
   * **Description**: Reads a 16-bit value from a specified LMS7002M register.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint32_t address`: The address of the LMS7002M register to read from.  
     * `uint16_t *value`: Pointer to a variable where the read value will be stored.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for reading internal settings or status from the LMS7002M chip.  
5. **`LMS_WriteRegisters(lms_device_t *device, const uint32_t *addresses, const uint32_t *values, size_t count, bool isFPGA)`**  
   * **Description**: Writes multiple values to a sequence of FPGA or LMS7002M registers.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint32_t *addresses`: Array of register addresses.  
     * `const uint32_t *values`: Array of values to write.  
     * `size_t count`: Number of registers to write.  
     * `bool isFPGA`: Set to `true` to write to FPGA registers, `false` for LMS7002M registers.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Efficiently write to multiple registers in a single function call.  
6. **`LMS_ReadRegisters(lms_device_t *device, const uint32_t *addresses, uint32_t *values, size_t count, bool isFPGA)`**  
   * **Description**: Reads multiple values from a sequence of FPGA or LMS7002M registers.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const uint32_t *addresses`: Array of register addresses to read from.  
     * `uint32_t *values`: Array where the read values will be stored.  
     * `size_t count`: Number of registers to read.  
     * `bool isFPGA`: Set to `true` to read from FPGA registers, `false` for LMS7002M registers.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Efficiently read multiple registers in a single function call.  
7. **`LMS_WriteParam(lms_device_t *device, const LMS7Parameter param, int value)`**  
   * **Description**: Writes a value to a specific LMS7002M parameter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const LMS7Parameter param`: The parameter to write to.  
     * `int value`: The value to set for the parameter.  
   * **Returns**: `int` \- The actual value written (may differ due to limitations), or `-1` on error.  
   * **Usage**: Provides a higher-level interface to set specific parameters without directly accessing registers.  
8. **`LMS_ReadParam(lms_device_t *device, const LMS7Parameter param)`**  
   * **Description**: Reads the current value of a specific LMS7002M parameter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const LMS7Parameter param`: The parameter to read.  
   * **Returns**: `int` \- The value of the parameter, or `-1` on error.  
   * **Usage**: Useful for checking the settings of specific parameters.  
9. **`LMS_SPIWrite(lms_device_t *device, uint16_t address, uint16_t value)`**  
   * **Description**: Writes a 16-bit value to a specified address over the SPI interface.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t address`: The address to write to.  
     * `uint16_t value`: The value to write.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Advanced function for direct SPI communication with the device.  
10. **`LMS_SPIRead(lms_device_t *device, uint16_t address, uint16_t *value)`**  
    * **Description**: Reads a 16-bit value from a specified address over the SPI interface.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `uint16_t address`: The address to read from.  
      * `uint16_t *value`: Pointer to store the read value.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Allows for direct SPI communication, useful for debugging or accessing non-standard features.  
11. **`LMS_GPIODirWrite(lms_device_t *device, const uint8_t *dir_buffer, size_t len)`**  
    * **Description**: Sets the direction (input/output) of the GPIO pins.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `const uint8_t *dir_buffer`: Buffer specifying the direction of each GPIO pin (1 for output, 0 for input).  
      * `size_t len`: Length of the buffer in bytes.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Configure the GPIO pins for interfacing with external hardware.  
12. **`LMS_GPIOWrite(lms_device_t *device, const uint8_t *buffer, size_t len)`**  
    * **Description**: Writes values to the GPIO pins.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `const uint8_t *buffer`: Buffer containing the values to write to each GPIO pin.  
      * `size_t len`: Length of the buffer in bytes.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Control external devices connected via GPIO.  
13. **`LMS_GPIODirRead(lms_device_t *device, uint8_t *dir_buffer, size_t len)`**  
    * **Description**: Reads the direction configuration of the GPIO pins.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `uint8_t *dir_buffer`: Buffer to store the direction of each GPIO pin.  
      * `size_t len`: Length of the buffer in bytes.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the current GPIO direction settings.  
14. **`LMS_GPIORead(lms_device_t *device, uint8_t *buffer, size_t len)`**  
    * **Description**: Reads the values of the GPIO pins.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `uint8_t *buffer`: Buffer to store the values read from the GPIO pins.  
      * `size_t len`: Length of the buffer in bytes.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Read input signals from external devices connected via GPIO.  
    * 

---

### **High-Level Control Functions API**

This section provides functions that offer a higher-level interface for controlling the LimeSDR device, simplifying common operations such as enabling channels, selecting antennas, and setting gains. These functions abstract away some of the complexity involved in direct register manipulation, making it easier to configure the device for typical use cases.

---

1. **`LMS_EnableChannel(lms_device_t *device, bool dir_tx, size_t chan, bool enable)`**  
   * **Description**: Enables or disables a specific RX or TX channel on the LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `bool dir_tx`: Direction flag; set to `true` for TX (transmit) channel, `false` for RX (receive) channel.  
     * `size_t chan`: Channel index (usually `0` or `1`).  
     * `bool enable`: Set to `true` to enable the channel, or `false` to disable it.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to activate or deactivate specific channels. This is useful in applications where only certain channels are needed, allowing for resource optimization and power savings.  
2. **`LMS_SetAntenna(lms_device_t *device, bool dir_tx, size_t chan, int antenna)`**  
   * **Description**: Selects an antenna port for a specific RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `int antenna`: Index of the antenna to select. Valid antenna indexes can be obtained using `LMS_GetAntennaList`.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to switch between different antenna ports, which may correspond to different frequency ranges or physical connectors on the device.  
3. **`LMS_GetAntenna(lms_device_t *device, bool dir_tx, size_t chan)`**  
   * **Description**: Retrieves the currently selected antenna index for a specified channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
   * **Returns**: `int` \- Antenna index on success, `-1` on error.  
   * **Usage**: Use this function to determine which antenna port is currently selected for a channel.  
4. **`LMS_GetAntennaList(lms_device_t *device, bool dir_tx, size_t chan, lms_name_t *list)`**  
   * **Description**: Retrieves a list of available antennas for a specified RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `lms_name_t *list`: Buffer to store the names of available antennas.  
   * **Returns**: `int` \- Number of antennas available, or `-1` on error.  
   * **Usage**: Use this function to obtain the list of antenna options before selecting one with `LMS_SetAntenna`.  
5. **`LMS_SetGaindB(lms_device_t *device, bool dir_tx, size_t chan, unsigned gain)`**  
   * **Description**: Sets the overall gain for a specific RX or TX channel in decibels (dB).  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `unsigned gain`: Desired gain in dB.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjusts the signal amplitude by setting the combined gain of multiple gain stages in the device. The function calculates and sets the appropriate values for individual gain stages to achieve the requested overall gain.  
6. **`LMS_GetGaindB(lms_device_t *device, bool dir_tx, size_t chan, unsigned *gain)`**  
   * **Description**: Retrieves the current overall gain setting for a specific RX or TX channel in decibels (dB).  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `unsigned *gain`: Pointer to store the retrieved gain value.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use to check the current gain setting applied to a channel.  
7. **`LMS_SetNormalizedGain(lms_device_t *device, bool dir_tx, size_t chan, float_type gain)`**  
   * **Description**: Sets the gain for a specific channel using a normalized value between 0.0 and 1.0.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type gain`: Normalized gain value (0.0 for minimum gain, 1.0 for maximum gain).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Provides a simplified way to set gain without needing to specify decibel values.  
8. **`LMS_GetNormalizedGain(lms_device_t *device, bool dir_tx, size_t chan, float_type *gain)`**  
   * **Description**: Retrieves the current normalized gain setting for a specific channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type *gain`: Pointer to store the normalized gain value.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use to obtain the current gain setting in normalized form.  
9. **`LMS_SetLPFBW(lms_device_t *device, bool dir_tx, size_t chan, float_type bandwidth)`**  
   * **Description**: Sets the bandwidth of the low-pass filter (LPF) for a given RX or TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `float_type bandwidth`: Desired LPF bandwidth in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Adjusts the LPF to match the signal bandwidth, reducing out-of-band noise and interference.  
10. **`LMS_GetLPFBW(lms_device_t *device, bool dir_tx, size_t chan, float_type *bandwidth)`**  
    * **Description**: Retrieves the current bandwidth setting of the low-pass filter for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *bandwidth`: Pointer to store the LPF bandwidth in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the LPF bandwidth currently in use.  
11. **`LMS_SetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type frequency)`**  
    * **Description**: Sets the Local Oscillator (LO) frequency for a specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type frequency`: Desired LO frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Tune the device to a specific RF frequency.  
12. **`LMS_GetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *frequency)`**  
    * **Description**: Retrieves the current Local Oscillator (LO) frequency for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *frequency`: Pointer to store the LO frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the LO frequency setting.  
13. **`LMS_SetSampleRateDir(lms_device_t *device, bool dir_tx, float_type rate, size_t oversample)`**  
    * **Description**: Sets the sample rate with direction-specific control, allowing different sample rates for RX and TX.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `float_type rate`: Desired sample rate in samples per second.  
      * `size_t oversample`: Oversampling factor (e.g., 1, 2, 4).  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Allows setting distinct sample rates for RX and TX paths, useful in applications where the transmit and receive bandwidths differ.  
14. **`LMS_SetupChannel(lms_device_t *device, bool dir_tx, size_t chan, lms_stream_t *stream)`**  
    * **Description**: Simplifies the setup process by enabling the channel and configuring the associated stream.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `lms_stream_t *stream`: Pointer to an `lms_stream_t` structure to configure the stream.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Streamlines the initialization of a channel and its data stream, combining multiple steps into one function.  
15. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
    * **Description**: Enables or disables caching of LMS7002M and FPGA register values for the specified device.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool enable`: Set to `true` to enable caching, `false` to disable.  
    * **Returns**: `int` \- `0` if successful, `-1` on failure.  
    * **Usage**: Improves performance by reducing redundant register reads. Should be enabled when frequent access to register values is required without the need for immediate updates from the hardware.  
    * 

---

### **Communication Routines**

This section covers low-level functions for data transmission and reception, providing direct access to communication endpoints. These routines are intended for advanced users who need fine-grained control over data handling, often bypassing higher-level stream APIs.

---

1. **`IConnection::ReceiveData(char* buffer, int length, int epIndex, int timeout)`**  
   * **Description**: Receives data from a specified communication endpoint on the LimeSDR device.  
   * **Arguments**:  
     * `char* buffer`: Pointer to the buffer where the received data will be stored.  
     * `int length`: The maximum number of bytes to receive.  
     * `int epIndex`: The index of the endpoint from which to receive data.  
     * `int timeout`: Timeout for the operation in milliseconds.  
   * **Returns**: `int` \- The actual number of bytes received, or a negative value on error.  
   * **Usage**: This function provides direct access to the USB or PCIe communication endpoints, allowing custom implementations of data handling. It is mainly used in situations where the default streaming mechanism does not meet specific application requirements.  
2. **`IConnection::SendData(const char* buffer, int length, int epIndex, int timeout)`**  
   * **Description**: Sends data to a specified communication endpoint on the LimeSDR device.  
   * **Arguments**:  
     * `const char* buffer`: Pointer to the buffer containing the data to be sent.  
     * `int length`: The number of bytes to send.  
     * `int epIndex`: The index of the endpoint to which data will be sent.  
     * `int timeout`: Timeout for the operation in milliseconds.  
   * **Returns**: `int` \- The actual number of bytes sent, or a negative value on error.  
   * **Usage**: Use this function for direct transmission of data to the device, bypassing higher-level abstractions. This can be useful for implementing custom protocols or for low-level debugging.  
3. **`IConnection::BeginDataReading(char* buffer, uint32_t length, int ep)`**  
   * **Description**: Initiates an asynchronous data reading operation from a specified endpoint.  
   * **Arguments**:  
     * `char* buffer`: Pointer to the buffer where the received data will be stored.  
     * `uint32_t length`: The maximum number of bytes to read.  
     * `int ep`: The endpoint index to read from.  
   * **Returns**: `int` \- A handle or identifier for the read operation, or a negative value on error.  
   * **Usage**: Allows for non-blocking, asynchronous data reception, which can improve performance in applications that can handle concurrent operations.  
4. **`IConnection::WaitForReading(int contextHandle, unsigned int timeout_ms)`**  
   * **Description**: Waits for a previously initiated asynchronous read operation to complete.  
   * **Arguments**:  
     * `int contextHandle`: The handle returned by `BeginDataReading`.  
     * `unsigned int timeout_ms`: Timeout in milliseconds.  
   * **Returns**: `int` \- The actual number of bytes read, or a negative value on error.  
   * **Usage**: Used in conjunction with `BeginDataReading` to retrieve data once the asynchronous operation is complete.  
5. **`IConnection::FinishDataReading(char* buffer, uint32_t length, int contextHandle)`**  
   * **Description**: Completes an asynchronous data reading operation and retrieves the data.  
   * **Arguments**:  
     * `char* buffer`: Buffer to store the received data.  
     * `uint32_t length`: The number of bytes to retrieve.  
     * `int contextHandle`: The handle returned by `BeginDataReading`.  
   * **Returns**: `int` \- The actual number of bytes read, or a negative value on error.  
   * **Usage**: Finalizes the asynchronous read operation, ensuring data integrity.  
6. **`IConnection::BeginDataSending(const char* buffer, uint32_t length, int ep)`**  
   * **Description**: Initiates an asynchronous data sending operation to a specified endpoint.  
   * **Arguments**:  
     * `const char* buffer`: Pointer to the data to send.  
     * `uint32_t length`: The number of bytes to send.  
     * `int ep`: The endpoint index to send data to.  
   * **Returns**: `int` \- A handle or identifier for the send operation, or a negative value on error.  
   * **Usage**: Allows for non-blocking, asynchronous data transmission.  
7. **`IConnection::WaitForSending(int contextHandle, unsigned int timeout_ms)`**  
   * **Description**: Waits for a previously initiated asynchronous send operation to complete.  
   * **Arguments**:  
     * `int contextHandle`: The handle returned by `BeginDataSending`.  
     * `unsigned int timeout_ms`: Timeout in milliseconds.  
   * **Returns**: `int` \- The actual number of bytes sent, or a negative value on error.  
   * **Usage**: Used in conjunction with `BeginDataSending` to ensure the data has been transmitted.  
8. **`IConnection::FinishDataSending(const char* buffer, uint32_t length, int contextHandle)`**  
   * **Description**: Completes an asynchronous data sending operation.  
   * **Arguments**:  
     * `const char* buffer`: Pointer to the data that was sent.  
     * `uint32_t length`: The number of bytes that were to be sent.  
     * `int contextHandle`: The handle returned by `BeginDataSending`.  
   * **Returns**: `int` \- The actual number of bytes sent, or a negative value on error.  
   * **Usage**: Finalizes the asynchronous send operation.  
9. **`IConnection::ResetStreamBuffers()`**  
   * **Description**: Resets the stream buffers and state machines in the device's FPGA.  
   * **Arguments**: None.  
   * **Returns**: `void`.  
   * **Usage**: Use this function to clear any residual data in the buffers, typically after stopping streams or recovering from errors.  
10. **`IConnection::GetStreamSize()`**  
    * **Description**: Retrieves the size of the stream endpoint buffers.  
    * **Arguments**: None.  
    * **Returns**: `int` \- The size of the stream buffer in bytes.  
    * **Usage**: Useful for optimizing buffer sizes in host applications to match the device's capabilities.

**Note:** The `IConnection` class represents an abstract interface for communication with the LimeSDR device. Depending on the actual connection type (e.g., USB, PCIe), the implementation of these methods may vary.

These communication routines are typically used in advanced applications where the developer needs direct control over data transfer, such as implementing custom data protocols, optimizing for latency, or integrating with non-standard data processing pipelines.

---

### **Clock and Frequency Control**

This section covers functions related to managing the LimeSDR device's clock settings, including selecting clock sources, setting frequencies, and synchronizing hardware time. Proper clock configuration is essential for accurate timing and frequency stability in software-defined radio (SDR) applications.

---

1. **`LMS_SetClockSource(lms_device_t *device, const lms_name_t source)`**  
   * **Description**: Sets the clock source for the LimeSDR device, allowing selection between internal and external references.  
   * **Arguments**:  
     * `lms_device_t *device`: Handle to the LimeSDR device.  
     * `const lms_name_t source`: Name of the clock source to set. Typical options include `"Internal"` and `"External"`.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function when an external reference clock is required for synchronization with other equipment, or to switch back to the internal clock source.  
2. **`LMS_GetClockSource(lms_device_t *device, lms_name_t *source)`**  
   * **Description**: Retrieves the current clock source used by the device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_name_t *source`: Buffer to store the name of the current clock source.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Useful for verifying which clock source is currently active, ensuring that the device is using the expected reference.  
3. **`LMS_GetClockSources(lms_device_t *device, lms_name_t *list)`**  
   * **Description**: Retrieves a list of available clock sources supported by the device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `lms_name_t *list`: Array to store the names of available clock sources.  
   * **Returns**: `int` \- Number of clock sources available, or `-1` on error.  
   * **Usage**: Use this function to enumerate the clock sources before selecting one with `LMS_SetClockSource`.  
4. **`LMS_VCTCXORead(lms_device_t *device, uint16_t *value)`**  
   * **Description**: Reads the current DAC value controlling the VCTCXO (Voltage-Controlled Temperature-Compensated Crystal Oscillator) trim, which affects the reference clock frequency.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t *value`: Pointer to store the DAC value (usually in the range 0 to 65535).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Retrieve the current VCTCXO calibration setting to compensate for frequency offsets.  
5. **`LMS_VCTCXOWrite(lms_device_t *device, uint16_t value)`**  
   * **Description**: Writes a DAC value to control the VCTCXO trim, adjusting the reference clock frequency.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t value`: DAC value to set (usually in the range 0 to 65535).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Calibrate the reference clock frequency by correcting observed frequency offsets during operation.  
6. **`LMS_GetReferenceClockRate(lms_device_t *device, float_type *rate)`**  
   * **Description**: Retrieves the frequency of the reference clock used by the device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `float_type *rate`: Pointer to store the reference clock rate in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Verify the reference clock frequency, especially when using an external clock source.  
7. **`LMS_SetReferenceClockRate(lms_device_t *device, float_type rate)`**  
   * **Description**: Sets the expected frequency of the external reference clock.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `float_type rate`: Expected reference clock rate in Hz.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Specify the frequency of an external reference clock so the device can correctly configure its internal phase-locked loops (PLLs).  
8. **`LMS_SetHardwareTimestamp(lms_device_t *device, uint64_t timestamp)`**  
   * **Description**: Sets the hardware timestamp counter to a specific value.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint64_t timestamp`: Timestamp value to set.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Synchronize the hardware time counter with other devices or systems, allowing for coordinated operations.  
9. **`LMS_GetHardwareTimestamp(lms_device_t *device)`**  
   * **Description**: Retrieves the current value of the hardware timestamp counter.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
   * **Returns**: `uint64_t` \- Current timestamp value.  
   * **Usage**: Obtain the current hardware time for time-stamping received samples or scheduling transmissions.  
10. **`LMS_EnableTimestamp(lms_device_t *device, bool enable)`**  
    * **Description**: Enables or disables the hardware timestamp counter.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool enable`: Set to `true` to enable timestamping, or `false` to disable.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Control whether timestamps are generated and associated with samples during streaming operations.  
11. **`LMS_SetSampleRate(lms_device_t *device, float_type rate, size_t oversample)`**  
    * **Description**: Sets the sample rate for both RX and TX paths, along with an optional oversampling factor.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `float_type rate`: Desired sample rate in samples per second (Hz).  
      * `size_t oversample`: Oversampling factor (e.g., 1, 2, 4). If set to `0`, the API will select the best oversampling value.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Adjust the sample rate to match the bandwidth requirements of your application.  
12. **`LMS_SetSampleRateDir(lms_device_t *device, bool dir_tx, float_type rate, size_t oversample)`**  
    * **Description**: Sets the sample rate for either RX or TX paths independently, along with an optional oversampling factor.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Set to `true` for TX, `false` for RX.  
      * `float_type rate`: Desired sample rate in samples per second (Hz).  
      * `size_t oversample`: Oversampling factor.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use when different sample rates are needed for RX and TX paths.  
13. **`LMS_GetSampleRate(lms_device_t *device, bool dir_tx, size_t chan, float_type *host_Hz, float_type *rf_Hz)`**  
    * **Description**: Retrieves the current sample rate settings for the specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Set to `true` for TX, `false` for RX.  
      * `size_t chan`: Channel index.  
      * `float_type *host_Hz`: Pointer to store the sample rate between the device and the host (in Hz).  
      * `float_type *rf_Hz`: Pointer to store the sample rate between the RF frontend and the device's DSP core (in Hz).  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Confirm the actual sample rates being used.  
14. **`LMS_SetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type frequency)`**  
    * **Description**: Sets the Local Oscillator (LO) frequency for a specified RX or TX channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type frequency`: Desired LO frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Tune the device to a specific RF frequency.  
15. **`LMS_GetLOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *frequency)`**  
    * **Description**: Retrieves the current Local Oscillator (LO) frequency for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *frequency`: Pointer to store the LO frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the LO frequency setting.  
16. **`LMS_SetNCOFrequency(lms_device_t *device, bool dir_tx, size_t chan, const float_type *freq, float_type pho)`**  
    * **Description**: Configures the Numerically Controlled Oscillator (NCO) frequencies and phase offset for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Set to `true` for TX, `false` for RX.  
      * `size_t chan`: Channel index.  
      * `const float_type *freq`: Array of NCO frequencies (up to 16 entries).  
      * `float_type pho`: Phase offset in degrees.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Use for applications requiring precise frequency adjustments or frequency hopping.  
17. **`LMS_GetNCOFrequency(lms_device_t *device, bool dir_tx, size_t chan, float_type *freq, float_type *pho)`**  
    * **Description**: Retrieves the current NCO frequency and phase offset settings for a specified channel.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `bool dir_tx`: Direction flag.  
      * `size_t chan`: Channel index.  
      * `float_type *freq`: Array to store NCO frequencies.  
      * `float_type *pho`: Pointer to store the phase offset.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify NCO configurations.  
18. **`LMS_SetClockFreq(lms_device_t *device, LMS_CLOCK_ID clock_id, float_type freq)`**  
    * **Description**: Sets the frequency of a specified clock within the device.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `LMS_CLOCK_ID clock_id`: Identifier for the clock to set (e.g., `LMS_CLOCK_SXR`, `LMS_CLOCK_SXT`, `LMS_CLOCK_CGEN`).  
      * `float_type freq`: Desired clock frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Advanced function for directly setting clock frequencies, useful for custom configurations.  
19. **`LMS_GetClockFreq(lms_device_t *device, LMS_CLOCK_ID clock_id, float_type *freq)`**  
    * **Description**: Retrieves the frequency of a specified clock within the device.  
    * **Arguments**:  
      * `lms_device_t *device`: Device handle.  
      * `LMS_CLOCK_ID clock_id`: Identifier for the clock.  
      * `float_type *freq`: Pointer to store the clock frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on error.  
    * **Usage**: Verify the frequency settings of internal clocks.

---

### **Logging and Error Handling**

This section covers functions that help manage logging and error reporting within the LimeSuite SDK. Proper logging and error handling are essential for debugging and maintaining applications that interface with LimeSDR devices. These functions allow developers to retrieve detailed error messages, register custom log handlers, and control the verbosity of log outputs.

---

1. **`LMS_GetLastErrorMessage(void)`**  
   * **Description**: Retrieves the last error message generated by the LimeSuite API. This function provides additional context about why the most recent API function call failed.  
   * **Returns**: `const char*` \- A string containing the last error message.  
   * **Usage**: Useful for debugging purposes, especially when an API function returns an error code (`-1`). By calling this function, developers can obtain a human-readable description of the error, which can aid in diagnosing issues.  
   * **Example**:

     `if (LMS_SetSampleRate(device, sample_rate, 0) != 0) {`  
         `const char* error_msg = LMS_GetLastErrorMessage();`  
         `printf("Error setting sample rate: %s\n", error_msg);`  
     `}`  
       
2. **`LMS_RegisterLogHandler(LMS_LogHandler handler)`**  
   * **Description**: Registers a custom log handler function to capture and process log messages generated by the LimeSuite API. This allows applications to integrate LimeSuite logging with their own logging systems.  
   * **Arguments**:  
   * `LMS_LogHandler handler`: A function pointer to the custom log handler. The handler should match the signature:

     `void LogHandler(int level, const char *message);`  
     *   
       * `level`: The log level, indicating the severity of the message (e.g., error, warning, info).  
       * `message`: The log message string.  
   * **Usage**: By providing a custom log handler, developers can redirect log messages to their preferred logging framework or file. This is particularly useful for applications that require centralized logging or need to process log messages in a specific way.  
   * **Example**:

     `void MyLogHandler(int level, const char *message) {`  
         `// Custom processing of log messages`  
         `printf("LimeSuite Log [Level %d]: %s\n", level, message);`  
     `}`  
       
     `LMS_RegisterLogHandler(MyLogHandler);`  
       
3. **`LMS_SetLogLevel(int level)`**  
   * **Description**: Sets the logging verbosity level for the LimeSuite API. This controls which log messages are generated based on their severity.  
   * **Arguments**:  
     * `int level`: The desired log level. Common levels include:  
       * `LMS_LOG_FATAL`: Fatal errors.  
       * `LMS_LOG_ERROR`: Error messages.  
       * `LMS_LOG_WARNING`: Warning messages.  
       * `LMS_LOG_INFO`: Informational messages.  
       * `LMS_LOG_DEBUG`: Debug messages.  
       * `LMS_LOG_ALL`: All messages.  
       * `LMS_LOG_NONE`: No messages.  
   * **Usage**: Adjust the log level to filter out unnecessary messages or to enable detailed debugging output. Lowering the verbosity can improve performance in production environments.  
   * **Example**:

     `LMS_SetLogLevel(LMS_LOG_WARNING);`  
       
4. **`LMS_EnableLogTimestamp(bool enable)`**  
   * **Description**: Enables or disables the inclusion of timestamps in log messages.  
   * **Arguments**:  
     * `bool enable`: Set to `true` to include timestamps, `false` to omit them.  
   * **Usage**: Including timestamps can be helpful for tracing events over time, especially when debugging time-sensitive issues.  
   * **Example**:

     `LMS_EnableLogTimestamp(true);`  
       
5. **`LMS_GetLogLevel(void)`**  
   * **Description**: Retrieves the current logging verbosity level.  
   * **Returns**: `int` \- The current log level.  
   * **Usage**: Use this function to check the current log level, which can be useful in conditional logging or for restoring previous settings.  
   * **Example**:

     `int current_level = LMS_GetLogLevel();`  
     `printf("Current log level: %d\n", current_level);`  
6. **`LMS_Log(int level, const char *format, ...)`**  
   * **Description**: Generates a log message with the specified level and formatted message. This function can be used within applications to log custom messages through the LimeSuite logging system.  
   * **Arguments**:  
     * `int level`: The log level (e.g., `LMS_LOG_INFO`).  
     * `const char *format`: A printf-style format string.  
     * `...`: Additional arguments as required by the format string.  
   * **Usage**: Allows developers to integrate their own log messages into the LimeSuite logging framework, ensuring consistent logging behavior.  
   * **Example**:  
  
     `LMS_Log(LMS_LOG_INFO, "Custom message: Device initialized successfully.");`  
   *   
7. **`LMS_ErrorToString(int error_code)`**  
   * **Description**: Converts an error code returned by LimeSuite functions into a human-readable string.  
   * **Arguments**:  
     * `int error_code`: The error code to convert.  
   * **Returns**: `const char*` \- A string describing the error.  
   * **Usage**: Useful for interpreting error codes without having to call `LMS_GetLastErrorMessage`, especially when handling multiple errors.  
   * **Example**:

     `int ret = LMS_SetLOFrequency(device, false, 0, frequency);`  
     `if (ret != 0) {`  
         `printf("Error: %s\n", LMS_ErrorToString(ret));`  
     `}`  
       
8. **`LMS_ResetLog(void)`**  
   * **Description**: Resets the internal log buffer, clearing any stored log messages.  
   * **Usage**: Use this function to clear the log history, which can be helpful before starting a new operation or test to avoid confusion with old messages.  
   * **Example**:  
     `LMS_ResetLog();`  
       
9. **`LMS_SaveLog(const char *filename)`**  
   * **Description**: Saves the current log messages to a specified file.  
   * **Arguments**:  
     * `const char *filename`: Path to the file where the log will be saved.  
   * **Usage**: Allows for persistent storage of log messages, which can be useful for post-analysis or when sharing logs for support purposes.  
   * **Example**:

     `LMS_SaveLog("lime_log.txt");`  
10. **`LMS_PrintLog(void)`**  
    * **Description**: Prints all current log messages to the standard output.  
    * **Usage**: Useful for displaying the entire log history in the console, which can assist in debugging.  
    * **Example**:

      `LMS_PrintLog();`  
      

---

### **LimeRFE-Specific Functions**

This section covers functions specifically designed for controlling the LimeRFE (Radio Front-End) module when used in conjunction with LimeSDR devices. The LimeRFE module provides additional capabilities such as filtering, amplification, and switching for various frequency bands, making it suitable for applications like amateur radio and cellular communication.

---

1. **`RFE_Open(const char* serialport, lms_device_t *dev)`**  
   * **Description**: Opens a connection to a LimeRFE device, initializing it for use.  
   * **Arguments**:  
     * `const char* serialport`: The serial port name or identifier for the LimeRFE device. If the LimeRFE is connected via USB, this would be the COM port (e.g., `"COM3"` on Windows or `"/dev/ttyUSB0"` on Linux). If set to `nullptr`, the function attempts to auto-detect the LimeRFE connected to the provided `lms_device_t`.  
     * `lms_device_t *dev`: Handle to the LimeSDR device with which the LimeRFE is associated.  
   * **Returns**: `rfe_dev_t*` \- A handle to the LimeRFE device on success, or `nullptr` on failure.  
   * **Usage**: Before performing any operations with the LimeRFE, you must open a connection to it. This function initializes communication between the host and the LimeRFE module.  
   * **Example**:

     `rfe_dev_t* rfe = RFE_Open(nullptr, device);`  
     `if (rfe == nullptr) {`  
         `printf("Failed to open LimeRFE\n");`  
     `}`  
       
2. **`RFE_Close(rfe_dev_t* rfe)`**  
   * **Description**: Closes the connection to the LimeRFE device and releases associated resources.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: Handle to the LimeRFE device obtained from `RFE_Open`.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Should be called when the LimeRFE is no longer needed to properly close the connection and free resources.  
   * **Example**:

     `if (RFE_Close(rfe) != 0) {`  
         `printf("Failed to close LimeRFE\n");`  
     `}`  
       
3. **`RFE_Configure(rfe_dev_t* rfe, int mode, int module, int in_port, int out_port, int attenuation, bool notch_on)`**  
   * **Description**: Configures the LimeRFE module settings, including operational mode, input/output ports, attenuation, and notch filter.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `int mode`: Operational mode, selecting the frequency band or application. Possible values include predefined modes like `RFE_MODE_NONE`, `RFE_MODE_HAM_160`, `RFE_MODE_HAM_80`, etc.  
     * `int module`: Module selection, e.g., `RFE_MODULE_NONE`, `RFE_MODULE_BAND1`, etc.  
     * `int in_port`: Input port selection. Possible values are `RFE_PORT_1`, `RFE_PORT_2`, etc.  
     * `int out_port`: Output port selection. Possible values are similar to `in_port`.  
     * `int attenuation`: Attenuation level in decibels (dB). Valid range is typically from 0 to 31 dB.  
     * `bool notch_on`: Enable or disable the notch filter (`true` to enable, `false` to disable).  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Use this function to set up the LimeRFE for a specific frequency band or application. The configuration adjusts internal switches, filters, and amplifiers to suit the desired operation.  
   * **Example**:

     `int mode = RFE_MODE_HAM_40; // 40-meter amateur band`  
     `int module = RFE_MODULE_BAND1;`  
     `int in_port = RFE_PORT_1;`  
     `int out_port = RFE_PORT_1;`  
     `int attenuation = 10; // 10 dB attenuation`  
     `bool notch_on = false;`  
       
     `if (RFE_Configure(rfe, mode, module, in_port, out_port, attenuation, notch_on) != 0) {`  
         `printf("Failed to configure LimeRFE\n");`  
     `}`  
       
4. **`RFE_Reset(rfe_dev_t* rfe)`**  
   * **Description**: Resets the LimeRFE module to its default settings.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Use this function to clear any configurations and return the LimeRFE to its default state.  
   * **Example**:

     `if (RFE_Reset(rfe) != 0) {`  
         `printf("Failed to reset LimeRFE\n");`  
     `}`  
       
5. **`RFE_GetInfo(rfe_dev_t* rfe, rfe_dev_info_t* info)`**  
   * **Description**: Retrieves information about the LimeRFE device, such as firmware version, hardware version, and serial number.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `rfe_dev_info_t* info`: Pointer to a structure where the device information will be stored.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Structure `rfe_dev_info_t` Fields**:  
     * `char hardwareVersion[64]`: Hardware version string.  
     * `char firmwareVersion[64]`: Firmware version string.  
     * `char serialNumber[64]`: Device serial number.  
     * Additional fields may include manufacturing date, calibration data, etc.  
   * **Usage**: Useful for verifying the device's firmware and hardware versions, which can be important for compatibility and troubleshooting.  
   * **Example**:

     `rfe_dev_info_t info;`  
     `if (RFE_GetInfo(rfe, &info) == 0) {`  
         `printf("LimeRFE Info:\n");`  
         `printf("Hardware Version: %s\n", info.hardwareVersion);`  
         `printf("Firmware Version: %s\n", info.firmwareVersion);`  
         `printf("Serial Number: %s\n", info.serialNumber);`  
     `} else {`  
         `printf("Failed to get LimeRFE info\n");`  
     `}`  
       
6. **`RFE_SetPower(rfe_dev_t* rfe, bool enable)`**  
   * **Description**: Enables or disables the power supply to the LimeRFE module.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `bool enable`: Set to `true` to turn on the power, `false` to turn it off.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Control the power state of the LimeRFE, which can be useful for power management or when performing maintenance tasks.  
   * **Example**:

     `if (RFE_SetPower(rfe, true) != 0) {`  
         `printf("Failed to enable LimeRFE power\n");`  
     `}`  
       
7. **`RFE_ReadADC(rfe_dev_t* rfe, uint8_t adc_id, float* value)`**  
   * **Description**: Reads the value from one of the LimeRFE's ADC (Analog-to-Digital Converter) channels.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `uint8_t adc_id`: ADC channel identifier.  
     * `float* value`: Pointer to store the read value.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Used to read sensor data from the LimeRFE, such as voltage or current measurements, which can be useful for monitoring and diagnostics.  
   * **Example**:

     `float adc_value;`  
     `uint8_t adc_id = 0; // Assuming ADC channel 0`  
     `if (RFE_ReadADC(rfe, adc_id, &adc_value) == 0) {`  
         `printf("ADC Channel %d Value: %f\n", adc_id, adc_value);`  
     `} else {`  
         `printf("Failed to read ADC value\n");`  
     `}`  
       
8. **`RFE_WriteGPIO(rfe_dev_t* rfe, uint8_t gpio_num, bool value)`**  
   * **Description**: Sets the state of a specified GPIO pin on the LimeRFE module.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `uint8_t gpio_num`: GPIO pin number.  
     * `bool value`: `true` to set the pin high, `false` to set it low.  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Control external devices connected to the LimeRFE via GPIO pins.  
   * **Example**:

     `uint8_t gpio_num = 2; // GPIO pin 2`  
     `if (RFE_WriteGPIO(rfe, gpio_num, true) != 0) {`  
         `printf("Failed to set GPIO pin %d\n", gpio_num);`  
     `}`  
       
9. **`RFE_ReadGPIO(rfe_dev_t* rfe, uint8_t gpio_num, bool* value)`**  
   * **Description**: Reads the state of a specified GPIO pin on the LimeRFE module.  
   * **Arguments**:  
     * `rfe_dev_t* rfe`: LimeRFE device handle.  
     * `uint8_t gpio_num`: GPIO pin number.  
     * `bool* value`: Pointer to store the pin state (`true` for high, `false` for low).  
   * **Returns**: `int` \- `0` on success, `-1` on failure.  
   * **Usage**: Monitor the state of GPIO pins, which may be connected to external sensors or devices.  
   * **Example**:

     `uint8_t gpio_num = 2; // GPIO pin 2`  
     `bool pin_state;`  
     `if (RFE_ReadGPIO(rfe, gpio_num, &pin_state) == 0) {`  
         `printf("GPIO pin %d state: %s\n", gpio_num, pin_state ? "High" : "Low");`  
     `} else {`  
         `printf("Failed to read GPIO pin %d\n", gpio_num);`  
     `}`  
       
10. **`RFE_UpdateFirmware(rfe_dev_t* rfe, const char* firmware_path, rfe_prog_callback_t callback)`**  
    * **Description**: Updates the firmware of the LimeRFE module using a provided firmware file.  
    * **Arguments**:  
      * `rfe_dev_t* rfe`: LimeRFE device handle.  
      * `const char* firmware_path`: Path to the firmware file to be uploaded.  
      * `rfe_prog_callback_t callback`: Optional callback function for progress updates during the firmware update process.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Keep the LimeRFE's firmware up to date by applying updates provided by the manufacturer, which may include bug fixes or new features.  
    * **Example**:

      `int progress_callback(int percent, const char* msg) {`  
          `printf("Firmware Update Progress: %d%% - %s\n", percent, msg);`  
          `return 0; // Return 0 to continue, non-zero to abort`  
      `}`  
        
      `if (RFE_UpdateFirmware(rfe, "path/to/firmware.hex", progress_callback) != 0) {`  
          `printf("Failed to update LimeRFE firmware\n");`  
      `}`  
        
11. **`RFE_SetFrequency(rfe_dev_t* rfe, double frequency)`**  
    * **Description**: Sets the operating frequency of the LimeRFE module.  
    * **Arguments**:  
      * `rfe_dev_t* rfe`: LimeRFE device handle.  
      * `double frequency`: Desired frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Adjust the frequency at which the LimeRFE operates, within the supported range of the module.  
    * **Example**:

      `double frequency = 7e6; // 7 MHz for 40-meter amateur band`  
      `if (RFE_SetFrequency(rfe, frequency) != 0) {`  
          `printf("Failed to set LimeRFE frequency\n");`  
      `}`  
        
12. **`RFE_GetFrequencyRange(rfe_dev_t* rfe, rfe_frequency_range_t* range)`**  
    * **Description**: Retrieves the supported frequency range of the LimeRFE module.  
    * **Arguments**:  
      * `rfe_dev_t* rfe`: LimeRFE device handle.  
      * `rfe_frequency_range_t* range`: Pointer to a structure to receive the frequency range information.  
    * **Structure `rfe_frequency_range_t` Fields**:  
      * `double min`: Minimum frequency in Hz.  
      * `double max`: Maximum frequency in Hz.  
    * **Returns**: `int` \- `0` on success, `-1` on failure.  
    * **Usage**: Use this function to determine the valid frequency range before setting frequencies with `RFE_SetFrequency`.  
    * **Example**:

      `rfe_frequency_range_t freq_range;`  
      `if (RFE_GetFrequencyRange(rfe, &freq_range) == 0) {`  
          `printf("LimeRFE Frequency Range: %.0f Hz - %.0f Hz\n", freq_range.min, freq_range.max);`  
      `} else {`  
          `printf("Failed to get LimeRFE frequency range\n");`  
      `}`  
        
      ---

**Note:** The actual function names, arguments, and behavior may vary depending on the version of the LimeSuite API and the LimeRFE module. Always refer to the official LimeSuite and LimeRFE documentation for the most accurate and up-to-date information.

* 

---

### **Synchronization**

Functions for ensuring consistency between software and hardware states.

---

1. **`LMS_Synchronize(lms_device_t *device, bool toChip)`**  
   * **Description**: Synchronizes the register values between the software cache and the hardware registers of the LimeSDR device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle obtained from `LMS_Open`.  
     * `bool toChip`: If `true`, the function writes the register values from the software cache to the hardware (i.e., updates the hardware registers with the cached values). If `false`, it reads the register values from the hardware into the software cache.  
   * **Returns**: `int` \- Returns `0` on success, or `-1` on error.  
   * **Usage**: This function is essential for maintaining consistency between the software and hardware states of the device, especially when multiple software components might be interacting with the device or when direct register access has been used. It ensures that any changes made in the software cache are reflected in the hardware registers, and vice versa.  
   * **Example**:  
   
     `// Synchronize software cache to hardware`  
     `if (LMS_Synchronize(device, true) != 0) {`  
         `printf("Error synchronizing to hardware: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     `// Synchronize hardware to software cache`  
     `if (LMS_Synchronize(device, false) != 0) {`  
         `printf("Error synchronizing from hardware: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     ---

     ### **Additional Functions**

Miscellaneous functions that provide extra capabilities.

---

1. **`LMS_UploadWFM(lms_device_t *device, const void **samples, uint8_t chCount, size_t sample_count, int format)`**  
   * **Description**: Uploads a waveform to the LimeSDR device's onboard memory for playback. This allows the device to generate signals autonomously without continuous streaming from the host.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `const void **samples`: An array of pointers to sample buffers, one for each channel. Each buffer contains the waveform samples to upload.  
     * `uint8_t chCount`: The number of channels to upload waveforms for (maximum of 2).  
     * `size_t sample_count`: The number of samples per channel. Must be a multiple of 4\.  
     * `int format`: The format of the samples. Possible values:  
       * `LMS_FMT_F32` (0): 32-bit floating-point format.  
       * `LMS_FMT_I16` (1): 16-bit integer format.  
       * `LMS_FMT_I12` (2): 12-bit integer format (packed into 16 bits).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to upload custom waveforms to the device for transmission. This is particularly useful for generating repetitive signals or test patterns without the need for continuous data streaming from the host computer, which can reduce USB or PCIe bandwidth usage.  
   * **Example**:

     `// Generate a sine wave`  
     `size_t sample_count = 1024;`  
     `float *waveform = malloc(sample_count * sizeof(float) * 2); // I and Q samples`  
       
     `for (size_t i = 0; i < sample_count; ++i) {`  
         `float phase = (2.0 * M_PI * i) / sample_count;`  
         `waveform[2*i] = sin(phase);     // I sample`  
         `waveform[2*i + 1] = cos(phase); // Q sample`  
     `}`  
       
     `const void *samples[] = { waveform };`  
     `if (LMS_UploadWFM(device, samples, 1, sample_count, LMS_FMT_F32) != 0) {`  
         `printf("Error uploading waveform: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     `free(waveform);`  
       
2. **`LMS_EnableTxWFM(lms_device_t *device, unsigned chan, bool active)`**  
   * **Description**: Enables or disables the transmission of the uploaded waveform on a specified TX channel.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `unsigned chan`: The index of the TX channel to control (usually `0` or `1`).  
     * `bool active`: Set to `true` to enable waveform playback, or `false` to disable it.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: After uploading a waveform using `LMS_UploadWFM`, use this function to start or stop its transmission. This function controls whether the device transmits the uploaded waveform on the specified channel.  
   *   
     `// Enable waveform transmission on TX channel 0`  
     `if (LMS_EnableTxWFM(device, 0, true) != 0) {`  
         `printf("Error enabling waveform transmission: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     `// ... transmission occurs ...`  
       
     `// Disable waveform transmission`  
     `if (LMS_EnableTxWFM(device, 0, false) != 0) {`  
         `printf("Error disabling waveform transmission: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     ---

     ### **Advanced Control and Configuration**

Functions for detailed configuration of filters and custom parameters.

---

1. **`LMS_SetGFIRCoeff(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, const float_type *coef, size_t count)`**  
   * **Description**: Sets the coefficients for a specified General Finite Impulse Response (GFIR) filter in the device's DSP chain. This allows for custom filter designs to be implemented in the signal path.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag; set to `true` for TX (transmit), `false` for RX (receive).  
     * `size_t chan`: Channel index (usually `0` or `1`).  
     * `lms_gfir_t filt`: The GFIR filter to configure. Possible values:  
       * `LMS_GFIR1`  
       * `LMS_GFIR2`  
       * `LMS_GFIR3`  
     * `const float_type *coef`: An array of filter coefficients.  
     * `size_t count`: The number of coefficients in the `coef` array.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to customize the filtering behavior of the device by specifying your own filter coefficients. The GFIR filters are programmable FIR filters that can be used to implement custom filters such as low-pass, high-pass, band-pass, or any arbitrary FIR filter.  
   * **Example**:

     `// Define a simple low-pass filter`  
     `float_type coef[16] = { /* coefficients */ };`  
       
     `if (LMS_SetGFIRCoeff(device, false, 0, LMS_GFIR1, coef, 16) != 0) {`  
         `printf("Error setting GFIR coefficients: %s\n", LMS_GetLastErrorMessage());`  
   * `}`  
   *   
2. **`LMS_EnableGFIR(lms_device_t *device, bool dir_tx, size_t chan, lms_gfir_t filt, bool enabled)`**  
   * **Description**: Enables or disables a specified GFIR filter in the signal processing chain.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag.  
     * `size_t chan`: Channel index.  
     * `lms_gfir_t filt`: The GFIR filter to enable or disable.  
     * `bool enabled`: Set to `true` to enable the filter, or `false` to disable it.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: After configuring a GFIR filter with `LMS_SetGFIRCoeff`, use this function to enable or disable the filter in the signal path. Disabling unnecessary filters can improve performance by reducing processing overhead.  
   * **Example**:

     `// Enable GFIR1 on RX channel 0`  
     `if (LMS_EnableGFIR(device, false, 0, LMS_GFIR1, true) != 0) {`  
         `printf("Error enabling GFIR filter: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     `// Disable GFIR1`  
     `if (LMS_EnableGFIR(device, false, 0, LMS_GFIR1, false) != 0) {`  
         `printf("Error disabling GFIR filter: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     ---

     ### **Utility and Device-Specific Configuration Functions**

Additional functions for device management.

---

1. **`LMS_EnableCache(lms_device_t *device, bool enable)`**  
   * **Description**: Enables or disables the caching of register values to improve performance by reducing redundant reads and writes to the device.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool enable`: Set to `true` to enable caching, or `false` to disable it.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Enabling cache can significantly improve performance in applications where registers are frequently accessed with the same values. However, caching should be disabled if registers are expected to change due to external factors or when precise synchronization is required.  
   * **Example**:

     `// Enable register caching`  
     `if (LMS_EnableCache(device, true) != 0) {`  
         `printf("Error enabling cache: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     `// Perform operations...`  
       
     `// Disable register caching`  
     `if (LMS_EnableCache(device, false) != 0) {`  
         `printf("Error disabling cache: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
2. **`LMS_GetDeviceInfo(lms_device_t *device)`**  
   * **Description**: Retrieves detailed information about the connected LimeSDR device, including device name, serial number, hardware version, firmware version, gateware version, and more.  
   * **Returns**: `const lms_dev_info_t*` \- Pointer to a structure containing device information.  
   * **Usage**: Useful for verifying the device details, ensuring compatibility, and for logging or display purposes.  
   * **Example**:

     `const lms_dev_info_t *info = LMS_GetDeviceInfo(device);`  
     `if (info) {`  
         `printf("Device Info:\n");`  
         `printf("Device Name: %s\n", info->deviceName);`  
         `printf("Serial Number: %s\n", info->boardSerialNumber);`  
         `printf("Hardware Version: %s\n", info->hardwareVersion);`  
         `printf("Firmware Version: %s\n", info->firmwareVersion);`  
         `printf("Gateware Version: %s\n", info->gatewareVersion);`  
     `} else {`  
         `printf("Error retrieving device info: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
     ---

     ### **Clock Functions**

Functions for managing the device's clock settings.

---

1. **`LMS_GetSampleRateRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Retrieves the supported range of sample rates for the specified RX or TX path.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `bool dir_tx`: Direction flag; `true` for TX, `false` for RX.  
     * `lms_range_t *range`: Pointer to an `lms_range_t` structure where the minimum, maximum, and step values will be stored.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to determine the valid sample rates that can be set on the device, ensuring that your application operates within the hardware capabilities.  
   * **Example**:

     `lms_range_t range;`  
     `if (LMS_GetSampleRateRange(device, false, &range) == 0) {`  
         `printf("RX Sample Rate Range: Min = %f Hz, Max = %f Hz, Step = %f Hz\n",`  
                `range.min, range.max, range.step);`  
     `} else {`  
         `printf("Error retrieving sample rate range: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
2. **`LMS_GetLOFrequencyRange(lms_device_t *device, bool dir_tx, lms_range_t *range)`**  
   * **Description**: Retrieves the supported range of Local Oscillator (LO) frequencies for the specified RX or TX path.  
   * **Arguments**:  
     * Same as above.  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to determine the valid frequency range for tuning the device, ensuring that your desired operating frequency is within the hardware limits.  
   * **Example**:

     `lms_range_t range;`  
     `if (LMS_GetLOFrequencyRange(device, true, &range) == 0) {`  
         `printf("TX LO Frequency Range: Min = %f Hz, Max = %f Hz, Step = %f Hz\n",`  
                `range.min, range.max, range.step);`  
     `} else {`  
         `printf("Error retrieving LO frequency range: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
3. **`LMS_VCTCXORead(lms_device_t *device, uint16_t *val)`**  
   * **Description**: Reads the current DAC value controlling the VCTCXO (Voltage-Controlled Temperature-Compensated Crystal Oscillator) trim, which affects the reference clock frequency.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t *val`: Pointer to store the DAC value (usually in the range 0 to 65535).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to retrieve the current VCTCXO calibration value, which can help verify the calibration status of the reference clock.  
   * **Example**:

     `uint16_t dac_value;`  
     `if (LMS_VCTCXORead(device, &dac_value) == 0) {`  
         `printf("VCTCXO DAC Value: %u\n", dac_value);`  
     `} else {`  
         `printf("Error reading VCTCXO DAC value: %s\n", LMS_GetLastErrorMessage());`  
     `}`  
       
4. **`LMS_VCTCXOWrite(lms_device_t *device, uint16_t val)`**  
   * **Description**: Writes a DAC value to control the VCTCXO trim, adjusting the reference clock frequency.  
   * **Arguments**:  
     * `lms_device_t *device`: Device handle.  
     * `uint16_t val`: DAC value to set (usually in the range 0 to 65535).  
   * **Returns**: `int` \- `0` on success, `-1` on error.  
   * **Usage**: Use this function to calibrate the reference clock frequency by correcting observed frequency offsets during operation. Adjusting the VCTCXO can improve frequency accuracy.  
   * **Example**:

     `uint16_t new_dac_value = 32000; // Example value`  
     `if (LMS_VCTCXOWrite(device, new_dac_value) != 0) {`  
         `printf("Error writing VCTCXO DAC value: %s\n", LMS_GetLastErrorMessage());`  
     `} else {`  
         `printf("VCTCXO DAC value set to: %u\n", new_dac_value);`  
     `}`  
       
     
