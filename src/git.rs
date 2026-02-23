use anyhow::{bail, Context, Result};
use std::process::Command;

/// Get the output of `git diff --staged`
pub fn get_staged_diff() -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--staged"])
        .output()
        .context("Failed to run git diff --staged")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff --staged failed: {stderr}");
    }

    let diff = String::from_utf8_lossy(&output.stdout).to_string();

    if diff.trim().is_empty() {
        bail!(
            "No staged changes found. Stage files with {} first.",
            colored::Colorize::yellow("git add <files>")
        );
    }

    Ok(diff)
}

/// Find the git repository root directory
pub fn find_repo_root() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to run git rev-parse")?;

    if !output.status.success() {
        bail!("Not in a git repository");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run `git commit -m "<message>" [extra_args...]`
pub fn run_commit(message: &str, extra_args: &[String]) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["commit", "-m", message]);
    cmd.args(extra_args);

    let status = cmd.status().context("Failed to run git commit")?;

    if !status.success() {
        bail!("git commit exited with status {status}");
    }

    Ok(())
}
