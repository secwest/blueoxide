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

    let iq_path = temporary_path("input.cf32");
    let pcap_path = temporary_path("output.pcapng");
    fs::write(&iq_path, iq_bytes).expect("write fixture");
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
