//! Progressive adoption without hiding execution errors or unreviewed debt.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::{Finding, Severity, ToolStatus, validate_contract_date};
use crate::error::{EgolintError, Result};
use crate::report::{RunReport, RunStatus};

static EXECUTABLE_DIGEST: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

/// Existing consumers retain blocking behavior unless they explicitly select another mode.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    Manual,
    Observe,
    Ratchet,
    #[default]
    Enforce,
}

/// Invocation provenance is data, not a command or publication authorization.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Invocation {
    #[default]
    Local,
    WorkflowDispatch,
    PullRequest,
    Push,
    Schedule,
    Release,
    Automatic,
}

impl Invocation {
    /// Missing event provenance in CI is automatic, never implicitly manual.
    #[must_use]
    pub fn from_environment() -> Self {
        match std::env::var("GITHUB_EVENT_NAME").as_deref() {
            Ok("workflow_dispatch") => Self::WorkflowDispatch,
            Ok("pull_request" | "pull_request_target") => Self::PullRequest,
            Ok("push") => Self::Push,
            Ok("schedule") => Self::Schedule,
            Ok("release") => Self::Release,
            Ok(_) => Self::Automatic,
            Err(_)
                if std::env::var("CI").is_ok_and(|v| {
                    !["", "0", "false", "no"].contains(&v.to_ascii_lowercase().as_str())
                }) =>
            {
                Self::Automatic
            }
            Err(_) => Self::Local,
        }
    }
    #[must_use]
    pub const fn is_manual(self) -> bool {
        matches!(self, Self::Local | Self::WorkflowDispatch)
    }
}

/// Ratchet needs a separately pinned, explicitly reviewed baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct EnforcementConfig {
    pub mode: EnforcementMode,
    pub repository: Option<String>,
    pub baseline: Option<PathBuf>,
    pub baseline_sha256: Option<String>,
}

impl EnforcementConfig {
    /// # Errors
    /// Rejects malformed identities, dates, paths or incomplete review metadata.
    pub fn validate(&self) -> Result<()> {
        if self.mode == EnforcementMode::Ratchet
            && (self.repository.is_none()
                || self.baseline.is_none()
                || self.baseline_sha256.is_none())
        {
            return Err(invalid(
                "ratchet requires repository, baseline and separately reviewed baseline-sha256",
            ));
        }
        if let Some(repository) = &self.repository {
            validate_repository(repository)?;
        }
        if let Some(path) = &self.baseline {
            validate_relative(path)?;
            if path.starts_with(".reports") {
                return Err(invalid(
                    "baseline must be outside the writable report boundary",
                ));
            }
        }
        if let Some(digest) = &self.baseline_sha256 {
            validate_sha(digest)?;
        }
        Ok(())
    }
    #[must_use]
    pub fn should_skip(&self, invocation: Invocation) -> bool {
        self.mode == EnforcementMode::Manual && !invocation.is_manual()
    }
}

/// Commit/tree provenance is explicit about dirty or unavailable source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub commit: String,
    pub tree: String,
    pub dirty: bool,
}

impl SourceSnapshot {
    #[must_use]
    pub fn discover(workspace: &Path) -> Option<Self> {
        let commit = git_text(workspace, &["rev-parse", "--verify", "HEAD^{commit}"])?;
        let tree = git_text(workspace, &["rev-parse", "--verify", "HEAD^{tree}"])?;
        let status = Command::new("git")
            .args(["status", "--porcelain=v1", "--untracked-files=all", "-z"])
            .current_dir(workspace)
            .output()
            .ok()?;
        if !status.status.success() {
            return None;
        }
        let dirty = status
            .stdout
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry.len() < 3 || !entry[3..].starts_with(b".reports/egolint/"));
        Some(Self {
            commit,
            tree,
            dirty,
        })
    }
}

/// Baseline entries carry no source excerpt or raw diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineFinding {
    pub fingerprint: String,
    pub tool_id: String,
    pub rule_id: String,
    pub path: Option<PathBuf>,
    pub severity: Severity,
    pub count: u64,
}

/// A checksum pin lives outside these bytes; an editable self-checksum is not trust.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub repository: String,
    pub represented_commit: String,
    pub represented_tree: String,
    pub policy_sha256: String,
    pub tool_versions: BTreeMap<String, String>,
    pub created_on: String,
    pub expires_on: String,
    pub reviewed_by: String,
    pub review_reference: String,
    pub findings: Vec<BaselineFinding>,
}

