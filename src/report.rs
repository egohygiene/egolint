//! Stable run-report contract for Relay and Observatory consumers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceReference, Finding, ProfileDefinition, Suppression, ToolResult,
};
use crate::error::{EgolintError, Result, exit_code};
use crate::plan::{Operation, PlanView};

/// Normalized outcome independent of any individual linter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// All blocking checks passed.
    Clean,
    /// The wrapped lint engine reported findings.
    Findings,
    /// The engine could not complete normally.
    ExecutionError,
}

impl RunStatus {
    /// Normalize a wrapped adapter status into the stable Egolint contract.
    #[must_use]
    pub const fn from_adapter_exit_code(exit_code: Option<i32>) -> Self {
        match exit_code {
            Some(exit_code::CLEAN) => Self::Clean,
            Some(exit_code::FINDINGS) => Self::Findings,
            _ => Self::ExecutionError,
        }
    }

    /// Return the stable Egolint process code for this outcome.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Clean => exit_code::CLEAN,
            Self::Findings => exit_code::FINDINGS,
            Self::ExecutionError => exit_code::RUNTIME,
        }
    }
}

/// How completely adapter evidence has been normalized into this report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportCompleteness {
    /// Only the adapter process outcome is known; raw details remain external.
    AdapterExitOnly,
    /// Available adapter summaries were normalized, but coverage is incomplete.
    Partial,
    /// Every available adapter result was normalized into typed contracts.
    Normalized,
}

/// Counts for data actually normalized into the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReportSummary {
    /// Number of normalized per-tool results.
    pub normalized_tool_results: u64,
    /// Number of normalized findings.
    pub normalized_findings: u64,
    /// Number of normalized suppression evaluations.
    pub normalized_suppressions: u64,
}

impl ReportSummary {
    fn from_contracts(
        tool_results: &[ToolResult],
        findings: &[Finding],
        suppressions: &[Suppression],
    ) -> Self {
        Self {
            normalized_tool_results: tool_results.len() as u64,
            normalized_findings: findings.len() as u64,
            normalized_suppressions: suppressions.len() as u64,
        }
    }
}

/// Versioned report written beside raw adapter output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RunReport {
    /// Output contract version.
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    /// Unix timestamp in seconds.
    pub generated_at_unix: u64,
    /// Operation executed.
    pub operation: Operation,
    /// Normalized outcome.
    pub status: RunStatus,
    /// Stable Egolint process exit code.
    pub egolint_exit_code: i32,
    /// Raw wrapped-adapter exit code when available.
    pub adapter_exit_code: Option<i32>,
    /// Requested container image.
    pub image: String,
    /// Versioned selected profile contract.
    pub profile: ProfileDefinition,
    /// Configuration provenance.
    pub config_sources: Vec<String>,
    /// Workspace-relative location of the report itself.
    pub report_path: PathBuf,
    /// Whether adapter details were fully normalized.
    pub completeness: ReportCompleteness,
    /// Exact counts for normalized contract arrays.
    pub summary: ReportSummary,
    /// Normalized per-tool results, when an adapter normalizer is available.
    pub tool_results: Vec<ToolResult>,
    /// Normalized findings, when an adapter normalizer is available.
    pub findings: Vec<Finding>,
    /// Evaluated suppressions, when a suppression engine is available.
    pub suppressions: Vec<Suppression>,
    /// Sanitized evidence supporting the run-level outcome.
    pub evidence: Vec<EvidenceReference>,
}

impl RunReport {
    /// Construct a report from a public plan and process status.
    #[must_use]
    pub fn from_plan(plan: &PlanView, adapter_exit_code: Option<i32>) -> Self {
        let status = RunStatus::from_adapter_exit_code(adapter_exit_code);
        let tool_results: Vec<ToolResult> = Vec::new();
        let findings: Vec<Finding> = Vec::new();
        let suppressions: Vec<Suppression> = Vec::new();
        Self {
            schema_version: CONTRACT_VERSION,
            generated_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
            operation: plan.operation,
            status,
            egolint_exit_code: status.exit_code(),
            adapter_exit_code,
            image: plan.image.clone(),
            profile: ProfileDefinition::built_in(plan.profile),
            config_sources: sanitized_config_sources(plan),
            report_path: plan.report_directory.join("run.json"),
            completeness: ReportCompleteness::AdapterExitOnly,
            summary: ReportSummary::from_contracts(&tool_results, &findings, &suppressions),
            tool_results,
            findings,
            suppressions,
            evidence: Vec::new(),
        }
    }

