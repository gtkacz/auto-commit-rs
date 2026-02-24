mod common;

use std::fs;

use auto_commit_rs::config::{global_config_path, AppConfig};
use serial_test::serial;

use crate::common::{DirGuard, EnvGuard};

fn acr_env_keys() -> [&'static str; 18] {
    [
        "ACR_CONFIG_HOME",
        "ACR_PROVIDER",
        "ACR_MODEL",
        "ACR_API_KEY",
        "ACR_API_URL",
        "ACR_API_HEADERS",
        "ACR_LOCALE",
        "ACR_ONE_LINER",
        "ACR_COMMIT_TEMPLATE",
        "ACR_LLM_SYSTEM_PROMPT",
        "ACR_USE_GITMOJI",
        "ACR_GITMOJI_FORMAT",
        "ACR_REVIEW_COMMIT",
        "ACR_POST_COMMIT_PUSH",
        "ACR_SUPPRESS_TOOL_OUTPUT",
        "ACR_WARN_STAGED_FILES_ENABLED",
        "ACR_WARN_STAGED_FILES_THRESHOLD",
        "ACR_CONFIRM_NEW_VERSION",
    ]
}

#[test]
#[serial]
fn load_uses_defaults_when_no_layers_exist() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let cfg_dir = tempfile::TempDir::new().expect("tempdir");

    let _env = EnvGuard::set(&[
        ("ACR_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("XDG_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("APPDATA", cfg_dir.path().to_string_lossy().as_ref()),
    ]);
    let _acr = EnvGuard::clear(&acr_env_keys());

    let _force = EnvGuard::set(&[
        ("ACR_PROVIDER", "groq"),
        ("ACR_MODEL", "llama-3.3-70b-versatile"),
        ("ACR_LOCALE", "en"),
        ("ACR_POST_COMMIT_PUSH", "ask"),
        ("ACR_WARN_STAGED_FILES_THRESHOLD", "20"),
        ("ACR_CONFIRM_NEW_VERSION", "1"),
    ]);
    let cfg = AppConfig::load().expect("config should load");
    assert_eq!(cfg.provider, "groq");
    assert_eq!(cfg.model, "llama-3.3-70b-versatile");
    assert_eq!(cfg.locale, "en");
    assert!(cfg.one_liner);
    assert_eq!(cfg.post_commit_push, "ask");
    assert_eq!(cfg.warn_staged_files_threshold, 20);
    assert!(cfg.confirm_new_version);
}

#[test]
#[serial]
fn load_applies_global_then_local_then_env_precedence() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let cfg_dir = tempfile::TempDir::new().expect("tempdir");

    let _env = EnvGuard::set(&[
        ("ACR_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("XDG_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("APPDATA", cfg_dir.path().to_string_lossy().as_ref()),
    ]);
    let _acr = EnvGuard::clear(&acr_env_keys());

    let global_path = global_config_path().expect("global path");
    fs::create_dir_all(
        global_path
            .parent()
            .expect("global config parent should be present"),
    )
    .expect("create global config dir");
    fs::write(
        &global_path,
        r#"
provider = "openai"
model = "gpt-4o-mini"
locale = "en"
post_commit_push = "always"
warn_staged_files_threshold = 7
"#,
    )
    .expect("write global config");

    fs::write(
        repo.path().join(".env"),
        r#"
# local override
ACR_PROVIDER=anthropic
ACR_MODEL=claude-local
ACR_POST_COMMIT_PUSH=never
ACR_WARN_STAGED_FILES_THRESHOLD=13
ACR_API_HEADERS='X-Foo: bar'
"#,
    )
    .expect("write local env");

    let _env_overrides = EnvGuard::set(&[
        ("ACR_MODEL", "env-model"),
        ("ACR_POST_COMMIT_PUSH", "invalid-value"),
        ("ACR_WARN_STAGED_FILES_THRESHOLD", "not-a-number"),
        ("ACR_USE_GITMOJI", "true"),
    ]);

    let cfg = AppConfig::load().expect("config should load");
    assert_eq!(cfg.provider, "anthropic");
    assert_eq!(cfg.model, "env-model");
    assert_eq!(cfg.post_commit_push, "ask");
    assert_eq!(cfg.warn_staged_files_threshold, 20);
    assert!(cfg.use_gitmoji);
    assert_eq!(cfg.api_headers, "X-Foo: bar");
}

