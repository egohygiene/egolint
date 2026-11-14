//! Static matrix shell diagnostics through real Git inventory and the public CLI.
//! The workflow's commands are data; the validator must never execute them.
#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::Command;

const DIRECT: &str = "os: [ubuntu-24.04, windows-2025]";
const INCLUDE: &str = "include:\n  - os: ubuntu-24.04\n  - os: windows-2025";

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn workflow(matrix: &str, selector: &str, default: &str) -> String {
    let mut text = String::new();
    if default == "workflow" {
        text.push_str("defaults:\n  run:\n    shell: bash\n");
    }
    text.push_str("jobs:\n  check:\n");
    if default == "job" {
        text.push_str("    defaults:\n      run:\n        shell: bash\n");
    }
    text.push_str("    strategy:\n");
    if matrix.starts_with("${{") {
        text.push_str(&format!("      matrix: {matrix}\n"));
    } else {
        text.push_str("      matrix:\n");
        for line in matrix.lines() {
            text.push_str(&format!("        {line}\n"));
        }
    }
    text.push_str(&format!("    runs-on: {selector}\n    steps:\n"));
    if default == "step" {
        text.push_str("      - shell: bash\n        run: touch SHOULD_NOT_EXECUTE\n");
    } else {
        text.push_str("      - run: touch SHOULD_NOT_EXECUTE\n");
    }
    text
}

fn base_cases() -> Vec<(&'static str, String, bool)> {
    let selector = "${{ matrix.os }}";
    vec![
        ("direct", workflow(DIRECT, selector, ""), true),
        ("include", workflow(INCLUDE, selector, ""), true),
        (
            "single-direct",
            workflow("os: [ubuntu-24.04]", selector, ""),
            false,
        ),
        (
            "single-include",
            workflow(
                "include:\n  - os: ubuntu-24.04\n  - os: ubuntu-22.04",
                selector,
                "",
            ),
            false,
        ),
        (
            "exclude",
            workflow(
                "os: [ubuntu-24.04, windows-2025]\nexclude:\n  - os: windows-2025",
                selector,
                "",
            ),
            false,
        ),
        (
            "partial-exclude",
            workflow(
                "os: [ubuntu-24.04, windows-2025]\nversion: [18, 20]\nexclude:\n  - os: windows-2025\n    version: 18",
                selector,
                "",
            ),
            true,
        ),
        (
            "include-after-exclude",
            workflow(
                "os: [ubuntu-24.04, windows-2025]\nexclude:\n  - os: windows-2025\ninclude:\n  - os: windows-2025",
                selector,
                "",
            ),
            true,
        ),
        (
            "include-new-combination",
            workflow(
                "os: [ubuntu-24.04]\ninclude:\n  - os: macos-15",
                selector,
                "",
            ),
            true,
        ),
        (
            "include-overlay",
            workflow(
                "os: [ubuntu-24.04]\ninclude:\n  - extra: first\n  - extra: second",
                selector,
                "",
            ),
            false,
        ),
        (
            "nested-selector",
            workflow(
                "runner: [{os: ubuntu-24.04}, {os: windows-2025}]",
                "${{ matrix.runner.os }}",
                "",
            ),
            true,
        ),
        (
            "selector-prefix",
            workflow(
                "os: [ubuntu-24.04, windows-2025]\nos-version: [macos-15]",
                "${{ matrix.os-version }}",
                "",
            ),
            false,
        ),
        (
            "dynamic-selector",
            workflow(DIRECT, "${{ matrix.os || 'ubuntu-latest' }}", ""),
            false,
        ),
        (
            "dynamic-matrix",
            workflow("${{ fromJSON(inputs.matrix) }}", selector, ""),
            false,
        ),
        (
            "dynamic-include",
            workflow("include: '${{ inputs.rows }}'", selector, ""),
            false,
        ),
    ]
}

fn check_case(name: &str, text: &str, expected: bool) -> Vec<serde_json::Value> {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet", "--template="]);
    std::fs::create_dir_all(root.path().join(".git/info")).unwrap();
    std::fs::write(root.path().join(".git/info/exclude"), ".reports/\n").unwrap();
    std::fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    std::fs::write(root.path().join(".github/workflows/matrix.yml"), text).unwrap();
    git(root.path(), &["add", "--all"]);
    let tree = git(root.path(), &["write-tree"]);
    let mut previous = None;
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("CI", "true")
            .arg("--workspace")
            .arg(root.path())
            .arg("validate")
            .output()
            .unwrap();
        // This rule stays advisory, including when a finding is present.
        assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
        let report: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.path().join(".reports/egolint/run.json")).unwrap(),
        )
        .unwrap();
        let findings = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["rule"]["rule_id"] == "EGO-PORT-WORKFLOW-001")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(findings.len(), usize::from(expected), "{name}: {report}");
        for finding in &findings {
            assert_eq!(finding["severity"], "warning");
            assert_eq!(finding["location"]["path"], ".github/workflows/matrix.yml");
            let line = text
                .lines()
                .position(|line| line.contains("run: touch SHOULD_NOT_EXECUTE"))
                .unwrap()
                + 1;
            assert_eq!(finding["location"]["start_line"], line);
            assert_eq!(finding["location"]["start_column"], 1);
        }
        if let Some(previous) = &previous {
            assert_eq!(&findings, previous, "{name}");
        }
        previous = Some(findings);
        assert_eq!(tree, git(root.path(), &["write-tree"]));
        assert!(!root.path().join("SHOULD_NOT_EXECUTE").exists());
    }
    previous.unwrap()
}

#[test]
fn equivalent_matrix_forms_preserve_advisory_locations_defaults_and_uncertainty() {
    let selector = "${{ matrix.os }}";
    let mut cases = base_cases();
    for default in ["step", "job", "workflow"] {
        for matrix in [DIRECT, INCLUDE] {
            cases.push((default, workflow(matrix, selector, default), false));
        }
    }
    // A different job's defaults.run mapping must not displace the finding line.
    let mixed = workflow(INCLUDE, selector, "").replacen("jobs:\n", "jobs:\n  first:\n    runs-on: ubuntu-24.04\n    defaults:\n      run: # this mapping is not a step\n        shell: bash\n    steps:\n      - run: echo first\n", 1);
    cases.push(("other-job-default-location", mixed, true));

    let mut paired = Vec::new();
    for (name, text, expected) in cases {
        let mut findings = check_case(name, &text, expected);
        if matches!(name, "direct" | "include") {
            let finding = findings.pop().unwrap();
            paired.push((
                finding["rule"].clone(),
                finding["severity"].clone(),
                finding["message"].clone(),
            ));
        }
    }
    assert_eq!(paired[0], paired[1]);
}
