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
- Configurable CRC-gated LE 1M data-channel decoding for a known connection
  access address, CRC initializer, and logical channel.
- LE 1M quadrature demodulation with integer timing-phase search, robust slicing,
  spectrum-inversion handling, and configurable access-address tolerance.
- Bounded streaming input for interleaved little-endian `f32` and signed 16-bit
  I/Q files, including packet recovery across block boundaries.
- Exact access-address sample indices, carrier-offset estimates, modulation
  deviation estimates, and discontinuity reset/reporting.
- Typed decoding for legacy advertising, scan, direct, and connection-request
  PDUs, including AD structures and validated CONNECT_IND timing/channel data.
- Data-channel header, CTEInfo, L2CAP-start, and LL control-PDU decoding while
  retaining unrecognized payload and MIC bytes losslessly.
- Validated data-channel maps plus Channel Selection Algorithms #1 and #2,
  including CONNECT_IND ChSel selection and event-counter channel calculation.
- Anchored connection-event tracking with wrap-safe instant handling, strict
  LL_CHANNEL_MAP_IND/LL_CONNECTION_UPDATE_IND parsing, and explicit anchor
  reacquisition after connection-parameter changes.
- CONNECT_IND event-0 acquisition windows, Core sleep-clock-accuracy mapping,
  receiver-clock widening, missed-event matching, and observation-driven
  re-anchoring.
- Dependency-free PCAPNG output using the standard Bluetooth LE link-layer
  pseudo-header.
- A hardware-neutral receive trait that requires backends to report overruns and
  dropped samples.
- A dependency-free dynamic-library loader for Windows, Linux, and macOS.
- A live libbladeRF receive backend using SC16 Q11 metadata samples, native
  hardware timestamps, timeout recovery, overrun detection, and timestamp-gap
  accounting.
- A live LimeSuite receive backend using interleaved `f32` I/Q, device-reported
  capability ranges, automatic RX calibration, hardware timestamps, FIFO
  overrun/dropped-packet status, and timestamp-gap accounting.
- A live libxtrx receive backend using single-channel 16-bit wire/host streams,
  in-tree Q11 conversion, hardware timestamps, finite timeout recovery, native
  overflow intervals, and exact timestamp-gap accounting.
- A finite live-capture pipeline that always stops the source after decode,
  callback, or read failures.

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

Probe runtime hardware-library availability:

```text
cargo run -- backends
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

Decode a recording from a known LE connection data channel:

```text
cargo run --release -- decode-data \
  --input connection.cf32 \
  --format f32le \
  --channel 12 \
  --sample-rate 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --block-samples 262144 \
  --output-pcap connection.pcapng
```

`decode-data` accepts data channels 0 through 36. The connection access address
and 24-bit CRC initializer normally come from a decoded CONNECT_IND. Data PDUs
are emitted only after CRC validation. When the CP bit is set, the separate
CTEInfo octet is retained and decoded without including it in the Length-counted
payload. The payload field remains lossless and can include an encrypted MIC
because decryption state is not yet tracked. Printed L2CAP and LL control
interpretations are explicitly plaintext hints; encrypted payloads remain
available as raw bytes but cannot yet be interpreted reliably.

Generate an offline event/channel/sample plan from an observed connection
anchor:

```text
cargo run --release -- connection-plan \
  --access-address 0x12345678 \
  --channel-map ffffffff1f \
  --csa 2 \
  --interval 24 \
  --sample-rate 4000000 \
  --anchor-event 0 \
  --anchor-sample 1000 \
  --events 10
```

The channel map is five hexadecimal octets in over-the-air order. Connection
intervals use 1.25 ms units, supervision timeouts use 10 ms units, and the
expected sample index is calculated relative to the observed access-address
sample without accumulating per-event rounding error. `--hop` selects the
5-through-16 hop increment for CSA#1. `--peer-sca` accepts the CONNECT_IND SCA
field from 0 through 7, while `--receiver-ppm` supplies the receiver sample
clock's worst-case error. Plans include the resulting earliest/latest sample
bounds.

Acquire event 0 from a decoded CONNECT_IND and continue with later CRC-valid
observations:

```text
cargo run --release -- connection-acquire \
  --access-address 0x12345678 \
  --channel-map ffffffff1f \
  --csa 1 \
  --hop 10 \
  --window-size 2 \
  --window-offset 3 \
  --interval 24 \
  --sample-rate 4000000 \
  --connect-sample 1000 \
  --peer-sca 0 \
  --receiver-ppm 20 \
  --central-observe 10:30000 \
  --observe 20:150020
