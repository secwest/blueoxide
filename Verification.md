# Blueoxide Verification Record

This file records checks that go beyond Blueoxide's own implementation. External
software is used only during development verification and is not a runtime
dependency.

## Environment

- Date: 2026-07-13 through 2026-07-22
- Rust: 1.95.0
- Python: 3.14.4
- NumPy: 2.4.6
- Scapy: 2.7.0

No libbladeRF, LimeSuite, libxtrx installation, or attached SDR was present in
the development environment. Native-backend verification in this increment is
therefore source/ABI review plus mock-native execution, not an over-the-air
hardware claim.

## Independent BLE PHY vectors

Reference:

- Project: Jiao Xianjun's BTLE
- Commit: `85401861e8f4b04b90cbaa0394c0f9d45ed02f18`
- Reference functions: `crc24_core` and `scramble_core`

CRC initialization was `0x555555`. Bytes were supplied least-significant bit
first as transmitted by BLE.

| PDU bytes | Expected transmitted CRC |
| --- | --- |
| `0000` | `1db538` |
| `0006010203040506` | `42f5f2` |
| `05002233445566778899aabbccddeeff` | `0cc832` |

Whitening input was `00060102030405061db538`.

| Channel | Expected whitened bytes |
| --- | --- |
| 0 | `40b4bdc11c334f599843a4` |
| 37 | `8dd456a33ea363b6688429` |
| 38 | `d6c345225adae489061097` |
| 39 | `1f314b5d86f2999cdc63fd` |

These values are fixed in `src/ble.rs` tests.

## Independent ADV_IND waveform and PCAPNG check

The BTLE reference generated CRC and whitening for:

```text
PDU: 000f0102030405060201060509424c5545
CRC: d70153
Channel: 37
```

An independent Python modulator generated 4 Msps complex `f32` I/Q with
250 kHz deviation and 30 kHz carrier offset. Blueoxide decoded it with
113-sample file blocks, forcing the packet across multiple reads.

Blueoxide result:

```text
sample=43
carrier_offset_hz=30000.0
deviation_hz=250000.0
ADV_IND advertiser=06:05:04:03:02:01
payload=0102030405060201060509424c5545
crc=d70153
```

Scapy 2.7.0 independently parsed the emitted PCAPNG as:

```text
BTLE_RF / BTLE / BTLE_ADV / BTLE_ADV_IND
```

It returned the exact dewhitened bytes:

```text
d6be898e000f0102030405060201060509424c5545d70153
```

## Independent CONNECT_IND check

Reference-generated PDU:

```text
052201020304050606050403020178563412123456020300180001006400ffffffff1f0a
```

Reference CRC:

```text
0d8bf8
```

The independent waveform used 4 Msps, 250 kHz deviation, and -45 kHz carrier
offset. Blueoxide recovered those RF values and decoded:

```text
CONNECT_IND
initiator=06:05:04:03:02:01
advertiser=01:02:03:04:05:06
access_address=0x12345678
interval=24 (30000 us)
latency=1
supervision_timeout=1000000 us
hop_increment=10
enabled_channels=37
```

Scapy independently parsed the file as:

```text
BTLE_RF / BTLE / BTLE_ADV / BTLE_CONNECT_REQ
interval=24
hop=10
```

Scapy exposes access-address bytes `78 56 34 12` as integer `0x78563412`;
Blueoxide reports the conventional little-endian numeric value `0x12345678`.
The underlying packet bytes agree.

## LE data-channel framing and channel-selection verification

Specification reference:

- Bluetooth Core Specification 6.0, Vol 6, Part B:
  - Data Physical Channel PDU and CTEInfo layout.
  - Data-channel PDU header and Length semantics.
  - Channel Selection Algorithms #1 and #2.

Primary implementation reference:

- Project: Zephyr Bluetooth Controller
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/controller/ll_sw/pdu.h`
  - `subsys/bluetooth/controller/ll_sw/pdu_df.h`
  - `subsys/bluetooth/controller/ll_sw/lll_chan.c`

Zephyr independently places CP at data-header bit 5, stores CTEInfo as one
octet before link-layer data, and defines its fields as five CTE-time bits, one
RFU bit, and two CTE-type bits. Its CSA#2 implementation derives the channel
identifier by XORing the access-address halves, performs three permute and
multiply-add rounds, uses `prn_e % 37`, and remaps with
`floor(used_count * prn_e / 65536)`.

Independent packet/CRC reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth4LE.py`
- Functions/types: `BTLE.compute_crc`, `BTLE_DATA`, `BTLE_CTRL`,
  `BTLE_CONNECT_REQ`

Scapy's data-header model at this revision predates explicit CP/CTEInfo
representation, so it was used as a raw-byte CRC oracle rather than as the
CTEInfo parser. Fixed results generated independently by `BTLE.compute_crc`
are:

| Header + CTEInfo + payload | CRC init | Transmitted CRC |
| --- | --- | --- |
| `220385112233` | `abcdef` | `27e2cf` |
| `210042` | `123456` | `7fd46c` |
| `3e0985050004000c01000200` | `abcdef` | `d482c9` |

These vectors prove that Blueoxide includes CTEInfo in CRC coverage while
excluding it from the Length-counted payload. The final vector also passes
through `decode-data` in 73-sample blocks and is checked byte-for-byte in the
emitted PCAPNG.

Connection-recovery reference:

- Project: virtualabs/btlejack
- Commit: `c487859888450f6a33f618180bac5358f104e367`
- Files: `btlejack/packets.py`, `btlejack/supervisors.py`

btlejack independently extracts access address, CRC initialization, interval,
channel map, and hop increment from connection requests and carries explicit
CSA#2 PRNG/event-counter recovery state. It was used to cross-check that these
parameters remain distinct inputs rather than being collapsed into packet
decoder state.

Zephyr's Core-derived CSA#2 vectors are fixed in `src/link_layer.rs`:

| Channel map | Event counters | Expected channels |
| --- | --- | --- |
| all 37 channels | 0, 1, 2, 3 | 25, 20, 6, 21 |
| `0006e0001e` | 6, 7, 8 | 23, 9, 34 |
| `0600000000` | 11, 12, 13 | 1, 2, 1 |

