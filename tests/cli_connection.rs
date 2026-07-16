use std::process::Command;

#[test]
fn cli_prints_core_csa2_connection_plan_with_sample_timing() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-plan",
            "--access-address",
            "0x8e89bed6",
            "--channel-map",
            "ffffffff1f",
            "--csa",
            "2",
            "--interval",
            "24",
            "--sample-rate",
            "4000000",
            "--anchor-event",
            "0",
            "--anchor-sample",
            "1000",
            "--events",
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
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        [
            "event=0 channel=25 frequency_hz=2456000000 expected_sample=1000",
            "event=1 channel=20 frequency_hz=2446000000 expected_sample=121000",
            "event=2 channel=6 frequency_hz=2416000000 expected_sample=241000",
        ]
    );
}

#[test]
fn cli_connection_plan_wraps_event_counter_and_rejects_bad_state() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-plan",
            "--access-address",
            "0x12345678",
            "--channel-map",
            "0600000000",
            "--csa",
            "1",
            "--hop",
            "5",
            "--interval",
            "24",
            "--sample-rate",
            "4000000",
            "--anchor-event",
            "65535",
            "--events",
            "2",
        ])
        .output()
        .expect("run blueoxide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert!(stdout.contains("event=65535"));
    assert!(stdout.contains("event=0"));
    assert!(stdout.contains("expected_sample=120000"));

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-plan",
            "--access-address",
            "0x12345678",
            "--channel-map",
            "0100000000",
            "--csa",
            "1",
            "--interval",
            "24",
            "--sample-rate",
            "4000000",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("enables fewer than two channels"));
}