impl Baseline {
    /// Create candidate baseline bytes only from a completed, clean, matching source report.
    /// # Errors
    /// Rejects incomplete execution, dirty/mismatched source and invalid review metadata.
    pub fn from_report(
        workspace: &Path,
        report: &RunReport,
        repository: String,
        created_on: String,
        expires_on: String,
        reviewed_by: String,
        review_reference: String,
    ) -> Result<Self> {
        report.validate()?;
        let source = report
            .source
            .as_ref()
            .ok_or_else(|| invalid("baseline report has no represented Git source"))?;
        if source.dirty || SourceSnapshot::discover(workspace).as_ref() != Some(source) {
            return Err(invalid(
                "baseline creation requires the report's exact clean checkout",
            ));
        }
        if report.status == RunStatus::ExecutionError
            || report.completeness != crate::report::ReportCompleteness::Normalized
            || report.tool_results.is_empty()
            || report
                .tool_results
                .iter()
                .any(|t| t.status == ToolStatus::Selected)
            || report
                .enforcement
                .as_ref()
                .is_some_and(|e| e.execution_skipped)
        {
            return Err(invalid("cannot baseline incomplete or skipped execution"));
        }
        if has_unattributed_blocking_outcome(report) {
            return Err(invalid(
                "cannot baseline a blocking outcome without comparable findings",
            ));
        }
        let producing_executable = report
            .enforcement
            .as_ref()
            .and_then(|decision| decision.executable_sha256.as_deref())
            .ok_or_else(|| invalid("baseline requires a fresh producing executable identity"))?;
        if executable_digest().map(String::as_str) != Some(producing_executable) {
            return Err(invalid(
                "baseline report was produced by a different executable; rerun validation",
            ));
        }
        let (policy_sha256, tool_versions) = policy_context(report)?;
        let baseline = Self {
            schema_version: 1,
            repository,
            represented_commit: source.commit.clone(),
            represented_tree: source.tree.clone(),
            policy_sha256,
            tool_versions,
            created_on,
            expires_on,
            reviewed_by,
            review_reference,
            findings: grouped_findings(&report.findings).into_values().collect(),
        };
        baseline.validate()?;
        Ok(baseline)
    }

    /// # Errors
    /// Rejects malformed identities, dates, paths or incomplete review metadata.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(invalid("unsupported baseline schema version"));
        }
        validate_repository(&self.repository)?;
        for value in [&self.represented_commit, &self.represented_tree] {
            validate_git_id(value)?;
        }
        validate_sha(&self.policy_sha256)?;
        validate_contract_date(&self.created_on)?;
        validate_contract_date(&self.expires_on)?;
        if self.created_on >= self.expires_on {
            return Err(invalid("baseline expiry must follow creation"));
        }
        for value in [&self.reviewed_by, &self.review_reference] {
            if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err(invalid(
                    "baseline requires bounded explicit review metadata",
                ));
            }
        }
        let mut identities = std::collections::BTreeSet::new();
        for entry in &self.findings {
            validate_sha(&entry.fingerprint)?;
            if entry.count == 0 || !identities.insert(&entry.fingerprint) {
                return Err(invalid("baseline has duplicate or empty finding groups"));
            }
            if let Some(path) = &entry.path {
                validate_relative(path)?;
            }
        }
        Ok(())
    }

    /// Exclusively create a caller-selected file. No baseline is automatically refreshed.
    /// # Errors
    /// Rejects unsafe/existing targets, invalid baselines and filesystem failures.
    pub fn write_new(&self, workspace: &Path, relative: &Path) -> Result<String> {
        self.validate()?;
        validate_relative(relative)?;
        let target = workspace.join(relative);
        let parent = target
            .parent()
            .ok_or_else(|| invalid("baseline requires a parent directory"))?;
        if !parent.is_dir() || parent.canonicalize().ok().as_deref() != Some(parent) {
            return Err(invalid(
                "baseline parent must exist without symlink aliases",
            ));
        }
        let mut bytes = serde_json::to_vec_pretty(self)?;
        bytes.push(b'\n');
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|source| EgolintError::Filesystem {
                path: target.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| EgolintError::Filesystem {
                path: target,
                source,
            })?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

/// Debt state remains visible even when adoption makes it nonblocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    Resolved,
    Unchanged,
    New,
    Worsened,
    Suppressed,
    Expired,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingDelta {
    pub finding: BaselineFinding,
    pub state: FindingState,
    pub blocking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStatus {
    NotUsed,
    Valid,
    Expired,
    Incompatible,
    Unavailable,
}

/// Effective process result is separate from the run's actual findings/execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnforcementDecision {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub mode: EnforcementMode,
    pub invocation: Invocation,
    pub execution_skipped: bool,
    pub baseline_status: BaselineStatus,
    pub baseline_sha256: Option<String>,
    /// Producing executable bytes; older reports remain readable but need a fresh run to baseline.
    #[serde(default)]
    pub executable_sha256: Option<String>,
    pub effective_exit_code: i32,
    pub changes: Vec<FindingDelta>,
    pub reason: String,
}

impl EnforcementDecision {
    /// Check adoption/result consistency without trusting an embedded success code.
    pub(crate) fn validate_for(&self, report: &RunReport) -> Result<()> {
        if self.schema_version != 1
            || self.reason.is_empty()
            || self.reason.len() > 1024
            || self.reason.chars().any(char::is_control)
        {
            return Err(invalid("invalid enforcement decision"));
        }
        if let Some(digest) = &self.executable_sha256 {
            validate_sha(digest)?;
        }
        let expected = if report.status == RunStatus::ExecutionError {
            3
        } else if self.execution_skipped {
            if self.mode != EnforcementMode::Manual
                || self.invocation.is_manual()
                || !report.findings.is_empty()
                || !report.tool_results.is_empty()
            {
                return Err(invalid("manual skip cannot conceal executed checks"));
            }
            0
        } else {
            match self.mode {
                EnforcementMode::Observe => 0,
                EnforcementMode::Manual | EnforcementMode::Enforce => report.status.exit_code(),
                EnforcementMode::Ratchet => match self.baseline_status {
                    BaselineStatus::Incompatible | BaselineStatus::Unavailable => 3,
                    BaselineStatus::Expired => {
                        if has_unattributed_blocking_outcome(report) {
                            3
                        } else {
                            1
                        }
                    }
                    BaselineStatus::Valid => {
                        if self.baseline_sha256.is_none() {
                            return Err(invalid("ratchet has no reviewed baseline digest"));
                        }
                        if has_unattributed_blocking_outcome(report) {
                            3
                        } else {
                            i32::from(self.changes.iter().any(|c| c.blocking))
                        }
                    }
                    BaselineStatus::NotUsed => {
                        return Err(invalid("ratchet did not evaluate a baseline"));
                    }
                },
            }
        };
        if self.effective_exit_code != expected
            || self.effective_exit_code != report.egolint_exit_code
        {
            return Err(invalid("inconsistent enforcement exit code"));
        }
        Ok(())
    }
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut counts = BTreeMap::new();
        for change in &self.changes {
            *counts.entry(format!("{:?}", change.state)).or_insert(0_u64) += change.finding.count;
        }
        format!(
            "Enforcement: {:?}; invocation={:?}; baseline={:?}; effective exit={}; debt={counts:?}; {}",
            self.mode, self.invocation, self.baseline_status, self.effective_exit_code, self.reason
        )
    }
}

