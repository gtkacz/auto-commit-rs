use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const DEFAULT_SYSTEM_PROMPT: &str = "You are to act as an author of a commit message in git. \
I'll send you an output of 'git diff --staged' command, and you are to convert \
it into a commit message. Follow the Conventional Commits specification.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_headers: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_true")]
    pub one_liner: bool,
    #[serde(default = "default_commit_template")]
    pub commit_template: String,
    #[serde(default = "default_system_prompt")]
    pub llm_system_prompt: String,
    #[serde(default)]
    pub use_gitmoji: bool,
    #[serde(default = "default_gitmoji_format")]
    pub gitmoji_format: String,
    #[serde(default)]
    pub review_commit: bool,
    #[serde(default = "default_post_commit_push")]
    pub post_commit_push: String,
    #[serde(default)]
    pub suppress_tool_output: bool,
    #[serde(default = "default_true")]
    pub warn_staged_files_enabled: bool,
    #[serde(default = "default_warn_staged_files_threshold")]
    pub warn_staged_files_threshold: usize,
}

fn default_provider() -> String {
    "groq".into()
}
fn default_model() -> String {
    "llama-3.3-70b-versatile".into()
}
fn default_locale() -> String {
    "en".into()
}
fn default_true() -> bool {
    true
}
fn default_post_commit_push() -> String {
    "ask".into()
}
fn default_commit_template() -> String {
    "$msg".into()
}
fn default_system_prompt() -> String {
    DEFAULT_SYSTEM_PROMPT.into()
}
fn default_gitmoji_format() -> String {
    "unicode".into()
}
fn default_warn_staged_files_threshold() -> usize {
    20
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: String::new(),
            api_url: String::new(),
            api_headers: String::new(),
            locale: default_locale(),
            one_liner: true,
            commit_template: default_commit_template(),
            llm_system_prompt: default_system_prompt(),
            use_gitmoji: false,
            gitmoji_format: default_gitmoji_format(),
            review_commit: false,
            post_commit_push: default_post_commit_push(),
            suppress_tool_output: false,
            warn_staged_files_enabled: true,
            warn_staged_files_threshold: default_warn_staged_files_threshold(),
        }
    }
}

/// Map of ACR_ env var suffix → struct field name
const ENV_FIELD_MAP: &[(&str, &str)] = &[
    ("PROVIDER", "provider"),
    ("MODEL", "model"),
    ("API_KEY", "api_key"),
    ("API_URL", "api_url"),
    ("API_HEADERS", "api_headers"),
    ("LOCALE", "locale"),
    ("ONE_LINER", "one_liner"),
    ("COMMIT_TEMPLATE", "commit_template"),
    ("LLM_SYSTEM_PROMPT", "llm_system_prompt"),
    ("USE_GITMOJI", "use_gitmoji"),
    ("GITMOJI_FORMAT", "gitmoji_format"),
    ("REVIEW_COMMIT", "review_commit"),
    ("POST_COMMIT_PUSH", "post_commit_push"),
    ("SUPPRESS_TOOL_OUTPUT", "suppress_tool_output"),
    ("WARN_STAGED_FILES_ENABLED", "warn_staged_files_enabled"),
    ("WARN_STAGED_FILES_THRESHOLD", "warn_staged_files_threshold"),
];

