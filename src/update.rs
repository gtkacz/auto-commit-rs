use anyhow::{Context, Result};
use colored::Colorize;
use std::time::Duration;

const GITHUB_REPO: &str = "gtkacz/smart-commit-rs";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct VersionCheck {
    pub latest: String,
    pub current: String,
    pub update_available: bool,
}

/// Fetch the latest release tag from GitHub API with a short timeout
pub fn fetch_latest_version() -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let response: serde_json::Value = agent
        .get(&url)
        .set("User-Agent", "cgen")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to reach GitHub API")?
        .into_json()
        .context("Failed to parse GitHub API response")?;

    let tag = response["tag_name"]
        .as_str()
        .context("No tag_name in GitHub release response")?;

    Ok(tag.to_string())
}

/// Parse a version string (strips leading 'v' if present) into (major, minor, patch)
pub fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let v = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

/// Check if a newer version is available on GitHub
pub fn check_version() -> Result<VersionCheck> {
    let latest = fetch_latest_version()?;
    let current = CURRENT_VERSION.to_string();

    let update_available = match (parse_semver(&latest), parse_semver(&current)) {
        (Some(latest_v), Some(current_v)) => latest_v > current_v,
        _ => false,
    };

    Ok(VersionCheck {
        latest,
        current,
        update_available,
    })
}

/// Run the appropriate update command for the current platform
pub fn run_update() -> Result<()> {
    if is_cargo_available() {
        println!("{}", "Updating via cargo...".cyan().bold());
        let status = std::process::Command::new("cargo")
            .args(["install", "smart-commit-rs"])
            .status()
            .context("Failed to run cargo install")?;

        if !status.success() {
            anyhow::bail!("cargo install failed with exit code {}", status);
        }
    } else {
        run_platform_installer()?;
    }

    println!("{}", "Update complete!".green().bold());
    Ok(())
}

fn is_cargo_available() -> bool {
    std::process::Command::new("cargo")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_platform_installer() -> Result<()> {
    if cfg!(target_os = "windows") {
        println!("{}", "Updating via PowerShell installer...".cyan().bold());
        let status = std::process::Command::new("powershell")
            .args([
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "irm https://raw.githubusercontent.com/gtkacz/smart-commit-rs/main/scripts/install.ps1 | iex",
            ])
            .status()
            .context("Failed to run PowerShell installer")?;

        if !status.success() {
            anyhow::bail!("PowerShell installer failed");
        }
    } else {
        println!("{}", "Updating via install script...".cyan().bold());
        let status = std::process::Command::new("bash")
            .args([
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/gtkacz/smart-commit-rs/main/scripts/install.sh | bash",
            ])
            .status()
            .context("Failed to run install script")?;

        if !status.success() {
            anyhow::bail!("Install script failed");
        }
    }

    Ok(())
}

/// Print a warning that a newer version is available
pub fn print_update_warning(latest: &str) {
    eprintln!(
        "\n{}  {} → {}  (run {} to update)",
        "Update available!".yellow().bold(),
        CURRENT_VERSION.dimmed(),
        latest.green(),
        "cgen update".cyan(),
    );
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}
