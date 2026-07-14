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
- Bounded streaming LE 1M decoding across input blocks with exact
  access-address sample positions and explicit discontinuity resets.
- Preservation of repeated identical advertisements at distinct sample
  positions.
- Packet-local carrier-offset, modulation-deviation, and discriminator
  separation estimates.
- Interleaved little-endian `f32` and signed 16-bit streaming I/Q readers with
  allocation limits, short-read handling, framing validation, and non-finite
  sample rejection.
- Typed semantic decoding for legacy advertising, direct advertising, scan
  requests/responses, scannable advertising, and CONNECT_IND.
- Advertising Data structure parsing with known type names and UTF-8 local-name
  access.
- CONNECT_IND validation for timing, latency, channel map, hop increment,
  reserved bits, and supervision-timeout relationships.
- Dependency-free PCAPNG output with nanosecond timestamps and
  `LINKTYPE_BLUETOOTH_LE_LL_WITH_PHDR`.
- `blueoxide channels` and `blueoxide decode` commands.
- A hardware-neutral `IqSource` receive trait and SDR configuration validation.
- Receive metadata fields for sample index, dropped samples, and overrun state.
- An in-tree Windows/Unix dynamic-library loader with tested symbol lookup.
- A runtime-loaded libbladeRF receive backend with SC16 Q11 metadata streaming,
  immediate continuous receive, FPGA timestamps, timeout recovery, overrun and
  dropped-sample reporting, applied-rate reporting, deterministic library
  overrides, and validated lifecycle cleanup.
- A finite live-capture loop with sample and duration limits, cross-block
  decoding, capture-relative packet timestamps, and unconditional source stop
  after stream-time failures.
- `blueoxide backends` and `blueoxide capture --device bladerf` commands.
- Live CLI validation for duration overflow, zero-sized reads, zero timeouts,
  unsupported devices, and missing native libraries.
- Unit tests for channel mapping, whitening, CRC, PDU validation, GFSK
  demodulation, malformed I/Q input, SDR configuration failures, native backend
  lifecycle, timeout recovery, timestamp gaps/overflow, cleanup, and ABI layout.
- Cross-implementation CRC/whitening fixtures and external Scapy PCAPNG
  interoperability checks.
- Linux and Windows GitHub Actions gates for formatting, tests, Clippy, and
  release builds.
- Initial README and design log.

### Changed

- Defined Blueoxide as a receive and capture package first. Active signal
  injection remains planned but will use a separate transmit subsystem.
- Excluded the original standalone SDR/channelizer sketches from the Cargo build
  until their behavior is migrated to validated backends.

### Known limitations

- LimeSDR and XTRX backends are not connected to `IqSource` yet.
- The bladeRF backend has not yet been exercised with an installed libbladeRF
  library or physical radio in the development environment.
- bladeRF live capture currently supports the RX0/X1 stream layout only.
- The demodulator requires an integer multiple of 1 MHz sample rate.
- Hardware-correlated wall-clock time, calibrated RSSI/SNR, connection
  following, extended advertising, higher protocol layers, Bluetooth Classic,
  LE 2M, and LE Coded PHY remain to be implemented.
