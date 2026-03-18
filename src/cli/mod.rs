mod args;
mod config_resolution;

#[cfg(test)]
mod tests;

pub use args::*;

use crate::commands::{run_clean, run_generate, run_init, run_list_agents, run_status};
use crate::config;
use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

const SUMMARY: &str = "Manage AI context rules across different AI coding agents";

fn home_dir() -> anyhow::Result<PathBuf> {
    let path = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME environment variable is not set")?;
    anyhow::ensure!(!path.as_os_str().is_empty(), "HOME environment variable is empty");
    Ok(path)
}

pub fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.summary {
        println!("{SUMMARY}");
        return Ok(());
    }

    let current_dir = std::env::current_dir()?;

    let config = config::load_config(&current_dir)?;

    let use_claude_skills = config
        .as_ref()
        .and_then(|c| c.use_claude_skills)
        .unwrap_or(false);

    match cli.command {
        Some(Commands::Init(init_args)) => run_init(&current_dir, init_args),
        Some(Commands::Generate(args)) => {
            let final_args = args.with_config(config.as_ref());
            let generate_dir = if final_args.global {
                home_dir()?
            } else {
                current_dir.clone()
            };
            run_generate(&generate_dir, final_args, use_claude_skills)
        }
        Some(Commands::Status(args)) => {
            let final_args = args.with_config(config.as_ref());
            run_status(&current_dir, final_args, use_claude_skills)
        }
        Some(Commands::Clean(args)) => {
            let nested_depth = args.nested_depth_args.with_config(config.as_ref());
            run_clean(&current_dir, nested_depth, use_claude_skills)
        }
        Some(Commands::ListAgents) => run_list_agents(use_claude_skills),
        None => {
            // If no command is provided and --summary is not used, show help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            Ok(())
        }
    }
}
