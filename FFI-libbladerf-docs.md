### **1\. Device Management**

These functions cover device connection, initialization, and information handling.

* **`bladerf_open`**: Opens a device using an identifier string. Takes pointers to the device handle and device identifier as parameters.  
* **`bladerf_open_with_devinfo`**: Similar to `bladerf_open`, but uses a `bladerf_devinfo` struct for more detailed initialization.  
* **`bladerf_close`**: Closes a device, releasing associated resources.  
* **`bladerf_get_devinfo`**: Fills a `bladerf_devinfo` structure with device info.  
* **`bladerf_get_device_speed`**: Retrieves device speed as `bladerf_dev_speed`.

### **2\. Firmware and FPGA Management**

These functions manage firmware loading, FPGA flashing, and associated configurations.

* **`bladerf_load_fpga`**: Loads an FPGA image from a file. Parameters include device handle and file path.  
* **`bladerf_flash_fpga`**: Writes the FPGA image to onboard flash memory.  
* **`bladerf_flash_firmware`**: Loads new firmware from a specified file.  
* **`bladerf_erase_stored_fpga`**: Erases the stored FPGA image from flash.  
* **`bladerf_jump_to_bootloader`**: Sends the device into bootloader mode for firmware updates.

### **3\. Frequency and Gain Control**

Handles frequency tuning, gain adjustments, and channel configuration.

* **`bladerf_set_frequency`**: Sets the RF frequency for a specified channel.  
* **`bladerf_get_frequency`**: Gets the current frequency for a channel.  
* **`bladerf_set_gain`**: Configures gain for a specific channel.  
* **`bladerf_get_gain`**: Retrieves the current gain setting.  
* **`bladerf_set_rf_port`**: Configures the RF port for different modes.

### **4\. Sample Rate, Bandwidth, and Tuning**

Controls sample rates, bandwidth, and other tuning modes.

* **`bladerf_set_sample_rate`**: Sets sample rate for a specified channel, adjusting based on hardware limits.  
* **`bladerf_get_sample_rate`**: Gets the current sample rate.  
* **`bladerf_set_bandwidth`**: Adjusts channel bandwidth.  
* **`bladerf_set_tuning_mode`**: Sets the tuning mode, such as `bladerf_tuning_mode` parameters.

### **5\. Data Streaming and Synchronization**

Includes stream initialization, submission of buffers, and control over data transfers.

* **`bladerf_init_stream`**: Initializes a data stream with parameters including buffer count, format, and callback function.  
* **`bladerf_stream`**: Starts data streaming on the defined channel layout.  
* **`bladerf_submit_stream_buffer`**: Submits a buffer to a stream.  
* **`bladerf_set_stream_timeout`**: Sets timeout for data transfer in streaming mode.  
* **`bladerf_get_stream_timeout`**: Retrieves the current streaming timeout setting.

### **6\. Calibration and Gain Tables**

Loads and manages calibration data, particularly for gain adjustments.

* **`bladerf_load_gain_calibration`**: Loads gain calibration for a channel from a specified file.  
* **`bladerf_enable_gain_calibration`**: Enables or disables gain calibration.  
* **`bladerf_get_gain_range`**: Retrieves the gain range supported by a channel.

### **7\. Loopback and Debug**

Functions that enable loopback testing and debugging of configurations.

* **`bladerf_set_loopback`**: Enables loopback mode for diagnostic purposes.  
* **`bladerf_get_loopback`**: Retrieves the current loopback mode.  
* **`bladerf_is_loopback_mode_supported`**: Checks if a loopback mode is supported by the device.

### **8\. Low-Level Access and Control**

Handles low-level access to GPIO, SPI, flash memory, and configuration registers.

