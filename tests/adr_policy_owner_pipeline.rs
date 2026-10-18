//! Exercise owner selection and unchanged consumer inheritance through the native CLI.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let target = root.join(path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(target, content).unwrap();
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn workspace(owner: bool) -> TempDir {
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
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repository-intelligence");
    for path in [
        "ROADMAP.md",
        "docs/decisions/README.md",
        "docs/decisions/ADR-001-validation-contract.md",
        "docs/decisions/ADR-002-legacy-projection.md",
        "docs/decisions/ADR-003-versioned-projection.md",
    ] {
        write(
            root.path(),
            path,
            std::fs::read(fixtures.join("valid").join(path)).unwrap(),
        );
    }
    let selection = if owner { "policy-owner" } else { "valid" };
    write(
        root.path(),
        "policy.toml",
        std::fs::read(fixtures.join(selection).join("policy.toml")).unwrap(),
    );
    if owner {
        let authority =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/hygiene/adr-policy-owner");
        for path in [
            "docs/decisions/POLICY.md",
            "docs/decisions/RATIFICATION.md",
            "schemas/architecture-decision.v1.schema.json",
            "schemas/architecture-decision-policy-reference.v1.schema.json",
            "catalog/contracts.yaml",
        ] {
            write(
                root.path(),
                path,
                std::fs::read(authority.join(path)).unwrap(),
            );
        }
    } else {
        write(
            root.path(),
            "docs/decisions/policy-reference.json",
            std::fs::read(fixtures.join("valid/docs/decisions/policy-reference.json")).unwrap(),
        );
    }
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
            "docs: add synthetic ADR fixture",
        ],
    );
    root
}

fn source_digests(root: &Path) -> BTreeMap<String, String> {
    git(root, &["ls-files", "-z"])
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| {
            let digest = format!(
                "{:x}",
                Sha256::digest(std::fs::read(root.join(path)).unwrap())
            );
            (path.to_owned(), digest)
        })
        .collect()
}

fn cli(root: &Path) -> (Output, Value) {
    let revision = git(root, &["rev-parse", "--verify", "HEAD"]);
    let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root)
        .args([
            "validate",
            "--repository-intelligence",
            "policy.toml",
            "--represented-commit",
            revision.trim(),
        ])
        .output()
        .unwrap();
    let path = root.join(".reports/egolint/repository-intelligence.json");
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{error}: {output:?}"));
    (output, serde_json::from_slice(&bytes).unwrap())
}

#[test]
fn owner_and_ordinary_consumer_emit_valid_reports_without_source_mutation() {
    for owner in [true, false] {
        let root = workspace(owner);
        let before = source_digests(root.path());
        let (output, report) = cli(root.path());
        assert!(output.status.success(), "{output:?}\n{report}");
        assert_eq!(report["status"], "valid");
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["catalog_version"], "0.1.0-alpha.4");
        assert_eq!(report["summary"]["adrs_inspected"], 3);
        assert_eq!(before, source_digests(root.path()));
        let (_, repeated) = cli(root.path());
        assert_eq!(report, repeated);
        assert!(root.path().join(".reports/egolint/run.json").is_file());
        assert!(root.path().join(".reports/egolint/egolint.sarif").is_file());
    }
}

#[test]
fn modified_owner_authority_blocks_and_remains_unmodified_by_validation() {
    let root = workspace(true);
    write(
        root.path(),
        "docs/decisions/POLICY.md",
        "# Spoofed policy\n",
    );
    let before = source_digests(root.path());
    let (output, report) = cli(root.path());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(report["status"], "invalid");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["rule_id"] == "EGO-INTEL-CONTRACT-001"
                && item["location"]["path"] == "docs/decisions/POLICY.md")
    );
    assert_eq!(before, source_digests(root.path()));
}

#[test]
fn missing_consumer_reference_and_unapproved_owner_decision_still_block() {
    let consumer = workspace(false);
    std::fs::remove_file(consumer.path().join("docs/decisions/policy-reference.json")).unwrap();
    let (output, report) = cli(consumer.path());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["rule_id"] == "EGO-INTEL-ADR-METADATA-001")
    );

    let owner = workspace(true);
    let path = owner
        .path()
        .join("docs/decisions/ADR-001-validation-contract.md");
    let raw = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, raw.replace("status: proposed", "status: accepted")).unwrap();
    let before = source_digests(owner.path());
    let (output, report) = cli(owner.path());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["rule_id"] == "EGO-INTEL-ADR-LIFECYCLE-001")
    );
    assert_eq!(before, source_digests(owner.path()));
}
