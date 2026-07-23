use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args(args)
        .output()
        .expect("run blueoxide")
}

#[test]
fn backends_reports_missing_override_without_failing_command() {
    let missing = std::env::temp_dir().join("blueoxide-library-that-does-not-exist.dll");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .arg("backends")
        .env("BLUEOXIDE_BLADERF_LIBRARY", &missing)
        .env("BLUEOXIDE_LIMESUITE_LIBRARY", &missing)
        .env("BLUEOXIDE_XTRX_LIBRARY", &missing)
        .output()
        .expect("run blueoxide");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bladerf  unavailable:"));
    assert!(stdout.contains(&missing.to_string_lossy().to_string()));
    assert!(stdout.contains("limesdr  unavailable:"));
    assert!(stdout.contains("xtrx     unavailable:"));
}

#[test]
fn capture_rejects_unsupported_device_before_loading_native_library() {
    let output = run(&["capture", "--device", "unknown", "--channel", "37"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("capture device \"unknown\" is not implemented")
    );
}

#[test]
fn capture_rejects_unrepresentable_duration_without_panicking() {
    let output = run(&[
        "capture",
        "--device",
        "bladerf",
        "--channel",
        "37",
        "--seconds",
        "1e300",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--seconds must be finite"));
    assert!(!stderr.to_ascii_lowercase().contains("panicked"));
}

#[test]
fn capture_validates_buffer_and_timeout_before_loading_library() {
    for (option, value, expected) in [
        ("--block-samples", "0", "--block-samples must be"),
        ("--read-timeout-ms", "0", "--read-timeout-ms must be"),
    ] {
        let output = run(&[
            "capture",
            "--device",
            "bladerf",
            "--channel",
            "37",
            option,
            value,
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}

#[test]
fn capture_data_validates_connection_before_loading_library() {
    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "37",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0xabcdef",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("data channel in 0..=36"));

    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "12",
        "--crc-init",
        "0xabcdef",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires --access-address"));

    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "12",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0x1000000",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("exceeds 24 bits"));

    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "12",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0xabcdef",
        "--phy",
        "2m",
        "--sample-rate",
        "3000000",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("integer multiple of 2000000 Hz"));

    let output = run(&[
        "capture-data",
        "--device",
        "unknown",
        "--channel",
        "12",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0xabcdef",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("capture-data device \"unknown\" is not implemented")
    );

    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "12",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0xabcdef",
        "--channel-map",
        "ffffffff1f",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("require --assert-central-observations")
    );

    let output = run(&[
        "capture-data",
        "--device",
        "bladerf",
        "--channel",
        "12",
        "--access-address",
        "0x12345678",
        "--crc-init",
        "0xabcdef",
        "--assert-central-observations",
        "--first-event",
        "0",
        "--channel-map",
        "ffffffff1f",
        "--csa",
        "2",
        "--interval",
        "24",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("event 0 uses channel 31, not tuned channel 12")
    );
}

#[test]
fn capture_data_valid_configuration_reaches_native_backend() {
    let missing = std::env::temp_dir().join("blueoxide-data-library-that-does-not-exist.dll");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "capture-data",
            "--device",
            "bladerf",
            "--channel",
            "31",
            "--access-address",
            "0x12345678",
            "--crc-init",
            "0xabcdef",
            "--phy",
            "2m",
            "--sample-rate",
            "8000000",
            "--assert-central-observations",
            "--first-event",
            "0",
            "--channel-map",
            "ffffffff1f",
            "--csa",
            "2",
            "--interval",
            "24",
            "--seconds",
            "0.001",
        ])
        .env("BLUEOXIDE_BLADERF_LIBRARY", &missing)
        .output()
        .expect("run blueoxide");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to load native library"));
    assert!(stderr.contains(&missing.to_string_lossy().to_string()));
}

#[test]
fn capture_missing_library_is_reported_as_an_error() {
    let missing = std::env::temp_dir().join("blueoxide-library-that-does-not-exist.dll");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "capture",
            "--device",
            "bladerf",
            "--channel",
            "37",
            "--seconds",
            "0.001",
        ])
        .env("BLUEOXIDE_BLADERF_LIBRARY", &missing)
        .output()
        .expect("run blueoxide");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to load native library"));
    assert!(stderr.contains(&missing.to_string_lossy().to_string()));
}

#[test]
fn limesdr_capture_missing_library_is_reported_as_an_error() {
    let missing = std::env::temp_dir().join("blueoxide-limesuite-that-does-not-exist.dll");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "capture",
            "--device",
            "limesdr",
            "--channel",
            "37",
            "--seconds",
            "0.001",
        ])
        .env("BLUEOXIDE_LIMESUITE_LIBRARY", &missing)
        .output()
        .expect("run blueoxide");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to load native library"));
    assert!(stderr.contains(&missing.to_string_lossy().to_string()));
}

#[test]
fn xtrx_capture_missing_library_is_reported_as_an_error() {
    let missing = std::env::temp_dir().join("blueoxide-libxtrx-that-does-not-exist.dll");
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "capture",
            "--device",
            "xtrx",
            "--channel",
            "37",
            "--seconds",
            "0.001",
        ])
        .env("BLUEOXIDE_XTRX_LIBRARY", &missing)
        .output()
        .expect("run blueoxide");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to load native library"));
    assert!(stderr.contains(&missing.to_string_lossy().to_string()));
}
