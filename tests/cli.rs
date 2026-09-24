use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.arg("--help").assert().success().stdout(
        predicates::str::contains("work")
            .and(predicates::str::contains("break"))
            .and(predicates::str::contains("cycle")),
    );
}

#[test]
fn rejects_zero_minutes() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.args(["work", "0"]).assert().failure();
}

#[test]
fn rejects_non_numeric() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.args(["break", "abc"]).assert().failure();
}

#[test]
fn rejects_legacy_flag() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.arg("-t").assert().failure();
}

#[test]
fn rejects_zero_cycle_minutes() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.args(["cycle", "0", "5"]).assert().failure();
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.args(["cycle", "25", "0"]).assert().failure();
}

#[test]
fn work_alias_parses() {
    // `w` is a visible alias for `work`; --help must exit 0 so the
    // subcommand table exists. Full timer runs are covered by unit tests
    // (a real 1-minute run would be too slow for CI).
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.args(["w", "--help"]).assert().success();
}

#[test]
fn version_flag_succeeds() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn cycle_aliases_parse() {
    for alias in ["c", "full", "pomodoro"] {
        let mut cmd = Command::cargo_bin("tomato").unwrap();
        cmd.args([alias, "--help"]).assert().success();
    }
}

#[test]
fn non_tty_output_has_no_escape_codes() {
    // stdout is a pipe here, so the bar/title/bell must be suppressed.
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    let output = cmd
        .args(["work", "1"])
        .timeout(std::time::Duration::from_millis(300))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains('\x1b'), "unexpected escape codes: {stdout:?}");
    assert!(!stdout.contains('\x07'), "unexpected bell: {stdout:?}");
}

#[cfg(unix)]
#[test]
fn sigterm_exits_gracefully() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command as StdCommand, Stdio};

    let mut child = StdCommand::new(env!("CARGO_BIN_EXE_tomato"))
        .args(["work", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // The header is printed after the signal handler is installed, so once
    // it arrives SIGTERM is guaranteed to hit the handler rather than kill
    // the process outright (a fixed sleep raced on loaded CI runners).
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut header = String::new();
    stdout.read_line(&mut header).unwrap();
    assert!(header.contains("tomato 1 minutes"), "unexpected header: {header:?}");

    let kill = StdCommand::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(kill.success());

    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(143), "SIGTERM should exit 128+15 after cleanup");
}

#[cfg(unix)]
#[test]
fn sighup_with_closed_output_exits_cleanly() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command as StdCommand, Stdio};

    // Simulates the terminal window closing: stdout/stderr are gone (writes
    // fail) and SIGHUP arrives. Writing the interrupt message used to panic
    // (exit 101) instead of exiting via the signal path.
    let mut child = StdCommand::new(env!("CARGO_BIN_EXE_tomato"))
        .args(["work", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut header = String::new();
    stdout.read_line(&mut header).unwrap();
    drop(stdout);
    drop(child.stderr.take());

    let kill = StdCommand::new("kill")
        .args(["-HUP", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(kill.success());

    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(129), "SIGHUP should exit 128+1 via the signal path");
}
