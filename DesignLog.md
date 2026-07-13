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
  behind Cargo features.
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
- Duplicate suppression currently uses packet bytes, so identical repeated
  advertisements in one input buffer collapse into one result.
- The decoder does not yet preserve absolute sample offsets through every timing
  phase, RSSI, noise estimate, or wall-clock/hardware timestamps.
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

