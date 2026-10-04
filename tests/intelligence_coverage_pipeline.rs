//! Owner fixtures, compatibility, privacy, uncertainty, and the actual CLI.
use std::path::Path;
use std::process::{Command, Output};

use egolint::intelligence_coverage::{
    CoverageKnowledge, CoverageReport, CoverageStatus, MAX_INPUT_BYTES, validate_coverage,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FIXTURES: &str = "tests/fixtures/repository-intelligence/coverage";

fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(FIXTURES)
                .join(format!("{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}
fn check(value: &Value) -> CoverageReport {
    validate_coverage(&serde_json::to_vec(value).unwrap()).unwrap()
}
fn cli(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_egolint"))
        .args(["intelligence", "validate-coverage", "--input"])
        .arg(path)
        .output()
        .unwrap()
}
fn cli_value(value: &Value) -> Output {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("capture.json");
    let bytes = serde_json::to_vec(value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let result = cli(&path);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    result
}

#[test]
fn pinned_owner_fixtures_pass_library_and_cli_without_mutation() {
    for name in [
        "full",
        "roadmap-only",
        "provider-denied",
        "observed-empty",
        "stale",
        "partial",
        "failed",
        "not-applicable",
    ] {
        let input = fixture(name);
        let result = check(&input);
        assert_eq!(result.status, CoverageStatus::Valid, "{name}: {result:?}");
        assert_eq!(result.coverage, CoverageKnowledge::Explicit);
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["domains"], input["collection_coverage"]);
        assert_eq!(json["scope"], "collection-coverage");
        let output = cli_value(&input);
        assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            json
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn old_complete_input_keeps_unknown_completeness_and_cannot_smuggle_coverage() {
    let legacy = fixture("complete-quest");
    let result = check(&legacy);
    assert_eq!(result.status, CoverageStatus::Valid);
    assert_eq!(result.coverage, CoverageKnowledge::LegacyUnknown);
    assert!(result.domains.is_none());
    assert_eq!(cli_value(&legacy).status.code(), Some(0));
    let mut candidate = fixture("full");
    candidate["contract_version"] = json!("1.0.0-alpha.1");
    assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
    for version in ["1.0.0-alpha.3", "1.0.0", "protected/hidden"] {
        candidate["contract_version"] = json!(version);
        let report = check(&candidate);
        assert_eq!(report.status, CoverageStatus::Invalid);
        assert!(!serde_json::to_string(&report).unwrap().contains(version));
    }
}

#[test]
fn empty_arrays_never_upgrade_unknown_partial_denied_or_failed_collection() {
    for (name, state) in [
        ("roadmap-only", "uncollected"),
        ("provider-denied", "unavailable"),
        ("partial", "partial"),
        ("failed", "failed"),
        ("observed-empty", "observed_empty"),
    ] {
        let result = serde_json::to_value(check(&fixture(name))).unwrap();
        assert_eq!(result["domains"]["issues"]["collection"], state);
    }
    let result = serde_json::to_value(check(&fixture("stale"))).unwrap();
    assert_eq!(result["domains"]["issues"]["freshness"], "stale");
    for reason in ["filtered", "truncated", "incomplete"] {
        let mut candidate = fixture("partial");
        candidate["collection_coverage"]["issues"]["reason"] = json!(reason);
        assert_eq!(check(&candidate).status, CoverageStatus::Valid);
        candidate["collection_coverage"]["issues"]["collection"] = json!("observed_empty");
        assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
    }
}

#[test]
fn every_domain_and_all_four_fields_are_required() {
    let source = fixture("full");
    for domain in source["collection_coverage"].as_object().unwrap().keys() {
        let mut candidate = source.clone();
        candidate["collection_coverage"]
            .as_object_mut()
            .unwrap()
            .remove(domain);
        assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
        for field in ["collection", "freshness", "reason", "observed_at"] {
            let mut candidate = source.clone();
            candidate["collection_coverage"][domain]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
        }
    }
}

#[test]
fn false_empty_and_false_observed_claims_are_rejected() {
    let source = fixture("full");
    for domain in source["collection_coverage"].as_object().unwrap().keys() {
        for name in [
            "observed-empty",
            "provider-denied",
            "failed",
            "roadmap-only",
            "not-applicable",
        ] {
            let mut candidate = source.clone();
            let other = fixture(name);
            // Use a valid empty-domain claim; changing its domain must not hide records.
            let key = if name == "not-applicable" {
                "deployments"
            } else {
                "issues"
            };
            candidate["collection_coverage"][domain] = other["collection_coverage"][key].clone();
            assert_eq!(
                check(&candidate).status,
                CoverageStatus::Invalid,
                "{name} {domain}"
            );
        }
    }
    let mut candidate = fixture("observed-empty");
    candidate["collection_coverage"]["issues"]["collection"] = json!("observed");
    assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
}

#[test]
fn denied_or_malformed_input_never_echoes_protected_payloads_counts_or_paths() {
    let forbidden = "protected/hidden-971";
    for key in ["repository", "count", "url", "message", "extensions"] {
        let mut candidate = fixture("provider-denied");
        candidate["collection_coverage"]["issues"][key] = json!(forbidden);
        let result = cli_value(&candidate);
        assert_eq!(result.status.code(), Some(1));
        assert!(
            !String::from_utf8(result.stdout)
                .unwrap()
                .contains(forbidden)
        );
        assert!(
            !String::from_utf8(result.stderr)
                .unwrap()
                .contains(forbidden)
        );
    }
    for visibility in ["private", "internal", "unknown"] {
        let mut candidate = fixture("full");
        candidate["repository"] = json!(forbidden);
        candidate["visibility"] = json!(visibility);
        let result = cli_value(&candidate);
        assert_eq!(result.status.code(), Some(2));
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["status"], "unavailable");
        assert!(report["domains"].is_null());
        assert!(!report.to_string().contains(forbidden));
    }
    for collection in ["sources", "entities", "events"] {
        let mut candidate = fixture("full");
        candidate[collection][0]["visibility"] = json!("private");
        assert_eq!(check(&candidate).status, CoverageStatus::Unavailable);
    }
}

#[test]
fn malformed_values_and_duplicate_json_keys_fail_closed_without_panics() {
    for value in [
        Value::Null,
        json!([]),
        json!({}),
        json!(true),
        json!(42),
        json!("protected/hidden"),
    ] {
        let mut candidate = fixture("full");
        candidate["collection_coverage"] = value.clone();
        assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
        for field in ["collection", "freshness", "reason", "observed_at"] {
            let mut candidate = fixture("full");
            candidate["collection_coverage"]["issues"][field] = value.clone();
            assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
        }
    }
    let input = serde_json::to_string(&fixture("full")).unwrap();
    let duplicate = input.replacen(
        "\"collection\":\"observed\"",
        "\"collection\":\"unavailable\",\"collection\":\"observed\"",
        1,
    );
    assert_ne!(input, duplicate);
    assert_eq!(
        validate_coverage(duplicate.as_bytes()).unwrap().status,
        CoverageStatus::Invalid
    );
    for input in [b"{".as_slice(), b"\xff", b"null", b"[]", b"true"] {
        assert_ne!(
            validate_coverage(input).unwrap().status,
            CoverageStatus::Valid
        );
    }
}

#[test]
fn observation_times_are_real_utc_and_cannot_be_in_the_future() {
    for time in [
        "2026-02-30T12:00:00Z",
        "2026-08-26",
        "2026-08-26 12:00:00Z",
        "2026-08-26T24:00:00Z",
        "2026-08-26T12:00:60Z",
        "2026-08-26T12:00:00+00:00",
        "2099-01-01T00:00:00Z",
        "保护数据",
    ] {
        let mut candidate = fixture("full");
        candidate["collection_coverage"]["issues"]["observed_at"] = json!(time);
        assert_eq!(check(&candidate).status, CoverageStatus::Invalid, "{time}");
    }
    for (observation, claim, valid) in [
        (".1", ".1000", true),
        (".1", ".1001", false),
        (".1", ".09", true),
        ("", ".000", true),
    ] {
        let mut candidate = fixture("full");
        candidate["observed_at"] = json!(format!("2026-09-01T12:00:00{observation}Z"));
        candidate["collection_coverage"]["issues"]["observed_at"] =
            json!(format!("2026-09-01T12:00:00{claim}Z"));
        assert_eq!(check(&candidate).status == CoverageStatus::Valid, valid);
    }
}

#[test]
fn root_context_and_history_are_not_inferred_from_external_records() {
    let mut candidate = fixture("roadmap-only");
    let mut external = fixture("full")["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "issue")
        .unwrap()
        .clone();
    external["repository"] = json!("egohygiene/example");
    external["id"] = json!("ri:egohygiene/example:issue:1");
    candidate["entities"].as_array_mut().unwrap().push(external);
    assert_eq!(check(&candidate).status, CoverageStatus::Valid);
    let result = serde_json::to_value(check(&candidate)).unwrap();
    assert_eq!(result["domains"]["issues"]["collection"], "uncollected");
    assert_eq!(result["domains"]["history"]["collection"], "uncollected");
    candidate["events"] = fixture("full")["events"].clone();
    assert_eq!(check(&candidate).status, CoverageStatus::Invalid); // dangling context cannot claim emptiness
}

#[test]
fn invalid_context_and_unknown_domains_cannot_suppress_domain_record_presence() {
    for field in [
        "repository",
        "projection_id",
        "represented_commit",
        "entities",
        "events",
        "sources",
    ] {
        let mut candidate = fixture("full");
        candidate.as_object_mut().unwrap().remove(field);
        assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
    }
    let mut candidate = fixture("full");
    candidate["entities"][0]["kind"] = json!("made_up");
    assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
    candidate = fixture("full");
    candidate["collection_coverage"]["protected/hidden"] = json!({"count": 971});
    assert_eq!(check(&candidate).status, CoverageStatus::Invalid);
    assert!(
        !serde_json::to_string(&check(&candidate))
            .unwrap()
            .contains("971")
    );
}

#[test]
fn cli_size_regular_file_and_missing_input_errors_are_bounded() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("private-name.json");
    for input in [&path, &root.path().to_path_buf()] {
        let output = cli(input);
        assert_eq!(output.status.code(), Some(2));
        assert!(
            !String::from_utf8(output.stderr)
                .unwrap()
                .contains(root.path().to_str().unwrap())
        );
    }
    std::fs::write(&path, vec![b' '; MAX_INPUT_BYTES + 1]).unwrap();
    assert_eq!(cli(&path).status.code(), Some(2));
    assert_eq!(
        validate_coverage(&vec![b' '; MAX_INPUT_BYTES + 1])
            .unwrap()
            .status,
        CoverageStatus::Invalid
    );
}

#[cfg(unix)]
#[test]
fn symlink_inputs_are_not_followed() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.json");
    std::fs::write(&source, serde_json::to_vec(&fixture("full")).unwrap()).unwrap();
    let link = root.path().join("link.json");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert_eq!(cli(&link).status.code(), Some(2));
}

#[test]
fn pinned_fixture_bytes_and_every_embedded_owner_artifact_match_the_lock() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock: Value = serde_json::from_str(include_str!(
        "../.config/rules/repository-intelligence-coverage-sources.v1.json"
    ))
    .unwrap();
    for section in ["artifacts", "fixtures"] {
        for item in lock[section].as_array().unwrap() {
            let key = if section == "artifacts" {
                "vendored_path"
            } else {
                "fixture_path"
            };
            let bytes = std::fs::read(root.join(item[key].as_str().unwrap())).unwrap();
            assert_eq!(item["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        }
    }
    let schema_output = Command::new(env!("CARGO_BIN_EXE_egolint"))
        .args(["schema", "intelligence-coverage-report"])
        .output()
        .unwrap();
    assert!(schema_output.status.success());
    assert_eq!(
        schema_output.stdout,
        std::fs::read(root.join("schemas/intelligence-coverage-report.schema.json")).unwrap()
    );
}

#[test]
fn output_is_deterministic_across_paths_and_collection_order() {
    let mut input = fixture("full");
    let first = cli_value(&input).stdout;
    input["entities"].as_array_mut().unwrap().reverse();
    input["events"].as_array_mut().unwrap().reverse();
    input["sources"].as_array_mut().unwrap().reverse();
    assert_eq!(first, cli_value(&input).stdout);
}
