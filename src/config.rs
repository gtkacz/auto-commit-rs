use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FieldSubgroup {
    pub name: &'static str,
    pub fields: Vec<(&'static str, &'static str, String)>,
}

pub struct FieldGroup {
    pub name: &'static str,
    pub fields: Vec<(&'static str, &'static str, String)>,
    pub subgroups: Vec<FieldSubgroup>,
}

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
    #[serde(default = "default_true")]
    pub confirm_new_version: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    #[serde(default = "default_true")]
    pub fallback_enabled: bool,
    #[serde(default = "default_true")]
    pub track_generated_commits: bool,
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
pub fn default_true() -> bool {
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
            review_commit: true,
            post_commit_push: default_post_commit_push(),
            suppress_tool_output: false,
            warn_staged_files_enabled: true,
            warn_staged_files_threshold: default_warn_staged_files_threshold(),
            confirm_new_version: true,
            auto_update: None,
            fallback_enabled: true,
            track_generated_commits: true,
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
    ("CONFIRM_NEW_VERSION", "confirm_new_version"),
    ("AUTO_UPDATE", "auto_update"),
    ("FALLBACK_ENABLED", "fallback_enabled"),
    ("TRACK_GENERATED_COMMITS", "track_generated_commits"),
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
        cfg.ensure_valid_locale()?;

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
        self.confirm_new_version = other.confirm_new_version;
        if other.auto_update.is_some() {
            self.auto_update = other.auto_update;
        }
        self.fallback_enabled = other.fallback_enabled;
        self.track_generated_commits = other.track_generated_commits;
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
                    "CONFIRM_NEW_VERSION" => {
                        self.confirm_new_version = val == "1" || val.eq_ignore_ascii_case("true")
                    }
                    "AUTO_UPDATE" => {
                        self.auto_update =
                            Some(val == "1" || val.eq_ignore_ascii_case("true"));
                    }
                    "FALLBACK_ENABLED" => {
                        self.fallback_enabled = val == "1" || val.eq_ignore_ascii_case("true");
                    }
                    "TRACK_GENERATED_COMMITS" => {
                        self.track_generated_commits =
                            val == "1" || val.eq_ignore_ascii_case("true");
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
        lines.push(format!(
            "ACR_CONFIRM_NEW_VERSION={}",
            if self.confirm_new_version { "1" } else { "0" }
        ));
        if let Some(auto_update) = self.auto_update {
            lines.push(format!(
                "ACR_AUTO_UPDATE={}",
                if auto_update { "1" } else { "0" }
            ));
        }
        lines.push(format!(
            "ACR_FALLBACK_ENABLED={}",
            if self.fallback_enabled { "1" } else { "0" }
        ));
        lines.push(format!(
            "ACR_TRACK_GENERATED_COMMITS={}",
            if self.track_generated_commits {
                "1"
            } else {
                "0"
            }
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
                    "enabled".into()
                } else {
                    "disabled".into()
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
                    "enabled".into()
                } else {
                    "disabled".into()
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
                    "enabled".into()
                } else {
                    "disabled".into()
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
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            ),
            (
                "Warn Staged Files",
                "WARN_STAGED_FILES_ENABLED",
                if self.warn_staged_files_enabled {
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            ),
            (
                "Staged Warn Threshold",
                "WARN_STAGED_FILES_THRESHOLD",
                self.warn_staged_files_threshold.to_string(),
            ),
            (
                "Confirm New Version",
                "CONFIRM_NEW_VERSION",
                if self.confirm_new_version {
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            ),
            (
                "Auto Update",
                "AUTO_UPDATE",
                match self.auto_update {
                    Some(true) => "enabled".into(),
                    Some(false) => "disabled".into(),
                    None => "(not set)".into(),
                },
            ),
            (
                "Fallback Enabled",
                "FALLBACK_ENABLED",
                if self.fallback_enabled {
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            ),
            (
                "Track Generated Commits",
                "TRACK_GENERATED_COMMITS",
                if self.track_generated_commits {
                    "enabled".into()
                } else {
                    "disabled".into()
                },
            ),
        ]
    }

    /// Field groups for the interactive config UI
    pub fn grouped_fields(&self) -> Vec<FieldGroup> {
        let fields = self.fields_display();
        let field_map: std::collections::HashMap<&str, (&'static str, String)> = fields
            .iter()
            .map(|(name, suffix, val)| (*suffix, (*name, val.clone())))
            .collect();

        let basic_keys: &[&'static str] = &["PROVIDER", "MODEL", "API_KEY", "API_URL"];
        let llm_keys: &[&'static str] = &[
            "API_HEADERS",
            "LOCALE",
            "LLM_SYSTEM_PROMPT",
            "COMMIT_TEMPLATE",
            "FALLBACK_ENABLED",
        ];
        let commit_keys: &[&'static str] = &[
            "ONE_LINER",
            "USE_GITMOJI",
            "GITMOJI_FORMAT",
            "REVIEW_COMMIT",
            "TRACK_GENERATED_COMMITS",
        ];
        let post_commit_keys: &[&'static str] = &["POST_COMMIT_PUSH", "SUPPRESS_TOOL_OUTPUT"];
        let warnings_keys: &[&'static str] = &[
            "WARN_STAGED_FILES_ENABLED",
            "WARN_STAGED_FILES_THRESHOLD",
            "CONFIRM_NEW_VERSION",
            "AUTO_UPDATE",
        ];

        let collect =
            |keys: &[&'static str]| -> Vec<(&'static str, &'static str, String)> {
                keys.iter()
                    .filter_map(|k| {
                        field_map
                            .get(k)
                            .map(|(name, val)| (*name, *k, val.clone()))
                    })
                    .collect()
            };

        vec![
            FieldGroup {
                name: "Basic",
                fields: collect(basic_keys),
                subgroups: vec![],
            },
            FieldGroup {
                name: "Advanced",
                fields: vec![],
                subgroups: vec![
                    FieldSubgroup {
                        name: "LLM Settings",
                        fields: collect(llm_keys),
                    },
                    FieldSubgroup {
                        name: "Commit Behavior",
                        fields: collect(commit_keys),
                    },
                    FieldSubgroup {
                        name: "Post-Commit",
                        fields: collect(post_commit_keys),
                    },
                    FieldSubgroup {
                        name: "Warnings & Updates",
                        fields: collect(warnings_keys),
                    },
                ],
            },
        ]
    }

    /// Set a field by its env suffix
    pub fn set_field(&mut self, suffix: &str, value: &str) -> Result<()> {
        match suffix {
            "PROVIDER" => self.provider = value.into(),
            "MODEL" => self.model = value.into(),
            "API_KEY" => self.api_key = value.into(),
            "API_URL" => self.api_url = value.into(),
            "API_HEADERS" => self.api_headers = value.into(),
            "LOCALE" => {
                let locale = normalize_locale(value);
                validate_locale(&locale)?;
                self.locale = locale;
            }
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
            "CONFIRM_NEW_VERSION" => {
                self.confirm_new_version = value == "1" || value.eq_ignore_ascii_case("true");
            }
            "AUTO_UPDATE" => {
                self.auto_update = Some(value == "1" || value.eq_ignore_ascii_case("true"));
            }
            "FALLBACK_ENABLED" => {
                self.fallback_enabled = value == "1" || value.eq_ignore_ascii_case("true");
            }
            "TRACK_GENERATED_COMMITS" => {
                self.track_generated_commits = value == "1" || value.eq_ignore_ascii_case("true");
            }
            _ => {}
        }
        Ok(())
    }

    fn ensure_valid_locale(&mut self) -> Result<()> {
        self.locale = normalize_locale(&self.locale);
        validate_locale(&self.locale)
    }
}

/// Global config file path
pub fn global_config_path() -> Option<PathBuf> {
    if let Some(override_dir) = std::env::var_os("ACR_CONFIG_HOME") {
        let override_path = PathBuf::from(override_dir);
        if !override_path.as_os_str().is_empty() {
            return Some(override_path.join("cgen").join("config.toml"));
        }
    }
    dirs::config_dir().map(|d| d.join("cgen").join("config.toml"))
}

/// Save only the auto_update preference to global config without overwriting other fields
pub fn save_auto_update_preference(value: bool) -> Result<()> {
    let path = global_config_path().context("Could not determine global config directory")?;

    let mut table: toml::Table = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        content.parse().unwrap_or_default()
    } else {
        toml::Table::new()
    };

    table.insert("auto_update".to_string(), toml::Value::Boolean(value));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(&table).context("Failed to serialize config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
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

fn normalize_locale(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        default_locale()
    } else {
        normalized.to_ascii_lowercase()
    }
}

fn validate_locale(locale: &str) -> Result<()> {
    if locale == "en" || locale_has_i18n(locale) {
        return Ok(());
    }
    anyhow::bail!(
        "Unsupported locale '{}'. Only 'en' is available unless matching i18n resources exist. Set locale with `cgen config` or add i18n files first.",
        locale
    );
}

fn locale_has_i18n(locale: &str) -> bool {
    locale_i18n_dirs()
        .iter()
        .any(|dir| locale_exists_in_i18n_dir(dir, locale))
}

fn locale_i18n_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(repo_root) = crate::git::find_repo_root() {
        dirs.push(PathBuf::from(repo_root).join("i18n"));
    }
    if let Ok(current_dir) = std::env::current_dir() {
        let i18n_dir = current_dir.join("i18n");
        if !dirs.contains(&i18n_dir) {
            dirs.push(i18n_dir);
        }
    }
    dirs
}

fn locale_exists_in_i18n_dir(i18n_dir: &PathBuf, locale: &str) -> bool {
    if !i18n_dir.exists() {
        return false;
    }
    if i18n_dir.join(locale).is_dir() {
        return true;
    }

    let entries = match std::fs::read_dir(i18n_dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    entries.filter_map(|entry| entry.ok()).any(|entry| {
        let path = entry.path();
        if path.is_file() {
            return path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.eq_ignore_ascii_case(locale))
                .unwrap_or(false);
        }
        false
    })
}

/// Get description for a field by its env suffix
pub fn field_description(suffix: &str) -> &'static str {
    match suffix {
        "PROVIDER" => "LLM provider (gemini, openai, anthropic, groq, or custom)",
        "MODEL" => "Model identifier for the selected provider",
        "API_KEY" => "API key for authenticating with the LLM provider",
        "API_URL" => "Custom API endpoint URL (leave empty to use provider default)",
        "API_HEADERS" => "Additional HTTP headers for API requests (JSON format)",
        "LOCALE" => "Language locale for commit messages (e.g., en, pt-br)",
        "ONE_LINER" => "Generate single-line commit messages when enabled",
        "COMMIT_TEMPLATE" => "Template for commit message ($msg is replaced with generated text)",
        "LLM_SYSTEM_PROMPT" => "System prompt sent to the LLM for context",
        "USE_GITMOJI" => "Prepend gitmoji to commit messages when enabled",
        "GITMOJI_FORMAT" => "Gitmoji style: unicode (🎨) or shortcode (:art:)",
        "REVIEW_COMMIT" => "Review and approve commit message before creating commit",
        "POST_COMMIT_PUSH" => "Push behavior after commit: ask, always, or never",
        "SUPPRESS_TOOL_OUTPUT" => "Hide git command output when enabled",
        "WARN_STAGED_FILES_ENABLED" => "Warn when staged file count exceeds threshold",
        "WARN_STAGED_FILES_THRESHOLD" => "Number of staged files before warning is shown",
        "CONFIRM_NEW_VERSION" => "Ask for confirmation before creating version tags",
        "AUTO_UPDATE" => "Automatically update cgen when new versions are available",
        "FALLBACK_ENABLED" => "Try fallback presets if primary LLM call fails",
        "TRACK_GENERATED_COMMITS" => "Track commits generated by cgen for history view",
        _ => "",
    }
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
