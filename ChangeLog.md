# Blueoxide Change Log

## Unreleased

### Added

- A buildable Rust 2024 Cargo package with no third-party runtime dependencies
  in the default receive core.
- Bluetooth LE logical-channel validation and complete 0 through 39 center
  frequency mapping.
- In-tree Bluetooth LE whitening and 24-bit CRC implementations.
- Primary advertising PDU detection for channels 37, 38, and 39 with:
  - Configurable access-address error tolerance.
  - Normal and inverted spectrum handling.
  - Bounded payload parsing.
  - Mandatory CRC validation.
- LE 1M quadrature demodulation with exhaustive integer symbol-phase search and
  robust decision-threshold estimation.
- Interleaved little-endian `f32` and signed 16-bit offline I/Q readers with
  allocation limits, framing validation, and non-finite sample rejection.
- `blueoxide channels` and `blueoxide decode` commands.
- A hardware-neutral `IqSource` receive trait and SDR configuration validation.
- Receive metadata fields for sample index, dropped samples, and overrun state.
- Unit tests for channel mapping, whitening, CRC, PDU validation, GFSK
  demodulation, malformed I/Q input, and SDR configuration failures.
- Initial README and design log.

### Changed

- Defined Blueoxide as a receive and capture package first. Active signal
  injection remains planned but will use a separate transmit subsystem.
- Excluded the original standalone SDR/channelizer sketches from the Cargo build
  until their behavior is migrated to validated backends.

### Known limitations

- No hardware backend is connected to the new `IqSource` API yet.
- The executable currently decodes offline LE 1M primary-advertising IQ only.
- The demodulator is block-oriented and requires an integer multiple of 1 MHz
  sample rate.
- Packet timestamps, RSSI/SNR estimates, PCAPNG output, connection following,
  Bluetooth Classic, LE 2M, and LE Coded PHY remain to be implemented.