* **`bladerf_config_gpio_read`**: Reads the GPIO configuration register.  
* **`bladerf_config_gpio_write`**: Writes data to the GPIO configuration register.  
* **`bladerf_write_flash`**: Writes data to flash memory at a specified page.  
* **`bladerf_read_flash`**: Reads data from flash memory.  
* **`bladerf_erase_flash`**: Erases a specified range in flash memory.  
* **`bladerf_get_vctcxo_trim`**: Retrieves the current VCTCXO trim DAC setting.

### **9\. Error Handling and Logging**

Provides functions for handling error states and logging configurations.

* **`bladerf_strerror`**: Converts error codes to human-readable strings.  
* **`bladerf_log_set_verbosity`**: Sets the verbosity level of logging for debug output.

---

### **Section 1: Device Management**

These functions primarily deal with initializing, connecting to, and managing the state of bladeRF devices.

#### **1\. `bladerf_open`**

* **Description**: Opens a bladeRF device based on a device identifier string. If successful, it returns a handle to the device.  
* **Parameters**:  
  * `struct bladerf **dev`: Pointer to the device handle that will be populated upon successful open.  
  * `const char *dev_id`: Device identifier string specifying which device to open. This can be a serial number, USB bus/address, or wildcard ("\*") to open any available device.

#### **2\. `bladerf_open_with_devinfo`**

* **Description**: Opens a bladeRF device using a populated `bladerf_devinfo` structure, allowing for more specific control over device selection.  
* **Parameters**:  
  * `struct bladerf **opened_device`: Pointer to the device handle that will be populated.  
  * `struct bladerf_devinfo *devinfo`: A pre-populated `bladerf_devinfo` structure containing device details (e.g., serial number, USB bus, and address).

#### **3\. `bladerf_close`**

* **Description**: Closes a previously opened device, releasing resources associated with it.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle to be closed.

#### **4\. `bladerf_get_devinfo`**

* **Description**: Retrieves the device information of an open bladeRF device, populating a `bladerf_devinfo` structure.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `struct bladerf_devinfo *info`: Pointer to a `bladerf_devinfo` structure where device information will be stored.

#### **5\. `bladerf_get_device_speed`**

* **Description**: Gets the device speed (e.g., high-speed, super-speed).  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: The device speed as an enumerated value of type `bladerf_dev_speed`.

#### **6\. `bladerf_get_device_list`**

* **Description**: Retrieves a list of available bladeRF devices. Allocates and populates a list of `bladerf_devinfo` structures for each detected device.  
* **Parameters**:  
  * `struct bladerf_devinfo **devices`: Pointer to an array of `bladerf_devinfo` structures that will be populated with information about each device.  
* **Returns**: Integer indicating the number of devices found or an error code if no devices are available.

#### **7\. `bladerf_free_device_list`**

* **Description**: Frees the memory associated with a list of devices retrieved by `bladerf_get_device_list`.  
* **Parameters**:  
  * `struct bladerf_devinfo *devices`: Pointer to the device list to be freed.

---

### **Section 2: Firmware and FPGA Management**

These functions facilitate loading FPGA images, flashing firmware, managing the bootloader, and erasing FPGA images from onboard storage.

#### **1\. `bladerf_load_fpga`**

* **Description**: Loads an FPGA image onto the device from a specified file. This operation is critical for configuring the FPGA to the required state for device operation.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *fpga_file`: Path to the FPGA image file to be loaded.

#### **2\. `bladerf_flash_fpga`**

* **Description**: Writes the FPGA image to the device’s onboard flash memory for autoloading on boot.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *fpga_file`: Path to the FPGA image file to be flashed to onboard memory.

#### **3\. `bladerf_erase_stored_fpga`**

* **Description**: Erases the stored FPGA image from the device’s onboard flash memory, preventing autoload on subsequent boots.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.

#### **4\. `bladerf_flash_firmware`**

