use auto_commit_rs::cli::{Cli, Command};
use clap::Parser;

#[test]
fn parses_prompt_subcommand() {
    let cli = Cli::try_parse_from(["cgen", "prompt"]).expect("prompt should parse");
    assert!(matches!(cli.command, Some(Command::Prompt)));
}

#[test]
fn parses_config_subcommand_without_scope_flag() {
    let cli = Cli::try_parse_from(["cgen", "config"]).expect("config should parse");
    assert!(matches!(cli.command, Some(Command::Config)));
}

#[test]
fn rejects_removed_config_global_flag() {
    let err = Cli::try_parse_from(["cgen", "config", "--global"]).expect_err("should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("--global"),
        "expected clap to mention removed --global flag, got: {rendered}"
    );
}
