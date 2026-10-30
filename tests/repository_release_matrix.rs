//! Source-only current-v1 release profile fixtures, independent of publication runtimes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Deserialize;
use serde_json::{Value, json};
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

fn native_command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_egolint"));
    for (name, _) in
        std::env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("EGOLINT_"))
    {
        command.env_remove(name);
    }
    command
        .args(["--workspace"])
        .arg(root)
        .args(["validate", "--runtime", "docker"]);
    command
}

fn validate(root: &Path, adoption: Option<&str>) -> Output {
    let mut command = native_command(root);
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

fn declaration(root: &Path) -> Value {
    serde_json::from_slice(
        &std::fs::read(root.join(".egohygiene/release.json")).expect("declaration"),
    )
    .expect("declaration JSON")
}

fn save_declaration(root: &Path, declaration: &Value) {
    write(
        root,
        ".egohygiene/release.json",
        serde_json::to_vec_pretty(declaration).expect("JSON"),
    );
}

fn remove(root: &Path, path: &str) {
    let path = root.join(path);
    if path.exists() {
        std::fs::remove_file(path).expect("remove selected fixture evidence");
    }
}

fn scenario(root: &Path, fixture: &Fixture, state: &str) -> Option<&'static str> {
    match state {
        "compliant" => None,
        "advisory_migration" => {
            remove(root, "CHANGELOG.md");
            Some("advisory")
        }
        "unavailable" => {
            remove(root, &fixture.negative_path);
            None
        }
        "external" => {
            let mut document = declaration(root);
            document["evidence"]["source"] = json!("external");
            document["delivery"]["channels"][0]["state"] = json!("external");
            save_declaration(root, &document);
            if fixture.lifecycle != "archived" {
                write(
                    root,
                    ".github/workflows/release.yml",
                    "name: External execution fixture\non:\n  workflow_dispatch:\njobs:\n  release:\n    uses: example/external-release/.github/workflows/publish.yml@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
                );
            }
            None
        }
        "invalid" => {
            let path = root.join(&fixture.negative_path);
            let text = std::fs::read_to_string(&path).expect("negative source");
            assert!(text.contains("0.1.0"), "{} negative authority", fixture.id);
            std::fs::write(path, text.replace("0.1.0", "01.0.0")).expect("nonconforming source");
            None
        }
        "not_applicable" => {
            for path in [
                "AGENTS.md",
                "CHANGELOG.md",
                "Taskfile.yml",
                ".github/workflows/release.yml",
            ] {
                remove(root, path);
            }
            for path in &fixture.authority_files {
                remove(root, path);
            }
            Some("not-applicable")
        }
        _ => panic!("unknown authored scenario {state}"),
    }
}

fn focused_projection(focused: &Value) -> Value {
    let checks: serde_json::Map<String, Value> = focused["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|check| {
            (
                check["slot_id"].as_str().expect("slot").to_owned(),
                json!([check["state"], check["requirement"]]),
            )
        })
        .collect();
    json!({
        "state": focused["state"],
        "repository_profile": focused["applicability"]["repository_profile"],
        "lifecycle": focused["applicability"]["lifecycle"],
        "adoption": focused["applicability"]["adoption_state"],
        "summary": focused["summary"],
        "checks": checks,
    })
}

