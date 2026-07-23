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

fn modulate_bits(bits: impl IntoIterator<Item = bool>) -> Vec<u8> {
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 13];
    for bit in bits {
        let frequency_hz = if bit { 250_000.0 } else { -250_000.0 } + 25_000.0;
        let step = TAU * frequency_hz / 4_000_000.0;
        for _ in 0..4 {
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
    fs::write(&iq_path, modulate_bits(bits)).expect("write fixture");
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
    fs::write(&iq_path, modulate_bits(bits)).expect("write fixture");
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
