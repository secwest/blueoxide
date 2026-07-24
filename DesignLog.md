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

### Initial limitations, partly superseded

- Primary advertising remains LE 1M and requires an integer multiple of 1 MHz,
  from 2 to 64 samples per symbol.
- Timing selection is exhaustive integer-phase search, not a streaming timing
  recovery loop.
- Carrier offset is absorbed by the slicing threshold only while the two GFSK
  levels remain separable.
- The decoder does not yet report calibrated RSSI, noise estimates, or a
  hardware-correlated wall-clock timestamp.
- This first increment omitted data-channel PDUs and LE 2M. Later entries add
  both for fixed-PHY offline recordings; LE Coded PHY and secondary
  advertising remain unsupported.

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
7. Add secondary advertising, extended advertising, and LE Coded PHY; LE 2M
   fixed-channel data decode is now complete.
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

## 2026-07-13: LimeSDR uses device-reported ranges and calibrated F32 RX

### Decision

Implement LimeSDR reception through the runtime-loaded LimeSuite C API using a
single `LMS_FMT_F32` stream per selected RX channel. Query the opened device for
its channel count, LO range, host sample-rate range, and analog LPF range rather
than publishing one static capability set for every LimeSDR model.

### Configuration order

1. Initialize the device.
2. Enable the selected RX channel.
3. Set and read back the host sample rate.
4. Set the LO frequency.
5. Set and read back the analog LPF bandwidth.
6. Set gain.
7. Run RX calibration using at least 2.5 MHz calibration bandwidth.
8. Create the F32 stream and start it only after all previous steps succeed.

The 2.5 MHz calibration floor follows SoapyLMS7's activation path. It does not
change the requested 2 MHz BLE receive LPF.

### Timestamp and status policy

`LMS_RecvStream` metadata reports the hardware counter value for the first
returned sample. Blueoxide compares that value with the expected next counter
to derive exact missing-sample counts and detect backward movement. After each
nonempty read it calls `LMS_GetStreamStatus`; native FIFO overruns and dropped
packet counts mark the read discontinuous even when the exact sample gap cannot
be inferred. Empty timeout reads do not query status because LimeSuite resets
those counters when status is read.

### Error and cleanup policy

- A zero-sample receive is a timeout/empty read; a negative return is a native
  error.
- Non-finite native F32 values, excessive returned counts, invalid range
  metadata, zero stream handles, and timestamp overflow are contract errors.
- Failed configuration disables the channel. Residual teardown state is
  retained and retried before a later configuration instead of being
  overwritten.
- Reconfiguration destroys the previous stream and disables its channel before
  enabling another.
- Drop stops, destroys, disables, and closes in that order.

### Verification limitation

The ABI and behavior are checked against LimeSuite commit
`699d05b7212aa612a9802c219dd6621be88c77db`, its SoapyLMS7 integration, and
gr-limesdr commit `244c6bf4f1cb52a8b4d27240d7a4c88c9542cbbb`. No LimeSDR hardware or installed
LimeSuite library was available for an over-the-air test.

## 2026-07-13: XTRX receive uses SISO INT16 with visible discontinuities

### Decision

Implement XTRX reception through the runtime-loaded libxtrx high-level API.
Use `XTRX_WF_16` on the transport and `XTRX_IQ_INT16` on the host, then convert
the interleaved signed I/Q values to `Complex32` in-tree with the same
`1 / 2048` scale used by libxtrx's own 16-bit-to-float path.

The backend exposes both physical receive channels as independent SISO sources.
Channel A uses `XTRX_RSP_SISO_MODE`; channel B adds `XTRX_RSP_SWAP_AB`, matching
SoapyXTRX's channel-selection convention. It does not expose an interleaved
dual-channel stream through the single-channel `IqSource` contract.

### Configuration and gain policy

1. Set RX sample rate with TX disabled and retain the actual returned rate.
2. Tune the shared RX LO and validate the returned frequency.
3. Tune the selected channel's RX bandwidth and retain the actual value.
4. Map the generic gain setting to the selected channel's LNA stage.
5. Select the automatic RX antenna for that channel.
6. Initialize native run parameters, override them for RX-only SISO, and start.

The published capabilities follow SoapyXTRX at the pinned libxtrx revision:
30 MHz through 3.8 GHz, two RX channels, RX rates through 80 MHz with the
unsupported 56.250001 through 61.437499 MHz gap rejected, and 1 through 60 MHz
RX bandwidth. Generic gain is deliberately limited to the LNA's 0 through
30 dB range. TIA/PGA-specific controls should be exposed separately rather
than hiding multiple stages behind one ambiguous number.

### Timeout and discontinuity policy

Every receive requests `RCVEX_TIMOUT`, `RCVEX_DONT_INSER_ZEROS`, and
`RCVEX_DROP_OLD_ON_OVERFLOW`. Negative timeout errno values are recoverable
empty reads. Other negative results terminate capture with the native code.

For successful reads, Blueoxide combines:

- `out_first_sample` continuity against the expected next sample.
- `out_resumed_at - out_overrun_at` when libxtrx reports an overflow event.
- Any native event bits, including unexpected filled-zero events.

The largest exact gap is reported as dropped samples. Backward timestamps mark
an overrun without wrapping subtraction. Invalid native overflow intervals and
timestamp arithmetic overflow are contract errors.

### Lifecycle and verification limitation

Drop stops RX before closing the device. Configuration and run failures leave
the state retryable without claiming a live stream.

The ABI and behavior are checked against libxtrx commit
`d9599fbf5be2714e6933c5a15acb3d8c57669859`, its bundled SoapyXTRX backend, and
gr-osmosdr commit `aa95a6b568e04d3d15a3b4b055562ffa611c217f`. No installed
libxtrx library or physical XTRX was available for an over-the-air test.

## 2026-07-15: Generalize LE framing before live connection following

### Decision

Use one bounded LE 1M frame decoder for advertising and connection data PDUs.
The frame configuration supplies the access address, CRC initializer, and PDU
layout. Keep the advertising APIs as compatibility wrappers while exposing a
generic streaming decoder for known connection parameters.

Add pure connection-state primitives before attempting hardware retuning:

- Validated 37-bit data-channel maps.
- Channel Selection Algorithms #1 and #2.
- CONNECT_IND ChSel interpretation.
- First transmit-window bounds relative to the end of CONNECT_IND.
- Event offsets relative to an observed connection anchor point.

### CTEInfo boundary

For data PDUs with CP set, CTEInfo is a separate octet after the two-octet data
header. The Length octet counts the following payload and optional MIC, not
CTEInfo. CRC and whitening cover CTEInfo in its over-the-air position.

