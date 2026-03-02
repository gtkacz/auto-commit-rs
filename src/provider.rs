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
    default_model: &'static str,
    format: RequestFormat,
    response_path: &'static str,
}

/// Built-in provider definitions
fn get_provider(name: &str) -> Option<ProviderDef> {
    match name {
        "gemini" => Some(ProviderDef {
            api_url: "https://generativelanguage.googleapis.com/v1beta/models/$ACR_MODEL:generateContent?key=$ACR_API_KEY",
            api_headers: "",
            default_model: "gemini-2.0-flash",
            format: RequestFormat::Gemini,
            response_path: "candidates.0.content.parts.0.text",
        }),
        "openai" => Some(ProviderDef {
            api_url: "https://api.openai.com/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "gpt-4o-mini",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "anthropic" => Some(ProviderDef {
            api_url: "https://api.anthropic.com/v1/messages",
            api_headers: "x-api-key: $ACR_API_KEY, anthropic-version: 2023-06-01",
            default_model: "claude-sonnet-4-20250514",
            format: RequestFormat::Anthropic,
            response_path: "content.0.text",
        }),
        "groq" => Some(ProviderDef {
            api_url: "https://api.groq.com/openai/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "llama-3.3-70b-versatile",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "grok" => Some(ProviderDef {
            api_url: "https://api.x.ai/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "grok-3",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "deepseek" => Some(ProviderDef {
            api_url: "https://api.deepseek.com/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "deepseek-chat",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "openrouter" => Some(ProviderDef {
            api_url: "https://openrouter.ai/api/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "openai/gpt-4o-mini",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "mistral" => Some(ProviderDef {
            api_url: "https://api.mistral.ai/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "mistral-small-latest",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "together" => Some(ProviderDef {
            api_url: "https://api.together.xyz/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "fireworks" => Some(ProviderDef {
            api_url: "https://api.fireworks.ai/inference/v1/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "accounts/fireworks/models/llama-v3p3-70b-instruct",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        "perplexity" => Some(ProviderDef {
            api_url: "https://api.perplexity.ai/chat/completions",
            api_headers: "Authorization: Bearer $ACR_API_KEY",
            default_model: "sonar",
            format: RequestFormat::OpenAiCompat,
            response_path: "choices.0.message.content",
        }),
        _ => None,
    }
}

/// Get the default model for a built-in provider, or empty string for unknown providers.
pub fn default_model_for(provider: &str) -> &'static str {
    get_provider(provider).map_or("", |p| p.default_model)
}

pub enum LlmCallError {
    HttpError { code: u16, body: String },
    TransportError(String),
    Other(anyhow::Error),
}

impl std::fmt::Display for LlmCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmCallError::HttpError { code, body } => {
                write!(f, "API returned HTTP {code}: {body}")
            }
            LlmCallError::TransportError(msg) => write!(f, "Network error: {msg}"),
            LlmCallError::Other(e) => write!(f, "{e}"),
        }
    }
}

fn call_llm_inner(cfg: &AppConfig, system_prompt: &str, diff: &str) -> Result<String, LlmCallError> {
    let (url, headers_raw, format, response_path) =
        resolve_provider(cfg).map_err(LlmCallError::Other)?;

    let url = interpolate(&url, cfg);
    let headers_raw = interpolate(&headers_raw, cfg);

    let body = build_request_body(format, &cfg.model, system_prompt, diff);
    let headers = parse_headers(&headers_raw);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} {elapsed}")
            .unwrap(),
    );
    spinner.set_message("Generating commit message...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let mut req = ureq::post(&url);
    for (key, val) in &headers {
        req = req.set(key, val);
    }
    req = req.set("Content-Type", "application/json");

    let response = req.send_json(&body);

    spinner.finish_and_clear();

    let response = match response {
        Ok(resp) => resp,
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            return Err(LlmCallError::HttpError { code, body });
        }
        Err(ureq::Error::Transport(t)) => {
            return Err(LlmCallError::TransportError(t.to_string()));
        }
    };

    let json: Value = response
        .into_json()
        .map_err(|e| LlmCallError::Other(anyhow::anyhow!("Failed to parse API response as JSON: {e}")))?;

    let message = extract_by_path(&json, &response_path).map_err(|e| {
        LlmCallError::Other(anyhow::anyhow!(
            "Failed to extract message from response at path '{}'. Response:\n{}\nError: {}",
            response_path,
            serde_json::to_string_pretty(&json).unwrap_or_default(),
            e
        ))
    })?;

    Ok(message)
}

