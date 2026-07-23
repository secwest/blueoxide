use blueoxide::ble::{
    BleChannel, LE_ADV_ACCESS_ADDRESS, LE_ADV_CRC_INIT, bytes_to_bits_lsb, crc24_bytes, whiten_bits,
};
use std::f32::consts::TAU;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_path(suffix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must follow the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("blueoxide-cli-{nonce}-{suffix}"))
}

fn modulate_bits(
    bits: impl IntoIterator<Item = bool>,
    samples_per_symbol: usize,
    deviation_hz: f32,
    sample_rate_hz: f32,
) -> Vec<u8> {
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 13];
    for bit in bits {
        let frequency_hz = if bit { deviation_hz } else { -deviation_hz } + 25_000.0;
        let step = TAU * frequency_hz / sample_rate_hz;
        for _ in 0..samples_per_symbol {
            phase += step;
            samples.push((phase.cos(), phase.sin()));
        }
    }
    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    iq_bytes
}

#[test]
fn cli_streams_decodes_and_writes_pcapng() {
    let channel = BleChannel::new(37).expect("valid channel");
    let payload = [
        1, 2, 3, 4, 5, 6, // advertiser
        2, 0x01, 0x06, // flags
        5, 0x09, b'B', b'L', b'U', b'E', // complete local name
    ];
    let mut pdu = vec![0x00, payload.len() as u8];
    pdu.extend_from_slice(&payload);
    pdu.extend_from_slice(&crc24_bytes(&pdu, LE_ADV_CRC_INIT));
    let expected_packet = [
        LE_ADV_ACCESS_ADDRESS.to_le_bytes().as_slice(),
        pdu.as_slice(),
    ]
    .concat();

    let mut body = bytes_to_bits_lsb(&pdu);
    whiten_bits(&mut body, channel);
    let mut bits = bytes_to_bits_lsb(&[0xaa]);
    bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
    bits.extend(body);

    let iq_path = temporary_path("input.cf32");
    let pcap_path = temporary_path("output.pcapng");
    fs::write(&iq_path, modulate_bits(bits, 4, 250_000.0, 4_000_000.0)).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode",
            "--input",
            iq_path.to_str().expect("UTF-8 temporary path"),
            "--format",
            "f32le",
            "--channel",
            "37",
            "--sample-rate",
            "4000000",
            "--block-samples",
            "101",
            "--aa-errors",
            "0",
            "--output-pcap",
            pcap_path.to_str().expect("UTF-8 temporary path"),
        ])
        .output()
        .expect("run blueoxide");

    let _ = fs::remove_file(&iq_path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stdout.contains("ADV_IND advertiser=06:05:04:03:02:01"));
    assert!(stdout.contains("carrier_offset_hz="));
    assert!(stdout.contains("deviation_hz="));
    assert!(stderr.contains("decoded 1 CRC-valid packet(s)"));

    let pcap = fs::read(&pcap_path).expect("read PCAPNG");
    let _ = fs::remove_file(&pcap_path);
    assert_eq!(&pcap[..4], &0x0a0d_0d0au32.to_le_bytes());
    assert!(
        pcap.windows(expected_packet.len())
            .any(|window| window == expected_packet)
    );
}

#[test]
fn cli_decodes_independent_extended_advertising_fixture() {
    // Header, payload, and CRC were generated with Scapy; the channel-37
    // whitening bytes were generated with Jiao Xianjun's BTLE implementation.
    let whitened_body = [
        0xca, 0xc6, 0x47, 0xf8, 0x3c, 0xa5, 0x65, 0xb4, 0x70, 0x37, 0xad, 0x92, 0x42, 0x54, 0xd9,
        0x17, 0x44, 0xe8, 0xad, 0xd2, 0x9f, 0x56, 0xd0, 0xb4, 0x81,
    ];
    let mut bits = bytes_to_bits_lsb(&[0xaa]);
    bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
    bits.extend(bytes_to_bits_lsb(&whitened_body));

    let iq_path = temporary_path("extended-advertising.cf32");
    fs::write(&iq_path, modulate_bits(bits, 4, 250_000.0, 4_000_000.0)).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode",
            "--input",
            iq_path.to_str().expect("UTF-8 temporary path"),
            "--format",
            "f32le",
            "--channel",
            "37",
            "--sample-rate",
            "4000000",
            "--block-samples",
            "101",
            "--aa-errors",
            "0",
        ])
        .output()
        .expect("run blueoxide");

    let _ = fs::remove_file(&iq_path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    for expected in [
        "ADV_EXT_IND",
        "advertiser=06:05:04:03:02:01",
        "sid=13 did=2748",
        "aux_channel=20",
        "aux_offset_us=87300",
        "aux_phy=LE-2M",
        "tx_power_dbm=-12",
        "acad_octets=3",
        "advertising_data_octets=3",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in stdout: {stdout}"
        );
    }
    assert!(stderr.contains("decoded 1 CRC-valid packet(s)"));
}

