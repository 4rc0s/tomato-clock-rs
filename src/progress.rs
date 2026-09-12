//! Pure, testable helpers for the progress bar and countdown.

/// Fraction of the timer that has elapsed, guarded against `total == 0`.
pub fn fraction(curr_secs: u64, total_secs: u64) -> f64 {
    if total_secs == 0 {
        return 1.0;
    }
    (curr_secs.min(total_secs) as f64) / (total_secs as f64)
}

/// Number of filled slots for a bar of `width` slots.
pub fn filled_slots(curr_secs: u64, total_secs: u64, width: u64) -> u64 {
    (fraction(curr_secs, total_secs) * width as f64).round() as u64
}

/// Bar width: one slot per minute, capped so long sessions stay readable.
pub fn bar_width(minutes: u64) -> u64 {
    minutes.clamp(1, 25)
}

/// `M:SS ⏰` countdown string for seconds remaining.
pub fn format_countdown(left_seconds: u64) -> String {
    format!("{}:{:0>2} ⏰", left_seconds / 60, left_seconds % 60)
}

/// Plain-text terminal/tab title for a session, e.g. `"23:04 ⏰ - 🍅 tomato"`.
/// The caller wraps this in an OSC escape (`\x1b]0;{title}\x07`).
pub fn session_title(label: &str, countdown: &str) -> String {
    format!("{countdown} - {label}")
}

/// Render one full progress line (without `\r` or newline).
/// Uses `🍅` for elapsed slots and `--` for remaining slots.
pub fn render_bar(curr_secs: u64, total_secs: u64, width: u64, extra: &str) -> String {
    let filled = filled_slots(curr_secs, total_secs, width).min(width);
    let mut out = String::new();
    for _ in 0..filled {
        out.push('🍅');
    }
    for _ in 0..width.saturating_sub(filled) {
        out.push_str("--");
    }
    out.push_str(&format!(" [{:.0}%]", fraction(curr_secs, total_secs) * 100.0));
    out.push(' ');
    out.push_str(extra);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_time_is_half_bar() {
        assert_eq!(filled_slots(30, 60, 10), 5);
        assert!((fraction(30, 60) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn clamps_beyond_total() {
        assert_eq!(filled_slots(999, 60, 10), 10);
        assert!((fraction(999, 60) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_total_does_not_panic() {
        assert_eq!(fraction(0, 0), 1.0);
        assert_eq!(filled_slots(0, 0, 10), 10);
    }

    #[test]
    fn countdown_formatting() {
        assert_eq!(format_countdown(1500), "25:00 ⏰");
        assert_eq!(format_countdown(599), "9:59 ⏰");
        assert_eq!(format_countdown(0), "0:00 ⏰");
    }

    #[test]
    fn title_formatting() {
        assert_eq!(session_title("🍅 tomato", "23:04 ⏰"), "23:04 ⏰ - 🍅 tomato");
        assert_eq!(session_title("🛀 break", "0:00 ⏰"), "0:00 ⏰ - 🛀 break");
    }

    #[test]
    fn width_is_capped() {
        assert_eq!(bar_width(1), 1);
        assert_eq!(bar_width(25), 25);
        assert_eq!(bar_width(50), 25);
        assert_eq!(bar_width(0), 1);
    }

    #[test]
    fn render_bar_shape() {
        let line = render_bar(0, 60, 4, "1:00 ⏰");
        assert_eq!(line, "-------- [0%] 1:00 ⏰");
        let line = render_bar(60, 60, 4, "0:00 ⏰");
        assert_eq!(line, "🍅🍅🍅🍅 [100%] 0:00 ⏰");
    }
}
