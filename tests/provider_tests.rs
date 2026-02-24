use auto_commit_rs::config::AppConfig;
use auto_commit_rs::provider;
use mockito::{Matcher, Server};

fn cfg_for(provider_name: &str, api_url: String) -> AppConfig {
    let mut cfg = AppConfig::default();
    cfg.provider = provider_name.to_string();
    cfg.model = "test-model".into();
    cfg.api_key = "test-key".into();
    cfg.api_url = api_url;
    cfg
}

#[test]
fn default_model_for_returns_known_and_unknown_defaults() {
    assert_eq!(provider::default_model_for("openai"), "gpt-4o-mini");
    assert_eq!(provider::default_model_for("groq"), "llama-3.3-70b-versatile");
    assert_eq!(provider::default_model_for("unknown"), "");
}

#[test]
fn call_llm_openai_compat_builds_expected_request() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/openai")
        .match_header("authorization", "Bearer test-key")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Regex(r#""model":"test-model""#.into()))
        .match_body(Matcher::Regex(r#""messages""#.into()))
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"content":"feat: mocked"}}]}"#)
        .create();

    let cfg = cfg_for("openai", format!("{}/openai", server.url()));
    let msg = provider::call_llm(&cfg, "system", "diff").expect("llm call");
    assert_eq!(msg, "feat: mocked");
    mock.assert();
}

#[test]
fn call_llm_gemini_uses_gemini_payload_and_response_path() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/gemini")
        .match_body(Matcher::Regex(r#""system_instruction""#.into()))
        .match_body(Matcher::Regex(r#""generationConfig""#.into()))
        .with_status(200)
        .with_body(r#"{"candidates":[{"content":{"parts":[{"text":"fix: gemini"}]}}]}"#)
        .create();

    let cfg = cfg_for("gemini", format!("{}/gemini", server.url()));
    let msg = provider::call_llm(&cfg, "system", "diff").expect("llm call");
    assert_eq!(msg, "fix: gemini");
    mock.assert();
}

#[test]
fn call_llm_anthropic_uses_anthropic_payload_and_headers() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/anthropic")
        .match_header("x-api-key", "test-key")
        .match_header("anthropic-version", "2023-06-01")
        .match_body(Matcher::Regex(r#""system":"system-prompt""#.into()))
        .with_status(200)
        .with_body(r#"{"content":[{"text":"docs: anthropic"}]}"#)
        .create();

    let cfg = cfg_for("anthropic", format!("{}/anthropic", server.url()));
    let msg = provider::call_llm(&cfg, "system-prompt", "diff").expect("llm call");
    assert_eq!(msg, "docs: anthropic");
    mock.assert();
}

#[test]
fn call_llm_custom_provider_requires_url() {
    let mut cfg = AppConfig::default();
    cfg.provider = "custom-provider".into();
    cfg.api_url.clear();
    cfg.api_key = "k".into();
    let err = provider::call_llm(&cfg, "system", "diff")
        .expect_err("missing custom URL should fail")
        .to_string();
    assert!(err.contains("Unknown provider"));
}

#[test]
fn call_llm_reports_http_status_and_bad_response_path_errors() {
    let mut server = Server::new();
    let status_mock = server
        .mock("POST", "/status")
        .with_status(401)
        .with_body("unauthorized")
        .create();

    let cfg = cfg_for("openai", format!("{}/status", server.url()));
    let status_err = provider::call_llm(&cfg, "system", "diff")
        .expect_err("status failure expected")
        .to_string();
    assert!(status_err.contains("HTTP 401"));
    status_mock.assert();

    let path_mock = server
        .mock("POST", "/missing-path")
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"wrong":"value"}}]}"#)
        .create();
    let cfg2 = cfg_for("openai", format!("{}/missing-path", server.url()));
    let path_err = provider::call_llm(&cfg2, "system", "diff")
        .expect_err("missing response path should fail")
        .to_string();
    assert!(path_err.contains("Failed to extract message"));
    path_mock.assert();
}

#[test]
fn call_llm_interpolates_custom_headers_and_url_variables() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", Matcher::Any)
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"content":"chore: custom"}}]}"#)
        .create();

    let mut cfg = cfg_for("custom", format!("{}/v1/$ACR_MODEL", server.url()));
    cfg.api_headers = "Authorization: Bearer $ACR_API_KEY, X-Model: $ACR_MODEL".into();
    cfg.model = "chat".into();

    let msg = provider::call_llm(&cfg, "system", "diff").expect("llm call");
    assert_eq!(msg, "chore: custom");
    mock.assert();
}
