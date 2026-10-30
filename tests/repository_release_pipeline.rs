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

fn publish_commands(root: &Path, commands: &Value) {
    publish_task(root, &json!({"cmds": commands}));
}

fn publish_task(root: &Path, task: &Value) {
    write(
        root,
        "Taskfile.yml",
        serde_json::to_vec(&json!({
            "version": "3",
            "tasks": {
                "release:plan": {"cmds": ["true"]},
                "release:prepare": {"cmds": ["true"]},
                "release:verify": {"cmds": ["true"]},
                "release:publish": task
            }
        }))
        .expect("Taskfile YAML-compatible JSON"),
    );
    write(
        root,
        ".github/workflows/release.yml",
        "name: Release\non:\n  workflow_dispatch:\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    );
}

#[test]
fn static_task_dispatch_matrix_preserves_native_evidence_agreement() {
    // These strings are source fixtures only. The native validator never runs Task commands.
    let cases = [
        (
            "echo-only",
            json!(["echo gh workflow run .github/workflows/release.yml"]),
            false,
        ),
        (
            "comment-only",
            json!(["# gh workflow run .github/workflows/release.yml"]),
            false,
        ),
        (
            "quoted-documentation",
            json!(["'gh workflow run .github/workflows/release.yml'"]),
            false,
        ),
        (
            "different-workflow-comment",
            json!(["gh workflow run .github/workflows/other.yml # .github/workflows/release.yml"]),
            false,
        ),
        (
            "different-workflow-argument",
            json!([
                "gh workflow run .github/workflows/other.yml --field documentation=.github/workflows/release.yml"
            ]),
            false,
        ),
        (
            "path-suffix",
            json!(["gh workflow run .github/workflows/release.yml.backup"]),
            false,
        ),
        (
            "compound",
            json!(["true && gh workflow run .github/workflows/release.yml"]),
            false,
        ),
        (
            "trailing-command",
            json!(["gh workflow run .github/workflows/release.yml; true"]),
            false,
        ),
        (
            "pipeline",
            json!(["echo text | gh workflow run .github/workflows/release.yml"]),
            false,
        ),
        (
            "command-substitution",
            json!(["gh workflow run $(echo .github/workflows/release.yml)"]),
            false,
        ),
        (
            "template",
            json!(["gh workflow run {{.WORKFLOW}} # .github/workflows/release.yml"]),
            false,
        ),
        (
            "newline",
            json!(["gh\nworkflow run .github/workflows/release.yml"]),
            false,
        ),
        (
            "multiple-commands",
            json!([
                "gh workflow run .github/workflows/release.yml",
                "echo complete"
            ]),
            false,
        ),
        (
            "task-reference",
            json!([{"task": "publish", "cmd": "gh workflow run .github/workflows/release.yml"}]),
            false,
        ),
        ("non-string-command", json!([{"cmd": null}]), false),
        (
            "command-options",
            json!(["gh workflow run .github/workflows/release.yml --ref main"]),
            false,
        ),
        (
            "command-modifier",
            json!([{"cmd": "gh workflow run .github/workflows/release.yml", "ignore_error": true}]),
            false,
        ),
    ];
    for (name, commands, accepted) in cases {
        let root = consumer();
        publish_commands(root.path(), &commands);
        assert_dispatch_source(root.path(), name, accepted);
    }
}

#[test]
fn literal_direct_task_dispatches_remain_accepted() {
    for command in [
        "gh workflow run .github/workflows/release.yml",
        "gh workflow run \".github/workflows/release.yml\"",
        "gh workflow run '.github/workflows/release.yml'",
        "  gh\tworkflow  run .github/workflows/release.yml  ",
    ] {
        let root = consumer();
        publish_commands(root.path(), &json!([command]));
        assert_dispatch_source(root.path(), command, true);
    }
    let root = consumer();
    publish_commands(
        root.path(),
        &json!([{"cmd": "gh workflow run .github/workflows/release.yml"}]),
    );
    assert_dispatch_source(root.path(), "cmd mapping", true);
    let root = consumer();
    publish_task(
        root.path(),
        &json!({
            "desc": "Dispatch the reviewed workflow",
            "summary": "Manual dispatch only",
            "silent": true,
            "cmds": ["gh workflow run .github/workflows/release.yml"]
        }),
    );
    assert_dispatch_source(root.path(), "descriptive task metadata", true);
}

#[test]
fn task_execution_features_remain_unavailable() {
    for (key, value) in [
        ("deps", json!(["other-task"])),
        ("preconditions", json!(["true"])),
        ("status", json!(["true"])),
        ("env", json!({"WORKFLOW": ".github/workflows/release.yml"})),
        ("dir", json!("other-directory")),
    ] {
        let root = consumer();
        let mut task = json!({"cmds": ["gh workflow run .github/workflows/release.yml"]});
        task[key] = value;
        publish_task(root.path(), &task);
        assert_dispatch_source(root.path(), key, false);
    }
}

fn assert_dispatch_source(root: &Path, name: &str, accepted: bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root)
        .arg("validate")
        .output()
        .expect("inspect source without executing Task commands");
    let text = output_text(&output);
    assert_eq!(
        output.status.code(),
        Some(i32::from(!accepted)),
        "{name}: {text}"
    );
    let focused = report(root, "repository-release.json");
    let check = focused["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["rule_id"] == "EGOLINT_RELEASE_TASK_HANDOFFS")
        .expect("task check");
    assert_eq!(
        check["state"],
        if accepted { "passed" } else { "unavailable" },
        "{name}"
    );
    assert_eq!(check["location"]["path"], "Taskfile.yml", "{name}");
    assert_eq!(
        focused["state"],
        if accepted { "compliant" } else { "unavailable" },
        "{name}"
    );
    assert_eq!(
        focused["summary"]["validation_complete"], accepted,
        "{name}"
    );
    let run = report(root, "run.json");
    let findings: Vec<_> = run["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|finding| finding["rule"]["rule_id"] == "EGOLINT_RELEASE_TASK_HANDOFFS")
        .collect();
    let tool = run["tool_results"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["tool_id"] == "EGOLINT_REPOSITORY_RELEASE")
        .expect("release tool");
    assert_eq!(
        tool["status"],
        if accepted {
            "passed"
        } else {
            "failed_findings"
        },
        "{name}"
    );
    assert_eq!(findings.len(), usize::from(!accepted), "{name}");
    let sarif = report(root, "egolint.sarif");
    let results: Vec<_> = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results")
        .iter()
        .filter(|result| result["properties"]["egolintToolId"] == "EGOLINT_REPOSITORY_RELEASE")
        .collect();
    assert_eq!(results.len(), findings.len(), "{name}");
    if !accepted {
        assert_eq!(findings[0]["severity"], "error", "{name}");
        assert_eq!(findings[0]["location"], check["location"], "{name}");
        assert_eq!(results[0]["level"], "error", "{name}");
        assert_eq!(
            results[0]["properties"]["egolintFindingId"], findings[0]["id"],
            "{name}"
        );
        assert_eq!(
            results[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "Taskfile.yml",
            "{name}"
        );
    }
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
