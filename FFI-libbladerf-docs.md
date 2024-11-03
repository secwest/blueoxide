# BladeRF

## 

## **Introduction to `libbladeRF` Documentation**

### **Overview of Hardware Capabilities**

### The bladeRF software-defined radio (SDR) platform, developed by Nuand, provides a versatile RF transceiver with high performance and extensive configurability. The bladeRF hardware is available in multiple models, including the bladeRF x40, bladeRF x115, and bladeRF 2.0 micro.    

| Feature | bladeRF x40/x115 | bladeRF 2.0 Micro xA4/xA9 | bladeRF x16 |
| ----- | ----- | ----- | ----- |
| **Frequency Range** | 300 MHz – 3.8 GHz  | 47 MHz – 6 GHz  | 47 MHz – 6 GHz  |
| **ADC/DAC Sample Rate** | Up to 40 MSPS  | Up to 61.44 MSPS  | Up to 122.88 MSPS |
| **ADC/DAC Resolution** | 12 bits | 12 bits  | 16 bits  |
| **FPGA** | Altera Cyclone IV: 40KLE (x40) or 115KLE (x115)  | Intel Cyclone V: 49KLE (xA4) or 301KLE (xA9)  | Intel Arria 10: 1150KLE  |
| **RF Bandwidth** | Up to 28 MHz | Up to 56 MHz  | Up to 61.44 MHz  |
| **MIMO Channels** | 2x2  | 2x2  | 4x4  |
| **USB Interface** | USB 3.0 SuperSpeed  | USB 3.0 SuperSpeed  | USB 3.1 Gen 2  |
| **Dimensions** | 5" x 3.5"  | 2.5" x 4.6"  | 6.5" x 4"  |
| **Operating Temperature** | 0°C to 70°C ([nuand.com](https://www.nuand.com/bladerf-1/)) | 0°C to 70°C (standard); \-40°C to 85°C (Thermal variants) | 0°C to 70°C  |
| **Power Supply** | USB bus-powered; optional external power  | USB bus-powered; optional external power  | External power supply required  |
| **Expansion Options** | GPIO, JTAG, expansion boards for additional interfaces | GPIO, JTAG, bias-tee for active antennas  | GPIO, JTAG, PCIe interface  |

### **Notes:**

* ### The **bladeRF x40/x115** models are the original versions, offering solid performance suitable for a wide range of applications.

* ### The **bladeRF 2.0 Micro xA4/xA9** models are the next-generation devices, featuring an extended frequency range, higher sample rates, and more powerful FPGAs, making them suitable for more demanding applications.

* ### The **bladeRF x16** is the latest model, offering significant enhancements over previous versions, including higher ADC/DAC resolution, increased sample rates, and expanded MIMO capabilities, making it ideal for complex and high-performance SDR applications.

* ### The **xA9** variant, with its 301KLE FPGA, provides significantly more resources for custom signal processing tasks compared to the **xA4**'s 49KLE FPGA.

* ### The **Thermal** variants of the bladeRF 2.0 Micro models are designed for extended temperature ranges, making them suitable for more challenging environmental conditions.

### 

###   Key capabilities of the bladeRF hardware include:

* ### **Frequency Range**: Supports a wide range of frequencies, from 47 MHz to 6 GHz (depending on the model), covering a broad spectrum for a variety of applications such as telecommunications, satellite, and IoT.

* ### **Channel Configurations**: bladeRF hardware supports multiple configurations for single and dual channels, enabling MIMO (Multiple Input Multiple Output) operations in advanced models.

* ### **Sample Rates**: The devices support adjustable sample rates ranging from kilohertz up to tens of megahertz, with certain models capable of sample rates over 60 MSPS (mega-samples per second).

* ### **FPGA Options**: bladeRF models offer different FPGA sizes, from 40K to 115K logic elements, with some models allowing dynamic reconfiguration of the FPGA for custom processing and acceleration.

* ### **Loopback Modes**: Supports several loopback configurations (such as RF loopback to specific LNAs and firmware-based loopback) for testing and diagnostic purposes.

* ### **Gain Control and Calibration**: Configurable gain settings with manual and automatic gain control (AGC), supported by calibration tables for fine-tuning across different frequencies and conditions.

### **Software Stack and `libbladeRF`**

### The `libbladeRF` library is the primary interface for interacting with bladeRF hardware, providing low-level control over hardware parameters, configuration options, and data streaming functions. It serves as a versatile API for software-defined radio applications, supporting complex use cases such as signal processing, modulation/demodulation, and RF experimentation. Key components in the `libbladeRF` stack include:

* ### **Device Initialization and Management**: Functions to open, close, and manage multiple bladeRF devices.

* ### **Configuration and Control**: Routines for setting frequencies, sample rates, gain, and other RF parameters.

* ### **Data Streaming**: Stream management functions to handle real-time data transfers to and from the device.

* ### **Calibration and Diagnostics**: Gain calibration, loopback modes, and diagnostic functions for troubleshooting and optimizing device performance.

### **Compilation and Setup Instructions**

#### **Prerequisites**

### To compile and install `libbladeRF`, ensure you have the following dependencies:

* ### **CMake**: A build system generator that simplifies compilation.

* ### **libusb**: The USB library required for device communication.

* ### **FPGA and Firmware Images**: Depending on your device model, you’ll need the correct FPGA and firmware images for full functionality. These images are typically available from the Nuand website.

#### **Compilation Steps**

1. ### **Download `libbladeRF` Source Code**: Obtain the latest version of `libbladeRF` from the Nuand GitHub repository or official download sources.

2. ### **Configure the Build**:

### Create a build directory:  `mkdir build && cd build`

### Use `cmake` to configure the project. For example:  `cmake .. -DCMAKE_BUILD_TYPE=Release`

3. ### **Compile**:

### Compile the library using `make`:  `make`

4. ### **Install the Library**:

### Install the library to the system:  `sudo make install`

* ### Run `sudo ldconfig` to refresh the library path.

#### **Additional Setup: FPGA and Firmware**

### After installation, load the FPGA image and update the firmware (if necessary) to ensure your bladeRF device is fully operational.

### **Load FPGA Image**: Use `bladeRF-cli` or a custom application using `libbladeRF` to load the FPGA image:  `bladeRF-cli -l /path/to/fpga_image.rbf`

* ### **Firmware Update**: Ensure the firmware is compatible with the library version. If required, update the firmware by placing the device in bootloader mode and using the provided firmware image.

### **Coding with `libbladeRF`**

### To code for `libbladeRF`, include the `bladerf.h` header in your application and link against `libbladeRF`. Here’s a minimal example to open a device, configure a frequency, and close the device:

### `#include <libbladeRF.h>`

### 

### `int main() {`

###     `struct bladerf *dev;`

###     `int status;`

### 

###     `// Open the first available device`

###     `status = bladerf_open(&dev, NULL);`

###     `if (status != 0) {`

###         `fprintf(stderr, "Failed to open bladeRF device: %s\n", bladerf_strerror(status));`

###         `return status;`

###     `}`

### 

###     `// Set RX frequency to 2.4 GHz`

###     `status = bladerf_set_frequency(dev, BLADERF_CHANNEL_RX(0), 2400000000);`

###     `if (status != 0) {`

###         `fprintf(stderr, "Failed to set frequency: %s\n", bladerf_strerror(status));`

###     `}`

### 

###     `// Close device`

###     `bladerf_close(dev);`

###     `return 0;`

### `}`

### 

### **Best Practices and Tips**

* ### **Error Handling**: Use `bladerf_strerror` and `bladerf_get_last_error` to retrieve detailed error messages, especially during configuration and streaming operations.

* ### **Data Streaming**: For applications requiring high-throughput data handling, tune buffer sizes and the number of buffers in `bladerf_init_stream` to optimize performance.

* ### **Version Compatibility**: Use `bladerf_check_fw_compat` to ensure firmware compatibility with your version of `libbladeRF`.

* ### **Loopback Testing**: For initial development and testing, use loopback modes to confirm data integrity without the need for external RF signals.

### This document serves as a reference for developing SDR applications with bladeRF hardware and `libbladeRF`. By following the setup steps and utilizing the outlined API functions, you can fully leverage the capabilities of the bladeRF platform in RF experimentation, signal processing, and beyond.

### 

### 

### 
### 

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


# Library Details

### **Section 1: Device Management**

These routines allow for opening, closing, and listing bladeRF devices, as well as obtaining basic device information. They are foundational for any bladeRF device interaction.

#### **1\. `bladerf_open`**    `int bladerf_open(struct bladerf **dev, const char *dev_id);`

* **Description**: Opens a bladeRF device based on a provided identifier string. This function initializes the device and allocates a handle for further operations.  
* **Parameters**:  
  * `struct bladerf **dev`: A pointer to a device handle pointer. This will be populated with the device handle if the function succeeds.  
  * `const char *dev_id`: Identifier string for selecting a device. This can be a serial number, USB bus/address, or `"*"` to open any available device.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_NODEV` if no devices are available, or `BLADERF_ERR_NO_MEM` if memory allocation fails).  
* **Important Notes**:  
  * If `dev_id` is `"*"` or `NULL`, the first available device is opened.  
  * Multiple devices can be opened simultaneously by calling `bladerf_open` with different `dev_id` strings.

#### **2\. `bladerf_open_with_devinfo`**

    
`int bladerf_open_with_devinfo(struct bladerf **opened_device, struct bladerf_devinfo *devinfo);`

* **Description**: Opens a bladeRF device using detailed device information in the form of a `bladerf_devinfo` structure, which provides more control over device selection.  
* **Parameters**:  
  * `struct bladerf **opened_device`: Pointer to a device handle pointer to be populated on success.  
  * `struct bladerf_devinfo *devinfo`: Pre-populated `bladerf_devinfo` structure with details such as serial number, USB bus, and address.  
* **Returns**: `0` on success, negative error code on failure (e.g., `BLADERF_ERR_NODEV` if the specified device isn’t found).  
* **Developer Tips**: Useful for cases where multiple devices are connected, and specific selection criteria (e.g., USB port or address) are required.

#### **3\. `bladerf_close`**

`void bladerf_close(struct bladerf *dev);`

* **Description**: Closes an open bladeRF device and releases associated resources.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle to be closed.  
* **Returns**: None.  
* **Important Notes**:  
  * Always call `bladerf_close` to avoid resource leaks when the device is no longer needed.

#### **4\. `bladerf_get_devinfo`**

`int bladerf_get_devinfo(struct bladerf *dev, struct bladerf_devinfo *info);`

* **Description**: Retrieves information about a connected bladeRF device and populates a `bladerf_devinfo` structure with details like serial number and USB configuration.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `struct bladerf_devinfo *info`: Pointer to a `bladerf_devinfo` structure where device information will be stored.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_INVAL` if the device handle is invalid).  
* **Useful Fields in `bladerf_devinfo`**:  
  * `serial`: Device serial number as a string.  
  * `backend`: The backend used, such as USB or `BLADERF_BACKEND_ANY`.  
  * `bus` and `address`: USB bus and address numbers, respectively, which are useful for differentiating devices.

#### **5\. `bladerf_get_device_speed`**

`bladerf_dev_speed bladerf_get_device_speed(struct bladerf *dev);`

* **Description**: Retrieves the USB communication speed of the bladeRF device, which could impact data throughput.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: Enumerated value of type `bladerf_dev_speed`, such as:  
  * `BLADERF_DEVICE_SPEED_UNKNOWN`: Device speed could not be determined.  
  * `BLADERF_DEVICE_SPEED_HIGH`: High-speed USB (USB 2.0).  
  * `BLADERF_DEVICE_SPEED_SUPER`: SuperSpeed USB (USB 3.0).  
* **Developer Tips**: Use this function to confirm USB speed when troubleshooting performance issues.

#### **6\. `bladerf_get_device_list`**

`int bladerf_get_device_list(struct bladerf_devinfo **devices);`

* **Description**: Retrieves a list of available bladeRF devices and allocates an array of `bladerf_devinfo` structures.  
* **Parameters**:  
  * `struct bladerf_devinfo **devices`: Pointer to an array of `bladerf_devinfo` structures. This array will be dynamically allocated and must be freed with `bladerf_free_device_list`.  
* **Returns**: Number of devices found on success, or `BLADERF_ERR_NODEV` if no devices are available.  
* **Developer Notes**:  
  * Use `bladerf_get_device_list` to enumerate devices and then open specific devices based on selection criteria.  
  * Check for memory allocation errors when using this function, especially in low-memory environments.

#### **7\. `bladerf_free_device_list`**

`void bladerf_free_device_list(struct bladerf_devinfo *devices);`

* **Description**: Frees memory allocated by `bladerf_get_device_list`.  
* **Parameters**:  
  * `struct bladerf_devinfo *devices`: Pointer to the list of devices returned by `bladerf_get_device_list`.  
* **Returns**: None.  
* **Important Notes**: Always call `bladerf_free_device_list` after using the device list to avoid memory leaks.

---

### **Section 2: Firmware and FPGA Management**

These routines are used to load and manage firmware and FPGA configurations, which are crucial for setting up the bladeRF device.

#### **1\. `bladerf_load_fpga`**

`int bladerf_load_fpga(struct bladerf *dev, const char *fpga_file);`

* **Description**: Loads an FPGA image from a specified file path. This function programs the device’s FPGA with a new configuration, enabling it to perform the desired SDR operations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *fpga_file`: File path to the FPGA image.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_IO` if the file could not be read, `BLADERF_ERR_INVAL` if an invalid parameter is passed).  
* **Important Notes**:  
  * Ensure the FPGA file is compatible with the device model.  
  * Loading an FPGA image takes several seconds, during which the device may become unresponsive.

