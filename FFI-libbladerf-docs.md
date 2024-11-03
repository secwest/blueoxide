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
