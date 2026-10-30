// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

//! Actual native CLI parity cases shared with the independent Aether schema check.

use std::path::Path;
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

fn write(root: &Path, path: &str, contents: impl AsRef<[u8]>) {
    let destination = root.join(path);
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::write(destination, contents).unwrap();
}

fn change(metadata: &mut Value, mutation: &Value) {
    let path = mutation["path"]
        .as_str()
        .unwrap()
        .split('.')
        .collect::<Vec<_>>();
    let mut parent = metadata;
    for field in &path[..path.len() - 1] {
        parent = if let Ok(index) = field.parse::<usize>() {
            &mut parent[index]
        } else {
            &mut parent[*field]
        };
    }
    let field = path.last().unwrap();
    match mutation["op"].as_str().unwrap() {
        "remove" => {
            parent.as_object_mut().unwrap().remove(*field);
        }
        "set" => parent[*field] = mutation["value"].clone(),
        "yaml-number" => {
            parent[*field] = format!(
                "__raw_yaml_number_{}__",
                mutation["value"].as_str().unwrap()
            )
            .into();
        }
        operation => panic!("unsupported fixture operation {operation}"),
    }
}

fn materialize_owner_contract(workspace: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "AGENTS.md",
        "vendor/hygiene/repository-continuity-policy.v1.json",
        "vendor/aether/aether.repository-continuity.v1.schema.json",
        "vendor/aether/repository-continuity.INSTRUCTION.md",
        "vendor/aether/CONTINUITY.template.md",
    ] {
        write(workspace, path, std::fs::read(root.join(path)).unwrap());
    }
    let mut policy: toml::Value = toml::from_str(include_str!(
        "../.config/dogfood/repository-continuity.toml"
    ))
    .unwrap();
    policy["repository"] = "example/public".into();
    policy["provider-projections"] = toml::Value::Array(Vec::new());
    write(workspace, "policy.toml", toml::to_string(&policy).unwrap());
}

fn assert_case(report: &Value, case: &Value, failures: &mut Vec<String>) {
    let name = case["name"].as_str().unwrap();
    let schema_valid = case["valid"].as_bool().unwrap();
    let valid = case["native_valid"].as_bool().unwrap_or(schema_valid);
    if schema_valid {
        assert!(
            report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diagnostic| diagnostic["rule_id"] != "EGO-CONTINUITY-SCHEMA-001"),
            "schema-valid input received schema diagnostic for {name}: {report}"
        );
    }
    if !schema_valid {
        assert!(
            report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["rule_id"] == "EGO-CONTINUITY-SCHEMA-001"),
            "schema-invalid input lacked schema diagnostic for {name}: {report}"
        );
    }
    let expected = if valid { "valid" } else { "invalid" };
    if report["evidence_layers"]["structural"] != expected || report["status"] != expected {
        failures.push(format!("{name}: expected {expected}; got {report}"));
    }
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|diagnostic| {
                matches!(diagnostic["severity"].as_str(), Some("warning" | "info"))
            }),
        "observe severity for {name}: {report}"
    );
}

#[test]
fn native_continuity_matches_pinned_schema_matrix_in_observe() {
    let fixture = include_str!("fixtures/repository-continuity/valid-public.md");
    let front = fixture.strip_prefix("---\n").unwrap();
    let (front, body) = front.split_once("\n---\n").unwrap();
    let mut control: Value = serde_yaml::from_str(front).unwrap();
    control["state"]["base"]["revision"] = "unborn".into();
    let matrix: Value = serde_json::from_str(include_str!(
        "fixtures/repository-continuity/schema-parity.json"
    ))
    .unwrap();
    let digest = format!(
        "{:x}",
        Sha256::digest(include_bytes!(
            "../vendor/aether/aether.repository-continuity.v1.schema.json"
        ))
    );
    assert_eq!(matrix["schema_sha256"], digest);
    let mut failures = Vec::new();
    for case in matrix["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path();
        let template = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet", "--initial-branch=main"])
                .arg(format!("--template={}", template.path().display()))
                .arg(workspace)
                .status()
                .unwrap()
                .success()
        );
        let mut metadata = control.clone();
        for mutation in case["changes"].as_array().unwrap() {
            change(&mut metadata, mutation);
        }
        write(
            workspace,
            "CONTINUITY.md",
            format!(
                "---\n{}\n---\n{body}",
                serde_json::to_string(&metadata)
                    .unwrap()
                    .replace("\"__raw_yaml_number_.nan__\"", ".nan")
                    .replace("\"__raw_yaml_number_.inf__\"", ".inf")
                    .replace("\"__raw_yaml_number_-.inf__\"", "-.inf")
            ),
        );
        materialize_owner_contract(workspace);
        let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("CI", "true")
            .arg("--workspace")
            .arg(workspace)
            .args([
                "validate",
                "--repository-continuity",
                "policy.toml",
                "--continuity-base",
                "unborn",
                "--continuity-head",
                "working-tree",
                "--continuity-disposition",
                "updated",
                "--continuity-evaluation-date",
                "2026-09-08",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "observe exit for {name}: {output:?}"
        );
        let report: Value = serde_json::from_slice(
            &std::fs::read(workspace.join(".reports/egolint/repository-continuity.json")).unwrap(),
        )
        .unwrap();
        assert_case(&report, case, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} native/schema mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
