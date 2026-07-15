# Blueoxide Verification Record

This file records checks that go beyond Blueoxide's own implementation. External
software is used only during development verification and is not a runtime
dependency.

## Environment

- Date: 2026-07-13
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
- Native backend error injection and device-removal tests.
- Wireshark/tshark regression checks in CI.
- Long-duration stream tests with sample overruns and retunes.
- Differential tests for extended advertising, data-channel following, L2CAP,
  ATT/GATT, SMP, LE Coded PHY, and Bluetooth Classic as those layers are added.