Additional tests cover CSA#1 progression/remapping, invalid channel maps and
hop increments, 16-bit event-counter limits, maximum 255-octet data PDUs split
across stream blocks, CP with zero payload, inverted spectrum, malformed CRC,
and reserved CTEInfo values.

## Connection-event and instant verification

Primary controller reference:

- Project: Zephyr
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files: `subsys/bluetooth/controller/ll_sw/pdu.h`,
  `ull_llcp_pdu.c`, `ull_llcp_phy.c`, and `ull_llcp_internal.h`

The fixed LL control layouts match Zephyr's packed structures and
little-endian encode/decode paths:

| PDU | Opcode | Parameter octets | Layout |
| --- | ---: | ---: | --- |
| LL_CONNECTION_UPDATE_IND | `00` | 11 | WinSize, WinOffset, Interval, Latency, Timeout, Instant |
| LL_CHANNEL_MAP_IND | `01` | 7 | ChM[5], Instant |
| LL_PHY_UPDATE_IND | `18` | 4 | C-to-P PHY, P-to-C PHY, Instant |

Zephyr defines `PHY_1M`, `PHY_2M`, and `PHY_CODED` as one-hot values
`0x01`, `0x02`, and `0x04`. Its update validation accepts zero for unchanged,
accepts at most one of those bits per direction, treats an all-zero update as
no change, and applies nonzero transmit/receive values according to role when
the instant is reached. Blueoxide uses the same directional representation and
keeps LE Coded as valid protocol state even though its demodulator is not yet
implemented.

Instant ordering follows the Core-derived modulo tests in Zephyr:

```text
future: ((instant - event_count) & 0xffff) < 0x7fff
reached or passed: ((event_count - instant) & 0xffff) <= 0x7fff
```

Blueoxide tests the required wrap cases `65532 -> 2` as future by six events,
`2 -> 65532` as passed by six events, equality as reached, and differences
`0x7fff` and `0x8000` as ambiguous. It accepts a valid instant only one event
ahead, reflecting passive observation of a retransmission rather than imposing
Zephyr's six-event procedure-initiation delta.

Tracker tests additionally verify:

- Channel-map installation before channel selection at the instant.
- Connection-parameter activation with mandatory anchor reacquisition.
- Directional PHY installation before returning the instant event.
- LE 1M/2M/Coded and unchanged field decoding, including invalid `0x03`.
- No-change PHY updates leaving the pending-instant slot available.
- Refusal to advance while the new anchor is unknown.
- Rejection of reached, passed, ambiguous, malformed, and overlapping updates.
- 16-bit event-counter wrap with a monotonic internal event index.
- Anchor-relative nearest-sample calculation without cumulative rounding drift.
- Offline CLI output for Core CSA#2 channels, BLE frequencies, sample timing,
  directional PHY state, malformed maps, and counter wrap.

## Anchor acquisition and clock-window verification

Additional Zephyr references at commit
`7d46db352251f85a6bc7b5961fb8a86e2f3125e4`:

- `subsys/bluetooth/controller/ll_sw/nordic/lll/lll_clock.c`
- `subsys/bluetooth/controller/ll_sw/ull_peripheral.c`
- `subsys/bluetooth/controller/ll_sw/ull_conn.c`

Zephyr's SCA lookup table is fixed as:

```text
{500, 250, 150, 100, 75, 50, 30, 20} ppm
```

Its peripheral connection setup calculates per-event widening with the sum of
local and peer clock accuracy, rounded upward, and caps accumulated widening at
half the connection interval minus the 150 us inter-frame spacing. Blueoxide
uses the equivalent elapsed-time calculation directly in sample units, avoiding
extra over-widening from rounding each event independently.

Fixed 4 MHz vectors cover:

| Case | Expected result |
| --- | --- |
| 30 ms after anchor, peer 500 ppm, receiver 20 ppm | 63 samples one-sided widening |
| 60 ms after anchor, same clocks | 125 samples one-sided widening |
| 7.5 ms interval with extreme receiver bound | capped at 14,400 samples |
| CONNECT_IND AA sample 1,000, WinOffset 3, WinSize 2 | nominal event-0 window 22,376 through 32,376 |
| Same event-0 window with 520 ppm combined error | widened bounds 22,360 through 32,392 |

The acquisition test accepts a caller-identified event-0 central transmission
on CSA#1 channel 10 at sample 30,000 and rejects the wrong channel,
out-of-window samples, advertising channels, invalid WinSize, invalid SCA,
non-integral LE 1M sample rates, and receiver bounds above one million ppm.
The CLI separately requires `--central-observe`; a generic `--observe` cannot
silently establish event 0 because Blueoxide does not infer packet direction.

The observation-recovery vector starts at event 0/sample 1,000, observes CSA#2
event 3/channel 21 at sample 361,050, reports two missed intervening events and
a 50-sample late error, then re-anchors. Event 4 is consequently expected at
sample 481,050 with only one interval of widening. Searches that fail channel
or time checks, or reach an unknown connection-update anchor, leave the tracker
unchanged.

## L2CAP PDU reassembly verification

Primary host/controller reference:

- Project: Zephyr
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/controller/hci/hci.c`
  - `subsys/bluetooth/host/conn.c`

Zephyr maps controller LLID start/continue values to HCI ACL start/continuation
flags. Its LE host reassembler drops old pending state when a new start arrives,
ignores empty continuations, waits for the little-endian L2CAP Length plus the
four-octet header, and rejects LE input longer than that exact total. Blueoxide
uses the same framing, replacement, and exact-length rules while reporting the
discarded passive-capture PDU explicitly.

Independent parser reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth4LE.py`

Scapy binds LLID 2 to `L2CAP_Hdr`, leaves LLID 1 data as continuation bytes, and
binds zero-length LLID 1 to its empty-PDU type. Its parser independently reports
the fixed fragments as:

```text
0206050004000c01 -> LLID=2 len=6 L2CAP length=5 cid=4 payload=0c01
0903000200       -> LLID=1 len=3 continuation=000200
0100             -> LLID=1 len=0 empty
```

The resulting Blueoxide PDU is CID `0x0004`, payload `0c01000200`, and two
link-layer fragments. Tests additionally cover complete one-fragment PDUs,
independent bidirectional state, exact duplicate suppression, replacement
starts, orphaned continuations, explicit gap reset, malformed public PDU
invariants, configured length limits, start overflow, and continuation
overflow.

