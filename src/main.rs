mod cli;
mod config;
mod git;
mod interpolation;
mod prompt;
mod provider;

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, Select, Text};

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
        Some(cli::Command::Undo) => {
            let cfg = config::AppConfig::load()?;
            run_undo(&cfg)?;
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

            let staged_files = git::list_staged_files().context("Failed to list staged files")?;
            print_staged_files(&staged_files);

            if cfg.warn_staged_files_enabled && staged_files.len() > cfg.warn_staged_files_threshold
            {
                let prompt = format!(
                    "You have {} staged files (threshold {}). Continue with commit generation?",
                    staged_files.len(),
                    cfg.warn_staged_files_threshold
                );
                let should_continue = Confirm::new(&prompt)
                    .with_default(false)
                    .prompt()
                    .unwrap_or(false);
                if !should_continue {
                    println!("{}", "Commit cancelled.".dimmed());
                    return Ok(());
                }
            }

            let diff = git::get_staged_diff().context("Failed to get staged diff")?;

            let system_prompt = prompt::build_system_prompt(&cfg);

            let mut message =
                provider::call_llm(&cfg, &system_prompt, &diff).context("LLM API call failed")?;

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

            if cli.dry_run {
                println!(
                    "\n{}",
                    "Dry run enabled. Commit not created.".yellow().bold()
                );
                return Ok(());
            }

            git::run_commit(&final_msg, &cli.extra_args, cfg.suppress_tool_output)
                .context("git commit failed")?;

            handle_post_commit_push(&cfg)?;
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
    let choices = vec!["Accept", "Regenerate", "Edit", "Cancel"];

    let answer = Select::new("", choices).without_help_message().prompt();

    match answer {
        Ok("Accept") => Ok(ReviewAction::Accept),
        Ok("Regenerate") => Ok(ReviewAction::Regenerate),
        Ok("Edit") => Ok(ReviewAction::Edit),
        _ => Ok(ReviewAction::Cancel),
    }
}

fn print_staged_files(staged_files: &[String]) {
    println!(
        "\n{} {}",
        "Staged files:".green().bold(),
        staged_files.len()
    );
    if staged_files.is_empty() {
        println!("  {}", "(none)".dimmed());
        return;
    }

    for file in staged_files {
        println!("  - {}", file);
    }
}

fn handle_post_commit_push(cfg: &config::AppConfig) -> Result<()> {
    match cfg.post_commit_push.as_str() {
        "never" => {}
        "always" => {
            git::run_push(cfg.suppress_tool_output).context("git push failed")?;
        }
        _ => {
            let should_push = Confirm::new("Commit created. Push now?")
                .with_default(true)
                .prompt()
                .unwrap_or(false);
            if should_push {
                git::run_push(cfg.suppress_tool_output).context("git push failed")?;
            }
        }
    }
    Ok(())
}

fn run_undo(cfg: &config::AppConfig) -> Result<()> {
    git::ensure_head_exists()?;

    if git::head_is_merge_commit()? {
        let proceed_merge =
            Confirm::new("Latest commit is a merge commit. Undo it with git reset --soft HEAD~1?")
                .with_default(false)
                .prompt()
                .unwrap_or(false);
        if !proceed_merge {
            println!("{}", "Undo cancelled.".dimmed());
            return Ok(());
        }
    }

    if !git::has_upstream_branch()? {
        println!(
            "{}",
            "No upstream branch detected. Assuming latest commit is not pushed."
                .yellow()
                .bold()
        );
    } else if git::is_head_pushed()? {
        let proceed_pushed =
            Confirm::new("Latest commit appears to be pushed already. Undo locally anyway?")
                .with_default(false)
                .prompt()
                .unwrap_or(false);
        if !proceed_pushed {
            println!("{}", "Undo cancelled.".dimmed());
            return Ok(());
        }
    }

    git::undo_last_commit_soft(cfg.suppress_tool_output).context("Failed to undo latest commit")?;
    println!("{}", "Latest commit undone (soft reset).".green().bold());
    Ok(())
}
