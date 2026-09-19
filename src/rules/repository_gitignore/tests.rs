//! Offline consumers and real Git exercise the pinned upstream behavior contract.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use super::*;

const DATE: &str = "2026-09-19";
const POLICY: &str = "foundation/gitignore-policy.toml";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repository-gitignore")
}

fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn consumer(name: &str) -> (TempDir, RepositoryGitignorePolicy) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let template = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["init", "--quiet", "--initial-branch=main"])
            .arg(format!("--template={}", template.path().display()))
            .status()
            .unwrap()
            .success()
    );
    for path in [
        "foundation/catalog.json",
        "foundation/ignore/universal.gitignore",
        "foundation/ignore/rust.gitignore",
    ] {
        write(
            root,
            &format!("vendor/empathy/{path}"),
            std::fs::read(fixture().join("source").join(path)).unwrap(),
        );
    }
    let policy = RepositoryGitignorePolicy::from_toml(
        &std::fs::read_to_string(fixture().join(name).join("policy.toml")).unwrap(),
        Path::new(POLICY),
    )
    .unwrap();
    let raw = std::fs::read(fixture().join(name).join("composition.json")).unwrap();
    write(root, &policy.composition_path, &raw);
    let composition: Value = serde_json::from_slice(&raw).unwrap();
    for file in composition["files"].as_array().unwrap() {
        write(
            root,
            file["path"].as_str().unwrap(),
            file["content"].as_str().unwrap(),
        );
    }
    (directory, policy)
}

fn evaluate(root: &Path, policy: &RepositoryGitignorePolicy) -> GitignoreEvaluation {
    evaluate_gitignore(root, policy, Path::new(POLICY), DATE).unwrap()
}

fn probe<'a>(result: &'a GitignoreEvaluation, path: &str) -> &'a GitignoreBehaviorEvidence {
    result
        .report
        .behavior
        .iter()
        .find(|entry| entry.path == path)
        .unwrap()
}

fn change_plan(
    root: &Path,
    policy: &mut RepositoryGitignorePolicy,
    change: impl FnOnce(&mut Value),
) {
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(root.join(&policy.composition_path)).unwrap())
            .unwrap();
    change(&mut value);
    policy.composition_sha256 = contract::canonical_digest(&value).unwrap();
    write(
        root,
        &policy.composition_path,
        serde_json::to_vec_pretty(&value).unwrap(),
    );
}

fn review_nested(policy: &mut RepositoryGitignorePolicy, path: &str, contents: &str) {
    policy.nested_policies.push(GitignoreNestedPolicy {
        path: path.to_owned(),
        sha256: digest(contents.as_bytes()),
        owner: "consumer-maintainer".to_owned(),
        reason: "Review inherited policy without exempting its behavior.".to_owned(),
        approval: "https://github.com/example/consumer/issues/1".to_owned(),
    });
}

fn exception(path: &str, policy_path: &str, contents: &str) -> GitignoreException {
    GitignoreException {
        path: path.to_owned(),
        policy_path: policy_path.to_owned(),
        policy_sha256: digest(contents.as_bytes()),
        owner: "consumer-maintainer".to_owned(),
        reason: "Synthetic exact-path fixture for exception expiry.".to_owned(),
        approval: "https://github.com/example/consumer/issues/2".to_owned(),
        expires_on: "2026-10-01".to_owned(),
        ignored: Some(false),
        allow_tracked: false,
    }
}

#[test]
fn accepted_filament_agrees_with_all_pinned_upstream_probes() {
    let (root, policy) = consumer("filament");
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.status,
        GitignoreValidationStatus::Valid,
        "{:?}",
        result.report.diagnostics
    );
    assert!(result.findings.is_empty());
    for expected in contract::catalog().unwrap().probes {
        let observed = probe(&result, &expected.path);
        assert_eq!(
            observed.actual_ignored, expected.ignored,
            "{}",
            expected.path
        );
    }
}

#[test]
fn scoped_rust_preserves_local_visibility_and_unrelated_sources() {
    let (root, policy) = consumer("scoped-rust");
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.status,
        GitignoreValidationStatus::Valid,
        "{:?}",
        result.report.diagnostics
    );
    for expected in &policy.probes {
        let observed = probe(&result, &expected.path);
        assert_eq!(observed.actual_ignored, expected.ignored);
    }
    assert_eq!(
        probe(&result, "crates/widget/target/keep.txt")
            .winning_rule
            .as_ref()
            .unwrap()
            .pattern,
        "!/target/keep.txt"
    );
}

