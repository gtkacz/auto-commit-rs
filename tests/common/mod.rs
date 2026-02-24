#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use auto_commit_rs::config::global_config_path;
use tempfile::TempDir;

pub struct DirGuard {
    original: PathBuf,
}

impl DirGuard {
    pub fn enter(path: &Path) -> Self {
        let original = std::env::current_dir().expect("failed to read current directory");
        std::env::set_current_dir(path).expect("failed to change current directory");
        Self { original }
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

pub struct EnvGuard {
    previous: HashMap<String, Option<String>>,
}

impl EnvGuard {
    pub fn set(pairs: &[(&str, &str)]) -> Self {
        let mut previous = HashMap::new();
        for (key, value) in pairs {
            previous.insert((*key).to_string(), std::env::var(key).ok());
            std::env::set_var(key, value);
        }
        Self { previous }
    }

    pub fn clear(keys: &[&str]) -> Self {
        let mut previous = HashMap::new();
        for key in keys {
            previous.insert((*key).to_string(), std::env::var(key).ok());
            std::env::remove_var(key);
        }
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}

enum GlobalConfigState {
    Missing,
    Existing(Vec<u8>),
    Unreadable,
}

pub struct GlobalConfigGuard {
    path: Option<PathBuf>,
    state: GlobalConfigState,
}

impl GlobalConfigGuard {
    pub fn backup() -> Self {
        let path = global_config_path();
        let state = match path.as_ref() {
            Some(config_path) if config_path.exists() => match std::fs::read(config_path) {
                Ok(bytes) => GlobalConfigState::Existing(bytes),
                Err(_) => GlobalConfigState::Unreadable,
            },
            _ => GlobalConfigState::Missing,
        };
        Self { path, state }
    }
}

impl Drop for GlobalConfigGuard {
    fn drop(&mut self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };

        match &self.state {
            GlobalConfigState::Missing => {
                let _ = std::fs::remove_file(path);
            }
            GlobalConfigState::Existing(bytes) => {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, bytes);
            }
            GlobalConfigState::Unreadable => {}
        }
    }
}

pub fn init_git_repo() -> TempDir {
    let repo = TempDir::new().expect("failed to create temp dir");
    git_ok(repo.path(), ["init"]);
    git_ok(repo.path(), ["config", "user.name", "Test User"]);
    git_ok(repo.path(), ["config", "user.email", "test@example.com"]);
    repo
}

pub fn write_file(path: &Path, content: &str) {
    std::fs::write(path, content).expect("failed to write file");
}

pub fn git_ok<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git command failed: git {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn git_stdout<const N: usize>(cwd: &Path, args: [&str; N]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git command failed: git {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn commit_file(repo_path: &Path, rel_path: &str, content: &str, message: &str) -> String {
    let full_path = repo_path.join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create parent directories");
    }
    write_file(&full_path, content);
    git_ok(repo_path, ["add", rel_path]);
    git_ok(repo_path, ["commit", "-m", message]);
    git_stdout(repo_path, ["rev-parse", "HEAD"])
}
