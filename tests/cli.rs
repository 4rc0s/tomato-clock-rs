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
