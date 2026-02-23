# cgen

Minimal CLI tool that generates git commit messages via LLMs.

Fast, lightweight, cross-platform. No runtime dependencies — just a single binary.

## Why Rust?

Tools like [opencommit](https://github.com/di-sukharev/opencommit) do the same thing but require Node.js and weigh in at **~100MB** of `node_modules`. cgen is a single **~2MB** static binary. No runtimes, no interpreters, no package managers — download and run.

| | cgen | opencommit |
|---|---|---|
| Install size | ~2 MB | ~100 MB |
| Runtime deps | None | Node.js |
| Startup time | Instant | ~300ms (Node cold start) |
| Distribution | Single binary | npm install |

## Install

### Linux / macOS (curl)

```sh
curl -fsSL https://raw.githubusercontent.com/gtkacz/rust-auto-commit/main/scripts/install.sh | bash
```

This detects your OS and architecture, downloads the latest release binary to `/usr/local/bin`, and makes it executable. Set `INSTALL_DIR` to change the target:

```sh
INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/gtkacz/rust-auto-commit/main/scripts/install.sh | bash
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/gtkacz/rust-auto-commit/main/scripts/install.ps1 | iex
```

This downloads the latest release to `%LOCALAPPDATA%\cgen\` and adds it to your user PATH.

### Cargo

```sh
cargo install --git https://github.com/gtkacz/rust-auto-commit
```

### Manual Download

Grab a binary from the [Releases](https://github.com/gtkacz/rust-auto-commit/releases) page and place it somewhere in your PATH.

Available binaries:
- `cgen-linux-amd64` — Linux x86_64
- `cgen-macos-amd64` — macOS Intel
- `cgen-macos-arm64` — macOS Apple Silicon
- `cgen-windows-amd64.exe` — Windows x86_64

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

Any arguments passed to `cgen` (without a subcommand) are forwarded directly to `git commit`.

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

## Contributing

Contributions are welcome! Whether it's a new provider (often just 5 lines), a bug fix, or a documentation improvement — every bit helps.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide, including how to set up the development environment and how to add a new default provider step-by-step.

## License

[MIT](LICENSE)
