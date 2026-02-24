use std::collections::HashSet;

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

    /// Print the final system prompt sent to the LLM (without diff payload)
    #[arg(long)]
    pub verbose: bool,

    /// Create a semantic version tag after a successful commit
    #[arg(long)]
    pub tag: bool,

    /// Extra arguments forwarded to `git commit`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub extra_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open interactive configuration editor
    Config,
    /// Undo latest commit (soft reset)
    Undo,
    /// Generate message from existing commit diff and rewrite commit message
    Alter {
        /// One hash: rewrite that commit from its own diff. Two hashes: use older..newer diff and rewrite newer.
        #[arg(value_name = "HASH", num_args = 1..=2)]
        commits: Vec<String>,
    },
    /// Update cgen to the latest version
    Update,
    /// Print the LLM system prompt without running anything
    Prompt,
}

pub fn parse() -> Cli {
    Cli::parse()
}

/// Menu entry types for the grouped interactive config
enum MenuEntry {
    GroupHeader {
        name: &'static str,
        expanded: bool,
    },
    SubgroupHeader {
        name: &'static str,
        is_last_subgroup: bool,
    },
    Field {
        display_name: &'static str,
        suffix: &'static str,
        value: String,
        is_last_in_group: bool,
        in_subgroup: bool,
        parent_is_last_subgroup: bool,
        is_last_in_subgroup: bool,
    },
}

