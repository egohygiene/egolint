// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

//! Current-run report lifecycle exercised with authentic native CLI evidence.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const REPORTS: &str = ".reports/egolint";
const INTELLIGENCE: &str = "repository-intelligence.json";
const GITIGNORE: &str = "repository-gitignore.json";
const RELEASE: &str = "repository-release.json";

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let target = root.join(path);
    std::fs::create_dir_all(target.parent().expect("fixture parent")).expect("fixture directory");
    std::fs::write(target, content).expect("fixture contents");
}

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .expect("fixture Git command");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).expect("Git stdout")
}

fn consumer() -> TempDir {
    let root = tempfile::tempdir().expect("workspace");
    let template = tempfile::tempdir().expect("empty Git template");
    git(
        root.path(),
        &[
            "init",
            "--quiet",
            &format!("--template={}", template.path().display()),
        ],
    );
    git(root.path(), &["config", "core.autocrlf", "false"]);
    git(
        root.path(),
        &[
            "config",
            "core.hooksPath",
            template.path().to_str().unwrap(),
        ],
    );
    write(root.path(), ".git/info/exclude", ".reports/\n");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for path in [
        "ROADMAP.md",
        "docs/decisions/README.md",
        "docs/decisions/ADR-0001-validation-contract.md",
        "docs/decisions/ADR-0002-legacy-projection.md",
        "docs/decisions/ADR-0003-versioned-projection.md",
        "docs/decisions/policy-reference.json",
    ] {
        write(
            root.path(),
            path,
            std::fs::read(fixtures.join("repository-intelligence/valid").join(path)).unwrap(),
        );
    }
    write(
        root.path(),
        "policy.toml",
        std::fs::read(fixtures.join("repository-intelligence/valid/policy.toml")).unwrap(),
    );
    for path in [
        "foundation/catalog.json",
        "foundation/ignore/universal.gitignore",
        "foundation/ignore/rust.gitignore",
    ] {
        write(
            root.path(),
            &format!("vendor/empathy/{path}"),
            std::fs::read(fixtures.join("repository-gitignore/source").join(path)).unwrap(),
        );
    }
    let composition =
        std::fs::read(fixtures.join("repository-gitignore/filament/composition.json")).unwrap();
    let value: Value = serde_json::from_slice(&composition).unwrap();
    write(root.path(), "foundation/gitignore-plan.json", composition);
    write(
        root.path(),
        "foundation/gitignore-policy.toml",
        std::fs::read(fixtures.join("repository-gitignore/filament/policy.toml")).unwrap(),
    );
    write(
        root.path(),
        ".gitignore",
        value["files"][0]["content"].as_str().unwrap(),
    );
    git(root.path(), &["add", "--all"]);
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
            "--message",
            "docs: add synthetic report lifecycle fixture",
        ],
    );
    root
}

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    command.env_clear().env("CI", "true");
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command.arg("--workspace").arg(root);
    command
}

fn successful(output: &Output) {
    assert!(output.status.success(), "{output:?}");
}

fn report_path(root: &Path, name: &str) -> std::path::PathBuf {
    root.join(REPORTS).join(name)
}

fn report(root: &Path, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(report_path(root, name)).expect("actual native report"))
        .expect("native report JSON")
}

