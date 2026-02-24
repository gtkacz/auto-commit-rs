use auto_commit_rs::update::parse_semver;

#[test]
fn parse_semver_handles_plain_versions() {
    assert_eq!(parse_semver("1.0.0"), Some((1, 0, 0)));
    assert_eq!(parse_semver("0.1.0"), Some((0, 1, 0)));
    assert_eq!(parse_semver("12.34.56"), Some((12, 34, 56)));
}

#[test]
fn parse_semver_strips_v_prefix() {
    assert_eq!(parse_semver("v1.0.0"), Some((1, 0, 0)));
    assert_eq!(parse_semver("v0.2.3"), Some((0, 2, 3)));
}

#[test]
fn parse_semver_rejects_invalid_formats() {
    assert_eq!(parse_semver(""), None);
    assert_eq!(parse_semver("1.0"), None);
    assert_eq!(parse_semver("1.0.0.0"), None);
    assert_eq!(parse_semver("abc"), None);
    assert_eq!(parse_semver("v1.x.0"), None);
    assert_eq!(parse_semver("not-a-version"), None);
}

#[test]
fn semver_comparison_works_for_update_detection() {
    let current = parse_semver("1.0.0").unwrap();
    let newer_patch = parse_semver("1.0.1").unwrap();
    let newer_minor = parse_semver("1.1.0").unwrap();
    let newer_major = parse_semver("2.0.0").unwrap();
    let same = parse_semver("1.0.0").unwrap();
    let older = parse_semver("0.9.0").unwrap();

    assert!(newer_patch > current);
    assert!(newer_minor > current);
    assert!(newer_major > current);
    assert!(!(same > current));
    assert!(!(older > current));
}

#[test]
fn current_version_is_valid_semver() {
    let version = auto_commit_rs::update::current_version();
    assert!(
        parse_semver(version).is_some(),
        "CARGO_PKG_VERSION '{}' should be valid semver",
        version
    );
}