#### **2\. `bladerf_flash_fpga`**

`int bladerf_flash_fpga(struct bladerf *dev, const char *fpga_file);`

* **Description**: Flashes the specified FPGA image to the device’s onboard flash memory. This enables the FPGA image to load automatically on device boot.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *fpga_file`: File path to the FPGA image.  
* **Returns**: `0` on success, negative error code on failure (e.g., `BLADERF_ERR_IO` if there’s an I/O error or `BLADERF_ERR_UNSUPPORTED` if the device does not support flash).  
* **Developer Tips**: Use this function for devices that require persistent FPGA configurations. Flashes can wear out over repeated writes, so use sparingly.

#### **3\. `bladerf_erase_stored_fpga`**

`int bladerf_erase_stored_fpga(struct bladerf *dev);`

* **Description**: Erases the stored FPGA image from onboard flash memory, effectively disabling automatic FPGA configuration at startup.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_UNSUPPORTED` if the device lacks flash memory).  
* **Important Notes**: This operation is non-reversible; re-flashing would be required to re-enable the FPGA configuration on boot.

#### **4\. `bladerf_flash_firmware`**

`int bladerf_flash_firmware(struct bladerf *dev, const char *firmware_file);`

* **Description**: Updates the device’s firmware by writing the specified firmware file to flash memory. Firmware updates may introduce new features or bug fixes.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *firmware_file`: File path to the firmware file.  
* **Returns**: `0` on success, negative error code on failure (e.g., `BLADERF_ERR_IO` if the file cannot be read).  
* **Developer Notes**:  
  * Firmware updates require a compatible firmware file for the specific bladeRF model.  
  * Avoid interrupting the device during firmware flashing to prevent corruption.

#### **5\. `bladerf_jump_to_bootloader`**

`int bladerf_jump_to_bootloader(struct bladerf *dev);`

* **Description**: Forces the device into bootloader mode, enabling recovery operations such as firmware reinstallation.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: `0` on success, or `BLADERF_ERR_UNSUPPORTED` if the device does not support bootloader access.  
* **Important Notes**:  
  * Devices in bootloader mode may not respond to regular commands. A re-flash of firmware may be required to restore standard operation.

#### **6\. `bladerf_get_bootloader_list`**

`int bladerf_get_bootloader_list(struct bladerf_devinfo **devices);`

* **Description**: Retrieves a list of devices currently in bootloader mode.  
* **Parameters**:  
  * `struct bladerf_devinfo **devices`: Pointer to an array of `bladerf_devinfo` structures. The list is dynamically allocated and should be freed with `bladerf_free_device_list`.  
* **Returns**: Number of devices in bootloader mode on success, or `BLADERF_ERR_NODEV` if no devices are found.  
* **Developer Tips**:  
  * Use this function to confirm device presence in bootloader mode before attempting firmware flashing.  
  * Memory management is essential; remember to free the device list after use.

#### **7\. `bladerf_load_fw_from_bootloader`**

`int bladerf_load_fw_from_bootloader(const char *device_identifier, bladerf_backend backend, uint8_t bus, uint8_t addr, const char *file);`

* **Description**: Loads firmware onto a device that is in bootloader mode, enabling recovery or initial firmware installation.  
* **Parameters**:  
  * `const char *device_identifier`: Device identifier string for the target device in bootloader mode.  
  * `bladerf_backend backend`: Specifies the backend (e.g., `BLADERF_BACKEND_USB`).  
  * `uint8_t bus`: USB bus number where the device is located.  
  * `uint8_t addr`: USB address of the device.  
  * `const char *file`: Path to the firmware file.  
* **Returns**: `0` on success, or a negative error code if loading fails (e.g., `BLADERF_ERR_NODEV` if the device is not found).  
* **Developer Notes**:  
  * Use for firmware recovery on devices that are unresponsive in normal mode.  
  * Requires careful matching of device identifiers when multiple devices are connected.

#### **8\. `bladerf_get_fw_log`**

`int bladerf_get_fw_log(struct bladerf *dev, const char *filename);`

* **Description**: Retrieves the firmware log and saves it to a file, useful for troubleshooting issues with firmware or device initialization.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const char *filename`: Path to the file where the firmware log will be saved. If `NULL`, the log is printed to `stdout`.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Important Notes**:  
  * Use this function for diagnostics and debugging when firmware initialization fails.  
  * Logs may provide details about hardware or compatibility issues impacting firmware behavior.