* **Description**: Flashes new firmware to the device. Typically used for firmware updates.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *firmware_file`: Path to the firmware file to be flashed.

#### **5\. `bladerf_jump_to_bootloader`**

* **Description**: Places the device into bootloader mode, enabling firmware updates via the FX3 bootloader.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: Integer status code indicating success or if the bootloader transition is unsupported.

#### **6\. `bladerf_get_bootloader_list`**

* **Description**: Retrieves a list of devices currently in bootloader mode.  
* **Parameters**:  
  * `struct bladerf_devinfo **devices`: Pointer to a list of device information structures that will be populated with devices found in bootloader mode.

#### **7\. `bladerf_load_fw_from_bootloader`**

* **Description**: Loads firmware onto a device that is in bootloader mode, facilitating recovery or initial firmware installation.  
* **Parameters**:  
  * `const char *device_identifier`: Device identifier string for the device in bootloader mode.  
  * `bladerf_backend backend`: Specifies the backend type (e.g., USB).  
  * `uint8_t bus`: USB bus number where the device is located.  
  * `uint8_t addr`: USB address of the device.  
  * `const char *file`: Path to the firmware file to be loaded.

#### **8\. `bladerf_get_fw_log`**

* **Description**: Retrieves the firmware log, often used for debugging or troubleshooting device firmware-related issues.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *filename`: Path to the file where the firmware log should be written. If `NULL`, the log is printed to `stdout`.

---

### **Section 3: Frequency and Gain Control**

These routines allow you to control the RF frequency, configure gain for different channels, and set the RF port mode.

#### **1\. `bladerf_set_frequency`**

* **Description**: Sets the operating frequency for a specified RF channel on the bladeRF device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the frequency (e.g., `BLADERF_CHANNEL_RX(0)` or `BLADERF_CHANNEL_TX(0)`).  
  * `bladerf_frequency frequency`: Desired frequency in Hz.

#### **2\. `bladerf_get_frequency`**

* **Description**: Retrieves the current operating frequency of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the frequency.  
  * `bladerf_frequency *frequency`: Pointer to a variable that will hold the frequency in Hz.

#### **3\. `bladerf_set_gain`**

* **Description**: Configures the gain for a specified RF channel. Gain settings are generally specific to the device model.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the gain.  
  * `int gain`: Gain level, typically an integer value based on the device's gain range.

#### **4\. `bladerf_get_gain`**

* **Description**: Retrieves the current gain setting of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the gain.  
  * `int *gain`: Pointer to an integer that will hold the current gain setting.

#### **5\. `bladerf_set_gain_mode`**

* **Description**: Sets the gain mode (e.g., manual or automatic) for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the gain mode.  
  * `bladerf_gain_mode mode`: Gain mode to be set, such as `BLADERF_GAIN_MGC` (manual) or `BLADERF_GAIN_AGC` (automatic).

#### **6\. `bladerf_get_gain_mode`**

* **Description**: Retrieves the current gain mode (manual or automatic) of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the gain mode.  
  * `bladerf_gain_mode *mode`: Pointer to a variable that will hold the current gain mode.

#### **7\. `bladerf_get_gain_range`**

* **Description**: Gets the valid gain range for a specified RF channel. This provides the minimum and maximum gain values supported by the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the gain range.  
  * `const struct bladerf_range **range`: Pointer to a `bladerf_range` structure that will hold the gain range details.

#### **8\. `bladerf_set_rf_port`**

* **Description**: Configures the RF port for the specified channel to a particular mode (e.g., TX or RX). This may vary depending on device capabilities.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the port configuration.  
  * `const char *port`: Name of the port configuration, such as `"TX"` or `"RX"`.

#### **9\. `bladerf_get_rf_port`**

* **Description**: Retrieves the current RF port configuration for the specified channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the port configuration.  
  * `const char **port`: Pointer to a variable that will hold the name of the current port configuration.

---

### **Section 4: Sample Rate, Bandwidth, and Tuning**

These functions allow you to control the sample rate, bandwidth, and other tuning-related settings, including oversampling where applicable.

#### **1\. `bladerf_set_sample_rate`**

