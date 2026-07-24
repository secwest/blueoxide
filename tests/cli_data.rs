use blueoxide::ble::{BleChannel, bytes_to_bits_lsb, crc24_bytes, whiten_bits};
use blueoxide::demod::LeUncodedPhy;
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
    append_uncoded_packet_samples(
        samples,
        phase,
        channel,
        access_address,
        crc_init,
        header,
        payload,
        LeUncodedPhy::Le1M,
        4,
        -30_000.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn append_uncoded_packet_samples(
    samples: &mut Vec<(f32, f32)>,
    phase: &mut f32,
    channel: BleChannel,
    access_address: u32,
    crc_init: u32,
    header: [u8; 2],
    payload: &[u8],
    phy: LeUncodedPhy,
    samples_per_symbol: usize,
    carrier_offset_hz: f32,
) {
    let mut pdu = Vec::from(header);
    pdu.extend_from_slice(payload);
    pdu.extend_from_slice(&crc24_bytes(&pdu, crc_init));
    let mut body = bytes_to_bits_lsb(&pdu);
    whiten_bits(&mut body, channel);
    let preamble = if access_address & 1 == 0 { 0xaa } else { 0x55 };
    let mut bits = bytes_to_bits_lsb(&vec![preamble; phy.preamble_octets()]);
    bits.extend(bytes_to_bits_lsb(&access_address.to_le_bytes()));
    bits.extend(body);
    append_uncoded_bit_samples(
        samples,
        phase,
        &bits,
        phy,
        samples_per_symbol,
        carrier_offset_hz,
    );
}

fn append_uncoded_bit_samples(
    samples: &mut Vec<(f32, f32)>,
    phase: &mut f32,
    bits: &[bool],
    phy: LeUncodedPhy,
    samples_per_symbol: usize,
    carrier_offset_hz: f32,
) {
    for bit in bits {
        let deviation_hz = phy.nominal_deviation_hz() as f32;
        let frequency_hz = if *bit { deviation_hz } else { -deviation_hz } + carrier_offset_hz;
        let sample_rate_hz = phy.symbol_rate() as f32 * samples_per_symbol as f32;
        let step = TAU * frequency_hz / sample_rate_hz;
        for _ in 0..samples_per_symbol {
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
fn cli_decodes_le_2m_data_channel_waveform() {
    let phy = LeUncodedPhy::Le2M;
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let pdu_with_crc = [
        0x02, 0x07, 0x03, 0x00, 0x04, 0x00, 0x0a, 0x01, 0x00, 0xf2, 0x83, 0x8c,
    ];
    // Jiao Xianjun's BTLE crc24_core/scramble_core independently produced
    // CRC f2838c and channel-12 whitened body 2ee8f3c789d25da03d55e53c.
    let bits = bytes_to_bits_lsb(&[
        0xaa, 0xaa, 0x78, 0x56, 0x34, 0x12, 0x2e, 0xe8, 0xf3, 0xc7, 0x89, 0xd2, 0x5d, 0xa0, 0x3d,
        0x55, 0xe5, 0x3c,
    ]);
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    append_uncoded_bit_samples(&mut samples, &mut phase, &bits, phy, 4, -120_000.0);

    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    let iq_path = temporary_path("le-2m.cf32");
    let pcap_path = temporary_path("le-2m.pcapng");
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
            "--phy",
            "2m",
            "--sample-rate",
            "8000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
            "--block-samples",
            "67",
            "--aa-errors",
            "0",
            "--plaintext-l2cap-direction",
            "central-to-peripheral",
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
    let pcap = fs::read(&pcap_path).expect("read PCAPNG");
    let _ = fs::remove_file(&pcap_path);
    let access_address_bytes = access_address.to_le_bytes();
    let phy_flags = 0x4c31u16.to_le_bytes();
    let expected_pcap_packet = [
        [channel.index(), 0, 0, 0].as_slice(),
        access_address_bytes.as_slice(),
        phy_flags.as_slice(),
        access_address_bytes.as_slice(),
        pdu_with_crc.as_slice(),
    ]
    .concat();
    assert!(
        pcap.windows(expected_pcap_packet.len())
            .any(|window| window == expected_pcap_packet),
        "PCAPNG lacks the LE 2M pseudo-header and packet bytes"
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stdout.contains("channel=12 phy=LE-2M"));
    let deviation_hz: f32 = stdout
        .split_once("deviation_hz=")
        .and_then(|(_, value)| value.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .expect("packet output includes numeric deviation estimate");
    assert!((deviation_hz - 500_000.0).abs() < 20_000.0);
    assert!(stdout.contains(
        "l2cap_pdu direction=central-to-peripheral cid=0x0004 length=3 fragments=1 payload=0a0100"
    ));
    assert!(stdout.contains(
        "att_pdu direction=central-to-peripheral opcode=0x0a name=read-request type=request handle=0x0001"
    ));
    assert!(stderr.contains("decoded 1 CRC-valid data-channel packet(s)"));
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
            "--session-key",
            "00",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected 16 hexadecimal octets"));

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-data",
            "--input",
            "unused.cf32",
            "--channel",
            "0",
            "--phy",
            "coded",
            "--sample-rate",
            "4000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected 1m or 2m"));

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "decode-data",
            "--input",
            "unused.cf32",
            "--channel",
            "0",
            "--phy",
            "2m",
            "--sample-rate",
            "5000000",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("LE-2M requires a sample rate that is an integer multiple of 2000000 Hz")
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
            "0xabcdef",
            "--enc-req",
            "03",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected 23 hexadecimal octets"));

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
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--enc-req",
            "039078563412efcdab74241302f1e0dfcebdac24abdcba",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--ltk, --enc-req, and --enc-rsp must be supplied together")
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
            "0xabcdef",
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--enc-req",
            "049078563412efcdab74241302f1e0dfcebdac24abdcba",
            "--enc-rsp",
            "047968574635241302bebaafde",
            "--decrypt-direction",
            "central-to-peripheral",
            "--packet-counter",
            "0",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--enc-req must begin with LL_ENC_REQ opcode 03")
    );

    let decryption_options = [
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
        "--session-key",
        "99ad1b5226a37e3e058e3b8e27c2c666",
        "--iv",
        "24abdcbabebaafde",
        "--decrypt-direction",
        "central-to-peripheral",
        "--packet-counter",
        "0",
    ];
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args(decryption_options)
        .args(["--plaintext-l2cap-direction", "peripheral-to-central"])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--plaintext-l2cap-direction must match --decrypt-direction")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args(decryption_options)
        .args(["--max-counter-skip", "65536"])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--max-counter-skip must be in 0..=65535")
    );

    let mut wide_counter_options = decryption_options;
    wide_counter_options[18] = "549755813888";
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args(wide_counter_options)
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds the 39-bit range"));
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
fn cli_authenticates_decrypts_and_reassembles_encrypted_waveforms() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let crc_init = 0x00ab_cdef;
    let first_ciphertext = [
        0x15, 0x2f, 0x22, 0x1f, 0xb9, 0x0d, 0x46, 0xca, 0x36, 0x13, 0xdc, 0x47, 0x79,
    ];
    let second_ciphertext = [
        0x55, 0x0a, 0x0e, 0x8b, 0xb1, 0x55, 0xe2, 0xa7, 0xdb, 0x0a, 0x85,
    ];
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, first_ciphertext.len() as u8],
        &first_ciphertext,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x16, first_ciphertext.len() as u8],
        &first_ciphertext,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x0a, second_ciphertext.len() as u8],
        &second_ciphertext,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    let mut damaged = second_ciphertext;
    damaged[0] ^= 1;
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, damaged.len() as u8],
        &damaged,
    );

    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    let iq_path = temporary_path("encrypted.cf32");
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
            "--session-key",
            "99ad1b5226a37e3e058e3b8e27c2c666",
            "--iv",
            "24abdcbabebaafde",
            "--decrypt-direction",
            "central-to-peripheral",
            "--packet-counter",
            "5",
            "--max-counter-skip",
            "2",
            "--plaintext-l2cap-direction",
            "central-to-peripheral",
        ])
        .output()
        .expect("run blueoxide");
    let derived_output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
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
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--enc-req",
            "039078563412efcdab74241302f1e0dfcebdac24abdcba",
            "--enc-rsp",
            "047968574635241302bebaafde",
            "--decrypt-direction",
            "central-to-peripheral",
            "--packet-counter",
            "5",
            "--max-counter-skip",
            "2",
            "--plaintext-l2cap-direction",
            "central-to-peripheral",
        ])
        .output()
        .expect("run blueoxide with derived encryption material");

    let _ = fs::remove_file(&iq_path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        derived_output.status.success(),
        "derived stderr: {}",
        String::from_utf8_lossy(&derived_output.stderr)
    );
    assert_eq!(derived_output.stdout, output.stdout);
    assert_eq!(derived_output.stderr, output.stderr);
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stdout.contains("payload=152f221fb90d46ca3613dc4779 crc="));
    assert!(stdout.contains("plaintext_hint=\"encrypted\""));
    assert!(stdout.contains(
        "decrypted_data direction=central-to-peripheral status=new packet_counter=7 skipped_counters=2 header=0209 payload=050004000c01000200"
    ));
    assert!(stdout.contains(
        "decrypted_data direction=central-to-peripheral status=retransmission packet_counter=7 skipped_counters=0 header=1609 payload=050004000c01000200"
    ));
    assert!(stdout.contains(
        "decrypted_data direction=central-to-peripheral status=new packet_counter=8 skipped_counters=0 header=0a07 payload=030004000a0100"
    ));
    assert!(stdout.contains(
        "att_pdu direction=central-to-peripheral opcode=0x0c name=read-blob-request type=request handle=0x0001 offset=2"
    ));
    assert!(stdout.contains(
        "att_pdu direction=central-to-peripheral opcode=0x0a name=read-request type=request handle=0x0001"
    ));
    assert!(stdout.contains("payload=540a0e8bb155e2a7db0a85"));
    assert!(stderr.contains("LE ACL decryption error: direction=central-to-peripheral"));
    assert!(stderr.contains(
        "authenticated 2 new encrypted packet(s); retransmissions=1 unencrypted_empty=0 skipped_counters=2 errors=1"
    ));
    assert!(stderr.contains(
        "reassembled 2 plaintext L2CAP PDU(s); duplicates=1 orphan_continuations=0 discarded_incomplete=0 errors=0 signaling_errors=0 att_errors=0 smp_errors=0"
    ));
}