---

### **Section 3: Frequency and Gain Control**

These functions manage the RF frequency and gain settings, allowing fine-tuned control over transmission and reception channels on the bladeRF device.

#### **1\. `bladerf_set_frequency`**

`int bladerf_set_frequency(struct bladerf *dev, bladerf_channel ch, bladerf_frequency frequency);`

* **Description**: Sets the operating frequency for a specified RF channel on the bladeRF device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: The RF channel (e.g., `BLADERF_CHANNEL_RX(0)` or `BLADERF_CHANNEL_TX(0)`).  
  * `bladerf_frequency frequency`: Desired frequency in Hz.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_INVAL` if the frequency is out of range).  
* **Developer Notes**:  
  * Ensure that the requested frequency falls within the supported range of the bladeRF model being used.  
  * Changing frequency may have minor settling delays.

#### **2\. `bladerf_get_frequency`**

`int bladerf_get_frequency(struct bladerf *dev, bladerf_channel ch, bladerf_frequency *frequency);`

* **Description**: Retrieves the current frequency setting of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel (e.g., `BLADERF_CHANNEL_RX(0)` or `BLADERF_CHANNEL_TX(0)`).  
  * `bladerf_frequency *frequency`: Pointer to a variable where the frequency in Hz will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Useful Tips**: This function is often used to verify the frequency setting after calling `bladerf_set_frequency`.

#### **3\. `bladerf_set_gain`**

`int bladerf_set_gain(struct bladerf *dev, bladerf_channel ch, int gain);`

* **Description**: Configures the gain for a specified RF channel, allowing adjustments in dB.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel (e.g., `BLADERF_CHANNEL_RX(0)`).  
  * `int gain`: Gain level in dB.  
* **Returns**: `0` on success, or a negative error code (e.g., `BLADERF_ERR_RANGE` if gain is outside valid range).  
* **Developer Tips**:  
  * Use in conjunction with `bladerf_get_gain_range` to ensure the gain value falls within device limits.  
  * Gain settings can directly impact signal-to-noise ratio (SNR) and dynamic range.

#### **4\. `bladerf_get_gain`**

`int bladerf_get_gain(struct bladerf *dev, bladerf_channel ch, int *gain);`

* **Description**: Retrieves the current gain setting of a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel (e.g., `BLADERF_CHANNEL_RX(0)`).  
  * `int *gain`: Pointer to a variable that will hold the gain setting in dB.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Important Note**: Gain levels may vary slightly across frequencies due to hardware characteristics.

#### **5\. `bladerf_set_gain_mode`**

`int bladerf_set_gain_mode(struct bladerf *dev, bladerf_channel ch, bladerf_gain_mode mode);`

* **Description**: Sets the gain mode for an RF channel, enabling either automatic or manual gain control.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `bladerf_gain_mode mode`: Gain mode (e.g., `BLADERF_GAIN_MGC` for manual or `BLADERF_GAIN_AGC` for automatic).  
* **Returns**: `0` on success, or `BLADERF_ERR_UNSUPPORTED` if the mode isn’t supported on the device.  
* **Developer Tips**:  
  * Use automatic gain control (AGC) mode for dynamic environments.  
  * Not all models support AGC; verify compatibility before use.

#### **6\. `bladerf_get_gain_mode`**

`int bladerf_get_gain_mode(struct bladerf *dev, bladerf_channel ch, bladerf_gain_mode *mode);`

* **Description**: Retrieves the current gain mode of an RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `bladerf_gain_mode *mode`: Pointer to a variable where the gain mode will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for verifying the current gain mode, particularly in systems where AGC is preferred.

#### **7\. `bladerf_get_gain_range`**

`int bladerf_get_gain_range(struct bladerf *dev, bladerf_channel ch, const struct bladerf_range **range);`

* **Description**: Provides the supported gain range for an RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `const struct bladerf_range **range`: Pointer to a `bladerf_range` structure that holds min and max gain values.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Check the `min` and `max` fields in the returned range to configure gain settings within supported limits.

#### **8\. `bladerf_set_rf_port`**

`int bladerf_set_rf_port(struct bladerf *dev, bladerf_channel ch, const char *port);`

* **Description**: Configures the RF port mode, such as selecting TX or RX paths, for specific device configurations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `const char *port`: Name of the port configuration, such as `"TX"` or `"RX"`.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Notes**: Port settings may vary based on the device hardware.

#### **9\. `bladerf_get_rf_port`**

`int bladerf_get_rf_port(struct bladerf *dev, bladerf_channel ch, const char **port);`

* **Description**: Retrieves the currently configured RF port mode.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `const char **port`: Pointer to a variable that will store the current port configuration name.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use to confirm the active port setting, especially when multiple modes are available.

---

### **Section 4: Sample Rate, Bandwidth, and Tuning**

These routines allow you to set and get sample rates, bandwidths, and configure tuning modes for the bladeRF device.

#### **1\. `bladerf_set_sample_rate`**

`int bladerf_set_sample_rate(struct bladerf *dev, bladerf_channel ch, bladerf_sample_rate rate, bladerf_sample_rate *actual);`

* **Description**: Sets the sample rate for a specified RF channel and returns the actual rate that was set, which may differ slightly due to hardware limitations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel (e.g., `BLADERF_CHANNEL_RX(0)` or `BLADERF_CHANNEL_TX(0)`).  
  * `bladerf_sample_rate rate`: Requested sample rate in samples per second.  
  * `bladerf_sample_rate *actual`: Pointer to a variable that will hold the actual sample rate applied.  
* **Returns**: `0` on success, or a negative error code (e.g., `BLADERF_ERR_RANGE` if the requested rate is unsupported).  
* **Developer Notes**: This function helps to achieve an optimal sampling rate within device constraints. The actual rate returned should be verified against the requested rate, especially in precision-sensitive applications.

#### **2\. `bladerf_get_sample_rate`**

`int bladerf_get_sample_rate(struct bladerf *dev, bladerf_channel ch, bladerf_sample_rate *rate);`

* **Description**: Retrieves the current sample rate for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `bladerf_sample_rate *rate`: Pointer to a variable where the current sample rate in samples per second will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for confirming the sample rate after initialization or configuration adjustments.

#### **3\. `bladerf_get_sample_rate_range`**

`int bladerf_get_sample_rate_range(struct bladerf *dev, bladerf_channel ch, const struct bladerf_range **range);`

* **Description**: Retrieves the minimum and maximum sample rates supported by a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `const struct bladerf_range **range`: Pointer to a `bladerf_range` structure that will hold the range of supported sample rates.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Use this to validate sample rate settings, particularly when configuring the device programmatically across different operational modes.

#### **4\. `bladerf_set_rational_sample_rate`**

`int bladerf_set_rational_sample_rate(struct bladerf *dev, bladerf_channel ch, struct bladerf_rational_rate *rate, struct bladerf_rational_rate *actual);`

* **Description**: Sets a rational sample rate for the specified channel, allowing finer control over the sample rate by specifying a fractional rate.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `struct bladerf_rational_rate *rate`: Pointer to a structure containing the desired sample rate as a rational fraction.  
  * `struct bladerf_rational_rate *actual`: Pointer to a structure that will be populated with the actual rate achieved.  
* **Returns**: `0` on success, or an error code on failure.  
* **Important Note**: This function is suitable for applications requiring non-integer sample rates.

#### **5\. `bladerf_get_rational_sample_rate`**

`int bladerf_get_rational_sample_rate(struct bladerf *dev, bladerf_channel ch, struct bladerf_rational_rate *rate);`

* **Description**: Retrieves the current rational sample rate of the specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `struct bladerf_rational_rate *rate`: Pointer to a `bladerf_rational_rate` structure that will be populated with the current rate as a rational fraction.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Useful for verifying complex sample rate configurations in systems requiring high precision.

#### **6\. `bladerf_set_bandwidth`**

`int bladerf_set_bandwidth(struct bladerf *dev, bladerf_channel ch, bladerf_bandwidth bandwidth, bladerf_bandwidth *actual);`

* **Description**: Sets the bandwidth for a specified RF channel and returns the actual bandwidth that was applied.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `bladerf_bandwidth bandwidth`: Desired bandwidth in Hz.  
  * `bladerf_bandwidth *actual`: Pointer to a variable that will hold the actual applied bandwidth.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Notes**: Checking the `actual` parameter after setting the bandwidth helps to confirm the closest achievable bandwidth.

#### **7\. `bladerf_get_bandwidth`**

`int bladerf_get_bandwidth(struct bladerf *dev, bladerf_channel ch, bladerf_bandwidth *bandwidth);`

* **Description**: Retrieves the current bandwidth setting for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `bladerf_bandwidth *bandwidth`: Pointer to a variable where the current bandwidth in Hz will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for confirming the bandwidth setting when reconfiguring the device for various modes.

#### **8\. `bladerf_get_bandwidth_range`**

`int bladerf_get_bandwidth_range(struct bladerf *dev, bladerf_channel ch, const struct bladerf_range **range);`

* **Description**: Retrieves the supported bandwidth range for a specified RF channel.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel.  
  * `const struct bladerf_range **range`: Pointer to a structure where the minimum and maximum bandwidths will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Referencing this range helps to ensure that bandwidth settings stay within the operational capabilities of the hardware.

#### **9\. `bladerf_set_tuning_mode`**

`int bladerf_set_tuning_mode(struct bladerf *dev, bladerf_tuning_mode mode);`

* **Description**: Configures the tuning mode of the device, which influences how frequencies are tuned and controlled.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_tuning_mode mode`: Desired tuning mode (e.g., `BLADERF_TUNING_MODE_HOST`, `BLADERF_TUNING_MODE_FPGA`, etc.).  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Important Notes**:  
  * Not all devices support multiple tuning modes.  
  * Some modes may improve performance or minimize tuning time.

