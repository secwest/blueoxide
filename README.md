# Blueoxide

Blueoxide is a Rust implementation of an over-the-air Bluetooth and Bluetooth
Low Energy receive and capture stack for LimeSDR, bladeRF, and XTRX radios.
The project favors in-tree DSP and protocol implementations so captures remain
reproducible and the core can be tested without vendor SDKs or attached radios.

## Current status

The repository now contains a dependency-free, buildable receive core with:

- Bluetooth LE channel-to-frequency mapping.
- LE whitening and 24-bit CRC implementations.
- CRC-gated decoding of primary advertising PDUs on channels 37, 38, and 39.
- LE 1M quadrature demodulation with integer timing-phase search, robust slicing,
  spectrum-inversion handling, and configurable access-address tolerance.
- Input support for interleaved little-endian `f32` and signed 16-bit I/Q files.
- A hardware-neutral receive trait that requires backends to report overruns and
  dropped samples.

The older root-level SDR and channelizer files are historical prototypes. They
are not part of the Cargo build because they depend on unverified crates, use
incomplete native APIs, and contain unsafe SIMD assumptions. Their useful intent
will be migrated into tested backend modules rather than silently presented as
working hardware support.

## Build and test

```text
cargo build
cargo test
```

List the BLE channel map:

```text
cargo run -- channels
```

Decode a baseband recording centered on BLE advertising channel 37:

```text
cargo run --release -- decode \
  --input capture.cf32 \
  --format f32le \
  --channel 37 \
  --sample-rate 4000000
```

The file must contain interleaved I then Q samples. `f32le` uses two
little-endian `f32` values per complex sample. `s16le` uses two little-endian
signed 16-bit values normalized to approximately `[-1, 1]`.

Only packets with a valid BLE CRC are emitted. The current demodulator requires
an integer oversampling ratio from 2 through 64 samples per LE 1M symbol.

## Development direction

The next backend work is to implement direct, feature-gated FFI for LimeSuite,
libbladeRF, and libxtrx, with vendor libraries as runtime/build prerequisites
but no wrapper-crate dependency. Those backends will feed the same `IqSource`
contract and will be exercised with recorded fixtures when hardware is absent.
Classic Bluetooth BR/EDR, data-channel connection following, LE 2M, and LE Coded
PHY support will be layered onto the shared receive pipeline. Active signal
injection and transmit support are intentionally deferred until receive,
timestamping, channelization, and packet validation are reliable; transmit will
be introduced as a separate subsystem rather than folded into the receive API.

See `DesignLog.md` for architectural decisions and `ChangeLog.md` for completed
increments.
