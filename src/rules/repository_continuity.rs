//! Offline, rollout-aware validation for repository continuity handoffs.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation,
};
use crate::error::{EgolintError, Result};

use super::{RepositoryEntryKind, RepositoryInventory};

/// Stable tool identifier used by normalized findings and tool results.
pub const TOOL_ID: &str = "EGOLINT_REPOSITORY_CONTINUITY";
/// Dedicated privacy-safe validation artifact.
pub const REPORT_PATH: &str = ".reports/egolint/repository-continuity.json";

const POLICY_CONTRACT: &str = "egolint.repository-continuity-validation/v1";
const REPORT_CONTRACT: &str = "egolint.repository-continuity-report/v1";
const CATALOG_PATH: &str = ".config/rules/repository-continuity.v1.toml";
const CATALOG_SOURCE: &str = include_str!("../../.config/rules/repository-continuity.v1.toml");
const AETHER_CONTRACT: &str = "aether.repository-continuity/v1";
const HYGIENE_PROFILE: &str = "egohygiene.repository-continuity-policy/v1";

const FILE_RULE: &str = "EGO-CONTRACT-FILE-001";
const CONTRACT_RULE: &str = "EGO-CONTINUITY-CONTRACT-001";
const SCHEMA_RULE: &str = "EGO-CONTINUITY-SCHEMA-001";
const STRUCTURE_RULE: &str = "EGO-CONTINUITY-STRUCTURE-001";
const IDENTITY_RULE: &str = "EGO-CONTINUITY-IDENTITY-001";
const LINK_RULE: &str = "EGO-CONTINUITY-LINK-001";
const AGENTS_RULE: &str = "EGO-CONTINUITY-AGENTS-001";
const FRESHNESS_RULE: &str = "EGO-CONTINUITY-FRESHNESS-001";
const COMPARISON_RULE: &str = "EGO-CONTINUITY-COMPARISON-001";
const PARALLEL_RULE: &str = "EGO-CONTINUITY-PARALLEL-001";
const EXCEPTION_RULE: &str = "EGO-CONTINUITY-EXCEPTION-001";
const LIVE_RULE: &str = "EGO-CONTINUITY-LIVE-001";
const SAFETY_RULE: &str = "EGO-CONTINUITY-SAFETY-001";

const EXPECTED_RULE_IDS: [&str; 13] = [
    FILE_RULE,
    CONTRACT_RULE,
    SCHEMA_RULE,
    STRUCTURE_RULE,
    IDENTITY_RULE,
    LINK_RULE,
    AGENTS_RULE,
    FRESHNESS_RULE,
    COMPARISON_RULE,
    PARALLEL_RULE,
    EXCEPTION_RULE,
    LIVE_RULE,
    SAFETY_RULE,
];

const REQUIRED_SECTIONS: [&str; 12] = [
    "Purpose and precedence",
    "Resume protocol",
    "Current objective and success conditions",
    "State snapshot",
    "Completed and material changes",
    "Validation and review evidence",
    "Blockers, risks, unknowns, and deferred work",
    "Next dependency-ready work",
    "Parallel changes and reconciliation",
    "Privacy and redaction",
    "Handoff update protocol",
    "Compaction and supersession",
];

/// Hygiene rollout stage applied to continuity diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityRolloutStage {
    /// Findings are visible but capped at warning severity.
    Observe,
    /// Only findings absent from the represented base remain blocking.
    Ratchet,
    /// Every error-level finding remains blocking.
    Enforce,
}

/// Pull-request update declaration supplied by the invoking workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityDisposition {
    /// The candidate intentionally updates the continuity checkpoint.
    Updated,
    /// A reviewer determined that the existing checkpoint remains current.
    ReviewedNoChange,
    /// A reviewed, unexpired exception permits no checkpoint update.
    Exception,
}

/// Lifecycle transition represented by the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityTransition {
    /// A candidate branch is being prepared or reviewed.
    PullRequest,
    /// A merge result is being reconciled after external merge confirmation.
    PostMerge,
}

/// Whether a separate adapter supplied live-state verification evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityLiveVerification {
    /// Ordinary offline validation has no external live-state evidence.
    Unavailable,
    /// A caller supplied stable evidence from an authorized live-state adapter.
    Verified,
}

/// Repository kind used by Hygiene applicability resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuityRepositoryKind {
    Standard,
    Mirror,
    GeneratedOnly,
    Template,
}

/// Repository lifecycle used by Hygiene applicability resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuityLifecycle {
    Active,
    Dormant,
    Archived,
}

/// Repository visibility asserted by local policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuityVisibility {
    Public,
    Private,
    Internal,
}

impl ContinuityVisibility {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Internal => "internal",
        }
    }
}

/// Immutable Hygiene policy lock selected by a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContinuityHygieneLock {
    pub id: String,
    pub version: String,
    pub status: String,
    pub source_repository: String,
    pub source_revision: String,
    pub source_path: PathBuf,
    pub digest: String,
}

/// Immutable Aether compatibility range selected by a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContinuityAetherLock {
    pub id: String,
    pub selected_version: String,
    pub minimum_version: String,
    pub maximum_version_exclusive: String,
    pub lifecycle: String,
    pub release_included: bool,
    pub source_repository: String,
    pub source_revision: String,
    pub source_path: PathBuf,
}

/// Reviewed exception projected from the Hygiene policy process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContinuityException {
    pub repository: String,
    pub owner: String,
    pub reason: String,
    pub approval: Option<String>,
    pub expires_on: String,
    pub review_trigger: String,
    pub validation_state: String,
    pub exit_criteria: String,
}

/// Repository-owned selection of the continuity validation capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RepositoryContinuityPolicy {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub id: String,
    pub repository: String,
    pub rollout_stage: ContinuityRolloutStage,
    pub repository_kind: ContinuityRepositoryKind,
    pub visibility: ContinuityVisibility,
    pub lifecycle: ContinuityLifecycle,
    pub continuity_path: PathBuf,
    pub agents_path: PathBuf,
    #[serde(default)]
    pub provider_projections: Vec<PathBuf>,
    pub profile_path: PathBuf,
    pub schema_path: PathBuf,
    pub instruction_path: PathBuf,
    pub template_path: PathBuf,
    pub hygiene_lock: ContinuityHygieneLock,
    pub aether_lock: ContinuityAetherLock,
    #[serde(default)]
    pub exceptions: Vec<ContinuityException>,
}

impl RepositoryContinuityPolicy {
    /// Decode and structurally validate a repository continuity policy.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed TOML, unsafe paths, duplicate
    /// projections, or an invalid local policy envelope. Unsupported upstream
    /// versions remain reportable findings during evaluation.
    pub fn from_toml(contents: &str, path: &Path) -> Result<Self> {
        let policy: Self = toml::from_str(contents).map_err(|source| EgolintError::Toml {
            path: path.to_path_buf(),
            source,
        })?;
        policy.validate_structure()?;
        Ok(policy)
    }

    fn validate_structure(&self) -> Result<()> {
        if self.schema_version != CONTRACT_VERSION || self.id != POLICY_CONTRACT {
            return Err(EgolintError::Configuration(format!(
                "repository-continuity policy must use schema-version {CONTRACT_VERSION} and id {POLICY_CONTRACT}"
            )));
        }
        if !valid_repository(&self.repository) {
            return Err(EgolintError::Configuration(
                "repository-continuity repository must use owner/name form".to_owned(),
            ));
        }
        for (name, path) in [
            ("continuity", &self.continuity_path),
            ("agents", &self.agents_path),
            ("Hygiene profile", &self.profile_path),
            ("Aether schema", &self.schema_path),
            ("Aether instruction", &self.instruction_path),
            ("Aether template", &self.template_path),
            ("Hygiene source", &self.hygiene_lock.source_path),
            ("Aether source", &self.aether_lock.source_path),
        ] {
            validate_relative_path(path, name)?;
        }
        if self.continuity_path != Path::new("CONTINUITY.md")
            || self.agents_path != Path::new("AGENTS.md")
        {
            return Err(EgolintError::Configuration(
                "continuity and agent paths must be exact-case root CONTINUITY.md and AGENTS.md"
                    .to_owned(),
            ));
        }
        ensure_unique_paths(&self.provider_projections, "provider projection")?;
        for path in &self.provider_projections {
            validate_relative_path(path, "provider projection")?;
            if path == &self.agents_path || path == &self.continuity_path {
                return Err(EgolintError::Configuration(
                    "provider projections may not duplicate root continuity surfaces".to_owned(),
                ));
            }
        }
        let mut exception_repositories = BTreeSet::new();
        for exception in &self.exceptions {
            if !exception_repositories.insert(exception.repository.as_str()) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate continuity exception for {}",
                    exception.repository
                )));
            }
        }
        Ok(())
    }
}

/// Complete explicit comparison input supplied by Relay or a local preflight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuityInvocation {
    pub base_revision: String,
    pub head_revision: String,
    pub parallel_heads: Vec<String>,
    pub disposition: ContinuityDisposition,
    pub transition: ContinuityTransition,
    pub live_verification: ContinuityLiveVerification,
    pub live_evidence: Vec<String>,
    pub evaluation_date: String,
}

impl ContinuityInvocation {
    /// Validate caller-supplied comparison evidence without resolving Git.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed revisions, dates, duplicate parallel
    /// heads, or contradictory live-state evidence.
    pub fn validate(&self) -> Result<()> {
        if self.base_revision != "unborn" && !valid_commit(&self.base_revision) {
            return Err(EgolintError::Configuration(
                "continuity base must be a full lowercase Git SHA or unborn".to_owned(),
            ));
        }
        if self.head_revision != "working-tree" && !valid_commit(&self.head_revision) {
            return Err(EgolintError::Configuration(
                "continuity head must be a full lowercase Git SHA or working-tree".to_owned(),
            ));
        }
        if !valid_date(&self.evaluation_date) {
            return Err(EgolintError::Configuration(
                "continuity evaluation date must be a real YYYY-MM-DD date".to_owned(),
            ));
        }
        let mut heads = BTreeSet::new();
        for head in &self.parallel_heads {
            if !valid_commit(head) {
                return Err(EgolintError::Configuration(
                    "parallel continuity heads must be full lowercase Git SHAs".to_owned(),
                ));
            }
            if !heads.insert(head) {
                return Err(EgolintError::Configuration(
                    "parallel continuity heads must be unique".to_owned(),
                ));
            }
            if head == &self.head_revision || head == &self.base_revision {
                return Err(EgolintError::Configuration(
                    "parallel continuity heads must differ from the represented base and head"
                        .to_owned(),
                ));
            }
        }
        if self.live_verification == ContinuityLiveVerification::Verified
            && self.live_evidence.is_empty()
        {
            return Err(EgolintError::Configuration(
                "verified continuity live state requires at least one stable evidence URL"
                    .to_owned(),
            ));
        }
        if self.live_verification == ContinuityLiveVerification::Unavailable
            && !self.live_evidence.is_empty()
        {
            return Err(EgolintError::Configuration(
                "live evidence may be supplied only with verified continuity live state".to_owned(),
            ));
        }
        if self
            .live_evidence
            .iter()
            .any(|value| !valid_stable_url(value))
        {
            return Err(EgolintError::Configuration(
                "continuity live evidence must use stable HTTPS URLs".to_owned(),
            ));
        }
        if self.live_evidence.iter().collect::<BTreeSet<_>>().len() != self.live_evidence.len() {
            return Err(EgolintError::Configuration(
                "continuity live evidence URLs must be unique".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Requirement resulting from Hygiene applicability resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityRequirement {
    Required,
    Advisory,
    NotApplicable,
}

/// Availability of one explicit Git revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityRevisionState {
    Available,
    Unavailable,
    Unborn,
    WorkingTree,
}

/// Privacy-safe representation of one requested revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ContinuityRevisionEvidence {
    pub requested: String,
    pub state: ContinuityRevisionState,
    pub resolved_revision: Option<String>,
}

/// Local topology determined without fetching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityTopology {
    Linear,
    MergeCommit,
    Diverged,
    SameRevision,
    WorkingTree,
    Unborn,
    Unavailable,
}

/// Status of one separately bounded evidence layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityEvidenceStatus {
    Valid,
    Invalid,
    Incomplete,
    Verified,
    Unavailable,
    NotApplicable,
    RequiresExternalVerification,
}

/// Explicit separation between deterministic and external evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ContinuityEvidenceLayers {
    pub structural: ContinuityEvidenceStatus,
    pub freshness_declaration: ContinuityEvidenceStatus,
    pub local_git: ContinuityEvidenceStatus,
    pub external_live_state: ContinuityEvidenceStatus,
}

/// Complete base/head comparison summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ContinuityComparisonReport {
    pub base: ContinuityRevisionEvidence,
    pub head: ContinuityRevisionEvidence,
    pub topology: ContinuityTopology,
    pub shallow_repository: bool,
    pub continuity_changed: Option<bool>,
    pub agents_changed: Option<bool>,
    pub parallel_heads_requested: u64,
    pub parallel_checkpoint_conflicts: u64,
    pub disposition: ContinuityDisposition,
    pub transition: ContinuityTransition,
    pub live_verification: ContinuityLiveVerification,
    pub live_evidence: Vec<String>,
}

/// Semantic validity independent of rollout-stage severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityValidationStatus {
    Valid,
    Invalid,
    Incomplete,
    NotApplicable,
}

/// One privacy-safe continuity diagnostic. Checkpoint prose is never copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ContinuityDiagnostic {
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub location: Option<SourceLocation>,
    pub expected_state: String,
    pub actual_state: String,
    pub message: String,
    pub remediation: String,
    pub contracts: Vec<String>,
}

/// Exact bounded counts for Relay, Pace, and Observatory consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ContinuityValidationSummary {
    pub diagnostics: u64,
    pub blocking_diagnostics: u64,
    pub files_checked: u64,
    pub provider_projections_checked: u64,
    pub markdown_links_checked: u64,
    pub exceptions_available: u64,
    pub exceptions_applied: u64,
    pub parallel_heads_checked: u64,
}

/// Dedicated deterministic continuity report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RepositoryContinuityReport {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub contract: String,
    pub catalog_version: String,
    pub repository: String,
    pub policy_path: PathBuf,
    pub rollout_stage: ContinuityRolloutStage,
    pub applicability: ContinuityRequirement,
    pub hygiene_profile: ContinuityHygieneLock,
    pub aether_contract: ContinuityAetherLock,
    pub comparison: ContinuityComparisonReport,
    pub evidence_layers: ContinuityEvidenceLayers,
    pub status: ContinuityValidationStatus,
    pub summary: ContinuityValidationSummary,
    pub diagnostics: Vec<ContinuityDiagnostic>,
}

/// Normalized findings plus the dedicated continuity report.
pub struct ContinuityEvaluation {
    pub findings: Vec<Finding>,
    pub report: RepositoryContinuityReport,
}