#[test]
fn pinned_sources_cannot_be_replaced_or_partially_missing() {
    for path in [
        "vendor/empathy/foundation/catalog.json",
        "vendor/empathy/foundation/ignore/universal.gitignore",
        "vendor/empathy/foundation/ignore/rust.gitignore",
    ] {
        let (root, policy) = consumer("filament");
        write(root.path(), path, "tampered\n");
        let result = evaluate(root.path(), &policy);
        assert_eq!(
            result.report.checks.source_integrity,
            GitignoreCheckStatus::Failed
        );
        assert_eq!(result.report.status, GitignoreValidationStatus::Invalid);
        assert!(!result.findings.is_empty());
    }
}

#[test]
fn source_identity_and_plan_digest_are_independent_checks() {
    let (root, mut policy) = consumer("filament");
    policy.source_revision = "0".repeat(40);
    assert_eq!(
        evaluate(root.path(), &policy)
            .report
            .checks
            .source_integrity,
        GitignoreCheckStatus::Failed
    );
    policy.source_revision = contract::catalog().unwrap().source.revision;
    policy.composition_sha256 = "0".repeat(64);
    let report = evaluate(root.path(), &policy).report;
    assert_eq!(report.checks.source_integrity, GitignoreCheckStatus::Passed);
    assert_eq!(report.checks.composition, GitignoreCheckStatus::Failed);
    assert_eq!(report.checks.presence, GitignoreCheckStatus::Passed);
    assert_eq!(report.checks.content, GitignoreCheckStatus::NotEvaluated);
}

#[test]
fn rehashed_plan_cannot_reorder_drop_or_reassign_the_baseline() {
    for mutation in 0..4 {
        let (root, mut policy) = consumer("filament");
        change_plan(root.path(), &mut policy, |value| match mutation {
            0 => value["files"][0]["layers"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            1 => {
                value["files"][0]["layers"].as_array_mut().unwrap().pop();
            }
            2 => value["files"][0]["ownership"] = "provider-owned".into(),
            _ => value["files"][0]["content"] = "# removed baseline\n".into(),
        });
        assert_eq!(
            evaluate(root.path(), &policy).report.checks.composition,
            GitignoreCheckStatus::Failed
        );
    }
}

#[test]
fn missing_policy_is_different_from_byte_drift_and_behavior_drift() {
    let (root, policy) = consumer("filament");
    std::fs::remove_file(root.path().join(".gitignore")).unwrap();
    let report = evaluate(root.path(), &policy).report;
    assert_eq!(report.checks.presence, GitignoreCheckStatus::Failed);
    assert_eq!(report.checks.content, GitignoreCheckStatus::Failed);
    assert_eq!(
        report.checks.effective_behavior,
        GitignoreCheckStatus::Failed
    );
    let (root, policy) = consumer("filament");
    let mut contents = std::fs::read(root.path().join(".gitignore")).unwrap();
    contents.extend_from_slice(b"\n# local unrecorded comment\n");
    write(root.path(), ".gitignore", contents);
    let report = evaluate(root.path(), &policy).report;
    assert_eq!(report.checks.presence, GitignoreCheckStatus::Passed);
    assert_eq!(report.checks.content, GitignoreCheckStatus::Failed);
    assert_eq!(
        report.checks.effective_behavior,
        GitignoreCheckStatus::Passed
    );
}

#[test]
fn preserved_local_text_is_not_a_content_exemption() {
    let (root, mut policy) = consumer("filament");
    change_plan(root.path(), &mut policy, |value| {
        value["files"][0]["override"] = "preserve".into();
    });
    assert_eq!(
        evaluate(root.path(), &policy).report.status,
        GitignoreValidationStatus::Valid
    );
    write(
        root.path(),
        ".gitignore",
        "# preserve does not mean conformant\n",
    );
    assert_eq!(
        evaluate(root.path(), &policy).report.checks.content,
        GitignoreCheckStatus::Failed
    );
}

#[test]
fn unreachable_negation_fails_even_when_plan_and_consumer_bytes_agree() {
    let (root, mut policy) = consumer("scoped-rust");
    let old = policy.scopes[1].local_additions.clone();
    let local = "/scratch/\n!/target/keep.txt\n".to_owned();
    policy.scopes[1].local_additions.clone_from(&local);
    change_plan(root.path(), &mut policy, |value| {
        let file = &mut value["files"][1];
        let content = file["content"].as_str().unwrap().replace(&old, &local);
        file["content_sha256"] = digest(content.as_bytes()).into();
        file["content"] = content.clone().into();
        file["layers"][1]["sha256"] = digest(local.as_bytes()).into();
        write(root.path(), "crates/widget/.gitignore", content);
    });
    let result = evaluate(root.path(), &policy);
    assert_eq!(result.report.checks.content, GitignoreCheckStatus::Passed);
    assert_eq!(
        result.report.checks.composition,
        GitignoreCheckStatus::Passed
    );
    assert_eq!(
        result.report.checks.effective_behavior,
        GitignoreCheckStatus::Failed
    );
    assert!(probe(&result, "crates/widget/target/keep.txt").actual_ignored);
}

#[test]
fn undeclared_nested_policy_is_incomplete_even_without_behavior_drift() {
    let (root, policy) = consumer("filament");
    write(
        root.path(),
        "nested/.gitignore",
        "# harmless but not inventoried\n",
    );
    let result = evaluate(root.path(), &policy);
    assert_eq!(result.report.status, GitignoreValidationStatus::Incomplete);
    assert_eq!(
        result.report.checks.effective_behavior,
        GitignoreCheckStatus::Passed
    );
    assert_eq!(
        result.report.checks.nested_policy,
        GitignoreCheckStatus::Incomplete
    );
    assert!(!result.findings.is_empty());
}

#[test]
fn inherited_empathy_policies_are_findings_not_blanket_exemptions() {
    let (root, mut policy) = consumer("filament");
    policy.probes.push(GitignoreProbe {
        path: "egolint/examples/target/source.rs".to_owned(),
        ignored: false,
    });
    policy.probes.push(GitignoreProbe {
        path: "holon/packs/react-vite/template/.env.sample.local".to_owned(),
        ignored: true,
    });
    let inherited: Value =
        serde_json::from_slice(&std::fs::read(fixture().join("inherited-policies.json")).unwrap())
            .unwrap();
    for file in inherited["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        let contents = file["content"].as_str().unwrap();
        assert_eq!(
            digest(contents.as_bytes()),
            file["sha256"].as_str().unwrap()
        );
        write(root.path(), path, contents);
        review_nested(&mut policy, path, contents);
    }
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.checks.nested_policy,
        GitignoreCheckStatus::Passed
    );
    assert_eq!(
        result.report.checks.effective_behavior,
        GitignoreCheckStatus::Failed
    );
    for path in [
        ".devcontainer/.env",
        "mantle/.env.example.local",
        "holon/packs/react-vite/template/.env.sample.local",
    ] {
        let observed = probe(&result, path);
        assert!(observed.expected_ignored);
        assert!(!observed.actual_ignored);
        assert!(observed.exception.is_none());
    }
    let observed = probe(&result, "egolint/examples/target/source.rs");
    assert!(!observed.expected_ignored);
    assert!(observed.actual_ignored);
}