pub fn interactive_config(global: bool) -> Result<()> {
    let mut cfg = AppConfig::load()?;
    let scope = if global { "global" } else { "local" };

    println!("\n{}  {} configuration\n", "cgen".cyan().bold(), scope);

    let mut expanded: HashSet<&str> = HashSet::new();
    expanded.insert("Basic");
    let mut first_render = true;

    loop {
        // Clear screen and re-home cursor for in-place redraw
        if first_render {
            first_render = false;
        } else {
            print!("\x1B[2J\x1B[H");
            println!("\n{}  {} configuration\n", "cgen".cyan().bold(), scope);
        }

        let groups = cfg.grouped_fields();
        let mut entries: Vec<MenuEntry> = Vec::new();

        for group in &groups {
            let is_expanded = expanded.contains(group.name);
            entries.push(MenuEntry::GroupHeader {
                name: group.name,
                expanded: is_expanded,
            });

            if is_expanded {
                let has_subgroups = !group.subgroups.is_empty();

                for (i, (display_name, suffix, val)) in group.fields.iter().enumerate() {
                    let is_last = !has_subgroups && i == group.fields.len() - 1;
                    entries.push(MenuEntry::Field {
                        display_name,
                        suffix,
                        value: val.clone(),
                        is_last_in_group: is_last,
                        in_subgroup: false,
                        parent_is_last_subgroup: false,
                        is_last_in_subgroup: false,
                    });
                }

                for (sg_idx, sg) in group.subgroups.iter().enumerate() {
                    let is_last_sg = sg_idx == group.subgroups.len() - 1;
                    entries.push(MenuEntry::SubgroupHeader {
                        name: sg.name,
                        is_last_subgroup: is_last_sg,
                    });
                    for (f_idx, (display_name, suffix, val)) in sg.fields.iter().enumerate() {
                        let is_last_field = f_idx == sg.fields.len() - 1;
                        entries.push(MenuEntry::Field {
                            display_name,
                            suffix,
                            value: val.clone(),
                            is_last_in_group: is_last_sg && is_last_field,
                            in_subgroup: true,
                            parent_is_last_subgroup: is_last_sg,
                            is_last_in_subgroup: is_last_field,
                        });
                    }
                }
            }
        }

        let options: Vec<String> = entries
            .iter()
            .map(|entry| match entry {
                MenuEntry::GroupHeader { name, expanded } => {
                    let arrow = if *expanded { "\u{25BC}" } else { "\u{25B6}" };
                    format!("{} {}", arrow, name.bold())
                }
                MenuEntry::SubgroupHeader {
                    name,
                    is_last_subgroup,
                } => {
                    let connector = if *is_last_subgroup {
                        "\u{2514}\u{2500}\u{2500}"
                    } else {
                        "\u{251C}\u{2500}\u{2500}"
                    };
                    format!("  {} {}", connector, name.bold().dimmed())
                }
                MenuEntry::Field {
                    display_name,
                    value,
                    in_subgroup,
                    parent_is_last_subgroup,
                    is_last_in_subgroup,
                    is_last_in_group,
                    ..
                } => {
                    if *in_subgroup {
                        // Indent deeper under subgroup, with continuation line from parent
                        let pipe = if *parent_is_last_subgroup {
                            " "
                        } else {
                            "\u{2502}"
                        };
                        let connector = if *is_last_in_subgroup {
                            "\u{2514}\u{2500}\u{2500}"
                        } else {
                            "\u{251C}\u{2500}\u{2500}"
                        };
                        format!(
                            "  {}   {} {:<22} {}",
                            pipe,
                            connector,
                            display_name,
                            value.dimmed()
                        )
                    } else {
                        let connector = if *is_last_in_group {
                            "\u{2514}\u{2500}\u{2500}"
                        } else {
                            "\u{251C}\u{2500}\u{2500}"
                        };
                        format!(
                            "  {} {:<22} {}",
                            connector,
                            display_name,
                            value.dimmed()
                        )
                    }
                }
            })
            .collect();

        let mut all_options = options.clone();
        all_options.push("Save & Exit".green().to_string());
        all_options.push("Exit without saving".red().to_string());

        let selection = Select::new("Edit a setting:", all_options)
            .with_page_size(22)
            .prompt();

        let selection = match selection {
            Ok(s) => s,
            Err(_) => break,
        };

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

        // Find which entry was selected
        let idx = options.iter().position(|o| selection.contains(o.as_str()));
        let idx = match idx {
            Some(i) => i,
            None => continue,
        };

        match &entries[idx] {
            MenuEntry::GroupHeader { name, expanded: is_expanded } => {
                if *is_expanded {
                    expanded.remove(name);
                } else {
                    expanded.insert(name);
                }
                continue;
            }
            MenuEntry::SubgroupHeader { .. } => {
                continue;
            }
            MenuEntry::Field { suffix, .. } => {
                let new_value = edit_field(suffix, &cfg);
                if let Some(val) = new_value {
                    if let Err(err) = cfg.set_field(suffix, &val) {
                        println!("  {} {}", "error:".red().bold(), err);
                        continue;
                    }
                    if *suffix == "PROVIDER" {
                        let default_model = crate::provider::default_model_for(&val);
                        cfg.set_field("MODEL", default_model)?;
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
        }
    }

    Ok(())
}

fn edit_field(suffix: &str, cfg: &AppConfig) -> Option<String> {
    match suffix {
        "PROVIDER" => {
            let choices = vec!["gemini", "openai", "anthropic", "groq", "(custom)"];
            match Select::new("Provider:", choices).prompt() {
                Ok("(custom)") => Text::new("Custom provider name:").prompt().ok(),
                Ok(v) => Some(v.to_string()),
                Err(_) => None,
            }
        }
        "ONE_LINER" => {
            let choices = vec!["enabled", "disabled"];
            Select::new("One-liner commits:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "USE_GITMOJI" => {
            let choices = vec!["disabled", "enabled"];
            Select::new("Use Gitmoji:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "GITMOJI_FORMAT" => {
            let choices = vec!["unicode", "shortcode"];
            Select::new("Gitmoji format:", choices)
                .prompt()
                .ok()
                .map(|v| v.to_string())
        }
        "REVIEW_COMMIT" => {
            let choices = vec!["disabled", "enabled"];
            Select::new("Review commit before confirming:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "POST_COMMIT_PUSH" => {
            let choices = vec!["ask", "always", "never"];
            Select::new("Post-commit push behavior:", choices)
                .prompt()
                .ok()
                .map(|v| v.to_string())
        }
        "SUPPRESS_TOOL_OUTPUT" => {
            let choices = vec!["disabled", "enabled"];
            Select::new("Suppress git command output:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "WARN_STAGED_FILES_ENABLED" => {
            let choices = vec!["enabled", "disabled"];
            Select::new("Warn when staged files exceed threshold:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "WARN_STAGED_FILES_THRESHOLD" => Text::new("Warn threshold (staged files count):")
            .with_help_message(
                "Integer value; warning shows when count is greater than this threshold",
            )
            .prompt()
            .ok(),
        "CONFIRM_NEW_VERSION" => {
            let choices = vec!["enabled", "disabled"];
            Select::new("Confirm new semantic version tag:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "AUTO_UPDATE" => {
            let choices = vec!["enabled", "disabled"];
            Select::new("Enable automatic updates:", choices)
                .prompt()
                .ok()
                .map(|v| if v == "enabled" { "1" } else { "0" }.to_string())
        }
        "API_KEY" => Text::new("API Key:")
            .with_help_message("Your LLM provider API key")
            .prompt()
            .ok(),
        _ => {
            let fields = cfg.fields_display();
            let field = fields.iter().find(|(_, s, _)| *s == suffix);
            match field {
                Some((name, _, val)) => {
                    let prompt_text = format!("{}:", name);
                    Text::new(&prompt_text).with_default(val).prompt().ok()
                }
                None => None,
            }
        }
    }
}