#[test]
fn all_fifty_four_profile_state_cases_match_reviewed_focused_goldens() {
    let golden: Value = serde_json::from_slice(
        &std::fs::read(fixture_root().join("expected-reports.json")).expect("focused goldens"),
    )
    .expect("golden JSON");
    let cases = golden["cases"].as_object().expect("golden cases");
    assert_eq!(golden["schema_version"], 1);
    let states = [
        "compliant",
        "advisory_migration",
        "unavailable",
        "external",
        "invalid",
        "not_applicable",
    ];
    let mut visited = std::collections::BTreeSet::new();
    for fixture in corpus().fixtures {
        for state in states {
            let id = format!("{}/{state}", fixture.id);
            assert!(visited.insert(id.clone()));
            let expected = cases
                .get(&id)
                .unwrap_or_else(|| panic!("missing golden {id}"));
            let root = consumer(&fixture);
            let adoption = scenario(root.path(), &fixture, state);
            let output = validate(root.path(), adoption);
            assert_eq!(
                output.status.code(),
                expected["exit_code"]
                    .as_i64()
                    .map(|code| i32::try_from(code).expect("exit code")),
                "{id}: {}",
                output_text(&output)
            );
            let focused = report(root.path(), "repository-release.json");
            assert_eq!(focused_projection(&focused), expected["focused"], "{id}");
            assert_eq!(focused["boundary"], golden["boundary"], "{id}");
            assert!(
                output_text(&output).contains(&format!(
                    "repository-release: {}",
                    expected["focused"]["state"].as_str().expect("golden state")
                )),
                "{id}: human-readable state agrees"
            );
            assert_projections(root.path(), &id, expected);
        }
    }
    assert_eq!(visited.len(), 54);
    assert_eq!(
        cases.len(),
        visited.len(),
        "no missing or unused golden cells"
    );
}

fn assert_projections(root: &Path, id: &str, expected: &Value) {
    let run = report(root, "run.json");
    let tool = run["tool_results"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["tool_id"] == "EGOLINT_REPOSITORY_RELEASE")
        .expect("release tool");
    let projection = json!({"status": tool["status"], "enforcement": tool["enforcement"], "finding_count": tool["finding_count"], "warning_count": tool["warning_count"]});
    assert_eq!(projection, expected["tool"], "{id}");
    let findings: Vec<_> = run["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|finding| finding["rule"]["tool_id"] == "EGOLINT_REPOSITORY_RELEASE")
        .collect();
    let projected: Vec<_> = findings.iter().map(|finding| json!({"rule": finding["rule"]["rule_id"], "severity": finding["severity"], "path": finding["location"]["path"]})).collect();
    assert_eq!(json!(projected), expected["findings"], "{id}");
    let focused = report(root, "repository-release.json");
    for finding in &findings {
        let check = focused["checks"]
            .as_array()
            .expect("focused checks")
            .iter()
            .find(|check| check["rule_id"] == finding["rule"]["rule_id"])
            .expect("focused finding rule");
        assert_eq!(finding["location"], check["location"], "{id}");
    }
    let sarif = report(root, "egolint.sarif");
    let results: Vec<_> = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results")
        .iter()
        .filter(|result| result["properties"]["egolintToolId"] == "EGOLINT_REPOSITORY_RELEASE")
        .collect();
    assert_eq!(results.len(), findings.len(), "{id}");
    for (finding, result) in findings.iter().zip(results) {
        let rule = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("SARIF rules")
            .iter()
            .find(|rule| rule["id"] == result["ruleId"])
            .expect("SARIF result references a declared rule");
        assert_eq!(rule["name"], finding["rule"]["rule_id"], "{id}");
        assert_eq!(
            rule["properties"]["egolintToolId"], finding["rule"]["tool_id"],
            "{id}"
        );
        assert_eq!(
            result["properties"]["egolintFindingId"], finding["id"],
            "{id}"
        );
        assert_eq!(
            result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            finding["location"]["path"],
            "{id}"
        );
        assert_eq!(result["level"], finding["severity"], "{id}");
    }
}

fn assert_check(root: &Path, state: &str, slot: &str, check_state: &str) {
    let focused = report(root, "repository-release.json");
    assert_eq!(focused["state"], state, "{slot}");
    assert_eq!(focused["boundary"]["network_access"], "not_performed");
    assert_eq!(focused["boundary"]["external_publication"], "not_verified");
    let check = focused["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["slot_id"] == slot)
        .expect("selected check");
    assert_eq!(check["state"], check_state, "{slot}");
}

#[test]
fn released_local_authorities_have_matching_and_drifted_controls() {
    for fixture in corpus().fixtures {
        if matches!(fixture.id.as_str(), "archived" | "workspace") {
            continue;
        }
        let root = consumer(&fixture);
        let mut document = declaration(root.path());
        document["release"]["state"] = json!("released");
        save_declaration(root.path(), &document);
        let output = validate(root.path(), None);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} match: {}",
            fixture.id,
            output_text(&output)
        );
        assert_check(root.path(), "compliant", "version_authority", "passed");
        let path = root.path().join(&fixture.negative_path);
        let text = std::fs::read_to_string(&path).expect("authority");
        assert!(text.contains("0.1.0"));
        std::fs::write(path, text.replace("0.1.0", "0.2.0")).expect("semantic authority drift");
        let output = validate(root.path(), None);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{} drift: {}",
            fixture.id,
            output_text(&output)
        );
        assert_check(root.path(), "invalid", "version_authority", "failed");
        let run = report(root.path(), "run.json");
        assert!(
            run["findings"]
                .as_array()
                .expect("findings")
                .iter()
                .any(|finding| {
                    finding["rule"]["rule_id"] == "EGOLINT_RELEASE_VERSION_AUTHORITY"
                        && finding["location"]["path"] == "CHANGELOG.md"
                        && finding["message"]
                            .as_str()
                            .expect("message")
                            .contains("differs")
                })
        );
    }
}