#[test]
fn broad_target_and_build_rules_hide_source_and_are_not_baseline_compatible() {
    let (root, policy) = consumer("filament");
    let mut contents = std::fs::read_to_string(root.path().join(".gitignore")).unwrap();
    contents.push_str("\ntarget/\nbuild/\n");
    write(root.path(), ".gitignore", contents);
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.checks.effective_behavior,
        GitignoreCheckStatus::Failed
    );
    for path in ["project/target/source.txt", "build/source.txt"] {
        let observed = probe(&result, path);
        assert!(!observed.expected_ignored);
        assert!(observed.actual_ignored);
    }
}

#[test]
fn exact_reviewed_exceptions_expire_and_never_erase_the_finding() {
    let (root, mut policy) = consumer("filament");
    let path = ".devcontainer/.gitignore";
    let contents = "!/.env\n";
    write(root.path(), path, contents);
    review_nested(&mut policy, path, contents);
    policy
        .exceptions
        .push(exception(".devcontainer/.env", path, contents));
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.status,
        GitignoreValidationStatus::ValidWithExceptions,
        "{:?}",
        result.report.diagnostics
    );
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Info);
    assert!(probe(&result, ".devcontainer/.env").exception.is_some());
    policy.exceptions[0].expires_on = "2026-09-18".to_owned();
    assert_eq!(
        evaluate(root.path(), &policy).report.status,
        GitignoreValidationStatus::Invalid
    );
    policy.exceptions[0].expires_on = "2026-10-01".to_owned();
    policy.exceptions[0].policy_sha256 = "0".repeat(64);
    assert_eq!(
        evaluate(root.path(), &policy).report.status,
        GitignoreValidationStatus::Invalid
    );
}

#[test]
fn changed_reviewed_nested_policy_fails_its_inventory_check() {
    let (root, mut policy) = consumer("filament");
    review_nested(&mut policy, "nested/.gitignore", "# reviewed\n");
    write(root.path(), "nested/.gitignore", "# changed\n");
    assert_eq!(
        evaluate(root.path(), &policy).report.checks.nested_policy,
        GitignoreCheckStatus::Failed
    );
}

