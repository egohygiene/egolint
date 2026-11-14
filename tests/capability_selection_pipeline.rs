// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

//! Actual CLI controls bind native execution to the public capability decisions.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn consumer(capabilities: &str) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    let template = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(format!("--template={}", template.path().display()))
            .arg(root.path())
            .status()
            .unwrap()
            .success()
    );
    write(
        root.path(),
        "config.toml",
        format!("config-version = 1\n[capabilities]\n{capabilities}\n"),
    );
    root
}

fn cli(root: &Path, arguments: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command
        .arg("--workspace")
        .arg(root)
        .args(["--config", "config.toml"])
        .args(arguments)
        .output()
        .unwrap()
}

fn report(root: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(".reports/egolint/run.json")).unwrap()).unwrap()
}

fn tool<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["tool_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["tool_id"] == id)
        .unwrap()
}

#[test]
fn disabled_portability_is_not_evaluated_or_reported_as_passed() {
    let root = consumer("disable = ['EGOLINT_PORTABILITY']");
    write(root.path(), "mixed.txt", "alpha\r\nbeta\n");
    let output = cli(root.path(), &["validate"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let run = report(root.path());
    assert_eq!(
        tool(&run, "EGOLINT_PORTABILITY")["status"],
        "disabled_by_configuration"
    );
    assert_eq!(tool(&run, "EGOLINT_PORTABILITY")["enforcement"], "disabled");
    assert!(
        run["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| { finding["rule"]["tool_id"] != "EGOLINT_PORTABILITY" })
    );

    write(root.path(), "config.toml", "config-version = 1\n");
    let control = cli(root.path(), &["validate"]);
    assert_eq!(control.status.code(), Some(1), "{control:?}");
    assert_eq!(
        tool(&report(root.path()), "EGOLINT_PORTABILITY")["status"],
        "failed_findings"
    );
}

#[test]
fn disabled_release_does_not_validate_or_publish_a_focused_report() {
    let root = consumer("disable = ['EGOLINT_REPOSITORY_RELEASE']");
    write(root.path(), ".egohygiene/release.json", "not valid JSON\n");
    let output = cli(root.path(), &["validate"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        tool(&report(root.path()), "EGOLINT_REPOSITORY_RELEASE")["status"],
        "disabled_by_configuration"
    );
    assert!(
        !root
            .path()
            .join(".reports/egolint/repository-release.json")
            .exists()
    );
    write(root.path(), "config.toml", "config-version = 1\n");
    let control = cli(root.path(), &["validate"]);
    assert!(
        root.path()
            .join(".reports/egolint/repository-release.json")
            .is_file(),
        "{control:?}"
    );
}

#[cfg(unix)]
#[test]
fn disabled_release_does_not_resolve_a_symlinked_declaration() {
    let root = consumer("disable = ['EGOLINT_REPOSITORY_RELEASE']");
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "release.json", "outside the consumer\n");
    std::fs::create_dir(root.path().join(".egohygiene")).unwrap();
    std::os::unix::fs::symlink(
        external.path().join("release.json"),
        root.path().join(".egohygiene/release.json"),
    )
    .unwrap();
    let output = cli(root.path(), &["validate"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        tool(&report(root.path()), "EGOLINT_REPOSITORY_RELEASE")["status"],
        "disabled_by_configuration"
    );
    assert!(
        !root
            .path()
            .join(".reports/egolint/repository-release.json")
            .exists()
    );
    write(root.path(), "config.toml", "config-version = 1\n");
    let control = cli(root.path(), &["validate"]);
    assert_eq!(control.status.code(), Some(2), "{control:?}");
    assert!(!root.path().join(".reports/egolint/run.json").exists());
}

#[test]
fn explicit_disabled_native_policy_is_a_configuration_error() {
    let root = consumer("disable = ['EGOLINT_REPOSITORY_CONTRACT']");
    for operation in ["plan", "validate", "lint"] {
        let output = cli(
            root.path(),
            &[operation, "--repository-contract", "missing.toml"],
        );
        assert_eq!(output.status.code(), Some(2), "{operation}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "explicitly requested native capability EGOLINT_REPOSITORY_CONTRACT is disabled"
        ));
    }
    assert!(!root.path().join(".reports/egolint/run.json").exists());
    write(
        root.path(),
        "config.toml",
        "config-version = 1\n[capabilities]\ndisable = ['EGOLINT_REPOSITORY_RELEASE']\n",
    );
    let output = cli(
        root.path(),
        &["validate", "--release-adoption-state", "required"],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "explicitly requested native capability EGOLINT_REPOSITORY_RELEASE is disabled"
        )
    );
}

#[test]
fn cli_native_disable_uses_the_same_normalization_as_the_planner() {
    let root = consumer("");
    for disabled in [
        "EGOLINT_REPOSITORY_CONTRACT",
        "egolint_repository_contract",
        " egolint_repository_contract ",
    ] {
        let output = cli(
            root.path(),
            &[
                "plan",
                "--repository-contract",
                "missing.toml",
                "--disable-linter",
                disabled,
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{disabled}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "explicitly requested native capability EGOLINT_REPOSITORY_CONTRACT is disabled"
        ));
    }
}

#[test]
fn native_declaration_requires_the_actual_typed_policy_input() {
    let root = consumer("declarations = ['EGOLINT_REPOSITORY_INTELLIGENCE']");
    let output = cli(root.path(), &["plan"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("requires its explicit CLI policy input")
    );
}

#[test]
fn legacy_selection_retains_original_native_behavior() {
    let root = consumer("enabled = false\ndisable = ['EGOLINT_PORTABILITY']");
    write(root.path(), "mixed.txt", "alpha\r\nbeta\n");
    let output = cli(root.path(), &["validate"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        tool(&report(root.path()), "EGOLINT_PORTABILITY")["status"],
        "failed_findings"
    );
}

#[test]
fn native_validation_reports_complete_results_only_for_its_execution_boundary() {
    let root = consumer("");
    write(root.path(), "example.js", "const answer = 42;\n");
    let output = cli(root.path(), &["validate"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let run = report(root.path());
    assert_eq!(run["completeness"], "normalized");
    let decisions = run["capabilities"].as_array().unwrap();
    assert!(decisions.iter().any(|decision| {
        decision["provider"] == "megalinter"
            && decision["reason"] == "not_executed_by_native_validation"
            && decision["state"] == "skipped"
    }));
    assert!(
        decisions
            .iter()
            .filter(|decision| decision["state"] == "selected")
            .all(|decision| { decision["placement"] == "native" })
    );
}
