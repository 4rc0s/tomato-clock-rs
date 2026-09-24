use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::cli::validate_minutes;
use crate::progress::{bar_width, format_countdown, render_bar, session_title};

/// Returned when the run is aborted. `main` maps this to exit code 130
/// (128 + SIGINT) so scripts can distinguish "aborted" from "done". The
/// signal handler fires for SIGINT, SIGTERM and SIGHUP.
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
    /// Whether ANSI/OSC escape sequences may be written (auto-detected
    /// from stdout being a TTY; `--quiet` still suppresses the bar).
    pub ansi: bool,
    pub interrupted: &'a AtomicBool,
}

/// Seconds remaining, saturating so scheduling stalls past the deadline
/// can't underflow (the old `minutes * 60 - elapsed` panicked on `u64`).
pub fn seconds_left(total_secs: u64, elapsed_secs: u64) -> u64 {
    total_secs.saturating_sub(elapsed_secs)
}

/// Whether the live progress bar / title / cursor hiding should render.
/// Requires both a TTY (no escape codes into pipes/logs) and not `--quiet`.
pub fn show_progress(quiet: bool, ansi: bool) -> bool {
    !quiet && ansi
}

/// Compute the next tick deadline, resyncing when we have fallen behind
/// (e.g. the laptop slept, or the process was stopped with Ctrl+Z). Without
/// the resync branch the loop would spin without sleeping until the stale
/// `next_tick` catches up.
pub fn next_deadline(next_tick: Duration, now: Duration) -> Duration {
    if now < next_tick {
        next_tick
    } else {
        now + Duration::from_secs(1)
    }
}

/// Time source for the countdown loop, abstracted so tests can run a full
/// session without waiting in real time. Times are offsets from an
/// arbitrary, clock-specific epoch.
pub trait Clock {
    fn now(&self) -> Duration;
    /// Sleep until `deadline`, returning `true` if interrupted.
    fn sleep_until(&self, deadline: Duration, interrupted: &AtomicBool) -> bool;
}

/// Production clock that keeps counting while the machine is suspended, so
/// a session ends at the real wall-clock time even if the laptop slept.
///
/// [`std::time::Instant`] can't be used: on Linux and macOS it stops during
/// suspend, which silently stretched sessions by the time spent asleep.
pub struct RealClock;

impl RealClock {
    /// Monotonic time including suspend: `CLOCK_BOOTTIME` on Linux,
    /// `CLOCK_MONOTONIC` on Apple platforms (which counts sleep there).
    #[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
    fn raw_now() -> Duration {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const CLOCK: libc::clockid_t = libc::CLOCK_BOOTTIME;
        #[cfg(target_vendor = "apple")]
        const CLOCK: libc::clockid_t = libc::CLOCK_MONOTONIC;

        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: `ts` is a valid, writable timespec and CLOCK is supported
        // on these targets, so clock_gettime only writes into `ts`.
        let rc = unsafe { libc::clock_gettime(CLOCK, &mut ts) };
        assert_eq!(rc, 0, "clock_gettime failed");
        Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }

    /// Elsewhere fall back to `Instant`, which already includes suspend on
    /// Windows (QueryPerformanceCounter).
    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    fn raw_now() -> Duration {
        use std::sync::OnceLock;
        use std::time::Instant;
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        EPOCH.get_or_init(Instant::now).elapsed()
    }
}

impl Clock for RealClock {
    fn now(&self) -> Duration {
        Self::raw_now()
    }

    /// Sleeps in short slices, re-reading the clock each time, so Ctrl+C is
    /// noticed within ~50ms and a resume from suspend is noticed right away
    /// (`thread::sleep` itself doesn't count time spent suspended).
    fn sleep_until(&self, deadline: Duration, interrupted: &AtomicBool) -> bool {
        loop {
            if interrupted.load(Ordering::Relaxed) {
                return true;
            }
            let now = self.now();
            if now >= deadline {
                return false;
            }
            std::thread::sleep((deadline - now).min(Duration::from_millis(50)));
        }
    }
}

/// Hides the cursor while the progress bar is live, restoring the cursor
/// and clearing the terminal title on drop (normal exit, interrupt, or
/// panic). No-op when `active` is false (i.e. non-TTY or `--quiet`).
struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn hide(active: bool) -> Self {
        if active {
            print!("\x1b[?25l");
            let _ = std::io::stdout().flush();
        }
        Self { active }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            // Show cursor again and clear the OSC 0 title we kept overwriting.
            print!("\x1b[?25h\x1b]0;\x07");
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
    run_with_clock(minutes, opts, &RealClock)
}