impl AppConfig {
    /// Load config with layered resolution: defaults → global TOML → local .env → env vars
    pub fn load() -> Result<Self> {
        let mut cfg = Self::default();

        // Layer 1: Global TOML
        if let Some(path) = global_config_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                let file_cfg: AppConfig = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse {}", path.display()))?;
                cfg.merge_from(&file_cfg);
            }
        }

        // Layer 2: Local .env (in git repo root)
        if let Ok(root) = crate::git::find_repo_root() {
            let env_path = PathBuf::from(&root).join(".env");
            if env_path.exists() {
                let env_map = parse_dotenv(&env_path)?;
                cfg.apply_env_map(&env_map);
            }
        }

        // Layer 3: Actual environment variables
        let mut env_map = HashMap::new();
        for (suffix, _) in ENV_FIELD_MAP {
            let key = format!("ACR_{suffix}");
            if let Ok(val) = std::env::var(&key) {
                env_map.insert(key, val);
            }
        }
        cfg.apply_env_map(&env_map);

        Ok(cfg)
    }

    fn merge_from(&mut self, other: &AppConfig) {
        if !other.provider.is_empty() {
            self.provider = other.provider.clone();
        }
        if !other.model.is_empty() {
            self.model = other.model.clone();
        }
        if !other.api_key.is_empty() {
            self.api_key = other.api_key.clone();
        }
        if !other.api_url.is_empty() {
            self.api_url = other.api_url.clone();
        }
        if !other.api_headers.is_empty() {
            self.api_headers = other.api_headers.clone();
        }
        if !other.locale.is_empty() {
            self.locale = other.locale.clone();
        }
        self.one_liner = other.one_liner;
        if !other.commit_template.is_empty() {
            self.commit_template = other.commit_template.clone();
        }
        if !other.llm_system_prompt.is_empty() {
            self.llm_system_prompt = other.llm_system_prompt.clone();
        }
        self.use_gitmoji = other.use_gitmoji;
        if !other.gitmoji_format.is_empty() {
            self.gitmoji_format = other.gitmoji_format.clone();
        }
        self.review_commit = other.review_commit;
        if !other.post_commit_push.is_empty() {
            self.post_commit_push = normalize_post_commit_push(&other.post_commit_push);
        }
        self.suppress_tool_output = other.suppress_tool_output;
        self.warn_staged_files_enabled = other.warn_staged_files_enabled;
        self.warn_staged_files_threshold = other.warn_staged_files_threshold;
    }

    fn apply_env_map(&mut self, map: &HashMap<String, String>) {
        for (suffix, _field) in ENV_FIELD_MAP {
            let key = format!("ACR_{suffix}");
            if let Some(val) = map.get(&key) {
                match *suffix {
                    "PROVIDER" => self.provider = val.clone(),
                    "MODEL" => self.model = val.clone(),
                    "API_KEY" => self.api_key = val.clone(),
                    "API_URL" => self.api_url = val.clone(),
                    "API_HEADERS" => self.api_headers = val.clone(),
                    "LOCALE" => self.locale = val.clone(),
                    "ONE_LINER" => self.one_liner = val == "1" || val.eq_ignore_ascii_case("true"),
                    "COMMIT_TEMPLATE" => self.commit_template = val.clone(),
                    "LLM_SYSTEM_PROMPT" => self.llm_system_prompt = val.clone(),
                    "USE_GITMOJI" => {
                        self.use_gitmoji = val == "1" || val.eq_ignore_ascii_case("true")
                    }
                    "GITMOJI_FORMAT" => self.gitmoji_format = val.clone(),
                    "REVIEW_COMMIT" => {
                        self.review_commit = val == "1" || val.eq_ignore_ascii_case("true")
                    }
                    "POST_COMMIT_PUSH" => self.post_commit_push = normalize_post_commit_push(val),
                    "SUPPRESS_TOOL_OUTPUT" => {
                        self.suppress_tool_output = val == "1" || val.eq_ignore_ascii_case("true")
                    }
                    "WARN_STAGED_FILES_ENABLED" => {
                        self.warn_staged_files_enabled =
                            val == "1" || val.eq_ignore_ascii_case("true")
                    }
                    "WARN_STAGED_FILES_THRESHOLD" => {
                        self.warn_staged_files_threshold =
                            parse_usize_or_default(val, default_warn_staged_files_threshold());
                    }
                    _ => {}
                }
            }
        }
    }

    /// Save to global TOML config file
    pub fn save_global(&self) -> Result<()> {
        let path = global_config_path().context("Could not determine global config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    /// Save to local .env file in the git repo root
    pub fn save_local(&self) -> Result<()> {
        let root = crate::git::find_repo_root().context("Not in a git repository")?;
        let env_path = PathBuf::from(&root).join(".env");

        let mut lines = Vec::new();
        lines.push(format!("ACR_PROVIDER={}", self.provider));
        lines.push(format!("ACR_MODEL={}", self.model));
        if !self.api_key.is_empty() {
            lines.push(format!("ACR_API_KEY={}", self.api_key));
        }
        if !self.api_url.is_empty() {
            lines.push(format!("ACR_API_URL={}", self.api_url));
        }
        if !self.api_headers.is_empty() {
            lines.push(format!("ACR_API_HEADERS={}", self.api_headers));
        }
        lines.push(format!("ACR_LOCALE={}", self.locale));
        lines.push(format!(
            "ACR_ONE_LINER={}",
            if self.one_liner { "1" } else { "0" }
        ));
        if self.commit_template != "$msg" {
            lines.push(format!("ACR_COMMIT_TEMPLATE={}", self.commit_template));
        }
        if self.llm_system_prompt != DEFAULT_SYSTEM_PROMPT {
            lines.push(format!("ACR_LLM_SYSTEM_PROMPT={}", self.llm_system_prompt));
        }
        lines.push(format!(
            "ACR_USE_GITMOJI={}",
            if self.use_gitmoji { "1" } else { "0" }
        ));
        lines.push(format!("ACR_GITMOJI_FORMAT={}", self.gitmoji_format));
        lines.push(format!(
            "ACR_REVIEW_COMMIT={}",
            if self.review_commit { "1" } else { "0" }
        ));
        lines.push(format!(
            "ACR_POST_COMMIT_PUSH={}",
            normalize_post_commit_push(&self.post_commit_push)
        ));
        lines.push(format!(
            "ACR_SUPPRESS_TOOL_OUTPUT={}",
            if self.suppress_tool_output { "1" } else { "0" }
        ));
        lines.push(format!(
            "ACR_WARN_STAGED_FILES_ENABLED={}",
            if self.warn_staged_files_enabled {
                "1"
            } else {
                "0"
            }
        ));
        lines.push(format!(
            "ACR_WARN_STAGED_FILES_THRESHOLD={}",
            self.warn_staged_files_threshold
        ));

        std::fs::write(&env_path, lines.join("\n") + "\n")
            .with_context(|| format!("Failed to write {}", env_path.display()))?;
        Ok(())
    }

    /// Get all fields as (display_name, env_suffix, current_value) tuples
    pub fn fields_display(&self) -> Vec<(&'static str, &'static str, String)> {
        vec![
            ("Provider", "PROVIDER", self.provider.clone()),
            ("Model", "MODEL", self.model.clone()),
            (
                "API Key",
                "API_KEY",
                if self.api_key.is_empty() {
                    "(not set)".into()
                } else {
                    mask_key(&self.api_key)
                },
            ),
            (
                "API URL",
                "API_URL",
                if self.api_url.is_empty() {
                    "(auto from provider)".into()
                } else {
                    self.api_url.clone()
                },
            ),
            (
                "API Headers",
                "API_HEADERS",
                if self.api_headers.is_empty() {
                    "(auto from provider)".into()
                } else {
                    self.api_headers.clone()
                },
            ),
            ("Locale", "LOCALE", self.locale.clone()),
            (
                "One-liner",
                "ONE_LINER",
                if self.one_liner {
                    "1 (yes)".into()
                } else {
                    "0 (no)".into()
                },
            ),
            (
                "Commit Template",
                "COMMIT_TEMPLATE",
                self.commit_template.clone(),
            ),
            (
                "System Prompt",
                "LLM_SYSTEM_PROMPT",
                truncate(&self.llm_system_prompt, 60),
            ),
            (
                "Use Gitmoji",
                "USE_GITMOJI",
                if self.use_gitmoji {
                    "1 (yes)".into()
                } else {
                    "0 (no)".into()
                },
            ),
            (
                "Gitmoji Format",
                "GITMOJI_FORMAT",
                self.gitmoji_format.clone(),
            ),
            (
                "Review Commit",
                "REVIEW_COMMIT",
                if self.review_commit {
                    "1 (yes)".into()
                } else {
                    "0 (no)".into()
                },
            ),
            (
                "Post Commit Push",
                "POST_COMMIT_PUSH",
                normalize_post_commit_push(&self.post_commit_push),
            ),
            (
                "Suppress Tool Output",
                "SUPPRESS_TOOL_OUTPUT",
                if self.suppress_tool_output {
                    "1 (yes)".into()
                } else {
                    "0 (no)".into()
                },
            ),
            (
                "Warn Staged Files",
                "WARN_STAGED_FILES_ENABLED",
                if self.warn_staged_files_enabled {
                    "1 (yes)".into()
                } else {
                    "0 (no)".into()
                },
            ),
            (
                "Staged Warn Threshold",
                "WARN_STAGED_FILES_THRESHOLD",
                self.warn_staged_files_threshold.to_string(),
            ),
        ]
    }

    /// Set a field by its env suffix
    pub fn set_field(&mut self, suffix: &str, value: &str) {
        match suffix {
            "PROVIDER" => self.provider = value.into(),
            "MODEL" => self.model = value.into(),
            "API_KEY" => self.api_key = value.into(),
            "API_URL" => self.api_url = value.into(),
            "API_HEADERS" => self.api_headers = value.into(),
            "LOCALE" => self.locale = value.into(),
            "ONE_LINER" => self.one_liner = value == "1" || value.eq_ignore_ascii_case("true"),
            "COMMIT_TEMPLATE" => self.commit_template = value.into(),
            "LLM_SYSTEM_PROMPT" => self.llm_system_prompt = value.into(),
            "USE_GITMOJI" => self.use_gitmoji = value == "1" || value.eq_ignore_ascii_case("true"),
            "GITMOJI_FORMAT" => self.gitmoji_format = value.into(),
            "REVIEW_COMMIT" => {
                self.review_commit = value == "1" || value.eq_ignore_ascii_case("true")
            }
            "POST_COMMIT_PUSH" => self.post_commit_push = normalize_post_commit_push(value),
            "SUPPRESS_TOOL_OUTPUT" => {
                self.suppress_tool_output = value == "1" || value.eq_ignore_ascii_case("true")
            }
            "WARN_STAGED_FILES_ENABLED" => {
                self.warn_staged_files_enabled = value == "1" || value.eq_ignore_ascii_case("true");
            }
            "WARN_STAGED_FILES_THRESHOLD" => {
                self.warn_staged_files_threshold =
                    parse_usize_or_default(value, default_warn_staged_files_threshold());
            }
            _ => {}
        }
    }
}

/// Global config file path
pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("cgen").join("config.toml"))
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn normalize_post_commit_push(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "never" => "never".into(),
        "always" => "always".into(),
        _ => "ask".into(),
    }
}

fn parse_usize_or_default(value: &str, default: usize) -> usize {
    value.trim().parse::<usize>().unwrap_or(default)
}

fn parse_dotenv(path: &PathBuf) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(key, val);
        }
    }
    Ok(map)
}