* **Description**: Sets the sample rate for a specified RF channel. The function returns the actual sample rate applied, which may differ slightly from the requested value depending on hardware constraints.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the sample rate.  
  * `bladerf_sample_rate rate`: Desired sample rate in samples per second.  
  * `bladerf_sample_rate *actual`: Pointer to a variable that will hold the actual sample rate applied.

#### **2\. `bladerf_get_sample_rate`**

* **Description**: Retrieves the current sample rate for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the sample rate.  
  * `bladerf_sample_rate *rate`: Pointer to a variable that will hold the sample rate in samples per second.

#### **3\. `bladerf_get_sample_rate_range`**

* **Description**: Gets the supported sample rate range for a specified RF channel, returning the minimum and maximum rates.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the sample rate range.  
  * `const struct bladerf_range **range`: Pointer to a `bladerf_range` structure that will be populated with the supported sample rate range.

#### **4\. `bladerf_set_rational_sample_rate`**

* **Description**: Sets a rational sample rate for a specified RF channel. This allows finer control by specifying a fractional rate.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the rational sample rate.  
  * `struct bladerf_rational_rate *rate`: Pointer to a `bladerf_rational_rate` structure containing the desired sample rate as a rational fraction.  
  * `struct bladerf_rational_rate *actual`: Pointer to a structure that will hold the actual sample rate applied as a rational fraction.

#### **5\. `bladerf_get_rational_sample_rate`**

* **Description**: Retrieves the current rational sample rate for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the rational sample rate.  
  * `struct bladerf_rational_rate *rate`: Pointer to a `bladerf_rational_rate` structure that will hold the sample rate as a rational fraction.

#### **6\. `bladerf_set_bandwidth`**

* **Description**: Sets the bandwidth for a specified RF channel. This function allows precise control of bandwidth for optimizing signal quality.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to set the bandwidth.  
  * `bladerf_bandwidth bandwidth`: Desired bandwidth in Hz.  
  * `bladerf_bandwidth *actual`: Pointer to a variable that will hold the actual bandwidth applied.

#### **7\. `bladerf_get_bandwidth`**

* **Description**: Retrieves the current bandwidth setting of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the bandwidth.  
  * `bladerf_bandwidth *bandwidth`: Pointer to a variable that will hold the current bandwidth in Hz.

#### **8\. `bladerf_get_bandwidth_range`**

* **Description**: Retrieves the supported bandwidth range for a specified RF channel, providing both the minimum and maximum bandwidth values.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the bandwidth range.  
  * `const struct bladerf_range **range`: Pointer to a `bladerf_range` structure that will be populated with the bandwidth range details.

#### **9\. `bladerf_set_tuning_mode`**

* **Description**: Configures the tuning mode for the device. Tuning mode can influence how frequencies are set and can vary by hardware capabilities.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_tuning_mode mode`: Tuning mode to be set (e.g., `BLADERF_TUNING_MODE_HOST`, `BLADERF_TUNING_MODE_FPGA`, etc.).

#### **10\. `bladerf_get_tuning_mode`**

* **Description**: Retrieves the current tuning mode of the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_tuning_mode *mode`: Pointer to a variable that will hold the current tuning mode.

---

### **Section 5: Data Streaming and Synchronization**

These functions manage streaming data to and from the bladeRF device, providing control over buffers, timeouts, and synchronization.

#### **1\. `bladerf_init_stream`**

