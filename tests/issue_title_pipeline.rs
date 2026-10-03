//! Candidate contract parity, source evidence, and the actual issue-title CLI boundary.

use std::process::{Command, Output};

use egolint::issue_titles::{IssueTitlePolicy, IssueTitleSnapshot, IssueTitleStatus};
use serde_json::{Value, json};

const CASES: &str =
    include_str!("../vendor/github/issue-titles/fixtures/issue-titles/cases.v1.json");

fn snapshot(title: &str, labels: &[&str]) -> IssueTitleSnapshot {
    IssueTitleSnapshot {
        schema_version: 1,
        complete: true,
        title: title.into(),
        labels: labels.iter().map(|label| (*label).into()).collect(),
    }
}

fn cli(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_egolint"))
        .args(arguments)
        .output()
        .expect("run Egolint")
}

fn cli_snapshot(value: &Value) -> Output {
    let root = tempfile::tempdir().expect("temporary snapshot directory");
    let path = root.path().join("snapshot.json");
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let output = cli(&["issue-title", "validate", "--input", path.to_str().unwrap()]);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    output
}

#[test]
fn upstream_validation_corpus_matches_library_and_cli() {
    let policy = IssueTitlePolicy::bundled().unwrap();
    let fixtures: Value = serde_json::from_str(CASES).unwrap();
    assert_eq!(fixtures["cases"].as_array().unwrap().len(), 18);
    for case in fixtures["cases"].as_array().unwrap() {
        let mut input = case["input"].clone();
        input["schema_version"] = json!(1);
        input["complete"] = json!(true);
        let snapshot = serde_json::from_value(input.clone()).unwrap();
        let report = policy.validate(&snapshot).unwrap();
        let report_json = serde_json::to_value(&report).unwrap();
        assert_eq!(
            report_json["status"], case["expected"]["status"],
            "{}",
            case["id"]
        );
        assert_eq!(
            report_json["type"], case["expected"]["type"],
            "{}",
            case["id"]
        );
        let output = cli_snapshot(&input);
        let expected_code = i32::from(report.status != IssueTitleStatus::Conformant);
        assert_eq!(output.status.code(), Some(expected_code), "{}", case["id"]);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            report_json
        );
    }
}

