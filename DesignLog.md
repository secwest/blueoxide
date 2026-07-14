# Blueoxide Design Log

This log records architectural choices, their rationale, known limitations, and
the conditions that should trigger revisiting them.

## 2026-07-13: Establish a buildable receive core

### Context

The initial repository contained standalone sketches for LimeSDR, bladeRF,
XTRX, HackRF, and a channelizer. There was no Cargo package, shared device
trait, test suite, executable contract, or documentation of which paths were
expected to work. The sketches referenced several wrapper crates and native
libraries but did not define dependencies or compile together. Some routines
also made unsafe SIMD and buffer-layout assumptions.

### Decision

Create a stable Rust package around protocol and DSP code that builds without
third-party Rust dependencies. Keep the original root-level files as historical
input until each useful behavior has been migrated and tested.

### Consequences

- `cargo build`, `cargo test`, and `cargo clippy` can validate the core without
  SDR hardware or vendor SDKs.
- Hardware support is not claimed merely because a prototype file exists.
- The original files remain visible for provenance, but are excluded from the
  Cargo module tree.
- Native SDR libraries will still be required for actual hardware access. The
  dependency policy is to avoid unnecessary wrapper crates, not to reimplement
  USB kernels or vendor firmware.

### Revisit when

The legacy files have no remaining unique information. At that point they can
be removed in a dedicated, documented cleanup.

## 2026-07-13: Receive and capture first; transmit later

### Decision

The current architecture is receive-first. Passive over-the-air reception is
the initial operating mode, but it is not a permanent project limitation.
Signal injection and transmit support will be introduced later as a separate
subsystem with its own configuration, timing, calibration, and safety controls.

### Rationale

Reliable receive, timestamping, sample-continuity reporting, demodulation, and
packet validation are prerequisites for measuring a transmitter. Separating RX
and TX contracts also prevents receive interfaces from accumulating ambiguous
duplex state and makes it possible to apply explicit controls to active RF use.

### Consequences

- `IqSource` describes receive streams only.
- Future transmit work should introduce an `IqSink` or radio session layer
  rather than adding write methods to `IqSource`.
- Full-duplex devices can later compose source and sink capabilities without
  forcing half-duplex devices into an inaccurate abstraction.

## 2026-07-13: Hardware-neutral receive contract

### Decision

All radio backends will implement `sdr::IqSource`. Configuration is validated
against reported device capabilities before streaming. Each read returns
`ReadMetadata`, including first-sample index, dropped-sample count, and overrun
state.

### Rationale

An SDR API that returns only a sample count can silently splice discontinuous
buffers. That is damaging for Bluetooth synchronization, connection following,
and evidentiary capture. Discontinuities must be explicit and must propagate
into packet metadata and capture files.

### Backend requirements

- LimeSuite, libbladeRF, and libxtrx adapters must use direct, reviewed FFI
  isolated behind runtime backend modules.
- Native status and error codes must be preserved in Blueoxide errors.
- Device open, configure, start, read, stop, and close operations must be
  idempotent where practical and unwind resources on partial initialization.
- Actual sample rate, center frequency, bandwidth, gain, and clock source must
  be reported when the native API can quantize requested values.
- Timeouts, short reads, overruns, and device removal are recoverable events,
  not panics.
- Backend tests must include a mock native boundary or recorded fixture so
  error paths can run without hardware.

## 2026-07-13: Implement LE 1M primary advertising first

### Decision

The first complete receive path targets Bluetooth LE 1M primary advertising
channels 37, 38, and 39.

### Rationale

Primary advertising has a fixed access address and CRC initialization value, so
it provides a bounded way to verify channel mapping, GFSK discrimination,
whitening, bit ordering, CRC, and PDU framing before adding connection state.
The result is also immediately useful for receiver bring-up and RF validation.

### Current processing chain

1. Read interleaved complex baseband samples.
2. Compute phase differences with a quadrature discriminator.
3. Average each possible integer samples-per-symbol phase.
4. Estimate a robust two-level slicing threshold.
5. Search normal and inverted bit streams for the advertising access address.
6. Dewhiten using the selected physical channel.
7. Parse the primary advertising header and bounded payload length.
8. Emit only packets with a valid Bluetooth LE CRC.

### Limitations

- Sample rate must currently be an integer multiple of 1 MHz, from 2 to 64
  samples per symbol.
- Timing selection is exhaustive integer-phase search, not a streaming timing
  recovery loop.
- Carrier offset is absorbed by the slicing threshold only while the two GFSK
  levels remain separable.
- The decoder does not yet report calibrated RSSI, noise estimates, or a
  hardware-correlated wall-clock timestamp.
- LE 2M, LE Coded PHY, secondary advertising, and data-channel PDUs are not yet
  decoded.

### Revisit when

Recorded fixtures from each supported SDR are available. Replace the block
demodulator with a streaming front end that has carrier tracking, symbol timing
recovery, per-packet quality metrics, and exact sample provenance.

