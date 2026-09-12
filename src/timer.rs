use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::anyhow;

use crate::cli::MAX_MINUTES;
use crate::progress::{bar_width, format_countdown, render_bar};

pub struct RunOptions<'a> {
    pub message: &'a str,
    pub no_notify: bool,
    pub quiet: bool,
    pub interrupted: &'a AtomicBool,
}

/// Seconds remaining, saturating so scheduling stalls past the deadline
/// can't underflow (the old `minutes * 60 - elapsed` panicked on `u64`).
pub fn seconds_left(total_secs: u64, elapsed_secs: u64) -> u64 {
    total_secs.saturating_sub(elapsed_secs)
}

pub fn validate_minutes(minutes: u64) -> anyhow::Result<u64> {
    if (1..=MAX_MINUTES).contains(&minutes) {
        Ok(minutes)
    } else {
        Err(anyhow!("minutes must be in range 1..={MAX_MINUTES}"))
    }
}

/// Run a single countdown timer with a tick-accurate loop.
pub fn run(minutes: u64, opts: &RunOptions<'_>) -> anyhow::Result<()> {
    let minutes = validate_minutes(minutes)?;
    let total_secs = minutes * 60;
    let width = bar_width(minutes);

    let start = Instant::now();
    let mut next_tick = start + Duration::from_secs(1);

    loop {
        if opts.interrupted.load(Ordering::Relaxed) {
            if !opts.quiet {
                println!();
            }
            eprintln!("interrupted — timer stopped");
            return Ok(());
        }

        let elapsed_secs = start.elapsed().as_secs();
        let left_secs = seconds_left(total_secs, elapsed_secs);
        let countdown = format_countdown(left_secs);

        if !opts.quiet {
            // Pad with spaces to clear leftover chars when the line shrinks
            // (e.g. "10:00" -> "9:59").
            let line = render_bar(elapsed_secs, total_secs, width, &countdown);
            print!("\r{line}   ");
            std::io::stdout().flush()?;
        }

        if left_secs == 0 {
            break;
        }

        // Sleep until the next 1s tick to avoid drift from loop overhead.
        let now = Instant::now();
        if now < next_tick {
            std::thread::sleep(next_tick - now);
        }
        next_tick += Duration::from_secs(1);
    }

    if !opts.quiet {
        println!();
    }

    println!("{}", opts.message);
    if !opts.no_notify
        && let Err(e) = crate::notify::notify(opts.message)
    {
        eprintln!("warning: {e:#}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturates_past_deadline() {
        assert_eq!(seconds_left(60, 0), 60);
        assert_eq!(seconds_left(60, 60), 0);
        // Old code panicked here on u64 underflow.
        assert_eq!(seconds_left(60, 61), 0);
        assert_eq!(seconds_left(60, 9999), 0);
    }

    #[test]
    fn rejects_invalid_minutes() {
        assert!(validate_minutes(0).is_err());
        assert!(validate_minutes(1).is_ok());
        assert!(validate_minutes(MAX_MINUTES).is_ok());
        assert!(validate_minutes(MAX_MINUTES + 1).is_err());
    }
}
