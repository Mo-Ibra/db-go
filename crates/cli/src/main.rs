//! DBGo command-line entry point.

mod args;
mod commands;
mod output;
mod util;

use clap::Parser;

use args::Cli;

/// Parse CLI arguments, run the command, and exit non-zero on failure.
fn main() {
    let cli = Cli::parse();
    if let Err(error) = commands::run(cli) {
        eprintln!("dbgo: {error}");
        std::process::exit(1);
    }
}
