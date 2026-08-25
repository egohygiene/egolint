//! Offline semantic validation for Repository Intelligence source records.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation,
};
use crate::error::{EgolintError, Result};

use super::{RepositoryEntryKind, RepositoryInventory};

/// Stable tool identifier used by normalized findings and tool results.
pub const TOOL_ID: &str = "EGOLINT_REPOSITORY_INTELLIGENCE";
/// Dedicated machine-readable validation artifact.
pub const REPORT_PATH: &str = ".reports/egolint/repository-intelligence.json";

const POLICY_CONTRACT: &str = "egolint.repository-intelligence-validation/v1";
const CATALOG_PATH: &str = ".config/rules/repository-intelligence.v1.toml";
const CATALOG_SOURCE: &str = include_str!("../../.config/rules/repository-intelligence.v1.toml");

const CONTRACT_RULE: &str = "EGO-INTEL-CONTRACT-001";
const ADOPTION_RULE: &str = "EGO-INTEL-ADOPTION-001";
const ADR_METADATA_RULE: &str = "EGO-INTEL-ADR-METADATA-001";
const ADR_LIFECYCLE_RULE: &str = "EGO-INTEL-ADR-LIFECYCLE-001";
const ADR_INDEX_RULE: &str = "EGO-INTEL-ADR-INDEX-001";
const ADR_LINEAGE_RULE: &str = "EGO-INTEL-ADR-LINEAGE-001";
const ROADMAP_STRUCTURE_RULE: &str = "EGO-INTEL-ROADMAP-STRUCTURE-001";
const ROADMAP_STATE_RULE: &str = "EGO-INTEL-ROADMAP-STATE-001";
const LINK_RULE: &str = "EGO-INTEL-LINK-001";
const TRAILER_RULE: &str = "EGO-INTEL-TRAILER-001";
const CYCLE_RULE: &str = "EGO-INTEL-CYCLE-001";

const EXPECTED_RULE_IDS: [&str; 11] = [
    CONTRACT_RULE,
    ADOPTION_RULE,
    ADR_METADATA_RULE,
    ADR_LIFECYCLE_RULE,
    ADR_INDEX_RULE,
    ADR_LINEAGE_RULE,
    ROADMAP_STRUCTURE_RULE,
    ROADMAP_STATE_RULE,
    LINK_RULE,
    TRAILER_RULE,
    CYCLE_RULE,
];

const ADR_CONTRACT: &str = "egohygiene.architecture-decision/v1";
const ADR_REFERENCE_CONTRACT: &str =
    "egohygiene.architecture-decision-policy-reference/v1";
const ROADMAP_CONTRACT: &str = "hygiene.roadmap/v1alpha1";

const REQUIRED_ADR_SECTIONS: [&str; 7] = [
    "Context",
    "Decision",
    "Alternatives considered and rejected",
    "Consequences and tradeoffs",
    "Implementation and evidence links",
    "Replacement or exit strategy",
    "Follow-up work",
];

/// Whether a Repository Intelligence source surface is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AdoptionState {
    /// The source is declared and must validate.
    Present,
    /// Migration has not yet established whether the source is available.
    Unknown,
    /// The source is intentionally irrelevant to this repository.
    NotApplicable,
}

/// Quality-gate behavior for the declared Repository Intelligence profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IntelligenceEnforcement {
    /// Error-level enabled rules fail the Egolint run.
    Blocking,
    /// Enabled rules remain visible as warnings without failing the run.
    Advisory,
}

/// Exact rules and enforcement selected by a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IntelligenceProfile {
    /// Stable repository-owned profile name.
    pub id: String,
    /// Whether enabled rule failures block CI.
    pub enforcement: IntelligenceEnforcement,
    /// Stable Egolint rule IDs evaluated by this profile.
    pub enabled_rules: Vec<String>,
}

/// One exact upstream contract supported by the bundled rule catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IntelligenceContractPin {
    /// Versioned contract identifier.
    pub id: String,
    /// Exact semantic contract or policy version.
    pub version: String,
    /// Current authority state, such as `proposed` or `approved`.
    pub authority: String,
    /// Canonical source repository.
    pub source_repository: String,
    /// Immutable 40-character Git revision.
    pub source_revision: String,
    /// Repository-relative canonical source path.
    pub source_path: PathBuf,
}

/// ADR surface declaration for incremental adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AdrSurface {
    /// Evidence availability.
    pub state: AdoptionState,
    /// Repository policy inheritance declaration.
    pub policy_reference: PathBuf,
    /// Directory containing canonical ADR Markdown.
    pub decision_directory: PathBuf,
    /// Canonical human-readable ADR index.
    pub index: PathBuf,
    /// Pinned external decision identities allowed in cross-references.
    #[serde(default)]
    pub known_external_decisions: Vec<String>,
}

/// Roadmap surface declaration for incremental adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RoadmapSurface {
    /// Evidence availability.
    pub state: AdoptionState,
    /// Canonical roadmap Markdown path.
    pub path: PathBuf,
    /// Pinned external roadmap step identities allowed in dependencies/trailers.
    #[serde(default)]
    pub known_external_steps: Vec<String>,
}

/// Commit-history surface declaration for incremental adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CommitHistorySurface {
    /// Evidence availability.
    pub state: AdoptionState,
    /// Maximum number of commits inspected from the represented revision.
    pub maximum_commits: u32,
}

/// Repository-owned, versioned semantic validation policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RepositoryIntelligencePolicy {
    /// Policy envelope version.
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    /// Exact Egolint policy contract identifier.
    pub id: String,
    /// Canonical repository identity.
    pub repository: String,
    /// Selected rule profile.
    pub profile: IntelligenceProfile,
    /// Exact upstream inputs supported by the bundled catalog.
    pub contracts: Vec<IntelligenceContractPin>,
    /// ADR source coverage and paths.
    pub adrs: AdrSurface,
    /// Roadmap source coverage and path.
    pub roadmap: RoadmapSurface,
    /// Commit source coverage and deterministic bound.
    pub commit_history: CommitHistorySurface,
}

impl RepositoryIntelligencePolicy {
    /// Decode and structurally validate a repository policy.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed TOML, unsafe paths, unsupported envelope
    /// versions, duplicate declarations, or an empty rule profile.
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
                "repository-intelligence policy must use schema-version {CONTRACT_VERSION} and id {POLICY_CONTRACT}"
            )));
        }
        if !valid_repository(&self.repository) {
            return Err(EgolintError::Configuration(
                "repository-intelligence repository must use egohygiene/owner-name form"
                    .to_owned(),
            ));
        }
        if self.profile.id.trim().is_empty() || self.profile.enabled_rules.is_empty() {
            return Err(EgolintError::Configuration(
                "repository-intelligence profile needs an id and at least one enabled rule"
                    .to_owned(),
            ));
        }
        ensure_unique(
            self.profile.enabled_rules.iter().map(String::as_str),
            "enabled repository-intelligence rule",
        )?;
        ensure_unique(
            self.contracts.iter().map(|contract| contract.id.as_str()),
            "repository-intelligence contract",
        )?;
        ensure_unique(
            self.adrs
                .known_external_decisions
                .iter()
                .map(String::as_str),
            "known external ADR",
        )?;
        ensure_unique(
            self.roadmap
                .known_external_steps
                .iter()
                .map(String::as_str),
            "known external roadmap step",
        )?;
        for (name, path) in [
            ("ADR policy-reference", &self.adrs.policy_reference),
            ("ADR decision-directory", &self.adrs.decision_directory),
            ("ADR index", &self.adrs.index),
            ("roadmap", &self.roadmap.path),
        ] {
            validate_relative_path(path, name)?;
        }
        if self.commit_history.maximum_commits == 0
            || self.commit_history.maximum_commits > 50_000
        {
            return Err(EgolintError::Configuration(
                "commit-history maximum-commits must be between 1 and 50000".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Explicit represented-commit evidence included in every validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RepresentedCommit {
    /// Whether a commit is available, unknown, or not applicable.
    pub state: AdoptionState,
    /// Full lowercase Git commit when `state` is `present`.
    pub revision: Option<String>,
}

impl RepresentedCommit {
    /// Parse a full commit, `unknown`, or `not-applicable` CLI value.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is neither an explicit absence state nor
    /// a full lowercase Git commit.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "unknown" => Ok(Self {
                state: AdoptionState::Unknown,
                revision: None,
            }),
            "not-applicable" => Ok(Self {
                state: AdoptionState::NotApplicable,
                revision: None,
            }),
            revision if valid_commit(revision) => Ok(Self {
                state: AdoptionState::Present,
                revision: Some(revision.to_owned()),
            }),
            _ => Err(EgolintError::Configuration(
                "represented commit must be a full lowercase Git SHA, unknown, or not-applicable"
                    .to_owned(),
            )),
        }
    }
}

/// One commit message inspected for optional linkage trailers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CommitRecord {
    /// Full commit SHA.
    pub sha: String,
    /// Complete commit message.
    pub message: String,
}

/// Bounded deterministic commit-history snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHistory {
    /// Commits in Git log order.
    pub records: Vec<CommitRecord>,
    /// Whether older history exists beyond the declared maximum.
    pub truncated: bool,
}

/// One structured semantic diagnostic in the dedicated report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct IntelligenceDiagnostic {
    /// Stable diagnostic identifier.
    pub id: String,
    /// Stable Egolint rule identifier.
    pub rule_id: String,
    /// Effective severity after profile enforcement.
    pub severity: Severity,
    /// Concise local diagnostic.
    pub message: String,
    /// Structured actionable repair guidance.
    pub remediation: String,
    /// Exact source location when available.
    pub location: Option<SourceLocation>,
    /// Hygiene contracts that define the validated meaning.
    pub contracts: Vec<String>,
}

