use anyhow::{bail, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::time::Duration;

use crate::config::AppConfig;
use crate::interpolation::interpolate;

#[derive(Debug, Clone, Copy, PartialEq)]
enum RequestFormat {
    Gemini,
    OpenAiCompat,
    Anthropic,
}

struct ProviderDef {
    api_url: &'static str,
    api_headers: &'static str,
    format: RequestFormat,
    response_path: &'static str,
}

/// Built-in provider definitions
fn get_provider(name: &str) -> Option<ProviderDef> {
    match name {
        "gemini" => Some(ProviderDef {
            api_url: "https://generativelanguage.googleapis.com/v1beta/models/$ACR_MODEL:generateContent?key=$ACR_API_KEY",
            api_headers: "",
            format: RequestFormat::Gemini,
            response_path: "candidates.0.content.parts.0.text",
        }),
        "openai" => Some(ProviderDef {
            api_url: "https://api.openai.com/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "anthropic" => Some(ProviderDef {
            api_url: "https://api.anthropic.com/v1/messages",
            api_headers: "x-api-key: $ACR_API_KEY, anthropic-version: 2023-06-01",
            format: RequestFormat::Anthropic,
            response_path: "content.0.text",
        }),
        _ => None,
    }
}

/// Call the LLM API and return the generated commit message
pub fn call_llm(cfg: &AppConfig, system_prompt: &str, diff: &str) -> Result<String> {
    let (url, headers_raw, format, response_path) = resolve_provider(cfg)?;

    let url = interpolate(&url, cfg);
    let headers_raw = interpolate(&headers_raw, cfg);

    let body = build_request_body(format, &cfg.model, system_prompt, diff);

    let headers = parse_headers(&headers_raw);

    // Spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} {elapsed}")
            .unwrap(),
    );
    spinner.set_message("Generating commit message...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    // HTTP request
    let mut req = ureq::post(&url);
    for (key, val) in &headers {
        req = req.set(key, val);
    }
    req = req.set("Content-Type", "application/json");

    let response = req.send_json(&body);

    spinner.finish_and_clear();

    let response = response.map_err(|e| {
        match e {
            ureq::Error::Status(code, resp) => {
                let body = resp.into_string().unwrap_or_default();
                anyhow::anyhow!("API returned HTTP {code}: {body}")
            }
            ureq::Error::Transport(t) => {
                anyhow::anyhow!("Network error: {t}")
            }
        }
    })?;

    let json: Value = response.into_json()
        .context("Failed to parse API response as JSON")?;

    let message = extract_by_path(&json, &response_path)
        .with_context(|| {
            format!(
                "Failed to extract message from response at path '{}'. Response:\n{}",
                response_path,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )
        })?;

    eprintln!("{} {}", "Commit message:".green().bold(), message.trim());

    Ok(message)
}

fn resolve_provider(cfg: &AppConfig) -> Result<(String, String, RequestFormat, String)> {
    if let Some(def) = get_provider(&cfg.provider) {
        let url = if cfg.api_url.is_empty() { def.api_url.to_string() } else { cfg.api_url.clone() };
        let headers = if cfg.api_headers.is_empty() { def.api_headers.to_string() } else { cfg.api_headers.clone() };
        Ok((url, headers, def.format, def.response_path.to_string()))
    } else {
        // Custom provider: require API URL, default to OpenAI-compatible format
        if cfg.api_url.is_empty() {
            bail!(
                "Unknown provider '{}'. Set {} for custom providers.",
                cfg.provider.yellow(),
                "ACR_API_URL".yellow()
            );
        }
        Ok((
            cfg.api_url.clone(),
            cfg.api_headers.clone(),
            RequestFormat::OpenAiCompat,
            "choices.0.message.content".to_string(),
        ))
    }
}

fn build_request_body(format: RequestFormat, model: &str, system_prompt: &str, diff: &str) -> Value {
    match format {
        RequestFormat::Gemini => {
            serde_json::json!({
                "system_instruction": {
                    "parts": [{ "text": system_prompt }]
                },
                "contents": [{
                    "role": "user",
                    "parts": [{ "text": diff }]
                }],
                "generationConfig": {
                    "temperature": 0
                }
            })
        }
        RequestFormat::OpenAiCompat => {
            serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": diff }
                ],
                "max_tokens": 512,
                "temperature": 0
            })
        }
        RequestFormat::Anthropic => {
            serde_json::json!({
                "model": model,
                "system": system_prompt,
                "messages": [
                    { "role": "user", "content": diff }
                ],
                "max_tokens": 512
            })
        }
    }
}

/// Parse "Key: Value, Key2: Value2" header string into pairs
fn parse_headers(raw: &str) -> Vec<(String, String)> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .filter_map(|pair| {
            let pair = pair.trim();
            pair.split_once(':').map(|(k, v)| {
                (k.trim().to_string(), v.trim().to_string())
            })
        })
        .collect()
}

/// Walk a JSON value by a dot-separated path like "candidates.0.content.parts.0.text"
fn extract_by_path(value: &Value, path: &str) -> Result<String> {
    let mut current = value;
    for segment in path.split('.') {
        current = if let Ok(index) = segment.parse::<usize>() {
            current.get(index)
                .with_context(|| format!("Array index {index} out of bounds"))?
        } else {
            current.get(segment)
                .with_context(|| format!("Key '{segment}' not found"))?
        };
    }
    current
        .as_str()
        .map(|s| s.to_string())
        .with_context(|| "Expected string value at path end".to_string())
}
