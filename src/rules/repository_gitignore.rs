//! Pinned, read-only validation of layered ignore policies using Git itself.

mod contract;
mod evaluation;
mod git;
mod inventory;

#[cfg(test)]
mod tests;

use std::io::Write;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation, validate_contract_date,
};
use crate::error::{EgolintError, Result};

/// Native tool identity shared by findings and run reports.
pub const TOOL_ID: &str = "EGOLINT_REPOSITORY_GITIGNORE";
/// Focused evidence report, containing policy metadata rather than file payloads.
pub const REPORT_PATH: &str = ".reports/egolint/repository-gitignore.json";
const CATALOG: &str = include_str!("../../.config/rules/repository-gitignore.v1.json");
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 50_000;
const MAX_PROBES: usize = 50_000;
const MAX_POLICIES: usize = 256;
const MAX_POLICY_BYTES: usize = 8 * 1024 * 1024;

/// Explicit scope selected by the consumer, using Empathy's ordered local text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GitignoreScope {
    pub root: String,
    pub overlays: Vec<String>,
    pub local_additions: String,
}

/// An exact consumer behavior assertion; paths are literal, never globs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreProbe {
    pub path: String,
    pub ignored: bool,
}

/// Reviewed inventory of an inherited policy; this alone exempts no behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GitignoreNestedPolicy {
    pub path: String,
    pub sha256: String,
    pub owner: String,
    pub reason: String,
    pub approval: String,
}

/// A time-bounded exception for one literal path, bound to its winning policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GitignoreException {
    pub path: String,
    pub policy_path: String,
    pub policy_sha256: String,
    pub owner: String,
    pub reason: String,
    pub approval: String,
    pub expires_on: String,
    pub ignored: Option<bool>,
    #[serde(default)]
    pub allow_tracked: bool,
}

/// Repository-owned selection of the accepted Empathy composition contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RepositoryGitignorePolicy {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub id: String,
    pub repository: String,
    pub source_revision: String,
    pub source_root: String,
    pub composition_path: String,
    pub composition_sha256: String,
    pub profiles: Vec<String>,
    pub scopes: Vec<GitignoreScope>,
    #[serde(default)]
    pub probes: Vec<GitignoreProbe>,
    #[serde(default)]
    pub nested_policies: Vec<GitignoreNestedPolicy>,
    #[serde(default)]
    pub exceptions: Vec<GitignoreException>,
}

impl RepositoryGitignorePolicy {
    /// Read a bounded, regular policy file beneath the consumer root.
    ///
    /// # Errors
    /// Returns an error for unsafe paths, unavailable files, or invalid policy.
    pub fn load(workspace: &Path, path: &Path) -> Result<Self> {
        let raw = inventory::read_file(workspace, &path.to_string_lossy())
            .map_err(EgolintError::Configuration)?;
        let text = std::str::from_utf8(&raw).map_err(|_| {
            EgolintError::Configuration("gitignore policy must be UTF-8".to_owned())
        })?;
        Self::from_toml(text, path)
    }

    /// Decode and validate the closed, repository-owned request.
    ///
    /// # Errors
    /// Returns a configuration error for unsupported or unsafe selections.
    pub fn from_toml(contents: &str, path: &Path) -> Result<Self> {
        let policy: Self = toml::from_str(contents).map_err(|source| EgolintError::Toml {
            path: path.to_path_buf(),
            source,
        })?;
        contract::validate_policy(&policy).map_err(EgolintError::Configuration)?;
        Ok(policy)
    }
}

/// Independent result for each bounded evidence layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitignoreCheckStatus {
    Passed,
    Failed,
    Incomplete,
    Unavailable,
    NotEvaluated,
}

/// Presence never substitutes for content, behavior, or evidence availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreChecks {
    pub source_integrity: GitignoreCheckStatus,
    pub composition: GitignoreCheckStatus,
    pub presence: GitignoreCheckStatus,
    pub content: GitignoreCheckStatus,
    pub effective_behavior: GitignoreCheckStatus,
    pub nested_policy: GitignoreCheckStatus,
    pub tracked_files: GitignoreCheckStatus,
    pub coverage: GitignoreCheckStatus,
}

/// Repository-wide status within the explicitly reported coverage boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitignoreValidationStatus {
    Valid,
    ValidWithExceptions,
    Invalid,
    Incomplete,
}

/// Git's winning rule, relative to the consumer policy snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreWinningRule {
    pub path: String,
    pub line: u32,
    pub pattern: String,
}

/// One evaluated path, retaining expected and actual effective behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreBehaviorEvidence {
    pub path: String,
    pub scope: String,
    pub expected_ignored: bool,
    pub actual_ignored: bool,
    pub winning_rule: Option<GitignoreWinningRule>,
    pub tracked: bool,
    pub exception: Option<String>,
}

/// Policy file identity; ignore text is not copied into the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreFileEvidence {
    pub path: String,
    pub scope: String,
    pub expected_sha256: Option<String>,
    pub actual_sha256: Option<String>,
    pub disposition: String,
}

/// Actionable diagnostic shared with normalized finding output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreDiagnostic {
    pub rule_id: String,
    pub status: GitignoreCheckStatus,
    pub path: Option<String>,
    pub scope: Option<String>,
    pub message: String,
    pub remediation: String,
}

/// Immutable provider identity from `EgoLint`'s reviewed adapter catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitignoreSource {
    pub repository: String,
    pub revision: String,
    pub foundation: String,
    pub format: String,
    pub catalog_path: String,
    pub catalog_sha256: String,
}