/// [`run`] against an arbitrary [`Clock`], so tests can complete a full
/// session instantly.
pub fn run_with_clock<C: Clock>(minutes: u64, opts: &RunOptions<'_>, clock: &C) -> anyhow::Result<()> {
    let minutes = validate_minutes(minutes).map_err(|e| anyhow::anyhow!(e))?;
    let total_secs = minutes * 60;
    let width = bar_width(minutes);
    let progress = show_progress(opts.quiet, opts.ansi);

    let _terminal = TerminalGuard::hide(progress);

    let start = clock.now();
    let mut next_tick = start + Duration::from_secs(1);

    loop {
        if opts.interrupted.load(Ordering::Relaxed) {
            if progress {
                println!();
            }
            return Err(Interrupted.into());
        }

        let elapsed_secs = clock.now().saturating_sub(start).as_secs();
        let left_secs = seconds_left(total_secs, elapsed_secs);
        let countdown = format_countdown(left_secs);

        if progress {
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
        // After a suspend or Ctrl+Z the deadline is stale, so resync instead
        // of busy-looping to catch up.
        let now = clock.now();
        let deadline = next_deadline(next_tick, now);
        if deadline == next_tick {
            if clock.sleep_until(deadline, opts.interrupted) {
                if progress {
                    println!();
                }
                return Err(Interrupted.into());
            }
            next_tick += Duration::from_secs(1);
        } else {
            next_tick = deadline;
        }
    }

    if progress {
        println!();
    }

    // Audible/visual bell so completion is noticed even without a
    // notification daemon (headless, muted D-Bus, etc.). Gated on ANSI
    // so redirected output stays clean.
    if opts.ansi {
        print!("\x07");
        let _ = std::io::stdout().flush();
    }

    println!("{}", opts.message);
    // Nested rather than a let chain: let chains need Rust 1.88, and the
    // crate's MSRV is 1.85.
    if !opts.no_notify {
        if let Err(e) = crate::notify::notify(opts.message) {
            eprintln!("warning: {e:#}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

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
    fn show_progress_requires_tty_and_not_quiet() {
        assert!(show_progress(false, true));
        assert!(!show_progress(false, false)); // piped output: no escapes
        assert!(!show_progress(true, true)); // --quiet overrides
        assert!(!show_progress(true, false));
    }

    #[test]
    fn resyncs_stale_deadline_instead_of_catching_up() {
        let stale = Duration::from_secs(5); // far behind
        let now = Duration::from_secs(120);
        // Should jump forward to now+1s, not return the stale tick.
        assert_eq!(next_deadline(stale, now), now + Duration::from_secs(1));
    }

    #[test]
    fn keeps_future_deadline() {
        let now = Duration::from_secs(10);
        let future = now + Duration::from_secs(1);
        assert_eq!(next_deadline(future, now), future);
    }

    #[test]
    fn real_clock_is_monotonic() {
        let a = RealClock.now();
        std::thread::sleep(Duration::from_millis(20));
        let b = RealClock.now();
        assert!(b >= a + Duration::from_millis(20), "{a:?} -> {b:?}");
    }

    #[test]
    fn sleep_notices_interrupt_quickly() {
        let flag = AtomicBool::new(false);
        let deadline = RealClock.now() + Duration::from_secs(30);
        // Simulate Ctrl+C arriving mid-sleep from another thread.
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                flag.store(true, Ordering::Relaxed);
            });
            let t0 = std::time::Instant::now();
            assert!(RealClock.sleep_until(deadline, &flag));
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
            ansi: false,
            interrupted: &flag,
        };
        let err = run(25, &opts).unwrap_err();
        assert!(err.downcast_ref::<Interrupted>().is_some());
    }

    /// Virtual clock: `now` advances only when `sleep_until` is called, so a
    /// whole session finishes without real waiting.
    struct FakeClock {
        now: Cell<Duration>,
        interrupt_after: Option<usize>,
        /// On the given sleep, overshoot the deadline by this much, as if
        /// the machine was suspended mid-sleep.
        suspend_at: Option<(usize, Duration)>,
        sleeps: Cell<usize>,
    }

    impl FakeClock {
        fn new() -> Self {
            Self {
                now: Cell::new(Duration::ZERO),
                interrupt_after: None,
                suspend_at: None,
                sleeps: Cell::new(0),
            }
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Duration {
            self.now.get()
        }

        fn sleep_until(&self, deadline: Duration, interrupted: &AtomicBool) -> bool {
            self.sleeps.set(self.sleeps.get() + 1);
            if self.interrupt_after == Some(self.sleeps.get()) {
                interrupted.store(true, Ordering::Relaxed);
            }
            if interrupted.load(Ordering::Relaxed) {
                return true;
            }
            let overshoot = match self.suspend_at {
                Some((n, d)) if n == self.sleeps.get() => d,
                _ => Duration::ZERO,
            };
            self.now.set(deadline + overshoot);
            false
        }
    }

    fn quiet_opts(interrupted: &AtomicBool) -> RunOptions<'_> {
        RunOptions {
            message: "done",
            label: "🍅 tomato",
            no_notify: true,
            quiet: true,
            ansi: false,
            interrupted,
        }
    }

    #[test]
    fn fake_clock_completes_full_countdown() {
        let clock = FakeClock::new();
        let flag = AtomicBool::new(false);
        run_with_clock(1, &quiet_opts(&flag), &clock).unwrap();
        assert_eq!(clock.sleeps.get(), 60);
    }

    #[test]
    fn fake_clock_interrupts_mid_run() {
        let mut clock = FakeClock::new();
        clock.interrupt_after = Some(3);
        let flag = AtomicBool::new(false);
        let err = run_with_clock(1, &quiet_opts(&flag), &clock).unwrap_err();
        assert!(err.downcast_ref::<Interrupted>().is_some());
        assert_eq!(clock.sleeps.get(), 3);
    }

    #[test]
    fn fake_clock_counts_suspended_time() {
        // Suspend for 50s during the 3rd one-second sleep: the session
        // should end at 60s of real time, not 60s of awake time.
        let mut clock = FakeClock::new();
        clock.suspend_at = Some((3, Duration::from_secs(50)));
        let flag = AtomicBool::new(false);
        run_with_clock(1, &quiet_opts(&flag), &clock).unwrap();
        assert_eq!(clock.now(), Duration::from_secs(60));
        // 3 sleeps to reach 53s, then a resync (no sleep), then 7 more.
        assert_eq!(clock.sleeps.get(), 10);
    }
}