/// Semantic validity independent of whether the profile blocks CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceValidationStatus {
    /// Every declared present surface passed enabled rules.
    Valid,
    /// At least one enabled semantic rule found invalid source data.
    Invalid,
    /// Unknown or truncated evidence prevents a conformance claim.
    Incomplete,
    /// Every source surface was explicitly not applicable.
    NotApplicable,
}

/// Exact counts for the dedicated semantic report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct IntelligenceValidationSummary {
    /// Number of structured diagnostics.
    pub diagnostics: u64,
    /// Number of effective error/critical diagnostics.
    pub blocking_diagnostics: u64,
    /// Number of commits inspected for optional trailers.
    pub commits_inspected: u64,
    /// Number of canonical ADR records inspected.
    pub adrs_inspected: u64,
    /// Number of canonical roadmap steps inspected.
    pub roadmap_steps_inspected: u64,
}

/// Versioned evidence artifact consumed directly by Relay and Observatory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RepositoryIntelligenceReport {
    /// Report contract version.
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    /// Exact report contract identifier.
    pub contract: String,
    /// Bundled rule-catalog version.
    pub catalog_version: String,
    /// Repository whose records were inspected.
    pub repository: String,
    /// Repository policy path.
    pub policy_path: PathBuf,
    /// Selected rule profile.
    pub profile: IntelligenceProfile,
    /// Exact Hygiene contract pins used for meaning.
    pub contracts: Vec<IntelligenceContractPin>,
    /// Exact represented commit or explicit absence state.
    pub represented_commit: RepresentedCommit,
    /// Declared ADR evidence coverage.
    pub adrs: AdoptionState,
    /// Declared roadmap evidence coverage.
    pub roadmap: AdoptionState,
    /// Declared commit-history evidence coverage.
    pub commit_history: AdoptionState,
    /// Whether the commit-history collector hit its explicit bound.
    pub commit_history_truncated: bool,
    /// Semantic validity independent of advisory/blocking enforcement.
    pub status: IntelligenceValidationStatus,
    /// Exact normalized counts.
    pub summary: IntelligenceValidationSummary,
    /// Stable ordered machine diagnostics with remediation.
    pub diagnostics: Vec<IntelligenceDiagnostic>,
}

/// Normalized and Egolint-compatible outputs from one semantic evaluation.
pub struct IntelligenceEvaluation {
    /// Existing v1 findings for run-report and SARIF compatibility.
    pub findings: Vec<Finding>,
    /// Dedicated Repository Intelligence evidence artifact.
    pub report: RepositoryIntelligenceReport,
}