Final local gate for this increment:

```text
104 library tests
4 connection planning/acquisition/synchronization CLI integration tests
3 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## LE L2CAP signaling verification

Primary implementation reference:

- Project: Zephyr
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/host/l2cap_internal.h`
  - `subsys/bluetooth/host/l2cap.c`
  - `include/zephyr/bluetooth/l2cap.h`

Zephyr's LE signaling receive path pulls one four-octet command header, requires
the command Length to equal the entire remaining L2CAP payload, and rejects
identifier zero before dispatch. Its packed command structures fix the field
order used by Blueoxide. Zephyr limits Enhanced Credit Based lists to five
channels and explicitly uses destination CID `0x0000` in responses when a
requested channel was not established.

Independent byte-layout reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth.py`

Using that source tree directly through `PYTHONPATH`, Scapy constructed each
command with `L2CAP_CmdHdr`, serialized it, reparsed it, and reproduced the
same bytes:

```text
12070800180028000000c800
0602040041004000
14030a0080004000000180000a00
17040e008000000180000a00400041004200
18050e00000180000a000900410000004300
190608002c01960041004200
```

These are respectively Connection Parameter Update Request, Disconnection
Request, LE Credit Based Connection Request, three-channel Enhanced Credit
Based Connection Request, a three-entry Enhanced Credit Based Connection
Response containing one refused-channel zero DCID, and a two-channel Enhanced
Credit Based Reconfigure Request.

Blueoxide tests cover non-signaling CIDs, truncated headers, zero identifiers,
both command-Length mismatch directions, unknown-code preservation, every
implemented fixed layout, invalid fixed sizes, one/five/six-entry Enhanced
Credit lists, odd lists, missing lists, invalid request IDs, valid zero response
DCIDs, Core connection-parameter ranges, and bounded arbitrary payloads. The
CLI fixture additionally proves that a malformed known command still emits its
complete raw `l2cap_pdu`.

Final local gate for this increment:

```text
112 library tests
4 connection planning/acquisition/synchronization CLI integration tests
4 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## ATT PDU verification

Normative reference:

- Bluetooth Core Specification 6.1, Vol 3, Part F, Sections 3.2.8 and
  3.4.1 through 3.4.8.

The Core tables establish fixed CID `0x0004` ATT framing, all currently assigned
opcodes, exact fixed fields, 2- or 16-octet UUID requests, and fixed-record
response layouts. The MTU exchange fields are unsigned 16-bit receive sizes
whose only value constraint is a minimum of the default ATT MTU, 23. The Core
also distinguishes the two variable tuple rules: Read Multiple Variable Length
Response may truncate only its final value, while Multiple Handle Value
Notification requires two or more complete tuples.

Primary implementation reference:

- Project: Zephyr
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/host/att_internal.h`
  - `subsys/bluetooth/host/att.c`
  - `include/zephyr/bluetooth/att.h`

Zephyr's packed ATT structures independently confirm every field order and
assigned opcode through `ATT_MULTIPLE_HANDLE_VALUE_NTF` and
`ATT_SIGNED_WRITE_CMD`. Its receive handlers require an opcode before
dispatch, reject invalid handle ranges, accept only 16- or 128-bit UUID request
forms, require at least two handles in multiple-read requests, and use the same
12-octet signed-write suffix. Blueoxide is intentionally stricter at the
capture syntax boundary where the Core requires complete fixed records.

Independent byte-layout reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth.py`

Using that source tree directly through `PYTHONPATH`, Scapy serialized each
common ATT class, wrapped it in `L2CAP_Hdr(cid=4)`, reparsed it through the CID
binding, and reproduced these ATT bytes:

```text
024000
040100ffff
060100ffff00280f18
080100ffff0328
0c01000200
0e010002000300
120300aabb
1604000200cc
1801
1b0500aabb
52060011
```

These are Exchange MTU, Find Information, Find By Type Value, Read By Type,
Read Blob, Read Multiple, Write, Prepare Write, Execute Write, Handle Value
Notification, and Write Command PDUs. Scapy does not model all newer ATT
opcodes, so the Core and Zephyr checks cover Read Multiple Variable Length,
Multiple Handle Value Notification, confirmation, and signed write.

Blueoxide tests cover fixed-channel selection, empty and unknown PDUs, every
assigned opcode family, both UUID widths, fixed-record divisibility, zero
handles, reversed ranges, exact fixed sizes, invalid Execute Write flags,
multiple-read handle counts, complete and final-value-truncated variable-read
responses, two-or-more complete multiple-notification tuples, signed-write
suffixes, and bounded arbitrary inputs. The CLI fixture reconstructs a valid
fragmented Read Blob Request and then a malformed known Read Request, proving
that both raw L2CAP payloads remain visible while only the malformed PDU
increments `att_errors`.

Final local gate for this increment:

```text
122 library tests
4 connection planning/acquisition/synchronization CLI integration tests
4 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## LE Security Manager Protocol verification

Normative reference:

- Bluetooth Core Specification 6.1, Vol 3, Part H, Sections 3.3, 3.5, and 3.6.

The Core defines one command per LE Security Manager L2CAP PDU on fixed CID
`0x0006`, with command codes `0x01` through `0x0e`. Its field tables establish
the five IO capabilities, two OOB values, AuthReq bonding and security flags,
7-through-16-octet maximum key size, four key-distribution flags, exact
cryptographic field widths, public/static-random identity addresses, and five
keypress values. Core 6.1 Pairing Failed reasons include `0x0f` Key Rejected
and `0x10` Busy.

Primary implementation reference:

- Project: Zephyr
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/host/smp.h`
  - `subsys/bluetooth/host/smp.c`

Zephyr's packed structures and 14-entry receive-handler table independently
confirm every command length and field order. Its receiver requires an exact
handler-specific parameter length, rejects unsupported/reserved command codes,
checks encryption key size, and validates Identity Address Information as an
identity address. The pinned revision names Pairing Failed through Key
Rejected; the newer Busy reason comes from the Core 6.1 table.