#[test]
fn reviewed_migrations_preserve_identifiers_and_repeat_without_drift() {
    let policy = IssueTitlePolicy::bundled().unwrap();
    let fixtures: Value = serde_json::from_str(CASES).unwrap();
    for case in fixtures["migrations"].as_array().unwrap() {
        let subject = case["reviewed_subject"].as_str().unwrap();
        let labels: Vec<&str> = case["before"]["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|label| label.as_str().unwrap())
            .collect();
        let kind = labels[0].strip_prefix("type:").unwrap();
        let proposal = policy.format(kind, subject).unwrap();
        assert_eq!(proposal.title, case["after"]["title"].as_str().unwrap());
        assert_eq!(case["before"]["labels"], case["after"]["labels"]);
        assert_eq!(proposal, policy.format(kind, subject).unwrap());
        let validated = policy
            .validate(&snapshot(&proposal.title, &labels))
            .unwrap();
        assert_eq!(validated.status, IssueTitleStatus::Conformant);
    }
    let subject = "🧭 [Release checkpoint 7] Keep MiXeD case — café e\u{301}";
    assert_eq!(
        policy.format("maintenance", subject).unwrap().title,
        format!("🧹 [maintenance] {subject}")
    );
}

#[test]
fn classification_uses_sets_and_unknown_types_take_precedence() {
    let policy = IssueTitlePolicy::bundled().unwrap();
    let title = "✨ [feature] Preserve identifiers";
    for labels in [
        vec!["type:feature", "type:feature", "area:automation"],
        vec!["area:automation", "type:feature"],
    ] {
        assert_eq!(
            policy.validate(&snapshot(title, &labels)).unwrap().status,
            IssueTitleStatus::Conformant
        );
    }
    assert_eq!(
        policy
            .validate(&snapshot(
                title,
                &["type:bug", "type:feature", "type:future"]
            ))
            .unwrap()
            .status,
        IssueTitleStatus::UnsupportedType
    );
    assert_eq!(
        policy
            .validate(&snapshot(title, &["🐛 bug"]))
            .unwrap()
            .status,
        IssueTitleStatus::NeedsClassification
    );
}

#[test]
fn unicode_and_whitespace_are_checked_without_normalizing_the_subject() {
    let policy = IssueTitlePolicy::bundled().unwrap();
    assert_eq!(
        policy
            .validate(&snapshot(
                "🏗 [architecture] Missing variation selector",
                &["type:architecture"]
            ))
            .unwrap()
            .status,
        IssueTitleStatus::Nonconformant
    );
    for subject in [
        "",
        " ",
        " padded",
        "padded ",
        "\u{00a0}padded",
        "line\nline",
        "line\rline",
        "line\u{000b}line",
        "line\u{000c}line",
        "line\u{0085}line",
        "line\u{2028}line",
        "line\u{2029}line",
    ] {
        assert!(policy.format("feature", subject).is_err());
        assert_eq!(
            policy
                .validate(&snapshot(
                    &format!("✨ [feature] {subject}"),
                    &["type:feature"]
                ))
                .unwrap()
                .status,
            IssueTitleStatus::Nonconformant
        );
    }
    assert!(policy.format("Feature", "Subject").is_err());
    assert!(policy.format("future", "Subject").is_err());
}

#[test]
fn incomplete_and_invalid_inputs_never_become_conformant() {
    let mut valid = json!({"schema_version":1, "complete":true,
        "title":"✨ [feature] A subject", "labels":["type:feature"]});
    for field in ["schema_version", "complete", "title", "labels"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<IssueTitleSnapshot>(missing.clone()).is_err());
        let result = cli_snapshot(&missing);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
    }
    for labels in [
        Value::Null,
        json!("type:feature"),
        json!([{"name":"type:feature"}]),
    ] {
        let mut malformed = valid.clone();
        malformed["labels"] = labels;
        assert_eq!(cli_snapshot(&malformed).status.code(), Some(2));
    }
    for (field, value) in [("schema_version", json!(2)), ("unexpected", json!(true))] {
        let mut unsupported = valid.clone();
        unsupported[field] = value;
        assert_eq!(cli_snapshot(&unsupported).status.code(), Some(2));
    }
    valid["complete"] = json!(false);
    let output = cli_snapshot(&valid);
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "unavailable");
    assert!(report["type"].is_null());
}

#[test]
fn formatter_cli_is_deterministic_and_exposes_candidate_provenance() {
    let args = [
        "issue-title",
        "format",
        "--type",
        "maintenance",
        "--reviewed-subject",
        "[Release checkpoint 7] Provide pinned tools",
    ];
    let first = cli(&args);
    let second = cli(&args);
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    let proposal: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        proposal["title"],
        "🧹 [maintenance] [Release checkpoint 7] Provide pinned tools"
    );
    assert_eq!(proposal["required_label"], "type:maintenance");
    assert_eq!(proposal["contract"]["authority"], "candidate");
    assert_eq!(
        proposal["contract"]["revision"],
        "19d2be9bf0191710508cefbb9f0b1abb3a40d9be"
    );
    assert_eq!(
        proposal["contract"]["source_digests"]
            .as_object()
            .unwrap()
            .len(),
        5
    );
    for (kind, subject) in [("future", "Subject"), ("feature", " padded")] {
        let output = cli(&[
            "issue-title",
            "format",
            "--type",
            kind,
            "--reviewed-subject",
            subject,
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn issue_title_schemas_match_checked_in_rust_projections() {
    for kind in [
        "issue-title-snapshot",
        "issue-title-report",
        "issue-title-proposal",
    ] {
        let output = cli(&["schema", kind]);
        assert_eq!(output.status.code(), Some(0));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("schemas/{kind}.schema.json"));
        assert_eq!(output.stdout, std::fs::read(path).unwrap());
    }
}
