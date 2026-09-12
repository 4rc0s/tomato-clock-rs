use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("tomato").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("work").and(predicates::str::contains("break")));
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
