mod cli;
mod config;
mod git;
mod interpolation;
mod prompt;
mod provider;

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Select, Text};

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

            let mut message = provider::call_llm(&cfg, &system_prompt, &diff)
                .context("LLM API call failed")?;

            let final_msg = if cfg.review_commit {
                loop {
                    let candidate = cfg.commit_template.replace("$msg", message.trim());

                    println!("\n{}", "Commit message:".green().bold());
                    println!("  {}\n", candidate);

                    match review_message()? {
                        ReviewAction::Accept => break candidate,
                        ReviewAction::Regenerate => {
                            message = provider::call_llm(&cfg, &system_prompt, &diff)
                                .context("LLM API call failed")?;
                        }
                        ReviewAction::Edit => {
                            let edited = Text::new("Edit commit message:")
                                .with_default(&candidate)
                                .prompt()?;
                            break edited;
                        }
                        ReviewAction::Cancel => {
                            println!("{}", "Commit cancelled.".dimmed());
                            return Ok(());
                        }
                    }
                }
            } else {
                let final_msg = cfg.commit_template.replace("$msg", message.trim());
                println!("\n{} {}", "Commit message:".green().bold(), final_msg);
                final_msg
            };

            git::run_commit(&final_msg, &cli.extra_args)
                .context("git commit failed")?;
        }
    }

    Ok(())
}

enum ReviewAction {
    Accept,
    Regenerate,
    Edit,
    Cancel,
}

fn review_message() -> Result<ReviewAction> {
    let choices = vec![
        "Accept",
        "Regenerate",
        "Edit",
        "Cancel",
    ];

    let answer = Select::new("", choices)
        .without_help_message()
        .prompt();

    match answer {
        Ok("Accept") => Ok(ReviewAction::Accept),
        Ok("Regenerate") => Ok(ReviewAction::Regenerate),
        Ok("Edit") => Ok(ReviewAction::Edit),
        _ => Ok(ReviewAction::Cancel),
    }
}