* **Description**: Initializes a data stream on the device, allocating resources as necessary and setting up buffers for data transfers.  
* **Parameters**:  
  * `struct bladerf_stream **stream`: Pointer to the stream structure to initialize.  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_stream_cb callback`: Callback function for handling data buffers during streaming.  
  * `void ***buffers`: Pointer to an array of buffers used for streaming. If `NULL`, the library allocates buffers.  
  * `size_t num_buffers`: Number of buffers to use in the stream.  
  * `bladerf_format format`: Data format of the samples (e.g., `BLADERF_FORMAT_SC16_Q11`).  
  * `size_t samples_per_buffer`: Number of samples in each buffer.  
  * `size_t num_transfers`: Number of asynchronous transfers to use during streaming.  
  * `void *data`: Additional user data to pass to the callback function.

#### **2\. `bladerf_stream`**

* **Description**: Begins streaming data using the previously initialized stream and the specified layout.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the initialized stream structure.  
  * `bladerf_channel_layout layout`: Specifies the channel layout, such as single or dual channel (e.g., `BLADERF_RX_X1` for single-channel receive).

#### **3\. `bladerf_submit_stream_buffer`**

* **Description**: Submits a buffer to the stream for data transfer. This function is used in a non-blocking way to manage buffer submissions.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure.  
  * `void *buffer`: Pointer to the buffer to be submitted for transfer.  
  * `unsigned int timeout_ms`: Timeout in milliseconds for buffer submission.

#### **4\. `bladerf_submit_stream_buffer_nb`**

* **Description**: Submits a buffer to the stream without a timeout, functioning as a non-blocking submission. Typically used for continuous streaming.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure.  
  * `void *buffer`: Pointer to the buffer to be submitted.

#### **5\. `bladerf_deinit_stream`**

* **Description**: Deinitializes a previously initialized stream, freeing associated resources and closing any open transfers.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure to deinitialize.

#### **6\. `bladerf_set_stream_timeout`**

* **Description**: Sets the timeout for data transfers on a specified streaming direction (transmit or receive).  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Streaming direction (`BLADERF_RX` for receive or `BLADERF_TX` for transmit).  
  * `unsigned int timeout`: Timeout in milliseconds for data transfers.

#### **7\. `bladerf_get_stream_timeout`**

* **Description**: Retrieves the current timeout setting for data transfers in the specified streaming direction.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Streaming direction (`BLADERF_RX` or `BLADERF_TX`).  
  * `unsigned int *timeout`: Pointer to a variable that will hold the current timeout in milliseconds.

#### **8\. `bladerf_sync_config`**

* **Description**: Configures synchronous streaming on the device, specifying buffer, format, and timeout settings for the channels.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel_layout layout`: Specifies the channel layout (e.g., `BLADERF_RX_X1`).  
  * `bladerf_format format`: Sample format for the streaming data (e.g., `BLADERF_FORMAT_SC16_Q11`).  
  * `unsigned int num_buffers`: Number of buffers to allocate for streaming.  
  * `unsigned int buffer_size`: Size of each buffer in samples.  
  * `unsigned int num_transfers`: Number of asynchronous transfers to use.  
  * `unsigned int stream_timeout`: Timeout in milliseconds for data transfers.

#### **9\. `bladerf_sync_tx`**

