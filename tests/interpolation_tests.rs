mod common;

use auto_commit_rs::config::AppConfig;
use auto_commit_rs::interpolation::interpolate;
use serial_test::serial;

use crate::common::EnvGuard;

#[test]
#[serial]
fn interpolate_replaces_known_variables_and_keeps_literals() {
    let mut cfg = AppConfig::default();
    cfg.provider = "openai".into();
    cfg.model = "gpt-4o-mini".into();
    cfg.api_key = "secret".into();
    cfg.locale = "en".into();

    let _env = EnvGuard::set(&[("CUSTOM_ENV", "custom")]);
    let result = interpolate(
        "provider=$ACR_PROVIDER model=$ACR_MODEL key=$ACR_API_KEY custom=$CUSTOM_ENV",
        &cfg,
    );

    assert_eq!(
        result,
        "provider=openai model=gpt-4o-mini key=secret custom=custom"
    );
}

#[test]
#[serial]
fn interpolate_replaces_unknown_variables_with_empty_string() {
    let cfg = AppConfig::default();
    let _env = EnvGuard::clear(&["DOES_NOT_EXIST"]);
    let result = interpolate("before:$DOES_NOT_EXIST:after", &cfg);
    assert_eq!(result, "before::after");
}

#[test]
#[serial]
fn interpolate_overrides_acr_variables_from_config_values() {
    let mut cfg = AppConfig::default();
    cfg.model = "model-from-config".into();
    let _env = EnvGuard::set(&[("ACR_MODEL", "model-from-env")]);
    let result = interpolate("model=$ACR_MODEL", &cfg);
    assert_eq!(result, "model=model-from-config");
}
