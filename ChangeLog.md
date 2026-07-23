# Blueoxide Change Log

## Unreleased

### Added

- A buildable Rust 2024 Cargo package with no third-party runtime dependencies
  in the default receive core.
- Bluetooth LE logical-channel validation and complete 0 through 39 center
  frequency mapping.
- In-tree Bluetooth LE whitening and 24-bit CRC implementations.
- Primary advertising PDU detection for channels 37, 38, and 39 with:
  - Configurable access-address error tolerance.
  - Normal and inverted spectrum handling.
  - Bounded payload parsing.
  - Mandatory CRC validation.
- LE 1M quadrature demodulation with exhaustive integer symbol-phase search and
  robust decision-threshold estimation.
- Bounded streaming LE 1M decoding across input blocks with exact
  access-address sample positions and explicit discontinuity resets.
- Generic uncoded LE demodulation and bounded streaming APIs with compatibility
  wrappers for the original LE 1M interfaces.
- LE 2M data-channel demodulation with a 2 Msymbol/s rate, 500 kHz nominal
  deviation, two-octet preamble handling, spectrum inversion, carrier-offset
  estimation, and cross-block packet recovery.
- Preservation of repeated identical advertisements at distinct sample
  positions.
- Packet-local carrier-offset, modulation-deviation, and discriminator
  separation estimates.
- Interleaved little-endian `f32` and signed 16-bit streaming I/Q readers with
  allocation limits, short-read handling, framing validation, and non-finite
  sample rejection.
- Typed semantic decoding for legacy advertising, direct advertising, scan
  requests/responses, scannable advertising, and CONNECT_IND.
- Strict semantic decoding for CRC-valid primary `ADV_EXT_IND`, including
  bounded common-header flags, typed AdvA/TargetA, CTEInfo, ADI, AuxPtr,
  SyncInfo, signed TxPower, and lossless residual ACAD and advertising data.
- `decode-secondary` fixed-channel LE 1M/2M secondary advertising reception,
  with full eight-bit Length framing through 255 octets, streaming decode,
  extended-header semantics, exact sample positions, and PHY-correct PCAPNG.
- Checked AuxPtr reception windows from exact parent access-address samples,
  including 30/300 microsecond quantization, parent airtime plus 300
  microsecond MAFS validation, 50/500 ppm advertiser accuracy combined with a
  caller receiver bound, LE 1M/2M parent timing, and LE Coded child-preamble
  adjustment.
- Stateful contextual extended-advertising tracking from primary ADV_EXT_IND
  through AUX_ADV_IND and AUX_CHAIN_IND, with channel/PHY/window matching,
  ADI and advertiser consistency, chain-field restrictions, non-mutating
  rejection, explicit reset, caller-bounded lossless data accumulation, and
  completion on the first valid fragment without AuxPtr.
- `extended-advertising-plan` for offline scheduling and chain reassembly from
  CRC-valid packet header/payload bytes sharing one exact sample coordinate
  system.
- Advertising Data structure parsing with known type names and UTF-8 local-name
  access.
- CONNECT_IND validation for timing, latency, channel map, hop increment,
  reserved bits, and supervision-timeout relationships.
- A shared CRC-gated LE frame decoder parameterized by access address, CRC
  initialization, and advertising/data PDU length semantics.
- LE data-channel header decoding for LLID, NESN, SN, MD, CP, reserved bits,
  payload length, optional CTEInfo, and lossless payload/MIC bytes.
- Dependency-free AES-128 and the Bluetooth LE ACL AES-CCM profile with
  four-octet MIC verification, masked-header authentication, 39-bit
  direction-specific nonces, and official Core sample-vector coverage.
- Explicit-state `LeAclDecryptor` support for caller-supplied session keys,
  combined IVs, transmitter direction, and initial packet counters, including
  MIC-gated advancement, retransmission counter reuse, zero-length PDU bypass,
  and bounded counter-skip recovery.
- Direction-checked `LL_ENC_REQ`/`LL_ENC_RSP` material tracking with exact
  retransmission handling, refresh invalidation, Core byte-order conversion,
  and AES session-key plus combined-IV derivation from a caller-selected LTK.
- `decode-data` support for deriving decryption material directly from
  `--ltk`, complete captured `--enc-req`, and complete captured `--enc-rsp`
  payloads as an alternative to supplying `--session-key` and `--iv`.
- CTEInfo decoding for CTE time, AoA/AoD type, and reserved values, with the
  CTEInfo octet included in CRC and capture bytes but excluded from the
  Length-counted payload.