fn emit_intelligence(root: &Path) {
    let revision = git(root, &["rev-parse", "--verify", "HEAD"]);
    successful(
        &command(root)
            .args([
                "validate",
                "--repository-intelligence",
                "policy.toml",
                "--represented-commit",
                revision.trim(),
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(report(root, INTELLIGENCE)["status"], "valid");
    assert_current_references(root);
}

fn emit_gitignore(root: &Path) {
    successful(
        &command(root)
            .args([
                "gitignore",
                "--policy",
                "foundation/gitignore-policy.toml",
                "--evaluation-date",
                "2026-09-19",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(report(root, GITIGNORE)["status"], "valid");
    assert_current_references(root);
}

fn assert_current_references(root: &Path) {
    fn visit(root: &Path, value: &Value) {
        match value {
            Value::Object(fields) => {
                if let Some(path) = fields
                    .get("path")
                    .and_then(Value::as_str)
                    .filter(|path| path.starts_with(".reports/egolint/"))
                {
                    assert!(
                        root.join(path).is_file(),
                        "missing current-run evidence {path}"
                    );
                }
                for child in fields.values() {
                    visit(root, child);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(root, child);
                }
            }
            _ => {}
        }
    }
    visit(root, &report(root, "run.json"));
    assert!(report_path(root, "egolint.sarif").is_file());
}

fn current_report_bytes(root: &Path) -> Vec<(&'static str, Vec<u8>)> {
    ["run.json", "egolint.sarif", INTELLIGENCE, RELEASE]
        .into_iter()
        .map(|name| (name, std::fs::read(report_path(root, name)).unwrap()))
        .collect()
}

fn assert_reports_unchanged(root: &Path, before: Vec<(&str, Vec<u8>)>) {
    for (name, bytes) in before {
        assert_eq!(
            std::fs::read(report_path(root, name)).unwrap(),
            bytes,
            "mutated selected input/report {name}"
        );
    }
}

fn assert_no_current_run(root: &Path) {
    for name in [
        "run.json",
        "egolint.sarif",
        INTELLIGENCE,
        GITIGNORE,
        RELEASE,
        "repository-continuity.json",
        "repository-presentation.json",
    ] {
        assert!(
            !report_path(root, name).exists(),
            "stale current-run output {name}"
        );
    }
}

#[test]
fn omitted_native_capability_removes_authentic_focused_evidence() {
    let root = consumer();
    emit_intelligence(root.path());
    let first = std::fs::read(report_path(root.path(), INTELLIGENCE)).unwrap();
    assert!(!first.is_empty());
    write(
        root.path(),
        ".reports/egolint/private-diagnostic.log",
        "retained private diagnostic\n",
    );
    write(
        root.path(),
        ".reports/egolint/javascript-package-quality.json",
        "unrelated family\n",
    );

    successful(&command(root.path()).arg("validate").output().unwrap());

    assert!(!report_path(root.path(), INTELLIGENCE).exists());
    let current = report(root.path(), "run.json");
    assert!(
        !serde_json::to_string(&current)
            .unwrap()
            .contains(INTELLIGENCE)
    );
    assert!(report_path(root.path(), RELEASE).is_file());
    assert_current_references(root.path());
    assert_eq!(
        std::fs::read_to_string(report_path(root.path(), "private-diagnostic.log")).unwrap(),
        "retained private diagnostic\n"
    );
    assert_eq!(
        std::fs::read_to_string(report_path(root.path(), "javascript-package-quality.json"))
            .unwrap(),
        "unrelated family\n"
    );
}

#[test]
fn early_policy_and_configuration_failures_invalidate_authentic_reports() {
    for failure in ["policy", "configuration"] {
        let root = consumer();
        emit_intelligence(root.path());
        let output = if failure == "policy" {
            command(root.path())
                .args([
                    "validate",
                    "--repository-intelligence",
                    "missing-policy.toml",
                    "--represented-commit",
                    "unknown",
                ])
                .output()
                .unwrap()
        } else {
            write(root.path(), "broken.toml", "invalid = [");
            command(root.path())
                .args(["--config", "broken.toml", "validate"])
                .output()
                .unwrap()
        };
        assert!(!output.status.success(), "{output:?}");
        assert_no_current_run(root.path());
    }
}

#[test]
fn native_and_gitignore_commands_replace_each_others_current_run() {
    let root = consumer();
    emit_intelligence(root.path());
    emit_gitignore(root.path());
    assert!(!report_path(root.path(), INTELLIGENCE).exists());
    assert!(!report_path(root.path(), RELEASE).exists());
    assert!(
        !serde_json::to_string(&report(root.path(), "run.json"))
            .unwrap()
            .contains(INTELLIGENCE)
    );

    emit_intelligence(root.path());
    assert!(!report_path(root.path(), GITIGNORE).exists());
    assert!(
        !serde_json::to_string(&report(root.path(), "run.json"))
            .unwrap()
            .contains(GITIGNORE)
    );
}

#[test]
fn failed_gitignore_invocation_clears_previous_native_report_set() {
    let root = consumer();
    emit_intelligence(root.path());
    let output = command(root.path())
        .args([
            "gitignore",
            "--policy",
            "missing.toml",
            "--evaluation-date",
            "2026-09-19",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert_no_current_run(root.path());
}

#[test]
fn read_only_commands_preserve_the_latest_authentic_reports() {
    let root = consumer();
    emit_intelligence(root.path());
    let before = current_report_bytes(root.path());
    successful(
        &command(root.path())
            .args(["schema", "report"])
            .output()
            .unwrap(),
    );
    successful(&command(root.path()).arg("plan").output().unwrap());
    assert_reports_unchanged(root.path(), before);
}

#[cfg(unix)]
#[test]
fn implicit_config_alias_to_authentic_report_is_rejected_before_cleanup() {
    use std::os::unix::fs::symlink;
    let root = consumer();
    emit_intelligence(root.path());
    let before = current_report_bytes(root.path());
    symlink(
        report_path(root.path(), "run.json"),
        root.path().join("egolint.toml"),
    )
    .unwrap();

    let output = command(root.path()).arg("validate").output().unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("report boundary"),
        "{output:?}"
    );
    assert_reports_unchanged(root.path(), before);
}

#[test]
fn selected_configuration_and_adapter_report_inputs_are_preserved() {
    for selection in [
        "implicit-adapter",
        "explicit-adapter",
        "environment-adapter",
        "cli-adapter",
        "explicit-config",
    ] {
        let root = consumer();
        emit_intelligence(root.path());
        let before = current_report_bytes(root.path());
        let mut invocation = command(root.path());
        match selection {
            "implicit-adapter" => {
                write(
                    root.path(),
                    "egolint.toml",
                    "config-version = 1\nmegalinter-config = \".reports/egolint/run.json\"\n",
                );
                invocation.arg("validate");
            }
            "explicit-adapter" => {
                write(
                    root.path(),
                    "selected.toml",
                    "config-version = 1\nmegalinter-config = \".reports/egolint/run.json\"\n",
                );
                invocation.args(["--config", "selected.toml", "validate"]);
            }
            "environment-adapter" => {
                invocation
                    .env("EGOLINT_MEGALINTER_CONFIG", ".reports/egolint/run.json")
                    .arg("validate");
            }
            "cli-adapter" => {
                invocation.args([
                    "validate",
                    "--megalinter-config",
                    ".reports/egolint/run.json",
                ]);
            }
            "explicit-config" => {
                invocation.args(["--config", ".reports/egolint/run.json", "validate"]);
            }
            _ => unreachable!("closed configuration-selection fixture"),
        }
        let output = invocation.output().unwrap();
        assert!(!output.status.success(), "{selection}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("report boundary"),
            "{selection}: {output:?}"
        );
        assert_reports_unchanged(root.path(), before);
    }
}

#[test]
fn fresh_configuration_failure_does_not_create_a_report_boundary() {
    let root = consumer();
    write(root.path(), "broken.toml", "invalid = [");

    let output = command(root.path())
        .args(["--config", "broken.toml", "validate"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(!root.path().join(".reports").exists());
}

#[cfg(unix)]
#[test]
fn non_regular_implicit_config_is_skipped_without_blocking() {
    use std::process::Stdio;
    use std::time::Duration;

    let root = consumer();
    emit_intelligence(root.path());
    let status = Command::new("mkfifo")
        .arg(root.path().join("egolint.toml"))
        .status()
        .unwrap();
    assert!(status.success());
    let mut child = command(root.path())
        .arg("validate")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            assert!(!report_path(root.path(), INTELLIGENCE).exists());
            assert_current_references(root.path());
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    panic!("implicit configuration preflight blocked on a non-regular file");
}

#[cfg(unix)]
fn simulated_lint(root: &Path, block_release_output: bool) -> Output {
    use std::os::unix::fs::PermissionsExt;
    let runtime = tempfile::tempdir().unwrap();
    let docker = runtime.path().join("docker");
    std::fs::write(
        &docker,
        r#"#!/bin/sh
set -eu
reports="${EGOLINT_TEST_REPORTS}"
for name in run.json repository-intelligence.json repository-gitignore.json repository-release.json; do
    test ! -e "${reports}/${name}"
done
printf 'observed clean report boundary\n' >"${reports}/simulator-observed.log"
if test "${EGOLINT_TEST_BLOCK_RELEASE}" = "true"; then
    mkdir "${reports}/repository-release.json"
fi
"#,
    )
    .unwrap();
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(std::iter::once(runtime.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    command(root)
        .env("PATH", path)
        .env("EGOLINT_TEST_REPORTS", report_path(root, ""))
        .env(
            "EGOLINT_TEST_BLOCK_RELEASE",
            block_release_output.to_string(),
        )
        .args([
            "lint",
            "--runtime",
            "docker",
            "--image",
            "egolint:test",
            "--pull-policy",
            "never",
            "--network",
            "none",
        ])
        .output()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn container_transition_observes_no_stale_focused_outputs() {
    let root = consumer();
    emit_intelligence(root.path());
    let output = simulated_lint(root.path(), false);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!report_path(root.path(), INTELLIGENCE).exists());
    assert!(report_path(root.path(), "simulator-observed.log").is_file());
    assert!(report_path(root.path(), RELEASE).is_file());
    assert_current_references(root.path());

    emit_gitignore(root.path());
    assert!(!report_path(root.path(), RELEASE).exists());
    assert_current_references(root.path());
}

#[cfg(unix)]
#[test]
fn aggregate_is_not_published_when_a_current_focused_write_fails() {
    let root = consumer();
    emit_intelligence(root.path());
    let output = simulated_lint(root.path(), true);
    assert!(!output.status.success(), "{output:?}");
    assert!(report_path(root.path(), "simulator-observed.log").is_file());
    assert!(report_path(root.path(), RELEASE).is_dir());
    assert!(!report_path(root.path(), "run.json").exists());
    assert!(!report_path(root.path(), INTELLIGENCE).exists());
}

#[cfg(unix)]
#[test]
fn symlinked_report_boundary_is_rejected_without_mutating_retained_evidence() {
    use std::os::unix::fs::symlink;
    let root = consumer();
    emit_intelligence(root.path());
    let bytes = std::fs::read(report_path(root.path(), INTELLIGENCE)).unwrap();
    let retained = root.path().join("retained-evidence");
    std::fs::rename(root.path().join(REPORTS), &retained).unwrap();
    symlink(&retained, root.path().join(REPORTS)).unwrap();
    let output = command(root.path()).arg("validate").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(std::fs::read(retained.join(INTELLIGENCE)).unwrap(), bytes);
}
