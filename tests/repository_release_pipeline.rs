//! End-to-end agreement for native repository-release evidence projections.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let destination = root.join(path);
    std::fs::create_dir_all(destination.parent().expect("fixture parent"))
        .expect("fixture directory");
    std::fs::write(destination, content).expect("fixture file");
}

fn consumer() -> TempDir {
    let root = tempfile::tempdir().expect("temporary workspace");
    let template = tempfile::tempdir().expect("temporary Git template");
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args(["init", "--quiet"])
            .arg(format!("--template={}", template.path().display()))
            .status()
            .expect("initialize fixture repository")
            .success()
    );
    let declaration = json!({
        "$schema": "https://egohygiene.io/schemas/aether/repository-release/v1.json",
        "schema_version": "egohygiene.repository-release/v1",
        "repository": {
            "id": "egohygiene/release-fixture",
            "lifecycle": "active",
            "release_profile": "cli-library"
        },
        "release": {
            "state": "unreleased",
            "tag_prefix": "v",
            "immutable_tags": true,
            "major_alias": "disabled"
        },
        "changelog": {
            "path": "CHANGELOG.md",
            "format": "keep-a-changelog/1.1",
            "unreleased_heading": "Unreleased"
        },
        "components": [{
            "id": "release-fixture",
            "kind": "crate",
            "version_authority": {
                "kind": "cargo-manifest",
                "path": "Cargo.toml"
            }
        }],
        "delivery": {
            "channels": [{"kind": "github-release", "state": "configured"}]
        },
        "evidence": {
            "source": "required",
            "change": "required",
            "provenance": "required",
            "sbom": "required",
            "signature": "required",
            "rollback": {
                "strategy": "revert-and-successor-tag",
                "instructions": "Revert the change and publish a reviewed successor tag."
            }
        },
        "automation": {
            "taskfile_path": "Taskfile.yml",
            "tasks": {
                "plan": "release:plan",
                "prepare": "release:prepare",
                "verify": "release:verify",
                "publish": "release:publish"
            },
            "github": {
                "manual_dispatch_required": true,
                "workflow_path": ".github/workflows/release.yml",
                "state": "configured"
            }
        }
    });
    write(
        root.path(),
        ".egohygiene/release.json",
        serde_json::to_vec_pretty(&declaration).expect("declaration JSON"),
    );
    write(
        root.path(),
        "AGENTS.md",
        "Release facts: .egohygiene/release.json\n",
    );
    write(
        root.path(),
        "CHANGELOG.md",
        "# Changelog\n\n## [Unreleased]\n",
    );
    write(
        root.path(),
        "Cargo.toml",
        "[package]\nname = \"release-fixture\"\nversion = \"0.1.0\"\n",
    );
    write(
        root.path(),
        "Taskfile.yml",
        "version: '3'\ntasks:\n  'release:plan': {cmds: ['true']}\n  'release:prepare': {cmds: ['true']}\n  'release:verify': {cmds: ['true']}\n  'release:publish': {cmds: ['gh workflow run .github/workflows/release.yml']}\n",
    );
    write(
        root.path(),
        ".github/workflows/release.yml",
        "name: Release\non:\n  workflow_dispatch:\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n",
    );
    root
}

fn report(root: &Path, file: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(root.join(".reports/egolint").join(file)).expect("report file"),
    )
    .expect("report JSON")
}

fn output_text(output: &Output) -> String {
    format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn focused_run_cli_and_sarif_share_release_finding_identity() {
    let root = consumer();
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root.path())
        .arg("validate")
        .output()
        .expect("run native validation");
    let text = output_text(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("EGOLINT_RELEASE_MANUAL_WORKFLOW"), "{text}");

    let focused = report(root.path(), "repository-release.json");
    let check = focused["checks"]
        .as_array()
        .expect("focused checks")
        .iter()
        .find(|check| check["rule_id"] == "EGOLINT_RELEASE_MANUAL_WORKFLOW")
        .expect("focused workflow check");
    assert_eq!(check["state"], "failed");
    assert_eq!(check["location"]["path"], ".github/workflows/release.yml");

    let run = report(root.path(), "run.json");
    let finding = run["findings"]
        .as_array()
        .expect("run findings")
        .iter()
        .find(|finding| finding["rule"]["rule_id"] == "EGOLINT_RELEASE_MANUAL_WORKFLOW")
        .expect("run workflow finding");
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["location"]["path"], ".github/workflows/release.yml");
    let tool = run["tool_results"]
        .as_array()
        .expect("tool results")
        .iter()
        .find(|tool| tool["tool_id"] == "EGOLINT_REPOSITORY_RELEASE")
        .expect("release tool result");
    assert_eq!(tool["status"], "failed_findings");
    assert_eq!(tool["finding_count"], 1);

    let sarif = report(root.path(), "egolint.sarif");
    let result = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results")
        .iter()
        .find(|result| result["properties"]["egolintToolId"] == "EGOLINT_REPOSITORY_RELEASE")
        .expect("SARIF release result");
    assert_eq!(result["level"], "error");
    assert_eq!(
        result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        ".github/workflows/release.yml"
    );
}
