use std::io::IsTerminal;
use std::sync::Arc;

use anyhow::Context;

use crate::cli::{BREAK_MINUTES, Cli, Command, WORK_MINUTES};
use crate::timer::{self, InterruptFlag, RunOptions};

/// Install handlers for SIGINT, SIGTERM and SIGHUP that store the signal's
/// number in `flag`, so the exit code can say which one stopped the timer.
///
/// Best-effort: if registration fails, the error is ignored and default
/// signal termination still applies.
#[cfg(unix)]
pub fn install_signal_handler(flag: &Arc<InterruptFlag>) {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        let _ = signal_hook::flag::register_usize(signal, Arc::clone(flag), signal as usize);
    }
}

/// Windows console events carry no signal number; all of them (Ctrl+C,
/// Ctrl+Break, closing the console) are reported as SIGINT.
#[cfg(not(unix))]
pub fn install_signal_handler(flag: &Arc<InterruptFlag>) {
    let flag = Arc::clone(flag);
    let _ = ctrlc::set_handler(move || {
        flag.store(timer::SIGINT as usize, std::sync::atomic::Ordering::Relaxed);
    });
}

/// Parse-independent entry point: dispatch `cli` to the chosen timer(s).
pub fn run(cli: Cli) -> anyhow::Result<()> {
    let interrupted = Arc::new(InterruptFlag::new(0));
    install_signal_handler(&interrupted);

    let no_notify = cli.no_notify;
    let quiet = cli.quiet;
    let ansi = std::io::stdout().is_terminal();

    match cli.command {
        None => {
            run_cycle(WORK_MINUTES, BREAK_MINUTES, no_notify, quiet, ansi, &interrupted)?;
        }
        Some(Command::Work { minutes }) => {
            outln!("🍅 tomato {minutes} minutes. Ctrl+C to exit");
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
            outln!("🛀 break {minutes} minutes. Ctrl+C to exit");
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
    interrupted: &InterruptFlag,
) -> anyhow::Result<()> {
    outln!("🍅 tomato {work_minutes} minutes. Ctrl+C to exit");
    run_timer(
        work_minutes,
        "🍅 tomato",
        "It is time to take a break",
        no_notify,
        quiet,
        ansi,
        interrupted,
    )?;
    outln!("🛀 break {break_minutes} minutes. Ctrl+C to exit");
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
    interrupted: &InterruptFlag,
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