/// Apply adoption only to completed validation; invalid configuration still returns an error.
/// # Errors
/// Rejects invalid configuration, report or baseline provenance and checksum mismatches.
pub fn evaluate(
    workspace: &Path,
    report: &RunReport,
    config: &EnforcementConfig,
    invocation: Invocation,
    evaluation_date: Option<&str>,
) -> Result<EnforcementDecision> {
    report.validate()?;
    config.validate()?;
    let skipped = config.should_skip(invocation);
    let mut decision = EnforcementDecision {
        schema_version: 1,
        mode: config.mode,
        invocation,
        execution_skipped: skipped,
        baseline_status: BaselineStatus::NotUsed,
        baseline_sha256: None,
        executable_sha256: executable_digest().cloned(),
        effective_exit_code: report.status.exit_code(),
        changes: Vec::new(),
        reason: "existing blocking policy applies".to_owned(),
    };
    if report.status == RunStatus::ExecutionError {
        decision.execution_skipped = false;
        "execution failure is never waived by adoption mode".clone_into(&mut decision.reason);
        return Ok(decision);
    }
    if skipped {
        if !report.tool_results.is_empty() || !report.findings.is_empty() {
            return Err(invalid("manual mode must skip before executing checks"));
        }
        decision.effective_exit_code = 0;
        "manual mode skips automatic invocation".clone_into(&mut decision.reason);
        return Ok(decision);
    }
    let current = grouped_findings(&report.findings);
    let mut suppressed_findings = report
        .findings
        .iter()
        .filter(|f| f.suppressed_by.is_some())
        .cloned()
        .collect::<Vec<_>>();
    for finding in &mut suppressed_findings {
        finding.suppressed_by = None;
    }
    let suppressed = grouped_findings(&suppressed_findings)
        .into_values()
        .map(|finding| FindingDelta {
            finding,
            state: FindingState::Suppressed,
            blocking: false,
        })
        .collect::<Vec<_>>();
    if config.mode != EnforcementMode::Ratchet {
        decision.changes = current
            .into_values()
            .map(|finding| {
                let state = FindingState::New;
                let blocking = severity_rank(finding.severity) >= severity_rank(Severity::Error);
                FindingDelta {
                    finding,
                    state,
                    blocking,
                }
            })
            .collect();
        decision.changes.extend(suppressed);
        if config.mode == EnforcementMode::Observe {
            decision.effective_exit_code = 0;
            "completed findings retained as nonblocking adoption evidence"
                .clone_into(&mut decision.reason);
        }
        return Ok(decision);
    }
    evaluate_ratchet(
        workspace,
        report,
        config,
        evaluation_date,
        decision,
        current,
        suppressed,
    )
}

