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
- Bounded streaming input for interleaved little-endian `f32` and signed 16-bit
  I/Q files, including packet recovery across block boundaries.
- Exact access-address sample indices, carrier-offset estimates, modulation
  deviation estimates, and discontinuity reset/reporting.
- Typed decoding for legacy advertising, scan, direct, and connection-request
  PDUs, including AD structures and validated CONNECT_IND timing/channel data.
- Dependency-free PCAPNG output using the standard Bluetooth LE link-layer
  pseudo-header.
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
  --sample-rate 4000000 \
  --block-samples 262144 \
  --output-pcap capture.pcapng
```

The file must contain interleaved I then Q samples. `f32le` uses two
little-endian `f32` values per complex sample. `s16le` uses two little-endian
signed 16-bit values normalized to approximately `[-1, 1]`.

Only packets with a valid BLE CRC are emitted. The current demodulator requires
an integer oversampling ratio from 2 through 64 samples per LE 1M symbol.
`--capture-start-ns` can supply the Unix timestamp of sample zero for PCAPNG;
without it, timestamps are relative to the Unix epoch.

The decoder processes the file in bounded blocks and retains enough overlap to
recover maximum-length primary advertisements split between reads. Repeated
identical advertisements are preserved when they occur at different sample
positions.

## Standalone implementation policy

Blueoxide implements protocol framing, CRC, whitening, demodulation, buffering,
semantic decoding, timestamp conversion, and PCAPNG serialization in-tree.
Third-party implementations may be used as development-time verification
oracles, but are not runtime dependencies.

The narrow hardware boundary may initially call LimeSuite, libbladeRF, or
libxtrx because those libraries contain device/firmware transport knowledge.
Backend code must keep that boundary isolated. The longer-term objective is to
move safely verifiable device control into Rust where protocol documentation,
firmware compatibility, and hardware tests make that practical.

## Development direction

The next backend work is to implement direct, feature-gated FFI for LimeSuite,
libbladeRF, and libxtrx, with vendor libraries as runtime/build prerequisites
but no wrapper-crate dependency. Those backends will feed the same `IqSource`
contract and will be exercised with recorded fixtures when hardware is absent.
Next receive work is direct LimeSuite, libbladeRF, and libxtrx integration,
followed by wideband channelization and BLE connection following. Full packet
decode is a project requirement: extended advertising, LL control, L2CAP,
ATT/GATT, SMP, LE 2M/Coded PHY, and Bluetooth Classic BR/EDR layers will be
added incrementally while retaining undecoded packet bytes losslessly.

Active signal injection and transmit support are intentionally deferred until
receive, timestamping, channelization, and packet validation are reliable;
transmit will be introduced as a separate subsystem rather than folded into the
receive API.

See `DesignLog.md` for architectural decisions and `ChangeLog.md` for completed
increments. `Verification.md` records independent cross-implementation checks.