/// Atomically write the dedicated continuity report in Egolint's report boundary.
///
/// # Errors
///
/// Returns an error when the destination escapes the report boundary or the
/// report cannot be serialized, synchronized, or persisted.
pub fn write_continuity_report_atomic(
    report: &RepositoryContinuityReport,
    path: &Path,
) -> Result<()> {
    if path != Path::new(REPORT_PATH) && !path.ends_with(REPORT_PATH) {
        return Err(EgolintError::Configuration(format!(
            "repository-continuity report path must end with {REPORT_PATH}"
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
    temporary
        .persist(&path)
        .map_err(|error| EgolintError::Filesystem {
            path,
            source: error.error,
        })?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct BundledCatalogSource {
    schema_version: u32,
    catalog_version: String,
    owner: String,
    tool_id: String,
    policy_source: String,
    upstream_contracts: Vec<CatalogContract>,
    rules: Vec<CatalogRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CatalogContract {
    id: String,
    version: String,
    authority: String,
    release_included: bool,
    source_repository: String,
    source_revision: String,
    source_path: PathBuf,
    digest: Option<String>,
    #[serde(default)]
    artifacts: Vec<CatalogArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CatalogArtifact {
    kind: String,
    path: PathBuf,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CatalogRule {
    id: String,
    title: String,
    default_severity: Severity,
    contracts: Vec<String>,
    remediation: String,
}

#[derive(Debug)]
struct BundledCatalog {
    version: String,
    contracts: BTreeMap<String, CatalogContract>,
    rules: BTreeMap<String, CatalogRule>,
}

impl BundledCatalog {
    fn load() -> Result<Self> {
        let source: BundledCatalogSource = toml::from_str(CATALOG_SOURCE).map_err(|error| {
            EgolintError::Configuration(format!(
                "bundled repository-continuity catalog is invalid: {error}"
            ))
        })?;
        if source.schema_version != CONTRACT_VERSION
            || source.owner != "egohygiene/egolint"
            || source.tool_id != TOOL_ID
            || source.policy_source != CATALOG_PATH
        {
            return Err(EgolintError::Configuration(
                "bundled repository-continuity catalog identity is invalid".to_owned(),
            ));
        }
        let contracts = source
            .upstream_contracts
            .into_iter()
            .map(|contract| (contract.id.clone(), contract))
            .collect::<BTreeMap<_, _>>();
        if contracts.len() != 2
            || !contracts.contains_key(HYGIENE_PROFILE)
            || !contracts.contains_key(AETHER_CONTRACT)
        {
            return Err(EgolintError::Configuration(
                "bundled continuity catalog must contain exactly the Hygiene and Aether contracts"
                    .to_owned(),
            ));
        }
        let rules = source
            .rules
            .into_iter()
            .map(|rule| (rule.id.clone(), rule))
            .collect::<BTreeMap<_, _>>();
        if rules.len() != EXPECTED_RULE_IDS.len()
            || EXPECTED_RULE_IDS
                .iter()
                .any(|rule| !rules.contains_key(*rule))
        {
            return Err(EgolintError::Configuration(
                "bundled continuity catalog rule inventory is incomplete or duplicated".to_owned(),
            ));
        }
        for rule in rules.values() {
            if rule.title.trim().is_empty()
                || rule.remediation.trim().is_empty()
                || rule.contracts.is_empty()
                || rule
                    .contracts
                    .iter()
                    .any(|contract| !contracts.contains_key(contract))
            {
                return Err(EgolintError::Configuration(format!(
                    "bundled continuity rule {} has invalid metadata",
                    rule.id
                )));
            }
        }
        Ok(Self {
            version: source.catalog_version,
            contracts,
            rules,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticImpact {
    Invalid,
    Incomplete,
    Advisory,
}

#[derive(Debug, Clone)]
struct RawDiagnostic {
    rule_id: &'static str,
    key: String,
    path: Option<PathBuf>,
    line: Option<u32>,
    expected: String,
    actual: String,
    message: String,
    impact: DiagnosticImpact,
    severity: Option<Severity>,
}

impl RawDiagnostic {
    fn new(
        rule_id: &'static str,
        key: impl Into<String>,
        path: Option<&Path>,
        expected: impl Into<String>,
        actual: impl Into<String>,
        message: impl Into<String>,
        impact: DiagnosticImpact,
    ) -> Self {
        Self {
            rule_id,
            key: key.into(),
            path: path.map(Path::to_path_buf),
            line: None,
            expected: expected.into(),
            actual: actual.into(),
            message: message.into(),
            impact,
            severity: None,
        }
    }

    const fn at_line(mut self, line: Option<u32>) -> Self {
        self.line = line;
        self
    }

    const fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    fn baseline_key(&self) -> String {
        format!(
            "{}\0{}\0{}",
            self.rule_id,
            self.path.as_deref().map_or_else(String::new, portable_path),
            self.key
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct CoreStats {
    files: u64,
    provider_projections: u64,
    markdown_links: u64,
}

#[derive(Debug, Clone)]
struct SnapshotEntry {
    kind: RepositoryEntryKind,
    content: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
struct RepositorySnapshot {
    entries: BTreeMap<PathBuf, SnapshotEntry>,
}

impl RepositorySnapshot {
    fn from_inventory(inventory: &RepositoryInventory) -> Self {
        let entries = inventory
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.path.clone(),
                    SnapshotEntry {
                        kind: entry.kind,
                        content: Some(entry.content.clone()),
                    },
                )
            })
            .collect();
        Self { entries }
    }

    fn get(&self, path: &Path) -> Option<&SnapshotEntry> {
        self.entries.get(path)
    }

    fn contains_path(&self, path: &Path) -> bool {
        self.entries.contains_key(path)
            || self
                .entries
                .keys()
                .any(|entry| entry.ancestors().skip(1).any(|ancestor| ancestor == path))
    }

    fn mis_cased(&self, path: &Path) -> Option<&Path> {
        let expected = portable_path(path).to_ascii_lowercase();
        self.entries
            .keys()
            .find(|candidate| portable_path(candidate).to_ascii_lowercase() == expected)
            .map(PathBuf::as_path)
    }
}

#[derive(Debug)]
struct GitComparison {
    base: ContinuityRevisionEvidence,
    head: ContinuityRevisionEvidence,
    base_snapshot: Option<RepositorySnapshot>,
    head_snapshot: Option<RepositorySnapshot>,
    topology: ContinuityTopology,
    shallow: bool,
    continuity_changed: Option<bool>,
    agents_changed: Option<bool>,
    parallel_conflicts: u64,
    parallel_checked: u64,
    diagnostics: Vec<RawDiagnostic>,
}

#[derive(Debug, Default)]
struct ParsedContinuity {
    metadata: Option<ContinuityMetadata>,
}

/// Evaluator retaining local policy and explicit comparison evidence.
pub struct RepositoryContinuityEvaluator<'a> {
    policy: &'a RepositoryContinuityPolicy,
    policy_path: PathBuf,
    invocation: ContinuityInvocation,
}

impl<'a> RepositoryContinuityEvaluator<'a> {
    /// Construct an evaluator from one validated local policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy path or invocation is unsafe.
    pub fn new(
        policy: &'a RepositoryContinuityPolicy,
        policy_path: &Path,
        invocation: ContinuityInvocation,
    ) -> Result<Self> {
        policy.validate_structure()?;
        validate_relative_path(policy_path, "repository-continuity policy")?;
        invocation.validate()?;
        Ok(Self {
            policy,
            policy_path: policy_path.to_path_buf(),
            invocation,
        })
    }

    /// Evaluate local projections, checkpoint structure, and explicit Git evidence.
    ///
    /// # Errors
    ///
    /// Returns an error only for invalid policy/catalog input or an inaccessible
    /// workspace. Conformance drift is returned as normalized findings.
    #[allow(clippy::too_many_lines)]
    pub fn evaluate(
        &self,
        workspace: &Path,
        inventory: &RepositoryInventory,
    ) -> Result<ContinuityEvaluation> {
        let catalog = BundledCatalog::load()?;
        let content_paths = self.content_paths();
        let mut git = collect_git_comparison(
            workspace,
            inventory,
            &content_paths,
            self.policy,
            &self.invocation,
        )?;
        let applicability = resolve_applicability(self.policy);
        let mut diagnostics = Vec::new();
        let mut stats = CoreStats::default();
        let mut parsed = ParsedContinuity::default();

        if let Some(head) = &git.head_snapshot {
            let (mut head_diagnostics, head_stats, head_parsed) =
                self.evaluate_snapshot(head, &catalog, applicability);
            diagnostics.append(&mut head_diagnostics);
            stats = head_stats;
            parsed = head_parsed;
        } else {
            diagnostics.push(
                RawDiagnostic::new(
                    COMPARISON_RULE,
                    "head-unavailable",
                    None,
                    "locally available comparison head",
                    "unavailable",
                    "the requested head revision is unavailable locally; no repository content was inferred",
                    DiagnosticImpact::Incomplete,
                )
                .with_severity(Severity::Error),
            );
        }

        let exception_state = if applicability == ContinuityRequirement::NotApplicable {
            diagnostics.clear();
            git.diagnostics.clear();
            ExceptionState::None
        } else {
            let exception_state =
                evaluate_exceptions(self.policy, &self.invocation, &mut diagnostics);
            evaluate_comparison(
                workspace,
                self.policy,
                &self.invocation,
                &git,
                parsed.metadata.as_ref(),
                exception_state,
                &mut diagnostics,
            );
            diagnostics.append(&mut git.diagnostics);
            exception_state
        };

        let baseline_keys = if self.policy.rollout_stage == ContinuityRolloutStage::Ratchet {
            if let Some(base) = &git.base_snapshot {
                let (baseline, _, _) = self.evaluate_snapshot(base, &catalog, applicability);
                baseline
                    .iter()
                    .map(RawDiagnostic::baseline_key)
                    .collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            }
        } else {
            BTreeSet::new()
        };

        diagnostics.sort_by(raw_diagnostic_order);
        diagnostics.dedup_by(|left, right| left.baseline_key() == right.baseline_key());

        let mut normalized = Vec::new();
        let mut findings = Vec::new();
        for raw in &diagnostics {
            let rule = catalog.rules.get(raw.rule_id).ok_or_else(|| {
                EgolintError::Configuration(format!(
                    "continuity diagnostic references unknown rule {}",
                    raw.rule_id
                ))
            })?;
            let severity = effective_severity(
                raw.severity.unwrap_or(rule.default_severity),
                self.policy.rollout_stage,
                applicability,
                baseline_keys.contains(&raw.baseline_key()),
            );
            let diagnostic = normalize_diagnostic(raw, rule, severity);
            findings.push(self.finding(&diagnostic));
            normalized.push(diagnostic);
        }
        for finding in &findings {
            finding.validate()?;
        }

        let overall_result = validation_status(applicability, &diagnostics);
        let layers = evidence_layers(
            applicability,
            &diagnostics,
            &self.invocation,
            parsed.metadata.as_ref(),
        );
        let blocking_diagnostics = normalized
            .iter()
            .filter(|item| matches!(item.severity, Severity::Error | Severity::Critical))
            .count() as u64;
        let exceptions_applied = u64::from(exception_state == ExceptionState::Applied);
        let report = RepositoryContinuityReport {
            schema_version: CONTRACT_VERSION,
            contract: REPORT_CONTRACT.to_owned(),
            catalog_version: catalog.version,
            repository: self.policy.repository.clone(),
            policy_path: self.policy_path.clone(),
            rollout_stage: self.policy.rollout_stage,
            applicability,
            hygiene_profile: self.policy.hygiene_lock.clone(),
            aether_contract: self.policy.aether_lock.clone(),
            comparison: ContinuityComparisonReport {
                base: git.base,
                head: git.head,
                topology: git.topology,
                shallow_repository: git.shallow,
                continuity_changed: git.continuity_changed,
                agents_changed: git.agents_changed,
                parallel_heads_requested: self.invocation.parallel_heads.len() as u64,
                parallel_checkpoint_conflicts: git.parallel_conflicts,
                disposition: self.invocation.disposition,
                transition: self.invocation.transition,
                live_verification: self.invocation.live_verification,
                live_evidence: self.invocation.live_evidence.clone(),
            },
            evidence_layers: layers,
            status: overall_result,
            summary: ContinuityValidationSummary {
                diagnostics: normalized.len() as u64,
                blocking_diagnostics,
                files_checked: stats.files,
                provider_projections_checked: stats.provider_projections,
                markdown_links_checked: stats.markdown_links,
                exceptions_available: self.policy.exceptions.len() as u64,
                exceptions_applied,
                parallel_heads_checked: git.parallel_checked,
            },
            diagnostics: normalized,
        };
        Ok(ContinuityEvaluation { findings, report })
    }

    fn content_paths(&self) -> BTreeSet<PathBuf> {
        let mut paths = BTreeSet::from([
            self.policy.continuity_path.clone(),
            self.policy.agents_path.clone(),
            self.policy.profile_path.clone(),
            self.policy.schema_path.clone(),
            self.policy.instruction_path.clone(),
            self.policy.template_path.clone(),
        ]);
        paths.extend(self.policy.provider_projections.iter().cloned());
        paths
    }

    fn evaluate_snapshot(
        &self,
        snapshot: &RepositorySnapshot,
        catalog: &BundledCatalog,
        applicability: ContinuityRequirement,
    ) -> (Vec<RawDiagnostic>, CoreStats, ParsedContinuity) {
        if applicability == ContinuityRequirement::NotApplicable {
            return (
                Vec::new(),
                CoreStats::default(),
                ParsedContinuity::default(),
            );
        }
        let mut diagnostics = Vec::new();
        let mut stats = CoreStats::default();
        evaluate_contract_projections(self.policy, snapshot, catalog, &mut diagnostics, &mut stats);
        let expected_block = expected_managed_block(self.policy, snapshot, &mut diagnostics);
        evaluate_agents(
            self.policy,
            snapshot,
            expected_block.as_deref(),
            &mut diagnostics,
            &mut stats,
        );
        let parsed =
            evaluate_continuity_document(self.policy, snapshot, &mut diagnostics, &mut stats);
        (diagnostics, stats, parsed)
    }

    fn finding(&self, diagnostic: &ContinuityDiagnostic) -> Finding {
        Finding {
            schema_version: CONTRACT_VERSION,
            id: diagnostic.id.clone(),
            rule: RuleIdentity {
                tool_id: TOOL_ID.to_owned(),
                rule_id: diagnostic.rule_id.clone(),
            },
            severity: diagnostic.severity,
            message: format!(
                "{} Remediation: {}",
                diagnostic.message, diagnostic.remediation
            ),
            location: diagnostic.location.clone(),
            ownership: RuleOwnership {
                owner: "egohygiene/egolint".to_owned(),
                policy_source: format!("{CATALOG_PATH}#{}", diagnostic.rule_id),
                configuration_path: Some(self.policy_path.clone()),
            },
            fingerprint: Some(
                diagnostic
                    .id
                    .strip_prefix(&format!("{}-", diagnostic.rule_id))
                    .unwrap_or(&diagnostic.id)
                    .to_owned(),
            ),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Policy,
                path: PathBuf::from(CATALOG_PATH),
                sha256: None,
                description: Some(format!(
                    "Egolint continuity rule mapped to {}.",
                    diagnostic.contracts.join(", ")
                )),
            }],
            suppressed_by: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityMetadata {
    schema_version: String,
    repository: ContinuityRepositoryMetadata,
    document: ContinuityDocumentMetadata,
    scope: ContinuityScopeMetadata,
    work: ContinuityWorkMetadata,
    state: ContinuityStateMetadata,
    review: ContinuityReviewMetadata,
    privacy: ContinuityPrivacyMetadata,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityRepositoryMetadata {
    id: String,
    visibility: String,
    default_branch: String,
    continuity_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContinuityDocumentStatus {
    Active,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityDocumentMetadata {
    status: ContinuityDocumentStatus,
    updated_at: String,
    max_bytes: u64,
    max_lines: u64,
    stale_reason: Option<String>,
    superseded_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityScopeMetadata {
    purpose: String,
    includes: Vec<String>,
    excludes: Vec<String>,
    precedence: Vec<String>,
    canonical_sources: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityWorkMetadata {
    objective: String,
    success_conditions: Vec<String>,
    active_issue: Option<ContinuityReference>,
    next: ContinuityNextWork,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityReference {
    provider: String,
    id: String,
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityNextWork {
    kind: String,
    id: String,
    description: String,
    readiness: String,
    references: Vec<String>,
    depends_on: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityStateMetadata {
    base: ContinuityBaseState,
    candidate: ContinuityCandidateState,
    live: ContinuityLiveState,
    parallel_changes: Vec<ContinuityReference>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityBaseState {
    revision: String,
    r#ref: String,
    verified_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContinuityHandoffState {
    NoActiveChange,
    InProgress,
    ReadyForReview,
    ReviewReferenceRecorded,
    PostMergeReconciliation,
    Abandoned,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityCandidateState {
    branch: Option<String>,
    revision: Option<String>,
    pull_request: Option<ContinuityReference>,
    handoff_state: ContinuityHandoffState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContinuityLiveStatus {
    Verified,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityLiveState {
    status: ContinuityLiveStatus,
    observed_at: String,
    default_branch_revision: Option<String>,
    issue_state: String,
    pull_request_state: String,
    notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContinuityReviewStatus {
    Passed,
    Partial,
    Failed,
    NotRun,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityReviewMetadata {
    status: ContinuityReviewStatus,
    reviewed_at: Option<String>,
    reviewed_by: Option<String>,
    evidence: Vec<ContinuityReviewEvidence>,
    environment_limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityReviewEvidence {
    command: String,
    outcome: String,
    observed_at: String,
    notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuityPrivacyMetadata {
    classification: String,
    contains_sensitive_data: bool,
    redactions: Vec<String>,
    excluded: Vec<String>,
    untrusted_content: String,
}

#[allow(clippy::too_many_lines)]
fn evaluate_contract_projections(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    catalog: &BundledCatalog,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) {
    let hygiene = catalog
        .contracts
        .get(HYGIENE_PROFILE)
        .expect("catalog validated Hygiene contract");
    let aether = catalog
        .contracts
        .get(AETHER_CONTRACT)
        .expect("catalog validated Aether contract");
    let hygiene_lock_valid = policy.hygiene_lock.id == hygiene.id
        && policy.hygiene_lock.version == hygiene.version
        && policy.hygiene_lock.status == hygiene.authority
        && policy.hygiene_lock.source_repository == hygiene.source_repository
        && policy.hygiene_lock.source_revision == hygiene.source_revision
        && policy.hygiene_lock.source_path == hygiene.source_path
        && hygiene
            .digest
            .as_deref()
            .is_some_and(|digest| policy.hygiene_lock.digest == digest);
    if !hygiene_lock_valid {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            "unsupported-hygiene-lock",
            Some(&policy.profile_path),
            "the exact bundled Hygiene continuity-policy lock",
            "unsupported lock",
            "the repository selects an unsupported Hygiene continuity-policy version or provenance",
            DiagnosticImpact::Invalid,
        ));
    }
    let aether_lock_valid = policy.aether_lock.id == aether.id
        && policy.aether_lock.selected_version == aether.version
        && policy.aether_lock.lifecycle == aether.authority
        && policy.aether_lock.release_included == aether.release_included
        && policy.aether_lock.source_repository == aether.source_repository
        && policy.aether_lock.source_revision == aether.source_revision
        && policy.aether_lock.source_path == aether.source_path
        && policy.aether_lock.minimum_version == "1.0.0"
        && policy.aether_lock.maximum_version_exclusive == "2.0.0"
        && version_in_range(
            &policy.aether_lock.selected_version,
            &policy.aether_lock.minimum_version,
            &policy.aether_lock.maximum_version_exclusive,
        );
    if !aether_lock_valid {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            "unsupported-aether-lock",
            Some(&policy.schema_path),
            "a supported Aether v1 compatibility range and immutable provenance",
            "unsupported lock",
            "the repository selects an unsupported Aether continuity contract or compatibility range",
            DiagnosticImpact::Invalid,
        ));
    }

    let profile = projection_bytes(
        snapshot,
        &policy.profile_path,
        "Hygiene continuity profile",
        diagnostics,
        stats,
    );
    if let Some(profile) = profile {
        let digest = sha256(profile);
        if digest != policy.hygiene_lock.digest {
            diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                "hygiene-profile-digest",
                Some(&policy.profile_path),
                "pinned Hygiene profile digest",
                "digest mismatch",
                "the local Hygiene continuity profile differs from the reviewed immutable projection",
                DiagnosticImpact::Invalid,
            ));
        }
        match serde_json::from_slice::<Value>(profile) {
            Ok(value) => {
                let profile_identity_valid = value.get("schema").and_then(Value::as_str)
                    == Some(HYGIENE_PROFILE)
                    && value.get("version").and_then(Value::as_str)
                        == Some(policy.hygiene_lock.version.as_str())
                    && value.get("status").and_then(Value::as_str)
                        == Some(policy.hygiene_lock.status.as_str());
                let upstream = value.get("upstream");
                let upstream_valid = upstream
                    .and_then(|value| value.get("contract_id"))
                    .and_then(Value::as_str)
                    == Some(AETHER_CONTRACT)
                    && upstream
                        .and_then(|value| value.get("contract_version"))
                        .and_then(Value::as_str)
                        == Some(policy.aether_lock.selected_version.as_str())
                    && upstream
                        .and_then(|value| value.get("revision"))
                        .and_then(Value::as_str)
                        == Some(policy.aether_lock.source_revision.as_str())
                    && upstream
                        .and_then(|value| value.get("lifecycle"))
                        .and_then(Value::as_str)
                        == Some(policy.aether_lock.lifecycle.as_str())
                    && upstream
                        .and_then(|value| value.get("release_included"))
                        .and_then(Value::as_bool)
                        == Some(policy.aether_lock.release_included);
                if !profile_identity_valid || !upstream_valid {
                    diagnostics.push(RawDiagnostic::new(
                        CONTRACT_RULE,
                        "hygiene-profile-content",
                        Some(&policy.profile_path),
                        "profile and upstream identities matching the selected locks",
                        "identity mismatch",
                        "the pinned Hygiene profile does not compose the selected Aether contract",
                        DiagnosticImpact::Invalid,
                    ));
                }
                validate_profile_artifacts(policy, &value, snapshot, aether, diagnostics, stats);
            }
            Err(_) => diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                "hygiene-profile-json",
                Some(&policy.profile_path),
                "valid reviewed Hygiene profile JSON",
                "malformed JSON",
                "the local Hygiene continuity profile is not valid JSON",
                DiagnosticImpact::Invalid,
            )),
        }
    }

    if policy.hygiene_lock.status != "active"
        || policy.aether_lock.lifecycle != "stable"
        || !policy.aether_lock.release_included
    {
        let impact = if policy.rollout_stage == ContinuityRolloutStage::Observe {
            DiagnosticImpact::Advisory
        } else {
            DiagnosticImpact::Invalid
        };
        diagnostics.push(
            RawDiagnostic::new(
                CONTRACT_RULE,
                "upstream-lifecycle",
                Some(&policy.profile_path),
                "active released Hygiene and stable releasable Aether inputs before ratchet or enforce",
                "proposed or unreleased upstream inputs",
                "continuity contracts are immutable but not yet released; rollout must remain observe",
                impact,
            )
            .with_severity(if impact == DiagnosticImpact::Advisory {
                Severity::Warning
            } else {
                Severity::Error
            }),
        );
    }
}

fn validate_profile_artifacts(
    policy: &RepositoryContinuityPolicy,
    profile: &Value,
    snapshot: &RepositorySnapshot,
    aether: &CatalogContract,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) {
    let declared = profile
        .get("upstream")
        .and_then(|value| value.get("artifacts"))
        .and_then(Value::as_array);
    for (kind, path) in [
        ("schema", &policy.schema_path),
        ("instruction", &policy.instruction_path),
        ("template", &policy.template_path),
    ] {
        let expected = aether
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == kind);
        let Some(expected) = expected else {
            diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                format!("catalog-artifact-{kind}"),
                Some(path),
                "bundled artifact declaration",
                "missing catalog entry",
                "the bundled continuity catalog omits a required Aether artifact",
                DiagnosticImpact::Invalid,
            ));
            continue;
        };
        let bytes = projection_bytes(
            snapshot,
            path,
            &format!("Aether {kind}"),
            diagnostics,
            stats,
        );
        if bytes.is_some_and(|bytes| sha256(bytes) != expected.digest) {
            diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                format!("aether-{kind}-digest"),
                Some(path),
                "pinned Aether artifact digest",
                "digest mismatch",
                format!("the local Aether {kind} differs from the reviewed immutable projection"),
                DiagnosticImpact::Invalid,
            ));
        }
        let profile_artifact = declared.and_then(|items| {
            items.iter().find_map(|item| {
                (item.get("kind").and_then(Value::as_str) == Some(kind)).then(|| {
                    (
                        item.get("path").and_then(Value::as_str),
                        item.get("sha256_utf8_lf").and_then(Value::as_str),
                    )
                })
            })
        });
        let expected_source_path = portable_path(&expected.path);
        if profile_artifact
            != Some((
                Some(expected_source_path.as_str()),
                Some(expected.digest.as_str()),
            ))
        {
            diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                format!("hygiene-aether-{kind}-pin"),
                Some(&policy.profile_path),
                "Hygiene artifact digest matching the supported Aether projection",
                "missing or mismatched artifact pin",
                format!("the Hygiene profile does not pin the supported Aether {kind} digest"),
                DiagnosticImpact::Invalid,
            ));
        }
        if !expected
            .path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            diagnostics.push(RawDiagnostic::new(
                CONTRACT_RULE,
                format!("catalog-aether-{kind}-path"),
                Some(path),
                "normalized upstream artifact path",
                "unsafe path",
                "the bundled continuity artifact path is unsafe",
                DiagnosticImpact::Invalid,
            ));
        }
    }
}

fn projection_bytes<'a>(
    snapshot: &'a RepositorySnapshot,
    path: &Path,
    label: &str,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) -> Option<&'a [u8]> {
    stats.files += 1;
    let Some(entry) = snapshot.get(path) else {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            format!("projection-missing-{}", portable_path(path)),
            Some(path),
            format!("regular local {label}"),
            "missing",
            format!("{label} is unavailable; runtime fetching is intentionally disabled"),
            DiagnosticImpact::Incomplete,
        ));
        return None;
    };
    if entry.kind != RepositoryEntryKind::File {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            format!("projection-kind-{}", portable_path(path)),
            Some(path),
            format!("regular local {label}"),
            "symbolic link",
            format!("{label} must be a reviewed regular-file projection"),
            DiagnosticImpact::Invalid,
        ));
        return None;
    }
    entry.content.as_deref()
}

fn expected_managed_block(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    diagnostics: &mut Vec<RawDiagnostic>,
) -> Option<String> {
    let bytes = snapshot.get(&policy.instruction_path)?.content.as_deref()?;
    let Ok(contents) = std::str::from_utf8(bytes) else {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            "instruction-utf8",
            Some(&policy.instruction_path),
            "UTF-8 Aether instruction projection",
            "non-UTF-8 content",
            "the pinned Aether instruction cannot be decoded",
            DiagnosticImpact::Invalid,
        ));
        return None;
    };
    let begin = "<!-- BEGIN AETHER REPOSITORY-CONTINUITY -->";
    let end = "<!-- END AETHER REPOSITORY-CONTINUITY -->";
    let Some(start) = contents.find(begin) else {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            "instruction-begin",
            Some(&policy.instruction_path),
            "managed continuity begin marker",
            "missing",
            "the pinned Aether instruction lacks its managed block",
            DiagnosticImpact::Invalid,
        ));
        return None;
    };
    let Some(relative_end) = contents[start..].find(end) else {
        diagnostics.push(RawDiagnostic::new(
            CONTRACT_RULE,
            "instruction-end",
            Some(&policy.instruction_path),
            "managed continuity end marker",
            "missing",
            "the pinned Aether instruction has an unterminated managed block",
            DiagnosticImpact::Invalid,
        ));
        return None;
    };
    let stop = start + relative_end + end.len();
    Some(contents[start..stop].trim_end().to_owned())
}

fn evaluate_agents(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    expected_block: Option<&str>,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) {
    let Some(contents) = required_utf8_file(snapshot, &policy.agents_path, diagnostics, stats)
    else {
        return;
    };
    validate_managed_block(
        &policy.agents_path,
        contents,
        expected_block,
        true,
        diagnostics,
    );
    for path in &policy.provider_projections {
        let Some(entry) = snapshot.get(path) else {
            continue;
        };
        stats.provider_projections += 1;
        if entry.kind != RepositoryEntryKind::File {
            diagnostics.push(RawDiagnostic::new(
                AGENTS_RULE,
                format!("projection-kind-{}", portable_path(path)),
                Some(path),
                "regular provider projection",
                "symbolic link",
                "provider instruction projections containing continuity wiring must be regular files",
                DiagnosticImpact::Invalid,
            ));
            continue;
        }
        let Some(bytes) = entry.content.as_deref() else {
            continue;
        };
        let Ok(contents) = std::str::from_utf8(bytes) else {
            diagnostics.push(RawDiagnostic::new(
                AGENTS_RULE,
                format!("projection-utf8-{}", portable_path(path)),
                Some(path),
                "UTF-8 provider projection",
                "non-UTF-8 content",
                "provider instruction projections must be UTF-8",
                DiagnosticImpact::Invalid,
            ));
            continue;
        };
        let lower = contents.to_ascii_lowercase();
        if contents.contains("AETHER REPOSITORY-CONTINUITY")
            || contents.contains("aether.repository-continuity/v1")
            || lower.contains("continuity.md")
            || lower.contains("repository continuity")
            || lower.contains("maintain-repository-continuity")
        {
            validate_managed_block(path, contents, expected_block, true, diagnostics);
        }
    }
}

fn validate_managed_block(
    path: &Path,
    contents: &str,
    expected_block: Option<&str>,
    required: bool,
    diagnostics: &mut Vec<RawDiagnostic>,
) {
    let begin = "<!-- BEGIN AETHER REPOSITORY-CONTINUITY -->";
    let end = "<!-- END AETHER REPOSITORY-CONTINUITY -->";
    let begins = contents.match_indices(begin).collect::<Vec<_>>();
    let ends = contents.match_indices(end).collect::<Vec<_>>();
    if begins.len() != 1 || ends.len() != 1 {
        if required || !begins.is_empty() || !ends.is_empty() {
            diagnostics.push(
                RawDiagnostic::new(
                    AGENTS_RULE,
                    format!("managed-block-count-{}", portable_path(path)),
                    Some(path),
                    "exactly one balanced managed continuity block",
                    format!("{} begin and {} end markers", begins.len(), ends.len()),
                    "the continuity instruction linkage is missing, duplicated, or unbalanced",
                    DiagnosticImpact::Invalid,
                )
                .at_line(
                    begins
                        .first()
                        .map(|(offset, _)| line_number(contents, *offset)),
                ),
            );
        }
        return;
    }
    let start = begins[0].0;
    let stop = ends[0].0 + end.len();
    if stop <= start {
        diagnostics.push(RawDiagnostic::new(
            AGENTS_RULE,
            format!("managed-block-order-{}", portable_path(path)),
            Some(path),
            "begin marker before end marker",
            "reversed markers",
            "the continuity instruction linkage has invalid marker order",
            DiagnosticImpact::Invalid,
        ));
        return;
    }
    if expected_block.is_none_or(|expected| contents[start..stop].trim_end() != expected) {
        diagnostics.push(
            RawDiagnostic::new(
                AGENTS_RULE,
                format!("managed-block-content-{}", portable_path(path)),
                Some(path),
                "the exact pinned Aether continuity instruction block",
                "locally modified or unsupported block",
                "the managed continuity block contradicts the pinned provider instruction",
                DiagnosticImpact::Invalid,
            )
            .at_line(Some(line_number(contents, start))),
        );
    }
}

fn evaluate_continuity_document(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) -> ParsedContinuity {
    let Some(contents) = required_utf8_file(snapshot, &policy.continuity_path, diagnostics, stats)
    else {
        return ParsedContinuity::default();
    };
    evaluate_placeholders_and_safety(policy, snapshot, contents, diagnostics);
    if contents.len() > 16_384 || contents.lines().count() > 240 {
        diagnostics.push(RawDiagnostic::new(
            STRUCTURE_RULE,
            "document-bounds",
            Some(&policy.continuity_path),
            "at most 16384 UTF-8 bytes and 240 lines",
            format!(
                "{} bytes and {} lines",
                contents.len(),
                contents.lines().count()
            ),
            "CONTINUITY.md exceeds the fixed Aether v1 compaction bounds",
            DiagnosticImpact::Invalid,
        ));
    }
    let Some((front_matter, body, body_line)) = split_front_matter(contents) else {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "front-matter-delimiters",
            Some(&policy.continuity_path),
            "one leading YAML front-matter document",
            "missing or malformed delimiters",
            "CONTINUITY.md must begin with a bounded YAML front-matter document",
            DiagnosticImpact::Invalid,
        ));
        evaluate_body(policy, snapshot, contents, 1, diagnostics, stats);
        return ParsedContinuity { metadata: None };
    };
    if front_matter.lines().any(unsafe_yaml_line) {
        diagnostics.push(RawDiagnostic::new(
            SAFETY_RULE,
            "unsafe-yaml-feature",
            Some(&policy.continuity_path),
            "plain YAML without tags, aliases, anchors, or merge keys",
            "unsafe YAML feature",
            "continuity front matter uses a YAML feature outside the deterministic subset",
            DiagnosticImpact::Invalid,
        ));
    }
    let metadata = if let Ok(metadata) = serde_yaml::from_str::<ContinuityMetadata>(front_matter) {
        validate_metadata(policy, snapshot, &metadata, diagnostics);
        Some(metadata)
    } else {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "front-matter-schema",
            Some(&policy.continuity_path),
            "Aether repository-continuity v1 metadata",
            "malformed or unsupported metadata",
            "continuity front matter does not decode against the supported closed v1 structure",
            DiagnosticImpact::Invalid,
        ));
        None
    };
    evaluate_body(policy, snapshot, body, body_line, diagnostics, stats);
    ParsedContinuity { metadata }
}

