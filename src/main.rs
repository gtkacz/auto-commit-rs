mod cli;
mod config;
mod git;
mod interpolation;
mod prompt;
mod provider;

use anyhow::{Context, Result};
use colored::Colorize;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = cli::parse();

    match cli.command {
        Some(cli::Command::Config { global }) => {
            cli::interactive_config(global)?;
        }
        None => {
            let cfg = config::AppConfig::load()?;

            if cfg.api_key.is_empty() {
                anyhow::bail!(
                    "No API key configured. Run {} or set {}",
                    "cgen config".yellow(),
                    "ACR_API_KEY".yellow()
                );
            }

            let diff = git::get_staged_diff()
                .context("Failed to get staged diff")?;

            let system_prompt = prompt::build_system_prompt(&cfg);
            let message = provider::call_llm(&cfg, &system_prompt, &diff)
                .context("LLM API call failed")?;

            let final_msg = cfg.commit_template.replace("$msg", message.trim());

            git::run_commit(&final_msg, &cli.extra_args)
                .context("git commit failed")?;
        }
    }

    Ok(())
}
