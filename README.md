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
- Fixed-channel secondary advertising decode on channels 0 through 36 with
  asserted LE 1M/2M PHY, full eight-bit Length framing, semantic extended
  headers, and PHY-correct PCAPNG output.
- Configurable CRC-gated LE 1M and LE 2M data-channel decoding for a known
  connection access address, CRC initializer, logical channel, and asserted
  PHY.
- Dependency-free AES-128/CCM authentication and decryption for explicitly
  directed LE ACL streams with caller-supplied session state, MIC-gated counter
  advancement, retransmission handling, and bounded counter resynchronization.
- Uncoded LE 1M/2M quadrature demodulation with integer timing-phase search,
  robust slicing, spectrum-inversion handling, and configurable
  access-address tolerance.
- Bounded streaming input for interleaved little-endian `f32` and signed 16-bit
  I/Q files, including packet recovery across block boundaries.
- Exact access-address sample indices, carrier-offset estimates, modulation
  deviation estimates, and discontinuity reset/reporting.
- Typed decoding for legacy advertising, scan, direct, and connection-request
  PDUs, including AD structures and validated CONNECT_IND timing/channel data.
- Strict primary-channel `ADV_EXT_IND` decoding with typed advertising mode,
  AdvA, TargetA, CTEInfo, ADI, AuxPtr, SyncInfo, and TxPower fields; residual
  ACAD and advertising data remain lossless raw bytes.
- Data-channel header, CTEInfo, and L2CAP-start decoding plus strict, lossless
  LL control-PDU syntax through Core 6.1 Channel Sounding and Frame Space
  Update, with future opcode payloads retained raw.
- Bounded, direction-explicit plaintext L2CAP PDU reassembly with independent
  central/peripheral state, exact retransmission suppression, malformed-length
  rejection, and discontinuity reset.
- Strict, lossless ATT PDU decoding on fixed CID `0x0004`, covering every
  assigned Core 6.1 opcode, fixed and variable record validation, raw unknown
  opcode preservation, and non-suppressing semantic error reporting.
- Strict, lossless LE Security Manager Protocol decoding on fixed CID `0x0006`,
  covering all assigned pairing and key-distribution commands, Core field
  validation, future-command preservation, and non-suppressing errors.
- Validated data-channel maps plus Channel Selection Algorithms #1 and #2,
  including CONNECT_IND ChSel selection and event-counter channel calculation.
- Anchored connection-event tracking with wrap-safe instant handling, strict
  LL_CHANNEL_MAP_IND/LL_CONNECTION_UPDATE_IND/LL_PHY_UPDATE_IND parsing,
  directional PHY state, and explicit anchor reacquisition after
  connection-parameter changes.
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
- Fixed-channel live LE 1M/2M data capture for a caller-supplied connection
  access address and CRC initializer, with exact hardware sample positions and
  BLE PCAPNG output.

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

Only packets with a valid BLE CRC are emitted. Primary advertising decode is
LE 1M and requires an integer oversampling ratio from 2 through 64 samples per
symbol. `--capture-start-ns` can supply the Unix timestamp of sample zero for
PCAPNG; without it, timestamps are relative to the Unix epoch.

CRC-valid primary `ADV_EXT_IND` packets receive strict bounded decoding of the
common extended header. The decoder reports typed advertiser and target
addresses, CTEInfo, ADI set/data identifiers, AuxPtr channel/timing/PHY,
periodic SyncInfo, signed transmit power, and exact residual ACAD and
advertising-data lengths. An AuxPtr is descriptive only: `decode` does not
retune to or receive the secondary advertising channel, reassemble chained
advertising data, or establish periodic synchronization state. LE Coded may be
represented by an AuxPtr but is not demodulated.

The decoder processes the file in bounded blocks and retains enough overlap to
recover maximum-length primary advertisements split between reads. Repeated
identical advertisements are preserved when they occur at different sample
positions.

Decode a recording already centered on one secondary advertising channel:

```text
cargo run --release -- decode-secondary \
  --input auxiliary.cf32 \
  --format f32le \
  --channel 20 \
  --phy 2m \
  --sample-rate 8000000 \
  --block-samples 262144 \
  --output-pcap auxiliary.pcapng
```

`decode-secondary` accepts channels 0 through 36 and an asserted
`--phy 1m|2m`. It uses the advertising access address and CRC initializer but
the full eight-bit secondary advertising Length field, allowing payloads up to
255 octets. PDU type `0x07` receives the same bounded extended-header semantic
decode as primary `ADV_EXT_IND`; other secondary advertising PDU types remain
lossless raw payloads instead of being assigned legacy primary meanings.

The command decodes one already selected channel and PHY. It does not infer
whether a type-`0x07` packet is AUX_ADV_IND, AUX_CHAIN_IND, AUX_SYNC_IND, or
AUX_SCAN_RSP because that name depends on scheduling context. It does not
follow AuxPtr, retune, combine channels, reassemble chains, maintain periodic
synchronization state, or demodulate LE Coded.

Decode a recording from a known LE connection data channel:

```text
cargo run --release -- decode-data \
  --input connection.cf32 \
  --format f32le \
  --channel 12 \
  --phy 1m \
  --sample-rate 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --block-samples 262144 \
  --output-pcap connection.pcapng
```

`decode-data` accepts data channels 0 through 36 and `--phy 1m|2m`, defaulting
to LE 1M. LE 2M uses a 2 Msymbol/s rate, 500 kHz nominal deviation, and a
two-octet alternating preamble; for example, an 8 Msps recording uses
`--phy 2m --sample-rate 8000000`. Both uncoded PHYs require an integer
oversampling ratio from 2 through 64 samples per symbol. The selected PHY is an
explicit caller assertion: `decode-data` does not infer it from samples or
switch demodulators within one recording. The separate connection tracker can
apply a typed `LL_PHY_UPDATE_IND` to offline directional state at its instant,
but capture-driven decoder switching is not yet connected.

The connection access address and 24-bit CRC initializer normally come from a
decoded CONNECT_IND. Data PDUs are emitted only after CRC validation. When the
CP bit is set, the separate CTEInfo octet is retained and decoded without
including it in the Length-counted payload. The payload field remains lossless
and can include an encrypted MIC when decryption is not configured. Printed
L2CAP and LL control interpretations are explicitly plaintext hints in that
mode; encrypted payloads remain available as raw bytes and are not guessed
from packet shape. PCAPNG output records the asserted LE 1M or LE 2M PHY in the
Bluetooth LE pseudo-header.

LLID `0b11` packets receive strict typed LL control decoding without requiring
L2CAP reassembly. Blueoxide validates exact parameter sizes for every assigned
Core 6.1 opcode from `LL_CONNECTION_UPDATE_IND` (`0x00`) through
`LL_FRAME_SPACE_RSP` (`0x3c`). Coverage includes encryption setup,
feature/version exchange, connection parameters, data length and PHY updates,
CTE requests, periodic synchronization, CIS establishment, power control,
connection subrating, channel classification, PAwR synchronization transfer,
24-octet feature pages, Channel Sounding security/capability/configuration/start
PDUs, FAE tables and CS channel maps, termination, and Frame Space Update.

The parser checks Core field ranges and cross-field relationships that are
provable from one PDU: reserved bits, PHY masks, offset ordering, data-length
limits, SyncInfo structure, Core 6.1 CIS Framing_Mode packing, subrate
relationships, Channel Sounding capability masks, antenna and role ranges,
excluded/minimum CS channels, mode combinations, algorithm #3c jump/repetition
limits, CS timing and SNR indices, and Frame Space masks/ranges. Future opcodes
retain every parameter octet as raw data.