/// Call LLM with fallback support. Returns (message, fallback_preset_name_if_used).
pub fn call_llm_with_fallback(
    cfg: &AppConfig,
    system_prompt: &str,
    diff: &str,
) -> Result<(String, Option<String>)> {
    match call_llm_inner(cfg, system_prompt, diff) {
        Ok(msg) => Ok((msg, None)),
        Err(LlmCallError::TransportError(msg)) => {
            anyhow::bail!("Network error: {msg}");
        }
        Err(LlmCallError::HttpError { code, body }) => {
            if !cfg.fallback_enabled {
                anyhow::bail!("API returned HTTP {code}: {body}");
            }

            let presets_file = match crate::preset::load_presets() {
                Ok(f) => f,
                Err(_) => anyhow::bail!("API returned HTTP {code}: {body}"),
            };

            if presets_file.fallback.order.is_empty() {
                anyhow::bail!("API returned HTTP {code}: {body}");
            }

            let current_fields = crate::preset::fields_from_config(cfg);
            let mut errors = vec![format!("Primary (HTTP {code})")];

            for &preset_id in &presets_file.fallback.order {
                let preset = match presets_file.presets.iter().find(|p| p.id == preset_id) {
                    Some(p) => p,
                    None => continue,
                };

                // Skip if this preset matches current config (dedup key comparison)
                if preset.fields.provider == current_fields.provider
                    && preset.fields.model == current_fields.model
                    && preset.fields.api_key == current_fields.api_key
                    && preset.fields.api_url == current_fields.api_url
                {
                    continue;
                }

                eprintln!(
                    "{} Primary failed (HTTP {}), trying: {}...",
                    "fallback:".yellow().bold(),
                    code,
                    preset.name
                );

                let mut temp_cfg = cfg.clone();
                crate::preset::apply_preset_to_config(&mut temp_cfg, preset);

                match call_llm_inner(&temp_cfg, system_prompt, diff) {
                    Ok(msg) => return Ok((msg, Some(preset.name.clone()))),
                    Err(LlmCallError::HttpError { code: fc, .. }) => {
                        errors.push(format!("{} (HTTP {fc})", preset.name));
                        continue;
                    }
                    Err(LlmCallError::TransportError(msg)) => {
                        anyhow::bail!("Network error during fallback to '{}': {msg}", preset.name);
                    }
                    Err(LlmCallError::Other(e)) => {
                        errors.push(format!("{} ({})", preset.name, e));
                        continue;
                    }
                }
            }

            anyhow::bail!(
                "All LLM providers failed: {}",
                errors.join(", ")
            );
        }
        Err(LlmCallError::Other(e)) => {
            anyhow::bail!("{e}");
        }
    }
}

/// Call the LLM API and return the generated commit message
pub fn call_llm(cfg: &AppConfig, system_prompt: &str, diff: &str) -> Result<String> {
    let (msg, _) = call_llm_with_fallback(cfg, system_prompt, diff)?;
    Ok(msg)
}

fn resolve_provider(cfg: &AppConfig) -> Result<(String, String, RequestFormat, String)> {
    if let Some(def) = get_provider(&cfg.provider) {
        let url = if cfg.api_url.is_empty() {
            def.api_url.to_string()
        } else {
            cfg.api_url.clone()
        };
        let headers = if cfg.api_headers.is_empty() {
            def.api_headers.to_string()
        } else {
            cfg.api_headers.clone()
        };
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

fn build_request_body(
    format: RequestFormat,
    model: &str,
    system_prompt: &str,
    diff: &str,
) -> Value {
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
            pair.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Walk a JSON value by a dot-separated path like "candidates.0.content.parts.0.text"
fn extract_by_path(value: &Value, path: &str) -> Result<String> {
    let mut current = value;
    for segment in path.split('.') {
        current = if let Ok(index) = segment.parse::<usize>() {
            current
                .get(index)
                .with_context(|| format!("Array index {index} out of bounds"))?
        } else {
            current
                .get(segment)
                .with_context(|| format!("Key '{segment}' not found"))?
        };
    }
    current
        .as_str()
        .map(|s| s.to_string())
        .with_context(|| "Expected string value at path end".to_string())
}
