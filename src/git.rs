use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};

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

/// List staged file paths
pub fn list_staged_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--staged", "--name-only"])
        .output()
        .context("Failed to run git diff --staged --name-only")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff --staged --name-only failed: {stderr}");
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    Ok(files)
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
pub fn run_commit(message: &str, extra_args: &[String], suppress_output: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["commit", "-m", message]);
    cmd.args(extra_args);
    configure_stdio(&mut cmd, suppress_output);
    let status = cmd.status().context("Failed to run git commit")?;

    if !status.success() {
        bail!("git commit exited with status {status}");
    }

    Ok(())
}

/// Run `git push`
pub fn run_push(suppress_output: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("push");
    configure_stdio(&mut cmd, suppress_output);

    let status = cmd.status().context("Failed to run git push")?;
    if !status.success() {
        bail!("git push exited with status {status}");
    }

    Ok(())
}

/// Returns true when HEAD exists on upstream branch
pub fn is_head_pushed() -> Result<bool> {
    if !has_upstream_branch()? {
        return Ok(false);
    }

    let output = Command::new("git")
        .args(["branch", "-r", "--contains", "HEAD"])
        .output()
        .context("Failed to determine whether HEAD is pushed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git branch -r --contains HEAD failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pushed = stdout
        .lines()
        .map(str::trim)
        .any(|line| !line.is_empty() && !line.contains("->"));
    Ok(pushed)
}

/// Returns true if HEAD has multiple parents
pub fn head_is_merge_commit() -> Result<bool> {
    ensure_head_exists()?;

    let output = Command::new("git")
        .args(["rev-list", "--parents", "-n", "1", "HEAD"])
        .output()
        .context("Failed to inspect latest commit parents")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-list --parents -n 1 HEAD failed: {stderr}");
    }

    let parent_count = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .count()
        .saturating_sub(1);
    Ok(parent_count > 1)
}

/// Undo latest commit, keep all changes staged
pub fn undo_last_commit_soft(suppress_output: bool) -> Result<()> {
    ensure_head_exists()?;

    let mut cmd = Command::new("git");
    cmd.args(["reset", "--soft", "HEAD~1"]);
    configure_stdio(&mut cmd, suppress_output);

    let status = cmd
        .status()
        .context("Failed to run git reset --soft HEAD~1")?;
    if !status.success() {
        bail!("git reset --soft HEAD~1 exited with status {status}");
    }
    Ok(())
}

pub fn has_upstream_branch() -> Result<bool> {
    let status = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to detect upstream branch")?;
    Ok(status.success())
}

pub fn ensure_head_exists() -> Result<()> {
    let status = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to run git rev-parse --verify HEAD")?;

    if !status.success() {
        bail!("No commits found in this repository.");
    }
    Ok(())
}

fn configure_stdio(cmd: &mut Command, suppress_output: bool) {
    if suppress_output {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
}