#[test]
#[serial]
fn save_local_writes_normalized_env_file() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let mut cfg = AppConfig::default();
    cfg.provider = "gemini".into();
    cfg.model = "gemini-2.0-flash".into();
    cfg.api_key = "secret-key-value".into();
    cfg.locale = "pl".into();
    cfg.post_commit_push = "unexpected".into();
    cfg.warn_staged_files_enabled = false;
    cfg.warn_staged_files_threshold = 42;
    cfg.confirm_new_version = false;

    cfg.save_local().expect("save local config");

    let env_content = fs::read_to_string(repo.path().join(".env")).expect("read .env");
    assert!(env_content.contains("ACR_PROVIDER=gemini"));
    assert!(env_content.contains("ACR_MODEL=gemini-2.0-flash"));
    assert!(env_content.contains("ACR_API_KEY=secret-key-value"));
    assert!(env_content.contains("ACR_POST_COMMIT_PUSH=ask"));
    assert!(env_content.contains("ACR_WARN_STAGED_FILES_ENABLED=0"));
    assert!(env_content.contains("ACR_WARN_STAGED_FILES_THRESHOLD=42"));
    assert!(env_content.contains("ACR_CONFIRM_NEW_VERSION=0"));
}

#[test]
fn set_field_parses_boolean_and_numeric_values() {
    let mut cfg = AppConfig::default();
    cfg.set_field("ONE_LINER", "0").expect("set one-liner");
    cfg.set_field("USE_GITMOJI", "true")
        .expect("set use gitmoji");
    cfg.set_field("WARN_STAGED_FILES_THRESHOLD", "15")
        .expect("set warning threshold");
    cfg.set_field("WARN_STAGED_FILES_THRESHOLD", "invalid")
        .expect("set invalid warning threshold");
    cfg.set_field("POST_COMMIT_PUSH", "ALWAYS")
        .expect("set post commit push");

    assert!(!cfg.one_liner);
    assert!(cfg.use_gitmoji);
    assert_eq!(cfg.warn_staged_files_threshold, 20);
    assert_eq!(cfg.post_commit_push, "always");
}

#[test]
#[serial]
fn load_errors_when_locale_has_no_i18n_resources() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let cfg_dir = tempfile::TempDir::new().expect("tempdir");

    let _env = EnvGuard::set(&[
        ("ACR_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("XDG_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("APPDATA", cfg_dir.path().to_string_lossy().as_ref()),
    ]);
    let _acr = EnvGuard::clear(&acr_env_keys());
    let _set_locale = EnvGuard::set(&[("ACR_LOCALE", "pl")]);

    let err = AppConfig::load().expect_err("expected locale validation error");
    assert!(
        err.to_string().contains("Unsupported locale"),
        "unexpected error: {err:#}"
    );
}

#[test]
#[serial]
fn load_accepts_non_english_locale_when_i18n_exists() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let cfg_dir = tempfile::TempDir::new().expect("tempdir");

    let _env = EnvGuard::set(&[
        ("ACR_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("XDG_CONFIG_HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("HOME", cfg_dir.path().to_string_lossy().as_ref()),
        ("APPDATA", cfg_dir.path().to_string_lossy().as_ref()),
    ]);
    let _acr = EnvGuard::clear(&acr_env_keys());
    let _set_locale = EnvGuard::set(&[("ACR_LOCALE", "pl")]);

    fs::create_dir_all(repo.path().join("i18n")).expect("create i18n dir");
    fs::write(repo.path().join("i18n").join("pl.toml"), "title = 'Polski'")
        .expect("write locale resource");

    let cfg = AppConfig::load().expect("config should load with i18n locale");
    assert_eq!(cfg.locale, "pl");
}

#[test]
fn fields_display_masks_api_key_and_shows_helpers() {
    let mut cfg = AppConfig::default();
    cfg.api_key = "abcd1234efgh5678".into();
    cfg.api_url.clear();
    cfg.api_headers.clear();

    let fields = cfg.fields_display();
    let api_key = fields
        .iter()
        .find(|(name, _, _)| *name == "API Key")
        .expect("api key field");
    let api_url = fields
        .iter()
        .find(|(name, _, _)| *name == "API URL")
        .expect("api url field");

    assert_eq!(api_key.2, "abcd...5678");
    assert_eq!(api_url.2, "(auto from provider)");
}
