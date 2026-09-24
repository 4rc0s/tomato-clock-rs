# 🍅 Tomato Clock (Rust)

[![Test](https://github.com/4rc0s/tomato-clock-rs/actions/workflows/test.yml/badge.svg)](https://github.com/4rc0s/tomato-clock-rs/actions/workflows/test.yml)
[![Release](https://github.com/4rc0s/tomato-clock-rs/actions/workflows/release.yml/badge.svg)](https://github.com/4rc0s/tomato-clock-rs/actions/workflows/release.yml)

Tomato Clock is a straightforward command-line Pomodoro application.

> Forked from [coolcode/tomato-clock-rs](https://github.com/coolcode/tomato-clock-rs).
> v0.2.0 is a breaking rewrite: `clap`-based subcommands, fixed timer
> underflow, input validation, graceful `Ctrl+C`, and tests.
> The old `-t` / `-b` flags are gone — see usage below.

- [Pomodoro Technique](https://en.wikipedia.org/wiki/Pomodoro_Technique)
- [番茄工作法](https://zh.wikipedia.org/zh-cn/%E7%95%AA%E8%8C%84%E5%B7%A5%E4%BD%9C%E6%B3%95)
- [Tomato Clock (Python)](https://github.com/coolcode/tomato-clock)

## Installation

- Install via source code:

```sh
git clone https://github.com/4rc0s/tomato-clock-rs.git
cd tomato-clock-rs
cargo build --release
./target/release/tomato --help
```

## How to use

```sh
tomato                # 25 min work + 5 min break
tomato work           # 25 min work session
tomato work 50        # 50 min work session (1-1440)
tomato break          # 5 min break
tomato break 15       # 15 min break
tomato cycle 50 10    # 50 min work + 10 min break
tomato --help         # full help
tomato --version      # version

tomato --no-notify work 25   # skip desktop notification
tomato --quiet break 5       # no progress bar

# Short aliases: w/work, b/break, c/cycle (also full/pomodoro)
# Ctrl+C, SIGTERM and SIGHUP abort with exit code 130 and restore the terminal.
```

## Terminal Output

```sh
🍅 tomato 25 minutes. Ctrl+C to exit
🍅🍅---------------------------------------------- [8%] 23:00 ⏰
```

The live bar (and terminal title / bell) is only drawn when stdout is a
terminal; `--quiet` suppresses the bar and title too, but a completion bell
still rings on a terminal.

## Desktop Notification

[notify-rust](https://github.com/hoodie/notify-rust)

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
