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
            "event=0 channel=25 frequency_hz=2456000000 expected_sample=1000 earliest_sample=1000 latest_sample=1000 widening_samples=0",
            "event=1 channel=20 frequency_hz=2446000000 expected_sample=121000 earliest_sample=120937 latest_sample=121063 widening_samples=63",
            "event=2 channel=6 frequency_hz=2416000000 expected_sample=241000 earliest_sample=240875 latest_sample=241125 widening_samples=125",
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

#[test]
fn cli_connection_sync_recovers_missed_events_and_reanchors() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-sync",
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
            "--peer-sca",
            "0",
            "--receiver-ppm",
            "20",
            "--max-event-advance",
            "5",
            "--observe",
            "21:361050",
            "--observe",
            "34:481020",
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
            "event=3 channel=21 observed_sample=361050 advanced_events=3 missed_events=2 expected_sample=361000 timing=late:50 earliest_sample=360812 latest_sample=361188 widening_samples=188",
            "event=4 channel=34 observed_sample=481020 advanced_events=1 missed_events=0 expected_sample=481050 timing=early:30 earliest_sample=480987 latest_sample=481113 widening_samples=63",
        ]
    );
}

#[test]
fn cli_connection_acquire_uses_connect_ind_window_then_tracks() {
    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-acquire",
            "--access-address",
            "0x12345678",
            "--channel-map",
            "ffffffff1f",
            "--csa",
            "1",
            "--hop",
            "10",
            "--window-size",
            "2",
            "--window-offset",
            "3",
            "--interval",
            "24",
            "--sample-rate",
            "4000000",
            "--connect-sample",
            "1000",
            "--peer-sca",
            "0",
            "--receiver-ppm",
            "20",
            "--central-observe",
            "10:30000",
            "--observe",
            "20:150020",
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
            "event=0 channel=10 central_sample=30000 connect_ind_sample=1000 nominal_start_sample=22376 nominal_end_sample=32376 earliest_sample=22360 latest_sample=32392 widening_samples=16",
            "event=1 channel=20 observed_sample=150020 advanced_events=1 missed_events=0 expected_sample=150000 timing=late:20 earliest_sample=149937 latest_sample=150063 widening_samples=63",
        ]
    );

    let output = Command::new(env!("CARGO_BIN_EXE_blueoxide"))
        .args([
            "connection-acquire",
            "--access-address",
            "0x12345678",
            "--channel-map",
            "ffffffff1f",
            "--csa",
            "1",
            "--hop",
            "10",
            "--interval",
            "24",
            "--sample-rate",
            "4000000",
            "--connect-sample",
            "1000",
            "--observe",
            "10:30000",
        ])
        .output()
        .expect("run blueoxide");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("requires --central-observe CHANNEL:SAMPLE")
    );
}