#### **10\. `bladerf_get_tuning_mode`**

`int bladerf_get_tuning_mode(struct bladerf *dev, bladerf_tuning_mode *mode);`

* **Description**: Retrieves the current tuning mode of the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_tuning_mode *mode`: Pointer to a variable that will hold the current tuning mode.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for confirming tuning mode configuration, especially in applications that dynamically adjust tuning behavior.

---

### **Section 5: Data Streaming and Synchronization**

These functions manage data streaming, buffer handling, synchronization, and associated configurations for the bladeRF device. They are essential for handling real-time data transfer between the device and host system.

#### **1\. `bladerf_init_stream`**

`int bladerf_init_stream(struct bladerf_stream **stream, struct bladerf *dev, bladerf_stream_cb callback, void ***buffers, size_t num_buffers, bladerf_format format, size_t samples_per_buffer, size_t num_transfers, void *data);`

* **Description**: Initializes a data stream with specified buffer and format settings. This function sets up the necessary infrastructure for asynchronous data streaming.  
* **Parameters**:  
  * `struct bladerf_stream **stream`: Pointer to the stream structure to initialize.  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_stream_cb callback`: Callback function for handling data buffers during streaming.  
  * `void ***buffers`: Pointer to an array of buffers used for streaming. If `NULL`, the library allocates buffers.  
  * `size_t num_buffers`: Number of buffers to use in the stream.  
  * `bladerf_format format`: Data format of the samples (e.g., `BLADERF_FORMAT_SC16_Q11`).  
  * `size_t samples_per_buffer`: Number of samples in each buffer.  
  * `size_t num_transfers`: Number of asynchronous transfers to use during streaming.  
  * `void *data`: Additional user data passed to the callback function.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**:  
  * Choose `num_buffers` and `samples_per_buffer` carefully to balance latency and memory usage.  
  * Ensure that `callback` is properly configured to handle incoming/outgoing data efficiently.

