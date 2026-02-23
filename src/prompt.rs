use crate::config::AppConfig;

const CONVENTIONAL_COMMIT_SPEC: &str = "\
Follow the Conventional Commits specification:
- Prefix with a type: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
- Optionally add a scope in parentheses: feat(parser):
- Follow with a colon and space, then a short description
- Examples: feat: add user login, fix(api): handle null response, docs: update README";

const GITMOJI_UNICODE_SPEC: &str = "\
Use Gitmoji: start the commit message with a relevant emoji in unicode format.
Examples: \u{26a1}\u{fe0f} Improve performance, \u{1f41b} Fix bug, \u{2728} Add new feature, \
\u{267b}\u{fe0f} Refactor code, \u{1f4dd} Update docs, \u{1f3a8} Improve UI";

const GITMOJI_SHORTCODE_SPEC: &str = "\
Use Gitmoji: start the commit message with a relevant emoji in :shortcode: format.
Examples: :zap: Improve performance, :bug: Fix bug, :sparkles: Add new feature, \
:recycle: Refactor code, :memo: Update docs, :art: Improve UI";

/// Build the full system prompt from config flags
pub fn build_system_prompt(cfg: &AppConfig) -> String {
    let mut parts = Vec::new();

    // Base prompt (user-overridable)
    parts.push(cfg.llm_system_prompt.clone());

    // Conventional commits
    parts.push(CONVENTIONAL_COMMIT_SPEC.to_string());

    // Gitmoji
    if cfg.use_gitmoji {
        let spec = match cfg.gitmoji_format.as_str() {
            "shortcode" => GITMOJI_SHORTCODE_SPEC,
            _ => GITMOJI_UNICODE_SPEC,
        };
        parts.push(spec.to_string());
    }

    // One-liner
    if cfg.one_liner {
        parts.push("Output ONLY a single line. No body, no footer, no explanations.".to_string());
    }

    // Locale
    if cfg.locale != "en" {
        parts.push(format!("Write the commit message in the '{}' locale.", cfg.locale));
    }

    // Universal closing instructions
    parts.push("Use present tense. Be concise. Output only the raw commit message, nothing else.".to_string());

    parts.join("\n\n")
}