fn required_utf8_file<'a>(
    snapshot: &'a RepositorySnapshot,
    path: &Path,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) -> Option<&'a str> {
    stats.files += 1;
    let Some(entry) = snapshot.get(path) else {
        let observed = snapshot
            .mis_cased(path)
            .map_or_else(|| "missing".to_owned(), portable_path);
        diagnostics.push(RawDiagnostic::new(
            FILE_RULE,
            format!("required-file-{}", portable_path(path)),
            Some(path),
            format!("exact-case regular file {}", portable_path(path)),
            observed,
            format!(
                "required continuity surface {} is missing or mis-cased",
                portable_path(path)
            ),
            DiagnosticImpact::Invalid,
        ));
        return None;
    };
    if entry.kind != RepositoryEntryKind::File {
        diagnostics.push(RawDiagnostic::new(
            FILE_RULE,
            format!("required-file-{}", portable_path(path)),
            Some(path),
            format!("regular file {}", portable_path(path)),
            "symbolic link",
            format!(
                "required continuity surface {} must not be a symbolic link",
                portable_path(path)
            ),
            DiagnosticImpact::Invalid,
        ));
        return None;
    }
    let Some(bytes) = entry.content.as_deref() else {
        diagnostics.push(RawDiagnostic::new(
            FILE_RULE,
            format!("required-file-content-{}", portable_path(path)),
            Some(path),
            "readable UTF-8 regular file",
            "content unavailable",
            format!(
                "required continuity surface {} could not be inspected",
                portable_path(path)
            ),
            DiagnosticImpact::Incomplete,
        ));
        return None;
    };
    if let Ok(contents) = std::str::from_utf8(bytes) {
        Some(contents)
    } else {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            format!("utf8-{}", portable_path(path)),
            Some(path),
            "UTF-8 content",
            "non-UTF-8 bytes",
            format!("{} must contain UTF-8 text", portable_path(path)),
            DiagnosticImpact::Invalid,
        ));
        None
    }
}