Independent byte-layout reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth.py`

Using that source tree directly through `PYTHONPATH`, Scapy built every SMP
class, wrapped it in `L2CAP_Hdr(cid=6)`, reparsed through `SM_Hdr`, and
reproduced these command bytes:

```text
0103000d100706
0204013d0c0304
03000102030405060708090a0b0c0d0e0f
04101112131415161718191a1b1c1d1e1f
0510
06000102030405060708090a0b0c0d0e0f
0734120001020304050607
08000102030405060708090a0b0c0d0e0f
09010102030405c6
0a000102030405060708090a0b0c0d0e0f
0b0d
0c000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f
0d000102030405060708090a0b0c0d0e0f
0e04
```

These cover all assigned Pairing, key-distribution, Security Request, Secure
Connections, and keypress commands. Blueoxide tests additionally reject every
known command's short and long forms, reserved IO/OOB/AuthReq/key-distribution
values, invalid key sizes, reserved failure and keypress values, invalid
identity address types, malformed static-random identities, and bounded
arbitrary inputs.

The CLI fixture emits a valid Pairing Request as typed `smp_pdu` output, then
reconstructs a malformed known Security Request. Both complete raw CID
`0x0006` payloads remain visible and only the malformed command increments
`smp_errors`.

Final local gate for this increment:

```text
132 library tests
4 connection planning/acquisition/synchronization CLI integration tests
5 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## LE Link Layer control PDU verification

Normative reference:

- Bluetooth Core Specification 6.1, Vol 6, Part B:
  - Section 2.4.2 and Table 2.22 for the control envelope and opcode table.
  - Sections 2.4.2.1 through 2.4.2.41 for opcodes `0x00..=0x2c`.
  - Section 2.3.4.6 for the embedded 18-octet SyncInfo field.

The official figures establish the exact Core 6.1 layouts that are absent or
outdated in older implementation parsers:

| PDU | Parameter octets | Core 6.1 detail |
| --- | ---: | --- |
| `LL_CIS_REQ` | 35 | 12-bit Max_SDU_C_To_P, 2 RFU bits, Framing_Mode, Framed |
| `LL_PERIODIC_SYNC_IND` | 34 | 18-octet SyncInfo plus connection/advertiser metadata |
| `LL_PERIODIC_SYNC_WR_IND` | 42 | 34-octet base, 4-octet RspAA, four timing octets |
| `LL_FEATURE_EXT_REQ/RSP` | 26 | MaxPage, PageNumber, 24-octet FeaturePage |

Primary implementation reference:

- Project: Zephyr Bluetooth Controller
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- File: `subsys/bluetooth/controller/ll_sw/pdu.h`

Zephyr's packed structures independently confirm field order and exact sizes
through `LL_CIS_TERMINATE_IND` (`0x22`), including little-endian encryption,
version, connection-parameter, length, PHY, periodic-sync, and CIS fields. Its
pinned structure predates the Core 6.1 CIS Framing_Mode assignment, so the
official Core figure takes precedence for those two former RFU bits.

Independent packet reference:

- Project: Scapy
- Commit: `de3399269bad8c9a6bfb1dc181c3876340c198b8`
- File: `scapy/layers/bluetooth4LE.py`

Scapy serialized and reparsed these representative control payloads, including
the opcode:

```text
03000102030405060708090a0b0c0d0e0f101112131415
0c0d34127856
14fb0048081b004808
1a8a
2308ff7f
2403fe7eff
250f010102
260200050009000100c800
270500090009000400c800
28010596
```

These are Encryption Request, Version Indication, Length Request, CTE Request,
Power Control Request/Response, Power Change Indication, Subrate
Request/Indication, and Channel Reporting Indication. The vectors are fixed in
`src/ll_control.rs`.

The pinned Scapy revision models commands only through
`LL_CHANNEL_STATUS_IND` (`0x29`). It also uses big-endian generic short fields
for some connection-parameter/instant classes and its `LL_CIS_REQ` class
predates Framing_Mode. Blueoxide does not treat those known discrepancies as
oracles; Zephyr's little-endian packed fields and the Core 6.1 figures govern
those layouts. Opcodes `0x2a..=0x2c` are verified directly against the Core
figures.

Channel Sounding and Frame Space normative references:

- Bluetooth Core Specification 6.1, Vol 6, Part B:
  - Sections 2.4.2.42 through 2.4.2.55 for the sixteen CtrData layouts.
  - Section 4.5.18.1 for CS step/subevent limits.
  - Sections 5.1.23 through 5.1.27 for the CS procedure relationships.
  - Section 5.1.30 for Frame Space Update response constraints.
- Bluetooth Core Specification 6.1, Vol 6, Part H:
  - Sections 4.1, 4.3, 4.4, and 4.7 for channel selection, timing indices,
    mode sequencing, and antenna switching.
- Bluetooth Core Specification 6.1, Vol 6, Part A:
  - Sections 3.1.3 and 5.3 for SNR output indices and ACI.

The official figures establish these exact parameter lengths:

| Opcode group | Parameter octets |
| --- | ---: |
| `LL_CS_SEC_REQ/RSP` | 20 |
| `LL_CS_CAPABILITIES_REQ/RSP` | 25 |
| `LL_CS_CONFIG_REQ`, `LL_CS_CONFIG_RSP` | 27, 1 |
| `LL_CS_REQ`, `LL_CS_RSP`, `LL_CS_IND` | 28, 21, 18 |
| `LL_CS_TERMINATE_REQ/RSP` | 4 |
| `LL_CS_FAE_REQ`, `LL_CS_FAE_RSP` | 0, 72 |
| `LL_CS_CHANNEL_MAP_IND` | 12 |
| `LL_FRAME_SPACE_REQ/RSP` | 7, 5 |

Independent Channel Sounding references:

- Google RootCanal, commit
  `39d127f60747402c1fc07a067fcadabd1232b793`,
  `packets/link_layer_packets.pdl` and LL/CS conformance vectors.
- Texas Instruments BLE controller, commit
  `68ca021502383f367d0bf2a5517fdd0dcb0ef909`,
  `ll_cs_ctrl_pkt_internal.h`.
- Zephyr, commit `6072d4880d2d8deeadb506929adca4dba44c8220`,
  `include/zephyr/bluetooth/{conn.h,cs.h,hci_types.h}`.
- Google Bumble, commit
  `35d35c7ea43728faac83d02457ff1c30fe4528ab`,
  `bumble/ll.py`.

