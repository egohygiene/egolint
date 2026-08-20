//! Publication-safe dependency-debt summary derived from normalized evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::Profile;
use crate::contracts::{CONTRACT_VERSION, Severity, SuppressionState, contract_version_schema};
use crate::error::{EgolintError, Result};
use crate::report::{ReportCompleteness, RunReport};
use crate::sarif::{validated_report_target, write_json_atomic};

/// Canonical compact JSON debt report path.
pub const DEBT_JSON_REPORT: &str = ".reports/egolint/debt.json";
/// Canonical human-readable debt report path.
pub const DEBT_MARKDOWN_REPORT: &str = ".reports/egolint/debt.md";

const OWNERSHIP_MATRIX: &str = include_str!("../.config/security/scanner-ownership.json");

/// Compact, sanitized dependency-debt evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DebtReport {
    /// Debt-contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Scanner ownership-policy version.
    pub policy_version: String,
    /// Timestamp inherited from the normalized run.
    pub generated_at_unix: u64,
    /// Selected Egolint profile.
    pub profile: String,
    /// Canonical workspace-relative source report.
    pub source_report: PathBuf,
    /// Source normalization coverage. Partial summaries contain observed
    /// counts only and are explicitly not publishable as complete debt.
    pub completeness: ReportCompleteness,
    /// Aggregate counts with no source paths, messages, or package names.
    pub summary: DebtSummary,
    /// Counts grouped only by canonical tool and severity.
    pub groups: Vec<DebtGroup>,
    /// Explicit database freshness state for network-backed authorities.
    pub freshness: Vec<DebtFreshness>,
}

/// Aggregate dependency-debt counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DebtSummary {
    /// Findings without an applied suppression.
    pub unsuppressed: u64,
    /// Findings with an applied suppression.
    pub suppressed: u64,
    /// Expired relevant suppressions, which remain blocking policy debt.
    pub expired_suppressions: u64,
    /// Unmatched relevant suppressions, which remain visible review debt.
    pub unmatched_suppressions: u64,
}

/// Sanitized count group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DebtGroup {
    /// Canonical scanner identifier.
    pub tool_id: String,
    /// Normalized severity.
    pub severity: Severity,
    /// Unsuppressed finding count.
    pub unsuppressed: u64,
    /// Suppressed finding count.
    pub suppressed: u64,
}

/// Database/source freshness disclosure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DebtFreshness {
    /// Canonical scanner identifier.
    pub tool_id: String,
    /// Honest source state. Unknown is the default until timestamp evidence is
    /// normalized by a scanner-specific adapter.
    pub state: FreshnessState,
    /// Source/database timestamp when normalized evidence provides one.
    pub observed_at_unix: Option<u64>,
}

/// Freshness state for an external vulnerability database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    /// Source evidence satisfies the configured age budget.
    Fresh,
    /// Source evidence is older than the configured age budget.
    Stale,
    /// No reviewed source timestamp was available.
    Unknown,
}