/// Deterministic bounded report for CLI, Relay, and other evidence consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryGitignoreReport {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub contract: String,
    pub repository: String,
    pub policy_path: PathBuf,
    pub policy_sha256: String,
    pub source: GitignoreSource,
    pub composition_sha256: String,
    pub evaluation_date: String,
    pub status: GitignoreValidationStatus,
    pub checks: GitignoreChecks,
    pub files: Vec<GitignoreFileEvidence>,
    pub behavior: Vec<GitignoreBehaviorEvidence>,
    pub diagnostics: Vec<GitignoreDiagnostic>,
    pub limitations: Vec<String>,
}

/// Focused report and standard findings, without repository file payloads.
pub struct GitignoreEvaluation {
    pub findings: Vec<Finding>,
    pub report: RepositoryGitignoreReport,
}

/// Evaluate only ignore policies, filesystem names, and Git index metadata.
///
/// # Errors
/// Returns an error for an unsafe invocation or invalid request. Missing Git,
/// malformed provider artifacts, drift, and incomplete coverage become findings.
pub fn evaluate_gitignore(
    workspace: &Path,
    policy: &RepositoryGitignorePolicy,
    policy_path: &Path,
    evaluation_date: &str,
) -> Result<GitignoreEvaluation> {
    validate_contract_date(evaluation_date)?;
    contract::validate_policy(policy).map_err(EgolintError::Configuration)?;
    safe_path(&policy_path.to_string_lossy()).map_err(EgolintError::Configuration)?;
    evaluation::evaluate(
        workspace,
        policy,
        policy_path,
        evaluation_date,
        Path::new("git"),
    )
}

/// Persist the focused report through the existing safe report boundary.
///
/// # Errors
/// Returns an error for an unsafe destination or serialization/filesystem failure.
pub fn write_gitignore_report_atomic(
    report: &RepositoryGitignoreReport,
    path: &Path,
) -> Result<()> {
    if !path.ends_with(REPORT_PATH) {
        return Err(EgolintError::Configuration(format!(
            "expected {REPORT_PATH}"
        )));
    }
    let (path, parent) = crate::sarif::validated_report_target(path)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(&parent).map_err(|source| EgolintError::Filesystem {
            path: parent.clone(),
            source,
        })?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), report)?;
    temporary
        .as_file_mut()
        .write_all(b"\n")
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|source| EgolintError::Filesystem {
            path: path.clone(),
            source,
        })?;
    temporary
        .persist(&path)
        .map_err(|error| EgolintError::Filesystem {
            path,
            source: error.error,
        })?;
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_path(path: &str) -> std::result::Result<(), String> {
    if path.is_empty()
        || path == ".reports/egolint"
        || path.starts_with(".reports/egolint/")
        || path.len() > 2048
        || path
            .chars()
            .any(|c| c.is_control() || "\\:*?[]<>|\"!".contains(c))
        || path.split('/').any(|part| {
            part.is_empty()
                || matches!(part, "." | "..")
                || part.trim() != part
                || part.eq_ignore_ascii_case(".git")
        })
    {
        return Err(
            "path must be normalized, relative, literal, and outside Git/report metadata"
                .to_owned(),
        );
    }
    Ok(())
}

fn scope_of(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ".".to_owned(), |(parent, _)| parent.to_owned())
}

fn scope_path(scope: &str, path: &str) -> String {
    if scope == "." {
        path.to_owned()
    } else {
        format!("{scope}/{path}")
    }
}

fn issue(
    report: &mut RepositoryGitignoreReport,
    rule: &str,
    status: GitignoreCheckStatus,
    path: Option<&str>,
    message: impl Into<String>,
    remediation: &str,
) {
    report.diagnostics.push(GitignoreDiagnostic {
        rule_id: format!("EGO-IGNORE-{rule}-001"),
        status,
        path: path.map(str::to_owned),
        scope: path.map(scope_of),
        message: message.into(),
        remediation: remediation.to_owned(),
    });
}

fn findings(report: &RepositoryGitignoreReport) -> Result<Vec<Finding>> {
    report
        .diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            let finding = Finding {
                schema_version: CONTRACT_VERSION,
                id: format!("gitignore-{:06}", index + 1),
                rule: RuleIdentity {
                    tool_id: TOOL_ID.to_owned(),
                    rule_id: diagnostic.rule_id.clone(),
                },
                severity: if diagnostic.status == GitignoreCheckStatus::Passed {
                    Severity::Info
                } else {
                    Severity::Error
                },
                message: format!("{} {}", diagnostic.message, diagnostic.remediation),
                location: diagnostic.path.as_ref().map(|path| SourceLocation {
                    path: PathBuf::from(path),
                    start_line: None,
                    start_column: None,
                    end_line: None,
                    end_column: None,
                }),
                ownership: RuleOwnership {
                    owner: "egohygiene/empathy".to_owned(),
                    policy_source: format!(
                        "https://github.com/{}/blob/{}/docs/foundation/gitignore/README.md",
                        report.source.repository, report.source.revision
                    ),
                    configuration_path: Some(report.policy_path.clone()),
                },
                fingerprint: None,
                suppressed_by: None,
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_VERSION,
                    kind: EvidenceKind::Other,
                    path: PathBuf::from(REPORT_PATH),
                    sha256: None,
                    description: Some(
                        "Bounded ignore policy, winning-rule, and coverage evidence.".to_owned(),
                    ),
                }],
            };
            finding.validate()?;
            Ok(finding)
        })
        .collect()
}