RootCanal independently confirms CS field ordering and bit packing, but uses
private high-value emulation opcodes and adds status fields to some internal
packets. TI confirms request/response/indication ordering but omits the Core
6.1 trailing RFU octets from response and indication structures. The official
Core figures therefore take precedence for public opcodes and exact lengths.
Zephyr independently confirms the optional T_IP1/T_IP2, T_FCS, T_PM, and
TX-SNR capability masks. Bumble confirms the public `0x2d..=0x3c` opcode table.

Tests cover every assigned opcode's short and long forms and every
`0x2d..=0x3c` valid typed layout. Focused cases exercise capability
cross-fields, antenna/role ranges, CS map exclusions and minimum channel count,
configuration create/remove behavior, legal mode pairs, timing indices,
algorithm #3c limits, request/response/indication offsets and RFU bytes, ACI,
PHY and SNR values, all 72 signed FAE entries, Frame Space masks/ranges, future
raw opcode preservation, and bounded arbitrary input through 80 parameter
octets. Fixed vectors derive from RootCanal configuration and TI packed request
layouts.

The waveform-backed CLI fixture emits a valid `LL_LENGTH_REQ`, a valid typed
`LL_FRAME_SPACE_REQ`, and a malformed known `LL_CS_CONFIG_RSP` with trailing
CtrData. All raw data-channel packets remain visible, valid commands receive
typed output, and only the malformed packet increments `ll_control_errors`.

Final local gate for this increment:

