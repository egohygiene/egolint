// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use egolint::debt::{DebtReport, write_debt_reports};
use egolint::plan::PlanView;
use egolint::sarif::{EGOLINT_SARIF_REPORT, write_sarif_atomic};
use egolint::{Operation, Profile, ReportCompleteness, RunReport, ToolStatus, normalize_workspace};

const DEBT_TOOLS: &[&str] = &[
    "REPOSITORY_DUSTILOCK",
    "REPOSITORY_GRYPE",
    "REPOSITORY_OSV_SCANNER",
    "REPOSITORY_SYFT",
    "REPOSITORY_TRIVY",
    "REPOSITORY_TRIVY_SBOM",
];

#[test]
fn all_six_debt_tools_flow_from_private_adapter_data_to_public_summaries() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_directory = workspace.path().join(".reports/egolint");
    std::fs::create_dir_all(&report_directory).expect("report directory");
    let linters = DEBT_TOOLS
        .iter()
        .map(|tool| {
            serde_json::json!({
                "descriptor_id": "REPOSITORY",
                "name": tool,
                "linter_name": tool,
                "is_active": true,
                "status": "success",
                "return_code": 0,
                "total_number_errors": 0,
                "total_number_warnings": 0
            })
        })
        .collect::<Vec<_>>();
    let runs = DEBT_TOOLS
        .iter()
        .map(|tool| {
            serde_json::json!({
                "tool": {"driver": {"name": tool}},
                "results": []
            })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        report_directory.join("mega-linter-report.json"),
        serde_json::to_vec(&serde_json::json!({
            "return_code": 0,
            "status": "success",
            "linters": linters
        }))
        .expect("adapter JSON"),
    )
    .expect("raw JSON report");
    std::fs::write(
        report_directory.join("mega-linter-report.sarif"),
        serde_json::to_vec(&serde_json::json!({
            "version": "2.1.0",
            "runs": runs
        }))
        .expect("adapter SARIF"),
    )
    .expect("raw SARIF report");

    let normalized = normalize_workspace(workspace.path(), Profile::DependencyDebt)
        .expect("adapter normalization")
        .expect("adapter evidence");
    assert_eq!(normalized.completeness, ReportCompleteness::Normalized);
    assert_eq!(normalized.tool_results.len(), DEBT_TOOLS.len());
    assert!(
        normalized
            .tool_results
            .iter()
            .all(|tool| tool.status == ToolStatus::Passed)
    );

    let plan = PlanView {
        schema_version: egolint::CONTRACT_VERSION,
        operation: Operation::Check,
        profile: Profile::DependencyDebt,
        runtime: "docker".to_owned(),
        image: "example.invalid/egolint-full@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
        workspace: workspace.path().canonicalize().expect("canonical workspace"),
        report_directory: PathBuf::from(".reports/egolint"),
        config_sources: vec!["compiled defaults".to_owned()],
        argv: Vec::new(),
    };
    let mut run = RunReport::from_plan(&plan, Some(0));
    run.set_normalized(
        normalized.tool_results,
        normalized.findings,
        Vec::new(),
        normalized.evidence,
        normalized.completeness,
    )
    .expect("normalized run contract");
    run.write_atomic(&report_directory.join("run.json"))
        .expect("run report");
    write_sarif_atomic(&run, &workspace.path().join(EGOLINT_SARIF_REPORT))
        .expect("canonical SARIF");
    let debt = DebtReport::from_run(&run).expect("debt projection");
    write_debt_reports(&debt, &report_directory).expect("debt reports");

    for name in ["run.json", "egolint.sarif", "debt.json", "debt.md"] {
        assert!(report_directory.join(name).is_file(), "missing {name}");
    }
    assert_eq!(debt.completeness, ReportCompleteness::Normalized);
    assert_eq!(debt.freshness.len(), 3);
}