#[allow(clippy::too_many_lines)]
fn validate_metadata(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    metadata: &ContinuityMetadata,
    diagnostics: &mut Vec<RawDiagnostic>,
) {
    if metadata.schema_version != AETHER_CONTRACT {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "schema-identifier",
            Some(&policy.continuity_path),
            AETHER_CONTRACT,
            "unsupported schema identifier",
            "CONTINUITY.md declares an unsupported schema identifier or major version",
            DiagnosticImpact::Invalid,
        ));
    }
    if metadata.repository.id != policy.repository
        || metadata.repository.visibility != policy.visibility.as_str()
        || metadata.repository.continuity_path != "CONTINUITY.md"
        || !valid_ref_name(&metadata.repository.default_branch)
    {
        diagnostics.push(RawDiagnostic::new(
            IDENTITY_RULE,
            "repository-identity",
            Some(&policy.continuity_path),
            format!(
                "{} with {} visibility and exact continuity path",
                policy.repository,
                policy.visibility.as_str()
            ),
            "mismatched or malformed repository identity",
            "continuity repository identity, visibility, default branch, or path disagrees with local policy",
            DiagnosticImpact::Invalid,
        ));
    }

    let document_valid = valid_datetime(&metadata.document.updated_at)
        && metadata.document.max_bytes == 16_384
        && metadata.document.max_lines == 240
        && match metadata.document.status {
            ContinuityDocumentStatus::Active => {
                metadata.document.stale_reason.is_none()
                    && metadata.document.superseded_by.is_none()
            }
            ContinuityDocumentStatus::Stale => {
                metadata
                    .document
                    .stale_reason
                    .as_deref()
                    .is_some_and(valid_nonempty)
                    && metadata.document.superseded_by.is_none()
            }
            ContinuityDocumentStatus::Superseded => {
                metadata.document.stale_reason.is_none()
                    && metadata
                        .document
                        .superseded_by
                        .as_deref()
                        .is_some_and(valid_nonempty)
            }
        };
    if !document_valid {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "document-metadata",
            Some(&policy.continuity_path),
            "valid lifecycle, RFC 3339 update time, and fixed v1 bounds",
            "invalid document metadata",
            "continuity document lifecycle metadata violates the supported Aether v1 contract",
            DiagnosticImpact::Invalid,
        ));
    }

    let expected_precedence = [
        "user-and-runtime-instructions",
        "scoped-repository-instructions",
        "live-repository-and-work-tracker-state",
        "canonical-repository-sources",
        "continuity-checkpoint",
    ];
    let scope_valid = valid_nonempty(&metadata.scope.purpose)
        && !metadata.scope.includes.is_empty()
        && unique_nonempty(&metadata.scope.includes)
        && !metadata.scope.excludes.is_empty()
        && unique_nonempty(&metadata.scope.excludes)
        && metadata
            .scope
            .precedence
            .iter()
            .map(String::as_str)
            .eq(expected_precedence)
        && !metadata.scope.canonical_sources.is_empty()
        && unique_nonempty(&metadata.scope.canonical_sources);
    if !scope_valid {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "scope-metadata",
            Some(&policy.continuity_path),
            "nonempty unique scope data and exact Aether precedence",
            "invalid scope metadata",
            "continuity scope or precedence is incomplete, duplicated, or reordered",
            DiagnosticImpact::Invalid,
        ));
    }
    for (index, source) in metadata.scope.canonical_sources.iter().enumerate() {
        let format_valid = valid_canonical_source(source);
        let resolves = source.starts_with("https://")
            || (format_valid && snapshot.contains_path(Path::new(source)));
        if !format_valid || !resolves {
            diagnostics.push(RawDiagnostic::new(
                LINK_RULE,
                format!("canonical-source-{index}"),
                Some(&policy.continuity_path),
                "normalized repository-relative path or stable HTTPS URL",
                "unsafe, moving, or unresolved source reference",
                "a canonical continuity source is not a stable resolving path or URL",
                DiagnosticImpact::Invalid,
            ));
        }
    }

    let work_valid = valid_nonempty(&metadata.work.objective)
        && !metadata.work.success_conditions.is_empty()
        && unique_nonempty(&metadata.work.success_conditions)
        && matches!(metadata.work.next.kind.as_str(), "issue" | "action")
        && valid_nonempty(&metadata.work.next.id)
        && valid_nonempty(&metadata.work.next.description)
        && matches!(
            metadata.work.next.readiness.as_str(),
            "ready" | "blocked" | "unknown"
        )
        && !metadata.work.next.references.is_empty()
        && metadata
            .work
            .next
            .references
            .iter()
            .all(|value| valid_stable_url(value))
        && unique_nonempty(&metadata.work.next.depends_on);
    if !work_valid {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "work-metadata",
            Some(&policy.continuity_path),
            "one objective, observable success, and stable next-work metadata",
            "invalid work metadata",
            "continuity objective, success conditions, or next-work declaration is malformed",
            DiagnosticImpact::Invalid,
        ));
    }
    if let Some(reference) = &metadata.work.active_issue {
        validate_reference(
            reference,
            ReferenceKind::Issue,
            &policy.continuity_path,
            "active-issue",
            diagnostics,
        );
    }
    for (index, reference) in metadata.state.parallel_changes.iter().enumerate() {
        validate_reference(
            reference,
            ReferenceKind::PullRequest,
            &policy.continuity_path,
            &format!("parallel-change-{index}"),
            diagnostics,
        );
    }
    if let Some(reference) = &metadata.state.candidate.pull_request {
        validate_reference(
            reference,
            ReferenceKind::PullRequest,
            &policy.continuity_path,
            "candidate-pull-request",
            diagnostics,
        );
    }

    let base_valid = (metadata.state.base.revision == "unborn"
        || valid_commit(&metadata.state.base.revision))
        && metadata.state.base.r#ref.starts_with("refs/heads/")
        && valid_ref_name(
            metadata
                .state
                .base
                .r#ref
                .strip_prefix("refs/heads/")
                .unwrap_or_default(),
        )
        && valid_datetime(&metadata.state.base.verified_at);
    let candidate_valid = metadata
        .state
        .candidate
        .branch
        .as_deref()
        .is_none_or(valid_ref_name)
        && metadata
            .state
            .candidate
            .revision
            .as_deref()
            .is_none_or(valid_commit);
    let live_notes = metadata.state.live.notes.to_ascii_lowercase();
    let live_uncertainty_valid = match metadata.state.live.status {
        ContinuityLiveStatus::Verified => ["verified", "checked", "observed", "showed"]
            .iter()
            .any(|word| live_notes.contains(word)),
        ContinuityLiveStatus::Partial | ContinuityLiveStatus::Unavailable => [
            "partial",
            "unavailable",
            "unknown",
            "not verified",
            "could not",
            "recheck",
        ]
        .iter()
        .any(|word| live_notes.contains(word)),
    };
    let live_valid = valid_datetime(&metadata.state.live.observed_at)
        && metadata
            .state
            .live
            .default_branch_revision
            .as_deref()
            .is_none_or(valid_commit)
        && matches!(
            metadata.state.live.issue_state.as_str(),
            "open" | "closed" | "unknown" | "not-applicable"
        )
        && matches!(
            metadata.state.live.pull_request_state.as_str(),
            "draft" | "open" | "merged" | "closed" | "unknown" | "not-applicable"
        )
        && valid_nonempty(&metadata.state.live.notes)
        && live_uncertainty_valid
        && (metadata.state.live.status != ContinuityLiveStatus::Verified
            || metadata.state.live.default_branch_revision.is_some());
    if !base_valid || !candidate_valid || !live_valid {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "state-metadata",
            Some(&policy.continuity_path),
            "valid base, nullable candidate, and explicitly qualified live state",
            "invalid state metadata",
            "continuity base, candidate, or live-state metadata is malformed",
            DiagnosticImpact::Invalid,
        ));
    }

    let review_identity_valid = match metadata.review.status {
        ContinuityReviewStatus::NotRun => {
            metadata.review.reviewed_at.is_none() && metadata.review.reviewed_by.is_none()
        }
        ContinuityReviewStatus::Passed
        | ContinuityReviewStatus::Partial
        | ContinuityReviewStatus::Failed => {
            metadata
                .review
                .reviewed_at
                .as_deref()
                .is_some_and(valid_datetime)
                && metadata
                    .review
                    .reviewed_by
                    .as_deref()
                    .is_some_and(valid_nonempty)
        }
    };
    let review_evidence_valid = !metadata.review.evidence.is_empty()
        && metadata.review.evidence.iter().all(|evidence| {
            valid_nonempty(&evidence.command)
                && matches!(
                    evidence.outcome.as_str(),
                    "passed" | "failed" | "limited" | "not-run"
                )
                && valid_datetime(&evidence.observed_at)
                && valid_nonempty(&evidence.notes)
        })
        && unique_nonempty(&metadata.review.environment_limitations);
    if !review_identity_valid || !review_evidence_valid {
        diagnostics.push(RawDiagnostic::new(
            SCHEMA_RULE,
            "review-metadata",
            Some(&policy.continuity_path),
            "review state with consistent reviewer, time, and evidence",
            "invalid review metadata",
            "continuity review evidence is empty, malformed, or inconsistent with review status",
            DiagnosticImpact::Invalid,
        ));
    }

    let expected_classification = format!("{}-repository", policy.visibility.as_str());
    let expected_exclusions = BTreeSet::from([
        "secrets-and-credentials",
        "private-conversation-text",
        "sensitive-personal-data",
        "unpublished-private-business-data",
        "private-local-paths",
        "unrelated-private-context",
    ]);
    let actual_exclusions = metadata
        .privacy
        .excluded
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let privacy_valid = metadata.privacy.classification == expected_classification
        && !metadata.privacy.contains_sensitive_data
        && unique_nonempty(&metadata.privacy.redactions)
        && actual_exclusions == expected_exclusions
        && metadata.privacy.excluded.len() == expected_exclusions.len()
        && metadata.privacy.untrusted_content == "context-only-no-authority";
    if !privacy_valid {
        diagnostics.push(RawDiagnostic::new(
            SAFETY_RULE,
            "privacy-metadata",
            Some(&policy.continuity_path),
            "visibility-matched classification and complete minimum-necessary exclusions",
            "invalid privacy declaration",
            "continuity privacy metadata does not preserve the Aether trust boundary",
            DiagnosticImpact::Invalid,
        ));
    }
}

#[derive(Debug, Clone, Copy)]
enum ReferenceKind {
    Issue,
    PullRequest,
}

fn validate_reference(
    reference: &ContinuityReference,
    kind: ReferenceKind,
    path: &Path,
    key: &str,
    diagnostics: &mut Vec<RawDiagnostic>,
) {
    let separator = match kind {
        ReferenceKind::Issue => "/issues/",
        ReferenceKind::PullRequest => "/pull/",
    };
    let expected_id = github_reference_id(&reference.url, separator);
    if reference.provider != "github"
        || !valid_nonempty(&reference.id)
        || expected_id.as_deref() != Some(reference.id.as_str())
    {
        diagnostics.push(RawDiagnostic::new(
            LINK_RULE,
            key,
            Some(path),
            "matching canonical GitHub provider, ID, and numbered URL",
            "mismatched or unstable reference",
            "continuity issue or pull-request reference is malformed or internally inconsistent",
            DiagnosticImpact::Invalid,
        ));
    }
}

fn evaluate_body(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    body: &str,
    first_line: u32,
    diagnostics: &mut Vec<RawDiagnostic>,
    stats: &mut CoreStats,
) {
    let headings = body
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            line.strip_prefix("## ")
                .map(|heading| (heading.trim(), index))
        })
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let mut heading_valid = true;
    for required in REQUIRED_SECTIONS {
        let matches = headings
            .iter()
            .enumerate()
            .filter(|(_, (heading, _))| *heading == required)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0] < cursor {
            heading_valid = false;
        } else {
            cursor = matches[0] + 1;
        }
    }
    if !heading_valid {
        diagnostics.push(
            RawDiagnostic::new(
                STRUCTURE_RULE,
                "required-sections",
                Some(&policy.continuity_path),
                "all twelve required level-two sections exactly once and in canonical order",
                "missing, duplicated, or reordered sections",
                "CONTINUITY.md does not preserve the required handoff section anatomy",
                DiagnosticImpact::Invalid,
            )
            .at_line(Some(first_line)),
        );
    }
    let titles = body
        .lines()
        .filter(|line| line.starts_with("# ") && !line.starts_with("## "))
        .collect::<Vec<_>>();
    let title_valid = titles.len() == 1 && titles[0].trim() != "# <Repository> continuity";
    if !title_valid {
        diagnostics.push(
            RawDiagnostic::new(
                STRUCTURE_RULE,
                "document-title",
                Some(&policy.continuity_path),
                "repository-specific level-one title",
                "missing or template title",
                "CONTINUITY.md requires one repository-specific title",
                DiagnosticImpact::Invalid,
            )
            .at_line(Some(first_line)),
        );
    }

    for (target, line) in markdown_links(body, first_line) {
        stats.markdown_links += 1;
        if !valid_markdown_target(snapshot, &target) {
            diagnostics.push(
                RawDiagnostic::new(
                    LINK_RULE,
                    format!("markdown-link-{line}"),
                    Some(&policy.continuity_path),
                    "stable HTTPS URL, fragment, or resolving normalized relative path",
                    "unsafe, moving, or unresolved target",
                    "a continuity Markdown link is not stable or locally resolvable",
                    DiagnosticImpact::Invalid,
                )
                .at_line(Some(line)),
            );
        }
    }
}