```text
151 library tests
4 connection planning/acquisition/synchronization CLI integration tests
6 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Internal audit matrix

The checked-in test suite covers:

- LE 1M at 2, 4, 8, and 16 samples per symbol.
- LE 2M at 2, 4, and 8 samples per symbol with -200 kHz, zero, and +200 kHz
  carrier offsets, normal/inverted spectra, and streaming block boundaries.
- Carrier offsets of -100 kHz, 0, and +100 kHz.
- Normal and conjugated/inverted complex spectra.
- Packets split across arbitrary stream blocks.
- Explicit sample gaps and decoder-state reset.
- Repeated byte-identical advertisements at distinct sample positions.
- Malformed and non-finite I/Q input.
- Short underlying reads.
- CRC corruption rejection.
- CTEInfo frame boundaries, CRC coverage, raw-value preservation, and maximum
  data-PDU stream retention.
- AES-128 FIPS cipher vector, Bluetooth Core LE CCM vectors in both
  directions, first encrypted control PDU, masked-header behavior, MIC failure,
  retransmission counter reuse, zero-length bypass, bounded counter search, and
  arbitrary bounded encrypted PDU inputs.
- Exact LL control layouts through Feature Page Exchange, Core 6.1 opcode
  naming, reserved-field validation, and non-suppressing malformed output.
- Truncated Advertising Data structures.
- Invalid CONNECT_IND timing and channel constraints.
- Arbitrary bounded advertising PDU payloads without panics.
- PCAPNG block lengths, link type, flags, byte order, and timestamp conversion.
- Dynamic symbol loading on the host operating system.
- bladeRF open/configure/start/read/stop/close ordering and drop cleanup.
- Q11 edge-value conversion, short native reads, receive timeouts, native
  overruns, forward timestamp gaps, backward timestamps, and timestamp overflow.
- Decoder validation and applied-sample-rate checks before hardware streaming.
- Live CLI failure handling without an installed vendor library.
- LimeSDR initialization/configuration/start/read/stop/destroy/disable/close
  ordering, reconfiguration teardown, and partial-failure cleanup.
- LimeSDR F32 I/Q validation, timeout behavior, stream-status counter handling,
  forward/backward timestamp discontinuities, and timestamp overflow.
- XTRX open/configure/run/read/stop/close ordering, channel A/B SISO selection,
  applied-rate reporting, and configuration/run failure recovery.
- XTRX Q11 conversion, finite timeout handling, overflow interval accounting,
  forward/backward timestamp discontinuities, and ABI layouts.

## libbladeRF ABI and behavior verification

Reference:

- Project: Nuand bladeRF / libbladeRF
- Commit: `41b7fc705651404e2a180c477309cb2d29f4d69b`
- Header: `host/libraries/libbladeRF/include/libbladeRF.h`
- Sync implementation:
  `host/libraries/libbladeRF/src/streaming/sync.c`
- Official metadata example:
  `host/libraries/libbladeRF/doc/examples/sync_rx_meta.c`

The reviewed source confirms:

| Blueoxide assumption | Official definition/behavior |
| --- | --- |
| RX channel mapping | `BLADERF_CHANNEL_RX(ch) = ch << 1` |
| X1 stream layout | `BLADERF_RX_X1 = 0` |
| Metadata format | `BLADERF_FORMAT_SC16_Q11_META = 2` at the pinned revision |
| Sample representation | Interleaved signed I then Q, Q11 range `[-2048, 2048)` |
| Immediate receive flag | `BLADERF_META_FLAG_RX_NOW = 1 << 31` |
| Overrun status | `BLADERF_META_STATUS_OVERRUN = 1 << 0` |
| Timeout status | `BLADERF_ERR_TIMEOUT = -6` |
| Timestamp type | Unsigned 64-bit free-running FPGA sample counter |
| Metadata ABI on 64-bit targets | Size 56 bytes, alignment 8 bytes |

The pinned `sync.c` implementation shows that a metadata read without RX_NOW is
timestamp-directed and can return `BLADERF_ERR_TIME_PAST`; with RX_NOW it writes
the timestamp of the first returned sample. It also sets `actual_count` to the
number of contiguous samples returned. Blueoxide has fixed regression tests
that require RX_NOW on every native read and reject an `actual_count` larger
than the supplied buffer.

The official metadata example and libbladeRF timestamp tests also initialize
continuous receive with RX_NOW. Blueoxide's mock ABI verifies the same input
flag, output timestamp/status/count handling, and Q11 conversion independently
of a loaded vendor library.

## LimeSuite ABI and behavior verification

Primary reference:

- Project: MyriadRF LimeSuite
- Commit: `699d05b7212aa612a9802c219dd6621be88c77db`
- Header: `src/lime/LimeSuite.h`
- C API implementation: `src/API/lms7_api.cpp`
- FIFO/stream implementation: `src/protocols/Streamer.cpp` and
  `src/protocols/fifo.h`

Cross-implementation reference:

- Project: MyriadRF gr-limesdr
- Commit: `244c6bf4f1cb52a8b4d27240d7a4c88c9542cbbb`
- Receive implementation: `lib/source_impl.cc`
- Device configuration: `lib/common/device_handler.cc`
- SoapyLMS7 at the pinned LimeSuite commit: `SoapyLMS7/Streaming.cpp`

The reviewed sources confirm:

| Blueoxide assumption | Reference behavior |
| --- | --- |
| Host floating type | LimeSuite `float_type` is C `double` |
| Stream sample format | `LMS_FMT_F32 = 0`, interleaved I then Q `float` scalars |
| Metadata timestamp | Hardware sample counter for the first returned RX sample |
| Timeout | `LMS_RecvStream` returns zero when no samples are popped |
| Error | `LMS_RecvStream` returns a negative value on failure |
| Status counters | FIFO overrun/underrun and dropped-packet counters reset when queried |
| Stream lifecycle | Setup, start, receive, stop, destroy |
| Calibration | Run after RF configuration; SoapyLMS7 uses at least 2.5 MHz |

gr-limesdr independently uses `LMS_FMT_F32`, passes receive metadata, calls
`LMS_GetStreamStatus` after reads, treats `droppedPackets` as reset-on-read, and
converts the metadata timestamp from sample ticks. SoapyLMS7 independently maps
CF32 to LimeSuite float32 with an int16 link format, treats zero reads as
timeouts, checks non-monotonic timestamps, and calibrates configured channels
when activating streams.

Blueoxide fixes the pinned 64-bit ABI layouts in tests:

| Structure | Size | Alignment |
| --- | ---: | ---: |
| `lms_stream_meta_t` | 16 | 8 |
| `lms_stream_t` | 32 | 8 |
| `lms_stream_status_t` | 48 | 8 |

Mock-native tests additionally verify field values passed to stream setup,
automatic calibration order, exact timestamp-gap arithmetic, status querying
only after nonempty reads, non-finite sample rejection, and cleanup after
initialization/configuration failures.

## libxtrx ABI and behavior verification

Primary reference:

- Project: xtrx-sdr libxtrx
- Commit: `d9599fbf5be2714e6933c5a15acb3d8c57669859`
- Header: `xtrx_api.h`
- High-level implementation: `xtrx.c`
- Official exercise program: `test_xtrx.c`
- Bundled Soapy implementation: `soapy/SoapyXTRX.cpp`

Independent cross-implementation reference:

- Project: Osmocom gr-osmosdr
- Commit: `aa95a6b568e04d3d15a3b4b055562ffa611c217f`
- Receive implementation: `lib/xtrx/xtrx_source_c.cc`
- Shared device configuration: `lib/xtrx/xtrx_obj.cc`

The reviewed sources confirm:

| Blueoxide assumption | Reference behavior |
| --- | --- |
| Receive direction | `XTRX_RX = 1` |
| Channel masks | `XTRX_CH_A = 1`, `XTRX_CH_B = 2`, `XTRX_CH_AB = 3` |
| Single-channel stream | `XTRX_RSP_SISO_MODE`; channel B additionally uses `XTRX_RSP_SWAP_AB` |
| Stream formats | `XTRX_WF_16 = 3`, `XTRX_IQ_INT16 = 2` |
| Host layout | One INT16 complex sample is interleaved I then Q and occupies four bytes |
| Native float scale | libxtrx converts 16-bit wire samples with `1 / 2048` |
| Receive result | Zero is success; negative errno values are failures |
| Timeout request | `RCVEX_TIMOUT` enables finite native timeout reporting |
| Gap policy | `RCVEX_DONT_INSER_ZEROS` skips missing packets; `RCVEX_DROP_OLD_ON_OVERFLOW` resumes at current data |
| Receive metadata | `out_samples`, `out_first_sample`, overflow event, overrun timestamp, and resume timestamp |
| Stream lifecycle | Initialize run parameters, `xtrx_run_ex`, repeated `xtrx_recv_sync_ex`, `xtrx_stop` |

The bundled SoapyXTRX independently uses SISO plus SWAP_AB for channel B,
16-bit wire and host formats for CS16, a 32,768-sample untimed RX start, and
hardware sample timestamps. It publishes 30 MHz through 3.8 GHz RF coverage,
RX sample-rate ranges of 0.2 through 56.25 MHz and 61.4375 through 80 MHz,
1 through 60 MHz receive bandwidth, and a 0 through 30 dB LNA range.

gr-osmosdr independently starts RX through `xtrx_run_ex`, receives through
`xtrx_recv_sync_ex`, requests no inserted zeros plus old-packet dropping, uses
`out_first_sample` for time tags, and returns `out_samples` to its scheduler.
The official `test_xtrx.c` additionally computes dropped samples from
`out_resumed_at - out_overrun_at` when the overflow event is present.

Blueoxide fixes the pinned 64-bit ABI layouts in tests:

| Structure | Size | Alignment |
| --- | ---: | ---: |
| `xtrx_run_stream_params_t` | 48 | 4 |
| `xtrx_run_params_t` | 160 | 8 |
| `xtrx_recv_ex_info_t` | 56 | 8 |

Mock-native tests verify exact run flags and formats, channel-B swap selection,
Q11 edge values, timeout errno recovery, native and timestamp-derived gap
arithmetic, invalid overflow metadata rejection, and stop-before-close cleanup.

An additional Windows FFI smoke test compiled a temporary DLL directly against
the pinned `xtrx_api.h`. Blueoxide loaded that DLL through
`BLUEOXIDE_XTRX_LIBRARY`, validated its run parameters, consumed interleaved
INT16 samples through the real dynamic-call boundary, accepted a fractional
hardware-reported LO, and reported a scripted eight-sample overflow as:

```text
capture complete: samples=3136 packets=0 overruns=1 dropped=8 discontinuities=1
```

This verifies symbol loading and the actual C ABI on the development host, but
does not substitute for a physical-radio test.

## LE ACL encryption verification

Normative references:

- Bluetooth Core Specification, Vol 6, Part E, Sections 1 and 2 for LE
  encryption, nonce construction, packet counters, direction, authenticated
  header masking, MIC generation, and encrypted payload framing.
- Bluetooth Core Specification, Vol 6, Part C, Section 1 for the published
  encryption sample data and intermediate B/X/A/S values.
- FIPS 197 for the AES-128 block-cipher example.

Blueoxide fixes the Core sample session material as:

```text
AES session key: 99ad1b5226a37e3e058e3b8e27c2c666
LL IV octets:    24abdcbabebaafde
```

The material-reconstruction test additionally fixes the complete Core exchange:

```text
LTK (HCI/SMP order): bf01fb9d4ef3bc36d874f5394138684c
LL_ENC_REQ raw:       039078563412efcdab74241302f1e0dfcebdac24abdcba
LL_ENC_RSP raw:       047968574635241302bebaafde
```

The opcode-bearing request contains least-significant-octet-first Rand, EDIV,
SKDm, and IVm; the response contains SKDs and IVs in the same raw captured
order. Tests require the central-to-peripheral request before the
peripheral-to-central response, accept exact retransmissions idempotently,
reject reversed directions and out-of-order responses without state mutation,
and invalidate ready material when a different request starts a refresh.

Pinned Scapy commit `de3399269bad8c9a6bfb1dc181c3876340c198b8`
independently serializes the Core numeric fields through its `XLELongField`,
`XLEShortField`, and `XLEIntField` definitions to the exact raw request and
response parameter octets above. .NET
`System.Security.Cryptography.Aes.EncryptEcb` independently derives
`99ad1b5226a37e3e058e3b8e27c2c666` after converting the raw LTK and SKD fields
to conventional AES order.

The library decrypts and authenticates all four published encrypted Link Layer
examples:

| Direction | Counter | Header | Ciphertext and MIC | Plaintext |
| --- | ---: | --- | --- | --- |
| Central to peripheral | 0 | `0f05` | `9fcda7f448` | `06` |
| Peripheral to central | 0 | `0705` | `a34c13a415` | `06` |
| Central to peripheral | 1 | `0e1f` | `7a70d664...f75a6d33` | 27-octet Core DATA1 payload |
| Peripheral to central | 1 | `061f` | `f38881e7...89b96088` | 27-octet Core DATA2 payload |

The first header octet is authenticated after masking with `0xe3`. Tests modify
NESN, SN, and MD while preserving a valid MIC, then modify LLID and require MIC
failure. Separate cases verify that a failed MIC does not change the next
counter, a retransmission reuses its prior counter, a zero-length PDU consumes
none, and a stale counter can recover only within the configured bounded
search. Arbitrary payload lengths from zero through 255 are exercised without
panics.

Independent implementation reference:

- Project: Zephyr Bluetooth Controller
- Commit: `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`
- Files:
  - `subsys/bluetooth/controller/ll_sw/ull_llcp_enc.c`
  - `subsys/bluetooth/controller/ll_sw/openisa/lll/lll_conn.c`
  - `subsys/bluetooth/controller/ll_sw/openisa/hal/RV32M1/ecb.c`
  - `subsys/bluetooth/controller/ll_sw/openisa/hal/RV32M1/radio/radio.c`
  - `subsys/bluetooth/controller/ll_sw/nordic/hal/nrf5/radio/radio.c`
  - `tests/bluetooth/controller/ctrl_encrypt/src/main.c`

The pinned controller independently confirms counter reset at encryption
setup, central-to-peripheral direction bit one, peripheral-to-central bit zero,
little-endian counter plus IV nonce placement, the `0xe3` ACL header mask,
counter advancement only for accepted nonempty encrypted packets, and
transmit-counter advancement only after acknowledgement rather than on
retransmission. Its controller encryption tests also pin the same Core LTK,
SKDm, SKDs, IVm, IVs, and expected session key while exercising the
central/peripheral LL encryption exchange.

The waveform-backed CLI fixture uses .NET `System.Security.Cryptography.AesCcm`
to generate two valid ATT-bearing ciphertext vectors independently of
Blueoxide:

```text
counter 7 plaintext 050004000c01000200 -> 152f221fb90d46ca3613dc4779
counter 8 plaintext 030004000a0100     -> 550a0e8bb155e2a7db0a85
```

The fixture starts Blueoxide at counter five with a two-counter search, verifies
the authenticated skip to seven, repeats the first packet with changed
NESN/MD, decrypts the next packet at eight, reconstructs both L2CAP/ATT PDUs,
and corrupts one later ciphertext octet. The raw corrupted packet remains in
output, no plaintext is emitted for it, its MIC failure is counted, and
cryptographic state does not advance. The same waveform is decoded a second
time from the Core LTK and complete ENC request/response payloads; stdout and
stderr must match direct session-key mode byte for byte. CLI validation also
covers malformed key and exchange widths, incomplete exchange options, wrong
control opcodes, conflicting asserted directions, excessive skip bounds, and a
packet counter outside 39 bits.

Final local gate for this increment:

```text
158 library tests
4 connection planning/acquisition/synchronization CLI integration tests
7 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