#### **2\. `bladerf_stream`**

`int bladerf_stream(struct bladerf_stream *stream, bladerf_channel_layout layout);`

* **Description**: Begins streaming data using the initialized stream configuration.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the initialized stream structure.  
  * `bladerf_channel_layout layout`: Specifies the channel layout, such as single or dual channel (e.g., `BLADERF_RX_X1`).  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Important Note**: Stream processing begins after calling this function, so ensure that buffers and callbacks are fully prepared.

#### **3\. `bladerf_submit_stream_buffer`**

`int bladerf_submit_stream_buffer(struct bladerf_stream *stream, void *buffer, unsigned int timeout_ms);`

* **Description**: Submits a buffer to the stream for data transfer, used in non-blocking buffer submissions.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure.  
  * `void *buffer`: Pointer to the buffer to submit.  
  * `unsigned int timeout_ms`: Timeout in milliseconds for buffer submission.  
* **Returns**: `0` on success, or a negative error code on timeout or failure.  
* **Developer Tips**: Ideal for high-throughput applications needing asynchronous data handling.

#### **4\. `bladerf_submit_stream_buffer_nb`**

`int bladerf_submit_stream_buffer_nb(struct bladerf_stream *stream, void *buffer);`

* **Description**: Submits a buffer to the stream without specifying a timeout, functioning as a non-blocking submission.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure.  
  * `void *buffer`: Pointer to the buffer to be submitted.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Designed for continuous, uninterrupted streaming.

#### **5\. `bladerf_deinit_stream`**

`void bladerf_deinit_stream(struct bladerf_stream *stream);`

* **Description**: Deinitializes a previously initialized stream, freeing resources and stopping streaming operations.  
* **Parameters**:  
  * `struct bladerf_stream *stream`: Pointer to the stream structure to deinitialize.  
* **Returns**: None.  
* **Important Notes**: Always deinitialize streams to release memory and prevent resource leaks.

#### **6\. `bladerf_set_stream_timeout`**

`int bladerf_set_stream_timeout(struct bladerf *dev, bladerf_direction dir, unsigned int timeout);`

* **Description**: Sets the timeout for data transfers on a specified streaming direction.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Streaming direction (`BLADERF_RX` or `BLADERF_TX`).  
  * `unsigned int timeout`: Timeout in milliseconds for data transfers.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use to control latency in time-sensitive applications.

#### **7\. `bladerf_get_stream_timeout`**

`int bladerf_get_stream_timeout(struct bladerf *dev, bladerf_direction dir, unsigned int *timeout);`

* **Description**: Retrieves the current timeout setting for data transfers in the specified streaming direction.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Streaming direction (`BLADERF_RX` or `BLADERF_TX`).  
  * `unsigned int *timeout`: Pointer to a variable where the current timeout will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Use to confirm timeout settings and troubleshoot latency issues.

#### **8\. `bladerf_sync_config`**

`int bladerf_sync_config(struct bladerf *dev, bladerf_channel_layout layout, bladerf_format format, unsigned int num_buffers, unsigned int buffer_size, unsigned int num_transfers, unsigned int stream_timeout);`

* **Description**: Configures synchronous streaming, specifying buffer and timeout settings for each channel layout.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel_layout layout`: Specifies the channel layout (e.g., `BLADERF_RX_X1`).  
  * `bladerf_format format`: Data format for streaming (e.g., `BLADERF_FORMAT_SC16_Q11`).  
  * `unsigned int num_buffers`: Number of buffers to allocate for streaming.  
  * `unsigned int buffer_size`: Size of each buffer in samples.  
  * `unsigned int num_transfers`: Number of asynchronous transfers to use.  
  * `unsigned int stream_timeout`: Timeout for data transfers in milliseconds.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Adjust `num_buffers` and `buffer_size` to balance throughput and latency requirements.

#### **9\. `bladerf_sync_tx`**

`int bladerf_sync_tx(struct bladerf *dev, const void *samples, unsigned int num_samples, struct bladerf_metadata *metadata, unsigned int timeout_ms);`

* **Description**: Transmits a block of samples in synchronous mode, blocking until the transfer completes or times out.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const void *samples`: Pointer to sample data for transmission.  
  * `unsigned int num_samples`: Number of samples to transmit.  
  * `struct bladerf_metadata *metadata`: Metadata pointer (can be `NULL` if unused).  
  * `unsigned int timeout_ms`: Timeout for the transfer in milliseconds.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Ideal for precise sample control in time-sensitive transmit operations.

#### **10\. `bladerf_sync_rx`**

`int bladerf_sync_rx(struct bladerf *dev, void *samples, unsigned int num_samples, struct bladerf_metadata *metadata, unsigned int timeout_ms);`

