mod common;

use auto_commit_rs::git;
use serial_test::serial;

use crate::common::{commit_file, git_ok, git_stdout, write_file, DirGuard};

#[test]
fn compute_next_minor_tag_handles_valid_and_invalid_input() {
    assert_eq!(
        git::compute_next_minor_tag(None).expect("default version"),
        "0.1.0"
    );
    assert_eq!(
        git::compute_next_minor_tag(Some("1.4.9")).expect("increment"),
        "1.5.0"
    );
    assert!(git::compute_next_minor_tag(Some("v1.2.3")).is_err());
    assert!(git::compute_next_minor_tag(Some("1.2")).is_err());
}

#[test]
#[serial]
fn staged_diff_and_files_behave_for_empty_and_non_empty_index() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());

    assert!(git::get_staged_diff().is_err());
    assert!(git::list_staged_files().expect("list staged files").is_empty());

    write_file(&repo.path().join("src.txt"), "hello");
    git_ok(repo.path(), ["add", "src.txt"]);

    let files = git::list_staged_files().expect("list staged files");
    assert_eq!(files, vec!["src.txt".to_string()]);
    let diff = git::get_staged_diff().expect("staged diff");
    assert!(diff.contains("src.txt"));
}

#[test]
#[serial]
fn commit_and_undo_roundtrip() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());

    write_file(&repo.path().join("a.txt"), "1");
    git_ok(repo.path(), ["add", "a.txt"]);
    git::run_commit("test: add a", &[], true).expect("commit should succeed");
    write_file(&repo.path().join("a.txt"), "2");
    git_ok(repo.path(), ["add", "a.txt"]);
    git::run_commit("test: update a", &[], true).expect("second commit should succeed");
    git::ensure_head_exists().expect("head exists");

    assert_eq!(
        git_stdout(repo.path(), ["rev-list", "--count", "HEAD"]),
        "2".to_string()
    );

    git::undo_last_commit_soft(true).expect("undo should succeed");
    assert!(git::ensure_head_exists().is_ok());
    assert_eq!(
        git_stdout(repo.path(), ["diff", "--cached", "--name-only"]),
        "a.txt".to_string()
    );
}

#[test]
#[serial]
fn latest_tag_and_create_tag_work() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let _first = commit_file(repo.path(), "a.txt", "a", "feat: first");

    assert!(git::get_latest_tag().expect("latest tag").is_none());
    git::create_tag("0.1.0", true).expect("create tag");
    git::create_tag("0.2.0", true).expect("create tag");

    assert_eq!(
        git::get_latest_tag().expect("latest tag"),
        Some("0.2.0".to_string())
    );
}

#[test]
#[serial]
fn commit_and_range_diff_and_head_checks_work() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let first = commit_file(repo.path(), "a.txt", "one", "feat: first");
    let second = commit_file(repo.path(), "a.txt", "two", "feat: second");

    let commit_diff = git::get_commit_diff(&second).expect("commit diff");
    assert!(commit_diff.contains("-one"));
    assert!(commit_diff.contains("+two"));

    let range_diff = git::get_range_diff(&first, &second).expect("range diff");
    assert!(range_diff.contains("a.txt"));

    assert!(git::is_head_commit(&second).expect("head check"));
    assert!(!git::is_head_commit(&first).expect("head check"));
}

#[test]
#[serial]
fn upstream_and_pushed_detection_work() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let head = commit_file(repo.path(), "tracked.txt", "v1", "feat: tracked");
    assert!(!git::has_upstream_branch().expect("upstream check"));
    assert!(!git::is_head_pushed().expect("head pushed check"));
    assert!(!git::commit_is_pushed(&head).expect("commit pushed check"));

    let remote = tempfile::TempDir::new().expect("temp remote");
    git_ok(remote.path(), ["init", "--bare"]);
    let remote_url = remote.path().to_string_lossy().replace('\\', "/");
    git_ok(repo.path(), ["remote", "add", "origin", &remote_url]);
    git_ok(repo.path(), ["push", "-u", "origin", "HEAD"]);

    assert!(git::has_upstream_branch().expect("upstream check"));
    assert!(git::is_head_pushed().expect("head pushed check"));
    assert!(git::commit_is_pushed(&head).expect("commit pushed check"));
}

#[test]
#[serial]
fn rewrite_head_commit_message_amends_commit() {
    let repo = common::init_git_repo();
    let _cwd = DirGuard::enter(repo.path());
    let original = commit_file(repo.path(), "msg.txt", "v1", "feat: old");
    git::rewrite_commit_message("HEAD", "feat: new", true).expect("rewrite head");
    let current = git_stdout(repo.path(), ["log", "-1", "--pretty=%s"]);
    let rewritten = git_stdout(repo.path(), ["rev-parse", "HEAD"]);

    assert_eq!(current, "feat: new");
    assert_ne!(rewritten, original);
}
