# cgen

Minimal CLI tool that generates git commit messages via LLMs.

Fast, lightweight (~3-5MB binary), cross-platform. No runtime dependencies.

## Install

**Download a binary** from [Releases](../../releases) — no Rust needed.

Or with Cargo:

```sh
cargo install --git https://github.com/YOUR_USER/auto-commit-rs
```

## Quick Start

```sh
# 1. Set your API key (one-time)
cgen config
# or: export ACR_API_KEY=your-key-here

# 2. Stage files and generate commit
git add .
cgen
```

## Usage

```
cgen                    # Generate commit message and commit
cgen --no-verify        # Forward flags to git commit
cgen config             # Interactive config editor (local .env)
cgen config --global    # Interactive config editor (global TOML)
```

## Configuration

All settings use the `ACR_` prefix. Layered resolution: defaults → global TOML → local `.env` → env vars.

| Variable | Default | Description |
|----------|---------|-------------|
| `ACR_PROVIDER` | `gemini` | LLM provider (`gemini`, `openai`, `anthropic`, or custom) |
| `ACR_MODEL` | `gemini-2.0-flash` | Model name |
| `ACR_API_KEY` | — | API key (required) |
| `ACR_API_URL` | auto | API endpoint (auto-resolved from provider) |
| `ACR_API_HEADERS` | auto | Custom headers (`Key: Value, Key2: Value2`) |
| `ACR_LOCALE` | `en` | Commit message language |
| `ACR_ONE_LINER` | `1` | Single-line commits (`1`/`0`) |
| `ACR_COMMIT_TEMPLATE` | `$msg` | Template — `$msg` is replaced with LLM output |
| `ACR_LLM_SYSTEM_PROMPT` | (built-in) | Base system prompt |
| `ACR_USE_GITMOJI` | `0` | Enable gitmoji (`1`/`0`) |
| `ACR_GITMOJI_FORMAT` | `unicode` | Gitmoji style (`unicode`/`shortcode`) |

### Config Locations

- **Global**: `~/.config/cgen/config.toml` (Linux), `~/Library/Application Support/cgen/config.toml` (macOS), `%APPDATA%\cgen\config.toml` (Windows)
- **Local**: `.env` in git repo root

### Variable Interpolation

`ACR_API_URL` and `ACR_API_HEADERS` support `$VARIABLE` interpolation from environment variables:

```sh
ACR_API_URL=https://api.example.com/v1/$ACR_MODEL/chat
ACR_API_HEADERS=Authorization: Bearer $ACR_API_KEY, X-Custom: $MY_HEADER
```

## Providers

Built-in providers: **Gemini** (default), **OpenAI**, **Anthropic**.

For custom providers, set `ACR_PROVIDER` to any name and provide `ACR_API_URL`. Custom providers default to OpenAI-compatible request format.

```sh
export ACR_PROVIDER=ollama
export ACR_API_URL=http://localhost:11434/v1/chat/completions
export ACR_MODEL=llama3
```

## License

MIT