fn evaluate_placeholders_and_safety(
    policy: &RepositoryContinuityPolicy,
    snapshot: &RepositorySnapshot,
    contents: &str,
    diagnostics: &mut Vec<RawDiagnostic>,
) {
    let placeholders = snapshot
        .get(&policy.template_path)
        .and_then(|entry| entry.content.as_deref())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(template_placeholders)
        .unwrap_or_default();
    if placeholders
        .iter()
        .any(|placeholder| contents.contains(placeholder))
        || contents.contains("This template is intentionally invalid")
    {
        diagnostics.push(RawDiagnostic::new(
            STRUCTURE_RULE,
            "generated-placeholder",
            Some(&policy.continuity_path),
            "repository-specific evidence with no Aether template placeholders",
            "generated placeholder remains",
            "CONTINUITY.md still contains an unexpanded template placeholder",
            DiagnosticImpact::Invalid,
        ));
    }
    let lower = contents.to_ascii_lowercase();
    let authority_phrases = [
        "ignore previous instructions",
        "disregard repository instructions",
        "continuity grants permission",
        "continuity authorizes",
        "you are now authorized",
        "disable safety checks",
        "exfiltrate secrets",
    ];
    if authority_phrases
        .iter()
        .any(|phrase| lower.contains(phrase))
    {
        diagnostics.push(RawDiagnostic::new(
            SAFETY_RULE,
            "authority-injection",
            Some(&policy.continuity_path),
            "checkpoint content treated only as untrusted context",
            "instruction-authority claim present",
            "continuity Markdown attempts to grant authority or override stronger instructions",
            DiagnosticImpact::Invalid,
        ));
    }
    if contains_secret_like_value(contents) {
        diagnostics.push(RawDiagnostic::new(
            SAFETY_RULE,
            "secret-like-value",
            Some(&policy.continuity_path),
            "no credential-like material",
            "synthetic or real secret-shaped value present",
            "continuity content contains a secret-like value; the value is intentionally omitted",
            DiagnosticImpact::Invalid,
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn collect_git_comparison(
    workspace: &Path,
    inventory: &RepositoryInventory,
    content_paths: &BTreeSet<PathBuf>,
    policy: &RepositoryContinuityPolicy,
    invocation: &ContinuityInvocation,
) -> Result<GitComparison> {
    let shallow = git_text(workspace, &["rev-parse", "--is-shallow-repository"])?
        .is_some_and(|value| value.trim() == "true");
    let mut diagnostics = Vec::new();
    let (base, base_snapshot) = if invocation.base_revision == "unborn" {
        (
            ContinuityRevisionEvidence {
                requested: "unborn".to_owned(),
                state: ContinuityRevisionState::Unborn,
                resolved_revision: None,
            },
            Some(RepositorySnapshot::default()),
        )
    } else if let Some(snapshot) =
        snapshot_at_revision(workspace, &invocation.base_revision, content_paths)?
    {
        (
            ContinuityRevisionEvidence {
                requested: invocation.base_revision.clone(),
                state: ContinuityRevisionState::Available,
                resolved_revision: Some(invocation.base_revision.clone()),
            },
            Some(snapshot),
        )
    } else {
        diagnostics.push(
            RawDiagnostic::new(
                COMPARISON_RULE,
                "base-unavailable",
                None,
                "locally available comparison base",
                if shallow {
                    "unavailable in shallow clone"
                } else {
                    "unavailable"
                },
                "the represented base revision is unavailable locally and was not fetched",
                DiagnosticImpact::Incomplete,
            )
            .with_severity(Severity::Error),
        );
        (
            ContinuityRevisionEvidence {
                requested: invocation.base_revision.clone(),
                state: ContinuityRevisionState::Unavailable,
                resolved_revision: None,
            },
            None,
        )
    };
    let (head, head_snapshot) = if invocation.head_revision == "working-tree" {
        (
            ContinuityRevisionEvidence {
                requested: "working-tree".to_owned(),
                state: ContinuityRevisionState::WorkingTree,
                resolved_revision: git_text(workspace, &["rev-parse", "--verify", "HEAD"])?
                    .map(|value| value.trim().to_owned())
                    .filter(|value| valid_commit(value)),
            },
            Some(RepositorySnapshot::from_inventory(inventory)),
        )
    } else {
        match snapshot_at_revision(workspace, &invocation.head_revision, content_paths)? {
            Some(snapshot) => (
                ContinuityRevisionEvidence {
                    requested: invocation.head_revision.clone(),
                    state: ContinuityRevisionState::Available,
                    resolved_revision: Some(invocation.head_revision.clone()),
                },
                Some(snapshot),
            ),
            None => (
                ContinuityRevisionEvidence {
                    requested: invocation.head_revision.clone(),
                    state: ContinuityRevisionState::Unavailable,
                    resolved_revision: None,
                },
                None,
            ),
        }
    };
    let topology = determine_topology(workspace, invocation, &base, &head)?;
    if topology == ContinuityTopology::Diverged {
        diagnostics.push(
            RawDiagnostic::new(
                COMPARISON_RULE,
                "diverged-base-head",
                None,
                "comparison base that is an ancestor of the candidate head",
                "diverged or rebased history",
                "base/head topology diverges; the caller must select the actual represented baseline",
                DiagnosticImpact::Incomplete,
            )
            .with_severity(Severity::Error),
        );
    }
    let continuity_changed = snapshots_differ(
        base_snapshot.as_ref(),
        head_snapshot.as_ref(),
        &policy.continuity_path,
    );
    let agents_changed = snapshots_differ(
        base_snapshot.as_ref(),
        head_snapshot.as_ref(),
        &policy.agents_path,
    );
    let mut parallel_conflicts = 0_u64;
    let mut parallel_checked = 0_u64;
    for parallel in &invocation.parallel_heads {
        match snapshot_at_revision(workspace, parallel, content_paths)? {
            Some(snapshot) => {
                parallel_checked += 1;
                if base.state == ContinuityRevisionState::Available
                    && git_bytes(
                        workspace,
                        &[
                            "merge-base",
                            "--is-ancestor",
                            &invocation.base_revision,
                            parallel,
                        ],
                    )?
                    .is_none()
                {
                    diagnostics.push(
                        RawDiagnostic::new(
                            PARALLEL_RULE,
                            format!("parallel-head-diverged-{parallel}"),
                            Some(&policy.continuity_path),
                            "parallel head descended from the represented base",
                            "parallel history diverged from the represented base",
                            "a parallel candidate cannot be compared as same-baseline work",
                            DiagnosticImpact::Incomplete,
                        )
                        .with_severity(Severity::Error),
                    );
                    continue;
                }
                let parallel_changed = snapshots_differ(
                    base_snapshot.as_ref(),
                    Some(&snapshot),
                    &policy.continuity_path,
                );
                if continuity_changed == Some(true) && parallel_changed == Some(true) {
                    parallel_conflicts += 1;
                    diagnostics.push(
                        RawDiagnostic::new(
                            PARALLEL_RULE,
                            format!("parallel-checkpoint-conflict-{parallel}"),
                            Some(&policy.continuity_path),
                            "one reconciled current checkpoint",
                            "multiple heads modify CONTINUITY.md from the same baseline",
                            "a parallel candidate changes the same checkpoint and requires explicit reconciliation",
                            DiagnosticImpact::Incomplete,
                        )
                        .with_severity(Severity::Error),
                    );
                }
            }
            None => diagnostics.push(
                RawDiagnostic::new(
                    PARALLEL_RULE,
                    format!("parallel-head-unavailable-{parallel}"),
                    Some(&policy.continuity_path),
                    "locally available parallel head",
                    "unavailable",
                    "a declared parallel head is unavailable locally; no reconciliation result was inferred",
                    DiagnosticImpact::Incomplete,
                )
                .with_severity(Severity::Warning),
            ),
        }
    }
    Ok(GitComparison {
        base,
        head,
        base_snapshot,
        head_snapshot,
        topology,
        shallow,
        continuity_changed,
        agents_changed,
        parallel_conflicts,
        parallel_checked,
        diagnostics,
    })
}

fn snapshot_at_revision(
    workspace: &Path,
    revision: &str,
    content_paths: &BTreeSet<PathBuf>,
) -> Result<Option<RepositorySnapshot>> {
    let commit_expression = format!("{revision}^{{commit}}");
    if git_bytes(workspace, &["cat-file", "-e", &commit_expression])?.is_none() {
        return Ok(None);
    }
    let Some(tree) = git_bytes(workspace, &["ls-tree", "-r", "-z", "--full-tree", revision])?
    else {
        return Ok(None);
    };
    let mut entries = BTreeMap::new();
    for record in tree
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(EgolintError::Configuration(
                "Git tree record is missing its path separator".to_owned(),
            ));
        };
        let header = std::str::from_utf8(&record[..tab]).map_err(|_| {
            EgolintError::Configuration("Git tree metadata must contain UTF-8".to_owned())
        })?;
        let path = std::str::from_utf8(&record[tab + 1..]).map_err(|_| {
            EgolintError::Configuration("Git tree paths must contain UTF-8".to_owned())
        })?;
        let mut fields = header.split_ascii_whitespace();
        let mode = fields.next().unwrap_or_default();
        let kind = fields.next().unwrap_or_default();
        let object = fields.next().unwrap_or_default();
        if kind != "blob" || !valid_commit(object) {
            continue;
        }
        let path = PathBuf::from(path);
        if validate_relative_path(&path, "Git tree path").is_err() {
            return Err(EgolintError::Configuration(
                "Git tree contains an unsafe path".to_owned(),
            ));
        }
        let content = if content_paths.contains(&path) {
            git_bytes(workspace, &["cat-file", "blob", object])?
        } else {
            None
        };
        entries.insert(
            path,
            SnapshotEntry {
                kind: if mode == "120000" {
                    RepositoryEntryKind::Symlink
                } else {
                    RepositoryEntryKind::File
                },
                content,
            },
        );
    }
    Ok(Some(RepositorySnapshot { entries }))
}

fn determine_topology(
    workspace: &Path,
    invocation: &ContinuityInvocation,
    base: &ContinuityRevisionEvidence,
    head: &ContinuityRevisionEvidence,
) -> Result<ContinuityTopology> {
    if base.state == ContinuityRevisionState::Unborn {
        return Ok(ContinuityTopology::Unborn);
    }
    if base.state != ContinuityRevisionState::Available {
        return Ok(ContinuityTopology::Unavailable);
    }
    let head_revision = if head.state == ContinuityRevisionState::WorkingTree {
        let Some(revision) = head.resolved_revision.as_deref() else {
            return Ok(ContinuityTopology::Unavailable);
        };
        revision
    } else if head.state == ContinuityRevisionState::Available {
        invocation.head_revision.as_str()
    } else {
        return Ok(ContinuityTopology::Unavailable);
    };
    if invocation.base_revision == head_revision {
        if head.state == ContinuityRevisionState::WorkingTree {
            return Ok(ContinuityTopology::WorkingTree);
        }
        return Ok(ContinuityTopology::SameRevision);
    }
    if git_bytes(
        workspace,
        &[
            "merge-base",
            "--is-ancestor",
            &invocation.base_revision,
            head_revision,
        ],
    )?
    .is_some()
    {
        let parents = git_text(
            workspace,
            &["rev-list", "--parents", "--max-count=1", head_revision],
        )?
        .map_or(0, |value| value.split_ascii_whitespace().count());
        return Ok(if head.state == ContinuityRevisionState::WorkingTree {
            ContinuityTopology::WorkingTree
        } else if parents > 2 {
            ContinuityTopology::MergeCommit
        } else {
            ContinuityTopology::Linear
        });
    }
    Ok(ContinuityTopology::Diverged)
}

fn snapshots_differ(
    base: Option<&RepositorySnapshot>,
    head: Option<&RepositorySnapshot>,
    path: &Path,
) -> Option<bool> {
    let (Some(base), Some(head)) = (base, head) else {
        return None;
    };
    let base = base.get(path);
    let head = head.get(path);
    Some(match (base, head) {
        (None, None) => false,
        (Some(_), None) | (None, Some(_)) => true,
        (Some(left), Some(right)) => {
            left.kind != right.kind || left.content.as_deref() != right.content.as_deref()
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExceptionState {
    None,
    Applied,
    Invalid,
}

fn evaluate_exceptions(
    policy: &RepositoryContinuityPolicy,
    invocation: &ContinuityInvocation,
    diagnostics: &mut Vec<RawDiagnostic>,
) -> ExceptionState {
    let mut selected = None;
    let mut invalid = false;
    for (index, exception) in policy.exceptions.iter().enumerate() {
        let basic_valid = valid_repository(&exception.repository)
            && exception.repository == policy.repository
            && valid_nonempty(&exception.owner)
            && valid_nonempty(&exception.reason)
            && valid_date(&exception.expires_on)
            && valid_nonempty(&exception.review_trigger)
            && matches!(
                exception.validation_state.as_str(),
                "proposed" | "approved" | "expired" | "revoked"
            )
            && valid_nonempty(&exception.exit_criteria)
            && exception.approval.as_deref().is_none_or(valid_stable_url);
        let approved_valid = exception.validation_state != "approved"
            || exception.approval.as_deref().is_some_and(valid_stable_url);
        let time_valid = exception.expires_on.as_str() >= invocation.evaluation_date.as_str();
        if !basic_valid || !approved_valid || !time_valid {
            invalid = true;
            diagnostics.push(RawDiagnostic::new(
                EXCEPTION_RULE,
                format!("invalid-exception-{index}"),
                Some(&policy.profile_path),
                "owned, approved when active, unexpired, and reviewable exception",
                "invalid, expired, or mismatched exception",
                "a continuity exception does not satisfy the Hygiene exception lifecycle",
                DiagnosticImpact::Invalid,
            ));
        }
        if exception.repository == policy.repository
            && exception.validation_state == "approved"
            && basic_valid
            && approved_valid
            && time_valid
        {
            selected = Some(exception);
        }
    }
    if invocation.disposition == ContinuityDisposition::Exception {
        if selected.is_some() {
            ExceptionState::Applied
        } else {
            diagnostics.push(RawDiagnostic::new(
                EXCEPTION_RULE,
                "missing-approved-exception",
                Some(&policy.profile_path),
                "one approved unexpired exception for the represented repository",
                "no applicable exception",
                "the comparison declares an exception without reviewed, unexpired policy evidence",
                DiagnosticImpact::Invalid,
            ));
            ExceptionState::Invalid
        }
    } else if invalid {
        ExceptionState::Invalid
    } else {
        ExceptionState::None
    }
}

#[allow(clippy::too_many_lines)]
fn evaluate_comparison(
    workspace: &Path,
    policy: &RepositoryContinuityPolicy,
    invocation: &ContinuityInvocation,
    git: &GitComparison,
    metadata: Option<&ContinuityMetadata>,
    exception_state: ExceptionState,
    diagnostics: &mut Vec<RawDiagnostic>,
) {
    let Some(metadata) = metadata else {
        return;
    };
    if invocation.disposition == ContinuityDisposition::Updated
        && metadata.state.base.revision != invocation.base_revision
    {
        diagnostics.push(RawDiagnostic::new(
            FRESHNESS_RULE,
            "represented-base",
            Some(&policy.continuity_path),
            "front-matter base matching the explicit comparison base",
            "stale or mismatched base metadata",
            "CONTINUITY.md does not represent the base revision supplied by the invoking workflow",
            DiagnosticImpact::Invalid,
        ));
    }
    if invocation.disposition != ContinuityDisposition::Updated
        && metadata.state.base.revision != "unborn"
        && git_bytes(
            workspace,
            &[
                "cat-file",
                "-e",
                &format!("{}^{{commit}}", metadata.state.base.revision),
            ],
        )
        .ok()
        .flatten()
        .is_none()
    {
        diagnostics.push(
            RawDiagnostic::new(
                FRESHNESS_RULE,
                "prior-base-unavailable",
                Some(&policy.continuity_path),
                "locally available prior checkpoint base for a no-change or exception result",
                "prior base unavailable",
                "the unchanged checkpoint's represented base cannot be verified locally",
                DiagnosticImpact::Incomplete,
            )
            .with_severity(Severity::Warning),
        );
    }
    if metadata.document.status == ContinuityDocumentStatus::Stale {
        diagnostics.push(
            RawDiagnostic::new(
                FRESHNESS_RULE,
                "document-stale",
                Some(&policy.continuity_path),
                "active checkpoint or policy-valid exception",
                "document marked stale",
                "the checkpoint explicitly reports stale state and cannot establish a fresh handoff",
                DiagnosticImpact::Incomplete,
            )
            .with_severity(Severity::Warning),
        );
    }
    match invocation.disposition {
        ContinuityDisposition::Updated => {
            if git.continuity_changed != Some(true) {
                diagnostics.push(RawDiagnostic::new(
                    COMPARISON_RULE,
                    "declared-update",
                    Some(&policy.continuity_path),
                    "CONTINUITY.md content changed between the represented base and head",
                    comparison_state(git.continuity_changed),
                    "the comparison declares an update but local Git evidence does not show one",
                    if git.continuity_changed.is_none() {
                        DiagnosticImpact::Incomplete
                    } else {
                        DiagnosticImpact::Invalid
                    },
                ));
            }
        }
        ContinuityDisposition::ReviewedNoChange => {
            let reviewed = matches!(
                metadata.review.status,
                ContinuityReviewStatus::Passed | ContinuityReviewStatus::Partial
            ) && metadata
                .review
                .evidence
                .iter()
                .any(|item| matches!(item.outcome.as_str(), "passed" | "limited"));
            if git.continuity_changed != Some(false) || !reviewed {
                diagnostics.push(RawDiagnostic::new(
                    COMPARISON_RULE,
                    "reviewed-no-change",
                    Some(&policy.continuity_path),
                    "unchanged checkpoint with explicit passed or limited review evidence",
                    if reviewed {
                        comparison_state(git.continuity_changed)
                    } else {
                        "review evidence unavailable".to_owned()
                    },
                    "a no-change disposition requires unchanged content and explicit review evidence",
                    if git.continuity_changed.is_none() {
                        DiagnosticImpact::Incomplete
                    } else {
                        DiagnosticImpact::Invalid
                    },
                ));
            }
        }
        ContinuityDisposition::Exception => {
            if exception_state != ExceptionState::Applied {
                diagnostics.push(RawDiagnostic::new(
                    COMPARISON_RULE,
                    "exception-disposition",
                    Some(&policy.continuity_path),
                    "applied reviewed exception",
                    "exception unavailable",
                    "the comparison cannot use an exception disposition without valid policy evidence",
                    DiagnosticImpact::Invalid,
                ));
            }
        }
    }

    match invocation.transition {
        ContinuityTransition::PullRequest => {
            if metadata.state.candidate.handoff_state
                == ContinuityHandoffState::PostMergeReconciliation
            {
                diagnostics.push(RawDiagnostic::new(
                    FRESHNESS_RULE,
                    "candidate-transition",
                    Some(&policy.continuity_path),
                    "candidate-safe pre-merge handoff state",
                    "post-merge reconciliation",
                    "a pull-request candidate declares a post-merge handoff state",
                    DiagnosticImpact::Invalid,
                ));
            }
        }
        ContinuityTransition::PostMerge => {
            if !matches!(
                metadata.state.candidate.handoff_state,
                ContinuityHandoffState::PostMergeReconciliation
                    | ContinuityHandoffState::NoActiveChange
            ) || matches!(
                metadata.state.live.pull_request_state.as_str(),
                "open" | "draft"
            ) {
                diagnostics.push(RawDiagnostic::new(
                    FRESHNESS_RULE,
                    "post-merge-transition",
                    Some(&policy.continuity_path),
                    "post-merge-reconciliation or no-active-change state without an open PR claim",
                    "stale candidate or open-PR state",
                    "the post-merge checkpoint still represents a pre-merge candidate",
                    DiagnosticImpact::Invalid,
                ));
            }
        }
    }

    if let Some(candidate) = &metadata.state.candidate.revision {
        let available = git_bytes(
            workspace,
            &["cat-file", "-e", &format!("{candidate}^{{commit}}")],
        )
        .ok()
        .flatten()
        .is_some();
        if !available {
            diagnostics.push(
                RawDiagnostic::new(
                    FRESHNESS_RULE,
                    "candidate-revision-unavailable",
                    Some(&policy.continuity_path),
                    "nullable or locally available candidate revision",
                    "unavailable candidate revision",
                    "the optional candidate revision cannot be verified from local Git evidence",
                    DiagnosticImpact::Incomplete,
                )
                .with_severity(Severity::Warning),
            );
        }
    }
    if let Some(default_branch) = &metadata.state.live.default_branch_revision {
        let available = git_bytes(
            workspace,
            &["cat-file", "-e", &format!("{default_branch}^{{commit}}")],
        )
        .ok()
        .flatten()
        .is_some();
        if !available {
            diagnostics.push(
                RawDiagnostic::new(
                    LIVE_RULE,
                    "default-branch-revision-unavailable",
                    Some(&policy.continuity_path),
                    "locally available or externally verified default-branch revision",
                    "unavailable locally",
                    "the recorded default-branch revision requires external verification",
                    DiagnosticImpact::Incomplete,
                )
                .with_severity(Severity::Warning),
            );
        }
    }
    if invocation.live_verification == ContinuityLiveVerification::Unavailable
        && (metadata.state.live.status == ContinuityLiveStatus::Verified
            || metadata.state.live.pull_request_state == "merged")
    {
        diagnostics.push(
            RawDiagnostic::new(
                LIVE_RULE,
                "external-live-verification",
                Some(&policy.continuity_path),
                "separate authorized live-state evidence",
                "not supplied to the offline validator",
                "mutable live or merge-state claims require verification by an external adapter",
                DiagnosticImpact::Incomplete,
            )
            .with_severity(Severity::Warning),
        );
    }
    if git.parallel_conflicts > 0 && metadata.state.parallel_changes.is_empty() {
        diagnostics.push(RawDiagnostic::new(
            PARALLEL_RULE,
            "parallel-reconciliation-evidence",
            Some(&policy.continuity_path),
            "recorded parallel change and manual reconciliation state",
            "parallel checkpoint change omitted",
            "local Git found a parallel checkpoint edit that the handoff does not acknowledge",
            DiagnosticImpact::Invalid,
        ));
    }
}

fn resolve_applicability(policy: &RepositoryContinuityPolicy) -> ContinuityRequirement {
    match policy.repository_kind {
        ContinuityRepositoryKind::Mirror => return ContinuityRequirement::NotApplicable,
        ContinuityRepositoryKind::GeneratedOnly => return ContinuityRequirement::Advisory,
        ContinuityRepositoryKind::Standard | ContinuityRepositoryKind::Template => {}
    }
    match policy.lifecycle {
        ContinuityLifecycle::Archived => ContinuityRequirement::NotApplicable,
        ContinuityLifecycle::Dormant => ContinuityRequirement::Advisory,
        ContinuityLifecycle::Active => ContinuityRequirement::Required,
    }
}

fn effective_severity(
    severity: Severity,
    rollout: ContinuityRolloutStage,
    applicability: ContinuityRequirement,
    existed_at_base: bool,
) -> Severity {
    if applicability == ContinuityRequirement::Advisory
        || rollout == ContinuityRolloutStage::Observe
        || (rollout == ContinuityRolloutStage::Ratchet && existed_at_base)
    {
        match severity {
            Severity::Error | Severity::Critical => Severity::Warning,
            other => other,
        }
    } else {
        severity
    }
}

fn normalize_diagnostic(
    raw: &RawDiagnostic,
    rule: &CatalogRule,
    severity: Severity,
) -> ContinuityDiagnostic {
    let location = raw.path.as_ref().map(|path| SourceLocation {
        path: path.clone(),
        start_line: raw.line,
        start_column: None,
        end_line: None,
        end_column: None,
    });
    let fingerprint = stable_fingerprint(raw.rule_id, location.as_ref(), &raw.key);
    ContinuityDiagnostic {
        id: format!("{}-{fingerprint}", raw.rule_id),
        rule_id: raw.rule_id.to_owned(),
        severity,
        location,
        expected_state: raw.expected.clone(),
        actual_state: raw.actual.clone(),
        message: raw.message.clone(),
        remediation: rule.remediation.clone(),
        contracts: rule.contracts.clone(),
    }
}

fn validation_status(
    applicability: ContinuityRequirement,
    diagnostics: &[RawDiagnostic],
) -> ContinuityValidationStatus {
    if applicability == ContinuityRequirement::NotApplicable {
        return ContinuityValidationStatus::NotApplicable;
    }
    if diagnostics
        .iter()
        .any(|item| item.impact == DiagnosticImpact::Invalid)
    {
        ContinuityValidationStatus::Invalid
    } else if diagnostics
        .iter()
        .any(|item| item.impact == DiagnosticImpact::Incomplete)
    {
        ContinuityValidationStatus::Incomplete
    } else {
        ContinuityValidationStatus::Valid
    }
}

fn evidence_layers(
    applicability: ContinuityRequirement,
    diagnostics: &[RawDiagnostic],
    invocation: &ContinuityInvocation,
    metadata: Option<&ContinuityMetadata>,
) -> ContinuityEvidenceLayers {
    if applicability == ContinuityRequirement::NotApplicable {
        return ContinuityEvidenceLayers {
            structural: ContinuityEvidenceStatus::NotApplicable,
            freshness_declaration: ContinuityEvidenceStatus::NotApplicable,
            local_git: ContinuityEvidenceStatus::NotApplicable,
            external_live_state: ContinuityEvidenceStatus::NotApplicable,
        };
    }
    let structural_rules = [
        FILE_RULE,
        CONTRACT_RULE,
        SCHEMA_RULE,
        STRUCTURE_RULE,
        IDENTITY_RULE,
        LINK_RULE,
        AGENTS_RULE,
        SAFETY_RULE,
    ];
    let freshness_rules = [
        FRESHNESS_RULE,
        COMPARISON_RULE,
        PARALLEL_RULE,
        EXCEPTION_RULE,
    ];
    let structural = layer_status(diagnostics, &structural_rules);
    let freshness_declaration = layer_status(diagnostics, &freshness_rules);
    let local_git = if diagnostics.iter().any(|item| {
        matches!(item.rule_id, COMPARISON_RULE | PARALLEL_RULE)
            && item.impact == DiagnosticImpact::Incomplete
    }) {
        ContinuityEvidenceStatus::Incomplete
    } else if diagnostics.iter().any(|item| {
        matches!(item.rule_id, COMPARISON_RULE | PARALLEL_RULE)
            && item.impact == DiagnosticImpact::Invalid
    }) {
        ContinuityEvidenceStatus::Invalid
    } else {
        ContinuityEvidenceStatus::Valid
    };
    let external_live_state =
        if invocation.live_verification == ContinuityLiveVerification::Verified {
            ContinuityEvidenceStatus::Verified
        } else if metadata.is_some_and(|metadata| {
            metadata.state.live.status == ContinuityLiveStatus::Verified
                || metadata.state.live.pull_request_state == "merged"
        }) {
            ContinuityEvidenceStatus::RequiresExternalVerification
        } else {
            ContinuityEvidenceStatus::Unavailable
        };
    ContinuityEvidenceLayers {
        structural,
        freshness_declaration,
        local_git,
        external_live_state,
    }
}

fn layer_status(diagnostics: &[RawDiagnostic], rules: &[&str]) -> ContinuityEvidenceStatus {
    if diagnostics
        .iter()
        .any(|item| rules.contains(&item.rule_id) && item.impact == DiagnosticImpact::Invalid)
    {
        ContinuityEvidenceStatus::Invalid
    } else if diagnostics
        .iter()
        .any(|item| rules.contains(&item.rule_id) && item.impact == DiagnosticImpact::Incomplete)
    {
        ContinuityEvidenceStatus::Incomplete
    } else {
        ContinuityEvidenceStatus::Valid
    }
}

fn comparison_state(value: Option<bool>) -> String {
    match value {
        Some(true) => "changed".to_owned(),
        Some(false) => "unchanged".to_owned(),
        None => "unavailable".to_owned(),
    }
}

fn git_bytes(workspace: &Path, arguments: &[&str]) -> Result<Option<Vec<u8>>> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(workspace)
        .output()
        .map_err(|source| EgolintError::Filesystem {
            path: workspace.to_path_buf(),
            source,
        })?;
    Ok(output.status.success().then_some(output.stdout))
}

fn git_text(workspace: &Path, arguments: &[&str]) -> Result<Option<String>> {
    let Some(bytes) = git_bytes(workspace, arguments)? else {
        return Ok(None);
    };
    String::from_utf8(bytes).map(Some).map_err(|_| {
        EgolintError::Configuration("Git command output must contain valid UTF-8".to_owned())
    })
}

fn raw_diagnostic_order(left: &RawDiagnostic, right: &RawDiagnostic) -> std::cmp::Ordering {
    left.path
        .as_deref()
        .unwrap_or_else(|| Path::new(""))
        .cmp(right.path.as_deref().unwrap_or_else(|| Path::new("")))
        .then_with(|| left.rule_id.cmp(right.rule_id))
        .then_with(|| left.key.cmp(&right.key))
}

fn stable_fingerprint(rule_id: &str, location: Option<&SourceLocation>, key: &str) -> String {
    let path = location
        .map(|location| portable_path(&location.path))
        .unwrap_or_default();
    let line = location
        .and_then(|location| location.start_line)
        .unwrap_or_default();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in rule_id
        .bytes()
        .chain([0])
        .chain(path.bytes())
        .chain([0])
        .chain(line.to_string().bytes())
        .chain([0])
        .chain(key.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("repository-continuity-v1-{hash:016x}")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn version_in_range(selected: &str, minimum: &str, maximum_exclusive: &str) -> bool {
    let (Some(selected), Some(minimum), Some(maximum)) = (
        semantic_version(selected),
        semantic_version(minimum),
        semantic_version(maximum_exclusive),
    ) else {
        return false;
    };
    selected >= minimum && selected < maximum
}

fn semantic_version(value: &str) -> Option<(u32, u32, u32)> {
    let mut fields = value.split('.');
    let major = fields.next()?.parse().ok()?;
    let minor = fields.next()?.parse().ok()?;
    let patch = fields.next()?.parse().ok()?;
    fields.next().is_none().then_some((major, minor, patch))
}

fn valid_repository(value: &str) -> bool {
    let mut fields = value.split('/');
    let owner = fields.next().unwrap_or_default();
    let repository = fields.next().unwrap_or_default();
    !owner.is_empty()
        && !repository.is_empty()
        && fields.next().is_none()
        && owner.bytes().all(valid_repository_byte)
        && repository.bytes().all(valid_repository_byte)
}

const fn valid_repository_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_ref_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('.')
        && !value.starts_with('/')
        && !value.ends_with('.')
        && !value.ends_with('/')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains('\\')
        && !value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
}

fn valid_nonempty(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn unique_nonempty(values: &[String]) -> bool {
    values.iter().all(|value| valid_nonempty(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return false;
    }
    let year = value[0..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    year > 0 && day > 0 && day <= maximum
}

fn valid_datetime(value: &str) -> bool {
    let Some((date, remainder)) = value.split_once('T') else {
        return false;
    };
    if !valid_date(date) {
        return false;
    }
    let (clock, zone) = if let Some(clock) = remainder.strip_suffix('Z') {
        (clock, "Z")
    } else {
        let Some(index) = remainder
            .char_indices()
            .rfind(|&(index, character)| index > 0 && matches!(character, '+' | '-'))
            .map(|(index, _)| index)
        else {
            return false;
        };
        (&remainder[..index], &remainder[index..])
    };
    let main_clock = clock.split_once('.').map_or(clock, |(main, fraction)| {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            ""
        } else {
            main
        }
    });
    let mut fields = main_clock.split(':');
    let hour = fields.next().and_then(|part| part.parse::<u32>().ok());
    let minute = fields.next().and_then(|part| part.parse::<u32>().ok());
    let second = fields.next().and_then(|part| part.parse::<u32>().ok());
    if fields.next().is_some()
        || !matches!(hour, Some(0..=23))
        || !matches!(minute, Some(0..=59))
        || !matches!(second, Some(0..=60))
    {
        return false;
    }
    if zone == "Z" {
        return true;
    }
    let bytes = zone.as_bytes();
    bytes.len() == 6
        && matches!(bytes[0], b'+' | b'-')
        && bytes[3] == b':'
        && zone[1..3].parse::<u32>().is_ok_and(|hours| hours <= 23)
        && zone[4..6].parse::<u32>().is_ok_and(|minutes| minutes <= 59)
}

fn valid_canonical_source(value: &str) -> bool {
    if value.starts_with("https://") {
        return valid_stable_url(value);
    }
    let path = Path::new(value);
    validate_relative_path(path, "canonical source").is_ok() && !value.contains('\\')
}

fn valid_stable_url(value: &str) -> bool {
    if !value.starts_with("https://")
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let Some((host, path)) = rest.split_once('/') else {
        return rest.contains('.');
    };
    if !host.contains('.') || host.is_empty() {
        return false;
    }
    if host != "github.com" {
        return true;
    }
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 2 || !valid_repository(&format!("{}/{}", segments[0], segments[1])) {
        return false;
    }
    if segments.len() >= 4 && matches!(segments[2], "issues" | "pull") {
        return segments[3].split(['#', '?']).next().is_some_and(|number| {
            !number.is_empty() && number != "0" && number.bytes().all(|byte| byte.is_ascii_digit())
        });
    }
    if segments.len() >= 4 && segments[2] == "commit" {
        return valid_commit(segments[3].split(['#', '?']).next().unwrap_or_default());
    }
    if segments.len() >= 5 && matches!(segments[2], "blob" | "tree") {
        return valid_commit(segments[3]);
    }
    true
}

fn github_reference_id(value: &str, separator: &str) -> Option<String> {
    let rest = value.strip_prefix("https://github.com/")?;
    let (repository, number) = rest.split_once(separator)?;
    if !valid_repository(repository)
        || number.is_empty()
        || number == "0"
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{repository}#{number}"))
}

fn validate_relative_path(path: &Path, name: &str) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.to_str().is_none() {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a nonempty Unicode path relative to the workspace"
        )));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "{name} must be normalized and remain inside the workspace"
        )));
    }
    Ok(())
}

fn ensure_unique_paths(paths: &[PathBuf], name: &str) -> Result<()> {
    if paths.iter().collect::<BTreeSet<_>>().len() != paths.len() {
        return Err(EgolintError::Configuration(format!(
            "{name} paths must be unique"
        )));
    }
    Ok(())
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn split_front_matter(contents: &str) -> Option<(&str, &str, u32)> {
    let rest = contents.strip_prefix("---\n")?;
    let delimiter = "\n---\n";
    let end = rest.find(delimiter)?;
    let front_matter = &rest[..end];
    let body = &rest[end + delimiter.len()..];
    let body_line = u32::try_from(front_matter.lines().count())
        .unwrap_or(u32::MAX)
        .saturating_add(4);
    Some((front_matter, body, body_line))
}

fn unsafe_yaml_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('!')
        || trimmed.starts_with('*')
        || trimmed.starts_with("<<:")
        || trimmed.contains(": &")
        || trimmed.contains(": *")
        || trimmed.contains(": !")
}

fn line_number(contents: &str, offset: usize) -> u32 {
    u32::try_from(
        contents[..offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count(),
    )
    .unwrap_or(u32::MAX)
    .saturating_add(1)
}

fn template_placeholders(contents: &str) -> BTreeSet<String> {
    let mut placeholders = BTreeSet::new();
    for line in contents.lines() {
        let mut cursor = 0;
        while let Some(relative) = line[cursor..].find('<') {
            let start = cursor + relative;
            if line[start..].starts_with("<!--") {
                cursor = start + 4;
                continue;
            }
            let Some(relative_end) = line[start..].find('>') else {
                break;
            };
            let end = start + relative_end + 1;
            let candidate = &line[start..end];
            if candidate.len() <= 160
                && !candidate.starts_with("</")
                && candidate.bytes().any(|byte| byte.is_ascii_alphabetic())
            {
                placeholders.insert(candidate.to_owned());
            }
            cursor = end;
        }
    }
    placeholders
}

fn contains_secret_like_value(contents: &str) -> bool {
    let private_key_suffix = ["PRIVATE", "KEY-----"].join(" ");
    if ["", "RSA ", "OPENSSH "]
        .iter()
        .any(|kind| contents.contains(&format!("-----BEGIN {kind}{private_key_suffix}")))
    {
        return true;
    }
    contents
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        })
        .any(|token| {
            (token.starts_with("ghp_") && token.len() >= 24)
                || (token.starts_with("github_pat_") && token.len() >= 24)
                || (token.starts_with("sk_live_") && token.len() >= 24)
                || (token.starts_with("xoxb-") && token.len() >= 24)
                || (token.starts_with("AKIA")
                    && token.len() == 20
                    && token
                        .bytes()
                        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()))
        })
}