## 2026-07-13: CRC validation is mandatory for packet emission

### Decision

Access-address correlation creates candidates; it does not create packets.
Blueoxide emits a BLE advertising PDU only after dewhitening, bounded length
parsing, and CRC validation.

### Rationale

The 2.4 GHz ISM band is busy, and loose access-address matching alone produces
false positives. Mandatory CRC validation gives downstream capture and analysis
code a clear trust boundary. Diagnostic modes may later expose rejected
candidates, but they must be explicitly labeled and kept separate from packets.

## 2026-07-13: Bound offline input allocations

### Decision

The decoder CLI defaults to at most 16 million complex samples per invocation,
with an explicit `--max-samples` override. It rejects partial samples and
non-finite floating-point input.

### Rationale

Capture files can be malformed or unexpectedly large. A default allocation
limit prevents accidental memory exhaustion while the first implementation is
block-oriented.

### Revisit when

The decoder becomes streaming. The limit can then become a bounded block/ring
size rather than a whole-file limit.

## Receive roadmap

Work should proceed in this order unless hardware availability changes the
priority:

1. Add checked-in synthetic and recorded IQ fixtures plus decoder integration
   tests.
2. Make demodulation streaming across input blocks with exact sample indices.
3. Implement and validate LimeSuite, libbladeRF, and libxtrx receive backends.
4. Add wideband channelization for simultaneous BLE channels and efficient
   retuning plans for narrower devices.
5. Write timestamped PCAPNG with receiver metadata and discontinuity records.
6. Parse CONNECT_IND/AUX_CONNECT_REQ and follow LE data-channel hopping.
7. Add LE 2M, secondary advertising, extended advertising, and LE Coded PHY.
8. Add Bluetooth Classic BR/EDR inquiry, access-code correlation, hop
   reconstruction, and packet decoding.
9. Introduce a separately reviewed transmit/injection subsystem.

## 2026-07-13: Standalone means in-tree algorithms and narrow hardware boundaries

### Decision

Blueoxide will implement as much of the receive and decode stack as practical
in Rust within this repository. DSP, channelization, synchronization, protocol
state machines, packet parsing, capture formats, and analysis logic must not
depend on external command-line tools or runtime libraries.

Vendor libraries are permitted only at the device transport boundary while
direct control is not yet independently implementable and testable. They are
not permitted to own Bluetooth DSP or protocol behavior.

### Rationale

Replacing a documented algorithm with an opaque dependency weakens auditability
and reproducibility. Replacing a mature USB/PCIe transport library without
hardware tests can instead reduce reliability or damage device compatibility.
The boundary therefore follows what can be specified, tested, and maintained
in-tree rather than an absolute dependency count.

### Consequences

- PCAPNG is written directly without libpcap.
- BLE CRC, whitening, GFSK discrimination, framing, and semantic parsing remain
  in-tree.
- Hardware FFI modules must translate native samples and status into Blueoxide
  types immediately.
- External projects are verification oracles only and cannot become hidden
  runtime requirements.

## 2026-07-13: Streaming decode owns sample provenance

### Decision

Add `Le1mStreamDecoder` as a bounded streaming wrapper around the tested block
demodulator. Every observation carries the absolute sample index where its
access address begins. Noncontiguous input resets history and returns an
explicit `SampleDiscontinuity`.

### Rationale

Packets frequently cross USB transfer and file block boundaries. Concatenating
buffers without provenance can invent packets across dropped-sample gaps, while
decoding each buffer independently loses boundary packets.

### Consequences

- Maximum-length primary advertisements survive arbitrary normal block splits.
- Identical advertisements at different sample positions are preserved.
- Observations found at several timing phases are merged by sample position and
  packet bytes, retaining the phase with the strongest discriminator
  separation.
- The implementation retains a bounded overlap window rather than the full
  capture.
- Future hardware reads can pass their native sample counters directly to the
  decoder.

## 2026-07-13: Standard PCAPNG is the packet interchange format

### Decision

Write PCAPNG directly using link type 256,
`LINKTYPE_BLUETOOTH_LE_LL_WITH_PHDR`. Store dewhitened LE packets without the
preamble and include the access address, PDU, and CRC. Set flags indicating
reference access address validity, access-address offense validity, and checked
and valid CRC.

### Rationale

PCAPNG provides an interoperable analysis path without adding libpcap as a
runtime dependency. Nanosecond interface resolution preserves SDR-derived
timing.

### Byte order

PCAPNG and the BLE pseudo-header use little-endian multi-octet fields. Blueoxide
reports access addresses as conventional numeric values: transmitted bytes
`78 56 34 12` are displayed as `0x12345678`. Some analysis libraries expose
the same four bytes as the sequence integer `0x78563412`; this is a presentation
difference, not a wire-format difference.

## 2026-07-13: Packet decode is layered and lossless