* **Description**: Transmits a block of samples in synchronous mode, blocking until the transfer completes or times out.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const void *samples`: Pointer to the sample data to transmit.  
  * `unsigned int num_samples`: Number of samples to transmit.  
  * `struct bladerf_metadata *metadata`: Pointer to metadata (can be `NULL` if unused).  
  * `unsigned int timeout_ms`: Timeout in milliseconds for the transfer.

#### **10\. `bladerf_sync_rx`**

* **Description**: Receives a block of samples in synchronous mode, blocking until the specified number of samples has been received or a timeout occurs.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `void *samples`: Pointer to the buffer where received samples will be stored.  
  * `unsigned int num_samples`: Number of samples to receive.  
  * `struct bladerf_metadata *metadata`: Pointer to metadata (can be `NULL` if unused).  
  * `unsigned int timeout_ms`: Timeout in milliseconds for the transfer.

#### **11\. `bladerf_get_timestamp`**

* **Description**: Retrieves the timestamp of the last sample transmitted or received, depending on the direction specified.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Direction (`BLADERF_RX` or `BLADERF_TX`) for which to retrieve the timestamp.  
  * `bladerf_timestamp *timestamp`: Pointer to a variable that will hold the timestamp.

#### **12\. `bladerf_interleave_stream_buffer`**

* **Description**: Interleaves samples in a buffer according to a specified channel layout and format.  
* **Parameters**:  
  * `bladerf_channel_layout layout`: Channel layout for interleaving (e.g., `BLADERF_RX_X1`).  
  * `bladerf_format format`: Data format of samples (e.g., `BLADERF_FORMAT_SC16_Q11`).  
  * `unsigned int buffer_size`: Size of the buffer in samples.  
  * `void *samples`: Pointer to the sample buffer to interleave.

#### **13\. `bladerf_deinterleave_stream_buffer`**

* **Description**: De-interleaves samples in a buffer according to a specified channel layout and format.  
* **Parameters**:  
  * `bladerf_channel_layout layout`: Channel layout for de-interleaving.  
  * `bladerf_format format`: Data format of samples.  
  * `unsigned int buffer_size`: Size of the buffer in samples.  
  * `void *samples`: Pointer to the sample buffer to de-interleave.

---

### **Section 6: Calibration and Gain Tables**

These functions are crucial for managing gain calibration data, adjusting gain settings across different frequencies, and loading calibration tables for the bladeRF device.

#### **1\. `bladerf_load_gain_calibration`**

* **Description**: Loads a gain calibration file for a specified RF channel. This file contains gain values adjusted for specific frequencies.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to load the gain calibration.  
  * `const char *cal_file_loc`: Path to the calibration file to load.

#### **2\. `bladerf_enable_gain_calibration`**

* **Description**: Enables or disables gain calibration for a specified RF channel, allowing the device to apply pre-loaded gain calibration settings.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to enable/disable gain calibration.  
  * `bool en`: Boolean value indicating whether to enable (`true`) or disable (`false`) gain calibration.

#### **3\. `bladerf_print_gain_calibration`**

* **Description**: Outputs the loaded gain calibration data for a specified channel, with an option to include individual entries from the calibration table.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to print the gain calibration data.  
  * `bool with_entries`: Boolean indicating whether to print detailed entries from the calibration table.

#### **4\. `bladerf_get_gain_calibration`**

* **Description**: Retrieves a pointer to the gain calibration table structure for a specified RF channel. This allows programmatic access to the calibration data.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the gain calibration.  
  * `const struct bladerf_gain_cal_tbl **tbl`: Pointer to a variable that will be set to the address of the gain calibration table structure.

#### **5\. `bladerf_get_gain_target`**

* **Description**: Calculates and returns the target gain for a specified RF channel, accounting for calibration adjustments.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to calculate the gain target.  
  * `int *gain_target`: Pointer to a variable where the calculated gain target will be stored.

---

### **Section 7: Loopback and Debug**

These functions allow you to enable and retrieve loopback configurations, which are useful for testing and debugging the bladeRF device's performance under various configurations.

#### **1\. `bladerf_set_loopback`**

* **Description**: Sets the loopback mode for a specified bladeRF device. Loopback modes are useful for diagnostic testing where the device’s output is fed back to its input.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback l`: Loopback mode to set, which can be one of several supported modes (e.g., `BLADERF_LB_NONE`, `BLADERF_LB_FIRMWARE`, etc.).

#### **2\. `bladerf_get_loopback`**

* **Description**: Retrieves the current loopback mode setting of the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback *l`: Pointer to a variable that will hold the current loopback mode of the device.

#### **3\. `bladerf_is_loopback_mode_supported`**

* **Description**: Checks if a specified loopback mode is supported on the current device. This function is useful for determining compatibility with diagnostic configurations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback mode`: The loopback mode to check for support.  
* **Returns**: Boolean value indicating whether the mode is supported (`true` for supported, `false` otherwise).

#### **4\. `bladerf_get_loopback_modes`**