#[test]
fn workspace_versions_remain_independent_without_inventing_global_drift() {
    let fixture = corpus()
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == "workspace")
        .expect("workspace fixture");
    let root = consumer(&fixture);
    let mut document = declaration(root.path());
    document["release"]["state"] = json!("released");
    save_declaration(root.path(), &document);
    for version in ["0.1.0", "3.2.1"] {
        write(
            root.path(),
            "packages/client/package.json",
            format!("{{\"name\":\"client\",\"version\":\"{version}\"}}"),
        );
        let output = validate(root.path(), None);
        assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
        assert_check(root.path(), "compliant", "version_authority", "passed");
    }
}

#[test]
fn malformed_tag_and_declaration_shapes_remain_invalid_under_required_adoption() {
    let fixture = corpus()
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == "rust")
        .expect("Rust fixture");
    for (pointer, value) in [
        ("/release/tag_prefix", json!("release-")),
        ("/release/immutable_tags", json!(false)),
        ("/components", json!([])),
        ("/automation/github/state", json!("external")),
    ] {
        let root = consumer(&fixture);
        let mut document = declaration(root.path());
        *document.pointer_mut(pointer).expect("declaration field") = value;
        save_declaration(root.path(), &document);
        let output = validate(root.path(), Some("required"));
        assert_eq!(
            output.status.code(),
            Some(1),
            "{pointer}: {}",
            output_text(&output)
        );
        let focused = report(root.path(), "repository-release.json");
        assert_eq!(focused["state"], "invalid", "{pointer}");
        assert_eq!(focused["declaration"]["state"], "invalid", "{pointer}");
        assert_eq!(focused["boundary"]["external_publication"], "not_verified");
    }
}

#[test]
fn entrypoint_source_boundaries_preserve_unavailable_external_and_invalid() {
    let fixture = corpus()
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == "rust")
        .expect("Rust fixture");
    let cases = [
        (
            "workflow-missing",
            "unavailable",
            "manual_workflow",
            "unavailable",
        ),
        ("workflow-malformed", "invalid", "manual_workflow", "failed"),
        ("workflow-unpinned", "invalid", "manual_workflow", "failed"),
        ("workflow-push", "invalid", "manual_workflow", "failed"),
        (
            "workflow-unavailable",
            "unavailable",
            "manual_workflow",
            "unavailable",
        ),
        (
            "external-workflow-unpinned",
            "invalid",
            "manual_workflow",
            "failed",
        ),
        (
            "task-missing",
            "unavailable",
            "task_handoffs",
            "unavailable",
        ),
        ("task-malformed", "invalid", "task_handoffs", "failed"),
        (
            "task-external",
            "unavailable",
            "task_handoffs",
            "unavailable",
        ),
        ("task-echo", "unavailable", "task_handoffs", "unavailable"),
        (
            "task-wrong-workflow",
            "unavailable",
            "task_handoffs",
            "unavailable",
        ),
    ];
    for (case, state, slot, check_state) in cases {
        let root = consumer(&fixture);
        change_entrypoint(root.path(), &fixture, case);
        let output = validate(root.path(), None);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{case}: {}",
            output_text(&output)
        );
        assert_check(root.path(), state, slot, check_state);
    }
}

