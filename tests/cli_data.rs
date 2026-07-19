use blueoxide::ble::{BleChannel, bytes_to_bits_lsb, crc24_bytes, whiten_bits};
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

fn append_packet_samples(
    samples: &mut Vec<(f32, f32)>,
    phase: &mut f32,
    channel: BleChannel,
    access_address: u32,
    crc_init: u32,
    header: [u8; 2],
    payload: &[u8],
) {
    let mut pdu = Vec::from(header);
    pdu.extend_from_slice(payload);
    pdu.extend_from_slice(&crc24_bytes(&pdu, crc_init));
    let mut body = bytes_to_bits_lsb(&pdu);
    whiten_bits(&mut body, channel);
    let mut bits = bytes_to_bits_lsb(&[0xaa]);
    bits.extend(bytes_to_bits_lsb(&access_address.to_le_bytes()));
    bits.extend(body);
    for bit in bits {
        let frequency_hz = if bit { 250_000.0 } else { -250_000.0 } - 30_000.0;
        let step = TAU * frequency_hz / 4_000_000.0;
        for _ in 0..4 {
            *phase += step;
            samples.push((phase.cos(), phase.sin()));
        }
    }
}

#[test]
fn cli_decodes_data_channel_l2cap_and_writes_pcapng() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let payload = [
        5, 0, // L2CAP payload length
        4, 0, // ATT fixed channel
        0x0c, 1, 0, 2, 0, // ATT Read Blob Request
    ];
    let cte_info = 0x85;
    let mut pdu = vec![0x3e, payload.len() as u8, cte_info];
    pdu.extend_from_slice(&payload);
    pdu.extend_from_slice(&[0xd4, 0x82, 0xc9]);
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
            "0xabcdef",
            "--max-l2cap-payload",
            "64",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--max-l2cap-payload requires --plaintext-l2cap-direction")
    );
}

#[test]
fn cli_reassembles_asserted_plaintext_l2cap_fragments() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let crc_init = 0x00ab_cdef;
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, 6],
        &[5, 0, 4, 0, 0x0c, 1],
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x09, 3],
        &[0, 2, 0],
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, 9],
        &[5, 0, 4, 0, 0x0a, 1, 0, 2, 0],
    );
    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }

    let iq_path = temporary_path("fragmented.cf32");
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
            "89",
            "--aa-errors",
            "0",
            "--plaintext-l2cap-direction",
            "central-to-peripheral",
            "--max-l2cap-payload",
            "64",
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
    assert!(stdout.contains(
        "l2cap_pdu direction=central-to-peripheral cid=0x0004 length=5 fragments=2 payload=0c01000200"
    ));
    assert!(stdout.contains(
        "att_pdu direction=central-to-peripheral opcode=0x0c name=read-blob-request type=request handle=0x0001 offset=2"
    ));
    assert!(stdout.contains(
        "l2cap_pdu direction=central-to-peripheral cid=0x0004 length=5 fragments=1 payload=0a01000200"
    ));
    assert!(stderr.contains("plaintext ATT PDU decode error: direction=central-to-peripheral"));
    assert!(stderr.contains("decoded 3 CRC-valid data-channel packet(s)"));
    assert!(stderr.contains(
        "reassembled 2 plaintext L2CAP PDU(s); duplicates=0 orphan_continuations=0 discarded_incomplete=0 errors=0 signaling_errors=0 att_errors=1"
    ));
}

#[test]
fn cli_decodes_plaintext_le_l2cap_signaling_without_hiding_raw_pdus() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let crc_init = 0x00ab_cdef;
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    let connection_parameter_request = [
        12, 0, // L2CAP payload length
        5, 0, // LE signaling fixed channel
        0x12, 7, 8, 0, // command header
        24, 0, 40, 0, 0, 0, 200, 0, // command parameters
    ];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, connection_parameter_request.len() as u8],
        &connection_parameter_request,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    let malformed_response = [
        7, 0, // L2CAP payload length
        5, 0, // LE signaling fixed channel
        0x13, 8, 3, 0, 0, 0, 0xff, // valid envelope, invalid command size
    ];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x0a, malformed_response.len() as u8],
        &malformed_response,
    );

    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    let iq_path = temporary_path("signaling.cf32");
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
            "83",
            "--aa-errors",
            "0",
            "--plaintext-l2cap-direction",
            "peripheral-to-central",
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
    assert!(stdout.contains(
        "l2cap_pdu direction=peripheral-to-central cid=0x0005 length=12 fragments=1 payload=12070800180028000000c800"
    ));
    assert!(stdout.contains(
        "l2cap_signal direction=peripheral-to-central code=0x12 name=connection-parameter-update-request identifier=7 minimum_interval=24 maximum_interval=40 latency=0 supervision_timeout=200"
    ));
    assert!(stdout.contains(
        "l2cap_pdu direction=peripheral-to-central cid=0x0005 length=7 fragments=1 payload=130803000000ff"
    ));
    assert!(stderr.contains(
        "plaintext L2CAP signaling command decode error: direction=peripheral-to-central"
    ));
    assert!(stderr.contains("signaling_errors=1"));
}