* **Description**: Retrieves a list of supported loopback modes for the device, providing an overview of available diagnostic configurations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const struct bladerf_loopback_modes **modes`: Pointer to a variable that will hold the address of a structure listing supported loopback modes.

---

### **Section 8: Low-Level Access and Control**

These functions provide direct access to the device’s low-level hardware controls, including GPIO, SPI, flash memory, and configuration registers. They are essential for advanced configuration and custom hardware interactions.

#### **1\. `bladerf_config_gpio_read`**

* **Description**: Reads the current value of the configuration GPIO register.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t *val`: Pointer to a variable that will hold the GPIO register value upon successful read.

#### **2\. `bladerf_config_gpio_write`**

* **Description**: Writes a specified value to the configuration GPIO register, controlling various GPIO-related settings.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t val`: Value to write to the GPIO register.

#### **3\. `bladerf_read_flash`**

* **Description**: Reads data from the device’s flash memory starting from a specified page.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint8_t *buf`: Pointer to a buffer where the read data will be stored.  
  * `uint32_t page`: Starting page number in flash memory.  
  * `uint32_t count`: Number of pages to read.

#### **4\. `bladerf_write_flash`**

* **Description**: Writes data to the device’s flash memory starting from a specified page.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const uint8_t *buf`: Pointer to a buffer containing data to write.  
  * `uint32_t page`: Starting page number in flash memory.  
  * `uint32_t count`: Number of pages to write.

#### **5\. `bladerf_erase_flash`**

* **Description**: Erases a specified range in the device’s flash memory.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t erase_block`: Starting erase block number.  
  * `uint32_t count`: Number of blocks to erase.

#### **6\. `bladerf_set_vctcxo_tamer_mode`**

* **Description**: Sets the VCTCXO (Voltage-Controlled Temperature-Compensated Crystal Oscillator) tamer mode, which controls the external frequency reference.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_vctcxo_tamer_mode mode`: Mode to set, such as `BLADERF_VCTCXO_TAMER_DISABLED`, `BLADERF_VCTCXO_TAMER_1_PPS`, etc.

#### **7\. `bladerf_get_vctcxo_tamer_mode`**

* **Description**: Retrieves the current VCTCXO tamer mode of the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_vctcxo_tamer_mode *mode`: Pointer to a variable that will hold the current VCTCXO tamer mode.

#### **8\. `bladerf_get_vctcxo_trim`**

* **Description**: Reads the VCTCXO trim DAC setting, which is used for fine-tuning the frequency.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint16_t *trim`: Pointer to a variable that will hold the DAC trim value.

#### **9\. `bladerf_trim_dac_write`**

* **Description**: Writes a specified trim value to the DAC, controlling the VCTCXO for precise frequency adjustments.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint16_t trim`: DAC trim value to write.

#### **10\. `bladerf_read_trigger`**

* **Description**: Reads the state of a trigger signal on a specified RF channel. Trigger signals can be used for synchronizing external events.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: Channel on which to read the trigger signal.  
  * `bladerf_trigger_signal trigger`: Trigger signal to read.  
  * `uint8_t *val`: Pointer to a variable that will hold the trigger state.

#### **11\. `bladerf_write_trigger`**

* **Description**: Writes a state to a trigger signal on a specified RF channel, allowing control over synchronization events.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: Channel on which to write the trigger signal.  
  * `bladerf_trigger_signal trigger`: Trigger signal to configure.  
  * `uint8_t val`: Value to write to the trigger signal register.

#### **12\. `bladerf_wishbone_master_read`**

* **Description**: Reads data from the Wishbone master interface, used for direct register access on compatible devices.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t addr`: Address to read from.  
  * `uint32_t *data`: Pointer to a variable that will hold the read data.

#### **13\. `bladerf_wishbone_master_write`**

* **Description**: Writes data to the Wishbone master interface for direct register access.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t addr`: Address to write to.  
  * `uint32_t data`: Data to write.

---

### **Section 9: Error Handling and Logging**

