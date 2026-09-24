//! 🍅 Tomato Clock — a straightforward command-line Pomodoro timer.
//!
//! The binary is a thin wrapper around [`app::run`]; everything else lives
//! here so it can be unit- and integration-tested.

/// `print!` that ignores write errors and flushes immediately.
///
/// Output is best-effort: once the terminal is closed (SIGHUP) or a pipe
/// reader exits, `print!` would panic on the failed write instead of letting
/// the timer exit through its normal signal path.
macro_rules! out {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, $($arg)*);
        let _ = stdout.flush();
    }};
}

/// Best-effort `println!`; see [`out!`].
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

pub mod app;
pub mod cli;
pub mod notify;
pub mod progress;
pub mod timer;