fn change_entrypoint(root: &Path, fixture: &Fixture, case: &str) {
    let workflow = ".github/workflows/release.yml";
    match case {
        "workflow-missing" => remove(root, workflow),
        "workflow-malformed" => write(root, workflow, "on: [\n"),
        "workflow-unpinned" => {
            let text = std::fs::read_to_string(root.join(workflow)).expect("workflow");
            write(
                root,
                workflow,
                text.replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "v4"),
            );
        }
        "workflow-push" => {
            let text = std::fs::read_to_string(root.join(workflow)).expect("workflow");
            write(
                root,
                workflow,
                text.replace("  workflow_dispatch:", "  workflow_dispatch:\n  push:"),
            );
        }
        "workflow-unavailable" => {
            let mut document = declaration(root);
            document["automation"]["github"]["state"] = json!("unavailable");
            save_declaration(root, &document);
        }
        "external-workflow-unpinned" => {
            scenario(root, fixture, "external");
            let text = std::fs::read_to_string(root.join(workflow)).expect("external workflow");
            write(
                root,
                workflow,
                text.replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "main"),
            );
        }
        "task-missing" => remove(root, "Taskfile.yml"),
        "task-malformed" => write(root, "Taskfile.yml", "tasks: [\n"),
        "task-external" | "task-echo" | "task-wrong-workflow" => {
            let command = match case {
                "task-external" => json!({"task": "external:publish"}),
                "task-echo" => json!("echo gh workflow run .github/workflows/release.yml"),
                _ => json!(
                    "gh workflow run .github/workflows/other.yml # .github/workflows/release.yml"
                ),
            };
            let text = std::fs::read_to_string(root.join("Taskfile.yml")).expect("Taskfile");
            let mut taskfile: Value = serde_yaml::from_str(&text).expect("Taskfile data");
            taskfile["tasks"]["release:publish"]["cmds"] = json!([command]);
            if case == "task-external" {
                taskfile["includes"] = json!({"external": "https://example.invalid/Taskfile.yml"});
                let mut document = declaration(root);
                document["evidence"]["source"] = json!("external");
                save_declaration(root, &document);
            }
            write(
                root,
                "Taskfile.yml",
                serde_json::to_vec(&taskfile).expect("Task YAML-compatible JSON"),
            );
        }
        _ => panic!("unknown authored entrypoint case {case}"),
    }
}

#[cfg(unix)]
#[test]
fn contract_and_archived_validation_never_initializes_execution_runtimes() {
    use std::os::unix::fs::PermissionsExt;

    for fixture in corpus()
        .fixtures
        .into_iter()
        .filter(|fixture| matches!(fixture.id.as_str(), "contract-only" | "archived"))
    {
        let root = consumer(&fixture);
        let traps = tempfile::tempdir().expect("runtime traps");
        let marker = traps.path().join("invoked");
        for executable in [
            "docker", "podman", "node", "pnpm", "python3", "gh", "task", "cargo",
        ] {
            let path = traps.path().join(executable);
            std::fs::write(&path, "#!/bin/sh\nprintf 'unexpected runtime initialization' > \"$EGOLINT_TEST_RUNTIME_MARKER\"\nexit 93\n").expect("executable trap");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                .expect("trap permissions");
        }
        let mut paths = vec![traps.path().to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").expect("PATH"),
        ));
        let control = Command::new(traps.path().join("docker"))
            .arg("--version")
            .env("EGOLINT_TEST_RUNTIME_MARKER", &marker)
            .output()
            .expect("trusted trap positive control");
        assert_eq!(control.status.code(), Some(93));
        assert!(marker.exists(), "the trusted trap records invocation");
        std::fs::remove_file(&marker).expect("clear positive control marker");
        let output = native_command(root.path())
            .env("PATH", std::env::join_paths(paths).expect("trap PATH"))
            .env("EGOLINT_TEST_RUNTIME_MARKER", &marker)
            .output()
            .expect("native validation with runtime traps");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: {}",
            fixture.id,
            output_text(&output)
        );
        assert!(
            !marker.exists(),
            "{} initialized an irrelevant execution runtime",
            fixture.id
        );
        assert_check(root.path(), "compliant", "version_authority", "passed");
    }
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