#[test]
fn tracked_environment_is_reported_without_reading_or_unstaging_payload() {
    let (root, policy) = consumer("filament");
    let secret = "synthetic-payload-that-must-not-appear-in-reports";
    write(root.path(), ".env", secret);
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args(["add", "--force", "--", ".env"])
            .status()
            .unwrap()
            .success()
    );
    let index = std::fs::read(root.path().join(".git/index")).unwrap();
    let result = evaluate(root.path(), &policy);
    assert_eq!(
        result.report.checks.tracked_files,
        GitignoreCheckStatus::Failed
    );
    assert!(probe(&result, ".env").tracked);
    assert!(
        !serde_json::to_string(&result.report)
            .unwrap()
            .contains(secret)
    );
    assert_eq!(
        std::fs::read(root.path().join(".git/index")).unwrap(),
        index
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join(".env")).unwrap(),
        secret
    );
}

#[test]
fn missing_git_is_an_explicit_incomplete_blocking_result() {
    let (root, policy) = consumer("filament");
    let result = evaluation::evaluate(
        root.path(),
        &policy,
        Path::new(POLICY),
        DATE,
        &root.path().join("no-git-here"),
    )
    .unwrap();
    assert_eq!(result.report.status, GitignoreValidationStatus::Incomplete);
    assert_eq!(
        result.report.checks.effective_behavior,
        GitignoreCheckStatus::Unavailable
    );
    assert_eq!(
        result.report.checks.tracked_files,
        GitignoreCheckStatus::Unavailable
    );
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    );
}

#[test]
fn policy_paths_and_selection_are_closed_before_evaluation() {
    let (_, policy) = consumer("filament");
    for path in [
        "../outside",
        "/absolute",
        "C:/outside",
        "dir/../escape",
        ".git/index",
        ".reports/egolint/run.json",
        "dir\\escape",
    ] {
        let mut invalid = policy.clone();
        invalid.composition_path = path.to_owned();
        assert!(contract::validate_policy(&invalid).is_err(), "{path}");
    }
    let mut invalid = policy;
    invalid.scopes[0].local_additions = "target/\n".to_owned();
    assert!(contract::validate_policy(&invalid).is_err());
}

#[test]
fn nested_git_and_oversized_policy_cannot_produce_a_complete_pass() {
    let (root, policy) = consumer("filament");
    write(root.path(), "nested/.git", "gitdir: elsewhere\n");
    assert_eq!(
        evaluate(root.path(), &policy).report.checks.coverage,
        GitignoreCheckStatus::Incomplete
    );
    std::fs::remove_file(root.path().join("nested/.git")).unwrap();
    write(
        root.path(),
        "nested/.gitignore",
        vec![b'#'; MAX_FILE_BYTES + 1],
    );
    assert_eq!(
        evaluate(root.path(), &policy).report.checks.coverage,
        GitignoreCheckStatus::Incomplete
    );
}

#[test]
fn payloads_and_ambient_info_excludes_do_not_change_portable_policy_evidence() {
    let (root, policy) = consumer("filament");
    write(
        root.path(),
        "src/main.rs",
        "not parsed as Rust, never copied\n",
    );
    write(root.path(), ".git/info/exclude", "Cargo.lock\n");
    let result = evaluate(root.path(), &policy);
    assert_eq!(result.report.status, GitignoreValidationStatus::Valid);
    assert!(!probe(&result, "Cargo.lock").actual_ignored);
    assert!(!root.path().join(".reports").exists());
    let baseline: BTreeMap<_, _> = inventory::collect(root.path()).policies;
    assert_eq!(baseline.len(), 1);
}

#[cfg(unix)]
#[test]
fn symlinked_source_and_nested_policies_are_not_followed() {
    use std::os::unix::fs::symlink;
    let (root, policy) = consumer("filament");
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "outside", "!/.env\n");
    std::fs::create_dir_all(root.path().join("nested")).unwrap();
    symlink(
        external.path().join("outside"),
        root.path().join("nested/.gitignore"),
    )
    .unwrap();
    assert_eq!(
        evaluate(root.path(), &policy).report.checks.coverage,
        GitignoreCheckStatus::Incomplete
    );
    let source = root
        .path()
        .join("vendor/empathy/foundation/ignore/universal.gitignore");
    std::fs::remove_file(&source).unwrap();
    symlink(external.path().join("outside"), source).unwrap();
    assert_eq!(
        evaluate(root.path(), &policy)
            .report
            .checks
            .source_integrity,
        GitignoreCheckStatus::Failed
    );
}