impl DebtReport {
    /// Derive a compact report from an already validated run report.
    ///
    /// Messages, source locations, package coordinates, raw adapter payloads,
    /// and environment values are deliberately excluded.
    ///
    /// # Errors
    ///
    /// Returns an error when the run report or embedded ownership policy is
    /// invalid.
    pub fn from_run(report: &RunReport) -> Result<Self> {
        report.validate()?;
        if report.completeness == ReportCompleteness::AdapterExitOnly {
            return Err(EgolintError::Configuration(
                "dependency debt requires at least partially normalized evidence".to_owned(),
            ));
        }
        if report.profile.name != Profile::DependencyDebt {
            return Err(EgolintError::Configuration(
                "dependency debt requires the dependency-debt profile".to_owned(),
            ));
        }
        let ownership: OwnershipDocument = serde_json::from_str(OWNERSHIP_MATRIX)?;
        let relevant_tools = ownership.relevant_tools();
        let suppression_states = report
            .suppressions
            .iter()
            .map(|suppression| (suppression.id.as_str(), suppression.state))
            .collect::<BTreeMap<_, _>>();
        let mut grouped = BTreeMap::<(String, SeverityKey), (u64, u64)>::new();
        for finding in &report.findings {
            if !relevant_tools.contains(&finding.rule.tool_id) {
                continue;
            }
            let entry = grouped
                .entry((
                    finding.rule.tool_id.clone(),
                    SeverityKey::from(finding.severity),
                ))
                .or_default();
            let has_applied_suppression = finding
                .suppressed_by
                .as_deref()
                .and_then(|id| suppression_states.get(id))
                .is_some_and(|state| *state == SuppressionState::Applied);
            if has_applied_suppression {
                entry.1 += 1;
            } else {
                entry.0 += 1;
            }
        }
        let groups = grouped
            .into_iter()
            .map(
                |((tool_id, severity), (unsuppressed, suppressed))| DebtGroup {
                    tool_id,
                    severity: severity.into(),
                    unsuppressed,
                    suppressed,
                },
            )
            .collect::<Vec<_>>();
        let summary = DebtSummary {
            unsuppressed: groups.iter().map(|group| group.unsuppressed).sum(),
            suppressed: groups.iter().map(|group| group.suppressed).sum(),
            expired_suppressions: report
                .suppressions
                .iter()
                .filter(|suppression| relevant_tools.contains(&suppression.rule.tool_id))
                .filter(|suppression| suppression.state == SuppressionState::Expired)
                .count() as u64,
            unmatched_suppressions: report
                .suppressions
                .iter()
                .filter(|suppression| relevant_tools.contains(&suppression.rule.tool_id))
                .filter(|suppression| suppression.state == SuppressionState::Unmatched)
                .count() as u64,
        };
        let freshness = ownership
            .network_backed_tools()
            .into_iter()
            .map(|tool_id| DebtFreshness {
                tool_id,
                state: FreshnessState::Unknown,
                observed_at_unix: None,
            })
            .collect();
        let debt = Self {
            schema_version: CONTRACT_VERSION,
            policy_version: ownership.policy_version,
            generated_at_unix: report.generated_at_unix,
            profile: report.profile.name.as_str().to_owned(),
            source_report: PathBuf::from(".reports/egolint/run.json"),
            completeness: report.completeness,
            summary,
            groups,
            freshness,
        };
        debt.validate()?;
        Ok(debt)
    }

