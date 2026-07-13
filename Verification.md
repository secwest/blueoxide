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
- Native backend error injection and device-removal tests.
- Wireshark/tshark regression checks in CI.
- Long-duration stream tests with sample overruns and retunes.
- Differential tests for extended advertising, data-channel following, L2CAP,
  ATT/GATT, SMP, LE Coded PHY, and Bluetooth Classic as those layers are added.
