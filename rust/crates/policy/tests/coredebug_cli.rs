// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_help() {
    let bin = env!("CARGO_BIN_EXE_coredebug");

    for arg in &["help", "--help", "-h"] {
        let output = std::process::Command::new(bin)
            .arg(arg)
            .output()
            .expect("failed to execute coredebug");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("CoreShift Policy Diagnostics"));
        assert!(stdout.contains("Usage: coredebug"));
    }
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_probe_placeholder() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .arg("probe")
        .output()
        .expect("failed to execute coredebug");

    // This might succeed or fail depending on environment, but it should
    // contain the "available" message.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Running available substrate diagnostic probes"));
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_probe_paths_exit_status() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .args(["probe", "paths"])
        .output()
        .expect("failed to execute coredebug");

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("RESULT: PASS") {
        assert!(output.status.success());
    } else if stdout.contains("RESULT: FAIL") {
        assert!(!output.status.success());
    }
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_probe_paths() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .args(["probe", "paths"])
        .output()
        .expect("failed to execute coredebug");

    // We don't strictly assert success here because environment permissions
    // might vary, but we check if it actually ran the probe.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- Probing paths ---"));
    assert!(stdout.contains("RESULT:"));
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_unknown_arg() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .arg("--unknown-flag")
        .output()
        .expect("failed to execute coredebug");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: unknown argument '--unknown-flag'"));
}