* **Description**: Receives a block of samples in synchronous mode, blocking until the specified samples are received or a timeout occurs.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `void *samples`: Pointer to the buffer for storing received samples.  
  * `unsigned int num_samples`: Number of samples to receive.  
  * `struct bladerf_metadata *metadata`: Metadata pointer (can be `NULL` if unused).  
  * `unsigned int timeout_ms`: Timeout for the transfer in milliseconds.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: This function is suitable for applications needing controlled data capture.

#### **11\. `bladerf_get_timestamp`**

`int bladerf_get_timestamp(struct bladerf *dev, bladerf_direction dir, bladerf_timestamp *timestamp);`

* **Description**: Retrieves the timestamp of the last sample transmitted or received, based on the specified direction.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_direction dir`: Direction (`BLADERF_RX` or `BLADERF_TX`).  
  * `bladerf_timestamp *timestamp`: Pointer to a variable for storing the timestamp.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for synchronization in precise timing applications.

#### **12\. `bladerf_interleave_stream_buffer`**

`int bladerf_interleave_stream_buffer(bladerf_channel_layout layout, bladerf_format format, unsigned int buffer_size, void *samples);`

* **Description**: Interleaves samples in a buffer according to specified channel layout and format.  
* **Parameters**:  
  * `bladerf_channel_layout layout`: Channel layout for interleaving.  
  * `bladerf_format format`: Sample data format.  
  * `unsigned int buffer_size`: Size of the buffer.  
  * `void *samples`: Pointer to the sample buffer.  
* **Returns**: `0` on success.  
* **Developer Notes**: Prepares data for transmission in multi-channel setups.

#### **13\. `bladerf_deinterleave_stream_buffer`**

`int bladerf_deinterleave_stream_buffer(bladerf_channel_layout layout, bladerf_format format, unsigned int buffer_size, void *samples);`

* **Description**: De-interleaves samples in a buffer based on layout and format, useful for processing received data.  
* **Parameters**:  
  * `bladerf_channel_layout layout`: Channel layout for de-interleaving.  
  * `bladerf_format format`: Sample format.  
  * `unsigned int buffer_size`: Buffer size.  
  * `void *samples`: Pointer to sample buffer.  
* **Returns**: `0` on success.  
* **Usage**: Use to organize data from multi-channel reception.

---

### **Section 6: Calibration and Gain Tables**

These routines facilitate the loading, management, and application of gain calibration data, which can be critical for precise gain settings across various frequencies and environmental conditions.

#### **1\. `bladerf_load_gain_calibration`**

`int bladerf_load_gain_calibration(struct bladerf *dev, bladerf_channel ch, const char *cal_file_loc);`

* **Description**: Loads a gain calibration file for a specified RF channel, allowing the device to apply frequency-specific gain adjustments.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to load the gain calibration.  
  * `const char *cal_file_loc`: File path to the calibration file.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_INVAL` if the file is invalid or `BLADERF_ERR_NO_MEM` if memory allocation fails).  
* **Developer Notes**:  
  * Use this function when detailed gain calibration across frequencies is required.  
  * Ensure that the file format matches the device’s calibration requirements.

#### **2\. `bladerf_enable_gain_calibration`**

`int bladerf_enable_gain_calibration(struct bladerf *dev, bladerf_channel ch, bool en);`

* **Description**: Enables or disables gain calibration on a specified RF channel. When enabled, calibration data from the loaded file is applied during operation.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to enable or disable gain calibration.  
  * `bool en`: Boolean value (`true` to enable, `false` to disable).  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Ideal for applications that require dynamic gain adjustments based on pre-measured calibration data.

#### **3\. `bladerf_print_gain_calibration`**

`int bladerf_print_gain_calibration(struct bladerf *dev, bladerf_channel ch, bool with_entries);`

* **Description**: Outputs the loaded gain calibration data for a specified RF channel. This is useful for debugging or verification.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to print the gain calibration data.  
  * `bool with_entries`: Boolean indicating whether to print individual entries from the calibration table.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Use `with_entries = true` to view all calibration data points for in-depth analysis.

#### **4\. `bladerf_get_gain_calibration`**

`int bladerf_get_gain_calibration(struct bladerf *dev, bladerf_channel ch, const struct bladerf_gain_cal_tbl **tbl);`

* **Description**: Retrieves a pointer to the gain calibration table structure for a specified RF channel, allowing programmatic access to calibration data.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to retrieve the gain calibration.  
  * `const struct bladerf_gain_cal_tbl **tbl`: Pointer to a variable that will hold the address of the gain calibration table structure.  
* **Returns**: `0` on success, or a negative error code if the calibration data is unavailable.  
* **Important Note**: Access to raw calibration data allows for custom gain adjustments based on specific operational needs.

#### **5\. `bladerf_get_gain_target`**

`int bladerf_get_gain_target(struct bladerf *dev, bladerf_channel ch, int *gain_target);`

* **Description**: Calculates and retrieves the target gain for a specified RF channel, taking calibration adjustments into account.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: RF channel for which to calculate the gain target.  
  * `int *gain_target`: Pointer to a variable that will hold the calculated gain target in dB.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use in applications where a precise gain value is required, factoring in environmental or frequency-based calibration adjustments.

---

### **Section 7: Loopback and Debug**

These functions are designed for diagnostic testing and debugging by enabling loopback modes and retrieving loopback configurations on the bladeRF device.

#### **1\. `bladerf_set_loopback`**

`int bladerf_set_loopback(struct bladerf *dev, bladerf_loopback l);`

* **Description**: Configures the device to operate in a specific loopback mode, useful for diagnostics where transmitted signals are internally routed to the receiver.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback l`: Loopback mode to set, such as:  
    * `BLADERF_LB_NONE`: No loopback (normal operation).  
    * `BLADERF_LB_FIRMWARE`: Firmware-based loopback.  
    * `BLADERF_LB_RF_LNA1`, `BLADERF_LB_RF_LNA2`, `BLADERF_LB_RF_LNA3`: RF loopback to specific LNA (Low-Noise Amplifier) paths.  
* **Returns**: `0` on success, or a negative error code on failure (e.g., `BLADERF_ERR_UNSUPPORTED` if the mode is not supported).  
* **Developer Tips**: Use loopback modes for troubleshooting signal paths or verifying data integrity without needing an external transmitter.

#### **2\. `bladerf_get_loopback`**

`int bladerf_get_loopback(struct bladerf *dev, bladerf_loopback *l);`

* **Description**: Retrieves the current loopback mode setting of the device, which is useful for confirming diagnostic configurations.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback *l`: Pointer to a variable that will store the current loopback mode.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use this function to verify the active loopback mode, particularly in test setups.

#### **3\. `bladerf_is_loopback_mode_supported`**

`bool bladerf_is_loopback_mode_supported(struct bladerf *dev, bladerf_loopback mode);`

