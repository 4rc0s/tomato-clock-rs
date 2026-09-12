mod cli;
mod notify;
mod progress;
mod timer;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Context;
use clap::Parser;

use cli::{BREAK_MINUTES, Cli, Command, WORK_MINUTES};
use timer::RunOptions;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let interrupted = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&interrupted);
    // Only a warning if the handler can't be installed (e.g. in tests);
    // default SIGINT termination still applies.
    let _ = ctrlc::set_handler(move || {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let no_notify = cli.no_notify;
    let quiet = cli.quiet;

    match cli.command {
        None => {
            println!("🍅 tomato {WORK_MINUTES} minutes. Ctrl+C to exit");
            run_timer(
                WORK_MINUTES,
                "It is time to take a break",
                no_notify,
                quiet,
                &interrupted,
            )?;
            if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }
            println!("🛀 break {BREAK_MINUTES} minutes. Ctrl+C to exit");
            run_timer(BREAK_MINUTES, "It is time to work", no_notify, quiet, &interrupted)?;
        }
        Some(Command::Work { minutes }) => {
            println!("🍅 tomato {minutes} minutes. Ctrl+C to exit");
            run_timer(minutes, "It is time to take a break", no_notify, quiet, &interrupted)?;
        }
        Some(Command::Break { minutes }) => {
            println!("🛀 break {minutes} minutes. Ctrl+C to exit");
            run_timer(minutes, "It is time to work", no_notify, quiet, &interrupted)?;
        }
    }

    Ok(())
}

fn run_timer(
    minutes: u64,
    message: &str,
    no_notify: bool,
    quiet: bool,
    interrupted: &AtomicBool,
) -> anyhow::Result<()> {
    timer::run(
        minutes,
        &RunOptions {
            message,
            no_notify,
            quiet,
            interrupted,
        },
    )
    .with_context(|| format!("failed to run {minutes}-minute timer"))
}
