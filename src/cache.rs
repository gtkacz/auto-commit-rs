use anyhow::{Context, Result};
use colored::Colorize;
use inquire::Select;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCommit {
    pub hash: String,
    pub message_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoCache {
    pub repo_path: String,
    pub commits: Vec<CachedCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheIndex {
    pub repos: Vec<CacheIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndexEntry {
    pub repo_path: String,
    pub cache_file: String,
}

fn cache_dir() -> Option<PathBuf> {
    crate::config::global_config_path().map(|p| {
        p.parent()
            .expect("global config path should have a parent")
            .join("cache")
    })
}

fn repo_path_hash(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn index_path() -> Option<PathBuf> {
    cache_dir().map(|d| d.join("index.toml"))
}

fn load_index() -> Result<CacheIndex> {
    let path = match index_path() {
        Some(p) => p,
        None => return Ok(CacheIndex::default()),
    };
    if !path.exists() {
        return Ok(CacheIndex::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let idx: CacheIndex =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(idx)
}

fn save_index(index: &CacheIndex) -> Result<()> {
    let dir = cache_dir().context("Could not determine cache directory")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create {}", dir.display()))?;
    let path = dir.join("index.toml");
    let content = toml::to_string_pretty(index).context("Failed to serialize cache index")?;
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to rename temp file to {}", path.display()))?;
    Ok(())
}

fn load_repo_cache(repo_path: &str) -> Result<RepoCache> {
    let dir = match cache_dir() {
        Some(d) => d,
        None => {
            return Ok(RepoCache {
                repo_path: repo_path.into(),
                commits: Vec::new(),
            })
        }
    };
    let hash = repo_path_hash(repo_path);
    let path = dir.join(format!("{hash}.toml"));
    if !path.exists() {
        return Ok(RepoCache {
            repo_path: repo_path.into(),
            commits: Vec::new(),
        });
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let cache: RepoCache =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(cache)
}

fn save_repo_cache(cache: &RepoCache) -> Result<()> {
    let dir = cache_dir().context("Could not determine cache directory")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create {}", dir.display()))?;
    let hash = repo_path_hash(&cache.repo_path);
    let path = dir.join(format!("{hash}.toml"));
    let content = toml::to_string_pretty(cache).context("Failed to serialize repo cache")?;
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to rename temp file to {}", path.display()))?;
    Ok(())
}

pub fn record_commit(repo_path: &str, hash: &str, message_preview: &str) -> Result<()> {
    let mut index = load_index()?;
    let cache_file = format!("{}.toml", repo_path_hash(repo_path));

    if !index.repos.iter().any(|e| e.repo_path == repo_path) {
        index.repos.push(CacheIndexEntry {
            repo_path: repo_path.into(),
            cache_file,
        });
        save_index(&index)?;
    }

    let mut cache = load_repo_cache(repo_path)?;
    cache.commits.push(CachedCommit {
        hash: hash.into(),
        message_preview: message_preview.into(),
    });
    save_repo_cache(&cache)?;
    Ok(())
}

pub fn get_head_hash() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .context("Failed to run git rev-parse HEAD")?;
    if !output.status.success() {
        anyhow::bail!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn show_repo_commits(cache: &RepoCache) -> Result<()> {
    if cache.commits.is_empty() {
        println!("{}", "No tracked commits for this repository.".dimmed());
        return Ok(());
    }

    loop {
        let mut options: Vec<String> = cache
            .commits
            .iter()
            .rev()
            .map(|c| {
                let short = if c.hash.len() >= 7 {
                    &c.hash[..7]
                } else {
                    &c.hash
                };
                format!("{} {}", short, c.message_preview)
            })
            .collect();
        options.push("Back".into());

        let choice = match Select::new("Select commit to view:", options.clone()).prompt() {
            Ok(c) => c,
            Err(_) => break,
        };

        if choice == "Back" {
            break;
        }

        let idx = options.iter().position(|o| o == &choice).unwrap();
        let commit = &cache.commits[cache.commits.len() - 1 - idx];

        let status = std::process::Command::new("git")
            .args(["show", &commit.hash])
            .status();

        match status {
            Ok(s) if !s.success() => {
                println!(
                    "  {} Could not show commit {} (it may have been garbage collected)",
                    "error:".red().bold(),
                    &commit.hash[..7.min(commit.hash.len())]
                );
            }
            Err(e) => {
                println!("  {} {}", "error:".red().bold(), e);
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn interactive_history() -> Result<()> {
    match crate::git::find_repo_root() {
        Ok(repo_root) => {
            let cache = load_repo_cache(&repo_root)?;
            show_repo_commits(&cache)?;
        }
        Err(_) => {
            let index = load_index()?;
            if index.repos.is_empty() {
                println!("{}", "No tracked repositories found.".dimmed());
                return Ok(());
            }

            let options: Vec<String> = index.repos.iter().map(|e| e.repo_path.clone()).collect();
            match Select::new("Select repository:", options).prompt() {
                Ok(repo_path) => {
                    let cache = load_repo_cache(&repo_path)?;
                    show_repo_commits(&cache)?;
                }
                Err(_) => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_path_hash_deterministic() {
        let h1 = repo_path_hash("/home/user/project");
        let h2 = repo_path_hash("/home/user/project");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn test_repo_path_hash_different_paths() {
        let h1 = repo_path_hash("/home/user/project-a");
        let h2 = repo_path_hash("/home/user/project-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_cached_commit_serde() {
        let commit = CachedCommit {
            hash: "abc123def456".into(),
            message_preview: "feat: add login".into(),
        };
        let toml_str = toml::to_string(&commit).unwrap();
        let parsed: CachedCommit = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.hash, commit.hash);
        assert_eq!(parsed.message_preview, commit.message_preview);
    }

    #[test]
    fn test_repo_cache_serde() {
        let cache = RepoCache {
            repo_path: "/home/user/project".into(),
            commits: vec![
                CachedCommit {
                    hash: "aaa".into(),
                    message_preview: "first".into(),
                },
                CachedCommit {
                    hash: "bbb".into(),
                    message_preview: "second".into(),
                },
            ],
        };
        let toml_str = toml::to_string_pretty(&cache).unwrap();
        let parsed: RepoCache = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.commits.len(), 2);
        assert_eq!(parsed.repo_path, "/home/user/project");
    }

    #[test]
    fn test_cache_index_serde() {
        let index = CacheIndex {
            repos: vec![CacheIndexEntry {
                repo_path: "/home/user/project".into(),
                cache_file: "a1b2c3d4e5f67890.toml".into(),
            }],
        };
        let toml_str = toml::to_string_pretty(&index).unwrap();
        let parsed: CacheIndex = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.repos.len(), 1);
        assert_eq!(parsed.repos[0].cache_file, "a1b2c3d4e5f67890.toml");
    }
}