* **Description**: Checks if a specific loopback mode is supported by the current device model.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_loopback mode`: Loopback mode to check for support.  
* **Returns**: `true` if the mode is supported, `false` if not.  
* **Developer Notes**: Checking loopback mode support is crucial for multi-model deployments, as capabilities may vary across bladeRF models.

#### **4\. `bladerf_get_loopback_modes`**

`int bladerf_get_loopback_modes(struct bladerf *dev, const struct bladerf_loopback_modes **modes);`

* **Description**: Retrieves a list of supported loopback modes for the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const struct bladerf_loopback_modes **modes`: Pointer to a variable that will hold a list of supported loopback modes.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: This function is useful for configuring devices dynamically based on their capabilities.

---

### **Section 8: Low-Level Access and Control**

These functions provide direct access to the device’s low-level hardware components, including GPIO, SPI, flash memory, configuration registers, and more. They enable fine-grained control over device operations and are useful for advanced configuration and debugging.

#### **1\. `bladerf_config_gpio_read`**

`int bladerf_config_gpio_read(struct bladerf *dev, uint32_t *val);`

* **Description**: Reads the current value of the configuration GPIO register, providing insight into various device status bits.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t *val`: Pointer to a variable that will hold the GPIO register value.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Useful for reading status flags or device state indicators accessible via GPIO.

#### **2\. `bladerf_config_gpio_write`**

`int bladerf_config_gpio_write(struct bladerf *dev, uint32_t val);`

* **Description**: Writes a specified value to the configuration GPIO register, allowing control over GPIO-accessible features.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t val`: Value to write to the GPIO register.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Important Note**: Be cautious when modifying GPIO settings, as incorrect values may alter device behavior.

#### **3\. `bladerf_read_flash`**

`int bladerf_read_flash(struct bladerf *dev, uint8_t *buf, uint32_t page, uint32_t count);`

* **Description**: Reads data from flash memory, starting at the specified page, and fills the provided buffer.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint8_t *buf`: Pointer to a buffer where read data will be stored.  
  * `uint32_t page`: Starting flash page.  
  * `uint32_t count`: Number of pages to read.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for firmware validation or retrieving device configuration stored in flash memory.

#### **4\. `bladerf_write_flash`**

`int bladerf_write_flash(struct bladerf *dev, const uint8_t *buf, uint32_t page, uint32_t count);`

* **Description**: Writes data from the specified buffer to the device’s flash memory, starting at the specified page.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `const uint8_t *buf`: Pointer to the buffer containing data to write.  
  * `uint32_t page`: Starting flash page.  
  * `uint32_t count`: Number of pages to write.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Notes**: Flash writes are typically slower and can wear out over time, so minimize frequent writing.

#### **5\. `bladerf_erase_flash`**

`int bladerf_erase_flash(struct bladerf *dev, uint32_t erase_block, uint32_t count);`

* **Description**: Erases a specified range of blocks in the device’s flash memory.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t erase_block`: Starting block number.  
  * `uint32_t count`: Number of blocks to erase.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Erase flash data before performing firmware updates or resetting device state.

#### **6\. `bladerf_set_vctcxo_tamer_mode`**

`int bladerf_set_vctcxo_tamer_mode(struct bladerf *dev, bladerf_vctcxo_tamer_mode mode);`

* **Description**: Sets the VCTCXO (Voltage-Controlled Temperature-Compensated Crystal Oscillator) tamer mode, which manages the external frequency reference.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_vctcxo_tamer_mode mode`: Tamer mode, such as:  
    * `BLADERF_VCTCXO_TAMER_DISABLED`  
    * `BLADERF_VCTCXO_TAMER_1_PPS` (Pulse Per Second)  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Notes**: Enables synchronization with an external time or frequency reference, often useful in GPS-disciplined setups.

#### **7\. `bladerf_get_vctcxo_tamer_mode`**

`int bladerf_get_vctcxo_tamer_mode(struct bladerf *dev, bladerf_vctcxo_tamer_mode *mode);`

* **Description**: Retrieves the current VCTCXO tamer mode of the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_vctcxo_tamer_mode *mode`: Pointer to a variable that will hold the current tamer mode.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use this function to confirm the tamer mode for applications requiring frequency discipline.

#### **8\. `bladerf_get_vctcxo_trim`**

`int bladerf_get_vctcxo_trim(struct bladerf *dev, uint16_t *trim);`

* **Description**: Reads the current VCTCXO trim DAC setting, allowing fine adjustments to the oscillator.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint16_t *trim`: Pointer to a variable that will store the DAC trim value.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Useful for temperature drift adjustments in high-precision frequency applications.

#### **9\. `bladerf_trim_dac_write`**

`int bladerf_trim_dac_write(struct bladerf *dev, uint16_t trim);`

* **Description**: Writes a specified trim value to the DAC, allowing adjustments to the VCTCXO for precise frequency control.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint16_t trim`: DAC trim value to write.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Adjust the VCTCXO frequency, typically after calibration, to achieve high accuracy.

#### **10\. `bladerf_read_trigger`**

`int bladerf_read_trigger(struct bladerf *dev, bladerf_channel ch, bladerf_trigger_signal trigger, uint8_t *val);`

* **Description**: Reads the state of a trigger signal on a specified RF channel, which can be used for synchronizing external events.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: Channel to read the trigger signal from.  
  * `bladerf_trigger_signal trigger`: The trigger signal to read.  
  * `uint8_t *val`: Pointer to a variable where the trigger state will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Use in applications that require precise timing control or synchronization.

#### **11\. `bladerf_write_trigger`**

`int bladerf_write_trigger(struct bladerf *dev, bladerf_channel ch, bladerf_trigger_signal trigger, uint8_t val);`

* **Description**: Writes a value to a trigger signal on a specified RF channel, allowing synchronization with external devices.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_channel ch`: Channel to write the trigger signal.  
  * `bladerf_trigger_signal trigger`: Trigger signal to configure.  
  * `uint8_t val`: Value to set for the trigger signal.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Use for controlled synchronization in timing-sensitive operations.

#### **12\. `bladerf_wishbone_master_read`**

`int bladerf_wishbone_master_read(struct bladerf *dev, uint32_t addr, uint32_t *data);`

* **Description**: Reads data from the Wishbone master interface, useful for low-level register access in compatible devices.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t addr`: Register address to read from.  
  * `uint32_t *data`: Pointer to a variable where the read data will be stored.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Developer Tips**: Use for direct register interaction when developing or debugging custom features.

#### **13\. `bladerf_wishbone_master_write`**

`int bladerf_wishbone_master_write(struct bladerf *dev, uint32_t addr, uint32_t data);`

* **Description**: Writes data to the Wishbone master interface, enabling direct control over registers.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `uint32_t addr`: Register address to write to.  
  * `uint32_t data`: Data to write to the register.  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful for developing custom hardware integrations or low-level diagnostics.

---

### **Section 9: Error Handling and Logging**

These functions provide error handling support and control over logging verbosity, helping developers troubleshoot issues and retrieve human-readable error messages for debugging.

#### **1\. `bladerf_strerror`**

`const char *bladerf_strerror(int error);`

