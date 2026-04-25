// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use std::process::Command;

#[test]
fn test_cli_help() {
    let bin = env!("CARGO_BIN_EXE_corepolicy");

    for arg in &["help", "--help", "-h"] {
        let output = Command::new(bin)
            .arg(arg)
            .output()
            .expect("failed to execute corepolicy");

        assert!(output.status.success(), "help flag '{}' should exit 0", arg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("CoreShift Policy"),
            "stdout should contain branding"
        );
        assert!(
            stdout.contains("Usage: corepolicy"),
            "stdout should contain usage"
        );
    }
}

#[test]
fn test_cli_placeholder_no_args() {
    let bin = env!("CARGO_BIN_EXE_corepolicy");
    let output = Command::new(bin)
        .output()
        .expect("failed to execute corepolicy");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CoreShift Policy Daemon"),
        "stdout should contain branding"
    );
    assert!(
        stdout.contains("not yet implemented"),
        "stdout should mention it is not implemented"
    );
}

#[test]
fn test_cli_preload_not_implemented() {
    let bin = env!("CARGO_BIN_EXE_corepolicy");
    let output = Command::new(bin)
        .arg("-p")
        .output()
        .expect("failed to execute corepolicy");

    assert!(!output.status.success(), "-p should currently fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("preload runtime is not implemented yet"));
}

#[test]
fn test_cli_status_not_implemented() {
    let bin = env!("CARGO_BIN_EXE_corepolicy");
    let output = Command::new(bin)
        .arg("status")
        .output()
        .expect("failed to execute corepolicy");

    assert!(!output.status.success(), "status should currently fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("status reader is not implemented yet"));
}

#[test]
fn test_cli_unknown_arg() {
    let bin = env!("CARGO_BIN_EXE_corepolicy");
    let output = Command::new(bin)
        .arg("--unknown-feature")
        .output()
        .expect("failed to execute corepolicy");

    assert_eq!(output.status.code(), Some(2), "unknown arg should exit 2");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument '--unknown-feature'"));
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_help() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .arg("help")
        .output()
        .expect("failed to execute coredebug");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CoreShift Policy Diagnostics"));
}

#[test]
#[cfg(feature = "debug-cli")]
fn test_coredebug_test_low_level_placeholder() {
    let bin = env!("CARGO_BIN_EXE_coredebug");
    let output = std::process::Command::new(bin)
        .args(&["test", "low_level"])
        .output()
        .expect("failed to execute coredebug");

    assert!(!output.status.success()); // Currently planned but not implemented returns 1
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Running low_level diagnostic probes"));
    assert!(stdout.contains("Not implemented yet"));
}