`LePdu` and `DataChannelPdu` therefore retain CTEInfo separately. Semantic
helpers expose CTE time, type, and RFU state, but CRC-valid reserved values are
not discarded. This keeps captures lossless and lets later policy distinguish
malformed, unsupported, and newly assigned values.

L2CAP and LL control views are plaintext hints rather than proof that the
connection is unencrypted. The framing layer retains ciphertext and MIC bytes
unchanged; authoritative semantic decoding requires later encryption-state and
reassembly support.

### Timing boundary

CONNECT_IND determines the first transmit window, but not the exact anchor
point inside that window. The API exposes the window start and end relative to
the request and exposes later event spacing only relative to an observed
anchor. It deliberately does not present the beginning of the first window as
the established connection schedule.

### Current limitation

`decode-data` decodes a recording already centered on one known data channel.
This increment did not yet acquire the first packet's anchor point, follow
clock drift, or retune a radio across connection events. Those steps require
additional synchronization state plus wideband channelization or a timed-retune
backend contract built on the framing and selection primitives added here.

## 2026-07-15: Anchor connection-event state before changing SDR contracts

### State model

`ConnectionTracker` starts from an event counter and the exact hardware sample
where that event's access address was observed. It advances a separate 64-bit
internal event index while preserving the 16-bit on-air counter wrap. Expected
sample positions are calculated from the current anchor and interval with
checked integer arithmetic and one final nearest-sample rounding step, avoiding
per-event rounding drift.

CONNECT_IND can construct this tracker only after the caller has supplied an
observed anchor. The first transmit-window boundary remains a search bound, not
an invented anchor.

### Instant handling

The Core modulo ordering is represented explicitly as future, reached, passed,
or ambiguous. A passive receiver accepts any unambiguously future instant,
including a retransmission observed fewer than six events before the instant;
the six-event value used when initiating procedures is not imposed on received
PDUs. Reached, passed, and the two half-range ambiguous differences are
rejected.

Only one instant-based update may be pending. This conservative restriction
avoids silently choosing an ordering for overlapping procedures that the
tracker does not yet model.

At an LL_CHANNEL_MAP_IND instant, the new validated map is installed before
selecting that event's channel. At an LL_CONNECTION_UPDATE_IND instant, the new
interval, latency, and timeout become active, but timing enters an explicit
anchor-observation-required state carrying WinOffset and WinSize. The tracker
cannot advance again until the caller supplies the access-address sample
actually observed at that instant. A later increment extends the same pending
instant model to directional `LL_PHY_UPDATE_IND` state.

### Hardware boundary

The current `IqSource` contract supports configure/start/read/stop but no timed
retune while streaming. Connection-event state therefore remains pure protocol
logic plus an offline `connection-plan` command. Live following will require
either simultaneous wideband channelization or an explicit timed-tuning
contract; this increment does not pretend that repeated stop/configure/start
operations provide connection-event timing.

## 2026-07-16: Acquire and maintain anchors from bounded observations

### Clock model

Map CONNECT_IND SCA values 0 through 7 to the Core worst-case bounds of 500,
250, 150, 100, 75, 50, 30, and 20 ppm. The passive receiver adds its own
declared sample-clock error to the peer bound.

For an elapsed interval and sample rate, calculate the one-sided uncertainty as:

```text
ceil(elapsed_us * combined_ppm * sample_rate_hz / 1e12)
```

The calculation uses checked integer arithmetic. Connection-event widening is
capped at half the connection interval minus the 150 us inter-frame spacing,
matching the controller constraint that uncertainty must not consume adjacent
events. Every successful packet observation resets the elapsed-time origin, so
uncertainty grows only across events that were actually missed.

### First-event acquisition

A decoded legacy CONNECT_IND has a fixed 344 LE 1M symbols from the first
access-address bit through the end of its CRC. Starting from the demodulator's
exact access-address sample, add that frame length and the request's WinOffset /
WinSize to obtain event 0's nominal search window.

Expand both sides conservatively using the combined clock error evaluated at
the end of the transmit window. Accept the anchor only from a caller-identified,
CRC-valid central transmission on CSA#1/CSA#2's event-0 channel inside those
bounds. A peripheral response can also fall inside the broad window and is not
an anchor. Blueoxide does not yet infer packet direction; the typed library
input and `--central-observe` CLI option make that precondition explicit. The
earliest decoded packet is typically the central transmission only when the
capture is known not to have missed it. The window beginning remains a bound;
the observed central access-address sample becomes the anchor.

### Missed-event synchronization

Search a caller-bounded number of future events using both selected channel and
clock-widened sample range. Choose the matching event with the smallest absolute
timing error, reject equal-distance ambiguity, and leave the original tracker
unchanged when no match exists. On success, advance all pending protocol state
to that event and re-anchor at the observed access-address sample.

An ordinary observation search stops when it reaches a
LL_CONNECTION_UPDATE_IND whose new anchor has not been observed. It does not
search across unknown timing. The explicit connection-update anchor path
remains responsible for resuming progression.

### Offline and live boundaries

`connection-acquire`, `connection-sync`, and the widened `connection-plan`
exercise acquisition, missed-event recovery, and re-anchoring without hardware.
The remaining live problem is delivering data-channel observations at the
required frequencies. That still requires validated wideband channelization or
a backend contract with measurable timed retuning.

## 2026-07-16: Reassemble L2CAP PDUs only across explicit plaintext directions

### Layer boundary

LLID `0b10` starts a complete or fragmented L2CAP PDU and carries its
little-endian Length and CID header. Nonempty LLID `0b01` PDUs continue that
same L2CAP PDU; a zero-length LLID `0b01` is an empty link-layer PDU and adds no
fragment bytes.

The result is named `L2capPdu`, not `L2capSdu`. Link-layer reassembly restores
the L2CAP header's declared payload. Some L2CAP channel modes can apply their
own segmentation above that boundary, so calling every reconstructed payload
an SDU would overstate what has been decoded.

### Direction and encryption contract

Central-to-peripheral and peripheral-to-central traffic has independent
fragmentation and sequence state. `L2capReassembler` therefore requires an
explicit `LinkDirection` on every packet and stores independent state for both
directions. It does not infer direction from timing or packet contents.

The reassembler also cannot determine whether Length-counted bytes are
plaintext, ciphertext, or include a MIC. Its API requires plaintext or
already-decrypted packets. The `decode-data` integration is opt-in through
`--plaintext-l2cap-direction`, making the caller assertion visible instead of
silently interpreting every CRC-valid payload.

### Loss and recovery policy