/// Atomically write the dedicated Repository Intelligence report inside the
/// fixed Egolint report boundary.
///
/// # Errors
///
/// Returns an error when the target is unsafe or durable JSON persistence
/// fails.
pub fn write_intelligence_report_atomic(
    report: &RepositoryIntelligenceReport,
    path: &Path,
) -> Result<()> {
    if path != Path::new(REPORT_PATH) && !path.ends_with(REPORT_PATH) {
        return Err(EgolintError::Configuration(format!(
            "repository-intelligence report path must end with {REPORT_PATH}"
        )));
    }
    let (path, parent) = crate::sarif::validated_report_target(path)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&parent).map_err(|source| {
        EgolintError::Filesystem {
            path: parent.clone(),
            source,
        }
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
struct IntelligenceCatalog {
    schema_version: u32,
    catalog_version: String,
    owner: String,
    tool_id: String,
    policy_source: String,
    upstream_contracts: Vec<IntelligenceContractPin>,
    rules: Vec<IntelligenceRuleDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct IntelligenceRuleDefinition {
    id: String,
    title: String,
    default_severity: Severity,
    contracts: Vec<String>,
    remediation: String,
}

#[derive(Debug)]
struct BundledCatalog {
    version: String,
    owner: String,
    tool_id: String,
    policy_source: String,
    contracts: BTreeMap<String, IntelligenceContractPin>,
    rules: BTreeMap<String, IntelligenceRuleDefinition>,
}

impl BundledCatalog {
    fn load() -> Result<Self> {
        let catalog: IntelligenceCatalog =
            toml::from_str(CATALOG_SOURCE).map_err(|source| EgolintError::Toml {
                path: PathBuf::from(CATALOG_PATH),
                source,
            })?;
        if catalog.schema_version != CONTRACT_VERSION
            || catalog.owner != "egohygiene/egolint"
            || catalog.tool_id != TOOL_ID
            || catalog.policy_source != CATALOG_PATH
        {
            return Err(EgolintError::Configuration(
                "bundled Repository Intelligence catalog identity drifted".to_owned(),
            ));
        }
        let mut contracts = BTreeMap::new();
        for contract in catalog.upstream_contracts {
            if contracts.insert(contract.id.clone(), contract).is_some() {
                return Err(EgolintError::Configuration(
                    "bundled Repository Intelligence catalog has duplicate contracts".to_owned(),
                ));
            }
        }
        let mut rules = BTreeMap::new();
        for rule in catalog.rules {
            if rule.title.trim().is_empty()
                || rule.remediation.trim().is_empty()
                || rule.contracts.is_empty()
                || rule
                    .contracts
                    .iter()
                    .any(|contract| !contracts.contains_key(contract))
            {
                return Err(EgolintError::Configuration(format!(
                    "bundled Repository Intelligence rule {} is incomplete",
                    rule.id
                )));
            }
            if rules.insert(rule.id.clone(), rule).is_some() {
                return Err(EgolintError::Configuration(
                    "bundled Repository Intelligence catalog has duplicate rules".to_owned(),
                ));
            }
        }
        let expected = EXPECTED_RULE_IDS.into_iter().collect::<BTreeSet<_>>();
        let observed = rules.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if expected != observed {
            return Err(EgolintError::Configuration(format!(
                "bundled Repository Intelligence rule IDs drifted: expected {expected:?}, observed {observed:?}"
            )));
        }
        Ok(Self {
            version: catalog.catalog_version,
            owner: catalog.owner,
            tool_id: catalog.tool_id,
            policy_source: catalog.policy_source,
            contracts,
            rules,
        })
    }
}

/// Deterministically collect commit messages reachable from one represented
/// revision without network access.
///
/// # Errors
///
/// Returns an error when Git cannot resolve the explicit commit or emit the
/// bounded local history.
pub fn collect_commit_history(
    workspace: &Path,
    represented: &RepresentedCommit,
    maximum_commits: u32,
) -> Result<CommitHistory> {
    if represented.state != AdoptionState::Present {
        return Ok(CommitHistory {
            records: Vec::new(),
            truncated: false,
        });
    }
    let revision = represented
        .revision
        .as_deref()
        .expect("present represented commit validated by parser");
    let object = format!("{revision}^{{commit}}");
    let verify = Command::new("git")
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-C")
        .arg(workspace)
        .args(["cat-file", "-e", &object])
        .output()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if !verify.status.success() {
        return Err(EgolintError::Configuration(format!(
            "represented commit {revision} is not available in the local Git checkout"
        )));
    }
    let requested = maximum_commits.saturating_add(1).to_string();
    let output = Command::new("git")
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-C")
        .arg(workspace)
        .args([
            "log",
            "--no-show-signature",
            "--format=%H%x00%B%x00",
            "--max-count",
            &requested,
            revision,
            "--",
        ])
        .output()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(EgolintError::Configuration(format!(
            "could not collect represented commit history: {message}"
        )));
    }
    let fields = output.stdout.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut records = Vec::new();
    for pair in fields.chunks(2) {
        if pair.len() != 2 || pair[0].is_empty() {
            continue;
        }
        let sha_bytes = trim_ascii_whitespace(pair[0]);
        let sha = std::str::from_utf8(sha_bytes).map_err(|_| {
            EgolintError::Configuration("Git commit IDs must contain UTF-8".to_owned())
        })?;
        let message = std::str::from_utf8(pair[1]).map_err(|_| {
            EgolintError::Configuration("Git commit messages must contain UTF-8".to_owned())
        })?;
        if !valid_commit(sha) {
            return Err(EgolintError::Configuration(
                "Git returned a malformed commit ID".to_owned(),
            ));
        }
        records.push(CommitRecord {
            sha: sha.to_owned(),
            message: message.to_owned(),
        });
    }
    let shallow = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    let is_shallow = shallow.status.success() && shallow.stdout.starts_with(b"true");
    let truncated = records.len() > maximum_commits as usize || is_shallow;
    records.truncate(maximum_commits as usize);
    Ok(CommitHistory { records, truncated })
}

/// Bundled semantic evaluator for one repository-owned policy.
pub struct RepositoryIntelligenceEvaluator<'a> {
    policy: &'a RepositoryIntelligencePolicy,
    policy_path: PathBuf,
    represented_commit: RepresentedCommit,
    catalog: BundledCatalog,
    enabled_rules: BTreeSet<String>,
}

impl<'a> RepositoryIntelligenceEvaluator<'a> {
    /// Construct an evaluator and validate the local profile surface.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy path is unsafe or the profile names an
    /// unknown rule. Contract pin drift is a reportable diagnostic so callers
    /// still receive machine evidence.
    pub fn new(
        policy: &'a RepositoryIntelligencePolicy,
        policy_path: &Path,
        represented_commit: RepresentedCommit,
    ) -> Result<Self> {
        validate_relative_path(policy_path, "repository-intelligence policy path")?;
        let catalog = BundledCatalog::load()?;
        let enabled_rules = policy
            .profile
            .enabled_rules
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if let Some(unknown) = enabled_rules
            .iter()
            .find(|rule| !catalog.rules.contains_key(rule.as_str()))
        {
            return Err(EgolintError::Configuration(format!(
                "repository-intelligence profile enables unknown rule {unknown}"
            )));
        }
        Ok(Self {
            policy,
            policy_path: policy_path.to_path_buf(),
            represented_commit,
            catalog,
            enabled_rules,
        })
    }

    /// Evaluate declared ADR, roadmap, and commit linkage sources.
    ///
    /// # Errors
    ///
    /// Returns an error only for an invalid evaluator input. Semantic source
    /// drift is returned as stable diagnostics and normalized findings.
    pub fn evaluate(
        &self,
        inventory: &RepositoryInventory,
        history: &CommitHistory,
    ) -> Result<IntelligenceEvaluation> {
        let mut context = EvaluationContext::default();
        self.validate_contract_pins(&mut context)?;
        self.validate_adoption(history, &mut context)?;
        if self.policy.adrs.state == AdoptionState::Present {
            self.validate_adrs(inventory, &mut context)?;
        }
        if self.policy.roadmap.state == AdoptionState::Present {
            self.validate_roadmap(inventory, &mut context)?;
        }
        if self.policy.commit_history.state == AdoptionState::Present {
            self.validate_commit_trailers(history, &mut context)?;
        }
        context.diagnostics.sort_by(diagnostic_order);
        let blocking_diagnostics = context
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(diagnostic.severity, Severity::Error | Severity::Critical)
            })
            .count() as u64;
        let all_not_applicable = [
            self.policy.adrs.state,
            self.policy.roadmap.state,
            self.policy.commit_history.state,
        ]
        .into_iter()
        .all(|state| state == AdoptionState::NotApplicable);
        let incomplete = self.represented_commit.state == AdoptionState::Unknown
            || history.truncated
            || [
                self.policy.adrs.state,
                self.policy.roadmap.state,
                self.policy.commit_history.state,
            ]
            .into_iter()
            .any(|state| state == AdoptionState::Unknown);
        let invalid = context
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id != ADOPTION_RULE);
        let status = if invalid {
            IntelligenceValidationStatus::Invalid
        } else if all_not_applicable {
            IntelligenceValidationStatus::NotApplicable
        } else if incomplete {
            IntelligenceValidationStatus::Incomplete
        } else {
            IntelligenceValidationStatus::Valid
        };
        let findings = context
            .diagnostics
            .iter()
            .map(|diagnostic| self.normalized_finding(diagnostic))
            .collect::<Vec<_>>();
        for finding in &findings {
            finding.validate()?;
        }
        let report = RepositoryIntelligenceReport {
            schema_version: CONTRACT_VERSION,
            contract: POLICY_CONTRACT.to_owned(),
            catalog_version: self.catalog.version.clone(),
            repository: self.policy.repository.clone(),
            policy_path: self.policy_path.clone(),
            profile: self.policy.profile.clone(),
            contracts: self.policy.contracts.clone(),
            represented_commit: self.represented_commit.clone(),
            adrs: self.policy.adrs.state,
            roadmap: self.policy.roadmap.state,
            commit_history: self.policy.commit_history.state,
            commit_history_truncated: history.truncated,
            status,
            summary: IntelligenceValidationSummary {
                diagnostics: context.diagnostics.len() as u64,
                blocking_diagnostics,
                commits_inspected: history.records.len() as u64,
                adrs_inspected: context.adrs.len() as u64,
                roadmap_steps_inspected: context.roadmap_steps.len() as u64,
            },
            diagnostics: context.diagnostics,
        };
        Ok(IntelligenceEvaluation { findings, report })
    }

    fn validate_contract_pins(&self, context: &mut EvaluationContext) -> Result<()> {
        let declared = self
            .policy
            .contracts
            .iter()
            .map(|contract| (contract.id.as_str(), contract))
            .collect::<BTreeMap<_, _>>();
        let configured_contracts = self.catalog.contracts.keys().cloned().collect::<Vec<_>>();
        for contract_id in configured_contracts {
            let expected = self
                .catalog
                .contracts
                .get(&contract_id)
                .expect("catalog key came from catalog");
            match declared.get(contract_id.as_str()) {
                Some(observed) if *observed == expected => {}
                Some(observed) => self.emit(
                    context,
                    CONTRACT_RULE,
                    Some(location(&self.policy_path, None)),
                    format!(
                        "contract {} does not match the supported {} {} pin at {}",
                        observed.id,
                        expected.authority,
                        expected.version,
                        expected.source_revision
                    ),
                )?,
                None => self.emit(
                    context,
                    CONTRACT_RULE,
                    Some(location(&self.policy_path, None)),
                    format!("required supported contract {contract_id} is not declared"),
                )?,
            }
        }
        for contract in &self.policy.contracts {
            if !self.catalog.contracts.contains_key(&contract.id) {
                self.emit(
                    context,
                    CONTRACT_RULE,
                    Some(location(&self.policy_path, None)),
                    format!("contract {} is not supported by this Egolint catalog", contract.id),
                )?;
            }
        }
        Ok(())
    }

    fn validate_adoption(
        &self,
        history: &CommitHistory,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        for (surface, state) in [
            ("adrs", self.policy.adrs.state),
            ("roadmap", self.policy.roadmap.state),
            ("commit-history", self.policy.commit_history.state),
        ] {
            if state == AdoptionState::Unknown {
                self.emit(
                    context,
                    ADOPTION_RULE,
                    Some(location(&self.policy_path, None)),
                    format!("{surface} evidence is explicitly unknown; conformance is incomplete"),
                )?;
            }
        }
        if self.represented_commit.state == AdoptionState::Unknown {
            self.emit(
                context,
                ADOPTION_RULE,
                Some(location(&self.policy_path, None)),
                "represented commit is unknown; the report cannot claim a complete source snapshot"
                    .to_owned(),
            )?;
        }
        if self.represented_commit.state == AdoptionState::NotApplicable
            && [
                self.policy.adrs.state,
                self.policy.roadmap.state,
                self.policy.commit_history.state,
            ]
            .into_iter()
            .any(|state| state == AdoptionState::Present)
        {
            self.emit(
                context,
                ADOPTION_RULE,
                Some(location(&self.policy_path, None)),
                "represented commit cannot be not-applicable while a source surface is present"
                    .to_owned(),
            )?;
        }
        if history.truncated {
            self.emit(
                context,
                ADOPTION_RULE,
                Some(location(&self.policy_path, None)),
                format!(
                    "commit evidence is truncated at the declared maximum of {} commits",
                    self.policy.commit_history.maximum_commits
                ),
            )?;
        }
        Ok(())
    }

    fn emit(
        &self,
        context: &mut EvaluationContext,
        rule_id: &str,
        location: Option<SourceLocation>,
        message: String,
    ) -> Result<()> {
        if !self.enabled_rules.contains(rule_id) {
            return Ok(());
        }
        let definition = self.catalog.rules.get(rule_id).ok_or_else(|| {
            EgolintError::Configuration(format!("unknown bundled rule {rule_id}"))
        })?;
        let severity = match self.policy.profile.enforcement {
            IntelligenceEnforcement::Blocking => definition.default_severity,
            IntelligenceEnforcement::Advisory => match definition.default_severity {
                Severity::Info => Severity::Info,
                Severity::Warning | Severity::Error | Severity::Critical => Severity::Warning,
            },
        };
        let fingerprint = stable_fingerprint(rule_id, location.as_ref(), &message);
        context.diagnostics.push(IntelligenceDiagnostic {
            id: format!("{rule_id}-{fingerprint}"),
            rule_id: rule_id.to_owned(),
            severity,
            message,
            remediation: definition.remediation.clone(),
            location,
            contracts: definition.contracts.clone(),
        });
        Ok(())
    }

    fn normalized_finding(&self, diagnostic: &IntelligenceDiagnostic) -> Finding {
        let message = format!(
            "{} Remediation: {}",
            diagnostic.message, diagnostic.remediation
        );
        let fingerprint = stable_fingerprint(
            &diagnostic.rule_id,
            diagnostic.location.as_ref(),
            &diagnostic.message,
        );
        Finding {
            schema_version: CONTRACT_VERSION,
            id: diagnostic.id.clone(),
            rule: RuleIdentity {
                tool_id: self.catalog.tool_id.clone(),
                rule_id: diagnostic.rule_id.clone(),
            },
            severity: diagnostic.severity,
            message,
            location: diagnostic.location.clone(),
            ownership: RuleOwnership {
                owner: self.catalog.owner.clone(),
                policy_source: format!(
                    "{}#{}",
                    self.catalog.policy_source, diagnostic.rule_id
                ),
                configuration_path: Some(self.policy_path.clone()),
            },
            fingerprint: Some(fingerprint),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Policy,
                path: PathBuf::from(CATALOG_PATH),
                sha256: None,
                description: Some(format!(
                    "Egolint catalog rule mapped to {}.",
                    diagnostic.contracts.join(", ")
                )),
            }],
            suppressed_by: None,
        }
    }
}