#[test]
fn cli_validates_direction_tagged_encryption_trace_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args(["encryption-trace", "--packet", "c2p:030105"])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("encryption-trace requires --ltk"));

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "encryption-trace",
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("encryption-trace requires at least one --packet")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "encryption-trace",
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--packet",
            "c2p:030203",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("data Length declares 2 payload octets but 1 were supplied")
    );
}

#[test]
fn cli_tracks_initial_pause_refresh_and_bidirectional_encrypted_data() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "encryption-trace",
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--packet",
            "c2p:0317039078563412efcdab74241302f1e0dfcebdac24abdcba",
            "--packet",
            "p2c:070d047968574635241302bebaafde",
            "--packet",
            "p2c:030105",
            "--packet",
            "c2p:13059fcda7f448",
            "--packet",
            "p2c:0705a34c13a415",
            "--packet",
            "c2p:0f056705b5b139",
            "--packet",
            "p2c:0b05ef83ed096c",
            "--packet",
            "c2p:07010b",
            "--packet",
            "c2p:0317039078563412efcdab74241202f1e0dfcebdac24abdcba",
            "--packet",
            "p2c:070d047868574635241302bebaafdf",
            "--packet",
            "p2c:030105",
            "--packet",
            "c2p:1305b2dd7a7e9a",
            "--packet",
            "p2c:07050ce74620c8",
            "--packet",
            "c2p:020d265ced8b95e2f33651732dace1",
            "--packet",
            "p2c:060b0d567b5b51aa49678426f8",
        ])
        .output()
        .expect("run blueoxide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stdout.contains(
        "raw_encryption_packet index=3 direction=central-to-peripheral header=1305 payload=9fcda7f448"
    ));
    assert!(stdout.contains(
        "protection=encrypted-new packet_counter=0 skipped_counters=0 state_before=awaiting-central-start-encryption-response state_after=awaiting-peripheral-start-encryption-response header=1301 payload=06 control=LL_START_ENC_RSP"
    ));
    assert!(stdout.contains(
        "state_before=awaiting-peripheral-pause-encryption-response state_after=awaiting-central-pause-encryption-response"
    ));
    assert!(stdout.contains(
        "state_before=awaiting-central-pause-encryption-response state_after=awaiting-refresh-encryption-request header=0701 payload=0b control=LL_PAUSE_ENC_RSP"
    ));
    assert!(stdout.contains(
        "encryption_observation index=13 direction=central-to-peripheral protection=encrypted-new packet_counter=1 skipped_counters=0 state_before=encrypted state_after=encrypted header=0209 payload=050004000a01000200 control=none"
    ));
    assert!(stdout.contains(
        "encryption_observation index=14 direction=peripheral-to-central protection=encrypted-new packet_counter=1 skipped_counters=0 state_before=encrypted state_after=encrypted header=0607 payload=030004000a0100 control=none"
    ));
    assert!(stderr.contains(
        "processed 15 directed encryption packet(s); accepted=15 errors=0 final_state=encrypted"
    ));
}

