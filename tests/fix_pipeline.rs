// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn git(workspace: &Path, arguments: &[&str]) -> Output {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .expect("Git execution");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn output_value<'a>(stdout: &'a str, prefix: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .unwrap_or_else(|| panic!("missing {prefix:?} in {stdout}"))
}

#[test]
// The end-to-end trust-boundary regression is intentionally kept as one linear scenario.
#[allow(clippy::too_many_lines)]
fn cli_fix_preview_and_reviewed_apply_never_copy_live_or_report_data() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let workspace = temporary.path().join("workspace");
    let fake_bin = temporary.path().join("bin");
    let fake_home = temporary.path().join("home");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::create_dir_all(&fake_bin).expect("fake bin");
    std::fs::create_dir_all(&fake_home).expect("fake home");
    std::fs::write(workspace.join(".gitignore"), ".env\n.reports/\n").expect("gitignore");
    std::fs::write(workspace.join("example.txt"), "before\n").expect("tracked file");
    git(&workspace, &["init", "--quiet"]);
    git(&workspace, &["add", "--all", "--"]);
    git(
        &workspace,
        &[
            "-c",
            "user.name=Egolint Test",
            "-c",
            "user.email=egolint@invalid.example",
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "--message",
            "Fixture",
        ],
    );
    std::fs::write(workspace.join(".env"), "IGNORED_SECRET=never-copy\n").expect("ignored secret");

    let fake_docker = fake_bin.join("docker");
    std::fs::write(
        &fake_docker,
        r#"#!/usr/bin/env bash
set -o errexit
set -o nounset
set -o pipefail
candidate=""
while [[ $# -gt 0 ]]; do
  if [[ $1 == "--mount" ]]; then
    shift
    specification="$1"
    if [[ ${specification} == *"target=/tmp/lint" && ${specification} != *"target=/tmp/lint/.reports/egolint"* ]]; then
      candidate="${specification#*source=}"
      candidate="${candidate%%,target=*}"
    fi
  fi
  shift
done
test -n "${candidate}"
printf 'after\n' >"${candidate}/example.txt"
mkdir -p "${candidate}/.reports/untrusted"
printf 'PRIVATE-RAW-REPORT\n' >"${candidate}/.reports/untrusted/raw.log"
"#,
    )
    .expect("fake Docker");
    std::fs::set_permissions(&fake_docker, std::fs::Permissions::from_mode(0o755))
        .expect("fake Docker mode");
    let test_path = std::env::join_paths(std::iter::once(fake_bin.clone()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("test PATH");
    let binary = env!("CARGO_BIN_EXE_egolint");
    let preview = Command::new(binary)
        .env("PATH", &test_path)
        .env("HOME", &fake_home)
        .env("CI", "true")
        .args([
            OsStr::new("--workspace"),
            workspace.as_os_str(),
            OsStr::new("fix"),
            OsStr::new("--profile"),
            OsStr::new("holistic"),
            OsStr::new("--runtime"),
            OsStr::new("docker"),
            OsStr::new("--image"),
            OsStr::new("egolint-full:test"),
            OsStr::new("--pull-policy"),
            OsStr::new("never"),
            OsStr::new("--network"),
            OsStr::new("none"),
            OsStr::new("--enable-linter"),
            OsStr::new("PYTHON_RUFF"),
        ])
        .output()
        .expect("fix preview");
    assert!(
        preview.status.success(),
        "preview failed: {}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let stdout = String::from_utf8(preview.stdout).expect("preview stdout");
    let patch_sha256 = output_value(&stdout, "Patch SHA-256: ");
    let base_commit = output_value(&stdout, "Base commit: ");
    let post_tree = output_value(&stdout, "Expected post-tree: ");
    let patch_contents =
        std::fs::read_to_string(workspace.join(".reports/egolint/fixes.patch")).expect("fix patch");
    assert!(patch_contents.contains("+after"));
    assert!(!patch_contents.contains("IGNORED_SECRET"));
    assert!(!patch_contents.contains("PRIVATE-RAW-REPORT"));
    assert!(!patch_contents.contains(".reports/"));
    assert_eq!(
        std::fs::read_to_string(workspace.join("example.txt")).expect("original file"),
        "before\n"
    );

    let apply = Command::new(binary)
        .env("PATH", &test_path)
        .env("HOME", &fake_home)
        .env("CI", "true")
        .args([
            OsStr::new("--workspace"),
            workspace.as_os_str(),
            OsStr::new("apply-fix"),
            OsStr::new("--patch-sha256"),
            OsStr::new(patch_sha256),
            OsStr::new("--base-commit"),
            OsStr::new(base_commit),
            OsStr::new("--post-tree"),
            OsStr::new(post_tree),
        ])
        .output()
        .expect("reviewed apply");
    assert!(
        apply.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(workspace.join("example.txt")).expect("applied file"),
        "after\n"
    );
    assert_eq!(
        String::from_utf8(git(&workspace, &["diff", "--cached", "--name-only"]).stdout)
            .expect("staged path"),
        "example.txt\n"
    );
}
