use crate::config::AppConfig;

const CONVENTIONAL_COMMIT_SPEC: &str = "\
Write all commit messages strictly following the Conventional Commits specification.

Use the following format:
<type>[optional scope][optional !]: <description>

[optional body]

[optional footer(s)]

Rules to follow:
1. Type: MUST be a noun. Use `feat` for new features, `fix` for bug fixes, or other relevant types (e.g., `docs`, `chore`, `refactor`).
2. Scope: OPTIONAL. A noun describing the affected section of the codebase, enclosed in parentheses (e.g., `fix(parser):`).
3. Description: REQUIRED. A concise summary immediately following the type/scope, colon, and space.
4. Body: OPTIONAL. Provide additional context. MUST begin one blank line after the description.
5. Footer: OPTIONAL. MUST begin one blank line after the body. Use token-value pairs (e.g., `Reviewed-by: Name`). Token words must be hyphenated.
6. Breaking Changes: MUST be indicated by either an exclamation mark `!` immediately before the colon (e.g., `feat!:`) OR an uppercase `BREAKING CHANGE: <description>` in the footer.";

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
        parts.push("Craft a concise, single sentence, commit message that encapsulates all changes made, with an emphasis on the primary updates. If the modifications share a common theme or scope, mention it succinctly; otherwise, leave the scope out to maintain focus. The goal is to provide a clear and unified overview of the changes in one single message.".to_string());
    }

    // Locale
    if cfg.locale != "en" {
        parts.push(format!(
            "Write the commit message in the '{}' locale.",
            cfg.locale
        ));
    }

    // Universal closing instructions
    parts.push(
        "Use present tense. Be concise. Output only the raw commit message, nothing else."
            .to_string(),
    );

    parts.join("\n\n")
}