#[test]
fn cli_decodes_independent_secondary_advertising_fixture() {
    // Scapy generated CRC d63ff5. Jiao Xianjun's BTLE scramble_core generated
    // these channel-20 bytes from a 70-octet secondary advertising payload.
    let whitened_body = [
        0xb3, 0x13, 0x62, 0xc6, 0xa8, 0x1b, 0x6f, 0x59, 0x49, 0x02, 0x2e, 0x3f, 0x84, 0xfe, 0xb9,
        0x53, 0xf9, 0x2e, 0xb1, 0xe1, 0xd3, 0x04, 0xbf, 0x24, 0x2d, 0x0e, 0xc4, 0xfc, 0x01, 0x6f,
        0xcd, 0x3a, 0x6e, 0x01, 0xcf, 0x65, 0x7d, 0x1e, 0x42, 0x0d, 0x08, 0x9d, 0x79, 0x67, 0x98,
        0x1f, 0x4f, 0xb6, 0x9d, 0x2e, 0xc8, 0x1f, 0x12, 0xab, 0x84, 0xa1, 0xa2, 0x6c, 0x9f, 0x92,
        0xec, 0x2f, 0x06, 0x78, 0x6c, 0xb1, 0xc3, 0xaa, 0xad, 0xf9, 0xef, 0xff, 0x92, 0x1f, 0xac,
    ];
    let dewhitened_body = [
        0x47, 0x46, 0x0a, 0x09, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xbc, 0xda, 0x99, 0x00, 0x01,
        0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
        0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0xd6, 0x3f, 0xf5,
    ];
    let mut bits = bytes_to_bits_lsb(&[0xaa, 0xaa]);
    bits.extend(bytes_to_bits_lsb(&LE_ADV_ACCESS_ADDRESS.to_le_bytes()));
    bits.extend(bytes_to_bits_lsb(&whitened_body));

    let iq_path = temporary_path("secondary-advertising.cf32");
    let pcap_path = temporary_path("secondary-advertising.pcapng");
    fs::write(&iq_path, modulate_bits(bits, 4, 500_000.0, 8_000_000.0)).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-secondary",
            "--input",
            iq_path.to_str().expect("UTF-8 temporary path"),
            "--format",
            "f32le",
            "--channel",
            "20",
            "--phy",
            "2m",
            "--sample-rate",
            "8000000",
            "--block-samples",
            "137",
            "--aa-errors",
            "0",
            "--output-pcap",
            pcap_path.to_str().expect("UTF-8 temporary path"),
        ])
        .output()
        .expect("run blueoxide");

    let _ = fs::remove_file(&iq_path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    for expected in [
        "channel=20 phy=LE-2M",
        "header=4746",
        "ADV_EXT_IND",
        "advertiser=06:05:04:03:02:01",
        "sid=13 did=2748",
        "acad_octets=1",
        "advertising_data_octets=59",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in stdout: {stdout}"
        );
    }
    assert!(stderr.contains("decoded 1 CRC-valid packet(s)"));

    let pcap = fs::read(&pcap_path).expect("read PCAPNG");
    let _ = fs::remove_file(&pcap_path);
    let expected_packet = [
        LE_ADV_ACCESS_ADDRESS.to_le_bytes().as_slice(),
        dewhitened_body.as_slice(),
    ]
    .concat();
    let packet_offset = pcap
        .windows(expected_packet.len())
        .position(|window| window == expected_packet)
        .expect("PCAPNG contains exact dewhitened secondary advertising packet");
    let flags = u16::from_le_bytes([pcap[packet_offset - 2], pcap[packet_offset - 1]]);
    assert_eq!(flags & 0x4000, 0x4000);
}