fn evaluate_ratchet(
    workspace: &Path,
    report: &RunReport,
    config: &EnforcementConfig,
    evaluation_date: Option<&str>,
    mut decision: EnforcementDecision,
    current: BTreeMap<String, BaselineFinding>,
    suppressed: Vec<FindingDelta>,
) -> Result<EnforcementDecision> {
    let date = evaluation_date.ok_or_else(|| invalid("ratchet requires --evaluation-date"))?;
    validate_contract_date(date)?;
    let (baseline, digest) = load_baseline(workspace, config)?;
    decision.baseline_sha256 = Some(digest);
    let (policy, versions) = policy_context(report)?;
    if policy != baseline.policy_sha256 || versions != baseline.tool_versions {
        decision.baseline_status = BaselineStatus::Incompatible;
        decision.effective_exit_code = 3;
        "policy/tool/scope changed; explicit baseline review required, not a repository regression claim".clone_into(&mut decision.reason);
        decision.changes = current
            .into_values()
            .map(|finding| FindingDelta {
                finding,
                state: FindingState::Unavailable,
                blocking: true,
            })
            .collect();
        return Ok(decision);
    }
    let expired = date >= baseline.expires_on.as_str();
    if date < baseline.created_on.as_str() {
        return Err(invalid("evaluation date precedes baseline review"));
    }
    let mut previous = baseline
        .findings
        .into_iter()
        .map(|f| (f.fingerprint.clone(), f))
        .collect::<BTreeMap<_, _>>();
    for (key, finding) in current {
        let old = previous.remove(&key);
        let state = if expired {
            FindingState::Expired
        } else if let Some(old) = old {
            if finding.count > old.count
                || severity_rank(finding.severity) > severity_rank(old.severity)
            {
                FindingState::Worsened
            } else {
                FindingState::Unchanged
            }
        } else {
            FindingState::New
        };
        let blocking = matches!(
            state,
            FindingState::New | FindingState::Worsened | FindingState::Expired
        ) && severity_rank(finding.severity) >= severity_rank(Severity::Error);
        decision.changes.push(FindingDelta {
            finding,
            state,
            blocking,
        });
    }
    decision
        .changes
        .extend(previous.into_values().map(|finding| FindingDelta {
            finding,
            state: FindingState::Resolved,
            blocking: false,
        }));
    decision.changes.extend(suppressed);
    decision
        .changes
        .sort_by(|a, b| a.finding.fingerprint.cmp(&b.finding.fingerprint));
    decision.baseline_status = if expired {
        BaselineStatus::Expired
    } else {
        BaselineStatus::Valid
    };
    let blocking = expired || decision.changes.iter().any(|c| c.blocking);
    let unattributed = has_unattributed_blocking_outcome(report);
    if unattributed {
        decision.baseline_status = BaselineStatus::Unavailable;
    }
    decision.effective_exit_code = if unattributed { 3 } else { i32::from(blocking) };
    let reason = if unattributed {
        "blocking adapter outcome lacks baseline-comparable findings"
    } else if expired {
        "baseline review expired"
    } else {
        "new/worsened blocking debt fails; reviewed unchanged debt remains visible"
    };
    reason.clone_into(&mut decision.reason);
    Ok(decision)
}

