//! Source-backed self-consumer evidence for the current-v1 release profile.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

const RELEASE_TOOL: &str = "EGOLINT_REPOSITORY_RELEASE";
const WORKFLOW_RULE: &str = "EGOLINT_RELEASE_MANUAL_WORKFLOW";

fn self_consumer() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary self-consumer");
    let template = tempfile::tempdir().expect("empty Git template");
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(format!("--template={}", template.path().display()))
            .arg(root.path())
            .status()
            .expect("initialize self-consumer")
            .success()
    );
    for path in [
        ".egohygiene/release.json",
        "AGENTS.md",
        "Cargo.toml",
        "CHANGELOG.md",
        "Taskfile.yml",
        ".github/workflows/release.yml",
    ] {
        let destination = root.path().join(path);
        std::fs::create_dir_all(destination.parent().expect("source parent"))
            .expect("source directory");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(path),
            destination,
        )
        .unwrap_or_else(|error| panic!("required self-consumer source {path}: {error}"));
    }
    root
}

fn report(root: &Path, name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(root.join(".reports/egolint").join(name)).expect("native report"),
    )
    .expect("native JSON")
}

fn validate(root: &Path) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    for (name, _) in
        std::env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("EGOLINT_"))
    {
        command.env_remove(name);
    }
    let output = command
        .arg("--workspace")
        .arg(root)
        .arg("validate")
        .output()
        .expect("native source validation; release Tasks are never executed");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn real_self_profile_has_manual_source_conformance_and_stable_evidence() {
    let root = self_consumer();
    validate(root.path());
    let focused = report(root.path(), "repository-release.json");
    assert_eq!(focused["repository"], "egohygiene/egolint");
    assert_eq!(focused["state"], "advisory");
    assert_eq!(
        focused["applicability"]["repository_profile"],
        "cli-library"
    );
    assert_eq!(focused["applicability"]["lifecycle"], "incubating");
    assert_eq!(focused["applicability"]["adoption_state"], "advisory");
    assert_eq!(focused["summary"]["validation_complete"], true);
    let checks = focused["checks"].as_array().expect("seven checks");
    assert_eq!(checks.len(), 7);
    for check in checks {
        assert_eq!(check["requirement"], "advisory");
        assert_eq!(check["state"], "passed");
    }
    assert_eq!(focused["boundary"]["network_access"], "not_performed");
    assert_eq!(focused["boundary"]["external_publication"], "not_verified");
    let run = report(root.path(), "run.json");
    let findings: Vec<_> = run["findings"]
        .as_array()
        .expect("normalized findings")
        .iter()
        .filter(|finding| finding["rule"]["tool_id"] == RELEASE_TOOL)
        .collect();
    assert!(findings.is_empty());
    let tool = run["tool_results"]
        .as_array()
        .expect("tool results")
        .iter()
        .find(|tool| tool["tool_id"] == RELEASE_TOOL)
        .expect("release result");
    assert_eq!(tool["status"], "passed");
    assert_eq!(tool["enforcement"], "advisory");
    assert_eq!(tool["finding_count"], 0);
    assert_eq!(tool["warning_count"], 0);
    let sarif = report(root.path(), "egolint.sarif");
    let results: Vec<_> = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results")
        .iter()
        .filter(|result| result["properties"]["egolintToolId"] == RELEASE_TOOL)
        .collect();
    assert!(results.is_empty());
    validate(root.path());
    assert_eq!(report(root.path(), "repository-release.json"), focused);
}

#[test]
fn real_task_publish_is_literal_manual_dispatch_and_planning_is_local() {
    let taskfile: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../Taskfile.yml")).expect("real Taskfile");
    let tasks = &taskfile["tasks"];
    for name in ["release:plan", "release:prepare", "release:verify"] {
        assert!(
            tasks[name]["cmds"].is_sequence(),
            "missing local task {name}"
        );
    }
    assert_eq!(
        tasks["release:prepare"]["env"]["RELEASE_PLAN"].as_str(),
        Some("{{.RELEASE_PLAN}}")
    );
    let prepare = tasks["release:prepare"]["cmds"][0]
        .as_str()
        .expect("literal preparation command");
    assert!(prepare.contains("--output \"$RELEASE_PLAN\""));
    assert!(!prepare.contains("{{.RELEASE_PLAN}}"));
    assert_eq!(
        tasks["release:publish"]["cmds"][0].as_str(),
        Some("gh workflow run .github/workflows/release.yml")
    );
    assert_eq!(
        tasks["release:publish"]["cmds"]
            .as_sequence()
            .unwrap()
            .len(),
        1
    );
    let workflow = include_str!("../.github/workflows/release.yml");
    assert!(!workflow.contains("  push:\n"));
    assert!(workflow.contains("Record non-publishing rehearsal result"));
}

#[test]
fn restoring_tag_automation_retains_the_visible_advisory_failure() {
    let root = self_consumer();
    let path = root.path().join(".github/workflows/release.yml");
    let mut workflow: serde_yaml::Value =
        serde_yaml::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    workflow["on"]["push"] = serde_yaml::from_str("tags: ['v*.*.*']").unwrap();
    std::fs::write(&path, serde_yaml::to_string(&workflow).unwrap()).unwrap();
    validate(root.path());
    let focused = report(root.path(), "repository-release.json");
    let workflow_check = focused["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["rule_id"] == WORKFLOW_RULE)
        .unwrap();
    assert_eq!(workflow_check["state"], "failed");
    assert_eq!(workflow_check["requirement"], "advisory");
    let run = report(root.path(), "run.json");
    let finding = run["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["rule"]["rule_id"] == WORKFLOW_RULE)
        .unwrap();
    assert_eq!(finding["severity"], "warning");
    assert_eq!(focused["boundary"]["external_publication"], "not_verified");
}