#[test]
fn cli_rejects_invalid_secondary_advertising_configuration_before_input_open() {
    let primary_channel = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-secondary",
            "--input",
            "missing.cf32",
            "--channel",
            "37",
            "--sample-rate",
            "4000000",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(primary_channel.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&primary_channel.stderr)
            .contains("secondary advertising channel in 0..=36")
    );

    let invalid_rate = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-secondary",
            "--input",
            "missing.cf32",
            "--channel",
            "20",
            "--phy",
            "2m",
            "--sample-rate",
            "7000000",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(invalid_rate.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid_rate.stderr).contains("integer multiple of 2000000"));

    let zero_block = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-secondary",
            "--input",
            "missing.cf32",
            "--channel",
            "20",
            "--sample-rate",
            "4000000",
            "--block-samples",
            "0",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(zero_block.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&zero_block.stderr)
            .contains("--block-samples must be greater than zero")
    );
}

#[test]
fn cli_decodes_independent_periodic_advertising_fixture() {
    // Scapy commit de339926 generated CRC 58c8ce with init abcdef.
    // Jiao Xianjun BTLE commit 8540186 generated the channel-27 whitening.
    let whitened_body = [
        0xc4, 0x19, 0x34, 0x42, 0xe3, 0x5f, 0xf4, 0x9d, 0xc2, 0x09, 0x18,
    ];
    let dewhitened_body = [
        0x07, 0x06, 0x03, 0x08, 0xbc, 0xda, 0x02, 0x01, 0x58, 0xc8, 0xce,
    ];
    let access_address = 0x1234_5678u32;
    let mut bits = bytes_to_bits_lsb(&[0xaa, 0xaa]);
    bits.extend(bytes_to_bits_lsb(&access_address.to_le_bytes()));
    bits.extend(bytes_to_bits_lsb(&whitened_body));

    let iq_path = temporary_path("periodic-advertising.cf32");
    let pcap_path = temporary_path("periodic-advertising.pcapng");
    fs::write(&iq_path, modulate_bits(bits, 4, 500_000.0, 8_000_000.0)).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-periodic",
            "--input",
            iq_path.to_str().expect("UTF-8 temporary path"),
            "--format",
            "f32le",
            "--channel",
            "27",
            "--phy",
            "2m",
            "--sample-rate",
            "8000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
            "--block-samples",
            "79",
            "--aa-errors",
            "0",
            "--output-pcap",
            pcap_path.to_str().expect("UTF-8 temporary path"),
        ])
        .output()
        .expect("run blueoxide");

    let _ = fs::remove_file(&iq_path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    for expected in [
        "channel=27 phy=LE-2M",
        "access_address=12345678",
        "header=0706",
        "crc=58c8ce",
        "AUX_SYNC_IND",
        "sid=13 did=2748",
        "advertising_data_octets=2",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in stdout: {stdout}"
        );
    }
    assert!(stderr.contains("decoded 1 CRC-valid packet(s)"));

    let pcap = fs::read(&pcap_path).expect("read PCAPNG");
    let _ = fs::remove_file(&pcap_path);
    let expected_packet = [
        access_address.to_le_bytes().as_slice(),
        dewhitened_body.as_slice(),
    ]
    .concat();
    let packet_offset = pcap
        .windows(expected_packet.len())
        .position(|window| window == expected_packet)
        .expect("PCAPNG contains exact dewhitened periodic advertising packet");
    assert_eq!(
        &pcap[packet_offset - 6..packet_offset - 2],
        &access_address.to_le_bytes()
    );
    let flags = u16::from_le_bytes([pcap[packet_offset - 2], pcap[packet_offset - 1]]);
    assert_eq!(flags & 0x4000, 0x4000);
}

#[test]
fn cli_rejects_invalid_periodic_advertising_configuration_before_input_open() {
    let primary_channel = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-periodic",
            "--input",
            "missing.cf32",
            "--channel",
            "37",
            "--sample-rate",
            "8000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(primary_channel.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&primary_channel.stderr)
            .contains("periodic advertising channel in 0..=36")
    );

    let wide_crc = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-periodic",
            "--input",
            "missing.cf32",
            "--channel",
            "27",
            "--sample-rate",
            "8000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0x1000000",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(wide_crc.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&wide_crc.stderr).contains("exceeds 24 bits"));

    let zero_block = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-periodic",
            "--input",
            "missing.cf32",
            "--channel",
            "27",
            "--sample-rate",
            "8000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
            "--block-samples",
            "0",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(zero_block.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&zero_block.stderr)
            .contains("--block-samples must be greater than zero")
    );
}