The state machine enforces a configurable maximum no larger than the 16-bit
L2CAP payload length, rejects start/continuation overflow, and reports
orphaned continuations. A new valid start replaces and reports an incomplete
PDU, allowing recovery at the next framing boundary. Exact consecutive
retransmissions are suppressed using LLID, SN, CTEInfo, and payload while
ignoring acknowledgement/header fields that can change on retransmission.

Visible sample discontinuities reset reassembly and report the discarded
partial PDU. Invisible packet loss cannot be repaired from a one-bit sequence
number. In particular, a recording centered on one data channel is normally
incomplete for a hopping connection. CLI reassembly is authoritative only when
the caller knows the input contains every ordered packet in the asserted
direction.

## 2026-07-17: Decode one strict command per LE signaling PDU

### Signaling envelope

LE fixed channel CID `0x0005` carries one signaling command in each L2CAP PDU.
The decoder requires the complete four-octet Code, Identifier, and Length
header, rejects identifier zero, and requires Length to equal every remaining
payload octet. It does not scan for concatenated commands or accept trailing
bytes. This follows the LE signaling receive path rather than the more general
BR/EDR signaling model.

The borrowed `L2capSignalingCommand` always retains raw code, identifier, and
parameters. Unknown command codes therefore decode to an explicit raw variant
instead of failing or discarding bytes. Known commands use exact fixed sizes or
bounded variable layouts before exposing typed fields.

### Enhanced Credit Based lists

Enhanced Credit Based Connection and Reconfigure requests require one through
five source/channel IDs, an even parameter length, and nonzero IDs. Responses
also require one through five destination-ID entries, but zero is preserved:
`0x0000` means that the corresponding requested channel was not established.
Rejecting zero response entries would discard valid partial-refusal responses.

Connection Parameter Update Requests reuse the existing Core interval,
latency, supervision-timeout, and timeout-relationship validation, with an
additional minimum-interval versus maximum-interval ordering check.

### Output boundary

`decode-data` attempts signaling decode only after the caller has enabled
direction-explicit plaintext L2CAP reassembly and a complete CID `0x0005` PDU
has been reconstructed. It prints the lossless `l2cap_pdu` first, then a
separate `l2cap_signal` line. Envelope or known-command errors are counted and
reported independently; they cannot remove packet or PDU output.

## 2026-07-17: Decode ATT syntax without inventing GATT or EATT state

### Fixed-channel boundary

The fixed ATT bearer uses L2CAP CID `0x0004`. `AttPdu` borrows the opcode and
all remaining parameter bytes from a completed `L2capPdu`; unknown opcodes
remain explicit raw variants. ATT interpretation occurs only after the caller
has opted into direction-explicit plaintext reassembly. Encrypted bytes are not
probed for plausible opcodes.

Enhanced ATT uses dynamically allocated LE credit-based channels. Recognizing
those channels requires connection state that associates an established CID
with the EATT PSM. Payload shape alone is insufficient, so `att_pdu()` only
recognizes fixed CID `0x0004` and does not guess EATT.

### Strict syntax and truncation

Every assigned Core 6.1 ATT opcode has a typed layout. Exact-size PDUs reject
both truncation and trailing bytes. Variable layouts enforce nonzero handles,
ordered handle ranges, 2- or 16-octet UUIDs, complete fixed records, at least
two handles for both multiple-read requests, valid Execute Write flags, and a
complete 12-octet signed-write authentication signature.

Two superficially similar tuple lists require different handling. A Multiple
Handle Value Notification contains two or more complete handle/length/value
tuples and may not truncate a tuple. A Read Multiple Variable Length Response
may truncate only the final tuple's value; if the MTU boundary would split its
two-octet Value Length, the entire tuple is omitted. The typed value therefore
retains both the declared length and the bytes actually present, with an
explicit truncation flag.

Exchange MTU fields are 16-bit receive capacities. Core 6.1 requires values of
at least the default ATT MTU of 23 but does not impose 517 as a syntax maximum.
The often-used value 517 follows from a 512-octet GATT attribute plus ATT
overhead; it is not the maximum encodable receive MTU. The stateless parser
accepts all values from 23 through `0xffff` and does not enforce a previously
negotiated bearer MTU.

### Output and state limits

`decode-data` prints a complete raw `l2cap_pdu` before attempting ATT decode,
then emits a separate `att_pdu` line with opcode name, method type, and typed
fields. Empty fixed-channel payloads and malformed known PDUs increment a
separate ATT error counter without suppressing bytes.

This increment does not perform GATT service discovery, infer characteristic
semantics, maintain request/response transactions, track negotiated MTU, or
verify signed-write authentication. Those require state above the lossless ATT
syntax layer established here.

## 2026-07-19: Decode LE SMP commands without claiming pairing state

### Transport boundary

LE carries the Security Manager Protocol on fixed L2CAP CID `0x0006`.
`SmpPdu` borrows the one-octet command code and all remaining parameters from a
completed plaintext `L2capPdu`. Reserved future command codes remain raw
variants. The separate BR/EDR Security Manager channel is not inferred or
decoded by this fixed-LE helper.

As with ATT, interpretation occurs only after direction-explicit plaintext
reassembly. SMP payloads may become encrypted after pairing starts, and the
decoder does not probe ciphertext for plausible command codes.

### Syntax validation

All 14 commands assigned through Core 6.1 have exact typed layouts. Pairing
Request and Response validate IO Capability, OOB presence, the two-bit bonding
field, reserved AuthReq bits, the 7-through-16-octet key-size range, and
four-bit key-distribution masks. Pairing fields allow CT2 in AuthReq; Security
Request uses the same security flags but reserves CT2.

Fixed cryptographic fields retain their exact byte order and widths: 128-bit
confirm/random/key/check values, 16-bit EDIV plus 64-bit Rand, and 256-bit X and
Y public-key coordinates. Identity Address Information accepts only public or
static-random address types and verifies the static-random marker and
nondegenerate random portion.

Core 6.1 extends Pairing Failed reasons beyond older parser tables with `0x0f`
Key Rejected and `0x10` Busy. Both are named. Other reserved reasons and
keypress values are rejected, while an entirely unknown command code remains
lossless for forward compatibility.

### Deliberate state boundary

The syntax layer does not decide whether a command is legal for the observed
role or pairing phase. It does not correlate request/response feature masks,
enforce key-distribution order, run the 30-second Security Manager timer,
derive keys, verify confirm/DHKey values, or authenticate distributed identity
material. Those operations require a connection-scoped pairing transcript and
cryptographic state.