fn markdown_links(contents: &str, first_line: u32) -> Vec<(String, u32)> {
    let mut links = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let mut cursor = 0;
        while let Some(relative) = line[cursor..].find("](") {
            let start = cursor + relative + 2;
            let Some(close) = line[start..].find(')') else {
                break;
            };
            let target = line[start..start + close].trim();
            if !target.is_empty() {
                links.push((
                    target.to_owned(),
                    first_line.saturating_add(u32::try_from(index).unwrap_or(u32::MAX)),
                ));
            }
            cursor = start + close + 1;
        }
    }
    links
}

fn valid_markdown_target(snapshot: &RepositorySnapshot, value: &str) -> bool {
    let value = value.trim_matches(['<', '>']);
    if let Some(fragment) = value.strip_prefix('#') {
        return !fragment.is_empty() && !fragment.contains(char::is_whitespace);
    }
    if value.starts_with("https://") {
        return valid_stable_url(value);
    }
    if value.contains("://")
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(char::is_whitespace)
    {
        return false;
    }
    let path = value.split_once('#').map_or(value, |(path, _)| path);
    if path.is_empty() {
        return true;
    }
    let path = Path::new(path.strip_prefix("./").unwrap_or(path));
    validate_relative_path(path, "Markdown target").is_ok() && snapshot.contains_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = include_str!("../../.config/dogfood/repository-continuity.toml");
    const PUBLIC_CONTINUITY: &str =
        include_str!("../../tests/fixtures/repository-continuity/valid-public.md");
    const PRIVATE_CONTINUITY: &str =
        include_str!("../../tests/fixtures/repository-continuity/valid-private.md");
    const MALFORMED_CONTINUITY: &str =
        include_str!("../../tests/fixtures/repository-continuity/malformed.md");
    const MALICIOUS_FRAGMENT: &str =
        include_str!("../../tests/fixtures/repository-continuity/malicious-authority.md");
    const SYNTHETIC_SECRET_PARTS: &str =
        include_str!("../../tests/fixtures/repository-continuity/synthetic-secret.parts");
    const MANAGED_AGENTS: &str = include_str!("../../AGENTS.md");
    const HYGIENE_PROFILE_BYTES: &[u8] =
        include_bytes!("../../vendor/hygiene/repository-continuity-policy.v1.json");
    const AETHER_SCHEMA_BYTES: &[u8] =
        include_bytes!("../../vendor/aether/aether.repository-continuity.v1.schema.json");
    const AETHER_INSTRUCTION_BYTES: &[u8] =
        include_bytes!("../../vendor/aether/repository-continuity.INSTRUCTION.md");
    const AETHER_TEMPLATE_BYTES: &[u8] =
        include_bytes!("../../vendor/aether/CONTINUITY.template.md");

    fn policy(repository: &str, visibility: ContinuityVisibility) -> RepositoryContinuityPolicy {
        let mut policy = RepositoryContinuityPolicy::from_toml(
            POLICY,
            Path::new(".config/dogfood/repository-continuity.toml"),
        )
        .expect("valid fixture policy");
        policy.repository = repository.to_owned();
        policy.visibility = visibility;
        policy.provider_projections.clear();
        policy
    }

    fn invocation(base_revision: &str, disposition: ContinuityDisposition) -> ContinuityInvocation {
        ContinuityInvocation {
            base_revision: base_revision.to_owned(),
            head_revision: "working-tree".to_owned(),
            parallel_heads: Vec::new(),
            disposition,
            transition: ContinuityTransition::PullRequest,
            live_verification: ContinuityLiveVerification::Unavailable,
            live_evidence: Vec::new(),
            evaluation_date: "2026-09-08".to_owned(),
        }
    }

    fn snapshot_entry(contents: impl Into<Vec<u8>>) -> SnapshotEntry {
        SnapshotEntry {
            kind: RepositoryEntryKind::File,
            content: Some(contents.into()),
        }
    }

    fn fixture_snapshot(continuity: &str) -> RepositorySnapshot {
        RepositorySnapshot {
            entries: BTreeMap::from([
                (
                    PathBuf::from("CONTINUITY.md"),
                    snapshot_entry(continuity.as_bytes().to_vec()),
                ),
                (
                    PathBuf::from("AGENTS.md"),
                    snapshot_entry(MANAGED_AGENTS.as_bytes().to_vec()),
                ),
                (
                    PathBuf::from("vendor/hygiene/repository-continuity-policy.v1.json"),
                    snapshot_entry(HYGIENE_PROFILE_BYTES.to_vec()),
                ),
                (
                    PathBuf::from("vendor/aether/aether.repository-continuity.v1.schema.json"),
                    snapshot_entry(AETHER_SCHEMA_BYTES.to_vec()),
                ),
                (
                    PathBuf::from("vendor/aether/repository-continuity.INSTRUCTION.md"),
                    snapshot_entry(AETHER_INSTRUCTION_BYTES.to_vec()),
                ),
                (
                    PathBuf::from("vendor/aether/CONTINUITY.template.md"),
                    snapshot_entry(AETHER_TEMPLATE_BYTES.to_vec()),
                ),
            ]),
        }
    }

    fn structural_diagnostics(
        policy: &RepositoryContinuityPolicy,
        snapshot: &RepositorySnapshot,
    ) -> Vec<RawDiagnostic> {
        let evaluator = RepositoryContinuityEvaluator::new(
            policy,
            Path::new(".config/dogfood/repository-continuity.toml"),
            invocation(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ContinuityDisposition::Updated,
            ),
        )
        .expect("continuity evaluator");
        let catalog = BundledCatalog::load().expect("bundled catalog");
        evaluator
            .evaluate_snapshot(snapshot, &catalog, ContinuityRequirement::Required)
            .0
    }

    fn rules(diagnostics: &[RawDiagnostic]) -> BTreeSet<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id)
            .collect()
    }

    fn parsed_metadata(contents: &str) -> ContinuityMetadata {
        let (front_matter, _, _) = split_front_matter(contents).expect("fixture front matter");
        serde_yaml::from_str(front_matter).expect("fixture metadata")
    }

    fn revision_evidence(revision: &str) -> ContinuityRevisionEvidence {
        ContinuityRevisionEvidence {
            requested: revision.to_owned(),
            state: ContinuityRevisionState::Available,
            resolved_revision: Some(revision.to_owned()),
        }
    }

    fn comparison(revision: &str, continuity_changed: Option<bool>) -> GitComparison {
        GitComparison {
            base: revision_evidence(revision),
            head: ContinuityRevisionEvidence {
                requested: "working-tree".to_owned(),
                state: ContinuityRevisionState::WorkingTree,
                resolved_revision: Some(revision.to_owned()),
            },
            base_snapshot: None,
            head_snapshot: None,
            topology: ContinuityTopology::WorkingTree,
            shallow: false,
            continuity_changed,
            agents_changed: Some(false),
            parallel_conflicts: 0,
            parallel_checked: 0,
            diagnostics: Vec::new(),
        }
    }

    fn run_git(workspace: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(workspace)
            .output()
            .expect("run fixture Git command");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("Git output is UTF-8")
            .trim()
            .to_owned()
    }

    fn initialize_repository() -> (tempfile::TempDir, PathBuf) {
        let temporary = tempfile::tempdir().expect("temporary repository parent");
        let workspace = temporary.path().join("repository");
        std::fs::create_dir(&workspace).expect("create fixture repository");
        run_git(&workspace, &["init", "--initial-branch=main"]);
        run_git(&workspace, &["config", "user.name", "Egolint Fixture"]);
        run_git(
            &workspace,
            &["config", "user.email", "fixture@example.invalid"],
        );
        (temporary, workspace)
    }

    fn write_file(workspace: &Path, path: &str, contents: &[u8]) {
        let destination = workspace.join(path);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent");
        }
        std::fs::write(destination, contents).expect("write fixture file");
    }

    fn materialize_repository(workspace: &Path, continuity: &str) {
        for (path, contents) in [
            ("CONTINUITY.md", continuity.as_bytes()),
            ("AGENTS.md", MANAGED_AGENTS.as_bytes()),
            (
                "vendor/hygiene/repository-continuity-policy.v1.json",
                HYGIENE_PROFILE_BYTES,
            ),
            (
                "vendor/aether/aether.repository-continuity.v1.schema.json",
                AETHER_SCHEMA_BYTES,
            ),
            (
                "vendor/aether/repository-continuity.INSTRUCTION.md",
                AETHER_INSTRUCTION_BYTES,
            ),
            (
                "vendor/aether/CONTINUITY.template.md",
                AETHER_TEMPLATE_BYTES,
            ),
        ] {
            write_file(workspace, path, contents);
        }
    }

    fn commit_all(workspace: &Path, message: &str) -> String {
        run_git(workspace, &["add", "--all"]);
        run_git(workspace, &["commit", "--message", message]);
        run_git(workspace, &["rev-parse", "HEAD"])
    }

    #[test]
    fn fresh_public_and_private_documents_are_structurally_valid() {
        for (repository, visibility, contents) in [
            (
                "example/public",
                ContinuityVisibility::Public,
                PUBLIC_CONTINUITY,
            ),
            (
                "example/private",
                ContinuityVisibility::Private,
                PRIVATE_CONTINUITY,
            ),
        ] {
            let diagnostics = structural_diagnostics(
                &policy(repository, visibility),
                &fixture_snapshot(contents),
            );
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.rule_id == CONTRACT_RULE
                        && diagnostic.impact == DiagnosticImpact::Advisory
                }),
                "unexpected structural diagnostics for {repository}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn exact_file_and_managed_block_failures_are_precise() {
        let policy = policy("example/public", ContinuityVisibility::Public);

        let mut missing = fixture_snapshot(PUBLIC_CONTINUITY);
        missing.entries.remove(Path::new("CONTINUITY.md"));
        assert!(rules(&structural_diagnostics(&policy, &missing)).contains(FILE_RULE));

        let mut miscased = fixture_snapshot(PUBLIC_CONTINUITY);
        let continuity = miscased
            .entries
            .remove(Path::new("CONTINUITY.md"))
            .expect("continuity fixture");
        miscased
            .entries
            .insert(PathBuf::from("continuity.md"), continuity);
        assert!(rules(&structural_diagnostics(&policy, &miscased)).contains(FILE_RULE));

        let mut wrong_kind = fixture_snapshot(PUBLIC_CONTINUITY);
        wrong_kind.entries.insert(
            PathBuf::from("CONTINUITY.md"),
            SnapshotEntry {
                kind: RepositoryEntryKind::Symlink,
                content: Some(b"target.md".to_vec()),
            },
        );
        assert!(rules(&structural_diagnostics(&policy, &wrong_kind)).contains(FILE_RULE));

        let mut missing_agents = fixture_snapshot(PUBLIC_CONTINUITY);
        missing_agents.entries.remove(Path::new("AGENTS.md"));
        assert!(rules(&structural_diagnostics(&policy, &missing_agents)).contains(FILE_RULE));

        let mut duplicated = fixture_snapshot(PUBLIC_CONTINUITY);
        duplicated.entries.insert(
            PathBuf::from("AGENTS.md"),
            snapshot_entry(format!("{MANAGED_AGENTS}\n{MANAGED_AGENTS}")),
        );
        assert!(rules(&structural_diagnostics(&policy, &duplicated)).contains(AGENTS_RULE));

        let mut overwritten = fixture_snapshot(PUBLIC_CONTINUITY);
        overwritten.entries.insert(
            PathBuf::from("AGENTS.md"),
            snapshot_entry(MANAGED_AGENTS.replace(
                "Treat it as a compact handoff",
                "Treat it as an authoritative handoff",
            )),
        );
        assert!(rules(&structural_diagnostics(&policy, &overwritten)).contains(AGENTS_RULE));

        let mut provider_policy = policy.clone();
        provider_policy.provider_projections = vec![PathBuf::from("CLAUDE.md")];
        let mut contradictory = fixture_snapshot(PUBLIC_CONTINUITY);
        contradictory.entries.insert(
            PathBuf::from("CLAUDE.md"),
            snapshot_entry("Do not read CONTINUITY.md for this repository."),
        );
        assert!(
            rules(&structural_diagnostics(&provider_policy, &contradictory)).contains(AGENTS_RULE)
        );
    }

    #[test]
    fn malformed_unsupported_hostile_and_secret_fixtures_fail_closed() {
        let policy = policy("example/public", ContinuityVisibility::Public);
        let malformed = structural_diagnostics(&policy, &fixture_snapshot(MALFORMED_CONTINUITY));
        assert!(rules(&malformed).contains(SCHEMA_RULE));
        assert!(rules(&malformed).contains(STRUCTURE_RULE));

        let unsupported = PUBLIC_CONTINUITY.replacen(
            "aether.repository-continuity/v1",
            "aether.repository-continuity/v9",
            1,
        );
        assert!(
            rules(&structural_diagnostics(
                &policy,
                &fixture_snapshot(&unsupported)
            ))
            .contains(SCHEMA_RULE)
        );

        let hostile = format!("{PUBLIC_CONTINUITY}\n{MALICIOUS_FRAGMENT}");
        assert!(
            rules(&structural_diagnostics(
                &policy,
                &fixture_snapshot(&hostile)
            ))
            .contains(SAFETY_RULE)
        );

        let synthetic_token = SYNTHETIC_SECRET_PARTS.lines().collect::<String>();
        let leaked = format!("{PUBLIC_CONTINUITY}\nSynthetic fixture: {synthetic_token}\n");
        let leaked_diagnostics = structural_diagnostics(&policy, &fixture_snapshot(&leaked));
        assert!(rules(&leaked_diagnostics).contains(SAFETY_RULE));
        assert!(
            leaked_diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.actual.contains(&synthetic_token))
        );
    }

    #[test]
    fn candidate_postmerge_noop_and_exception_semantics_are_distinct() {
        let workspace = Path::new(".");
        let current = git_text(workspace, &["rev-parse", "--verify", "HEAD"])
            .expect("inspect current Git repository")
            .expect("current revision")
            .trim()
            .to_owned();
        let policy = policy("example/public", ContinuityVisibility::Public);
        let mut metadata = parsed_metadata(PUBLIC_CONTINUITY);
        metadata.state.base.revision.clone_from(&current);

        let mut candidate_diagnostics = Vec::new();
        evaluate_comparison(
            workspace,
            &policy,
            &invocation(&current, ContinuityDisposition::Updated),
            &comparison(&current, Some(true)),
            Some(&metadata),
            ExceptionState::None,
            &mut candidate_diagnostics,
        );
        assert!(candidate_diagnostics.is_empty());

        let mut postmerge = invocation(&current, ContinuityDisposition::Updated);
        postmerge.transition = ContinuityTransition::PostMerge;
        metadata.state.live.pull_request_state = "open".to_owned();
        let mut postmerge_diagnostics = Vec::new();
        evaluate_comparison(
            workspace,
            &policy,
            &postmerge,
            &comparison(&current, Some(true)),
            Some(&metadata),
            ExceptionState::None,
            &mut postmerge_diagnostics,
        );
        assert!(rules(&postmerge_diagnostics).contains(FRESHNESS_RULE));

        metadata.state.live.pull_request_state = "not-applicable".to_owned();
        let mut noop_diagnostics = Vec::new();
        evaluate_comparison(
            workspace,
            &policy,
            &invocation(&current, ContinuityDisposition::ReviewedNoChange),
            &comparison(&current, Some(false)),
            Some(&metadata),
            ExceptionState::None,
            &mut noop_diagnostics,
        );
        assert!(!rules(&noop_diagnostics).contains(COMPARISON_RULE));

        let mut excepted_policy = policy.clone();
        excepted_policy.exceptions.push(ContinuityException {
            repository: "example/public".to_owned(),
            owner: "fixture-owner".to_owned(),
            reason: "Bounded synthetic migration fixture.".to_owned(),
            approval: Some("https://github.com/example/public/issues/9".to_owned()),
            expires_on: "2026-09-30".to_owned(),
            review_trigger: "Contract lifecycle changes.".to_owned(),
            validation_state: "approved".to_owned(),
            exit_criteria: "Remove after the fixture migration.".to_owned(),
        });
        let exception_invocation = invocation(&current, ContinuityDisposition::Exception);
        let mut exception_diagnostics = Vec::new();
        assert_eq!(
            evaluate_exceptions(
                &excepted_policy,
                &exception_invocation,
                &mut exception_diagnostics,
            ),
            ExceptionState::Applied
        );
        assert!(exception_diagnostics.is_empty());

        excepted_policy.exceptions[0].expires_on = "2026-09-01".to_owned();
        assert_eq!(
            evaluate_exceptions(&excepted_policy, &exception_invocation, &mut Vec::new(),),
            ExceptionState::Invalid
        );
    }

    #[test]
    fn rollout_and_applicability_do_not_overclaim_conformance() {
        assert_eq!(
            effective_severity(
                Severity::Critical,
                ContinuityRolloutStage::Observe,
                ContinuityRequirement::Required,
                false,
            ),
            Severity::Warning
        );
        assert_eq!(
            effective_severity(
                Severity::Error,
                ContinuityRolloutStage::Ratchet,
                ContinuityRequirement::Required,
                true,
            ),
            Severity::Warning
        );
        assert_eq!(
            effective_severity(
                Severity::Error,
                ContinuityRolloutStage::Ratchet,
                ContinuityRequirement::Required,
                false,
            ),
            Severity::Error
        );
        assert_eq!(
            effective_severity(
                Severity::Critical,
                ContinuityRolloutStage::Enforce,
                ContinuityRequirement::Required,
                false,
            ),
            Severity::Critical
        );

        let mut generated = policy("example/public", ContinuityVisibility::Public);
        generated.repository_kind = ContinuityRepositoryKind::GeneratedOnly;
        assert_eq!(
            resolve_applicability(&generated),
            ContinuityRequirement::Advisory
        );
        generated.repository_kind = ContinuityRepositoryKind::Mirror;
        assert_eq!(
            resolve_applicability(&generated),
            ContinuityRequirement::NotApplicable
        );
        generated.repository_kind = ContinuityRepositoryKind::Standard;
        generated.lifecycle = ContinuityLifecycle::Archived;
        assert_eq!(
            resolve_applicability(&generated),
            ContinuityRequirement::NotApplicable
        );

        let (_temporary, workspace) = initialize_repository();
        write_file(&workspace, "seed.txt", b"fixture");
        let base = commit_all(&workspace, "test: initialize mirror fixture");
        let inventory = RepositoryInventory::discover(&workspace).expect("mirror inventory");
        let evaluator = RepositoryContinuityEvaluator::new(
            &generated,
            Path::new("policy.toml"),
            invocation(&base, ContinuityDisposition::Updated),
        )
        .expect("mirror evaluator");
        let evaluation = evaluator
            .evaluate(&workspace, &inventory)
            .expect("mirror evaluation");
        assert_eq!(
            evaluation.report.status,
            ContinuityValidationStatus::NotApplicable
        );
        assert!(evaluation.findings.is_empty());
    }

    #[test]
    fn topology_distinguishes_linear_merge_and_rebased_histories() {
        let (_temporary, workspace) = initialize_repository();
        write_file(&workspace, "base.txt", b"base");
        let base = commit_all(&workspace, "test: base");

        run_git(&workspace, &["switch", "--create", "candidate"]);
        write_file(&workspace, "candidate.txt", b"candidate");
        let candidate = commit_all(&workspace, "test: candidate");

        run_git(&workspace, &["switch", "--create", "parallel", &base]);
        write_file(&workspace, "parallel.txt", b"parallel");
        let parallel = commit_all(&workspace, "test: parallel");

        assert_eq!(
            determine_topology(
                &workspace,
                &ContinuityInvocation {
                    head_revision: candidate.clone(),
                    ..invocation(&base, ContinuityDisposition::Updated)
                },
                &revision_evidence(&base),
                &revision_evidence(&candidate),
            )
            .expect("linear topology"),
            ContinuityTopology::Linear
        );
        assert_eq!(
            determine_topology(
                &workspace,
                &ContinuityInvocation {
                    base_revision: candidate.clone(),
                    head_revision: parallel.clone(),
                    ..invocation(&candidate, ContinuityDisposition::Updated)
                },
                &revision_evidence(&candidate),
                &revision_evidence(&parallel),
            )
            .expect("diverged topology"),
            ContinuityTopology::Diverged
        );

        run_git(&workspace, &["switch", "candidate"]);
        run_git(
            &workspace,
            &["merge", "--no-ff", "parallel", "--message", "test: merge"],
        );
        let merge = run_git(&workspace, &["rev-parse", "HEAD"]);
        assert_eq!(
            determine_topology(
                &workspace,
                &ContinuityInvocation {
                    head_revision: merge.clone(),
                    ..invocation(&base, ContinuityDisposition::Updated)
                },
                &revision_evidence(&base),
                &revision_evidence(&merge),
            )
            .expect("merge topology"),
            ContinuityTopology::MergeCommit
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn parallel_heads_and_shallow_missing_bases_remain_explicit() {
        let policy = policy("example/public", ContinuityVisibility::Public);
        let (_temporary, workspace) = initialize_repository();
        materialize_repository(&workspace, PUBLIC_CONTINUITY);
        let base = commit_all(&workspace, "test: continuity base");

        run_git(&workspace, &["switch", "--create", "candidate"]);
        let candidate_contents = PUBLIC_CONTINUITY
            .replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &base)
            .replace(
                "Exercise the public continuity fixture.",
                "Exercise candidate continuity state.",
            );
        write_file(&workspace, "CONTINUITY.md", candidate_contents.as_bytes());
        let candidate = commit_all(&workspace, "test: candidate continuity");

        run_git(&workspace, &["switch", "--create", "parallel", &base]);
        let parallel_contents = PUBLIC_CONTINUITY
            .replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &base)
            .replace(
                "Exercise the public continuity fixture.",
                "Exercise parallel continuity state.",
            );
        write_file(&workspace, "CONTINUITY.md", parallel_contents.as_bytes());
        let parallel = commit_all(&workspace, "test: parallel continuity");
        run_git(&workspace, &["switch", "candidate"]);

        let inventory = RepositoryInventory::discover(&workspace).expect("candidate inventory");
        let mut parallel_invocation = invocation(&base, ContinuityDisposition::Updated);
        parallel_invocation.head_revision.clone_from(&candidate);
        parallel_invocation.parallel_heads.push(parallel);
        let evaluator = RepositoryContinuityEvaluator::new(
            &policy,
            Path::new("policy.toml"),
            parallel_invocation,
        )
        .expect("parallel evaluator");
        let evaluation = evaluator
            .evaluate(&workspace, &inventory)
            .expect("parallel evaluation");
        assert_eq!(
            evaluation.report.comparison.parallel_checkpoint_conflicts, 1,
            "{:#?}",
            evaluation.report
        );
        assert!(
            evaluation
                .report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == PARALLEL_RULE)
        );

        run_git(&workspace, &["switch", "main"]);
        write_file(&workspace, "second.txt", b"second");
        commit_all(&workspace, "test: shallow head");
        let clone_parent = tempfile::tempdir().expect("shallow clone parent");
        let clone = clone_parent.path().join("shallow");
        let output = Command::new("git")
            .args(["clone", "--no-local", "--depth=1"])
            .arg(&workspace)
            .arg(&clone)
            .output()
            .expect("create shallow clone");
        assert!(
            output.status.success(),
            "shallow clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let shallow_inventory = RepositoryInventory::discover(&clone).expect("shallow inventory");
        let shallow_evaluator = RepositoryContinuityEvaluator::new(
            &policy,
            Path::new("policy.toml"),
            invocation(&base, ContinuityDisposition::Updated),
        )
        .expect("shallow evaluator");
        let shallow = collect_git_comparison(
            &clone,
            &shallow_inventory,
            &shallow_evaluator.content_paths(),
            &policy,
            &shallow_evaluator.invocation,
        )
        .expect("shallow comparison");
        assert!(shallow.shallow);
        assert_eq!(shallow.base.state, ContinuityRevisionState::Unavailable);
        assert!(rules(&shallow.diagnostics).contains(COMPARISON_RULE));
    }

    #[test]
    fn reports_and_human_finding_inputs_are_deterministic() {
        let policy = policy("example/public", ContinuityVisibility::Public);
        let (_temporary, workspace) = initialize_repository();
        materialize_repository(&workspace, PUBLIC_CONTINUITY);
        let base = commit_all(&workspace, "test: deterministic base");
        let contents = PUBLIC_CONTINUITY
            .replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &base)
            .replace(
                "Exercise the public continuity fixture.",
                "Exercise deterministic continuity state.",
            );
        write_file(&workspace, "CONTINUITY.md", contents.as_bytes());
        let inventory = RepositoryInventory::discover(&workspace).expect("working inventory");
        let evaluator = RepositoryContinuityEvaluator::new(
            &policy,
            Path::new("policy.toml"),
            invocation(&base, ContinuityDisposition::Updated),
        )
        .expect("deterministic evaluator");

        let first = evaluator
            .evaluate(&workspace, &inventory)
            .expect("first evaluation");
        let second = evaluator
            .evaluate(&workspace, &inventory)
            .expect("second evaluation");
        assert_eq!(
            serde_json::to_string_pretty(&first.report).expect("first JSON"),
            serde_json::to_string_pretty(&second.report).expect("second JSON")
        );
        let human_inputs = |evaluation: &ContinuityEvaluation| {
            evaluation
                .findings
                .iter()
                .map(|finding| {
                    format!(
                        "{:?}:{}:{}",
                        finding.severity, finding.rule.rule_id, finding.message
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(human_inputs(&first), human_inputs(&second));
        assert_eq!(
            first.report.status,
            ContinuityValidationStatus::Valid,
            "{:#?}",
            first.report
        );
    }

    #[test]
    fn scenario_manifest_names_every_required_adversarial_case() {
        let scenarios = include_str!("../../tests/fixtures/repository-continuity/scenarios.txt");
        for expected in [
            "valid-fresh-public",
            "valid-fresh-private",
            "missing-continuity",
            "miscased-continuity",
            "missing-agents-block",
            "duplicate-agents-block",
            "overwritten-agents-block",
            "malformed-metadata",
            "unsupported-version",
            "candidate-before-merge",
            "stale-open-pr-after-merge",
            "parallel-same-baseline",
            "reviewed-no-change",
            "approved-exception",
            "invalid-exception",
            "observe-rollout",
            "ratchet-rollout",
            "enforce-rollout",
            "merge-commit",
            "squash-merge",
            "rebase-divergence",
            "shallow-missing-base",
            "malicious-authority",
            "synthetic-secret-like-value",
            "deterministic-json-and-human-findings",
        ] {
            assert!(scenarios.lines().any(|scenario| scenario == expected));
        }
    }
}
