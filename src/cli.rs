use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use inquire::{Select, Text};

use crate::config::AppConfig;

#[derive(Parser, Debug)]
#[command(
    name = "cgen",
    about = "Generate git commit messages via LLMs",
    version,
    after_help = "Any arguments after `cgen` (without a subcommand) are forwarded to `git commit`."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Generate and print commit message without creating a commit
    #[arg(long)]
    pub dry_run: bool,

    /// Extra arguments forwarded to `git commit`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub extra_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open interactive configuration editor
    Config {
        /// Edit global config instead of local .env
        #[arg(long, short)]
        global: bool,
    },
    /// Undo latest commit (soft reset)
    Undo,
    /// Generate message from existing commit diff and rewrite commit message
    Alter {
        /// One hash: rewrite that commit from its own diff. Two hashes: use older..newer diff and rewrite newer.
        #[arg(value_name = "HASH", num_args = 1..=2)]
        commits: Vec<String>,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}

pub fn interactive_config(global: bool) -> Result<()> {
    let mut cfg = AppConfig::load().unwrap_or_default();
    let scope = if global { "global" } else { "local" };

    println!("\n{}  {} configuration\n", "cgen".cyan().bold(), scope);

    loop {
        let fields = cfg.fields_display();
        let options: Vec<String> = fields
            .iter()
            .map(|(name, _suffix, val)| format!("{:<18} {}", name, val.dimmed()))
            .collect();

        let mut all_options = options.clone();
        all_options.push("Save & Exit".green().to_string());
        all_options.push("Exit without saving".red().to_string());

        let selection = Select::new("Edit a setting:", all_options)
            .with_page_size(17)
            .prompt();

        let selection = match selection {
            Ok(s) => s,
            Err(_) => break, // Ctrl+C / ESC
        };

        // Check for exit options
        if selection.contains("Save & Exit") {
            if global {
                cfg.save_global()?;
                let path = crate::config::global_config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                println!("\n{} Saved to {}", "done!".green().bold(), path.dimmed());
            } else {
                cfg.save_local()?;
                println!("\n{} Saved to {}", "done!".green().bold(), ".env".dimmed());
            }
            break;
        }
        if selection.contains("Exit without saving") {
            println!("{}", "Cancelled.".dimmed());
            break;
        }

        // Find which field was selected
        let idx = options.iter().position(|o| selection.contains(o.as_str()));
        let idx = match idx {
            Some(i) => i,
            None => continue,
        };

        let (_name, suffix, _val) = &fields[idx];

        // Edit the field
        let new_value = match *suffix {
            "PROVIDER" => {
                let choices = vec!["gemini", "openai", "anthropic", "groq", "(custom)"];
                match Select::new("Provider:", choices).prompt() {
                    Ok("(custom)") => Text::new("Custom provider name:").prompt().ok(),
                    Ok(v) => Some(v.to_string()),
                    Err(_) => None,
                }
            }
            "ONE_LINER" => {
                let choices = vec!["1 (yes)", "0 (no)"];
                Select::new("One-liner commits:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.chars().next().unwrap().to_string())
            }
            "USE_GITMOJI" => {
                let choices = vec!["0 (no)", "1 (yes)"];
                Select::new("Use Gitmoji:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.chars().next().unwrap().to_string())
            }
            "GITMOJI_FORMAT" => {
                let choices = vec!["unicode", "shortcode"];
                Select::new("Gitmoji format:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.to_string())
            }
            "REVIEW_COMMIT" => {
                let choices = vec!["0 (no)", "1 (yes)"];
                Select::new("Review commit before confirming:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.chars().next().unwrap().to_string())
            }
            "POST_COMMIT_PUSH" => {
                let choices = vec!["ask", "always", "never"];
                Select::new("Post-commit push behavior:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.to_string())
            }
            "SUPPRESS_TOOL_OUTPUT" => {
                let choices = vec!["0 (no)", "1 (yes)"];
                Select::new("Suppress git command output:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.chars().next().unwrap().to_string())
            }
            "WARN_STAGED_FILES_ENABLED" => {
                let choices = vec!["1 (yes)", "0 (no)"];
                Select::new("Warn when staged files exceed threshold:", choices)
                    .prompt()
                    .ok()
                    .map(|v| v.chars().next().unwrap().to_string())
            }
            "WARN_STAGED_FILES_THRESHOLD" => Text::new("Warn threshold (staged files count):")
                .with_help_message(
                    "Integer value; warning shows when count is greater than this threshold",
                )
                .prompt()
                .ok(),
            "API_KEY" => Text::new("API Key:")
                .with_help_message("Your LLM provider API key")
                .prompt()
                .ok(),
            _ => {
                let current = fields[idx].2.clone();
                let prompt_text = format!("{}:", fields[idx].0);
                Text::new(&prompt_text).with_default(&current).prompt().ok()
            }
        };

        if let Some(val) = new_value {
            cfg.set_field(suffix, &val);

            // When switching providers, auto-set the model to that provider's default
            if *suffix == "PROVIDER" {
                let default_model = crate::provider::default_model_for(&val);
                cfg.set_field("MODEL", default_model);
                if default_model.is_empty() {
                    println!(
                        "  {} Model cleared (set it manually)",
                        "note:".yellow().bold()
                    );
                } else {
                    println!(
                        "  {} Model set to {}",
                        "note:".yellow().bold(),
                        default_model.dimmed()
                    );
                }
            }
        }
    }

    Ok(())
}