#[test]
fn cli_keeps_raw_packets_visible_and_continues_after_mic_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "encryption-trace",
            "--ltk",
            "bf01fb9d4ef3bc36d874f5394138684c",
            "--packet",
            "c2p:0317039078563412efcdab74241302f1e0dfcebdac24abdcba",
            "--packet",
            "p2c:070d047968574635241302bebaafde",
            "--packet",
            "p2c:030105",
            "--packet",
            "c2p:13059fcda7f448",
            "--packet",
            "p2c:0705a34c13a415",
            "--packet",
            "c2p:0f056605b5b139",
            "--packet",
            "c2p:0f056705b5b139",
        ])
        .output()
        .expect("run blueoxide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stdout.contains(
        "raw_encryption_packet index=5 direction=central-to-peripheral header=0f05 payload=6605b5b139"
    ));
    assert!(stdout.contains(
        "encryption_observation index=6 direction=central-to-peripheral protection=encrypted-new packet_counter=1"
    ));
    assert!(stderr.contains(
        "encryption observation error: index=5 direction=central-to-peripheral state=encrypted"
    ));
    assert!(stderr.contains(
        "processed 7 directed encryption packet(s); accepted=6 errors=1 final_state=awaiting-peripheral-pause-encryption-response"
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

#[test]
fn cli_decodes_plaintext_smp_without_hiding_raw_pdus() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let crc_init = 0x00ab_cdef;
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    let pairing_request = [
        7, 0, // L2CAP payload length
        6, 0, // LE Security Manager fixed channel
        0x01, 0x03, 0x00, 0x0d, 16, 0x07, 0x06,
    ];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x02, pairing_request.len() as u8],
        &pairing_request,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    let malformed_security_request = [
        3, 0, // L2CAP payload length
        6, 0, // LE Security Manager fixed channel
        0x0b, 0x01, 0xff, // known command with one trailing octet
    ];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x0a, malformed_security_request.len() as u8],
        &malformed_security_request,
    );

    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    let iq_path = temporary_path("smp.cf32");
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
            "79",
            "--aa-errors",
            "0",
            "--plaintext-l2cap-direction",
            "central-to-peripheral",
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
        "l2cap_pdu direction=central-to-peripheral cid=0x0006 length=7 fragments=1 payload=0103000d100706"
    ));
    assert!(stdout.contains(
        "smp_pdu direction=central-to-peripheral code=0x01 name=pairing-request io_capability=no-input-no-output oob=false auth=0x0d bonding=true mitm=true secure_connections=true keypress=false ct2=false maximum_key_size=16"
    ));
    assert!(stdout.contains(
        "l2cap_pdu direction=central-to-peripheral cid=0x0006 length=3 fragments=1 payload=0b01ff"
    ));
    assert!(stderr.contains("plaintext SMP PDU decode error: direction=central-to-peripheral"));
    assert!(stderr.contains(
        "reassembled 2 plaintext L2CAP PDU(s); duplicates=0 orphan_continuations=0 discarded_incomplete=0 errors=0 signaling_errors=0 att_errors=0 smp_errors=1"
    ));
}

