use clap::{Parser, Subcommand};

pub const WORK_MINUTES: u64 = 25;
pub const BREAK_MINUTES: u64 = 5;
pub const MAX_MINUTES: u64 = 1440;

/// Range check shared by clap parsing and the timer.
pub fn validate_minutes(minutes: u64) -> Result<u64, String> {
    if (1..=MAX_MINUTES).contains(&minutes) {
        Ok(minutes)
    } else {
        Err(format!("minutes must be in range 1..={MAX_MINUTES}"))
    }
}

fn parse_minutes(s: &str) -> Result<u64, String> {
    let minutes: u64 = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid number of minutes"))?;
    validate_minutes(minutes)
}

#[derive(Parser, Debug)]
#[command(
    name = "tomato",
    version,
    about = "🍅 Tomato Clock — a straightforward command-line Pomodoro timer"
)]
pub struct Cli {
    /// Which timer to run. Omit to run a work session followed by a break.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Skip desktop notifications (still prints the message).
    #[arg(global = true, long = "no-notify")]
    pub no_notify: bool,

    /// Suppress the progress bar output.
    #[arg(global = true, short, long)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Start a work session.
    #[command(visible_aliases = ["w", "focus", "tomato"])]
    Work {
        /// Length of the work session in minutes.
        #[arg(default_value_t = WORK_MINUTES, value_parser = parse_minutes)]
        minutes: u64,
    },
    /// Take a break.
    #[command(visible_aliases = ["b", "rest"])]
    Break {
        /// Length of the break in minutes.
        #[arg(default_value_t = BREAK_MINUTES, value_parser = parse_minutes)]
        minutes: u64,
    },
    /// Run a work session followed by a break (like bare `tomato`,
    /// but with custom lengths).
    #[command(visible_aliases = ["c", "full", "pomodoro"])]
    Cycle {
        /// Length of the work session in minutes.
        #[arg(default_value_t = WORK_MINUTES, value_parser = parse_minutes)]
        work_minutes: u64,
        /// Length of the break in minutes.
        #[arg(default_value_t = BREAK_MINUTES, value_parser = parse_minutes)]
        break_minutes: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn defaults_to_work_minutes() {
        let cli = Cli::try_parse_from(["tomato", "work"]).unwrap();
        assert_eq!(cli.command, Some(Command::Work { minutes: WORK_MINUTES }));
    }

    #[test]
    fn rejects_zero_and_out_of_range() {
        assert!(Cli::try_parse_from(["tomato", "work", "0"]).is_err());
        assert!(Cli::try_parse_from(["tomato", "break", "0"]).is_err());
        assert!(Cli::try_parse_from(["tomato", "work", "99999"]).is_err());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(Cli::try_parse_from(["tomato", "work", "abc"]).is_err());
    }

    #[test]
    fn validates_minutes_range() {
        assert!(validate_minutes(0).is_err());
        assert!(validate_minutes(1).is_ok());
        assert!(validate_minutes(MAX_MINUTES).is_ok());
        assert!(validate_minutes(MAX_MINUTES + 1).is_err());
    }

    #[test]
    fn parses_global_flags() {
        let cli = Cli::try_parse_from(["tomato", "--no-notify", "-q", "break", "3"]).unwrap();
        assert!(cli.no_notify);
        assert!(cli.quiet);
        assert_eq!(cli.command, Some(Command::Break { minutes: 3 }));
    }

    #[test]
    fn parses_aliases() {
        let cli = Cli::try_parse_from(["tomato", "w", "10"]).unwrap();
        assert_eq!(cli.command, Some(Command::Work { minutes: 10 }));
        let cli = Cli::try_parse_from(["tomato", "b"]).unwrap();
        assert_eq!(cli.command, Some(Command::Break { minutes: BREAK_MINUTES }));
    }

    #[test]
    fn parses_cycle_with_defaults() {
        let cli = Cli::try_parse_from(["tomato", "cycle"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Cycle {
                work_minutes: WORK_MINUTES,
                break_minutes: BREAK_MINUTES,
            })
        );
        let cli = Cli::try_parse_from(["tomato", "cycle", "50", "10"]).unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Cycle {
                work_minutes: 50,
                break_minutes: 10,
            })
        );
    }
}