A malformed known PDU remains visible in the complete packet line and
increments `ll_control_errors`. Encryption and Channel Sounding security output
can contain Rand, EDIV, session-key diversifiers, initialization vectors,
nonces, and personalization vectors and must be handled as sensitive capture
data. The parser does not enforce procedure order, role legality relative to
connection history, capability negotiation, instant timing relative to an
observed event, application of CS or Frame Space changes, or encryption state.
The separate encryption-material tracker does enforce central
`LL_ENC_REQ`/peripheral `LL_ENC_RSP` direction and order when a caller supplies
the matching LTK.

For a recording with known link-encryption state, authenticate and decrypt one
asserted transmitter direction:

```text
cargo run --release -- decode-data \
  --input central-encrypted.cf32 \
  --format f32le \
  --channel 12 \
  --sample-rate 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --session-key 99ad1b5226a37e3e058e3b8e27c2c666 \
  --iv 24abdcbabebaafde \
  --decrypt-direction central-to-peripheral \
  --packet-counter 0 \
  --max-counter-skip 32 \
  --plaintext-l2cap-direction central-to-peripheral
```

`--session-key` is the already-derived 16-octet AES key in left-to-right AES
input order. `--iv` is the eight nonce octets in Link Layer order: the four
central IV octets followed by the four peripheral IV octets.

Alternatively, derive both values from a caller-selected LTK and the complete
captured LL control payloads, including their opcode octets:

```text
cargo run --release -- decode-data \
  --input central-encrypted.cf32 \
  --format f32le \
  --channel 12 \
  --sample-rate 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --ltk bf01fb9d4ef3bc36d874f5394138684c \
  --enc-req 039078563412efcdab74241302f1e0dfcebdac24abdcba \
  --enc-rsp 047968574635241302bebaafde \
  --decrypt-direction central-to-peripheral \
  --packet-counter 0 \
  --max-counter-skip 32
```

`--ltk` uses the least-significant-octet-first order carried by HCI and SMP key
fields. `--enc-req` requires all 23 raw captured octets from opcode `0x03`
through IVm; `--enc-rsp` requires all 13 raw captured octets from opcode
`0x04` through IVs. The tracker validates the strict PDU layouts and
transmitter roles, converts the key and SKD fields into AES input order, and
concatenates the raw IVm and IVs fields into nonce order. These options are
mutually exclusive with `--session-key` and `--iv`.

Direction and the initial 39-bit packet counter are also caller assertions.
The decryptor advances its per-direction counter only after a valid four-octet
MIC. Exact retransmissions reuse the last authenticated counter, zero-length
data PDUs consume no counter, and `--max-counter-skip` permits a bounded forward
MIC search after missing encrypted packets. A MIC failure or confirmed skipped
counter resets any incomplete L2CAP reassembly before later plaintext is used.

Every CRC-valid ciphertext packet is still printed and written to PCAPNG
unchanged. Authenticated bytes appear on a separate `decrypted_data` line and
only those bytes enter LL control, L2CAP, ATT, signaling, or SMP decoding. The
PCAPNG file remains an over-the-air ciphertext capture, not a synthesized
plaintext trace. LTKs, session keys, and captured encryption material supplied
on a command line may be visible in shell history and process inspection and
must be handled as sensitive data.

Material reconstruction does not select an LTK from Rand/EDIV or pairing
history, infer packet direction or the initial counter, model
`LL_START_ENC_*`/`LL_PAUSE_ENC_*` activation, or create independent
bidirectional decryptors automatically. A complete capture-driven encryption
procedure still requires those states.

For a recording that is already known to contain a complete, ordered plaintext
stream from one link direction, opt into L2CAP PDU reassembly:

```text
cargo run --release -- decode-data \
  --input central-plaintext.cf32 \
  --format f32le \
  --channel 12 \
  --sample-rate 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --plaintext-l2cap-direction central-to-peripheral \
  --max-l2cap-payload 65535
```