`decode-data` prints `l2cap_pdu` before `smp_pdu`. Known-command failures are
counted independently as `smp_errors` and cannot suppress raw reconstructed
bytes. Since those bytes can contain long-term and identity keys, all SMP
capture output is sensitive even when semantic decoding fails.

## 2026-07-19: Type LL control syntax before adding procedure state

### Framing boundary

An LL control PDU is already bounded by a CRC-valid data-channel PDU with LLID
`0b11`. Its first payload octet is the Opcode and every remaining octet is
CtrData. Unlike ATT, signaling, and SMP, this decode does not depend on L2CAP
reassembly. `ControlPdu::decode` therefore operates directly on the complete
Length-counted link-layer payload while the original `DataChannelPdu` retains
the opcode and CtrData bytes.

Every pre-Channel-Sounding command from `0x00` through `0x2c` has an exact
typed layout. Known commands reject both short and trailing CtrData. The Core
6.1 opcode table is named through `LL_FRAME_SPACE_RSP` (`0x3c`); assigned
Channel Sounding and Frame Space opcodes remain explicit raw payloads rather
than being mislabeled as unknown or guessed from incomplete state.

### Core 6.1 corrections

Older implementation tables are not sufficient for the current layouts.
Core 6.1 gives `LL_CIS_REQ` a Framing_Mode bit next to Framed, reducing that
octet pair's RFU field from three bits to two. It defines
`LL_PERIODIC_SYNC_WR_IND` as the 34-octet `LL_PERIODIC_SYNC_IND` CtrData plus
RspAA and four PAwR timing octets, for 42 parameter octets total. Feature Page
Exchange uses MaxPage, PageNumber, and a 24-octet FeaturePage, for 26 parameter
octets rather than an eight-octet legacy feature mask.

The syntax layer validates reserved bits and value relationships that do not
require connection history: connection parameter ranges and ordered unique
offsets; data-length limits; PHY masks; CTE length/type; SyncInfo map, interval,
and offset flags; CIS framing, SDU/PDU, NSE, BN, FT, ISO interval, and offset
bounds; power PHY/flag values; subrate factors, latency, and continuation
numbers; channel-reporting timing; channel classifications; and feature-page
numbers.

### State and output boundary

This parser does not infer link direction or role, decide whether a procedure
is legal in the current state, apply PHY/subrate/CIS changes, compare instants
to an observed event counter, construct session keys, or decrypt subsequent
packets. Existing connection tracking continues to schedule only
`LL_CONNECTION_UPDATE_IND` and `LL_CHANNEL_MAP_IND`.

`decode-data` always prints the raw packet. A malformed known control command
adds `decode_error` to its plaintext hint and increments
`ll_control_errors`; it cannot suppress capture bytes. Encryption request and
response descriptions expose Rand, EDIV, SKD, and IV fields, so LL control
output is sensitive even before higher-layer keys are available.

## 2026-07-19: Type Channel Sounding and Frame Space syntax statelessly

### Layout and ownership

Core 6.1 assigns sixteen more LL control opcodes from `LL_CS_SEC_RSP` (`0x2d`)
through `LL_FRAME_SPACE_RSP` (`0x3c`). Their parameter layouts range from the
zero-octet FAE request to the 72-octet signed per-channel FAE response. They are
now typed in `src/ll_control/cs.rs`, while `src/ll_control.rs` remains the
public dispatch point. Only future opcodes remain `Raw`.

The CS types retain every valid wire field, including the 20-octet security
triplet, capability masks, ten-octet CS channel map, packed configuration
fields, 24-bit offsets and subevent lengths, signed power delta and FAE values,
termination reason, and Frame Space masks. CLI descriptions expose all of
these values. Security IV, nonce, and personalization-vector output is
sensitive capture material.

### Strict single-PDU validation

The decoder rejects short or trailing layouts, RFU bits, reserved capability
masks and values, impossible antenna/role counts, invalid CS channel maps,
unsupported mode pairings, malformed timing-index selections, invalid
algorithm #3c shape/jump/repetition combinations, bad offsets/subevent fields,
reserved ACI/PHY/SNR values, and invalid Frame Space ranges or masks.
`LL_CS_CONFIG_REQ` removal requires all fields that become RFU to be zero.

Some mandatory timing values are implicit rather than represented in the
capability masks: T_IP1/T_IP2 index 7, T_FCS index 9, and T_PM index 2.
Accordingly, capability masks validate only the optional indices while
configuration PDUs accept the complete index ranges. The wire
TX_SNR_Capability field has an RFU least-significant bit; the typed value shifts
that bit out so bit zero denotes SNR output index zero.

`LL_CS_RSP` inherits the request's 500 microsecond minimum and ordered offset
rules. `LL_CS_IND.Offset` does not have that minimum and may be zero. Checks
that depend on the active connection interval, current event counter,
previously exchanged capabilities, or a selected antenna configuration are
left to future connection-scoped procedure state.

### Independent checks and state boundary

The field order was compared with pinned Google RootCanal PDL and Texas
Instruments packed CS structures. RootCanal's private emulation opcodes and
synthetic status fields are not over-the-air fields. TI omits the Core 6.1
trailing RFU octets from CS response/indication structures, so the official
Core figures govern those lengths. Pinned Zephyr capability definitions
independently confirm optional timing and TX-SNR masks, and Bumble confirms the
assigned public opcode table.

This increment does not implement the Channel Sounding procedures. It does not
decide whether the sender may initiate a PDU, correlate request/response
values, choose an ACI, apply a CS channel map or Frame Space value, validate an
instant against the observed event, derive CS security state, or schedule CS
subevents. Those operations require connection history and negotiated local
and remote capabilities.

## 2026-07-21: Decrypt LE ACL only from explicit authenticated state

### Cryptographic boundary

LE ACL payload protection is AES-128 CCM with a four-octet MIC. The 13-octet
nonce consists of the 39-bit direction-specific packet counter in little-endian
order, the transmitter direction in the high bit of counter octet four, and the
eight-octet Link Layer IV. Central-to-peripheral uses direction bit one;
peripheral-to-central uses zero. The one-octet associated data is the first
data-channel header octet masked with `0xe3`, authenticating LLID, CP, and RFU
while excluding NESN, SN, and MD.

The dependency-free core now contains an in-tree AES-128 block cipher and the
fixed LE CCM profile rather than a general cryptographic API. The public
`LeAclDecryptor` accepts an already-derived session key, combined IV, explicit
`LinkDirection`, initial 39-bit counter, and bounded maximum counter skip. It
does not accept an LTK or claim to reconstruct the encryption procedure.
The private table-based AES implementation is scoped to offline capture
analysis; it is not exposed as a general-purpose or side-channel-hardened
cryptographic service.