#[derive(Default)]
struct EvaluationContext {
    diagnostics: Vec<IntelligenceDiagnostic>,
    adrs: BTreeMap<String, AdrRecord>,
    roadmap_steps: BTreeMap<String, RoadmapStep>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyReference {
    schema: String,
    repository: String,
    policy: InheritedPolicy,
    decision_directory: PathBuf,
    index: PathBuf,
    extensions: Vec<PolicyExtension>,
    exceptions: Vec<PolicyException>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InheritedPolicy {
    contract: String,
    version: String,
    source: InheritedPolicySource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InheritedPolicySource {
    repository: String,
    revision: String,
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyExtension {
    id: String,
    kind: String,
    schema: PathBuf,
    required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyException {
    rule: String,
    reason: String,
    status: String,
    owner: String,
    approval_evidence: Option<String>,
    expires: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdrMetadata {
    schema: String,
    id: String,
    title: String,
    status: String,
    date: String,
    decision_scope: String,
    visibility: String,
    owners: Vec<String>,
    issue: Option<String>,
    pull_request: Option<String>,
    related: Vec<String>,
    supersedes: Vec<String>,
    superseded_by: Vec<String>,
    affected_repositories: Vec<String>,
    affected_contracts: Vec<String>,
    implementation_status: String,
    evidence: Vec<AdrEvidence>,
    exceptions: Vec<AdrException>,
    approval: Option<AdrApproval>,
    #[serde(default)]
    extensions: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdrEvidence {
    #[serde(rename = "type")]
    kind: String,
    url: String,
    description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdrApproval {
    date: String,
    by: String,
    evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdrException {
    rule: String,
    reason: String,
    status: String,
    owner: String,
    approval_evidence: Option<String>,
    expires: Option<String>,
}

#[derive(Debug, Clone)]
struct AdrRecord {
    path: PathBuf,
    line: u32,
    metadata: AdrMetadata,
}

impl RepositoryIntelligenceEvaluator<'_> {
    fn validate_adrs(
        &self,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let reference = self.parse_policy_reference(inventory, context)?;
        let mut records = Vec::new();
        for entry in inventory.entries() {
            if entry.kind != RepositoryEntryKind::File
                || !entry.path.starts_with(&self.policy.adrs.decision_directory)
                || entry.path.extension().and_then(|extension| extension.to_str()) != Some("md")
            {
                continue;
            }
            let Some(name) = entry.path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with("ADR-") || name == "ADR-TEMPLATE.md" {
                continue;
            }
            match std::str::from_utf8(&entry.content) {
                Ok(contents) => {
                    if let Some(record) = self.parse_adr(&entry.path, contents, context)? {
                        records.push(record);
                    }
                }
                Err(_) => self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(&entry.path, Some(1))),
                    "ADR Markdown must contain UTF-8".to_owned(),
                )?,
            }
        }
        records.sort_by(|left, right| left.path.cmp(&right.path));
        for record in records {
            let id = record.metadata.id.clone();
            if let Some(previous) = context.adrs.insert(id.clone(), record.clone()) {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(&record.path, Some(record.line))),
                    format!(
                        "duplicate ADR id {id}; first canonical record is {}",
                        portable_path(&previous.path)
                    ),
                )?;
            }
        }
        self.validate_adr_index(inventory, context)?;
        self.validate_adr_references(context)?;
        if let Some(reference) = reference {
            self.validate_required_extensions(&reference, context)?;
        }
        Ok(())
    }

    fn parse_policy_reference(
        &self,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> Result<Option<PolicyReference>> {
        let path = &self.policy.adrs.policy_reference;
        let Some(entry) = inventory.get(path) else {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, None)),
                "declared ADR policy-reference file is missing".to_owned(),
            )?;
            return Ok(None);
        };
        if entry.kind != RepositoryEntryKind::File {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, None)),
                "ADR policy-reference must be a regular file".to_owned(),
            )?;
            return Ok(None);
        }
        let reference = match serde_json::from_slice::<PolicyReference>(&entry.content) {
            Ok(reference) => reference,
            Err(error) => {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(error.line() as u32))),
                    format!("ADR policy-reference JSON is invalid: {error}"),
                )?;
                return Ok(None);
            }
        };
        let expected = self.catalog.contracts.get(ADR_CONTRACT).ok_or_else(|| {
            EgolintError::Configuration("bundled ADR contract is missing".to_owned())
        })?;
        let identity_matches = reference.schema == ADR_REFERENCE_CONTRACT
            && reference.repository == self.policy.repository
            && reference.policy.contract == ADR_CONTRACT
            && reference.policy.version == expected.version
            && reference.policy.source.repository == expected.source_repository
            && reference.policy.source.revision == expected.source_revision
            && reference.policy.source.path == Path::new("docs/decisions/POLICY.md")
            && reference.decision_directory == self.policy.adrs.decision_directory
            && reference.index == self.policy.adrs.index;
        if !identity_matches {
            self.emit(
                context,
                CONTRACT_RULE,
                Some(location(path, Some(1))),
                "ADR policy-reference does not match the repository, local paths, or supported immutable Hygiene policy pin"
                    .to_owned(),
            )?;
        }
        let mut extension_ids = BTreeSet::new();
        for extension in &reference.extensions {
            let valid = valid_contract_id(&extension.id)
                && matches!(extension.kind.as_str(), "metadata" | "validation")
                && validate_relative_path(&extension.schema, "extension schema").is_ok()
                && portable_path(&extension.schema).starts_with("schemas/")
                && extension.schema.extension().and_then(|value| value.to_str()) == Some("json")
                && extension_ids.insert(extension.id.clone());
            if !valid {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(1))),
                    format!("ADR policy extension {} is malformed or duplicated", extension.id),
                )?;
            }
        }
        for exception in &reference.exceptions {
            self.validate_policy_exception(exception, path, context)?;
        }
        Ok(Some(reference))
    }

    fn parse_adr(
        &self,
        path: &Path,
        contents: &str,
        context: &mut EvaluationContext,
    ) -> Result<Option<AdrRecord>> {
        let Some((front_matter, body, body_line)) = split_front_matter(contents) else {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, Some(1))),
                "ADR must begin with one YAML front matter document".to_owned(),
            )?;
            return Ok(None);
        };
        if front_matter.lines().any(unsafe_yaml_line) {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, Some(2))),
                "ADR front matter may not use YAML tags, anchors, aliases, or merge keys"
                    .to_owned(),
            )?;
            return Ok(None);
        }
        let metadata = match serde_yaml::from_str::<AdrMetadata>(front_matter) {
            Ok(metadata) => metadata,
            Err(error) => {
                let line = error.location().map_or(2, |location| location.line() as u32 + 1);
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(line))),
                    format!("ADR front matter is invalid: {error}"),
                )?;
                return Ok(None);
            }
        };
        self.validate_adr_metadata(path, &metadata, body, body_line, context)?;
        Ok(Some(AdrRecord {
            path: path.to_path_buf(),
            line: 2,
            metadata,
        }))
    }

    fn validate_adr_metadata(
        &self,
        path: &Path,
        metadata: &AdrMetadata,
        body: &str,
        body_line: u32,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        if metadata.schema != ADR_CONTRACT
            || !valid_adr_id(&metadata.id)
            || metadata.title.trim().is_empty()
            || metadata.title.len() > 160
            || !valid_date(&metadata.date)
            || !matches!(metadata.decision_scope.as_str(), "repository" | "organization")
            || !matches!(metadata.visibility.as_str(), "public" | "internal" | "private")
            || metadata.owners.is_empty()
            || !all_unique_nonempty(&metadata.owners)
            || metadata.affected_repositories.is_empty()
            || !all_unique_nonempty(&metadata.affected_repositories)
            || !all_unique_nonempty(&metadata.affected_contracts)
        {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, Some(2))),
                format!("ADR {} has malformed required v1 metadata", metadata.id),
            )?;
        }
        let filename_matches = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&format!("{}-", metadata.id)));
        if !filename_matches {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, Some(2))),
                format!("ADR id {} does not agree with its canonical filename", metadata.id),
            )?;
        }
        let heading = format!("# {}:", metadata.id);
        if !body.lines().any(|line| line.starts_with(&heading)) {
            self.emit(
                context,
                ADR_METADATA_RULE,
                Some(location(path, Some(body_line))),
                format!("ADR {} is missing a matching level-one heading", metadata.id),
            )?;
        }
        let mut prior = 0;
        for section in REQUIRED_ADR_SECTIONS {
            let needle = format!("## {section}");
            let Some(offset) = body.find(&needle) else {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(body_line))),
                    format!("ADR {} is missing required section {section}", metadata.id),
                )?;
                continue;
            };
            if offset < prior {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(body_line + line_number(body, offset) - 1))),
                    format!("ADR {} required sections are out of order", metadata.id),
                )?;
            }
            prior = offset;
        }
        self.validate_adr_lifecycle(path, metadata, context)?;
        self.validate_adr_links(path, metadata, context)?;
        Ok(())
    }

    fn validate_adr_lifecycle(
        &self,
        path: &Path,
        metadata: &AdrMetadata,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let valid_status = matches!(
            metadata.status.as_str(),
            "proposed" | "accepted" | "rejected" | "superseded" | "deprecated"
        );
        let valid_implementation = matches!(
            metadata.implementation_status.as_str(),
            "not_started"
                | "in_progress"
                | "implemented"
                | "verified"
                | "not_applicable"
                | "unknown"
        );
        if !valid_status || !valid_implementation {
            self.emit(
                context,
                ADR_LIFECYCLE_RULE,
                Some(location(path, Some(2))),
                format!("ADR {} declares an unsupported lifecycle state", metadata.id),
            )?;
        }
        match metadata.status.as_str() {
            "proposed" if metadata.approval.is_some() || !metadata.superseded_by.is_empty() => {
                self.emit(
                    context,
                    ADR_LIFECYCLE_RULE,
                    Some(location(path, Some(2))),
                    format!(
                        "proposed ADR {} may not claim approval or a superseding decision",
                        metadata.id
                    ),
                )?;
            }
            "accepted" | "rejected" | "superseded" | "deprecated"
                if metadata.approval.as_ref().is_none_or(|approval| {
                    !valid_date(&approval.date)
                        || approval.by.trim().is_empty()
                        || !valid_http_url(&approval.evidence)
                }) =>
            {
                self.emit(
                    context,
                    ADR_LIFECYCLE_RULE,
                    Some(location(path, Some(2))),
                    format!(
                        "non-proposed ADR {} requires durable human disposition evidence",
                        metadata.id
                    ),
                )?;
            }
            _ => {}
        }
        if metadata.status == "superseded" && metadata.superseded_by.is_empty() {
            self.emit(
                context,
                ADR_LIFECYCLE_RULE,
                Some(location(path, Some(2))),
                format!("superseded ADR {} needs a replacement link", metadata.id),
            )?;
        }
        if matches!(metadata.status.as_str(), "rejected" | "deprecated")
            && !metadata.superseded_by.is_empty()
        {
            self.emit(
                context,
                ADR_LIFECYCLE_RULE,
                Some(location(path, Some(2))),
                format!(
                    "{} ADR {} may not name a superseding decision",
                    metadata.status, metadata.id
                ),
            )?;
        }
        if metadata.implementation_status == "verified"
            && !metadata
                .evidence
                .iter()
                .any(|evidence| evidence.kind == "validation")
        {
            self.emit(
                context,
                ADR_LIFECYCLE_RULE,
                Some(location(path, Some(2))),
                format!("verified ADR {} needs validation evidence", metadata.id),
            )?;
        }
        for exception in &metadata.exceptions {
            if exception.rule.trim().is_empty()
                || exception.reason.trim().is_empty()
                || exception.owner.trim().is_empty()
                || !matches!(exception.status.as_str(), "proposed" | "approved" | "expired")
                || exception
                    .expires
                    .as_deref()
                    .is_some_and(|date| !valid_date(date))
                || (exception.status == "approved"
                    && exception
                        .approval_evidence
                        .as_deref()
                        .is_none_or(|url| !valid_http_url(url)))
            {
                self.emit(
                    context,
                    ADR_LIFECYCLE_RULE,
                    Some(location(path, Some(2))),
                    format!("ADR {} contains a malformed exception", metadata.id),
                )?;
            }
        }
        Ok(())
    }

    fn validate_adr_links(
        &self,
        path: &Path,
        metadata: &AdrMetadata,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        for (kind, url) in [
            ("issue", metadata.issue.as_deref()),
            ("pull request", metadata.pull_request.as_deref()),
        ] {
            if let Some(url) = url {
                let expected_segment = if kind == "issue" { "/issues/" } else { "/pull/" };
                if !valid_github_number_url(url, expected_segment) {
                    self.emit(
                        context,
                        LINK_RULE,
                        Some(location(path, Some(2))),
                        format!("ADR {} has a malformed {kind} URL", metadata.id),
                    )?;
                }
            }
        }
        for evidence in &metadata.evidence {
            let structurally_valid = !evidence.description.trim().is_empty()
                && matches!(
                    evidence.kind.as_str(),
                    "approval"
                        | "issue"
                        | "pull_request"
                        | "commit"
                        | "release"
                        | "workflow_run"
                        | "documentation"
                        | "implementation"
                        | "validation"
                        | "external"
                )
                && valid_http_url(&evidence.url)
                && match evidence.kind.as_str() {
                    "issue" => valid_github_number_url(&evidence.url, "/issues/"),
                    "pull_request" => valid_github_number_url(&evidence.url, "/pull/"),
                    "commit" => valid_github_commit_url(&evidence.url),
                    _ => true,
                };
            if !structurally_valid {
                self.emit(
                    context,
                    LINK_RULE,
                    Some(location(path, Some(2))),
                    format!("ADR {} contains malformed {} evidence", metadata.id, evidence.kind),
                )?;
            }
        }
        for contract in &metadata.affected_contracts {
            if !valid_contract_id(contract) {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(2))),
                    format!("ADR {} contains malformed affected contract {contract}", metadata.id),
                )?;
            }
        }
        for repository in &metadata.affected_repositories {
            if repository != "egohygiene/*" && !valid_repository(repository) {
                self.emit(
                    context,
                    ADR_METADATA_RULE,
                    Some(location(path, Some(2))),
                    format!(
                        "ADR {} contains malformed affected repository {repository}",
                        metadata.id
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn validate_adr_index(
        &self,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let path = &self.policy.adrs.index;
        let Some(entry) = inventory.get(path) else {
            self.emit(
                context,
                ADR_INDEX_RULE,
                Some(location(path, None)),
                "declared ADR index is missing".to_owned(),
            )?;
            return Ok(());
        };
        let Ok(contents) = std::str::from_utf8(&entry.content) else {
            self.emit(
                context,
                ADR_INDEX_RULE,
                Some(location(path, Some(1))),
                "ADR index must contain UTF-8 Markdown".to_owned(),
            )?;
            return Ok(());
        };
        let links = markdown_adr_links(contents);
        let index_parent = path.parent().unwrap_or_else(|| Path::new(""));
        let records = context.adrs.values().cloned().collect::<Vec<_>>();
        for record in records {
            let matching = links
                .iter()
                .filter(|(id, _, _)| id == &record.metadata.id)
                .collect::<Vec<_>>();
            let expected = record
                .path
                .strip_prefix(index_parent)
                .unwrap_or(&record.path);
            if matching.len() != 1 || Path::new(&matching[0].1) != expected {
                self.emit(
                    context,
                    ADR_INDEX_RULE,
                    Some(location(path, matching.first().map_or(Some(1), |link| Some(link.2)))),
                    format!(
                        "ADR index must link {} exactly once to {}",
                        record.metadata.id,
                        portable_path(expected)
                    ),
                )?;
            }
        }
        for (id, _, line) in links {
            if !context.adrs.contains_key(&id) {
                self.emit(
                    context,
                    ADR_INDEX_RULE,
                    Some(location(path, Some(line))),
                    format!("ADR index contains dangling canonical entry {id}"),
                )?;
            }
        }
        Ok(())
    }

    fn validate_adr_references(&self, context: &mut EvaluationContext) -> Result<()> {
        let records = context.adrs.values().cloned().collect::<Vec<_>>();
        let local_ids = context.adrs.keys().cloned().collect::<BTreeSet<_>>();
        let external = self
            .policy
            .adrs
            .known_external_decisions
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for record in &records {
            for reference in record
                .metadata
                .related
                .iter()
                .chain(&record.metadata.supersedes)
                .chain(&record.metadata.superseded_by)
            {
                let resolved = if valid_adr_id(reference) {
                    local_ids.contains(reference)
                } else {
                    valid_external_decision_ref(reference) && external.contains(reference)
                };
                if !resolved {
                    self.emit(
                        context,
                        ADR_LINEAGE_RULE,
                        Some(location(&record.path, Some(record.line))),
                        format!(
                            "ADR {} contains dangling or undeclared decision reference {reference}",
                            record.metadata.id
                        ),
                    )?;
                }
                if reference == &record.metadata.id {
                    self.emit(
                        context,
                        ADR_LINEAGE_RULE,
                        Some(location(&record.path, Some(record.line))),
                        format!("ADR {} may not reference itself in lineage", record.metadata.id),
                    )?;
                }
            }
        }
        for replacement in &records {
            for old_id in &replacement.metadata.supersedes {
                if !valid_adr_id(old_id) {
                    continue;
                }
                let Some(old) = context.adrs.get(old_id) else {
                    continue;
                };
                if replacement.metadata.status == "accepted"
                    && (old.metadata.status != "superseded"
                        || !old
                            .metadata
                            .superseded_by
                            .contains(&replacement.metadata.id))
                {
                    self.emit(
                        context,
                        ADR_LINEAGE_RULE,
                        Some(location(&replacement.path, Some(replacement.line))),
                        format!(
                            "accepted replacement {} is not linked back from superseded {old_id}",
                            replacement.metadata.id
                        ),
                    )?;
                }
            }
            for replacement_id in &replacement.metadata.superseded_by {
                if !valid_adr_id(replacement_id) {
                    continue;
                }
                let Some(newer) = context.adrs.get(replacement_id) else {
                    continue;
                };
                if newer.metadata.status != "accepted"
                    || !newer.metadata.supersedes.contains(&replacement.metadata.id)
                {
                    self.emit(
                        context,
                        ADR_LINEAGE_RULE,
                        Some(location(&replacement.path, Some(replacement.line))),
                        format!(
                            "superseded ADR {} is not reciprocally linked to accepted {replacement_id}",
                            replacement.metadata.id
                        ),
                    )?;
                }
            }
        }
        let graph = records
            .iter()
            .map(|record| {
                (
                    record.metadata.id.clone(),
                    record
                        .metadata
                        .supersedes
                        .iter()
                        .filter(|reference| valid_adr_id(reference))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for cycle in graph_cycles(&graph) {
            let path = context
                .adrs
                .get(&cycle[0])
                .map_or_else(|| self.policy.adrs.index.clone(), |record| record.path.clone());
            self.emit(
                context,
                CYCLE_RULE,
                Some(location(&path, None)),
                format!("ADR supersession cycle detected: {}", cycle.join(" -> ")),
            )?;
        }
        Ok(())
    }

    fn validate_required_extensions(
        &self,
        reference: &PolicyReference,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let required = reference
            .extensions
            .iter()
            .filter(|extension| extension.required && extension.kind == "metadata")
            .map(|extension| extension.id.as_str())
            .collect::<Vec<_>>();
        let records = context.adrs.values().cloned().collect::<Vec<_>>();
        for record in records {
            for extension in &required {
                if !record.metadata.extensions.contains_key(*extension) {
                    self.emit(
                        context,
                        ADR_METADATA_RULE,
                        Some(location(&record.path, Some(record.line))),
                        format!(
                            "ADR {} is missing required registered extension {extension}",
                            record.metadata.id
                        ),
                    )?;
                }
            }
            for extension in record.metadata.extensions.keys() {
                if !reference
                    .extensions
                    .iter()
                    .any(|registered| &registered.id == extension)
                {
                    self.emit(
                        context,
                        ADR_METADATA_RULE,
                        Some(location(&record.path, Some(record.line))),
                        format!(
                            "ADR {} uses unregistered extension {extension}",
                            record.metadata.id
                        ),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn validate_policy_exception(
        &self,
        exception: &PolicyException,
        path: &Path,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        if exception.rule.trim().is_empty()
            || exception.reason.trim().is_empty()
            || exception.owner.trim().is_empty()
            || !matches!(exception.status.as_str(), "proposed" | "approved" | "expired")
            || exception
                .expires
                .as_deref()
                .is_some_and(|date| !valid_date(date))
            || (exception.status == "approved"
                && exception
                    .approval_evidence
                    .as_deref()
                    .is_none_or(|url| !valid_http_url(url)))
        {
            self.emit(
                context,
                ADR_LIFECYCLE_RULE,
                Some(location(path, Some(1))),
                "ADR policy-reference contains a malformed exception".to_owned(),
            )?;
        }
        Ok(())
    }
}

impl RepositoryIntelligenceEvaluator<'_> {
    fn validate_commit_trailers(
        &self,
        history: &CommitHistory,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let local_adrs = context.adrs.keys().cloned().collect::<BTreeSet<_>>();
        let external_adrs = self
            .policy
            .adrs
            .known_external_decisions
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let local_steps = context
            .roadmap_steps
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let external_steps = self
            .policy
            .roadmap
            .known_external_steps
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for commit in &history.records {
            if !valid_commit(&commit.sha) {
                self.emit(
                    context,
                    TRAILER_RULE,
                    Some(commit_location(&commit.sha, 1)),
                    "commit history contains a malformed commit identity".to_owned(),
                )?;
                continue;
            }
            for (line_index, line) in commit.message.lines().enumerate() {
                let line_number = u32::try_from(line_index).unwrap_or(u32::MAX).saturating_add(1);
                let trimmed = line.trim();
                if trimmed.starts_with("Roadmap-Step")
                    && !trimmed.starts_with("Roadmap-Step:")
                {
                    self.emit(
                        context,
                        TRAILER_RULE,
                        Some(commit_location(&commit.sha, line_number)),
                        format!("commit {} contains a malformed Roadmap-Step trailer", commit.sha),
                    )?;
                    continue;
                }
                if trimmed.starts_with("ADR-Ref") && !trimmed.starts_with("ADR-Ref:") {
                    self.emit(
                        context,
                        TRAILER_RULE,
                        Some(commit_location(&commit.sha, line_number)),
                        format!("commit {} contains a malformed ADR-Ref trailer", commit.sha),
                    )?;
                    continue;
                }
                if let Some(value) = trimmed.strip_prefix("Roadmap-Step:") {
                    let value = value.trim();
                    if value.is_empty()
                        || value.contains(',')
                        || !valid_step_id(value)
                        || (self.policy.roadmap.state == AdoptionState::Present
                            && !local_steps.contains(value)
                            && !external_steps.contains(value))
                    {
                        self.emit(
                            context,
                            TRAILER_RULE,
                            Some(commit_location(&commit.sha, line_number)),
                            format!(
                                "commit {} Roadmap-Step trailer does not resolve: {value}",
                                commit.sha
                            ),
                        )?;
                    }
                }
                if let Some(value) = trimmed.strip_prefix("ADR-Ref:") {
                    let value = value.trim();
                    let structurally_valid = valid_adr_id(value)
                        || valid_external_decision_ref(value);
                    if value.is_empty()
                        || value.contains(',')
                        || !structurally_valid
                        || (self.policy.adrs.state == AdoptionState::Present
                            && !local_adrs.contains(value)
                            && !external_adrs.contains(value))
                    {
                        self.emit(
                            context,
                            TRAILER_RULE,
                            Some(commit_location(&commit.sha, line_number)),
                            format!(
                                "commit {} ADR-Ref trailer does not resolve: {value}",
                                commit.sha
                            ),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CommentBlock {
    contents: String,
    line: u32,
    start: usize,
    end: usize,
}

fn html_comment_blocks(contents: &str, name: &str) -> Vec<CommentBlock> {
    let opening = format!("<!-- {name}");
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = contents[cursor..].find(&opening) {
        let start = cursor + relative_start;
        let body_start = start + opening.len();
        let Some(relative_end) = contents[body_start..].find("-->") else {
            break;
        };
        let end = body_start + relative_end + 3;
        blocks.push(CommentBlock {
            contents: contents[body_start..body_start + relative_end]
                .trim()
                .to_owned(),
            line: line_number(contents, start),
            start,
            end,
        });
        cursor = end;
    }
    blocks
}

fn trim_ascii_whitespace(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(u8::is_ascii_whitespace) {
        value = &value[1..];
    }
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value = &value[..value.len() - 1];
    }
    value
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

fn markdown_adr_links(contents: &str) -> Vec<(String, String, u32)> {
    let mut links = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let mut cursor = 0;
        while let Some(relative) = line[cursor..].find("[ADR-") {
            let start = cursor + relative + 1;
            let Some(close_label_relative) = line[start..].find("](") else {
                break;
            };
            let close_label = start + close_label_relative;
            let target_start = close_label + 2;
            let Some(close_target_relative) = line[target_start..].find(')') else {
                break;
            };
            let close_target = target_start + close_target_relative;
            let id = &line[start..close_label];
            if valid_adr_id(id) {
                links.push((
                    id.to_owned(),
                    line[target_start..close_target].to_owned(),
                    u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                ));
            }
            cursor = close_target.saturating_add(1);
        }
    }
    links
}

fn graph_cycles(graph: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        visiting: &mut Vec<String>,
        complete: &mut BTreeSet<String>,
        cycles: &mut BTreeSet<Vec<String>>,
    ) {
        if let Some(position) = visiting.iter().position(|candidate| candidate == node) {
            let mut cycle = visiting[position..].to_vec();
            cycle.push(node.to_owned());
            if cycle.len() > 2 {
                let body = &cycle[..cycle.len() - 1];
                let minimum = body
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, value)| *value)
                    .map_or(0, |(index, _)| index);
                let mut canonical = body[minimum..]
                    .iter()
                    .chain(&body[..minimum])
                    .cloned()
                    .collect::<Vec<_>>();
                canonical.push(canonical[0].clone());
                cycles.insert(canonical);
            }
            return;
        }
        if complete.contains(node) {
            return;
        }
        visiting.push(node.to_owned());
        if let Some(edges) = graph.get(node) {
            for edge in edges {
                if graph.contains_key(edge) {
                    visit(edge, graph, visiting, complete, cycles);
                }
            }
        }
        visiting.pop();
        complete.insert(node.to_owned());
    }

    let mut complete = BTreeSet::new();
    let mut cycles = BTreeSet::new();
    for node in graph.keys() {
        visit(node, graph, &mut Vec::new(), &mut complete, &mut cycles);
    }
    cycles.into_iter().collect()
}

fn valid_repository(value: &str) -> bool {
    let Some(repository) = value.strip_prefix("egohygiene/") else {
        return false;
    };
    repository == ".github"
        || (!repository.is_empty()
            && repository
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
            && repository.as_bytes()[0].is_ascii_alphanumeric())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_adr_id(value: &str) -> bool {
    let Some(number) = value.strip_prefix("ADR-") else {
        return false;
    };
    matches!(number.len(), 3 | 4) && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_external_decision_ref(value: &str) -> bool {
    let Some((repository, id)) = value.split_once('#') else {
        return false;
    };
    repository
        .split_once('/')
        .is_some_and(|(owner, name)| {
            !owner.is_empty()
                && !name.is_empty()
                && owner
                    .bytes()
                    .chain(name.bytes())
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        })
        && valid_adr_id(id)
}

fn valid_contract_id(value: &str) -> bool {
    let Some((name, version)) = value.rsplit_once("/v") else {
        return false;
    };
    name.starts_with("egohygiene.")
        && name.len() > "egohygiene.".len()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        && !version.is_empty()
        && version.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_step_id(value: &str) -> bool {
    value.len() >= 5
        && value.contains('-')
        && value.as_bytes()[0].is_ascii_uppercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.ends_with('-')
        && !value.contains("--")
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
    day > 0 && day <= maximum
}

fn valid_http_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(|character| character.is_control() || character.is_whitespace())
        && value.split_once("://").is_some_and(|(_, rest)| rest.contains('.'))
}

fn valid_github_number_url(value: &str, segment: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://github.com/") else {
        return false;
    };
    let Some((repository, number)) = rest.split_once(segment) else {
        return false;
    };
    repository.split('/').count() == 2
        && !repository.contains(char::is_whitespace)
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && number != "0"
}

fn valid_github_commit_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://github.com/") else {
        return false;
    };
    let Some((repository, revision)) = rest.split_once("/commit/") else {
        return false;
    };
    repository.split('/').count() == 2 && valid_commit(revision)
}

fn valid_route(value: &str) -> bool {
    value.starts_with('/')
        && value.ends_with('/')
        && !value.contains("..")
        && !value.contains(char::is_whitespace)
}

fn valid_issue_reference(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::Number(number) => number.as_u64().is_some_and(|number| number > 0),
        serde_yaml::Value::String(reference) => {
            valid_github_number_url(reference, "/issues/")
                || reference.split_once('#').is_some_and(|(repository, number)| {
                    repository.split('/').count() == 2
                        && !repository.is_empty()
                        && !number.is_empty()
                        && number.bytes().all(|byte| byte.is_ascii_digit())
                        && number != "0"
                })
        }
        _ => false,
    }
}

fn all_unique_nonempty(values: &[String]) -> bool {
    values.iter().all(|value| !value.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn ensure_unique<'a>(values: impl Iterator<Item = &'a str>, name: &str) -> Result<()> {
    let mut observed = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() || !observed.insert(value) {
            return Err(EgolintError::Configuration(format!(
                "{name} declarations must be nonempty and unique"
            )));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path, name: &str) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.to_str().is_none()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a normalized workspace-relative Unicode path"
        )));
    }
    Ok(())
}

fn line_number(contents: &str, offset: usize) -> u32 {
    u32::try_from(contents[..offset].bytes().filter(|byte| *byte == b'\n').count())
        .unwrap_or(u32::MAX)
        .saturating_add(1)
}

fn location(path: &Path, line: Option<u32>) -> SourceLocation {
    SourceLocation {
        path: path.to_path_buf(),
        start_line: line,
        start_column: line.map(|_| 1),
        end_line: None,
        end_column: None,
    }
}

fn commit_location(sha: &str, line: u32) -> SourceLocation {
    location(Path::new("@git").join(sha).as_path(), Some(line))
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn stable_fingerprint(
    rule_id: &str,
    location: Option<&SourceLocation>,
    message: &str,
) -> String {
    let path = location.map_or_else(String::new, |location| portable_path(&location.path));
    let line = location.and_then(|location| location.start_line).unwrap_or(0);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in rule_id
        .bytes()
        .chain([0])
        .chain(path.bytes())
        .chain([0])
        .chain(line.to_string().bytes())
        .chain([0])
        .chain(message.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("repository-intelligence-v1-{hash:016x}")
}

fn diagnostic_order(
    left: &IntelligenceDiagnostic,
    right: &IntelligenceDiagnostic,
) -> std::cmp::Ordering {
    let left_path = left
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    let right_path = right
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    left_path
        .cmp(right_path)
        .then_with(|| {
            left.location
                .as_ref()
                .and_then(|location| location.start_line)
                .cmp(
                    &right
                        .location
                        .as_ref()
                        .and_then(|location| location.start_line),
                )
        })
        .then_with(|| left.rule_id.cmp(&right.rule_id))
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RepositoryEntry;

    const POLICY: &str =
        include_str!("../../tests/fixtures/repository-intelligence/valid/policy.toml");
    const POLICY_REFERENCE: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/valid/docs/decisions/policy-reference.json"
    );
    const VALID_INDEX: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/valid/docs/decisions/README.md"
    );
    const VALID_ADR: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/valid/docs/decisions/ADR-001-validation-contract.md"
    );
    const VALID_ADR_TWO: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/valid/docs/decisions/ADR-002-legacy-projection.md"
    );
    const VALID_ADR_THREE: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/valid/docs/decisions/ADR-003-versioned-projection.md"
    );
    const VALID_ROADMAP: &str =
        include_str!("../../tests/fixtures/repository-intelligence/valid/ROADMAP.md");
    const HOSTILE_INDEX: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/hostile/docs/decisions/README.md"
    );
    const HOSTILE_ADR_ONE: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/hostile/docs/decisions/ADR-001-first.md"
    );
    const HOSTILE_ADR_TWO: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/hostile/docs/decisions/ADR-002-second.md"
    );
    const HOSTILE_ADR_DUPLICATE: &str = include_str!(
        "../../tests/fixtures/repository-intelligence/hostile/docs/decisions/ADR-003-duplicate.md"
    );
    const HOSTILE_ROADMAP: &str =
        include_str!("../../tests/fixtures/repository-intelligence/hostile/ROADMAP.md");

    fn policy() -> RepositoryIntelligencePolicy {
        RepositoryIntelligencePolicy::from_toml(
            POLICY,
            Path::new("tests/fixtures/repository-intelligence/valid/policy.toml"),
        )
        .expect("valid repository-intelligence policy")
    }

    fn entry(path: &str, contents: &str) -> RepositoryEntry {
        RepositoryEntry::file(path, Some(100_644), contents.as_bytes().to_vec())
    }

    fn valid_inventory() -> RepositoryInventory {
        RepositoryInventory::from_entries(vec![
            entry("ROADMAP.md", VALID_ROADMAP),
            entry("docs/decisions/ADR-001-validation-contract.md", VALID_ADR),
            entry("docs/decisions/ADR-002-legacy-projection.md", VALID_ADR_TWO),
            entry("docs/decisions/ADR-003-versioned-projection.md", VALID_ADR_THREE),
            entry("docs/decisions/README.md", VALID_INDEX),
            entry(
                "docs/decisions/policy-reference.json",
                POLICY_REFERENCE,
            ),
        ])
        .expect("valid fixture inventory")
    }

    fn represented() -> RepresentedCommit {
        RepresentedCommit::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("represented commit")
    }

    #[test]
    fn positive_adr_roadmap_and_trailer_fixture_is_clean() {
        let policy = policy();
        let evaluator = RepositoryIntelligenceEvaluator::new(
            &policy,
            Path::new(".config/egolint/repository-intelligence.toml"),
            represented(),
        )
        .expect("evaluator");
        let history = CommitHistory {
            records: vec![CommitRecord {
                sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                message: "feat(intelligence): validate linkage\n\nRoadmap-Step: EGL-RI-002\nADR-Ref: ADR-001\n"
                    .to_owned(),
            }],
            truncated: false,
        };

        let evaluation = evaluator
            .evaluate(&valid_inventory(), &history)
            .expect("semantic evaluation");

        assert!(evaluation.findings.is_empty());
        assert_eq!(evaluation.report.status, IntelligenceValidationStatus::Valid);
        assert_eq!(evaluation.report.summary.adrs_inspected, 3);
        assert_eq!(evaluation.report.summary.roadmap_steps_inspected, 2);
        assert_eq!(evaluation.report.summary.commits_inspected, 1);
    }

    #[test]
    fn hostile_fixtures_report_lifecycle_links_states_trailers_and_cycles() {
        let policy = policy();
        let evaluator = RepositoryIntelligenceEvaluator::new(
            &policy,
            Path::new(".config/egolint/repository-intelligence.toml"),
            represented(),
        )
        .expect("evaluator");
        let inventory = RepositoryInventory::from_entries(vec![
            entry("ROADMAP.md", HOSTILE_ROADMAP),
            entry("docs/decisions/ADR-001-first.md", HOSTILE_ADR_ONE),
            entry("docs/decisions/ADR-002-second.md", HOSTILE_ADR_TWO),
            entry(
                "docs/decisions/ADR-003-duplicate.md",
                HOSTILE_ADR_DUPLICATE,
            ),
            entry("docs/decisions/README.md", HOSTILE_INDEX),
            entry(
                "docs/decisions/policy-reference.json",
                POLICY_REFERENCE,
            ),
        ])
        .expect("hostile fixture inventory");
        let history = CommitHistory {
            records: vec![CommitRecord {
                sha: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
                message: "feat: hostile trailer\n\nRoadmap-Step MISSING\nADR-Ref: ADR-999\n"
                    .to_owned(),
            }],
            truncated: false,
        };

        let evaluation = evaluator
            .evaluate(&inventory, &history)
            .expect("semantic evaluation");
        let rules = evaluation
            .report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id.as_str())
            .collect::<BTreeSet<_>>();

        for expected in [
            ADR_METADATA_RULE,
            ADR_LIFECYCLE_RULE,
            ADR_INDEX_RULE,
            ADR_LINEAGE_RULE,
            ROADMAP_STRUCTURE_RULE,
            ROADMAP_STATE_RULE,
            LINK_RULE,
            TRAILER_RULE,
            CYCLE_RULE,
        ] {
            assert!(rules.contains(expected), "missing hostile rule {expected}");
        }
        assert_eq!(evaluation.report.status, IntelligenceValidationStatus::Invalid);
        assert!(evaluation.report.summary.blocking_diagnostics > 0);
        assert!(evaluation.report.diagnostics.iter().all(|diagnostic| {
            !diagnostic.remediation.is_empty() && diagnostic.location.is_some()
        }));
    }

    #[test]
    fn advisory_and_unknown_adoption_never_claim_false_conformance() {
        let mut policy = policy();
        policy.profile.enforcement = IntelligenceEnforcement::Advisory;
        policy.adrs.state = AdoptionState::Unknown;
        policy.roadmap.state = AdoptionState::NotApplicable;
        policy.commit_history.state = AdoptionState::Unknown;
        let evaluator = RepositoryIntelligenceEvaluator::new(
            &policy,
            Path::new(".config/egolint/repository-intelligence.toml"),
            RepresentedCommit::parse("unknown").expect("unknown source"),
        )
        .expect("evaluator");

        let evaluation = evaluator
            .evaluate(&RepositoryInventory::default(), &CommitHistory {
                records: Vec::new(),
                truncated: false,
            })
            .expect("semantic evaluation");

        assert_eq!(evaluation.report.status, IntelligenceValidationStatus::Incomplete);
        assert!(
            evaluation
                .findings
                .iter()
                .all(|finding| finding.severity == Severity::Warning)
        );
        assert_eq!(evaluation.report.summary.blocking_diagnostics, 0);
    }

    #[test]
    fn profile_only_evaluates_enabled_rules() {
        let mut policy = policy();
        policy.profile.enabled_rules = vec![ADOPTION_RULE.to_owned()];
        policy.adrs.state = AdoptionState::Unknown;
        policy.roadmap.state = AdoptionState::NotApplicable;
        policy.commit_history.state = AdoptionState::NotApplicable;
        let evaluator = RepositoryIntelligenceEvaluator::new(
            &policy,
            Path::new("policy.toml"),
            RepresentedCommit::parse("unknown").expect("unknown source"),
        )
        .expect("evaluator");

        let evaluation = evaluator
            .evaluate(&RepositoryInventory::default(), &CommitHistory {
                records: Vec::new(),
                truncated: false,
            })
            .expect("semantic evaluation");

        assert!(!evaluation.report.diagnostics.is_empty());
        assert!(
            evaluation
                .report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule_id == ADOPTION_RULE)
        );
    }

    #[test]
    fn policy_schema_pins_version_one() {
        let schema = serde_json::to_value(schemars::schema_for!(RepositoryIntelligencePolicy))
            .expect("repository-intelligence policy schema");

        assert_eq!(schema["properties"]["schema-version"]["const"], 1);
    }
}


