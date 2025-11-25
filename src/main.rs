mod agents;
mod cli;
mod commands;
mod config;
mod constants;
mod models;
mod operations;
mod utils;

use cli::run_cli;

fn main() {
    if let Err(e) = run_cli() {
        eprintln!("❌ Error: {e:?}");
        std::process::exit(1);
    }
}