### Counter and retransmission state

Each transmitter direction has an independent packet counter. A new nonempty
packet advances state only after MIC verification. If SN matches the last
authenticated packet, the decryptor first retries that packet's counter;
successful authentication identifies a retransmission and leaves the next
counter unchanged. A zero-length data PDU bypasses CCM and consumes no counter.

Capture loss can make a caller's next expected counter stale. The optional
bounded search tries the expected counter through the configured skip limit and
accepts only a MIC-valid result. The CLI caps that search at 65,535 counters to
bound CPU work. A successful skip reports the exact number of absent encrypted
counter values. A failed search changes no cryptographic state.

SN alone is not used to infer how many packets were missed. Empty packets can
change sequence behavior without consuming an encryption counter, and a
fixed-channel recording can miss arbitrary hopping events. The MIC is the only
acceptance oracle.

### Lossless output and higher-layer state

`decode-data` always emits and captures the original CRC-valid ciphertext and
MIC. When decryption is configured, that raw line is marked `encrypted`; a
separate `decrypted_data` line reports direction, new/retransmission/empty
status, authenticated counter, skipped counters, adjusted plaintext Length,
and plaintext bytes. Only authenticated plaintext reaches LL control and
L2CAP/ATT/signaling/SMP decoding.

A decryption failure or a successful nonzero counter skip represents a
plaintext stream gap. The CLI discards and reports an incomplete L2CAP PDU
before accepting later plaintext. PCAPNG remains an over-the-air ciphertext
record rather than being rewritten with synthetic decrypted packets.

### Deliberate state boundary

The decryptor does not infer packet direction, discover the initial counter,
combine `LL_ENC_REQ` and `LL_ENC_RSP` into procedure state, derive a session key
from an LTK/SKD, pause or refresh encryption, or derive keys from SMP. Callers
must create independent state for each direction. Command-line session keys are
sensitive and can be exposed through shell history or process inspection.

## 2026-07-22: Reconstruct encryption material without overstating procedure state

### Exchange and byte-order boundary

`LeEncryptionMaterialTracker` accepts a caller-selected 16-octet LTK and
explicitly direction-tagged LL control PDUs. Only a central-to-peripheral
`LL_ENC_REQ` followed by a peripheral-to-central `LL_ENC_RSP` can produce
material. Both PDUs pass through the existing strict control decoder, so short,
trailing, or wrong-opcode payloads cannot enter derivation.

The HCI/SMP LTK and parsed SKDm, SKDs, IVm, and IVs arrays preserve their
least-significant-octet-first storage or wire order. The LTK is reversed for
conventional AES key order; the AES plaintext is reversed SKDs followed by
reversed SKDm. Raw IVm followed by raw IVs already forms the eight nonce octets
used by the software CCM implementation. With the Core sample LTK and raw
exchange this produces session key `99ad1b5226a37e3e058e3b8e27c2c666`
and IV `24abdcbabebaafde`, matching the published encrypted ACL examples.

### Passive state policy

An exact repeated request or response is idempotent. A different new central
request invalidates previously derived material and starts a fresh exchange.
A response before a request, a response from the central, a request from the
peripheral, or a different response after material is ready is rejected
without mutating accepted state.

This is deliberately material state, not the complete encryption procedure.
The tracker does not choose an LTK from Rand/EDIV, correlate SMP pairing, infer
direction, model `LL_START_ENC_REQ`/`LL_START_ENC_RSP`, pause or activation
boundaries, initialize two directional packet streams, or infer counters.

### CLI integration

`decode-data` now accepts either the direct `--session-key`/`--iv` pair or
`--ltk` plus complete opcode-bearing `--enc-req` and `--enc-rsp` payloads.
Both modes still require the caller's transmitter direction and initial
packet counter and feed identical derived bytes into `LeAclDecryptor`.
The modes are mutually exclusive, and all key/control arguments remain
sensitive command-line data.

## 2026-07-22: Share uncoded framing while keeping PHY selection explicit

### Demodulation boundary

LE 1M and LE 2M share access-address search, whitening, CRC validation,
data-header parsing, packet deduplication, and bounded stream retention. The
generic `LeUncodedPhy`, `LeUncodedDemodConfig`,
`decode_le_uncoded_detailed`, and `LeUncodedPacketStreamDecoder` APIs select
the symbol rate, nominal deviation, and preamble length. Existing
`Le1mDemodConfig`, `decode_le_1m_detailed`, `Le1mPacketStreamDecoder`, and
advertising stream APIs remain compatibility wrappers.

LE 2M uses 2 Msymbol/s, 500 kHz nominal deviation for modulation index 0.5,
and a two-octet alternating preamble. Primary advertising remains LE 1M. The
bit-level frame decoder begins at the access address, so the longer preamble is
part of PHY synchronization and test-vector construction rather than a second
Link Layer framing format.

### Caller assertion and connection state

`decode-data --phy 1m|2m` is an explicit assertion about a recording already
centered on one known data channel. The decoder validates an integer 2 through
64 samples per selected-PHY symbol, but it does not classify PHY from I/Q,
infer packet direction, or switch PHY within one decoder instance. The
connection tracker described in the next entry can correlate a decoded
`LL_PHY_UPDATE_IND` with an event and instant, but it does not drive the
demodulator.

Applying tracked PHY changes to received I/Q belongs with capture-driven
observation delivery and channel following. Live retuning and decoder switching
remain separate work; this increment decodes a fixed-PHY offline stream without
claiming those capabilities.

### Capture metadata

Each `ReceivedLePdu` retains its asserted PHY so observations from different
PHY configurations cannot deduplicate together. Text output includes
`phy=LE-1M` or `phy=LE-2M`, and PCAPNG sets the two-bit PHY field in the
standard Bluetooth LE Link Layer pseudo-header. Advertising capture continues
to emit the LE 1M value.

## 2026-07-22: Apply directional PHY state at the connection instant

### Typed state

`LePhy` represents the one-hot Link Layer values `0x01` for LE 1M, `0x02` for
LE 2M, and `0x04` for LE Coded. `PhyUpdateInd` stores each direction as
`Option<LePhy>` so zero means unchanged rather than an invented fourth PHY.
The existing strict control decoder and `ConnectionTracker::schedule_control`
share this parser, preventing syntax and state paths from disagreeing.

`ConnectionPhyState` keeps central-to-peripheral and
peripheral-to-central PHYs independently. A tracker created from CONNECT_IND
starts with LE 1M in both directions. `new_with_phy` exists for a passive
receiver that establishes an anchor after earlier procedures and therefore
must assert the already-active directional state.

### Instant policy

