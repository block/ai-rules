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

/// Redirect stdout to /dev/null so all `println!` calls become no-ops.
/// Stderr remains unaffected.
#[cfg(unix)]
pub fn suppress_stdout() {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;
    if let Ok(devnull) = File::open("/dev/null") {
        unsafe {
            libc::dup2(devnull.as_raw_fd(), libc::STDOUT_FILENO);
        }
    }
}
