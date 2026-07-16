use blueoxide::ble::{BleChannel, bytes_to_bits_lsb, whiten_bits};
use std::f32::consts::TAU;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_path(suffix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must follow the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("blueoxide-data-cli-{nonce}-{suffix}"))
}

#[test]
fn cli_decodes_data_channel_l2cap_and_writes_pcapng() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let payload = [
        5, 0, // L2CAP payload length
        4, 0, // ATT fixed channel
        0x0a, 1, 0, 2, 0, // ATT Read Request fragment
    ];
    let cte_info = 0x85;
    let mut pdu = vec![0x3e, payload.len() as u8, cte_info];
    pdu.extend_from_slice(&payload);
    pdu.extend_from_slice(&[0x42, 0x18, 0x93]);
    let expected_packet = [access_address.to_le_bytes().as_slice(), pdu.as_slice()].concat();

    let mut body = bytes_to_bits_lsb(&pdu);
    whiten_bits(&mut body, channel);
    let mut bits = bytes_to_bits_lsb(&[0xaa]);
    bits.extend(bytes_to_bits_lsb(&access_address.to_le_bytes()));
    bits.extend(body);

    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    for bit in bits {
        let frequency_hz = if bit { 250_000.0 } else { -250_000.0 } - 30_000.0;
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
            "decode-data",
            "--input",
            iq_path.to_str().expect("UTF-8 temporary path"),
            "--format",
            "f32le",
            "--channel",
            "12",
            "--sample-rate",
            "4000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
            "--block-samples",
            "73",
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
    assert!(stdout.contains("access_address=12345678"));
    assert!(stdout.contains("llid=start-or-complete"));
    assert!(stdout.contains("cte=0x85:AoD-2us:40us:rfu=false:reserved=false"));
    assert!(stdout.contains("L2CAP length=5 cid=0x0004"));
    assert!(stderr.contains("decoded 1 CRC-valid data-channel packet(s)"));

    let pcap = fs::read(&pcap_path).expect("read PCAPNG");
    let _ = fs::remove_file(&pcap_path);
    assert_eq!(&pcap[..4], &0x0a0d_0d0au32.to_le_bytes());
    assert!(
        pcap.windows(expected_packet.len())
            .any(|window| window == expected_packet)
    );
}

#[test]
fn cli_rejects_advertising_channel_and_wide_crc_init() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-data",
            "--input",
            "unused.cf32",
            "--channel",
            "37",
            "--sample-rate",
            "4000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0x1000000",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("decode-data requires a data channel")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-data",
            "--input",
            "unused.cf32",
            "--channel",
            "0",
            "--sample-rate",
            "4000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0x1000000",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds 24 bits"));
}