These functions are used to interpret error codes, retrieve error messages, and set the verbosity of logging output for troubleshooting and diagnostic purposes.

#### **1\. `bladerf_strerror`**

* **Description**: Converts a `libbladeRF` error code into a human-readable string, providing a description of the error.  
* **Parameters**:  
  * `int error`: The error code to convert.  
* **Returns**: A constant string describing the error.

#### **2\. `bladerf_errstr`**

* **Description**: Retrieves a detailed error message for a specific `libbladeRF` error, useful for error diagnostics.  
* **Parameters**:  
  * `int error_code`: The error code for which to retrieve the error string.  
  * `char *err_str`: Pointer to a character buffer where the detailed error message will be written.  
  * `size_t err_str_size`: Size of the `err_str` buffer to ensure the error message does not exceed buffer limits.

#### **3\. `bladerf_log_set_verbosity`**

* **Description**: Sets the verbosity level of logging output for the `libbladeRF` library. This controls the amount of debugging information output by the library during operation.  
* **Parameters**:  
  * `bladerf_log_level level`: Desired logging verbosity level, which can be one of several predefined levels such as `BLADERF_LOG_LEVEL_SILENT`, `BLADERF_LOG_LEVEL_INFO`, `BLADERF_LOG_LEVEL_DEBUG`, etc.

#### **4\. `bladerf_log_get_verbosity`**

* **Description**: Retrieves the current logging verbosity level of the `libbladeRF` library.  
* **Parameters**:  
  * `bladerf_log_level *level`: Pointer to a variable where the current verbosity level will be stored.

#### **5\. `bladerf_get_last_error`**

* **Description**: Retrieves the last error code encountered by the device for further analysis.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `int *error`: Pointer to a variable that will store the last error code encountered.

---

### **Section 10: Miscellaneous Utility Functions**

These routines provide utility features, including setting device-specific configuration values, checking capabilities, and retrieving device information.

#### **1\. `bladerf_get_serial`**

* **Description**: Retrieves the serial number of the bladeRF device as a human-readable string.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `char *serial`: Pointer to a character buffer where the serial number string will be stored.  
  * `size_t len`: Length of the provided buffer to avoid overflow.

#### **2\. `bladerf_get_fpga_size`**

* **Description**: Retrieves the FPGA size supported by the device, which can vary by model.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_fpga_size *size`: Pointer to a variable where the FPGA size will be stored (e.g., `BLADERF_FPGA_40KLE`, `BLADERF_FPGA_115KLE`).

#### **3\. `bladerf_get_fpga_bytes`**

* **Description**: Returns the number of bytes required for the FPGA configuration, useful for determining FPGA file compatibility.  
* **Parameters**:  
  * `bladerf_fpga_size size`: FPGA size enumerator value.  
* **Returns**: Integer representing the size in bytes.

#### **4\. `bladerf_is_fpga_configured`**

* **Description**: Checks whether an FPGA image is currently loaded and configured on the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: Boolean value indicating whether the FPGA is configured (`true`) or not (`false`).

#### **5\. `bladerf_get_version`**

* **Description**: Retrieves version information for the library and associated components.  
* **Parameters**:  
  * `struct bladerf_version *version`: Pointer to a `bladerf_version` structure that will be populated with version information for the `libbladeRF` library, FPGA, and firmware.

#### **6\. `bladerf_get_board_name`**

* **Description**: Retrieves a human-readable name for the board type, which can vary by bladeRF model.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `char *name`: Pointer to a character buffer where the board name will be stored.  
  * `size_t len`: Length of the buffer to ensure name fits.

#### **7\. `bladerf_check_fw_compat`**

* **Description**: Checks if the device's firmware version is compatible with the current library version, ensuring that all features will function as expected.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `struct bladerf_version *required`: Pointer to a `bladerf_version` structure specifying the minimum required firmware version.  
* **Returns**: Integer status code indicating compatibility or an error code if incompatible.