A PHY update that changes either direction uses the same wrap-safe future
instant validation and single-pending-update slot as channel-map and
connection-parameter updates. The specified directions are installed before
the instant event is returned; zero fields preserve their existing state. A
valid no-change PDU has Instant zero, completes without reserving pending state,
and does not block a later instant-based update.

The tracker represents LE Coded state even though Blueoxide cannot yet
demodulate LE Coded packets. Protocol state must remain lossless rather than
silently coercing an unsupported PHY to LE 1M or LE 2M.

### Offline and live boundary

`connection-plan`, `connection-sync`, and `connection-acquire` now print both
directional PHYs. Planning and synchronization can assert PHY state at an
arbitrary anchor and schedule one `C2P:P2C:INSTANT` transition;
`connection-acquire` rejects non-1M initial state because event 0 follows
CONNECT_IND.

This is offline protocol scheduling, not automatic receive reconfiguration.
At this increment, `decode-data` still consumed one channel and one asserted
uncoded PHY, while live capture still lacked data observation delivery plus
timed channel/PHY switching.

## 2026-07-22: Reuse the live capture lifecycle for one data channel

### Shared stream boundary

Advertising and data capture now pass their existing stream decoders through
one internal capture engine. The engine owns backend configuration, applied
sample-rate verification, start/read/stop ordering, hardware-counter-relative
packet positions, dropped-sample and overrun accounting, discontinuity counts,
and unconditional stop after read or callback failure. Decoder adapters only
normalize packet batches; they do not duplicate source lifecycle logic.

The public `capture_data_channel` entry point validates channels 0 through 36,
requires a typed data-channel `LeFrameConfig`, and accepts the same explicit
LE 1M/2M demodulator configuration as `decode-data`. The CLI constructs that
frame configuration from the caller's access address and CRC initializer.
`CapturedDataChannelPdu` retains both the absolute hardware access-address
sample and its capture-relative position for timestamp output.

### Fixed-tuning policy

`capture-data` deliberately configures one BLE data-channel center frequency
for the whole finite capture. It does not claim connection following: packets
on other hop channels are absent, direction is not inferred, and decoded
control procedures do not retune the source or replace the asserted PHY.
This provides real backend delivery into the generalized data decoder while
keeping timed retuning or wideband channelization as an explicit later layer.

Raw CRC-valid packets are printed and written to PCAPNG with their asserted
PHY. As in undecrypted offline decoding, LL control and L2CAP interpretations
are labeled plaintext hints; malformed semantic views are reported without
stopping lossless capture.

## 2026-07-22: Associate asserted live central packets with fixed-channel events

### Direction boundary

A receiver tuned to one data channel cannot determine central-to-peripheral
direction from an isolated CRC-valid PDU. Live event association is therefore
disabled unless the caller supplies `--assert-central-observations`. Under that
option every decoded packet is treated as a central-transmission candidate; a
peripheral response must not be passed through this mode as an anchor.

The caller also supplies the first event counter, channel map, CSA selection,
connection interval, and optional timing/search bounds. Configuration is
rejected before native-library loading unless the asserted first event selects
the tuned channel. This prevents the first arbitrary packet from anchoring an
internally inconsistent hopping sequence.

### State and failure policy

The first asserted candidate anchors the supplied event counter at its exact
access-address sample. Later candidates use the existing clock-widened,
bounded `ConnectionTracker::synchronize_observation` search. That search works
on cloned tracker state and commits only a unique match, so a rejected
same-event packet, wrong-time candidate, or unmatched recurrence cannot advance
or re-anchor the live tracker.

Raw packet delivery remains the primary capture contract. Event-association
failure is counted and reported, but it does not suppress packet text, skip
PCAPNG serialization, terminate the SDR stream, or discard later candidates.
Successful matches report the event counter, selected channel, expected and
observed samples, timing error, clock-widened bounds, and skipped-event count.

### Deliberate limits

This bridge does not infer direction, retune among hop channels, switch the
demodulator PHY, or automatically schedule decoded channel-map, connection-
parameter, or PHY control procedures. Those require direction-aware packet
routing and timed radio control beyond a fixed-channel receive stream.

## 2026-07-22: Decode primary extended advertising without implying following

### Semantic boundary

CRC-valid primary-channel PDU type 7 packets are decoded as `ADV_EXT_IND`.
The parser bounds ExtHdrLen against the received payload, rejects reserved
common-header and flag values, and consumes the optional fields in their
specified order. Advertising mode, AdvA, TargetA, CTEInfo, ADI, AuxPtr,
SyncInfo, and signed TxPower have typed representations. Any bytes left inside
ExtHdrLen are retained as ACAD, and all bytes after the extended header remain
lossless advertising data.

ADI retains its 12-bit DID and four-bit SID. AuxPtr retains its data-channel
index, 50/500 ppm clock-accuracy class, 30/300 microsecond offset units,
13-bit offset, and LE 1M/2M/Coded PHY. SyncInfo retains its packet offset,
offset adjustment, periodic interval, validated channel map and sleep-clock
accuracy, access address, CRC initializer, and event counter. Invalid AuxPtr
channels/PHY values, reserved SyncInfo bits, undersized periodic intervals,
invalid channel maps, and every truncated optional field are rejected without
discarding the original CRC-valid packet bytes.

### Receive boundary

An AuxPtr is decoded metadata, not a receive schedule. Blueoxide still receives
only the primary advertising channels in this path and does not retune to a
secondary channel, reassemble AUX_CHAIN_IND data, maintain periodic
advertising synchronization state, or demodulate LE Coded. These require
timestamp-preserving multi-channel delivery and PHY-specific demodulation
beyond semantic parsing of one primary PDU.

### Verification boundary

The wire layouts were checked independently against Zephyr controller commit
`7d46db352251f85a6bc7b5961fb8a86e2f3125e4` and Wireshark commit
`403c9a36ea7fa8f2d69a449be1f4fa97c52817c0`. Fixed CRCs come from Scapy commit
`de3399269bad8c9a6bfb1dc181c3876340c198b8`; the CLI fixture's channel-37
whitening comes from Jiao Xianjun's BTLE implementation rather than
Blueoxide's own helper.

## 2026-07-23: Decode fixed-channel secondary advertising before following

### Framing distinction

Primary and secondary advertising share access address `0x8e89bed6`, CRC
initializer `0x555555`, whitening, and the advertising PDU header shape. They
do not share Length semantics: primary advertising uses the lower six bits and
is bounded to 37 payload octets, while secondary advertising uses the complete
Length octet and may carry 255 payload octets. `LePduLayout` therefore keeps
separate primary and secondary advertising variants rather than weakening the
primary bound or treating auxiliary traffic as connection data.

