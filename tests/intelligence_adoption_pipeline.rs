//! Keep adoption evidence, semantic status, and enforcement distinct through the native CLI.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const ADOPTION_RULE: &str = "EGO-INTEL-ADOPTION-001";

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn workspace(states: [&str; 3], enforcement: &str, adoption: bool, maximum: u32) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    git(
        root.path(),
        &[
            "init",
            "--quiet",
            &format!("--template={}", empty.path().display()),
        ],
    );
    git(
        root.path(),
        &["config", "core.hooksPath", empty.path().to_str().unwrap()],
    );
    git(root.path(), &["config", "core.autocrlf", "false"]);
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repository-intelligence/valid");
    for path in [
        "ROADMAP.md",
        "docs/decisions/README.md",
        "docs/decisions/ADR-0001-validation-contract.md",
        "docs/decisions/ADR-0002-legacy-projection.md",
        "docs/decisions/ADR-0003-versioned-projection.md",
        "docs/decisions/policy-reference.json",
    ] {
        let target = root.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(fixtures.join(path), target).unwrap();
    }
    let mut policy: toml::Value =
        toml::from_str(&std::fs::read_to_string(fixtures.join("policy.toml")).unwrap()).unwrap();
    for (surface, state) in ["adrs", "roadmap", "commit-history"]
        .into_iter()
        .zip(states)
    {
        policy[surface]["state"] = toml::Value::String(state.to_owned());
    }
    policy["profile"]["enforcement"] = toml::Value::String(enforcement.to_owned());
    if !adoption {
        policy["profile"]["enabled-rules"]
            .as_array_mut()
            .unwrap()
            .retain(|rule| rule.as_str() != Some(ADOPTION_RULE));
    }
    policy["commit-history"]["maximum-commits"] = toml::Value::Integer(i64::from(maximum));
    std::fs::write(
        root.path().join("policy.toml"),
        toml::to_string(&policy).unwrap(),
    )
    .unwrap();
    git(root.path(), &["add", "--all"]);
    for message in [
        "docs: add synthetic source",
        "docs: second synthetic commit",
    ] {
        git(
            root.path(),
            &[
                "-c",
                "user.name=Egolint Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--allow-empty",
                "--message",
                message,
            ],
        );
    }
    root
}

fn cli(root: &Path, represented: &str) -> (Output, Value) {
    let represented = if represented == "exact" {
        git(root, &["rev-parse", "--verify", "HEAD"])
    } else {
        represented.to_owned()
    };
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root)
        .args([
            "validate",
            "--repository-intelligence",
            "policy.toml",
            "--represented-commit",
            &represented,
        ])
        .output()
        .unwrap();
    let bytes = std::fs::read(root.join(".reports/egolint/repository-intelligence.json"))
        .unwrap_or_else(|error| panic!("{error}: {output:?}"));
    (output, serde_json::from_slice(&bytes).unwrap())
}

fn assert_report(output: &Output, report: &Value, status: &str, messages: &[&str]) {
    assert_eq!(output.status.code(), Some(0), "{output:?}\n{report}");
    assert_eq!(report["status"], status, "{report}");
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), messages.len(), "{report}");
    for expected in messages {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic["rule_id"] == ADOPTION_RULE
                    && diagnostic["severity"] == "warning"
                    && diagnostic["message"].as_str().unwrap().contains(expected)
            }),
            "missing {expected}: {report}"
        );
    }
    assert_eq!(report["summary"]["blocking_diagnostics"], 0);
}

#[test]
fn present_source_matrix_distinguishes_exact_unknown_and_contradictory_commits() {
    for states in [
        ["present", "present", "present"],
        ["present", "not-applicable", "not-applicable"],
        ["not-applicable", "present", "not-applicable"],
        ["not-applicable", "not-applicable", "present"],
    ] {
        for enforcement in ["blocking", "advisory"] {
            let root = workspace(states, enforcement, true, 100);
            for (represented, status, message) in [
                ("exact", "valid", None),
                (
                    "unknown",
                    "incomplete",
                    Some("represented commit is unknown"),
                ),
                (
                    "not-applicable",
                    "incomplete",
                    Some("represented commit cannot be not-applicable"),
                ),
            ] {
                let (output, report) = cli(root.path(), represented);
                assert_report(
                    &output,
                    &report,
                    status,
                    &message.into_iter().collect::<Vec<_>>(),
                );
                assert_eq!(report["commit_history_truncated"], false);
            }
        }
    }
}

#[test]
fn non_adoption_is_distinct_from_missing_and_truncated_evidence() {
    let absent = workspace(["not-applicable"; 3], "blocking", true, 100);
    for represented in ["exact", "not-applicable"] {
        let (output, report) = cli(absent.path(), represented);
        assert_report(&output, &report, "not_applicable", &[]);
    }
    let unknown = workspace(
        ["unknown", "not-applicable", "not-applicable"],
        "advisory",
        true,
        100,
    );
    let (output, report) = cli(unknown.path(), "exact");
    assert_report(
        &output,
        &report,
        "incomplete",
        &["adrs evidence is explicitly unknown"],
    );

    let truncated = workspace(["present"; 3], "blocking", true, 1);
    let (output, report) = cli(truncated.path(), "exact");
    assert_report(
        &output,
        &report,
        "incomplete",
        &["commit evidence is truncated"],
    );
    assert_eq!(report["commit_history_truncated"], true);
    assert_eq!(report["summary"]["commits_inspected"], 1);
}

#[test]
fn disabling_adoption_diagnostics_does_not_upgrade_incomplete_evidence() {
    let root = workspace(["present"; 3], "blocking", false, 100);
    for (represented, status) in [
        ("exact", "valid"),
        ("unknown", "incomplete"),
        ("not-applicable", "incomplete"),
    ] {
        let (output, report) = cli(root.path(), represented);
        assert_report(&output, &report, status, &[]);
    }
}
