//! Native CLI evidence without Docker or ordinary repository payload reads.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repository-gitignore")
}

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let destination = root.join(path);
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::write(destination, content).unwrap();
}

fn consumer() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    let template = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args(["init", "--quiet"])
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
            root.path(),
            &format!("vendor/empathy/{path}"),
            std::fs::read(fixture().join("source").join(path)).unwrap(),
        );
    }
    let raw = std::fs::read(fixture().join("filament/composition.json")).unwrap();
    let composition: Value = serde_json::from_slice(&raw).unwrap();
    write(root.path(), "foundation/gitignore-plan.json", raw);
    write(
        root.path(),
        "foundation/gitignore-policy.toml",
        std::fs::read(fixture().join("filament/policy.toml")).unwrap(),
    );
    write(
        root.path(),
        ".gitignore",
        composition["files"][0]["content"].as_str().unwrap(),
    );
    root
}

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    command.arg("--workspace").arg(root).args([
        "gitignore",
        "--policy",
        "foundation/gitignore-policy.toml",
        "--evaluation-date",
        "2026-09-19",
    ]);
    command
}

fn report(root: &Path, file: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(".reports/egolint").join(file)).unwrap())
        .unwrap()
}

fn output_text(output: &Output) -> String {
    format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn native_cli_emits_focused_json_run_json_and_sarif_without_a_runtime() {
    let root = consumer();
    let output = command(root.path()).output().unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    assert_eq!(
        report(root.path(), "repository-gitignore.json")["status"],
        "valid"
    );
    let run = report(root.path(), "run.json");
    assert_eq!(
        run["tool_results"][0]["tool_id"],
        "EGOLINT_REPOSITORY_GITIGNORE"
    );
    assert_eq!(run["tool_results"][0]["status"], "passed");
    assert!(run["findings"].as_array().unwrap().is_empty());
    assert_eq!(report(root.path(), "egolint.sarif")["version"], "2.1.0");
}

#[test]
fn nested_override_blocks_cli_and_carries_winning_rule_evidence() {
    let root = consumer();
    write(root.path(), ".devcontainer/.gitignore", "!/.env\n");
    let output = command(root.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    let focused = report(root.path(), "repository-gitignore.json");
    let evidence = focused["behavior"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == ".devcontainer/.env")
        .unwrap();
    assert_eq!(evidence["winning_rule"]["path"], ".devcontainer/.gitignore");
    assert_eq!(evidence["winning_rule"]["pattern"], "!/.env");
    assert_eq!(evidence["expected_ignored"], true);
    assert_eq!(evidence["actual_ignored"], false);
    assert!(
        !report(root.path(), "egolint.sarif")["runs"][0]["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn missing_git_emits_incomplete_evidence_and_nonzero_exit() {
    let root = consumer();
    let empty_path = tempfile::tempdir().unwrap();
    let output = command(root.path())
        .env("PATH", empty_path.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    let focused = report(root.path(), "repository-gitignore.json");
    assert_eq!(focused["status"], "incomplete");
    assert_eq!(focused["checks"]["effective_behavior"], "unavailable");
    assert!(
        !report(root.path(), "run.json")["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn malformed_composition_still_has_machine_readable_failed_evidence() {
    let root = consumer();
    write(root.path(), "foundation/gitignore-plan.json", "{malformed");
    let output = command(root.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    let focused = report(root.path(), "repository-gitignore.json");
    assert_eq!(focused["checks"]["composition"], "failed");
    assert_eq!(focused["checks"]["effective_behavior"], "not_evaluated");
}

#[test]
fn policy_escape_is_a_configuration_failure_before_report_writes() {
    let root = consumer();
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root.path())
        .args([
            "gitignore",
            "--policy",
            "../outside.toml",
            "--evaluation-date",
            "2026-09-19",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!root.path().join(".reports").exists());
}

#[test]
fn report_outputs_cannot_overwrite_policy_inputs() {
    let root = consumer();
    let policy = std::fs::read(root.path().join("foundation/gitignore-policy.toml")).unwrap();
    write(root.path(), ".reports/egolint/run.json", &policy);
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root.path())
        .args([
            "gitignore",
            "--policy",
            ".reports/egolint/run.json",
            "--evaluation-date",
            "2026-09-19",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read(root.path().join(".reports/egolint/run.json")).unwrap(),
        policy
    );
}