The generic uncoded demodulator and stream buffer now feed either advertising
layout into a checked `AdvertisingPdu` conversion. Advertising observations
retain the asserted uncoded PHY so LE 2M auxiliary packets set the correct
PCAPNG pseudo-header metadata. Existing primary wrappers remain LE 1M.

### Offline receive boundary

`decode-secondary` accepts a recording already centered on one channel from 0
through 36 and requires an asserted LE 1M or LE 2M PHY. It performs bounded
streaming demodulation, advertising CRC validation, exact sample positioning,
extended-header semantic decode for PDU type `0x07`, and optional PCAPNG
serialization. Configuration errors are rejected before the input file is
opened.

PDU type `0x07` is shared by ADV_EXT_IND, AUX_ADV_IND, AUX_SCAN_RSP,
AUX_SYNC_IND, and AUX_CHAIN_IND. A fixed isolated packet does not carry enough
context to select one of those names reliably, so Blueoxide reports the wire
type and preserves all bytes without inventing an AUX subtype. Legacy primary
PDU meanings are not applied to non-`0x07` packets received on channels 0
through 36.

### Deliberate limits

This increment does not consume an AuxPtr as a receive schedule, retune a
radio, combine multiple channels, classify contextual AUX subtypes, reassemble
advertising chains, establish periodic synchronization state, or demodulate LE
Coded. Those capabilities require timing provenance and cross-packet state;
fixed-channel secondary decode establishes the framing and packet-delivery
layer they can build on.

## 2026-07-23: Follow AuxPtr in offline sample coordinates before radio retuning

### Quantized timing model

AuxOffset is measured from the beginning of the parent packet, while Blueoxide
observations identify the beginning of the access address. For uncoded LE 1M
and LE 2M the preamble is eight microseconds on both PHYs, so an uncoded
parent-to-child access-address delta uses the AuxOffset directly. An LE Coded
child has an 80-microsecond preamble, so its represented access-address window
is shifted 72 microseconds later. Scheduling from an LE Coded parent remains
unsupported because its complete packet airtime depends on coded-PHY details
that the receive core does not yet model.

An encoded offset does not identify one exact instant. It represents the
interval beginning at `offset * unit` and spanning one 30- or 300-microsecond
unit. The public window therefore retains represented earliest/latest samples
instead of inventing a midpoint. Integer conversion floors the lower bound,
ceils the upper bound, and uses checked arithmetic. The widened bounds add the
ceiling of:

```text
AuxOffset_us * (AuxPtr_CA_ppm + receiver_ppm) * sample_rate_hz / 1e12
```

The quantization interval and clock widening remain separate. This follows the
Zephyr scanner model, which calculates drift from AuxOffset and independently
extends reception by the offset-unit window. Zero AuxOffset is not
schedulable. The represented interval must also reach at least the complete
uncoded parent airtime plus the 300-microsecond minimum auxiliary frame space.

### Context supplies the AUX subtype

PDU type `0x07` does not encode AUX_ADV_IND versus AUX_CHAIN_IND. A primary
ADV_EXT_IND AuxPtr establishes AUX_ADV_IND as the first expectation. An
AuxPtr in that AUX_ADV_IND or any later AUX_CHAIN_IND establishes
AUX_CHAIN_IND as the next expectation. Each observation must match the pending
channel, PHY, and widened sample window before its extended header is allowed
to affect state.

ADI SID/DID is the chain identity. When the initiating ADV_EXT_IND supplies
ADI, every auxiliary fragment must match it; a continuing sequence may
establish ADI at AUX_ADV_IND only when the primary omitted it. Any fragment
that points onward must have ADI. AUX_CHAIN_IND is restricted to
non-connectable/non-scannable mode and may not contain AdvA or TargetA.
Connectable or scannable AUX_ADV_IND may not carry a continuation AuxPtr.
Advertiser identity is retained when later chain fragments omit AdvA.

### Transactional reassembly and discontinuities

The tracker validates a cloned candidate and commits it only after all timing,
identity, contextual-field, and configured-size checks pass. A rejected
candidate therefore cannot consume the pending window or append partial data.
Advertising data is accumulated byte-for-byte through a caller-selected
maximum, and the first valid auxiliary fragment without AuxPtr completes the
result. `reset` explicitly discards active or completed state for a sample
discontinuity or a new sequence.

`extended-advertising-plan` exposes this state machine over repeated
CRC-validated header/payload observations sharing one exact sample coordinate
system. It is deliberately separate from `decode` and `decode-secondary`.
Current `IqSource` backends do not prove stop/retune/start timing continuity
and reject reconfiguration while streaming, so this increment does not claim
live AuxPtr following. Timed radio control or timestamp-preserving
channelization is still required to connect the scheduler to hardware.

## 2026-07-23: Synchronize periodic advertising from explicit SyncInfo

### Advertising framing with a nonstandard access address

Periodic advertising keeps the secondary advertising PDU layout but replaces
the standard advertising access address and CRC initializer with the values
carried by SyncInfo. Treating it as a data-channel PDU would incorrectly
enable CP/CTEInfo header semantics; forcing it through the original
`AdvertisingPdu` conversion would discard or reject the actual access address.

`LeFrameConfig::periodic_advertising` therefore combines the SyncInfo access
address and CRC initializer with the complete eight-bit secondary advertising
Length layout. `AdvertisingPdu::from_le_pdu` is the checked representation
boundary for this context, while `TryFrom<LePdu>` deliberately retains the
old standard-address requirement for primary and ordinary secondary
advertising. Packet bytes and the PCAPNG reference-access-address field both
use the retained address.

PDU type `0x07` still does not encode its AUX subtype. The periodic decoder
supplies typed `AUX_SYNC_IND` context to the extended-header parser rather than
rewriting display text or changing isolated `decode-secondary` behavior.

### First-event timing remains an interval

SyncPacketOffset is measured from the beginning of the packet containing
SyncInfo to the beginning of the first AUX_SYNC_IND. The containing packet and
periodic packet use the same PHY, so their preamble durations cancel when both
timestamps identify the beginning of the access address.

An encoded offset represents the interval from `offset * unit` through one
additional 30- or 300-microsecond unit. The tracker floors the lower
sample conversion and ceils the upper conversion. It keeps these represented
bounds separate from clock widening instead of selecting a midpoint.
Offset Adjust contributes 2,457,600 microseconds and is accepted only with
300-microsecond units. Zero offsets and short unadjusted offsets encoded with
300-microsecond units are rejected as unrepresentable or noncanonical.