    /// Validate counts, stable ordering, and freshness disclosure.
    ///
    /// # Errors
    ///
    /// Returns an error when a caller constructed inconsistent or unsafe debt
    /// evidence.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CONTRACT_VERSION {
            return Err(EgolintError::Configuration(format!(
                "debt schema-version must equal {CONTRACT_VERSION}"
            )));
        }
        validate_debt_token("debt policy-version", &self.policy_version)?;
        validate_debt_token("debt profile", &self.profile)?;
        if self.profile != Profile::DependencyDebt.as_str() {
            return Err(EgolintError::Configuration(
                "debt profile must be dependency-debt".to_owned(),
            ));
        }
        if self.source_report != Path::new(".reports/egolint/run.json") {
            return Err(EgolintError::Configuration(
                "debt source-report must be .reports/egolint/run.json".to_owned(),
            ));
        }
        if self.completeness == ReportCompleteness::AdapterExitOnly {
            return Err(EgolintError::Configuration(
                "dependency debt may not be derived from adapter-exit-only evidence".to_owned(),
            ));
        }
        let expected_unsuppressed: u64 = self.groups.iter().map(|group| group.unsuppressed).sum();
        let expected_suppressed: u64 = self.groups.iter().map(|group| group.suppressed).sum();
        if self.summary.unsuppressed != expected_unsuppressed
            || self.summary.suppressed != expected_suppressed
        {
            return Err(EgolintError::Configuration(
                "debt summary counts do not match grouped counts".to_owned(),
            ));
        }
        let mut prior = None;
        for group in &self.groups {
            validate_debt_token("debt tool identifier", &group.tool_id)?;
            let key = (group.tool_id.as_str(), SeverityKey::from(group.severity));
            if prior.is_some_and(|previous| previous >= key) {
                return Err(EgolintError::Configuration(
                    "debt groups must be unique and sorted".to_owned(),
                ));
            }
            prior = Some(key);
        }
        let mut freshness_tools = BTreeSet::new();
        let mut prior_freshness = None;
        for source in &self.freshness {
            validate_debt_token("debt freshness tool-id", &source.tool_id)?;
            if !freshness_tools.insert(source.tool_id.as_str()) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate debt freshness source {}",
                    source.tool_id
                )));
            }
            if prior_freshness.is_some_and(|previous| previous >= source.tool_id.as_str()) {
                return Err(EgolintError::Configuration(
                    "debt freshness sources must be sorted".to_owned(),
                ));
            }
            prior_freshness = Some(source.tool_id.as_str());
            match (source.state, source.observed_at_unix) {
                (FreshnessState::Unknown, None)
                | (FreshnessState::Fresh | FreshnessState::Stale, Some(_)) => {}
                (FreshnessState::Unknown, Some(_)) => {
                    return Err(EgolintError::Configuration(
                        "unknown debt freshness may not claim an observation timestamp".to_owned(),
                    ));
                }
                (FreshnessState::Fresh | FreshnessState::Stale, None) => {
                    return Err(EgolintError::Configuration(
                        "known debt freshness requires an observation timestamp".to_owned(),
                    ));
                }
            }
        }
        let ownership: OwnershipDocument = serde_json::from_str(OWNERSHIP_MATRIX)?;
        let relevant_tools = ownership.relevant_tools();
        if self
            .groups
            .iter()
            .any(|group| !relevant_tools.contains(&group.tool_id))
        {
            return Err(EgolintError::Configuration(
                "debt groups contain a tool outside the ownership policy".to_owned(),
            ));
        }
        let expected_freshness = ownership.network_backed_tools();
        let observed_freshness = self
            .freshness
            .iter()
            .map(|source| source.tool_id.clone())
            .collect::<Vec<_>>();
        if observed_freshness != expected_freshness {
            return Err(EgolintError::Configuration(
                "debt freshness sources do not match the ownership policy".to_owned(),
            ));
        }
        Ok(())
    }

    /// Render a compact Markdown summary with counts only.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "# Egolint dependency debt\n\n- Completeness: {}\n- Unsuppressed observed: {}\n- Suppressed observed: {}\n- Expired suppressions: {}\n- Unmatched suppressions: {}\n\n",
            completeness_name(self.completeness),
            self.summary.unsuppressed,
            self.summary.suppressed,
            self.summary.expired_suppressions,
            self.summary.unmatched_suppressions,
        );
        if self.completeness == ReportCompleteness::Partial {
            output.push_str(
                "> This is a partial observation. Missing scanner coverage may hide additional debt; do not interpret these counts as a complete inventory.\n\n",
            );
        }
        output.push_str("| Tool | Severity | Unsuppressed | Suppressed |\n");
        output.push_str("| --- | --- | ---: | ---: |\n");
        for group in &self.groups {
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                group.tool_id,
                severity_name(group.severity),
                group.unsuppressed,
                group.suppressed,
            ));
        }
        output.push_str("\n## Vulnerability database freshness\n\n");
        for source in &self.freshness {
            output.push_str(&format!(
                "- {}: {}\n",
                source.tool_id,
                freshness_name(source.state),
            ));
        }
        output
    }
}

fn validate_debt_token(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
    {
        Err(EgolintError::Configuration(format!(
            "{name} must be a bounded portable identifier"
        )))
    } else {
        Ok(())
    }
}