Commands:

```text
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
```

## LE 2M data-channel verification

PHY and controller references:

- Bluetooth Core Specification, Vol 6, Part A, uncoded LE PHY modulation and
  preamble definitions.
- Zephyr Bluetooth Controller commit
  `7d46db352251f85a6bc7b5961fb8a86e2f3125e4`.
- `subsys/bluetooth/controller/ll_sw/pdu.h`, where
  `PDU_PREAMBLE_SIZE(phy)` selects one octet for LE 1M and two for LE 2M.
- `subsys/bluetooth/controller/ll_sw/openisa/hal/RV32M1/radio/radio.c`, which
  selects `DR_2MBPS`, uses GFSK modulation index 0.5, and records two bits per
  microsecond.

The independent bit-level oracle is Jiao Xianjun's BTLE commit
`85401861e8f4b04b90cbaa0394c0f9d45ed02f18`. Its Python
`crc24_core` and `scramble_core` generated this fixed channel-12 vector:

```text
Access address:       12345678
CRC init:             abcdef
Header + payload:     0207030004000a0100
Transmitted CRC:      f2838c
Whitened body:        2ee8f3c789d25da03d55e53c
LE 2M over-air bytes: aaaa785634122ee8f3c789d25da03d55e53c
```

Scapy commit `de3399269bad8c9a6bfb1dc181c3876340c198b8`
independently returned the same `f2838c` CRC. The fixed whitened bytes are
embedded directly in the library and CLI waveform tests; those tests do not
call Blueoxide's CRC or whitening helpers to construct the LE 2M packet.