This option is an assertion by the caller, not direction detection. Without
decryption options it asserts that the captured bytes are already plaintext;
with decryption options it selects the matching authenticated plaintext stream.
Each emitted `l2cap_pdu` contains the direction, CID, declared payload length,
fragment count, and complete payload bytes. Central and peripheral streams
require separate direction, counter, and reassembly state. Exact consecutive
fragment retransmissions are suppressed; malformed lengths, orphaned
continuations, replacement starts, and incomplete end-of-input state are
reported without discarding the original CRC-valid packet output.

Completed plaintext PDUs on LE signaling CID `0x0005` are additionally decoded
as one strict signaling command. The four-octet command header requires a
nonzero identifier and a Length that exactly matches all remaining bytes.
Known disconnection, connection-parameter, LE credit-based, flow-control
credit, and Enhanced Credit Based commands receive typed `l2cap_signal`
output. Unknown command codes retain their raw parameters. A malformed known
command reports a signaling error after the complete raw `l2cap_pdu` line, so
semantic decoding never suppresses reconstructed bytes.

Completed plaintext PDUs on the fixed ATT channel CID `0x0004` receive a
separate `att_pdu` description. The decoder covers requests, responses,
commands, notifications, indications, and confirmations through the Core 6.1
opcode set, including 16-bit and 128-bit UUID forms, fixed-record discovery
responses, queued writes, signed writes, variable-length multiple reads, and
multiple-handle notifications. It validates exact fixed sizes, nonzero handles,
ordered ranges, record divisibility, complete tuple headers, and the permitted
final-value truncation in a Read Multiple Variable Length Response. Unknown
opcodes retain all parameter bytes. A malformed known PDU increments
`att_errors` only after the complete raw `l2cap_pdu` has been printed.

This layer is stateless ATT syntax. It does not infer attribute types, rebuild a
GATT database, track negotiated MTUs, verify signed-write authentication, or
identify Enhanced ATT on dynamically allocated L2CAP channels. EATT decoding
requires credit-based channel state and its assigned PSM, so dynamic CIDs are
not guessed from payload bytes.

Completed plaintext PDUs on the LE Security Manager fixed channel CID `0x0006`
receive a separate `smp_pdu` description. The decoder covers Pairing Request
and Response, Confirm, Random, Failed, all legacy key-distribution messages,
Security Request, Secure Connections public keys and DHKey checks, and keypress
notifications. It validates exact command lengths, IO and OOB values, bonding
and reserved AuthReq bits, 7-through-16-octet encryption key sizes, key
distribution masks, Pairing Failed reasons through Core 6.1 `Busy`, identity
address type/static-random structure, and keypress types. Reserved future
command codes retain every parameter byte.

SMP output is passive syntax, not a pairing engine. It does not enforce command
sequence or role, correlate Pairing Request and Response masks, derive STK/LTK,
session keys, or MacKey, verify confirms or DHKey checks, or automatically
configure link decryption. Raw and typed SMP output can contain LTK, IRK, CSRK,
random, and public-key material and must be handled as sensitive capture data.
Malformed known commands increment `smp_errors` only after the complete raw
`l2cap_pdu` has been printed.

An ordinary recording centered on one data channel is generally incomplete for
a hopping connection because packets transmitted on other channels are absent.
Do not use its reassembly output as authoritative unless the input is known to
cover every packet in the asserted direction. The reassembler resets on sample
discontinuities that are visible in the input, but it cannot detect packets
that were never delivered to the decoder.

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
  --phy-update 2m:unchanged:6 \
  --events 10