#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoadmapManifest {
    schema: String,
    repository: String,
    visibility: String,
    publication: String,
    route: String,
    updated: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoadmapStepMetadata {
    id: String,
    status: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    issues: Vec<serde_yaml::Value>,
}

#[derive(Debug, Clone)]
struct RoadmapStep {
    path: PathBuf,
    line: u32,
    metadata: RoadmapStepMetadata,
    checklist_total: u32,
    checklist_complete: u32,
}

impl RepositoryIntelligenceEvaluator<'_> {
    fn validate_roadmap(
        &self,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let path = &self.policy.roadmap.path;
        let Some(entry) = inventory.get(path) else {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, None)),
                "declared roadmap file is missing".to_owned(),
            )?;
            return Ok(());
        };
        if entry.kind != RepositoryEntryKind::File {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, None)),
                "roadmap must be a regular file".to_owned(),
            )?;
            return Ok(());
        }
        let Ok(contents) = std::str::from_utf8(&entry.content) else {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(1))),
                "roadmap must contain UTF-8 Markdown".to_owned(),
            )?;
            return Ok(());
        };
        self.validate_roadmap_manifest(path, contents, context)?;
        let blocks = html_comment_blocks(contents, "roadmap-step");
        if blocks.is_empty() {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(1))),
                "roadmap contains no versioned roadmap-step records".to_owned(),
            )?;
            return Ok(());
        }
        for (index, block) in blocks.iter().enumerate() {
            let metadata = match serde_yaml::from_str::<RoadmapStepMetadata>(&block.contents) {
                Ok(metadata) => metadata,
                Err(error) => {
                    self.emit(
                        context,
                        ROADMAP_STRUCTURE_RULE,
                        Some(location(path, Some(block.line))),
                        format!("roadmap step metadata is invalid: {error}"),
                    )?;
                    continue;
                }
            };
            let segment_end = blocks
                .get(index + 1)
                .map_or(contents.len(), |next| next.start);
            let segment = &contents[block.end..segment_end];
            let step = self.validate_roadmap_step(path, block.line, metadata, segment, context)?;
            let id = step.metadata.id.clone();
            if let Some(previous) = context.roadmap_steps.insert(id.clone(), step) {
                self.emit(
                    context,
                    ROADMAP_STRUCTURE_RULE,
                    Some(location(path, Some(block.line))),
                    format!(
                        "duplicate roadmap step id {id}; first declaration is on line {}",
                        previous.line
                    ),
                )?;
            }
        }
        self.validate_roadmap_graph(context)
    }

    fn validate_roadmap_manifest(
        &self,
        path: &Path,
        contents: &str,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let blocks = html_comment_blocks(contents, "roadmap-manifest");
        if blocks.len() != 1 {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(1))),
                format!(
                    "roadmap must contain exactly one roadmap-manifest; observed {}",
                    blocks.len()
                ),
            )?;
            return Ok(());
        }
        let block = &blocks[0];
        let manifest = match serde_yaml::from_str::<RoadmapManifest>(&block.contents) {
            Ok(manifest) => manifest,
            Err(error) => {
                self.emit(
                    context,
                    ROADMAP_STRUCTURE_RULE,
                    Some(location(path, Some(block.line))),
                    format!("roadmap manifest is invalid: {error}"),
                )?;
                return Ok(());
            }
        };
        if manifest.schema != ROADMAP_CONTRACT
            || manifest.repository != self.policy.repository
            || !matches!(manifest.visibility.as_str(), "public" | "internal" | "private")
            || !matches!(
                manifest.publication.as_str(),
                "canonical" | "composed" | "central" | "artifact-only" | "disabled"
            )
            || !valid_route(&manifest.route)
            || !valid_date(&manifest.updated)
        {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(block.line))),
                "roadmap manifest does not match the v1alpha1 repository contract".to_owned(),
            )?;
        }
        Ok(())
    }

    fn validate_roadmap_step(
        &self,
        path: &Path,
        line: u32,
        metadata: RoadmapStepMetadata,
        segment: &str,
        context: &mut EvaluationContext,
    ) -> Result<RoadmapStep> {
        if !valid_step_id(&metadata.id)
            || !matches!(
                metadata.status.as_str(),
                "complete"
                    | "active"
                    | "ready"
                    | "blocked"
                    | "planned"
                    | "deferred"
                    | "cancelled"
            )
            || !all_unique_nonempty(&metadata.depends_on)
        {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(line))),
                format!("roadmap step {} has malformed identity, state, or dependencies", metadata.id),
            )?;
        }
        let heading_matches = segment.lines().any(|candidate| {
            let trimmed = candidate.trim_start();
            let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
            (3..=6).contains(&hashes)
                && trimmed[hashes..]
                    .trim_start()
                    .starts_with(&format!("{} ", metadata.id))
        });
        if !heading_matches {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(line))),
                format!("roadmap step {} is missing its matching renderable heading", metadata.id),
            )?;
        }
        let outcome = segment
            .lines()
            .find_map(|candidate| candidate.strip_prefix("**Outcome:**"))
            .map(str::trim);
        if outcome.is_none_or(str::is_empty) {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(line))),
                format!("roadmap step {} needs one nonempty Outcome", metadata.id),
            )?;
        }
        let mut checklist_total = 0_u32;
        let mut checklist_complete = 0_u32;
        for candidate in segment.lines() {
            let trimmed = candidate.trim_start();
            if trimmed.starts_with("- [ ] ") {
                checklist_total += 1;
            } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                checklist_total += 1;
                checklist_complete += 1;
            }
        }
        if checklist_total == 0 || !segment.contains("**Exit criteria:**") {
            self.emit(
                context,
                ROADMAP_STRUCTURE_RULE,
                Some(location(path, Some(line))),
                format!("roadmap step {} needs checklist exit criteria", metadata.id),
            )?;
        }
        if let Some(body_state) = segment
            .lines()
            .find_map(|candidate| candidate.strip_prefix("**State:**"))
            .map(|state| state.trim().trim_matches('`'))
        {
            if !body_state.eq_ignore_ascii_case(&metadata.status) {
                self.emit(
                    context,
                    ROADMAP_STATE_RULE,
                    Some(location(path, Some(line))),
                    format!(
                        "roadmap step {} metadata state {} disagrees with body state {body_state}",
                        metadata.id, metadata.status
                    ),
                )?;
            }
        }
        if metadata.status == "complete" && checklist_complete != checklist_total {
            self.emit(
                context,
                ROADMAP_STATE_RULE,
                Some(location(path, Some(line))),
                format!(
                    "complete roadmap step {} has {}/{} checked exit criteria",
                    metadata.id, checklist_complete, checklist_total
                ),
            )?;
        }
        if metadata.status != "complete"
            && checklist_total > 0
            && checklist_complete == checklist_total
        {
            self.emit(
                context,
                ROADMAP_STATE_RULE,
                Some(location(path, Some(line))),
                format!(
                    "roadmap step {} has every exit criterion checked but state {}",
                    metadata.id, metadata.status
                ),
            )?;
        }
        for issue in &metadata.issues {
            if !valid_issue_reference(issue) {
                self.emit(
                    context,
                    LINK_RULE,
                    Some(location(path, Some(line))),
                    format!("roadmap step {} contains a malformed issue reference", metadata.id),
                )?;
            }
        }
        Ok(RoadmapStep {
            path: path.to_path_buf(),
            line,
            metadata,
            checklist_total,
            checklist_complete,
        })
    }

    fn validate_roadmap_graph(&self, context: &mut EvaluationContext) -> Result<()> {
        let local = context
            .roadmap_steps
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let external = self
            .policy
            .roadmap
            .known_external_steps
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let steps = context.roadmap_steps.values().cloned().collect::<Vec<_>>();
        for step in &steps {
            for dependency in &step.metadata.depends_on {
                if dependency == &step.metadata.id {
                    self.emit(
                        context,
                        CYCLE_RULE,
                        Some(location(&step.path, Some(step.line))),
                        format!("roadmap step {} depends on itself", step.metadata.id),
                    )?;
                } else if !local.contains(dependency) && !external.contains(dependency) {
                    self.emit(
                        context,
                        LINK_RULE,
                        Some(location(&step.path, Some(step.line))),
                        format!(
                            "roadmap step {} has dangling or undeclared dependency {dependency}",
                            step.metadata.id
                        ),
                    )?;
                }
            }
            if matches!(step.metadata.status.as_str(), "active" | "ready") {
                for dependency in &step.metadata.depends_on {
                    if let Some(local_dependency) = context.roadmap_steps.get(dependency) {
                        if local_dependency.metadata.status != "complete" {
                            self.emit(
                                context,
                                ROADMAP_STATE_RULE,
                                Some(location(&step.path, Some(step.line))),
                                format!(
                                    "{} roadmap step {} depends on incomplete {}",
                                    step.metadata.status, step.metadata.id, dependency
                                ),
                            )?;
                        }
                    }
                }
            }
            debug_assert!(step.checklist_complete <= step.checklist_total);
        }
        let graph = steps
            .iter()
            .map(|step| {
                (
                    step.metadata.id.clone(),
                    step.metadata
                        .depends_on
                        .iter()
                        .filter(|dependency| local.contains(*dependency))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for cycle in graph_cycles(&graph) {
            let step = context
                .roadmap_steps
                .get(&cycle[0])
                .expect("cycle nodes come from roadmap graph");
            let path = step.path.clone();
            let line = step.line;
            self.emit(
                context,
                CYCLE_RULE,
                Some(location(&path, Some(line))),
                format!("roadmap dependency cycle detected: {}", cycle.join(" -> ")),
            )?;
        }
        Ok(())
    }
}
