mod cli;
mod config;
mod db;
mod notify;
mod platform;
mod project;
mod task;
mod tui;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(err) = cli::run_cli(cli) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
