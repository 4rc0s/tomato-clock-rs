use std::io::Write;

use clap::Parser;

use tomato::cli::Cli;
use tomato::timer::Interrupted;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = tomato::app::run(cli) {
        if let Some(interrupted) = e.downcast_ref::<Interrupted>() {
            // Best-effort: after SIGHUP stderr may be gone, and eprintln!
            // would panic instead of exiting with the signal's code.
            let _ = writeln!(std::io::stderr(), "{interrupted}");
            std::process::exit(interrupted.exit_code());
        }
        let _ = writeln!(std::io::stderr(), "Error: {e:#}");
        std::process::exit(1);
    }
}