The initial widening is the ceiling of:

```text
elapsed_us * (advertiser_SCA_ppm + receiver_ppm) * sample_rate_hz / 1e12
```

Later unanchored events add the periodic interval in 1.25 millisecond units.
Widening is capped at half the periodic interval minus the 150 microsecond
inter-frame space so adjacent event searches cannot consume each other.

### CSA#2 progression and transactional re-anchoring

Periodic channel selection always uses CSA#2 with the SyncInfo access address,
channel map, and 16-bit event counter. Event-counter wrapping is natural;
an independent monotonic event index protects anchor-relative timing from the
wrap.

Observation matching searches the current event and a caller-bounded number
of later events. A match requires channel, inherited PHY, and widened sample
window agreement. If multiple candidates have the same absolute timing error,
the observation is ambiguous and rejected. All searching occurs on cloned
tracker state, so channel, PHY, timing, and ambiguity failures cannot advance
the schedule. A unique match replaces the quantized prediction with the exact
observed access-address sample; clock widening for later events then
accumulates from that anchor.

`periodic-advertising-plan` exposes this state over a CRC-validated packet
containing SyncInfo and optional repeated observations.
`decode-periodic` handles one already selected uncoded channel and PHY using
the same access address and CRC. Neither command retunes hardware, combines
channels, discovers the PHY, or demodulates LE Coded.

### Independent behavior boundary

The SyncInfo layout, offset units/adjustment, interval, inherited PHY, channel
map, access address, CRC initializer, event counter, CSA#2 call site, widening
accumulation, and widening cap were checked against Zephyr controller commit
`7d46db352251f85a6bc7b5961fb8a86e2f3125e4`. Wireshark commit
`403c9a36ea7fa8f2d69a449be1f4fa97c52817c0` independently confirms that raw
SyncOffset zero cannot be represented and that AUX_SYNC_IND is contextual.
Scapy commit `de3399269bad8c9a6bfb1dc181c3876340c198b8` generated the fixed
periodic CRC, and Jiao Xianjun BTLE commit
`85401861e8f4b04b90cbaa0394c0f9d45ed02f18` generated its whitening bytes.

## 2026-07-23: Drive LE encryption state from directed capture records

### Capture contract and state ownership

`LeEncryptionSessionTracker` accepts one caller-selected LTK, a bounded
counter-skip limit, and complete `DataChannelPdu` observations tagged with the
actual transmitter direction. It deliberately does not infer direction from
LLID, SN/NESN, packet timing, or control opcode. Those fields are not a
direction oracle, and fixed-channel captures can omit arbitrary connection
events.

The tracker owns the material exchange plus independent
central-to-peripheral and peripheral-to-central `LeAclDecryptor` instances.
Its public state names the next capture-visible procedure packet rather than a
local controller role. The initial states cover `LL_ENC_REQ`, `LL_ENC_RSP`,
`LL_START_ENC_REQ`, the central encrypted `LL_START_ENC_RSP`, and the
peripheral encrypted `LL_START_ENC_RSP`; the encrypted state then admits an
optional pause/refresh cycle.

### Directional activation and pause boundaries

The Core's three-way start handshake is asymmetric. The peripheral
`LL_START_ENC_REQ` is plaintext. Observing it installs both decryptors at
counter zero but enables only central-to-peripheral authentication. The
central `LL_START_ENC_RSP` must authenticate at central counter zero before
peripheral-to-central authentication is enabled. The final peripheral
`LL_START_ENC_RSP` must authenticate at peripheral counter zero before the
session enters the fully encrypted state.

Pause is asymmetric in the other direction. The central
`LL_PAUSE_ENC_REQ` and peripheral `LL_PAUSE_ENC_RSP` remain encrypted. After
the first response authenticates, future central-to-peripheral packets are
plaintext while the old peripheral decryptor is retained only long enough to
recognize an encrypted retransmission. The central's second
`LL_PAUSE_ENC_RSP` is plaintext; accepting it discards both old decryptors and
starts a fresh material exchange. The next `LL_START_ENC_REQ` installs two new
counter-zero decryptors from the refreshed SKD and IV.

An encrypted extended rejection of `LL_PAUSE_ENC_REQ` returns to the encrypted
state and preserves the active material and decryptors. A rejected initial
material exchange returns to the initial request state. A rejection after
encryption has already been paused returns to the refresh-request state rather
than claiming that the old encryption can resume.

### Transactional observations and retransmissions

Encrypted observations run against a cloned candidate decryptor. The candidate
counter and procedure transition are committed only after MIC verification,
strict control decoding, direction validation, and state validation all
succeed. A malformed packet, failed MIC, wrong direction, or out-of-order PDU
therefore changes neither procedure state nor packet counters. The caller
retains the borrowed raw packet on every error.

Encrypted retransmissions use `LeAclDecryptor`'s authenticated SN/counter
reuse. Plaintext procedure retransmissions are idempotent only when direction,
the stable LLID/CP/RFU header bits, SN, and complete control payload match the
immediately accepted plaintext control PDU; changing SN on the same repeated
payload is rejected. NESN and MD may change without hiding an LLID change as a
retransmission. Empty PDUs and a valid `LL_TERMINATE_IND` remain admissible
during the restricted handshake. `reset` drops procedure, material, and
counter state after a capture discontinuity while preserving the configured
LTK and skip bound.

### Offline integration boundary

`encryption-trace` accepts repeated
`DIRECTION:HEADERPAYLOADHEX` records. It prints the raw record before invoking
the tracker, reports plaintext/authenticated output and state transitions, and
continues after per-record errors. This is an offline protocol integration
surface, not a direction classifier or channel follower. It does not consume
I/Q, select an LTK from pairing state, merge captures, or route live hopping
observations. `decode-data` remains the explicit single-direction path for a
fixed-channel waveform with caller-supplied initial state.

### Independent behavior boundary

Bluetooth Core 6.1 Vol 6, Part B, Sections 5.1.3.1 and 5.1.3.2 define which
start and pause PDUs are plaintext or encrypted. Zephyr controller commit
`7d46db352251f85a6bc7b5961fb8a86e2f3125e4` independently implements separate
`enc_rx`/`enc_tx` transitions and resets both CCM counters in
`ull_llcp_enc.c`; its `ctrl_encrypt` tests exercise the same directional
sequence. Scapy commit `de3399269bad8c9a6bfb1dc181c3876340c198b8`
independently serializes the six encryption control opcodes. Pause and refresh
ciphertexts were generated outside Blueoxide with `cryptography 46.0.7`
backed by OpenSSL 3.5.6 after reproducing both published Core counter-zero
start-response vectors.
