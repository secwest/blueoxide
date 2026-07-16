# Blueoxide Verification Record

This file records checks that go beyond Blueoxide's own implementation. External
software is used only during development verification and is not a runtime
dependency.

## Environment

- Date: 2026-07-13 through 2026-07-15
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
| `3e0985050004000a01000200` | `abcdef` | `421893` |

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

Final local gate for this increment:

```text
83 library tests
2 data-channel CLI integration tests
1 advertising decode/PCAPNG integration test
7 live/backend CLI integration tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
git diff --check
```

## Internal audit matrix

The checked-in test suite covers:

- LE 1M at 2, 4, 8, and 16 samples per symbol.
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

Commands:

```text
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo doc --no-deps
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
- Differential tests for extended advertising, data-channel following, L2CAP,
  ATT/GATT, SMP, LE Coded PHY, and Bluetooth Classic as those layers are added.