fn has_unattributed_blocking_outcome(report: &RunReport) -> bool {
    let comparable = |finding: &Finding| {
        finding.suppressed_by.is_none()
            && severity_rank(finding.severity) >= severity_rank(Severity::Error)
    };
    // A native finding cannot account for an incompletely normalized engine failure.
    // A clean adapter result with partial advisory detail does not invent a failure.
    (report.completeness != crate::report::ReportCompleteness::Normalized
        && report.adapter_exit_code.is_some_and(|code| code != 0))
        || report.tool_results.iter().any(|tool| {
            tool.enforcement == crate::contracts::Enforcement::Blocking
                && tool.status == ToolStatus::FailedFindings
                && !report
                    .findings
                    .iter()
                    .any(|finding| finding.rule.tool_id == tool.tool_id && comparable(finding))
        })
        || (report.status == RunStatus::Findings && !report.findings.iter().any(comparable))
}
fn load_baseline(workspace: &Path, config: &EnforcementConfig) -> Result<(Baseline, String)> {
    let path = workspace.join(
        config
            .baseline
            .as_ref()
            .ok_or_else(|| invalid("missing baseline"))?,
    );
    let metadata = path
        .symlink_metadata()
        .map_err(|_| invalid("baseline is unavailable"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > 2 * 1024 * 1024
        || path.canonicalize().ok().as_ref() != Some(&path)
    {
        return Err(invalid(
            "baseline must be a bounded regular file without aliases",
        ));
    }
    let bytes = std::fs::read(&path).map_err(|source| EgolintError::Filesystem { path, source })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if Some(&digest) != config.baseline_sha256.as_ref() {
        return Err(invalid(
            "baseline SHA-256 differs from separately reviewed pin",
        ));
    }
    let baseline: Baseline =
        serde_json::from_slice(&bytes).map_err(|_| invalid("invalid baseline JSON"))?;
    baseline.validate()?;
    if Some(&baseline.repository) != config.repository.as_ref() {
        return Err(invalid("baseline repository does not match invocation"));
    }
    let actual_tree = git_text(
        workspace,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", baseline.represented_commit),
        ],
    );
    let ancestor = Command::new("git")
        .args([
            "merge-base",
            "--is-ancestor",
            &baseline.represented_commit,
            "HEAD",
        ])
        .current_dir(workspace)
        .output();
    if actual_tree.as_deref() != Some(&baseline.represented_tree)
        || !ancestor.is_ok_and(|o| o.status.success())
    {
        return Err(invalid(
            "baseline source must be an available ancestor with its exact represented tree",
        ));
    }
    Ok((baseline, digest))
}

/// Fingerprints ignore line/column and severity, preserve rule/path/message identity and duplicate counts.
#[must_use]
pub fn canonical_fingerprint(finding: &Finding) -> String {
    let message = &finding.message;
    let path = finding
        .location
        .as_ref()
        .map(|l| l.path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut digest = Sha256::new();
    digest.update(b"egolint/finding/v1\0");
    for part in [
        finding.rule.tool_id.as_str(),
        finding.rule.rule_id.as_str(),
        path.as_str(),
        message.as_str(),
    ] {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn grouped_findings(findings: &[Finding]) -> BTreeMap<String, BaselineFinding> {
    let mut groups: BTreeMap<String, BaselineFinding> = BTreeMap::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.suppressed_by.is_none())
    {
        let key = canonical_fingerprint(finding);
        let group = groups
            .entry(key.clone())
            .or_insert_with(|| BaselineFinding {
                fingerprint: key,
                tool_id: finding.rule.tool_id.clone(),
                rule_id: finding.rule.rule_id.clone(),
                path: finding.location.as_ref().map(|l| l.path.clone()),
                severity: finding.severity,
                count: 0,
            });
        group.count += 1;
        if severity_rank(finding.severity) > severity_rank(group.severity) {
            group.severity = finding.severity;
        }
    }
    groups
}

fn policy_context(report: &RunReport) -> Result<(String, BTreeMap<String, String>)> {
    let policy = report
        .policy_sha256
        .clone()
        .ok_or_else(|| invalid("baseline requires a fresh effective policy identity"))?;
    validate_sha(&policy)?;
    let mut versions = BTreeMap::new();
    let digest =
        executable_digest().ok_or_else(|| invalid("cannot bind baseline executable bytes"))?;
    versions.insert("egolint-executable-sha256".to_owned(), digest.clone());
    for tool in &report.tool_results {
        if matches!(
            tool.status,
            ToolStatus::NotApplicable
                | ToolStatus::DisabledByProfile
                | ToolStatus::DisabledByConfiguration
        ) {
            continue;
        }
        let version = report
            .capabilities
            .iter()
            .find(|c| c.id == tool.tool_id)
            .map(|c| c.version.clone())
            .or_else(|| {
                tool.tool_id
                    .starts_with("EGOLINT_")
                    .then(|| env!("CARGO_PKG_VERSION").to_owned())
            })
            .ok_or_else(|| invalid("baseline tool has no version provenance"))?;
        versions.insert(tool.tool_id.clone(), version);
        versions.insert(
            format!("enforcement:{}", tool.tool_id),
            format!("{:?}", tool.enforcement),
        );
        if report
            .capabilities
            .iter()
            .any(|c| c.id == tool.tool_id && c.provider == "megalinter")
        {
            let Some((_, digest)) = report.image.split_once("@sha256:") else {
                return Err(invalid(
                    "engine baseline requires an immutable image digest",
                ));
            };
            validate_sha(digest)?;
            versions.insert("engine-image".to_owned(), report.image.clone());
        }
    }
    versions.insert(
        "profile".to_owned(),
        report.profile.name.as_str().to_owned(),
    );
    Ok((policy, versions))
}

fn executable_digest() -> Option<&'static String> {
    EXECUTABLE_DIGEST
        .get_or_init(|| {
            let executable = std::env::current_exe().ok()?;
            Some(format!(
                "{:x}",
                Sha256::digest(std::fs::read(executable).ok()?)
            ))
        })
        .as_ref()
}

fn git_text(workspace: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_owned())
        })
        .flatten()
}
fn validate_repository(value: &str) -> Result<()> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err(invalid("repository must be an explicit owner/name"));
    }
    Ok(())
}
fn validate_sha(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("expected lowercase SHA-256"));
    }
    Ok(())
}
fn validate_git_id(value: &str) -> Result<()> {
    if ![40, 64].contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("expected full Git object identity"));
    }
    Ok(())
}
fn validate_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .to_str()
            .is_none_or(|s| s.contains('\\') || s.chars().any(char::is_control))
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(invalid(
            "baseline path must be normalized and repository-relative",
        ));
    }
    Ok(())
}
const fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 0,
        Severity::Warning => 1,
        Severity::Error => 2,
        Severity::Critical => 3,
    }
}
fn invalid(message: &str) -> EgolintError {
    EgolintError::Configuration(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Operation, PlanView};
    use crate::{
        Enforcement, Profile, ReportCompleteness, RuleIdentity, RuleOwnership, SourceLocation,
        ToolResult,
    };

    fn repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source.txt"), "synthetic policy fixture\n").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "source.txt"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(root.path())
                    .status()
                    .unwrap()
                    .success()
            );
        }
        root
    }
    fn finding(line: u32, severity: Severity) -> Finding {
        Finding {
            schema_version: 1,
            id: format!("FIXTURE-{line}"),
            rule: RuleIdentity {
                tool_id: "EGOLINT_PORTABILITY".to_owned(),
                rule_id: "FIXTURE-RULE".to_owned(),
            },
            severity,
            message: "synthetic blocking finding".to_owned(),
            location: Some(SourceLocation {
                path: "source.txt".into(),
                start_line: Some(line),
                start_column: Some(1),
                end_line: None,
                end_column: None,
            }),
            ownership: RuleOwnership {
                owner: "fixture".to_owned(),
                policy_source: "fixture/v1".to_owned(),
                configuration_path: None,
            },
            fingerprint: None,
            evidence: Vec::new(),
            suppressed_by: None,
        }
    }
    fn report(root: &Path, findings: Vec<Finding>, failed: bool) -> RunReport {
        let plan = PlanView {
            schema_version: 1,
            operation: Operation::Check,
            profile: Profile::Fast,
            runtime: "docker".to_owned(),
            image: "example.invalid/not-executed:fixture".to_owned(),
            workspace: root.to_owned(),
            report_directory: ".reports/egolint".into(),
            config_sources: Vec::new(),
            argv: Vec::new(),
            capabilities: Vec::new(),
            enforcement_mode: EnforcementMode::Enforce,
            invocation: Invocation::Local,
            source: SourceSnapshot::discover(root),
            policy_sha256: "f".repeat(64),
        };
        let tool = ToolResult {
            schema_version: 1,
            tool_id: "EGOLINT_PORTABILITY".to_owned(),
            owner: "fixture".to_owned(),
            policy_source: "fixture/v1".to_owned(),
            status: if failed {
                ToolStatus::ExecutionError
            } else if findings
                .iter()
                .any(|f| f.suppressed_by.is_none() && severity_rank(f.severity) >= 2)
            {
                ToolStatus::FailedFindings
            } else {
                ToolStatus::Passed
            },
            enforcement: Enforcement::Blocking,
            finding_count: findings.len() as u64,
            warning_count: 0,
            duration_ms: None,
            evidence: Vec::new(),
        };
        let mut report = RunReport::from_plan(&plan, Some(0));
        report
            .set_normalized(
                vec![tool],
                findings,
                Vec::new(),
                Vec::new(),
                ReportCompleteness::Normalized,
            )
            .unwrap();
        report.enforcement = Some(
            evaluate(
                root,
                &report,
                &EnforcementConfig::default(),
                Invocation::Local,
                None,
            )
            .unwrap(),
        );
        report
    }
    fn ratchet(root: &Path, report: &RunReport) -> EnforcementConfig {
        let baseline = Baseline::from_report(
            root,
            report,
            "fixture/repository".to_owned(),
            "2026-11-09".to_owned(),
            "2026-12-01".to_owned(),
            "synthetic-fixture-reviewer".to_owned(),
            "fixture-only".to_owned(),
        )
        .unwrap();
        let sha = baseline
            .write_new(root, Path::new("baseline.json"))
            .unwrap();
        EnforcementConfig {
            mode: EnforcementMode::Ratchet,
            repository: Some("fixture/repository".to_owned()),
            baseline: Some("baseline.json".into()),
            baseline_sha256: Some(sha),
        }
    }
    #[test]
    fn observe_retains_findings_and_never_waives_execution_failure() {
        let root = repository();
        let config = EnforcementConfig {
            mode: EnforcementMode::Observe,
            ..EnforcementConfig::default()
        };
        let report = report(root.path(), vec![finding(1, Severity::Error)], false);
        let decision = evaluate(root.path(), &report, &config, Invocation::Push, None).unwrap();
        assert_eq!(decision.effective_exit_code, 0);
        assert_eq!(report.status, RunStatus::Findings);
        assert_eq!(decision.changes.len(), 1);
        let failed = self::report(root.path(), Vec::new(), true);
        assert_eq!(
            evaluate(root.path(), &failed, &config, Invocation::Push, None)
                .unwrap()
                .effective_exit_code,
            3
        );
        let manual = EnforcementConfig {
            mode: EnforcementMode::Manual,
            ..EnforcementConfig::default()
        };
        assert!(manual.should_skip(Invocation::Schedule));
        assert!(!manual.should_skip(Invocation::WorkflowDispatch));
    }
    #[test]
    fn ratchet_handles_line_movement_worsening_duplicates_renames_and_expiry() {
        let root = repository();
        let old = report(root.path(), vec![finding(1, Severity::Error)], false);
        let config = ratchet(root.path(), &old);
        let moved = report(root.path(), vec![finding(30, Severity::Error)], false);
        let decision = evaluate(
            root.path(),
            &moved,
            &config,
            Invocation::PullRequest,
            Some("2026-11-10"),
        )
        .unwrap();
        assert_eq!(decision.effective_exit_code, 0);
        assert_eq!(decision.changes[0].state, FindingState::Unchanged);
        let worse = report(root.path(), vec![finding(30, Severity::Critical)], false);
        assert_eq!(
            evaluate(
                root.path(),
                &worse,
                &config,
                Invocation::PullRequest,
                Some("2026-11-10")
            )
            .unwrap()
            .changes[0]
                .state,
            FindingState::Worsened
        );
        let duplicate = report(
            root.path(),
            vec![finding(30, Severity::Error), finding(31, Severity::Error)],
            false,
        );
        assert_eq!(
            evaluate(
                root.path(),
                &duplicate,
                &config,
                Invocation::PullRequest,
                Some("2026-11-10")
            )
            .unwrap()
            .effective_exit_code,
            1
        );
        let mut renamed_finding = finding(30, Severity::Error);
        renamed_finding.location.as_mut().unwrap().path = "renamed.txt".into();
        let renamed = report(root.path(), vec![renamed_finding], false);
        let changes = evaluate(
            root.path(),
            &renamed,
            &config,
            Invocation::PullRequest,
            Some("2026-11-10"),
        )
        .unwrap();
        assert!(changes.changes.iter().any(|c| c.state == FindingState::New));
        assert!(
            changes
                .changes
                .iter()
                .any(|c| c.state == FindingState::Resolved)
        );
        let expired = evaluate(
            root.path(),
            &moved,
            &config,
            Invocation::PullRequest,
            Some("2026-12-01"),
        )
        .unwrap();
        assert_eq!(expired.baseline_status, BaselineStatus::Expired);
        assert_eq!(expired.effective_exit_code, 1);
        let mut bytes = std::fs::read(root.path().join("baseline.json")).unwrap();
        bytes.push(b' ');
        std::fs::write(root.path().join("baseline.json"), bytes).unwrap();
        assert!(
            evaluate(
                root.path(),
                &moved,
                &config,
                Invocation::PullRequest,
                Some("2026-11-10")
            )
            .is_err()
        );
    }
    #[test]
    fn suppression_cannot_hide_an_unsuppressed_duplicate_and_dirty_sources_cannot_be_baselined() {
        let root = repository();
        let old = report(root.path(), Vec::new(), false);
        let config = ratchet(root.path(), &old);
        let mut suppressed = finding(1, Severity::Error);
        suppressed.suppressed_by = Some("fixture-suppression".to_owned());
        // Grouping is checked independently; the public report requires actual suppression evidence.
        assert_eq!(
            grouped_findings(&[suppressed, finding(2, Severity::Error)])
                .values()
                .next()
                .unwrap()
                .count,
            1
        );
        std::fs::write(root.path().join("source.txt"), "changed\n").unwrap();
        let dirty = report(root.path(), vec![finding(1, Severity::Error)], false);
        assert!(
            Baseline::from_report(
                root.path(),
                &dirty,
                "fixture/repository".to_owned(),
                "2026-11-09".to_owned(),
                "2026-12-01".to_owned(),
                "fixture".to_owned(),
                "fixture".to_owned()
            )
            .is_err()
        );
        assert!(config.validate().is_ok());
    }

    fn baseline_candidate(root: &Path, report: &RunReport) -> Result<Baseline> {
        Baseline::from_report(
            root,
            report,
            "fixture/repository".to_owned(),
            "2026-11-09".to_owned(),
            "2026-12-01".to_owned(),
            "synthetic-fixture-reviewer".to_owned(),
            "fixture-only".to_owned(),
        )
    }

    #[test]
    fn baseline_requires_complete_evidence_from_the_producing_executable() {
        let root = repository();
        let original = report(root.path(), vec![finding(1, Severity::Error)], false);
        assert!(baseline_candidate(root.path(), &original).is_ok());

        let mut partial = original.clone();
        partial.completeness = ReportCompleteness::Partial;
        partial.validate().unwrap();
        assert!(
            baseline_candidate(root.path(), &partial)
                .unwrap_err()
                .to_string()
                .contains("incomplete"),
        );

        let mut older_json = serde_json::to_value(&original).unwrap();
        older_json["enforcement"]
            .as_object_mut()
            .unwrap()
            .remove("executable_sha256");
        let older: RunReport = serde_json::from_value(older_json).unwrap();
        // Legacy reports are readable, but cannot acquire the current producer's identity.
        older.validate().unwrap();
        assert!(
            baseline_candidate(root.path(), &older)
                .unwrap_err()
                .to_string()
                .contains("fresh producing executable"),
        );

        let mut different = original;
        different.enforcement.as_mut().unwrap().executable_sha256 = Some("0".repeat(64));
        different.validate().unwrap();
        assert!(
            baseline_candidate(root.path(), &different)
                .unwrap_err()
                .to_string()
                .contains("different executable"),
        );
    }

    #[test]
    fn unrelated_reviewed_findings_cannot_waive_an_incomplete_adapter_failure() {
        let root = repository();
        let original = report(root.path(), vec![finding(1, Severity::Error)], false);
        let config = ratchet(root.path(), &original);

        let mut partial = report(root.path(), vec![finding(1, Severity::Error)], false);
        partial.adapter_exit_code = Some(1);
        partial.completeness = ReportCompleteness::Partial;
        partial.validate().unwrap();
        let decision = evaluate(
            root.path(),
            &partial,
            &config,
            Invocation::PullRequest,
            Some("2026-11-10"),
        )
        .unwrap();
        assert_eq!(decision.effective_exit_code, 3);
        assert_eq!(decision.baseline_status, BaselineStatus::Unavailable);
        partial.egolint_exit_code = decision.effective_exit_code;
        partial.enforcement = Some(decision);
        partial.validate().unwrap();
    }

    #[test]
    fn each_blocking_tool_outcome_requires_its_own_comparable_findings() {
        let root = repository();
        let mut original = report(root.path(), vec![finding(1, Severity::Error)], false);
        let mut secondary = original.tool_results[0].clone();
        secondary.tool_id = "EGOLINT_SECONDARY".to_owned();
        secondary.status = ToolStatus::Passed;
        secondary.finding_count = 0;
        original.enforcement = None;
        original
            .set_normalized(
                vec![original.tool_results[0].clone(), secondary],
                original.findings.clone(),
                Vec::new(),
                Vec::new(),
                ReportCompleteness::Normalized,
            )
            .unwrap();
        original.enforcement = Some(
            evaluate(
                root.path(),
                &original,
                &EnforcementConfig::default(),
                Invocation::Local,
                None,
            )
            .unwrap(),
        );

        let mut unattributed = original.clone();
        unattributed.tool_results[1].status = ToolStatus::FailedFindings;
        unattributed.tool_results[1].finding_count = 1;
        unattributed.enforcement = None;
        unattributed.validate().unwrap();
        // The source is still clean: rejection must come from missing comparable tool evidence.
        assert!(
            baseline_candidate(root.path(), &unattributed)
                .unwrap_err()
                .to_string()
                .contains("without comparable findings"),
        );
        let config = ratchet(root.path(), &original);
        let decision = evaluate(
            root.path(),
            &unattributed,
            &config,
            Invocation::PullRequest,
            Some("2026-11-10"),
        )
        .unwrap();
        assert_eq!(decision.effective_exit_code, 3);
        assert_eq!(decision.baseline_status, BaselineStatus::Unavailable);
        unattributed.egolint_exit_code = decision.effective_exit_code;
        unattributed.enforcement = Some(decision);
        unattributed.validate().unwrap();
    }

    #[test]
    fn partial_advisory_detail_does_not_invent_an_adapter_failure() {
        let root = repository();
        let original = report(root.path(), vec![finding(1, Severity::Error)], false);
        let config = ratchet(root.path(), &original);
        let mut current = report(root.path(), vec![finding(30, Severity::Error)], false);
        current.completeness = ReportCompleteness::Partial;
        assert_eq!(current.adapter_exit_code, Some(0));
        let decision = evaluate(
            root.path(),
            &current,
            &config,
            Invocation::PullRequest,
            Some("2026-11-10"),
        )
        .unwrap();
        assert_eq!(decision.effective_exit_code, 0);
        assert_eq!(decision.baseline_status, BaselineStatus::Valid);
        assert_eq!(decision.changes[0].state, FindingState::Unchanged);
    }

    #[test]
    fn policy_tool_and_executable_changes_require_review_instead_of_claiming_new_debt() {
        for changed_input in ["policy", "tool", "executable"] {
            let root = repository();
            let report = report(root.path(), vec![finding(1, Severity::Error)], false);
            let mut baseline = baseline_candidate(root.path(), &report).unwrap();
            match changed_input {
                "policy" => baseline.policy_sha256 = "0".repeat(64),
                "tool" => {
                    baseline
                        .tool_versions
                        .insert("EGOLINT_PORTABILITY".to_owned(), "older-fixture".to_owned());
                }
                "executable" => {
                    baseline
                        .tool_versions
                        .insert("egolint-executable-sha256".to_owned(), "0".repeat(64));
                }
                _ => unreachable!(),
            }
            let config = EnforcementConfig {
                mode: EnforcementMode::Ratchet,
                repository: Some("fixture/repository".to_owned()),
                baseline: Some("baseline.json".into()),
                baseline_sha256: Some(
                    baseline
                        .write_new(root.path(), Path::new("baseline.json"))
                        .unwrap(),
                ),
            };
            let decision = evaluate(
                root.path(),
                &report,
                &config,
                Invocation::PullRequest,
                Some("2026-11-10"),
            )
            .unwrap();
            assert_eq!(decision.effective_exit_code, 3, "{changed_input}");
            assert_eq!(decision.baseline_status, BaselineStatus::Incompatible);
            assert!(
                decision
                    .reason
                    .contains("explicit baseline review required")
            );
            assert!(
                decision
                    .changes
                    .iter()
                    .all(|change| change.state == FindingState::Unavailable),
            );
        }
    }
}