- L2CAP start-header views that retain unimplemented payload unchanged.
- Strict lossless LL control-PDU decoding for every assigned Core 6.1 opcode
  (`0x00..=0x3c`), including exact layouts and field validation for encryption,
  feature/version, connection parameters, data length, PHY, CTE, periodic
  synchronization, CIS, power control, subrating, channel reporting, PAwR
  transfer, 24-octet feature pages, Channel Sounding security/capabilities/
  configuration/start/termination, signed FAE tables, CS channel maps, and
  Frame Space Update; future opcode payloads remain raw.
- Validated LE data-channel maps and Channel Selection Algorithms #1 and #2,
  including Core remapping vectors and 16-bit event-counter wrap behavior.
- CONNECT_IND ChSel handling, first transmit-window bounds, anchor-relative
  event timing, and construction of the selected channel algorithm.
- Validated connection parameters plus strict LL_CONNECTION_UPDATE_IND and
  LL_CHANNEL_MAP_IND parameter decoding.
- Anchored connection-event tracking with 16-bit counter wrapping, Core instant
  ordering, map application before instant-event channel selection, checked
  sample arithmetic, and explicit anchor reacquisition after timing changes.
- Typed LE 1M, LE 2M, and LE Coded directional connection PHY state with
  strict `LL_PHY_UPDATE_IND` one-hot/no-change decoding and application before
  returning the instant event, including event-counter wrap.
- CONNECT_IND construction of connection trackers and direct scheduling of
  supported decoded LL control PDUs.
- Core sleep-clock-accuracy ranges, combined peer/receiver clock-widened sample
  windows, and caps that prevent search windows from consuming adjacent
  connection events.
- CONNECT_IND event-0 acquisition from exact packet sample positions,
  WinOffset/WinSize, selected channel, and a caller-identified CRC-valid central
  transmission; packet direction is not inferred.
- Missed-event observation matching by channel and sample window with bounded
  search, timing-error reporting, and successful-observation re-anchoring.
- Bounded plaintext L2CAP PDU reassembly with explicit link direction,
  independent central/peripheral state, exact consecutive retransmission
  suppression, length/invariant validation, replacement/orphan reporting, and
  discontinuity reset.
- Strict lossless LE L2CAP signaling decoding for CID `0x0005`, including
  exact single-command envelopes, typed disconnection, connection-parameter,
  and credit-based command views, bounded Enhanced Credit Based channel lists,
  raw unknown-command preservation, and non-suppressing CLI error reporting.
- Strict lossless ATT decoding for fixed CID `0x0004`, including every Core 6.1
  opcode, typed request/response/command/notification/indication views, exact
  and variable-record validation, legal final-value truncation, raw unknown
  opcode preservation, and non-suppressing `att_pdu` CLI output.
- Strict lossless LE Security Manager Protocol decoding for fixed CID `0x0006`,
  including all Core 6.1 pairing and key-distribution commands, exact
  cryptographic layouts, pairing-feature and identity-address validation,
  future-command preservation, and non-suppressing `smp_pdu` CLI output.
- Dependency-free PCAPNG output with nanosecond timestamps and
  `LINKTYPE_BLUETOOTH_LE_LL_WITH_PHDR`.
- `blueoxide channels`, `blueoxide decode`, configurable
  `blueoxide decode-data`, and offline `blueoxide connection-plan`,
  `blueoxide connection-acquire`, and `blueoxide connection-sync` commands.
- Offline connection planning options for asserted directional anchor PHYs and
  one scheduled `--phy-update C2P:P2C:INSTANT`, with active PHY state included
  in planned and synchronized event output.
- Explicit `decode-data --phy 1m|2m` selection with LE 1M as the default,
  selected-PHY sample-rate validation, PHY-tagged packet output, and LE 2M
  PCAPNG pseudo-header metadata.
- Opt-in `decode-data --plaintext-l2cap-direction` output for complete,
  direction-asserted plaintext streams.
- Opt-in `decode-data` LE ACL authentication/decryption with separate lossless
  ciphertext and authenticated `decrypted_data` output, decryption/error
  counters, and safe L2CAP reset after MIC failure or confirmed packet loss.
- A hardware-neutral `IqSource` receive trait and SDR configuration validation.
- Receive metadata fields for sample index, dropped samples, and overrun state.
- An in-tree Windows/Unix dynamic-library loader with tested symbol lookup.
- A runtime-loaded libbladeRF receive backend with SC16 Q11 metadata streaming,
  immediate continuous receive, FPGA timestamps, timeout recovery, overrun and
  dropped-sample reporting, applied-rate reporting, deterministic library
  overrides, and validated lifecycle cleanup.