#[test]
fn cli_decodes_ll_control_pdus_without_hiding_malformed_packets() {
    let channel = BleChannel::new(12).expect("valid channel");
    let access_address = 0x1234_5678u32;
    let crc_init = 0x00ab_cdef;
    let mut phase = 0.0f32;
    let mut samples = vec![(1.0f32, 0.0f32); 11];
    let length_request = [0x14, 251, 0, 0x48, 0x08, 27, 0, 0x48, 0x08];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x03, length_request.len() as u8],
        &length_request,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    let frame_space_request = [0x3b, 100, 0, 200, 0, 3, 5, 0];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x0b, frame_space_request.len() as u8],
        &frame_space_request,
    );
    samples.extend(std::iter::repeat_n((phase.cos(), phase.sin()), 160));
    let malformed_cs_config_response = [0x31, 0x02, 0xff];
    append_packet_samples(
        &mut samples,
        &mut phase,
        channel,
        access_address,
        crc_init,
        [0x03, malformed_cs_config_response.len() as u8],
        &malformed_cs_config_response,
    );

    let mut iq_bytes = Vec::with_capacity(samples.len() * 8);
    for (i, q) in samples {
        iq_bytes.extend_from_slice(&i.to_le_bytes());
        iq_bytes.extend_from_slice(&q.to_le_bytes());
    }
    let iq_path = temporary_path("ll-control.cf32");
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
            "71",
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
    assert!(stdout.contains("payload=14fb0048081b004808"));
    assert!(stdout.contains(
        "plaintext_hint=\"LL_LENGTH_REQ opcode=0x14 max_rx_octets=251 max_rx_time_us=2120 max_tx_octets=27 max_tx_time_us=2120\""
    ));
    assert!(stdout.contains("payload=3b6400c800030500"));
    assert!(stdout.contains(
        "plaintext_hint=\"LL_FRAME_SPACE_REQ opcode=0x3b minimum_us=100 maximum_us=200 phys=0x03 spacing_types=0x0005\""
    ));
    assert!(stdout.contains("payload=3102ff"));
    assert!(stdout.contains("plaintext_hint=\"decode_error="));
    assert!(stderr.contains("LL control PDU decode error: opcode=0x31"));
    assert!(stderr.contains("ll_control_errors=1"));
}