```

The channel map is five hexadecimal octets in over-the-air order. Connection
intervals use 1.25 ms units, supervision timeouts use 10 ms units, and the
expected sample index is calculated relative to the observed access-address
sample without accumulating per-event rounding error. `--hop` selects the
5-through-16 hop increment for CSA#1. `--peer-sca` accepts the CONNECT_IND SCA
field from 0 through 7, while `--receiver-ppm` supplies the receiver sample
clock's worst-case error. Plans include the resulting earliest/latest sample
bounds and the active central-to-peripheral and peripheral-to-central PHYs.
`--c2p-phy` and `--p2c-phy` assert the directional state at an arbitrary
anchor. `--phy-update C2P:P2C:INSTANT` schedules a single update using
`1m`, `2m`, `coded`, or `unchanged` for each direction. A CONNECT_IND
acquisition always begins with LE 1M in both directions.

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
`LL_CONNECTION_UPDATE_IND` control PDUs, plus directional
`LL_PHY_UPDATE_IND`. A channel-map update is applied before choosing the
channel at its instant. A PHY update is applied before returning that event and
preserves any direction encoded as unchanged. A connection-parameter update
deliberately returns an anchor-observation-required state at its instant;
ordinary missed-event searches stop there, and scheduling resumes only after
the caller supplies the access-address sample actually observed in that event.
The conservative tracker permits only one pending instant-based update.

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

Capture one fixed connection data channel with any supported backend:

```text
cargo run --release -- capture-data \
  --device bladerf \
  --channel 12 \
  --phy 2m \
  --sample-rate 8000000 \
  --bandwidth 4000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --seconds 30 \
  --output-pcap connection-channel-12.pcapng
```

`capture-data` uses the same finite, timestamped SDR lifecycle as advertising
capture, but instantiates the generalized data-channel decoder. It accepts LE
1M or LE 2M, retains CP/CTEInfo at the correct frame boundary, prints the same
lossless packet and plaintext-hint fields as offline `decode-data`, and records
the asserted PHY in PCAPNG. Access address, CRC initialization, channel, and
PHY are caller-supplied connection state.

When an external observer can assert that every decoded packet is a central
transmission, the same fixed-channel capture can associate packet timestamps
with connection events:

```text
cargo run --release -- capture-data \
  --device bladerf \
  --channel 31 \
  --sample-rate 4000000 \
  --bandwidth 2000000 \
  --access-address 0x12345678 \
  --crc-init 0xabcdef \
  --assert-central-observations \
  --first-event 0 \
  --channel-map ffffffff1f \
  --csa 2 \
  --interval 24 \
  --seconds 30
```

`--assert-central-observations` is a caller assertion, not packet-direction
detection. A peripheral response must not be supplied as an event anchor. The
configured first event must select the tuned channel; later candidates are
searched from the current event through at most `--max-event-advance` event
advances using the asserted channel map, CSA state, connection interval, peer
sleep-clock accuracy, and receiver clock-error bound. Matches produce
`central_connection_event` records with event, timing-window, and missed-event
details. Rejected candidates are reported but do not mutate event state,
suppress the raw packet, stop capture, or prevent PCAPNG output.

This command remains tuned to one channel for the full capture. It does not
follow the connection hop sequence, infer packet direction, decrypt traffic,
reassemble L2CAP, automatically apply decoded LL control procedures, or apply
tracked channel/PHY changes to the radio. Its output is authoritative for
packets received on that tuned channel, not for the complete connection when
transmissions occur elsewhere.

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
synchronization, instant-based map/parameter updates, and direction-explicit
ACL decryption, captured LL encryption-material derivation, and plaintext L2CAP
PDU reassembly are now present. Fixed-channel live data observations are also
available; the next receive stages are wideband channelization or timed
retuning, routing those observations into connection-event state, applying
tracked PHY transitions to demodulator selection, and full live BLE connection
following. Full packet decode is a project requirement: AuxPtr-driven
secondary-channel following, extended-advertising chain reassembly, periodic
advertising synchronization, complete LL procedure state, automatic pairing
and LTK selection, bidirectional encryption-state tracking, L2CAP channel
state, stateful GATT reconstruction, LE Coded PHY demodulation, and Bluetooth
Classic BR/EDR layers will be added incrementally while retaining undecoded
packet bytes losslessly.

Active signal injection and transmit support are intentionally deferred until
receive, timestamping, channelization, and packet validation are reliable;
transmit will be introduced as a separate subsystem rather than folded into the
receive API.

See `DesignLog.md` for architectural decisions and `ChangeLog.md` for completed
increments. `Verification.md` records independent cross-implementation checks.
