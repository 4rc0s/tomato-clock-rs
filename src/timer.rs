use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::anyhow;

use crate::cli::MAX_MINUTES;
use crate::progress::{bar_width, format_countdown, render_bar, session_title};

/// Returned when the user hits Ctrl+C. `main` maps this to exit code 130
/// (128 + SIGINT) so scripts can distinguish "aborted" from "done".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interrupted — timer stopped")
    }
}

impl std::error::Error for Interrupted {}

pub struct RunOptions<'a> {
    pub message: &'a str,
    /// Short label used for the terminal title, e.g. `"🍅 tomato"`.
    pub label: &'a str,
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

/// Compute the next tick deadline, resyncing when we have fallen behind
/// (e.g. laptop slept). Without the resync branch the loop would spin
/// without sleeping until the stale `next_tick` catches up.
pub fn next_deadline(next_tick: Instant, now: Instant) -> Instant {
    if now < next_tick {
        next_tick
    } else {
        now + Duration::from_secs(1)
    }
}

/// Sleep until `deadline` in short slices so Ctrl+C is noticed within
/// ~50ms instead of blocking a full second. Returns `true` if interrupted.
pub fn interruptible_sleep(deadline: Instant, interrupted: &AtomicBool) -> bool {
    while Instant::now() < deadline {
        if interrupted.load(Ordering::Relaxed) {
            return true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(remaining.min(Duration::from_millis(50)));
    }
    interrupted.load(Ordering::Relaxed)
}

/// Hides the cursor while the progress bar is live, restoring it on drop
/// (normal exit, interrupt, or panic). No-op when `active` is false
/// (i.e. `--quiet`).
struct CursorGuard {
    active: bool,
}

impl CursorGuard {
    fn hide(active: bool) -> Self {
        if active {
            print!("\x1b[?25l");
            let _ = std::io::stdout().flush();
        }
        Self { active }
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        if self.active {
            print!("\x1b[?25h");
            let _ = std::io::stdout().flush();
        }
    }
}

fn set_terminal_title(title: &str) {
    // OSC 0: set window/tab title. Harmless on terminals that ignore it.
    print!("\x1b]0;{title}\x07");
}

/// Run a single countdown timer with a tick-accurate loop.
pub fn run(minutes: u64, opts: &RunOptions<'_>) -> anyhow::Result<()> {
    let minutes = validate_minutes(minutes)?;
    let total_secs = minutes * 60;
    let width = bar_width(minutes);

    let _cursor = CursorGuard::hide(!opts.quiet);

    let start = Instant::now();
    let mut next_tick = start + Duration::from_secs(1);

    loop {
        if opts.interrupted.load(Ordering::Relaxed) {
            if !opts.quiet {
                println!();
            }
            return Err(Interrupted.into());
        }

        let elapsed_secs = start.elapsed().as_secs();
        let left_secs = seconds_left(total_secs, elapsed_secs);
        let countdown = format_countdown(left_secs);

        if !opts.quiet {
            // Pad with spaces to clear leftover chars when the line shrinks
            // (e.g. "10:00" -> "9:59").
            let line = render_bar(elapsed_secs, total_secs, width, &countdown);
            let title = session_title(opts.label, &countdown);
            set_terminal_title(&title);
            print!("\r{line}   ");
            std::io::stdout().flush()?;
        }

        if left_secs == 0 {
            break;
        }

        // Sleep until the next 1s tick to avoid drift from loop overhead.
        // On wake-from-sleep the deadline is stale, so resync instead of
        // busy-looping to catch up.
        let now = Instant::now();
        if now < next_tick {
            if interruptible_sleep(next_tick, opts.interrupted) {
                if !opts.quiet {
                    println!();
                }
                return Err(Interrupted.into());
            }
            next_tick += Duration::from_secs(1);
        } else {
            next_tick = next_deadline(next_tick, now);
        }
    }

    if !opts.quiet {
        println!();
    }

    // Audible/visual bell so completion is noticed even without a
    // notification daemon (headless, muted D-Bus, etc.).
    print!("\x07");
    let _ = std::io::stdout().flush();

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

    #[test]
    fn resyncs_stale_deadline_instead_of_catching_up() {
        let start = Instant::now();
        let stale = start; // far behind
        let now = start + Duration::from_secs(120);
        let next = next_deadline(stale, now);
        // Should jump forward to ~now+1s, not return the stale tick.
        assert!(next > now);
        assert!(next <= now + Duration::from_secs(1) + Duration::from_millis(10));
    }

    #[test]
    fn keeps_future_deadline() {
        let now = Instant::now();
        let future = now + Duration::from_secs(1);
        assert_eq!(next_deadline(future, now), future);
    }

    #[test]
    fn sleep_notices_interrupt_quickly() {
        let flag = AtomicBool::new(false);
        let deadline = Instant::now() + Duration::from_secs(30);
        // Simulate Ctrl+C arriving mid-sleep from another thread.
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                flag.store(true, Ordering::Relaxed);
            });
            let t0 = Instant::now();
            assert!(interruptible_sleep(deadline, &flag));
            // Must return promptly, not after the full 30s.
            assert!(t0.elapsed() < Duration::from_secs(5));
        });
    }

    #[test]
    fn run_returns_interrupted_error() {
        let flag = AtomicBool::new(true); // already interrupted
        let opts = RunOptions {
            message: "done",
            label: "🍅 tomato",
            no_notify: true,
            quiet: true,
            interrupted: &flag,
        };
        let err = run(25, &opts).unwrap_err();
        assert!(err.downcast_ref::<Interrupted>().is_some());
    }
}
