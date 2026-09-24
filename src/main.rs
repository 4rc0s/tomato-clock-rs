use clap::Parser;

use tomato::cli::Cli;
use tomato::timer::Interrupted;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = tomato::app::run(cli) {
        if let Some(interrupted) = e.downcast_ref::<Interrupted>() {
            eprintln!("{interrupted}");
            std::process::exit(130);
        }
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