### Decision

Semantic decoders return typed structures but the original validated PDU bytes
remain available and are always written to capture output. Unsupported or
future PDU types are represented as raw payloads instead of being discarded.

### Initial layer

The first semantic layer decodes legacy ADV_IND, ADV_DIRECT_IND,
ADV_NONCONN_IND, SCAN_REQ, SCAN_RSP, CONNECT_IND, and ADV_SCAN_IND. It parses
AD structures and validates connection interval, latency, supervision timeout,
window, channel map, hop increment, and reserved channel-map bits.

### Planned layers

1. Extended and periodic advertising headers.
2. LE data-channel headers, channel selection algorithms, and LL control.
3. L2CAP signaling and reassembly.
4. ATT/GATT, SMP, and common assigned-number representations.
5. Bluetooth Classic baseband, LMP, L2CAP, and observable profile protocols.

Encryption metadata and undecodable ciphertext must remain lossless even when
keys are unavailable.

## 2026-07-13: Independent implementations are required verification oracles

### Decision

Protocol and capture-format changes must be checked against at least one
independent implementation or authoritative fixture when practical. The
resulting fixed vectors belong in the Blueoxide test suite.

### Evidence for this increment

- CRC and whitening vectors were generated by Jiao Xianjun's independent BTLE
  PHY implementation at commit
  `85401861e8f4b04b90cbaa0394c0f9d45ed02f18`.
- PCAPNG structure follows libpcap link-type definitions at commit
  `f283b98e2292c1577cf6a436b1c3915ac01d9e1a`.
- Scapy 2.7.0, tag commit
  `1de09fe85fe5c9d60ea5c6de130374e170b5bc28`, independently parsed generated
  files as ADV_IND and BTLE_CONNECT_REQ and agreed on packet bytes, CRC,
  interval, and hop increment.

Detailed commands and results are recorded in `Verification.md`.

## 2026-07-13: Load vendor libraries dynamically

### Decision

Use a small in-tree dynamic loader for the native SDR transport boundary rather
than link-time dependencies or Rust wrapper crates. Windows uses
`LoadLibraryW`/`GetProcAddress`/`FreeLibrary`; Unix platforms use
`dlopen`/`dlsym`/`dlclose`.

### Rationale

Protocol, DSP, file decoding, tests, and documentation must remain buildable on
machines without every vendor SDK. Dynamic loading makes backend availability a
runtime capability while retaining exact control over the reviewed ABI.

### Safety boundary

- Every loaded function type is transcribed from a pinned official header.
- A backend struct owns the library handle for at least as long as all copied
  function pointers.
- Symbol lookup is the only generic unsafe conversion from data pointer bits to
  a typed function pointer.
- Native device handles never enter protocol or DSP modules.
- Native buffers are sized with checked arithmetic before an FFI call.
- Mock-native tests exercise lifecycle and error paths without loading a vendor
  library.

### Consequences

The loader itself has no third-party dependency, but live operation still
requires the vendor library and device firmware. ABI changes require updating
the pinned source revision, reviewing every signature and layout, and rerunning
backend tests.

## 2026-07-13: First live backend uses bladeRF metadata RX

### Decision

Implement bladeRF reception through libbladeRF's synchronous
`BLADERF_FORMAT_SC16_Q11_META` API. Set `BLADERF_META_FLAG_RX_NOW` for every
continuous read, convert interleaved Q11 I/Q to `Complex32`, and propagate the
metadata timestamp, contiguous sample count, overrun flag, and inferred sample
gaps into `ReadMetadata`.

### Rationale

The metadata format exposes the free-running FPGA sample counter needed to
detect discontinuities. The non-metadata format could receive samples but could
not meet the capture provenance contract.

### Error policy

- Native timeout is a recoverable empty read.
- Other nonzero native status values become `Error::NativeCall` with operation,
  code, and the vendor error string.
- A success result with a null device, excessive returned sample count, or
  timestamp overflow is rejected as a native contract violation.
- Capture always calls `stop` after read, decode, or packet-output failure.
- `Drop` retries RX disable when necessary and always closes the device.

### Channel-layout decision

The backend exposes one receive channel because `BLADERF_RX_X1` maps to RX0.
Although bladeRF 2 hardware has two receivers, selecting RX1 correctly requires
an X2 stream and explicit deinterleaving. Reporting two usable channels before
that implementation would allow a configuration that enables RX1 while reading
RX0 data.

### Applied-rate decision

libbladeRF reports its applied sample rate and bandwidth. Blueoxide records
both and refuses to start the LE demodulator when the applied sample rate does
not exactly match the requested demodulator rate. This favors explicit failure
over silently decoding with an invalid symbol clock.

### Verification limitation

The native ABI and state machine are verified against the pinned official
libbladeRF source and mock-native tests. No bladeRF library or physical radio
was available in the development environment, so over-the-air behavior remains
an explicit pending hardware verification item.