    /// Replace adapter-exit-only detail with normalized contracts and
    /// recompute exact counts and the stable run status.
    ///
    /// Partial normalization preserves an adapter-level failure when the
    /// available normalized records appear clean, because absence of evidence
    /// is not evidence of a clean run.
    ///
    /// # Errors
    ///
    /// Returns an error when `AdapterExitOnly` is requested or any nested
    /// contract is inconsistent or unsafe.
    pub fn set_normalized(
        &mut self,
        tool_results: Vec<ToolResult>,
        findings: Vec<Finding>,
        suppressions: Vec<Suppression>,
        evidence: Vec<EvidenceReference>,
        completeness: ReportCompleteness,
    ) -> Result<()> {
        if completeness == ReportCompleteness::AdapterExitOnly {
            return Err(EgolintError::Configuration(
                "normalized report detail must be partial or normalized".to_owned(),
            ));
        }
        let normalized_status = normalized_status(&tool_results, &findings);
        let mut candidate = self.clone();
        candidate.status = if completeness == ReportCompleteness::Partial
            && normalized_status == RunStatus::Clean
        {
            self.status
        } else {
            normalized_status
        };
        candidate.egolint_exit_code = candidate.status.exit_code();
        candidate.summary = ReportSummary::from_contracts(&tool_results, &findings, &suppressions);
        candidate.tool_results = tool_results;
        candidate.findings = findings;
        candidate.suppressions = suppressions;
        candidate.evidence = evidence;
        candidate.completeness = completeness;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Validate report version, summary consistency, nested contracts, and
    /// workspace-relative evidence paths.
    ///
    /// # Errors
    ///
    /// Returns an error when the report is internally inconsistent or contains
    /// an unsupported or unsafe nested contract.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CONTRACT_VERSION {
            return Err(EgolintError::Configuration(format!(
                "report schema-version must equal {CONTRACT_VERSION}"
            )));
        }
        if self.egolint_exit_code != self.status.exit_code() {
            return Err(EgolintError::Configuration(
                "report Egolint exit code does not match its status".to_owned(),
            ));
        }
        self.profile.validate()?;
        let expected =
            ReportSummary::from_contracts(&self.tool_results, &self.findings, &self.suppressions);
        if self.summary != expected {
            return Err(EgolintError::Configuration(
                "report summary does not match normalized contract arrays".to_owned(),
            ));
        }
        let normalized_status = normalized_status(&self.tool_results, &self.findings);
        match self.completeness {
            ReportCompleteness::AdapterExitOnly
                if !self.tool_results.is_empty()
                    || !self.findings.is_empty()
                    || !self.suppressions.is_empty() =>
            {
                return Err(EgolintError::Configuration(
                    "adapter-exit-only reports may not claim normalized detail".to_owned(),
                ));
            }
            ReportCompleteness::Normalized if self.status != normalized_status => {
                return Err(EgolintError::Configuration(
                    "normalized report status does not match normalized detail".to_owned(),
                ));
            }
            ReportCompleteness::Partial
                if normalized_status != RunStatus::Clean && self.status != normalized_status =>
            {
                return Err(EgolintError::Configuration(
                    "partial report may not contradict a normalized failure".to_owned(),
                ));
            }
            _ => {}
        }
        if self.report_path != Path::new(".reports/egolint/run.json") {
            return Err(EgolintError::Configuration(
                "report-path must equal .reports/egolint/run.json".to_owned(),
            ));
        }
        for tool_result in &self.tool_results {
            tool_result.validate()?;
        }
        for finding in &self.findings {
            finding.validate()?;
        }
        for suppression in &self.suppressions {
            suppression.validate()?;
        }
        for evidence in &self.evidence {
            evidence.validate()?;
        }