The library modulates the vector at 2, 4, and 8 samples per symbol, checks
-200 kHz, zero, and +200 kHz carrier offsets in both normal and conjugated
spectra, and then feeds an 8 Msps version through 61-sample stream blocks.
Every case must recover one CRC-valid packet with PHY `LE-2M`, the exact ATT
Read Request payload, the expected inversion state, and a deviation estimate
within 20 kHz of 500 kHz.

The CLI fixture uses 8 Msps, -120 kHz carrier offset, channel 12, 67-sample
blocks, access address `0x12345678`, and CRC init `0xabcdef`. It must emit
`phy=LE-2M`, reconstruct L2CAP CID `0x0004`, decode ATT Read Request handle
`0x0001`, and write the exact dewhitened packet to PCAPNG. The captured
Bluetooth pseudo-header must contain flags `0x4c31`, including PHY value one
for LE 2M. Validation tests reject LE Coded/unknown names and sample rates that
are not integer multiples of 2 MHz before attempting to open the input file.

Final local gate for this increment:

```text
159 library tests
4 connection planning/acquisition/synchronization CLI integration tests
8 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Directional connection PHY verification

The typed control and tracker tests cover exact four-octet
`LL_PHY_UPDATE_IND` decoding, one-hot LE 1M/2M/Coded values, zero as unchanged,
invalid multiple-bit values, and the reserved all-unchanged/nonzero-Instant
combination. Tracker cases apply independent directional values before
returning the instant event across event-counter wrap, preserve unchanged
directions, reject overlapping instant procedures, and confirm that a valid
no-change update does not consume pending state.

The connection CLI tests start at event 65534 and schedule
`--phy-update 2m:coded:1`, verify LE 1M through event zero and the new
directional state at event one, assert already-active PHYs at an arbitrary
planning anchor, reject an invalid all-unchanged update, and reject non-LE-1M
anchor overrides for CONNECT_IND event-zero acquisition.

Final local gate for this increment:

```text
161 library tests
5 connection planning/acquisition/synchronization CLI integration tests
8 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Fixed-channel live data-capture verification

The live data test uses the exact channel-12 body:

```text
Header + CTEInfo + payload: 3e0985050004000a01000200
CRC init:                    abcdef
Transmitted CRC:             421893
```

Scapy commit `de3399269bad8c9a6bfb1dc181c3876340c198b8` was loaded directly
from the pinned source tree and independently returned `421893` from
`BTLE.compute_crc`. The nearby historical `0c` ATT opcode vector remains a
different body with CRC `d482c9`; the two fixtures are intentionally not
interchanged.

The fixed bytes use the independent CRC rather than a value produced by
Blueoxide's CRC implementation, then are whitened and modulated into LE 1M I/Q.
A mock SDR delivers the waveform in four hardware-timestamped blocks starting
at sample 70000.
`capture_data_channel` must recover exactly one packet, preserve header
`3e09`, CTEInfo `85`, all nine Length-counted payload octets, and CRC `421893`,
report a capture-relative packet position, account for every input sample, and
stop the source.

The same capture API also receives the complete independent LE 2M over-air
vector from the earlier PHY section at 8 Msps. That test bypasses Blueoxide's
CRC and whitening generation, requires PHY `LE-2M`, recovers header `0207`,
the seven-octet ATT-bearing payload, and CRC `f2838c`, and verifies the
hardware-applied 8 MHz rate before source start.

Separate tests reject advertising channel 37, a missing access address, a CRC
initializer wider than 24 bits, and an LE 2M sample rate not divisible by
2 MHz before native-library loading. A valid 8 Msps LE 2M `capture-data`
configuration reaches the bladeRF loader, where a deterministic missing-
library override proves command dispatch without requiring attached hardware.

Final local gate for this increment:

```text
164 library tests
5 connection planning/acquisition/synchronization CLI integration tests
8 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
9 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Fixed-channel live event-association verification

The live bridge reuses the already independently checked CSA#2 and
clock-window implementations. Its focused integration vector uses:

```text
Access address:       0x12345678
Channel map:          ffffffff1f
CSA:                  2
Connection interval:  24 (30 ms)
Sample rate:          4000000
First event/sample:   0 / 1000
Tuned channel:        31
```

Under this state, event 0 selects channel 31 and the next recurrence on channel
31 is event 7. Seven intervals advance the expected sample by 840,000, so an
observation at sample 841,050 is accepted as event 7 with a 50-sample late
timing error and `advanced_events=7`. A same-event candidate at sample 1,600 is
first rejected; the subsequent event-7 match proves that rejection did not
mutate tracker state.

CLI integration separately verifies that tracking options without
`--assert-central-observations` fail before backend loading, that asserting
event 0 while tuned to channel 12 reports the independently established
channel-31 mismatch, and that a complete channel-31 configuration reaches the
native backend loader. This is integration coverage of pinned CSA#2 behavior,
not a new independent channel-selection oracle.

The capture path always prints a decoded packet before attempting event
association and performs PCAPNG serialization after either a match or a
nonfatal rejection. The implementation review and tracker-state test therefore
cover the contract that an unmatched candidate cannot suppress raw capture or
stop later association attempts. Over-the-air verification still requires an
attached SDR and a direction-classified connection fixture.

Final local gate for this increment:

```text
166 library tests
5 connection planning/acquisition/synchronization CLI integration tests
8 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
9 live/backend CLI integration tests
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Remaining verification requirements

- Recorded over-the-air fixtures from LimeSDR, bladeRF, and XTRX.
- Live bladeRF smoke tests with libbladeRF and both bladeRF 1 and bladeRF 2.
- bladeRF 2 X2 receive/deinterleaving validation before exposing RX1.
- Live LimeSDR smoke tests across LimeSDR USB, Mini, and PCIe variants.
- Live XTRX smoke tests for channels A and B with induced DMA overruns.
- Native backend error injection and device-removal tests.
- Wireshark/tshark regression checks in CI.
- Long-duration stream tests with sample overruns and retunes.
- Differential tests for extended advertising, data-channel following,
  stateful GATT, EATT, pairing and automatic LTK selection, full LL encryption
  activation/pause state, automatic bidirectional encryption state, automatic
  capture-driven PHY transition delivery and demodulator switching, LE Coded
  PHY demodulation, and Bluetooth Classic as those layers are added.