* **Description**: Converts a `libbladeRF` error code into a human-readable string, providing a description of the error.  
* **Parameters**:  
  * `int error`: Error code to convert (typically returned by other `libbladeRF` functions).  
* **Returns**: A constant string describing the error. For example, `BLADERF_ERR_INVAL` might return `"Invalid argument"`.  
* **Developer Tips**: Use `bladerf_strerror` in error handling code to print clear error messages, enhancing debugging and logging output.

#### **2\. `bladerf_errstr`**

`int bladerf_errstr(int error_code, char *err_str, size_t err_str_size);`

* **Description**: Retrieves a detailed error message for a specific `libbladeRF` error code, useful for diagnostics.  
* **Parameters**:  
  * `int error_code`: Error code for which to retrieve a detailed message.  
  * `char *err_str`: Pointer to a buffer where the error message will be stored.  
  * `size_t err_str_size`: Size of the buffer to ensure the message does not exceed buffer limits.  
* **Returns**: `0` on success, or a negative error code if an invalid error code is passed.  
* **Usage**: Provides a detailed error message suitable for logging systems or user interfaces requiring in-depth error information.

#### **3\. `bladerf_log_set_verbosity`**

`void bladerf_log_set_verbosity(bladerf_log_level level);`

* **Description**: Sets the verbosity level of logging output for the `libbladeRF` library, controlling the amount of debugging information that is generated.  
* **Parameters**:  
  * `bladerf_log_level level`: Desired logging verbosity level, such as:  
    * `BLADERF_LOG_LEVEL_SILENT`: No logging output.  
    * `BLADERF_LOG_LEVEL_ERROR`: Log only errors.  
    * `BLADERF_LOG_LEVEL_WARNING`: Log warnings and errors.  
    * `BLADERF_LOG_LEVEL_INFO`: Log informational messages, warnings, and errors.  
    * `BLADERF_LOG_LEVEL_DEBUG`: Log detailed debugging information.  
* **Returns**: None.  
* **Developer Tips**: Use higher verbosity levels (`INFO` or `DEBUG`) during development and testing. Lower verbosity levels (`WARNING` or `ERROR`) are more appropriate for production environments.

#### **4\. `bladerf_log_get_verbosity`**

`int bladerf_log_get_verbosity(bladerf_log_level *level);`

* **Description**: Retrieves the current logging verbosity level of the `libbladeRF` library.  
* **Parameters**:  
  * `bladerf_log_level *level`: Pointer to a variable where the current verbosity level will be stored.  
* **Returns**: `0` on success, or a negative error code if the operation fails.  
* **Usage**: Useful for applications that adjust logging dynamically, allowing confirmation of the current logging level.

#### **5\. `bladerf_get_last_error`**

`int bladerf_get_last_error(struct bladerf *dev, int *error);`

* **Description**: Retrieves the last error code encountered by the device for further analysis, which can be particularly useful for debugging intermittent issues.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `int *error`: Pointer to a variable that will store the last error code encountered.  
* **Returns**: `0` on success, or a negative error code if retrieving the last error fails.  
* **Developer Tips**: This function can help identify the cause of recent device failures without the need for extensive logging.

---

### **Section 10: Miscellaneous Utility Functions**

These utility functions cover a range of device-related operations, including retrieving device information, checking compatibility, and managing configuration values.

#### **1\. `bladerf_get_serial`**

`int bladerf_get_serial(struct bladerf *dev, char *serial, size_t len);`

* **Description**: Retrieves the device's serial number as a human-readable string.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `char *serial`: Pointer to a character buffer where the serial number will be stored.  
  * `size_t len`: Length of the buffer to ensure the serial number fits.  
* **Returns**: `0` on success, or a negative error code if the operation fails.  
* **Developer Tips**: Use this to identify or differentiate between multiple devices connected to a system.

#### **2\. `bladerf_get_fpga_size`**

`int bladerf_get_fpga_size(struct bladerf *dev, bladerf_fpga_size *size);`

* **Description**: Retrieves the FPGA size supported by the device, which varies depending on the bladeRF model.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `bladerf_fpga_size *size`: Pointer to a variable that will store the FPGA size (e.g., `BLADERF_FPGA_40KLE`, `BLADERF_FPGA_115KLE`).  
* **Returns**: `0` on success, or a negative error code on failure.  
* **Usage**: Useful when verifying FPGA compatibility with device operations or configurations.

#### **3\. `bladerf_get_fpga_bytes`**

`int bladerf_get_fpga_bytes(bladerf_fpga_size size);`

* **Description**: Returns the number of bytes required for the specified FPGA configuration size.  
* **Parameters**:  
  * `bladerf_fpga_size size`: FPGA size enum value.  
* **Returns**: Integer representing the size in bytes or `BLADERF_ERR_INVAL` if the size is invalid.  
* **Developer Tips**: Useful for loading compatible FPGA files or confirming sufficient memory allocation for FPGA images.

#### **4\. `bladerf_is_fpga_configured`**

`int bladerf_is_fpga_configured(struct bladerf *dev);`

* **Description**: Checks whether an FPGA image is currently loaded and configured on the device.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
* **Returns**: `1` if FPGA is configured, `0` if not, or a negative error code if the operation fails.  
* **Usage**: Essential for verifying device readiness for operations requiring FPGA configuration.

#### **5\. `bladerf_get_version`**

`void bladerf_get_version(struct bladerf_version *version);`

* **Description**: Retrieves version information for the `libbladeRF` library, FPGA, and firmware components.  
* **Parameters**:  
  * `struct bladerf_version *version`: Pointer to a `bladerf_version` structure that will store version details for the library, FPGA, and firmware.  
* **Returns**: None.  
* **Developer Tips**: Useful for ensuring compatibility across library, firmware, and hardware versions, especially when using specific features dependent on certain versions.

#### **6\. `bladerf_get_board_name`**

`int bladerf_get_board_name(struct bladerf *dev, char *name, size_t len);`

* **Description**: Retrieves a human-readable name for the board type, such as "bladeRF x40" or "bladeRF x115".  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `char *name`: Pointer to a buffer where the board name will be stored.  
  * `size_t len`: Length of the buffer to ensure the name fits.  
* **Returns**: `0` on success, or a negative error code if the operation fails.  
* **Usage**: Useful for applications needing device-specific configurations or for distinguishing between bladeRF models.

#### **7\. `bladerf_check_fw_compat`**

`int bladerf_check_fw_compat(struct bladerf *dev, struct bladerf_version *required);`

* **Description**: Checks if the device's firmware version is compatible with the current library version, ensuring all features will function as expected.  
* **Parameters**:  
  * `struct bladerf *dev`: Pointer to the device handle.  
  * `struct bladerf_version *required`: Pointer to a `bladerf_version` structure specifying the minimum required firmware version.  
* **Returns**: `0` if compatible, `BLADERF_ERR_UPDATE_FW` if an update is required.  
* **Developer Tips**: Always perform compatibility checks when using specific features or functions that may depend on firmware version.
