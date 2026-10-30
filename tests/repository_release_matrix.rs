//! Source-only current-v1 release profile fixtures, independent of publication runtimes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Deserialize;
use serde_json::Value;
use tempfile::TempDir;

const FIXTURES: &str = "tests/fixtures/repository-release";
const REPORT_DIRECTORY: &str = ".reports/egolint";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    id: String,
    profile: String,
    lifecycle: String,
    authority_files: Vec<String>,
    negative_path: String,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES)
}

fn corpus() -> Corpus {
    let corpus: Corpus = serde_json::from_slice(
        &std::fs::read(fixture_root().join("profiles.json")).expect("profile corpus"),
    )
    .expect("closed profile corpus");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.fixtures.len(), 9);
    corpus
}

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    std::fs::write(path, content).expect("fixture file");
}

fn copy(root: &Path, source: &str, destination: &str) {
    write(
        root,
        destination,
        std::fs::read(fixture_root().join(source)).expect("authored fixture source"),
    );
}

fn consumer(fixture: &Fixture) -> TempDir {
    let root = tempfile::tempdir().expect("temporary consumer");
    let template = tempfile::tempdir().expect("empty Git template");
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args(["init", "--quiet"])
            .arg(format!("--template={}", template.path().display()))
            .status()
            .expect("initialize synthetic consumer")
            .success()
    );
    copy(root.path(), "common/AGENTS.md", "AGENTS.md");
    copy(root.path(), "common/CHANGELOG.md", "CHANGELOG.md");
    if fixture.lifecycle != "archived" {
        copy(root.path(), "common/Taskfile.yml", "Taskfile.yml");
        copy(
            root.path(),
            "common/release-workflow.yml",
            ".github/workflows/release.yml",
        );
    }
    copy(
        root.path(),
        &format!("{}/declaration.json", fixture.id),
        ".egohygiene/release.json",
    );
    for path in &fixture.authority_files {
        copy(root.path(), &format!("{}/{path}", fixture.id), path);
    }
    root
}

fn validate(root: &Path, adoption: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    command
        .args(["--workspace"])
        .arg(root)
        .args(["validate", "--runtime", "docker"]);
    if let Some(adoption) = adoption {
        command.args(["--release-adoption-state", adoption]);
    }
    command.output().expect("native source validation")
}

fn report(root: &Path, name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(root.join(REPORT_DIRECTORY).join(name)).expect("native report"),
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
fn nine_authored_profiles_have_compliant_native_baselines() {
    for fixture in corpus().fixtures {
        let root = consumer(&fixture);
        assert!(!fixture.negative_path.is_empty());
        let output = validate(root.path(), None);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: {}",
            fixture.id,
            output_text(&output)
        );
        let focused = report(root.path(), "repository-release.json");
        assert_eq!(focused["state"], "compliant", "{}", fixture.id);
        assert_eq!(
            focused["applicability"]["repository_profile"],
            fixture.profile
        );
        assert_eq!(focused["applicability"]["lifecycle"], fixture.lifecycle);
        assert_eq!(focused["summary"]["validation_complete"], true);
        assert_eq!(focused["boundary"]["network_access"], "not_performed");
        assert_eq!(focused["boundary"]["external_publication"], "not_verified");
    }
}