#[test]
fn cli_plans_and_reassembles_contextual_extended_advertising() {
    // AuxOffset timing and contextual AUX subtype behavior are fixed from the
    // pinned Zephyr and Wireshark implementations documented in Verification.md.
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "extended-advertising-plan",
            "--sample-rate",
            "4000000",
            "--receiver-ppm",
            "20",
            "--packet",
            "37:1m:1000:47070618bc2a541400",
            "--packet",
            "20:1m:3400:470a0618bc2a551400010203",
            "--packet",
            "21:1m:5800:47060308bc2a0405",
        ])
        .output()
        .expect("run blueoxide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    for expected in [
        "packet=0 kind=ADV_EXT_IND channel=37 phy=LE-1M sample=1000 status=awaiting",
        "next_kind=AUX_ADV_IND next_channel=20",
        "represented_earliest_sample=3400 represented_latest_sample=3520 earliest_sample=3399 latest_sample=3521",
        "packet=1 kind=AUX_ADV_IND channel=20 phy=LE-1M sample=3400 status=awaiting fragments=1 advertising_data_octets=3",
        "next_kind=AUX_CHAIN_IND next_channel=21",
        "packet=2 kind=AUX_CHAIN_IND channel=21 phy=LE-1M sample=5800 status=complete",
        "adi=sid:2:did:2748",
        "fragments=2",
        "advertising_data_octets=5 advertising_data=0102030405",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in stdout: {stdout}"
        );
    }
}

#[test]
fn cli_rejects_extended_advertising_observation_outside_window() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "extended-advertising-plan",
            "--sample-rate",
            "4000000",
            "--packet",
            "37:1m:1000:47070618bc2a541400",
            "--packet",
            "20:1m:4000:47060308bc2a0102",
        ])
        .output()
        .expect("run blueoxide");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("AUX_ADV_IND sample 4000 is outside 3399..=3521")
    );
}

#[test]
fn cli_plans_and_reanchors_periodic_advertising() {
    // SyncInfo interpretation and periodic CSA#2 scheduling are fixed against
    // the pinned Zephyr and Wireshark revisions documented in Verification.md.
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "periodic-advertising-plan",
            "--sync-packet",
            "20:2m:1000:4714132021632000ffffffff7f78563412efcdab6745",
            "--sample-rate",
            "4000000",
            "--receiver-ppm",
            "20",
            "--events",
            "2",
            "--observe",
            "27:2m:10792700",
            "--observe",
            "32:2m:11112705",
            "--max-event-advance",
            "3",
        ])
        .output()
        .expect("run blueoxide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    for expected in [
        "sync_packet channel=20 phy=LE-2M sample=1000 access_address=12345678 crc_init=abcdef interval_us=40000 event=17767 channel_map=ffffffff1f advertiser_sca_ppm=100",
        "observation=0 event=17767 channel=27 phy=LE-2M observed_sample=10792700 advanced_events=0 missed_events=0 timing=on-time:0",
        "represented_earliest_sample=10792600 represented_latest_sample=10793800 earliest_sample=10791305 latest_sample=10795095 widening_samples=1295",
        "observation=1 event=17769 channel=32 phy=LE-2M observed_sample=11112705 advanced_events=2 missed_events=2 timing=late:5",
        "represented_earliest_sample=11112700 represented_latest_sample=11112700 earliest_sample=11112661 latest_sample=11112739 widening_samples=39",
        "plan=0 event=17769 channel=32 frequency_hz=2470000000 phy=LE-2M represented_earliest_sample=11112705 represented_latest_sample=11112705 earliest_sample=11112705 latest_sample=11112705 quantization_width_samples=0 widening_samples=0",
        "plan=1 event=17770 channel=5 frequency_hz=2414000000 phy=LE-2M represented_earliest_sample=11272705 represented_latest_sample=11272705 earliest_sample=11272685 latest_sample=11272725 quantization_width_samples=0 widening_samples=20",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected:?} in stdout: {stdout}"
        );
    }
}
