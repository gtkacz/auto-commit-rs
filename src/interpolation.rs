use crate::config::AppConfig;
use regex_lite::Regex;

/// Interpolate `$VARIABLE_NAME` patterns in a string using environment variables.
/// Before interpolation, ACR_ config values are temporarily set as env vars.
pub fn interpolate(template: &str, cfg: &AppConfig) -> String {
    // Temporarily set ACR_ env vars from config so $ACR_MODEL etc. resolve
    let env_pairs = [
        ("ACR_PROVIDER", &cfg.provider),
        ("ACR_MODEL", &cfg.model),
        ("ACR_API_KEY", &cfg.api_key),
        ("ACR_LOCALE", &cfg.locale),
    ];
    for (key, val) in &env_pairs {
        std::env::set_var(key, val);
    }

    let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    let result = re.replace_all(template, |caps: &regex_lite::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).unwrap_or_default()
    });

    result.into_owned()
}