```

`--connect-sample` is the CONNECT_IND access-address sample. The first
`--central-observe` value must be a caller-identified, CRC-valid transmission
from the central on event 0's selected channel inside the clock-widened
WinOffset / WinSize search window. Blueoxide does not infer direction from an
isolated data PDU: a peripheral response in that window is not an anchor.
Usually the central transmission is the earliest decoded packet in the event,
but packet order is not sufficient when the capture may have missed it. Each
later `--observe CHANNEL:SAMPLE` value is matched against the expected hopping
sequence and timing windows. A successful match reports skipped events and
timing error, then uses the observed access-address sample as the new anchor.

For a connection with an existing anchor, use `connection-sync` with repeated
`--observe CHANNEL:SAMPLE` values. `--max-event-advance` bounds the amount of
state searched for each observation.

The library tracker can schedule decoded `LL_CHANNEL_MAP_IND` and
`LL_CONNECTION_UPDATE_IND` control PDUs. A channel-map update is applied before
choosing the channel at its instant. A connection-parameter update deliberately
returns an anchor-observation-required state at its instant; ordinary
missed-event searches stop there, and scheduling resumes only after the caller
supplies the access-address sample actually observed in that event.

Capture live BLE advertising traffic from bladeRF RX0:

```text
cargo run --release -- capture \
  --device bladerf \
  --channel 37 \
  --sample-rate 4000000 \
  --bandwidth 2000000 \
  --gain 30 \
  --seconds 30 \
  --output-pcap capture.pcapng
```

Use `--device limesdr` for LimeSDR:

```text
cargo run --release -- capture \
  --device limesdr \
  --channel 37 \
  --sample-rate 4000000 \
  --bandwidth 2000000 \
  --gain 30 \
  --seconds 30 \
  --output-pcap capture.pcapng
```

Use `--device xtrx` for XTRX channel A. Add `--rx-channel 1` for channel B:

```text
cargo run --release -- capture \
  --device xtrx \
  --channel 37 \
  --sample-rate 4000000 \
  --bandwidth 2000000 \
  --gain 30 \
  --rx-channel 0 \
  --seconds 30 \
  --output-pcap capture.pcapng
```

The bladeRF backend loads the vendor library at runtime, so the project still
builds and its DSP/protocol tests run without an installed SDR SDK. The default
library names are `bladeRF.dll`/`libbladeRF.dll` on Windows,
`libbladeRF.so.2`/`libbladeRF.so` on Linux, and `libbladeRF.dylib` on macOS.
Set `BLUEOXIDE_BLADERF_LIBRARY` to an exact library path or name when the
library is installed elsewhere. When set, this override is exclusive so a
misconfigured path cannot silently select a different installation.

Live capture currently uses libbladeRF's `BLADERF_RX_X1` layout, which maps to
hardware RX0. RX1 is rejected until an X2 stream and explicit channel
deinterleaving are implemented. The hardware-applied sample rate must exactly
match the LE demodulator rate; a quantized mismatch is reported instead of
silently corrupting symbol timing. Native receive timeouts are treated as empty
reads, while other native failures stop capture and are returned with their
libbladeRF status code and error string.

The LimeSDR backend loads `LimeSuite.dll`/`libLimeSuite.dll` on Windows,
`libLimeSuite.so` on Linux, or `libLimeSuite.dylib` on macOS. Set
`BLUEOXIDE_LIMESUITE_LIBRARY` to an exact path or name to override discovery.
At open time Blueoxide queries the device's RX channel count, LO range, sample
rate range, and LPF range. It uses LimeSuite's `LMS_FMT_F32` stream format,
checks every returned scalar for finiteness, and calibrates RX after frequency,
rate, bandwidth, and gain configuration. Calibration uses at least 2.5 MHz,
matching SoapyLMS7 behavior, while the receive LPF remains at the requested
bandwidth.

The XTRX backend loads `xtrx.dll`/`libxtrx.dll` on Windows,
`libxtrx.so.0`/`libxtrx.so` on Linux, or
`libxtrx.0.dylib`/`libxtrx.dylib` on macOS. Set
`BLUEOXIDE_XTRX_LIBRARY` to an exact path or name to override discovery. It
configures receive-only SISO streaming with 16-bit wire and host formats,
converts the device's Q11 sample values in-tree, and supports channel A or B.
Channel B follows libxtrx's established `SISO_MODE|SWAP_AB` selection
convention. The generic `--gain` value maps to the XTRX LNA stage and is
therefore limited to 0 through 30 dB; additional stage-specific gain controls
remain future CLI work.

XTRX reads request finite native timeouts and disable gap-filling. Native
overflow intervals and sample-counter jumps are reported as dropped samples,
so the streaming decoder resets across discontinuities instead of treating
inserted zeros as received RF data.

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

The next hardware work is recorded fixtures and live smoke tests from all three
supported SDR families. Connection framing, channel selection, anchored event
progression, clock-error windows, offline anchor acquisition, observation
synchronization, and instant-based map/parameter updates are now present; the
next receive stages are wideband channelization or timed retuning, automatic
capture-driven observation delivery, and live BLE connection following. Full
packet decode is a project requirement: extended advertising, complete LL
control semantics, L2CAP reassembly, ATT/GATT, SMP, encryption, LE 2M/Coded
PHY, and Bluetooth Classic BR/EDR layers will be added incrementally while
retaining undecoded packet bytes losslessly.

Active signal injection and transmit support are intentionally deferred until
receive, timestamping, channelization, and packet validation are reliable;
transmit will be introduced as a separate subsystem rather than folded into the
receive API.

See `DesignLog.md` for architectural decisions and `ChangeLog.md` for completed
increments. `Verification.md` records independent cross-implementation checks.
