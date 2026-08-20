// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

//! Executable compatibility contract for the first extracted Empathy baseline.

use std::path::{Path, PathBuf};

use egolint::{
    Config, ConfigOverrides, ExecutionPlan, Finding, Operation, PlanOptions, Profile,
    ProfileDefinition, ReportCompleteness, RunReport, Suppression,
};
use serde::Deserialize;

const EMPATHY_SOURCE_COMMIT: &str = "560aff8430c2f170dadae9161a4603a71c41acbf";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityManifest {
    schema_version: u32,
    fixture_id: String,
    source: Source,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    repository: String,
    commit: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    profile: Profile,
    catalog_tools: usize,
    fast_tools: usize,
    holistic_tools: usize,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/compatibility/empathy-v1")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&std::fs::read(path).expect("read compatibility fixture"))
        .expect("decode compatibility fixture")
}

#[test]
fn empathy_v1_provenance_and_profile_resolution_remain_compatible() {
    let fixture = fixture_root();
    let manifest: CompatibilityManifest = read_json(&fixture.join("manifest.json"));

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.fixture_id, "empathy-v1");
    assert_eq!(manifest.source.repository, "egohygiene/empathy");
    assert_eq!(manifest.source.commit, EMPATHY_SOURCE_COMMIT);
    assert_eq!(manifest.source.path, "egolint");

    let resolved = Config::resolve(&fixture, None, &ConfigOverrides::default())
        .expect("resolve Empathy compatibility configuration");
    assert_eq!(resolved.config.profile, manifest.expected.profile);
    assert_eq!(resolved.config.profile, Profile::Holistic);
    assert!(
        resolved
            .sources
            .iter()
            .any(|source| source.ends_with("egolint.toml"))
    );

    let plan = ExecutionPlan::build(
        &fixture,
        &resolved,
        Operation::Check,
        &PlanOptions::default(),
        false,
    )
    .expect("build compatibility plan without a container runtime");
    assert_eq!(plan.view.profile, Profile::Holistic);
    assert_eq!(plan.view.schema_version, 1);

    let report = RunReport::from_plan(&plan.view, Some(0));
    report.validate().expect("valid adapter-exit-only report");
    assert_eq!(report.completeness, ReportCompleteness::AdapterExitOnly);
    assert!(report.findings.is_empty());
    assert!(
        report
            .config_sources
            .iter()
            .all(|source| !Path::new(source).is_absolute())
    );
}

#[test]
fn empathy_v1_policy_inventory_and_normalized_examples_are_stable() {
    let fixture = fixture_root();
    let manifest: CompatibilityManifest = read_json(&fixture.join("manifest.json"));
    let matrix: serde_json::Value = read_json(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(".config/megalinter/tool-matrix.json"),
    );
    let tools = matrix["tools"].as_array().expect("tool matrix array");
    let fast = tools
        .iter()
        .filter(|tool| tool["profiles"]["fast"] == "selected")
        .count();
    let holistic = tools
        .iter()
        .filter(|tool| tool["profiles"]["holistic"] == "selected")
        .count();

    assert_eq!(tools.len(), manifest.expected.catalog_tools);
    assert_eq!(fast, manifest.expected.fast_tools);
    assert_eq!(holistic, manifest.expected.holistic_tools);

    let finding: Finding = read_json(&fixture.join("finding.json"));
    let suppression: Suppression = read_json(&fixture.join("suppression.json"));
    let report: RunReport = read_json(&fixture.join("report.json"));
    finding.validate().expect("valid compatibility finding");
    suppression
        .validate()
        .expect("valid compatibility suppression");
    report.validate().expect("valid compatibility report");
    assert_eq!(
        report.profile,
        ProfileDefinition::built_in(Profile::Holistic)
    );
    assert_eq!(report.findings, vec![finding]);
    assert_eq!(report.suppressions, vec![suppression]);

    let encoded = serde_json::to_vec(&report).expect("encode compatibility report");
    let round_trip: RunReport = serde_json::from_slice(&encoded).expect("round-trip report");
    assert_eq!(round_trip, report);

    let fixture_text = String::from_utf8(encoded).expect("report JSON is UTF-8");
    assert!(!fixture_text.contains("_cached_subprocess_env"));
    assert!(!fixture_text.contains("GITHUB_TOKEN"));
}