/// Write compact JSON and Markdown debt summaries atomically.
///
/// # Errors
///
/// Returns an error for invalid evidence or a failed durable write.
pub fn write_debt_reports(debt: &DebtReport, directory: &Path) -> Result<()> {
    debt.validate()?;
    let json_path = directory.join("debt.json");
    let markdown_path = directory.join("debt.md");
    validated_report_target(&json_path)?;
    validated_report_target(&markdown_path)?;
    let value = serde_json::to_value(debt)?;
    write_json_atomic(&value, &json_path)?;
    write_text_atomic(&debt.to_markdown(), &markdown_path)
}

#[derive(Debug, Deserialize)]
struct OwnershipDocument {
    policy_version: String,
    capabilities: BTreeMap<String, OwnershipCapability>,
}

impl OwnershipDocument {
    fn relevant_tools(&self) -> BTreeSet<String> {
        self.capabilities
            .iter()
            .filter(|(name, _)| name.starts_with("dependency_"))
            .flat_map(|(_, capability)| {
                capability
                    .primary
                    .iter()
                    .chain(&capability.advisory)
                    .cloned()
            })
            .collect()
    }

    fn network_backed_tools(&self) -> Vec<String> {
        let relevant = self.relevant_tools();
        [
            "REPOSITORY_GRYPE",
            "REPOSITORY_OSV_SCANNER",
            "REPOSITORY_TRIVY",
        ]
        .into_iter()
        .filter(|tool| relevant.contains(*tool))
        .map(str::to_owned)
        .collect()
    }
}

#[derive(Debug, Deserialize)]
struct OwnershipCapability {
    #[serde(default)]
    primary: Vec<String>,
    #[serde(default)]
    advisory: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SeverityKey {
    Info,
    Warning,
    Error,
    Critical,
}

impl From<Severity> for SeverityKey {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Info => Self::Info,
            Severity::Warning => Self::Warning,
            Severity::Error => Self::Error,
            Severity::Critical => Self::Critical,
        }
    }
}

impl From<SeverityKey> for Severity {
    fn from(value: SeverityKey) -> Self {
        match value {
            SeverityKey::Info => Self::Info,
            SeverityKey::Warning => Self::Warning,
            SeverityKey::Error => Self::Error,
            SeverityKey::Critical => Self::Critical,
        }
    }
}

const fn severity_name(value: Severity) -> &'static str {
    match value {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Critical => "critical",
    }
}

const fn freshness_name(value: FreshnessState) -> &'static str {
    match value {
        FreshnessState::Fresh => "fresh",
        FreshnessState::Stale => "stale",
        FreshnessState::Unknown => "unknown",
    }
}

const fn completeness_name(value: ReportCompleteness) -> &'static str {
    match value {
        ReportCompleteness::AdapterExitOnly => "adapter-exit-only",
        ReportCompleteness::Partial => "partial",
        ReportCompleteness::Normalized => "normalized",
    }
}