        let suppression_ids = self
            .suppressions
            .iter()
            .map(|suppression| suppression.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for finding in &self.findings {
            if let Some(suppression_id) = finding.suppressed_by.as_deref() {
                if !suppression_ids.contains(suppression_id) {
                    return Err(EgolintError::Configuration(format!(
                        "finding {} references missing suppression {suppression_id}",
                        finding.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Atomically persist this report without following an existing target
    /// symlink. The temporary file is created beside the destination so the
    /// final replacement stays on one filesystem.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization, durable temporary-file writing, or
    /// the final atomic replacement fails.
    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let (path, parent) = crate::sarif::validated_report_target(path)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&parent).map_err(|source| {
            EgolintError::Filesystem {
                path: parent.clone(),
                source,
            }
        })?;
        serde_json::to_writer_pretty(temporary.as_file_mut(), self)?;
        temporary
            .as_file_mut()
            .write_all(b"\n")
            .map_err(|source| EgolintError::Filesystem {
                path: path.clone(),
                source,
            })?;
        temporary
            .as_file_mut()
            .sync_all()
            .map_err(|source| EgolintError::Filesystem {
                path: path.clone(),
                source,
            })?;
        let (revalidated_path, revalidated_parent) = crate::sarif::validated_report_target(&path)?;
        if revalidated_path != path || revalidated_parent != parent {
            return Err(EgolintError::RuntimeExecution(
                "run report destination changed before persistence".to_owned(),
            ));
        }
        temporary
            .persist(&path)
            .map_err(|error| EgolintError::Filesystem {
                path,
                source: error.error,
            })?;
        Ok(())
    }
}

fn sanitized_config_sources(plan: &PlanView) -> Vec<String> {
    plan.config_sources
        .iter()
        .map(|source| {
            let path = Path::new(source);
            if let Ok(relative) = path.strip_prefix(&plan.workspace) {
                relative.display().to_string()
            } else if path.is_absolute() {
                "<external configuration>".to_owned()
            } else {
                source.clone()
            }
        })
        .collect()
}

fn normalized_status(tool_results: &[ToolResult], findings: &[Finding]) -> RunStatus {
    use crate::contracts::{Enforcement, Severity, ToolStatus};

    let execution_failed = tool_results.iter().any(|result| {
        matches!(
            result.status,
            ToolStatus::MissingFromImage
                | ToolStatus::ConfigurationError
                | ToolStatus::ExecutionError
                | ToolStatus::TimedOut
        )
    });
    if execution_failed {
        return RunStatus::ExecutionError;
    }
    let blocking_tool_findings = tool_results.iter().any(|result| {
        result.enforcement == Enforcement::Blocking && result.status == ToolStatus::FailedFindings
    });
    let unsuppressed_blocking_findings = findings.iter().any(|finding| {
        finding.suppressed_by.is_none()
            && matches!(finding.severity, Severity::Error | Severity::Critical)
    });
    if blocking_tool_findings || unsuppressed_blocking_findings {
        RunStatus::Findings
    } else {
        RunStatus::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use crate::contracts::{EvidenceKind, RuleIdentity, RuleOwnership, Severity};

    #[test]
    fn adapter_exit_codes_are_normalized_without_losing_raw_status() {
        for (raw, expected_status, expected_egolint) in [
            (Some(0), RunStatus::Clean, exit_code::CLEAN),
            (Some(1), RunStatus::Findings, exit_code::FINDINGS),
            (Some(2), RunStatus::ExecutionError, exit_code::RUNTIME),
            (Some(127), RunStatus::ExecutionError, exit_code::RUNTIME),
            (None, RunStatus::ExecutionError, exit_code::RUNTIME),
        ] {
            let status = RunStatus::from_adapter_exit_code(raw);
            assert_eq!(status, expected_status);
            assert_eq!(status.exit_code(), expected_egolint);
        }
    }

    #[test]
    fn run_report_preserves_raw_adapter_status_and_stable_egolint_status() {
        let plan = PlanView {
            schema_version: 1,
            operation: Operation::Check,
            profile: Profile::Fast,
            runtime: "docker".to_owned(),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            workspace: PathBuf::from("/workspace"),
            report_directory: PathBuf::from(".reports/egolint"),
            config_sources: vec!["compiled defaults".to_owned()],
            argv: vec!["docker".to_owned(), "run".to_owned()],
        };

        let report = RunReport::from_plan(&plan, Some(42));

        assert_eq!(report.status, RunStatus::ExecutionError);
        assert_eq!(report.egolint_exit_code, exit_code::RUNTIME);
        assert_eq!(report.adapter_exit_code, Some(42));
    }

    #[test]
    fn normalized_advisory_findings_do_not_fail_the_run() {
        let mut report = RunReport::from_plan(&plan(), Some(1));
        report
            .set_normalized(
                Vec::new(),
                vec![finding(Severity::Warning)],
                Vec::new(),
                Vec::new(),
                ReportCompleteness::Normalized,
            )
            .expect("normalize advisory result");
        assert_eq!(report.status, RunStatus::Clean);
        assert_eq!(report.egolint_exit_code, exit_code::CLEAN);

        report
            .set_normalized(
                Vec::new(),
                vec![finding(Severity::Error)],
                Vec::new(),
                Vec::new(),
                ReportCompleteness::Normalized,
            )
            .expect("normalize blocking result");
        assert_eq!(report.status, RunStatus::Findings);
        assert_eq!(report.egolint_exit_code, exit_code::FINDINGS);
    }

    #[test]
    fn failed_normalization_does_not_mutate_the_existing_report() {
        let mut report = RunReport::from_plan(&plan(), Some(0));
        let original = report.clone();
        let unsafe_evidence = EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::AdapterLog,
            path: PathBuf::from("../outside.log"),
            sha256: None,
            description: None,
        };

        assert!(
            report
                .set_normalized(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    vec![unsafe_evidence],
                    ReportCompleteness::Normalized,
                )
                .is_err()
        );
        assert_eq!(report, original);
    }

    #[test]
    fn atomic_report_write_replaces_an_existing_regular_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("run.json");
        std::fs::write(&path, "stale").expect("stale report");

        let report = RunReport::from_plan(&plan(), Some(0));
        report.write_atomic(&path).expect("atomic report write");

        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("persisted report"))
                .expect("valid report JSON");
        assert_eq!(value["status"], "clean");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_report_write_rejects_symlink_without_overwriting_its_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        let outside = tempfile::NamedTempFile::new().expect("outside target");
        std::fs::write(outside.path(), "sentinel").expect("outside sentinel");
        let path = directory.path().join("run.json");
        symlink(outside.path(), &path).expect("malicious report link");

        assert!(
            RunReport::from_plan(&plan(), Some(0))
                .write_atomic(&path)
                .is_err()
        );

        assert_eq!(
            std::fs::read_to_string(outside.path()).expect("outside target"),
            "sentinel"
        );
        assert!(
            std::fs::symlink_metadata(&path)
                .expect("report metadata")
                .file_type()
                .is_symlink()
        );
    }

    fn plan() -> PlanView {
        PlanView {
            schema_version: 1,
            operation: Operation::Check,
            profile: Profile::Fast,
            runtime: "docker".to_owned(),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            workspace: PathBuf::from("/workspace"),
            report_directory: PathBuf::from(".reports/egolint"),
            config_sources: vec!["compiled defaults".to_owned()],
            argv: vec!["docker".to_owned(), "run".to_owned()],
        }
    }

    fn finding(severity: Severity) -> Finding {
        Finding {
            schema_version: CONTRACT_VERSION,
            id: "FINDING-1".to_owned(),
            rule: RuleIdentity {
                tool_id: "EGOLINT_TEST".to_owned(),
                rule_id: "test-rule".to_owned(),
            },
            severity,
            message: "Synthetic report status finding.".to_owned(),
            location: None,
            ownership: RuleOwnership {
                owner: "egolint".to_owned(),
                policy_source: "test-policy".to_owned(),
                configuration_path: None,
            },
            fingerprint: None,
            evidence: Vec::new(),
            suppressed_by: None,
        }
    }
}