- A runtime-loaded LimeSuite receive backend with:
  - Device-specific RX channel, LO, sample-rate, and LPF capability queries.
  - Interleaved finite `f32` I/Q reception and hardware sample timestamps.
  - Automatic RX calibration after radio configuration, with the established
    2.5 MHz minimum calibration bandwidth.
  - FIFO overrun and dropped-packet status plus exact timestamp-gap accounting.
  - Applied sample-rate/bandwidth reporting, reconfiguration teardown, and
    cleanup after partial initialization or stream failures.
  - Deterministic `BLUEOXIDE_LIMESUITE_LIBRARY` override support.
- A runtime-loaded libxtrx receive backend with:
  - Receive-only SISO operation for hardware channels A and B.
  - `XTRX_WF_16`/`XTRX_IQ_INT16` streaming and in-tree Q11 conversion.
  - Applied sample-rate/bandwidth reporting and validation of XTRX's
    discontinuous supported sample-rate ranges.
  - Finite native timeouts, hardware sample timestamps, overflow intervals,
    exact timestamp-gap accounting, and no synthetic gap filling.
  - Automatic RX antenna selection and explicit 0 through 30 dB LNA gain.
  - Deterministic `BLUEOXIDE_XTRX_LIBRARY` override support.
- A finite live-capture loop with sample and duration limits, cross-block
  decoding, capture-relative packet timestamps, and unconditional source stop
  after stream-time failures.
- A shared live-capture engine for advertising and data decoders, preserving
  applied-rate checks, exact hardware sample positions, discontinuity/drop
  accounting, callback failure behavior, and unconditional source stop.
- `blueoxide backends` and live capture through
  `--device bladerf|limesdr|xtrx`.
- `blueoxide capture-data` for fixed-channel live LE 1M/2M connection traffic
  with caller-supplied access address and CRC initializer, lossless data/CTE
  output, and PHY-tagged BLE PCAPNG.
- Opt-in `capture-data --assert-central-observations` association of live
  packet timestamps with connection events, including first-event channel
  validation, bounded missed-event recovery, clock-widened timing reports, and
  nonfatal candidate rejection that preserves raw packet and PCAPNG output.
- Live CLI validation for duration overflow, zero-sized reads, zero timeouts,
  unsupported devices, and missing native libraries.
- Unit tests for channel mapping, whitening, CRC, PDU validation, GFSK
  demodulation, malformed I/Q input, SDR configuration failures, native backend
  lifecycle, timeout recovery, timestamp gaps/overflow, cleanup, and ABI layout.
- Cross-implementation CRC/whitening fixtures and external Scapy PCAPNG
  interoperability checks.
- Fixed Scapy CRC vectors for CTE-bearing data PDUs and Zephyr/Core CSA#2
  all-channel and remapping vectors.
- Linux and Windows GitHub Actions gates for formatting, tests, Clippy, and
  release builds.
- Initial README and design log.

### Changed

- Defined Blueoxide as a receive and capture package first. Active signal
  injection remains planned but will use a separate transmit subsystem.
- Excluded the original standalone SDR/channelizer sketches from the Cargo build
  until their behavior is migrated to validated backends.
- Updated the CI checkout action after GitHub reported the previous action's
  Node.js runtime as deprecated.

### Known limitations

- The bladeRF, LimeSDR, and XTRX backends have not yet been exercised with
  installed vendor libraries or physical radios in the development
  environment.
- bladeRF live capture currently supports the RX0/X1 stream layout only.
- The uncoded demodulator requires 2 through 64 samples per symbol and an
  integer multiple of the selected 1 MHz or 2 MHz symbol rate.
- Hardware-correlated wall-clock time, calibrated RSSI/SNR, live
  multi-channel connection following and retuning, automatic direction
  classification and unasserted routing of live observations, live or
  AuxPtr-driven secondary advertising reception, periodic-advertising
  synchronization state, automatic direction/counter inference, LTK selection
  from pairing state, LL encryption start/pause state, bidirectional
  encryption tracking, stateful L2CAP channels, GATT/EATT and pairing state,
  capture-driven PHY transition delivery/demodulator switching, Bluetooth
  Classic, and LE Coded PHY demodulation remain to be implemented. Contextual
  AUX_ADV_IND/AUX_CHAIN_IND classification and chain reassembly are available
  offline when packets share exact sample coordinates.
- Channel Sounding and Frame Space LL control syntax is typed, but its
  connection-scoped procedure state is not yet implemented.