fn write_text_atomic(contents: &str, path: &Path) -> Result<()> {
    let (validated_path, parent) = validated_report_target(path)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(&parent).map_err(|source| EgolintError::Filesystem {
            path: parent.clone(),
            source,
        })?;
    temporary
        .as_file_mut()
        .write_all(contents.as_bytes())
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|source| EgolintError::Filesystem {
            path: validated_path.clone(),
            source,
        })?;
    let (revalidated_path, revalidated_parent) = validated_report_target(&validated_path)?;
    if revalidated_path != validated_path || revalidated_parent != parent {
        return Err(EgolintError::RuntimeExecution(
            "text report destination changed before persistence".to_owned(),
        ));
    }
    temporary
        .persist(&validated_path)
        .map_err(|error| EgolintError::Filesystem {
            path: validated_path,
            source: error.error,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        Finding, ProfileDefinition, RuleIdentity, RuleOwnership, SourceLocation, Suppression,
    };
    use crate::plan::Operation;
    use crate::report::{ReportCompleteness, ReportSummary, RunStatus};

    #[test]
    fn compact_debt_never_copies_sensitive_messages_or_paths() {
        let report = RunReport {
            schema_version: CONTRACT_VERSION,
            generated_at_unix: 42,
            operation: Operation::Check,
            status: RunStatus::Findings,
            egolint_exit_code: 1,
            adapter_exit_code: Some(1),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            profile: ProfileDefinition::built_in(Profile::DependencyDebt),
            config_sources: vec!["compiled defaults".to_owned()],
            report_path: PathBuf::from(".reports/egolint/run.json"),
            completeness: ReportCompleteness::Normalized,
            summary: ReportSummary {
                normalized_tool_results: 0,
                normalized_findings: 1,
                normalized_suppressions: 0,
            },
            tool_results: Vec::new(),
            findings: vec![Finding {
                schema_version: CONTRACT_VERSION,
                id: "DEBT-1".to_owned(),
                rule: RuleIdentity {
                    tool_id: "REPOSITORY_OSV_SCANNER".to_owned(),
                    rule_id: "OSV-EXAMPLE".to_owned(),
                },
                severity: Severity::Critical,
                message: "SECRET-VALUE private.registry.example/internal-package".to_owned(),
                location: Some(SourceLocation {
                    path: PathBuf::from("private/path/package-lock.json"),
                    start_line: Some(1),
                    start_column: Some(1),
                    end_line: None,
                    end_column: None,
                }),
                ownership: RuleOwnership {
                    owner: "egolint".to_owned(),
                    policy_source: ".config/security/scanner-ownership.json".to_owned(),
                    configuration_path: None,
                },
                fingerprint: Some("debt-v1-example".to_owned()),
                evidence: Vec::new(),
                suppressed_by: None,
            }],
            suppressions: Vec::new(),
            evidence: Vec::new(),
        };

        let debt = DebtReport::from_run(&report).expect("compact debt report");
        let serialized = serde_json::to_string(&debt).expect("debt JSON");
        let markdown = debt.to_markdown();
        for sensitive in ["SECRET-VALUE", "private.registry", "private/path"] {
            assert!(!serialized.contains(sensitive));
            assert!(!markdown.contains(sensitive));
        }
        assert_eq!(debt.summary.unsuppressed, 1);
        assert!(
            debt.freshness
                .iter()
                .all(|source| source.state == FreshnessState::Unknown)
        );

        let mut partial = report.clone();
        partial.completeness = ReportCompleteness::Partial;
        let partial_debt = DebtReport::from_run(&partial).expect("partial observed debt");
        assert_eq!(partial_debt.completeness, ReportCompleteness::Partial);
        assert!(partial_debt.to_markdown().contains("partial observation"));

        let mut exit_only = partial;
        exit_only.completeness = ReportCompleteness::AdapterExitOnly;
        assert!(DebtReport::from_run(&exit_only).is_err());

        let mut suppressed = report;
        suppressed.findings[0].suppressed_by = Some("SUP-DEBT-1".to_owned());
        suppressed.suppressions.push(Suppression {
            schema_version: CONTRACT_VERSION,
            id: "SUP-DEBT-1".to_owned(),
            rule: suppressed.findings[0].rule.clone(),
            path: None,
            fingerprint: None,
            owner: "egohygiene/egolint".to_owned(),
            justification: "Reviewed dependency exception".to_owned(),
            expires_on: "2099-12-31".to_owned(),
            state: SuppressionState::Applied,
            evidence: Vec::new(),
        });
        suppressed.summary.normalized_suppressions = 1;
        suppressed.status = RunStatus::Clean;
        suppressed.egolint_exit_code = 0;
        suppressed.adapter_exit_code = Some(0);

        let applied = DebtReport::from_run(&suppressed).expect("applied debt suppression");
        assert_eq!(applied.summary.suppressed, 1);
        assert_eq!(applied.summary.unsuppressed, 0);

        suppressed.suppressions[0].state = SuppressionState::Expired;
        let expired = DebtReport::from_run(&suppressed).expect("expired debt suppression");
        assert_eq!(expired.summary.suppressed, 0);
        assert_eq!(expired.summary.unsuppressed, 1);
        assert_eq!(expired.summary.expired_suppressions, 1);
    }
}
