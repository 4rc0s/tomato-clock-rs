use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Context;

use crate::cli::{BREAK_MINUTES, Cli, Command, WORK_MINUTES};
use crate::timer::{self, RunOptions};

/// Install the SIGINT/SIGTERM/SIGHUP handler that flips `flag`.
///
/// Best-effort: if a handler is already installed (e.g. repeated calls from
/// tests) the error is ignored and default signal termination still applies.
pub fn install_signal_handler(flag: &Arc<AtomicBool>) {
    let flag = Arc::clone(flag);
    let _ = ctrlc::set_handler(move || {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });
}

/// Parse-independent entry point: dispatch `cli` to the chosen timer(s).
pub fn run(cli: Cli) -> anyhow::Result<()> {
    let interrupted = Arc::new(AtomicBool::new(false));
    install_signal_handler(&interrupted);

    let no_notify = cli.no_notify;
    let quiet = cli.quiet;
    let ansi = std::io::stdout().is_terminal();

    match cli.command {
        None => {
            run_cycle(WORK_MINUTES, BREAK_MINUTES, no_notify, quiet, ansi, &interrupted)?;
        }
        Some(Command::Work { minutes }) => {
            println!("🍅 tomato {minutes} minutes. Ctrl+C to exit");
            run_timer(
                minutes,
                "🍅 tomato",
                "It is time to take a break",
                no_notify,
                quiet,
                ansi,
                &interrupted,
            )?;
        }
        Some(Command::Break { minutes }) => {
            println!("🛀 break {minutes} minutes. Ctrl+C to exit");
            run_timer(
                minutes,
                "🛀 break",
                "It is time to work",
                no_notify,
                quiet,
                ansi,
                &interrupted,
            )?;
        }
        Some(Command::Cycle {
            work_minutes,
            break_minutes,
        }) => {
            run_cycle(work_minutes, break_minutes, no_notify, quiet, ansi, &interrupted)?;
        }
    }

    Ok(())
}

fn run_cycle(
    work_minutes: u64,
    break_minutes: u64,
    no_notify: bool,
    quiet: bool,
    ansi: bool,
    interrupted: &AtomicBool,
) -> anyhow::Result<()> {
    println!("🍅 tomato {work_minutes} minutes. Ctrl+C to exit");
    run_timer(
        work_minutes,
        "🍅 tomato",
        "It is time to take a break",
        no_notify,
        quiet,
        ansi,
        interrupted,
    )?;
    println!("🛀 break {break_minutes} minutes. Ctrl+C to exit");
    run_timer(
        break_minutes,
        "🛀 break",
        "It is time to work",
        no_notify,
        quiet,
        ansi,
        interrupted,
    )?;
    Ok(())
}

fn run_timer(
    minutes: u64,
    label: &str,
    message: &str,
    no_notify: bool,
    quiet: bool,
    ansi: bool,
    interrupted: &AtomicBool,
) -> anyhow::Result<()> {
    timer::run(
        minutes,
        &RunOptions {
            message,
            label,
            no_notify,
            quiet,
            ansi,
            interrupted,
        },
    )
    .with_context(|| format!("failed to run {minutes}-minute timer"))
}
