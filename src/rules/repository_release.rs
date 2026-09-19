//! Offline repository-release applicability and focused evidence reporting.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

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

mod checks;

/// Stable tool identifier used by the universal native capability.
pub const TOOL_ID: &str = "EGOLINT_REPOSITORY_RELEASE";
/// Dedicated repository-release evidence artifact.
pub const REPORT_PATH: &str = ".reports/egolint/repository-release.json";
/// Repository-owned declaration selected by the accepted Hygiene policy.
pub const DECLARATION_PATH: &str = ".egohygiene/release.json";

const REPORT_CONTRACT: &str = "egolint.repository-release-report/v1";
const SOURCE_CONTRACT: &str = "egolint.repository-release-sources/v1";
const HYGIENE_POLICY_ID: &str = "egohygiene.repository-release-policy/v1";
const HYGIENE_POLICY_VERSION: &str = "1.0.0-alpha.1";
const AETHER_CONTRACT_ID: &str = "egohygiene.repository-release/v1";
const AETHER_CONTRACT_VERSION: &str = "1.0.0";
const AETHER_SCHEMA_URL: &str = "https://egohygiene.io/schemas/aether/repository-release/v1.json";
const AETHER_SOURCE_REVISION: &str = "8a2a3d08f3aa9da3847bd5277843506ab855192e";
const HYGIENE_SOURCE_REVISION: &str = "28f9d6c7519d820644572634ba4476614f418d83";
const SOURCE_LOCK: &str = include_str!("../../.config/rules/repository-release-sources.v1.json");
const AETHER_SCHEMA: &str =
    include_str!("../../vendor/aether/aether.repository-release.v1.schema.json");
const HYGIENE_POLICY: &str = include_str!("../../vendor/hygiene/repository-release-policy.v1.json");
const MAXIMUM_DECLARATION_BYTES: usize = 4 * 1024 * 1024;

const POLICY_PATH: &str = "vendor/hygiene/repository-release-policy.v1.json";

const DECLARATION_RULE: &str = "EGOLINT_RELEASE_AETHER_DECLARATION";

/// Repository profile named by the Aether declaration and Hygiene policy.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryReleaseProfile {
    CliLibrary,
    ContainerImage,
    Contract,
    InternalOnly,
    NpmPackage,
    Publication,
    PythonPackage,
    StaticSite,
    Workspace,
}

/// Repository lifecycle named by the Aether declaration and Hygiene policy.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseLifecycle {
    Active,
    Incubating,
    Internal,
    Archived,
}

/// Repository-wide adoption state selected by policy or an authorized planner.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
    ValueEnum,
)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum ReleaseAdoptionState {
    Required,
    Advisory,
    Exempt,
    NotApplicable,
}

/// Effective applicability of one Hygiene policy slot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRequirement {
    Required,
    Advisory,
    NotApplicable,
}

/// Stable focused-report state. These values never assert publication success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseEvidenceState {
    Compliant,
    Advisory,
    Unavailable,
    External,
    Invalid,
    NotApplicable,
}

impl ReleaseEvidenceState {
    /// Stable human-readable spelling shared with serialized reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compliant => "compliant",
            Self::Advisory => "advisory",
            Self::Unavailable => "unavailable",
            Self::External => "external",
            Self::Invalid => "invalid",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Whether repository facts were selected explicitly or derived from policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseApplicabilitySource {
    Explicit,
    LifecyclePolicy,
    Unavailable,
}

/// Structural state of the repository-owned Aether declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseDeclarationState {
    Present,
    Missing,
    Invalid,
}

/// Immutable source identity recorded in the accepted source lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSourceLock {
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub source_path: PathBuf,
    pub vendored_path: PathBuf,
    pub git_blob_sha1: String,
    pub sha256: String,
}

/// Exact accepted Hygiene profile and its Aether contract binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleasePolicyReference {
    pub id: String,
    pub version: String,
    pub status: String,
    pub owner: String,
    pub hygiene_source: ReleaseSourceLock,
    pub aether_contract_id: String,
    pub aether_contract_version: String,
    pub aether_schema_url: String,
    pub aether_source: ReleaseSourceLock,
}

/// Privacy-safe identity and state of the local declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseDeclarationReference {
    pub path: PathBuf,
    pub state: ReleaseDeclarationState,
    pub sha256: Option<String>,
}

/// Effective applicability for one versioned Hygiene slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseSlotApplicability {
    pub id: String,
    pub authority: String,
    pub requirement: ReleaseRequirement,
}

/// Result of one source-pinned Hygiene release-policy slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseCheckState {
    Passed,
    Failed,
    Unavailable,
    External,
    NotApplicable,
}

/// Stable native evidence for one repository-release check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseCheckResult {
    pub slot_id: String,
    pub rule_id: String,
    pub requirement: ReleaseRequirement,
    pub authority: String,
    pub state: ReleaseCheckState,
    pub message: String,
    pub remediation: String,
    pub location: SourceLocation,
    pub evidence: Vec<EvidenceReference>,
}

/// Deterministic applicability result consumed by later release checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseApplicability {
    pub repository_profile: Option<RepositoryReleaseProfile>,
    pub lifecycle: Option<ReleaseLifecycle>,
    pub adoption_state: Option<ReleaseAdoptionState>,
    pub source: ReleaseApplicabilitySource,
    pub slots: Vec<ReleaseSlotApplicability>,
}

/// Bounded counts explaining the focused state without retaining file payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseEvaluationSummary {
    pub required_slots: u64,
    pub advisory_slots: u64,
    pub not_applicable_slots: u64,
    pub validation_complete: bool,
    pub checks_completed: u64,
    pub checks_failed: u64,
    pub external_evidence_items: u64,
    pub unavailable_evidence_items: u64,
}

/// Explicit non-claims retained in every focused report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReleaseEvidenceBoundary {
    pub network_access: ReleaseNetworkAccess,
    pub external_publication: ReleaseExternalPublication,
}

/// Network behavior of the native evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseNetworkAccess {
    NotPerformed,
}

/// What Egolint knows about externally owned delivery adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseExternalPublication {
    NotVerified,
}

/// Versioned, closed report for release applicability and mechanical checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RepositoryReleaseReport {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub contract: String,
    pub repository: Option<String>,
    pub policy: ReleasePolicyReference,
    pub declaration: ReleaseDeclarationReference,
    pub applicability: ReleaseApplicability,
    pub checks: Vec<ReleaseCheckResult>,
    pub state: ReleaseEvidenceState,
    pub summary: ReleaseEvaluationSummary,
    pub boundary: ReleaseEvidenceBoundary,
    pub rationale: String,
}

/// Native repository-release result shared by reports, findings, and SARIF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryReleaseEvaluation {
    pub report: RepositoryReleaseReport,
    pub findings: Vec<Finding>,
}

impl RepositoryReleaseReport {
    /// Validate cross-field state invariants before persisting the report.
    ///
    /// # Errors
    ///
    /// Returns an error when a state overclaims its declaration, coverage, or
    /// externally owned publication evidence.
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CONTRACT_VERSION || self.contract != REPORT_CONTRACT {
            return Err(configuration(
                "unsupported repository-release report contract",
            ));
        }
        if self.declaration.path != Path::new(DECLARATION_PATH) {
            return Err(configuration("repository-release declaration path drifted"));
        }
        if self.rationale.trim().is_empty()
            || self.rationale.len() > 2_048
            || self.rationale.chars().any(char::is_control)
        {
            return Err(configuration("repository-release rationale is invalid"));
        }
        if self
            .repository
            .as_deref()
            .is_some_and(|name| !valid_repository(name))
        {
            return Err(configuration(
                "repository-release repository must use owner/name form",
            ));
        }
        match self.declaration.state {
            ReleaseDeclarationState::Present if self.declaration.sha256.is_none() => {
                return Err(configuration("present declaration requires a digest"));
            }
            ReleaseDeclarationState::Missing if self.declaration.sha256.is_some() => {
                return Err(configuration("missing declaration may not have a digest"));
            }
            _ => {}
        }
        validate_digest(self.declaration.sha256.as_deref(), "declaration")?;
        validate_source_reference(&self.policy)?;

        let mut slot_ids = BTreeSet::new();
        for slot in &self.applicability.slots {
            if slot.id.trim().is_empty()
                || slot.authority.trim().is_empty()
                || !slot_ids.insert(slot.id.as_str())
            {
                return Err(configuration(
                    "repository-release slots must be unique and named",
                ));
            }
        }
        if self.checks.len() != self.applicability.slots.len() {
            return Err(configuration(
                "repository-release checks must cover every resolved slot",
            ));
        }
        let slots_by_id = self
            .applicability
            .slots
            .iter()
            .map(|slot| (slot.id.as_str(), slot))
            .collect::<BTreeMap<_, _>>();
        let mut check_ids = BTreeSet::new();
        for check in &self.checks {
            let Some(slot) = slots_by_id.get(check.slot_id.as_str()) else {
                return Err(configuration(
                    "repository-release check references an unknown slot",
                ));
            };
            if check.rule_id != checks::rule_id_for_slot(&check.slot_id)
                || check.requirement != slot.requirement
                || check.authority != slot.authority
                || !check_ids.insert(check.slot_id.as_str())
                || check.message.trim().is_empty()
                || check.remediation.trim().is_empty()
                || check.message.len() > 4_096
                || check.remediation.len() > 4_096
                || check.message.chars().any(char::is_control)
                || check.remediation.chars().any(char::is_control)
            {
                return Err(configuration(
                    "repository-release check identity or text is invalid",
                ));
            }
            if (check.requirement == ReleaseRequirement::NotApplicable)
                != (check.state == ReleaseCheckState::NotApplicable)
            {
                return Err(configuration(
                    "repository-release check state contradicts applicability",
                ));
            }
            check.location.validate()?;
            for evidence in &check.evidence {
                evidence.validate()?;
            }
        }
        let required = count_requirement(&self.applicability.slots, ReleaseRequirement::Required);
        let advisory = count_requirement(&self.applicability.slots, ReleaseRequirement::Advisory);
        let not_applicable =
            count_requirement(&self.applicability.slots, ReleaseRequirement::NotApplicable);
        if self.summary.required_slots != required
            || self.summary.advisory_slots != advisory
            || self.summary.not_applicable_slots != not_applicable
        {
            return Err(configuration(
                "repository-release slot counts do not match applicability",
            ));
        }
        let applicable = required + advisory;
        let checks_completed = self
            .checks
            .iter()
            .filter(|check| {
                matches!(
                    check.state,
                    ReleaseCheckState::Passed
                        | ReleaseCheckState::Failed
                        | ReleaseCheckState::External
                )
            })
            .count() as u64;
        let checks_failed = self
            .checks
            .iter()
            .filter(|check| check.state == ReleaseCheckState::Failed)
            .count() as u64;
        let unavailable_checks = self
            .checks
            .iter()
            .filter(|check| check.state == ReleaseCheckState::Unavailable)
            .count() as u64;
        let external_checks = self
            .checks
            .iter()
            .filter(|check| check.state == ReleaseCheckState::External)
            .count() as u64;
        let validation_complete = checks_completed == applicable && unavailable_checks == 0;
        if self.summary.checks_completed != checks_completed
            || self.summary.checks_failed != checks_failed
            || self.summary.validation_complete != validation_complete
            || self.summary.external_evidence_items < external_checks
            || self.summary.unavailable_evidence_items < unavailable_checks
        {
            return Err(configuration(
                "repository-release validation summary does not match checks",
            ));
        }
        match self.state {
            ReleaseEvidenceState::Compliant
                if self.declaration.state != ReleaseDeclarationState::Present
                    || !self.summary.validation_complete
                    || self.summary.checks_failed != 0
                    || self.summary.checks_completed < applicable
                    || self.summary.external_evidence_items != 0
                    || self.summary.unavailable_evidence_items != 0
                    || self.checks.iter().any(|check| {
                        check.state != ReleaseCheckState::Passed
                            && check.state != ReleaseCheckState::NotApplicable
                    }) =>
            {
                return Err(configuration(
                    "compliant release evidence requires complete local validation",
                ));
            }
            ReleaseEvidenceState::Advisory
                if !matches!(
                    self.applicability.adoption_state,
                    Some(ReleaseAdoptionState::Advisory | ReleaseAdoptionState::Exempt)
                ) || self.declaration.state != ReleaseDeclarationState::Present =>
            {
                return Err(configuration(
                    "advisory evidence requires advisory or exempt adoption",
                ));
            }
            ReleaseEvidenceState::Unavailable
                if self.declaration.state != ReleaseDeclarationState::Missing
                    && self.summary.unavailable_evidence_items == 0
                    && self.summary.validation_complete =>
            {
                return Err(configuration(
                    "unavailable evidence requires a missing or incomplete source",
                ));
            }
            ReleaseEvidenceState::External if self.summary.external_evidence_items == 0 => {
                return Err(configuration(
                    "external evidence requires an explicitly external item",
                ));
            }
            ReleaseEvidenceState::Invalid
                if self.declaration.state != ReleaseDeclarationState::Invalid
                    && self.summary.checks_failed == 0 =>
            {
                return Err(configuration(
                    "invalid evidence requires an invalid declaration or failed check",
                ));
            }
            ReleaseEvidenceState::NotApplicable
                if self.applicability.adoption_state
                    != Some(ReleaseAdoptionState::NotApplicable)
                    || self
                        .applicability
                        .slots
                        .iter()
                        .any(|slot| slot.requirement != ReleaseRequirement::NotApplicable)
                    || self
                        .checks
                        .iter()
                        .any(|check| check.state != ReleaseCheckState::NotApplicable) =>
            {
                return Err(configuration(
                    "not-applicable evidence requires explicit non-applicability",
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// Render a compact human-readable state without implying publication.
    #[must_use]
    pub fn render_text(&self) -> String {
        format!(
            "repository-release: {} — {} (required: {}, advisory: {}, not applicable: {}; network not performed; external publication not verified; see {})",
            self.state.as_str(),
            self.rationale,
            self.summary.required_slots,
            self.summary.advisory_slots,
            self.summary.not_applicable_slots,
            REPORT_PATH,
        )
    }
}

/// Source-pinned release applicability evaluator.
pub struct RepositoryReleaseEvaluator {
    policy: HygienePolicy,
    policy_reference: ReleasePolicyReference,
}

impl RepositoryReleaseEvaluator {
    /// Load and verify the immutable source lock and its vendored inputs.
    ///
    /// # Errors
    ///
    /// Returns an error when bundled bytes, source identities, or policy axes
    /// drift from the accepted checkpoint-1 boundary.
    pub fn bundled() -> Result<Self> {
        let source_lock: SourceEnvelope = serde_json::from_str(SOURCE_LOCK)?;
        let policy: HygienePolicy = serde_json::from_str(HYGIENE_POLICY)?;
        let aether_schema: Value = serde_json::from_str(AETHER_SCHEMA)?;
        validate_bundled_inputs(&source_lock, &policy, &aether_schema)?;
        let hygiene_source = source_lock
            .sources
            .iter()
            .find(|source| source.id == "hygiene-repository-release-profile")
            .cloned()
            .ok_or_else(|| configuration("missing Hygiene repository-release source"))?;
        let aether_source = source_lock
            .sources
            .iter()
            .find(|source| source.id == "aether-repository-release-schema")
            .cloned()
            .ok_or_else(|| configuration("missing Aether repository-release source"))?;
        let policy_reference = ReleasePolicyReference {
            id: policy.schema.clone(),
            version: policy.version.clone(),
            status: policy.status.clone(),
            owner: policy.owner.clone(),
            hygiene_source,
            aether_contract_id: policy.aether_contract.id.clone(),
            aether_contract_version: policy.aether_contract.version.clone(),
            aether_schema_url: policy.aether_contract.schema_url.clone(),
            aether_source,
        };
        Ok(Self {
            policy,
            policy_reference,
        })
    }

    /// Resolve local declaration applicability without network access.
    ///
    /// `explicit_adoption` is intended for an authorized planner. Without it,
    /// rollout is derived from the accepted lifecycle policy. The evaluator
    /// derives validation coverage from the local repository; callers cannot
    /// supply or inflate completion counters.
    ///
    /// # Errors
    ///
    /// Returns an error only when the generated focused report violates its
    /// native contract; malformed repository input is represented as `invalid`.
    #[allow(clippy::too_many_lines)]
    pub fn evaluate(
        &self,
        inventory: &RepositoryInventory,
        explicit_adoption: Option<ReleaseAdoptionState>,
    ) -> Result<RepositoryReleaseEvaluation> {
        let declaration = inspect_declaration(inventory);
        let (
            reference,
            repository,
            profile,
            lifecycle,
            document,
            declared_external,
            declared_unavailable,
            invalid_reason,
        ) = match declaration {
            DeclarationInspection::Missing => (
                ReleaseDeclarationReference {
                    path: PathBuf::from(DECLARATION_PATH),
                    state: ReleaseDeclarationState::Missing,
                    sha256: None,
                },
                None,
                None,
                None,
                None,
                0,
                0,
                None,
            ),
            DeclarationInspection::Invalid { digest, reason } => (
                ReleaseDeclarationReference {
                    path: PathBuf::from(DECLARATION_PATH),
                    state: ReleaseDeclarationState::Invalid,
                    sha256: digest,
                },
                None,
                None,
                None,
                None,
                0,
                0,
                Some(reason),
            ),
            DeclarationInspection::Present(data) => (
                ReleaseDeclarationReference {
                    path: PathBuf::from(DECLARATION_PATH),
                    state: ReleaseDeclarationState::Present,
                    sha256: Some(data.digest),
                },
                Some(data.repository),
                Some(data.profile),
                Some(data.lifecycle),
                Some(data.document),
                data.external_evidence_items,
                data.unavailable_evidence_items,
                None,
            ),
        };

        let adoption = explicit_adoption.or_else(|| lifecycle.map(|value| self.rollout(value)));
        let applicability_source = if explicit_adoption.is_some() {
            ReleaseApplicabilitySource::Explicit
        } else if lifecycle.is_some() {
            ReleaseApplicabilitySource::LifecyclePolicy
        } else {
            ReleaseApplicabilitySource::Unavailable
        };
        let slots = if adoption == Some(ReleaseAdoptionState::NotApplicable) {
            self.policy
                .slots
                .iter()
                .map(|slot| ReleaseSlotApplicability {
                    id: slot.id.clone(),
                    authority: slot.authority.clone(),
                    requirement: ReleaseRequirement::NotApplicable,
                })
                .collect()
        } else if let (Some(profile), Some(lifecycle), Some(adoption)) =
            (profile, lifecycle, adoption)
        {
            self.resolve_slots(profile, lifecycle, adoption)
        } else {
            Vec::new()
        };
        let required_slots = count_requirement(&slots, ReleaseRequirement::Required);
        let advisory_slots = count_requirement(&slots, ReleaseRequirement::Advisory);
        let not_applicable_slots = count_requirement(&slots, ReleaseRequirement::NotApplicable);
        let check_results =
            checks::evaluate(inventory, document.as_ref(), &slots, &self.policy_reference)?;
        let checks_completed = check_results
            .iter()
            .filter(|check| {
                matches!(
                    check.state,
                    ReleaseCheckState::Passed
                        | ReleaseCheckState::Failed
                        | ReleaseCheckState::External
                )
            })
            .count() as u64;
        let checks_failed = check_results
            .iter()
            .filter(|check| check.state == ReleaseCheckState::Failed)
            .count() as u64;
        let unavailable_checks = check_results
            .iter()
            .filter(|check| check.state == ReleaseCheckState::Unavailable)
            .count() as u64;
        let external_checks = check_results
            .iter()
            .filter(|check| check.state == ReleaseCheckState::External)
            .count() as u64;
        let validation_complete =
            checks_completed == required_slots + advisory_slots && unavailable_checks == 0;
        let external = declared_external.max(external_checks);
        let unavailable = declared_unavailable.max(unavailable_checks);
        let (state, rationale) = resolve_state(
            reference.state,
            adoption,
            validation_complete,
            checks_completed,
            checks_failed,
            required_slots + advisory_slots,
            external,
            unavailable,
            invalid_reason.as_deref(),
        );
        let report = RepositoryReleaseReport {
            schema_version: CONTRACT_VERSION,
            contract: REPORT_CONTRACT.to_owned(),
            repository,
            policy: self.policy_reference.clone(),
            declaration: reference,
            applicability: ReleaseApplicability {
                repository_profile: profile,
                lifecycle,
                adoption_state: adoption,
                source: applicability_source,
                slots,
            },
            checks: check_results,
            state,
            summary: ReleaseEvaluationSummary {
                required_slots,
                advisory_slots,
                not_applicable_slots,
                validation_complete,
                checks_completed,
                checks_failed,
                external_evidence_items: external,
                unavailable_evidence_items: unavailable,
            },
            boundary: ReleaseEvidenceBoundary {
                network_access: ReleaseNetworkAccess::NotPerformed,
                external_publication: ReleaseExternalPublication::NotVerified,
            },
            rationale,
        };
        report.validate()?;
        let mut findings = checks::findings(&report.checks);
        if report.applicability.slots.is_empty() {
            if let Some(finding) = declaration_finding(&report) {
                findings.push(finding);
            }
        }
        findings.sort_by(|left, right| {
            left.rule
                .rule_id
                .cmp(&right.rule.rule_id)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(RepositoryReleaseEvaluation { report, findings })
    }

    fn rollout(&self, lifecycle: ReleaseLifecycle) -> ReleaseAdoptionState {
        match lifecycle {
            ReleaseLifecycle::Incubating => self.policy.migration.incubating_repositories,
            ReleaseLifecycle::Active => self.policy.migration.existing_active_repositories,
            ReleaseLifecycle::Internal | ReleaseLifecycle::Archived => {
                self.policy.migration.new_repositories
            }
        }
    }

    fn resolve_slots(
        &self,
        profile: RepositoryReleaseProfile,
        lifecycle: ReleaseLifecycle,
        adoption: ReleaseAdoptionState,
    ) -> Vec<ReleaseSlotApplicability> {
        let profile_name = serialized(&profile);
        let lifecycle_name = serialized(&lifecycle);
        self.policy
            .slots
            .iter()
            .map(|slot| {
                let profile_requirement = self
                    .policy
                    .profile_overrides
                    .get(&profile_name)
                    .and_then(|overrides| overrides.get(&slot.id))
                    .copied()
                    .unwrap_or(slot.default_requirement);
                let lifecycle_requirement = self
                    .policy
                    .lifecycle_overrides
                    .get(&lifecycle_name)
                    .and_then(|overrides| overrides.get(&slot.id))
                    .copied()
                    .unwrap_or(profile_requirement);
                let requirement = match adoption {
                    ReleaseAdoptionState::Required => lifecycle_requirement,
                    ReleaseAdoptionState::Advisory | ReleaseAdoptionState::Exempt => {
                        match lifecycle_requirement {
                            ReleaseRequirement::Required => ReleaseRequirement::Advisory,
                            value => value,
                        }
                    }
                    ReleaseAdoptionState::NotApplicable => ReleaseRequirement::NotApplicable,
                };
                ReleaseSlotApplicability {
                    id: slot.id.clone(),
                    authority: slot.authority.clone(),
                    requirement,
                }
            })
            .collect()
    }
}

/// Atomically write the focused report inside Egolint's report boundary.
///
/// # Errors
///
/// Returns an error when the report is inconsistent, the destination escapes
/// the report boundary, or durable persistence fails.
pub fn write_release_report_atomic(report: &RepositoryReleaseReport, path: &Path) -> Result<()> {
    report.validate()?;
    if path != Path::new(REPORT_PATH) && !path.ends_with(REPORT_PATH) {
        return Err(configuration(&format!(
            "repository-release report path must end with {REPORT_PATH}"
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
#[serde(deny_unknown_fields)]
struct SourceEnvelope {
    schema_version: u32,
    contract: String,
    reviewed_at: String,
    sources: Vec<ReleaseSourceLock>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HygienePolicy {
    schema: String,
    version: String,
    status: String,
    owner: String,
    updated: String,
    purpose: String,
    aether_contract: HygieneAetherContract,
    requirements: Vec<ReleaseRequirement>,
    adoption_states: Vec<ReleaseAdoptionState>,
    repository_profiles: Vec<RepositoryReleaseProfile>,
    lifecycles: Vec<ReleaseLifecycle>,
    visibilities: Vec<String>,
    slots: Vec<HygieneSlot>,
    profile_overrides: BTreeMap<String, BTreeMap<String, ReleaseRequirement>>,
    lifecycle_overrides: BTreeMap<String, BTreeMap<String, ReleaseRequirement>>,
    visibility_overrides: BTreeMap<String, BTreeMap<String, ReleaseRequirement>>,
    migration: HygieneMigration,
    examples: Vec<Value>,
    ownership: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HygieneAetherContract {
    id: String,
    version: String,
    repository: String,
    revision: String,
    specification: PathBuf,
    schema_url: String,
    authoring_skill: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HygieneSlot {
    id: String,
    title: String,
    default_requirement: ReleaseRequirement,
    authority: String,
    description: String,
    checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HygieneMigration {
    new_repositories: ReleaseAdoptionState,
    existing_active_repositories: ReleaseAdoptionState,
    incubating_repositories: ReleaseAdoptionState,
    legacy_history: Value,
    exceptions: Value,
    repository_facts: HygieneRepositoryFacts,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HygieneRepositoryFacts {
    declaration_path: PathBuf,
    external_registry_required: bool,
    one_source_of_truth: String,
}

struct DeclarationData {
    digest: String,
    repository: String,
    profile: RepositoryReleaseProfile,
    lifecycle: ReleaseLifecycle,
    document: checks::ReleaseDeclaration,
    external_evidence_items: u64,
    unavailable_evidence_items: u64,
}

enum DeclarationInspection {
    Missing,
    Invalid {
        digest: Option<String>,
        reason: String,
    },
    Present(Box<DeclarationData>),
}

fn inspect_declaration(inventory: &RepositoryInventory) -> DeclarationInspection {
    let Some(entry) = inventory.get(Path::new(DECLARATION_PATH)) else {
        return DeclarationInspection::Missing;
    };
    if entry.kind != RepositoryEntryKind::File {
        return DeclarationInspection::Invalid {
            digest: None,
            reason: "the release declaration must be a regular file, not a symbolic link"
                .to_owned(),
        };
    }
    let digest = digest(&entry.content);
    if entry.content.len() > MAXIMUM_DECLARATION_BYTES {
        return DeclarationInspection::Invalid {
            digest: Some(digest),
            reason: "the release declaration exceeds the 4 MiB inspection limit".to_owned(),
        };
    }
    let value: Value = match serde_json::from_slice(&entry.content) {
        Ok(value) => value,
        Err(_) => {
            return DeclarationInspection::Invalid {
                digest: Some(digest),
                reason: "the release declaration is not valid JSON".to_owned(),
            };
        }
    };
    let document = match checks::parse_declaration(value.clone()) {
        Ok(document) => document,
        Err(reason) => return invalid_declaration(digest, &reason),
    };
    let repository_id = document.repository.id.clone();
    let profile = document.repository.release_profile;
    let lifecycle = document.repository.lifecycle;
    let (external_evidence_items, unavailable_evidence_items) = evidence_counts(&value);
    DeclarationInspection::Present(Box::new(DeclarationData {
        digest,
        repository: repository_id,
        profile,
        lifecycle,
        document,
        external_evidence_items,
        unavailable_evidence_items,
    }))
}

fn invalid_declaration(digest: String, reason: &str) -> DeclarationInspection {
    DeclarationInspection::Invalid {
        digest: Some(digest),
        reason: reason.to_owned(),
    }
}

fn evidence_counts(value: &Value) -> (u64, u64) {
    let mut states = Vec::new();
    if let Some(channels) = value
        .pointer("/delivery/channels")
        .and_then(Value::as_array)
    {
        states.extend(
            channels
                .iter()
                .filter_map(|channel| channel.get("state").and_then(Value::as_str)),
        );
    }
    if let Some(evidence) = value.get("evidence").and_then(Value::as_object) {
        for name in ["source", "change", "provenance", "sbom", "signature"] {
            if let Some(state) = evidence.get(name).and_then(Value::as_str) {
                states.push(state);
            }
        }
    }
    if let Some(state) = value
        .pointer("/automation/github/state")
        .and_then(Value::as_str)
    {
        states.push(state);
    }
    (
        states.iter().filter(|state| **state == "external").count() as u64,
        states
            .iter()
            .filter(|state| **state == "unavailable")
            .count() as u64,
    )
}

fn declaration_finding(report: &RepositoryReleaseReport) -> Option<Finding> {
    let (message, remediation) = match report.declaration.state {
        ReleaseDeclarationState::Missing => (
            "The repository release declaration is unavailable.",
            format!(
                "Create {DECLARATION_PATH} from the pinned Aether contract, or have an authorized planner select not-applicable."
            ),
        ),
        ReleaseDeclarationState::Invalid => (
            report.rationale.as_str(),
            format!(
                "Repair {DECLARATION_PATH} so it satisfies the pinned Aether repository-release schema."
            ),
        ),
        ReleaseDeclarationState::Present => return None,
    };
    let location = SourceLocation {
        path: PathBuf::from(DECLARATION_PATH),
        start_line: None,
        start_column: None,
        end_line: None,
        end_column: None,
    };
    let normalized_message = format!("{message} Remediation: {remediation}");
    let fingerprint = stable_fingerprint(DECLARATION_RULE, &location, &normalized_message);
    let severity = if report.applicability.adoption_state == Some(ReleaseAdoptionState::Required) {
        Severity::Error
    } else {
        Severity::Warning
    };
    Some(Finding {
        schema_version: CONTRACT_VERSION,
        id: format!("{DECLARATION_RULE}-{fingerprint}"),
        rule: RuleIdentity {
            tool_id: TOOL_ID.to_owned(),
            rule_id: DECLARATION_RULE.to_owned(),
        },
        severity,
        message: normalized_message,
        location: Some(location),
        ownership: RuleOwnership {
            owner: "egohygiene/egolint".to_owned(),
            policy_source: format!("{POLICY_PATH}#aether_declaration"),
            configuration_path: Some(PathBuf::from(POLICY_PATH)),
        },
        fingerprint: Some(fingerprint),
        evidence: vec![EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Configuration,
            path: PathBuf::from(DECLARATION_PATH),
            sha256: report.declaration.sha256.clone(),
            description: Some("Repository-owned Aether release declaration.".to_owned()),
        }],
        suppressed_by: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_state(
    declaration: ReleaseDeclarationState,
    adoption: Option<ReleaseAdoptionState>,
    validation_complete: bool,
    checks_completed: u64,
    checks_failed: u64,
    applicable_slots: u64,
    external: u64,
    unavailable: u64,
    invalid_reason: Option<&str>,
) -> (ReleaseEvidenceState, String) {
    if let Some(reason) = invalid_reason {
        return (ReleaseEvidenceState::Invalid, reason.to_owned());
    }
    if adoption == Some(ReleaseAdoptionState::NotApplicable) {
        return (
            ReleaseEvidenceState::NotApplicable,
            "an authorized planner explicitly marked repository release policy not applicable"
                .to_owned(),
        );
    }
    if declaration == ReleaseDeclarationState::Missing {
        return (
            ReleaseEvidenceState::Unavailable,
            "the repository release declaration is not available".to_owned(),
        );
    }
    if matches!(
        adoption,
        Some(ReleaseAdoptionState::Advisory | ReleaseAdoptionState::Exempt)
    ) {
        let rationale = if checks_failed > 0 || unavailable > 0 || external > 0 {
            "the accepted rollout keeps visible incomplete or failing release requirements advisory"
        } else {
            "the accepted rollout keeps applicable release requirements advisory"
        };
        return (ReleaseEvidenceState::Advisory, rationale.to_owned());
    }
    if checks_failed > 0 {
        return (
            ReleaseEvidenceState::Invalid,
            "one or more completed repository release checks failed".to_owned(),
        );
    }
    if unavailable > 0 {
        return (
            ReleaseEvidenceState::Unavailable,
            "the declaration marks one or more release evidence sources unavailable".to_owned(),
        );
    }
    if external > 0 {
        return (
            ReleaseEvidenceState::External,
            "the declaration assigns evidence to an external owner; reachability and publication were not checked"
                .to_owned(),
        );
    }
    if !validation_complete || checks_completed < applicable_slots {
        return (
            ReleaseEvidenceState::Unavailable,
            "applicability resolved, but complete conformance checks are not available".to_owned(),
        );
    }
    (
        ReleaseEvidenceState::Compliant,
        "all applicable local repository release checks completed without failure".to_owned(),
    )
}

#[allow(clippy::too_many_lines)]
fn validate_bundled_inputs(
    source_lock: &SourceEnvelope,
    policy: &HygienePolicy,
    aether_schema: &Value,
) -> Result<()> {
    if source_lock.schema_version != CONTRACT_VERSION
        || source_lock.contract != SOURCE_CONTRACT
        || source_lock.sources.len() != 2
        || source_lock.reviewed_at != "2026-09-19"
    {
        return Err(configuration(
            "repository-release source lock is unsupported",
        ));
    }
    let source_ids = source_lock
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect::<BTreeSet<_>>();
    if source_ids
        != BTreeSet::from([
            "aether-repository-release-schema",
            "hygiene-repository-release-profile",
        ])
    {
        return Err(configuration(
            "repository-release source identities drifted",
        ));
    }
    for source in &source_lock.sources {
        let (expected_repository, expected_revision, expected_path, expected_digest, bytes) =
            match source.id.as_str() {
                "aether-repository-release-schema" => (
                    "egohygiene/aether",
                    AETHER_SOURCE_REVISION,
                    Path::new("vendor/aether/aether.repository-release.v1.schema.json"),
                    "8431ea7651336aa7695aee5b9aedce536b1e7c76c3cb15927f64a6206f9cc1f5",
                    AETHER_SCHEMA.as_bytes(),
                ),
                "hygiene-repository-release-profile" => (
                    "egohygiene/hygiene",
                    HYGIENE_SOURCE_REVISION,
                    Path::new("vendor/hygiene/repository-release-policy.v1.json"),
                    "20030513c311416c5130a15af897590892025b55cfd79d210be362d47748063f",
                    HYGIENE_POLICY.as_bytes(),
                ),
                _ => unreachable!("source identities were checked above"),
            };
        if source.repository != expected_repository
            || source.revision != expected_revision
            || source.vendored_path != expected_path
            || source.sha256 != expected_digest
            || digest(bytes) != source.sha256
            || !lowercase_hex(&source.git_blob_sha1, 40)
        {
            return Err(configuration("repository-release source pin drifted"));
        }
    }
    if policy.schema != HYGIENE_POLICY_ID
        || policy.version != HYGIENE_POLICY_VERSION
        || policy.owner != "egohygiene/hygiene"
        || policy.status != "proposed"
        || policy.updated != "2026-08-31"
        || policy.purpose.trim().is_empty()
        || policy.aether_contract.id != AETHER_CONTRACT_ID
        || policy.aether_contract.version != AETHER_CONTRACT_VERSION
        || policy.aether_contract.repository != "egohygiene/aether"
        || policy.aether_contract.revision != AETHER_SOURCE_REVISION
        || policy.aether_contract.schema_url != AETHER_SCHEMA_URL
        || policy.aether_contract.specification
            != Path::new("library/organization/specs/release/repository-release.spec.md")
        || !policy.aether_contract.authoring_skill.is_object()
        || aether_schema.get("$id").and_then(Value::as_str) != Some(AETHER_SCHEMA_URL)
    {
        return Err(configuration("repository-release policy binding drifted"));
    }
    let expected_requirements = BTreeSet::from([
        ReleaseRequirement::Required,
        ReleaseRequirement::Advisory,
        ReleaseRequirement::NotApplicable,
    ]);
    let expected_adoption = BTreeSet::from([
        ReleaseAdoptionState::Required,
        ReleaseAdoptionState::Advisory,
        ReleaseAdoptionState::Exempt,
        ReleaseAdoptionState::NotApplicable,
    ]);
    let expected_profiles = BTreeSet::from([
        RepositoryReleaseProfile::CliLibrary,
        RepositoryReleaseProfile::ContainerImage,
        RepositoryReleaseProfile::Contract,
        RepositoryReleaseProfile::InternalOnly,
        RepositoryReleaseProfile::NpmPackage,
        RepositoryReleaseProfile::Publication,
        RepositoryReleaseProfile::PythonPackage,
        RepositoryReleaseProfile::StaticSite,
        RepositoryReleaseProfile::Workspace,
    ]);
    let expected_lifecycles = BTreeSet::from([
        ReleaseLifecycle::Active,
        ReleaseLifecycle::Incubating,
        ReleaseLifecycle::Internal,
        ReleaseLifecycle::Archived,
    ]);
    if policy.requirements.iter().copied().collect::<BTreeSet<_>>() != expected_requirements
        || policy
            .adoption_states
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_adoption
        || policy
            .repository_profiles
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_profiles
        || policy.lifecycles.iter().copied().collect::<BTreeSet<_>>() != expected_lifecycles
        || policy.visibilities.iter().collect::<BTreeSet<_>>().len() != 3
        || policy.visibility_overrides.len() != 3
        || policy.examples.is_empty()
        || policy.ownership.is_empty()
    {
        return Err(configuration("repository-release policy axes drifted"));
    }
    let mut slot_ids = BTreeSet::new();
    for slot in &policy.slots {
        if slot.id.trim().is_empty()
            || slot.title.trim().is_empty()
            || slot.authority.trim().is_empty()
            || slot.description.trim().is_empty()
            || slot.checks.is_empty()
            || !slot_ids.insert(slot.id.as_str())
        {
            return Err(configuration("repository-release policy slots are invalid"));
        }
    }
    let expected_slot_ids = BTreeSet::from([
        "agents_profile_pointer",
        "aether_declaration",
        "changelog",
        "manual_workflow",
        "release_rollback_docs",
        "task_handoffs",
        "version_authority",
    ]);
    if slot_ids != expected_slot_ids {
        return Err(configuration(
            "repository-release policy slots drifted from native rule coverage",
        ));
    }
    validate_overrides(
        &policy.profile_overrides,
        &slot_ids,
        expected_profiles.len(),
    )?;
    validate_overrides(
        &policy.lifecycle_overrides,
        &slot_ids,
        expected_lifecycles.len(),
    )?;
    validate_overrides(&policy.visibility_overrides, &slot_ids, 3)?;
    if policy.migration.new_repositories != ReleaseAdoptionState::Required
        || policy.migration.existing_active_repositories != ReleaseAdoptionState::Required
        || policy.migration.incubating_repositories != ReleaseAdoptionState::Advisory
        || !policy.migration.legacy_history.is_object()
        || !policy.migration.exceptions.is_object()
        || policy.migration.repository_facts.declaration_path != Path::new(DECLARATION_PATH)
        || policy.migration.repository_facts.external_registry_required
        || policy
            .migration
            .repository_facts
            .one_source_of_truth
            .trim()
            .is_empty()
    {
        return Err(configuration("repository-release migration policy drifted"));
    }
    Ok(())
}

fn validate_overrides(
    overrides: &BTreeMap<String, BTreeMap<String, ReleaseRequirement>>,
    slots: &BTreeSet<&str>,
    expected_axes: usize,
) -> Result<()> {
    if overrides.len() != expected_axes
        || overrides
            .values()
            .flat_map(BTreeMap::keys)
            .any(|slot| !slots.contains(slot.as_str()))
    {
        Err(configuration("repository-release policy overrides drifted"))
    } else {
        Ok(())
    }
}

fn validate_source_reference(reference: &ReleasePolicyReference) -> Result<()> {
    if reference.id != HYGIENE_POLICY_ID
        || reference.version != HYGIENE_POLICY_VERSION
        || reference.status != "proposed"
        || reference.owner != "egohygiene/hygiene"
        || reference.aether_contract_id != AETHER_CONTRACT_ID
        || reference.aether_contract_version != AETHER_CONTRACT_VERSION
        || reference.aether_schema_url != AETHER_SCHEMA_URL
        || reference.hygiene_source.revision != HYGIENE_SOURCE_REVISION
        || reference.aether_source.revision != AETHER_SOURCE_REVISION
    {
        return Err(configuration(
            "repository-release report source identity drifted",
        ));
    }
    for source in [&reference.hygiene_source, &reference.aether_source] {
        if !lowercase_hex(&source.sha256, 64)
            || !lowercase_hex(&source.git_blob_sha1, 40)
            || !lowercase_hex(&source.revision, 40)
        {
            return Err(configuration(
                "repository-release report source digest is invalid",
            ));
        }
    }
    Ok(())
}

fn validate_digest(value: Option<&str>, name: &str) -> Result<()> {
    if value.is_none_or(|digest| lowercase_hex(digest, 64)) {
        Ok(())
    } else {
        Err(configuration(&format!(
            "repository-release {name} digest is invalid"
        )))
    }
}

fn count_requirement(slots: &[ReleaseSlotApplicability], target: ReleaseRequirement) -> u64 {
    slots
        .iter()
        .filter(|slot| slot.requirement == target)
        .count() as u64
}

fn serialized<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("policy enums always serialize as strings")
}

fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    let valid = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    };
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(name), None) if valid(owner) && valid(name))
}

fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn stable_fingerprint(rule_id: &str, location: &SourceLocation, message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(location.path.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    hasher.update(message.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_owned()
}

fn configuration(message: &str) -> EgolintError {
    EgolintError::Configuration(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RepositoryEntry;

    fn declaration(lifecycle: &str, evidence_state: &str) -> RepositoryInventory {
        let release_state = if lifecycle == "archived" {
            "frozen"
        } else {
            "unreleased"
        };
        let contents = format!(
            r#"{{
  "$schema": "{AETHER_SCHEMA_URL}",
  "schema_version": "{AETHER_CONTRACT_ID}",
  "repository": {{
    "id": "egohygiene/example",
    "lifecycle": "{lifecycle}",
    "release_profile": "cli-library"
  }},
  "release": {{
    "state": "{release_state}",
    "tag_prefix": "v",
    "immutable_tags": true,
    "major_alias": "disabled"
  }},
  "changelog": {{
    "path": "CHANGELOG.md",
    "format": "keep-a-changelog/1.1",
    "unreleased_heading": "Unreleased"
  }},
  "components": [{{
    "id": "egolint",
    "kind": "crate",
    "version_authority": {{"kind": "cargo-manifest", "path": "Cargo.toml"}}
  }}],
  "delivery": {{"channels": [{{"kind": "github-release", "state": "configured"}}]}},
  "evidence": {{
    "source": "{evidence_state}",
    "change": "required",
    "provenance": "required",
    "sbom": "required",
    "signature": "required",
    "rollback": {{
      "strategy": "revert-and-successor-tag",
      "instructions": "Revert the change and publish a reviewed successor tag."
    }}
  }},
  "automation": {{
    "taskfile_path": "Taskfile.yml",
    "tasks": {{
      "plan": "release:plan",
      "prepare": "release:prepare",
      "verify": "release:verify",
      "publish": "release:publish"
    }},
    "github": {{
      "manual_dispatch_required": true,
      "workflow_path": ".github/workflows/release.yml",
      "state": "configured"
    }}
  }}
}}"#
        );
        RepositoryInventory::from_entries(vec![
            RepositoryEntry::file(
                DECLARATION_PATH,
                Some(100_644),
                contents.into_bytes(),
            ),
            RepositoryEntry::file(
                "AGENTS.md",
                Some(100_644),
                b"Release facts: .egohygiene/release.json\n".to_vec(),
            ),
            RepositoryEntry::file(
                "CHANGELOG.md",
                Some(100_644),
                b"# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-09-19\n".to_vec(),
            ),
            RepositoryEntry::file(
                "Cargo.toml",
                Some(100_644),
                b"[package]\nname = \"egolint\"\nversion = \"0.1.0\"\n".to_vec(),
            ),
            RepositoryEntry::file(
                "Taskfile.yml",
                Some(100_644),
                b"version: '3'\ntasks:\n  'release:plan': {cmds: ['true']}\n  'release:prepare': {cmds: ['true']}\n  'release:verify': {cmds: ['true']}\n  'release:publish': {cmds: ['gh workflow run .github/workflows/release.yml']}\n"
                    .to_vec(),
            ),
            RepositoryEntry::file(
                ".github/workflows/release.yml",
                Some(100_644),
                b"name: Release\non:\n  workflow_dispatch:\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"
                    .to_vec(),
            ),
        ])
        .expect("valid test inventory")
    }

    fn evaluate(
        inventory: &RepositoryInventory,
        adoption: Option<ReleaseAdoptionState>,
    ) -> RepositoryReleaseReport {
        RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(inventory, adoption)
            .expect("valid report")
            .report
    }

    #[test]
    fn bundled_policy_is_source_pinned_and_archived_slots_are_scoped() {
        let evaluator = RepositoryReleaseEvaluator::bundled().expect("bundled policy");
        let report = evaluator
            .evaluate(&declaration("archived", "required"), None)
            .expect("archived report")
            .report;

        assert_eq!(
            report.policy.hygiene_source.revision,
            HYGIENE_SOURCE_REVISION
        );
        assert_eq!(report.policy.aether_source.revision, AETHER_SOURCE_REVISION);
        assert_eq!(report.summary.required_slots, 4);
        assert_eq!(report.summary.advisory_slots, 1);
        assert_eq!(report.summary.not_applicable_slots, 2);
        assert_eq!(report.state, ReleaseEvidenceState::Compliant);
        assert_eq!(report.checks.len(), 7);
    }

    #[test]
    fn every_evidence_state_has_a_valid_positive_report() {
        let active = declaration("active", "required");
        let compliant = evaluate(&active, None);
        let advisory = evaluate(&declaration("incubating", "required"), None);
        let unavailable = evaluate(&RepositoryInventory::default(), None);
        let external = evaluate(&declaration("active", "external"), None);
        let invalid_inventory = RepositoryInventory::from_entries(vec![RepositoryEntry::file(
            DECLARATION_PATH,
            None,
            b"not json".to_vec(),
        )])
        .expect("hostile inventory");
        let invalid = evaluate(&invalid_inventory, None);
        let not_applicable = evaluate(
            &RepositoryInventory::default(),
            Some(ReleaseAdoptionState::NotApplicable),
        );

        for (report, expected) in [
            (compliant, ReleaseEvidenceState::Compliant),
            (advisory, ReleaseEvidenceState::Advisory),
            (unavailable, ReleaseEvidenceState::Unavailable),
            (external, ReleaseEvidenceState::External),
            (invalid, ReleaseEvidenceState::Invalid),
            (not_applicable, ReleaseEvidenceState::NotApplicable),
        ] {
            assert_eq!(report.state, expected);
            report.validate().expect("state invariant");
            assert_eq!(
                report.boundary.external_publication,
                ReleaseExternalPublication::NotVerified
            );
        }
    }

    #[test]
    fn every_evidence_state_rejects_a_hostile_overclaim() {
        let unavailable = evaluate(&RepositoryInventory::default(), None);
        let compliant = evaluate(&declaration("active", "required"), None);
        for state in [
            ReleaseEvidenceState::Compliant,
            ReleaseEvidenceState::Advisory,
        ] {
            let mut hostile = unavailable.clone();
            hostile.state = state;
            assert!(hostile.validate().is_err(), "hostile {state:?}");
        }
        for state in [
            ReleaseEvidenceState::Unavailable,
            ReleaseEvidenceState::External,
            ReleaseEvidenceState::Invalid,
            ReleaseEvidenceState::NotApplicable,
        ] {
            let mut hostile = compliant.clone();
            hostile.state = state;
            assert!(hostile.validate().is_err(), "hostile {state:?}");
        }
    }

    #[test]
    fn explicit_not_applicable_needs_no_declaration_and_keeps_all_slots_off() {
        let report = evaluate(
            &RepositoryInventory::default(),
            Some(ReleaseAdoptionState::NotApplicable),
        );

        assert_eq!(report.state, ReleaseEvidenceState::NotApplicable);
        assert_eq!(report.summary.not_applicable_slots, 7);
        assert!(
            report
                .applicability
                .slots
                .iter()
                .all(|slot| slot.requirement == ReleaseRequirement::NotApplicable)
        );
    }

    #[test]
    fn human_rendering_preserves_external_ownership_boundaries() {
        let evaluation = RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(&declaration("active", "external"), None)
            .expect("valid evaluation");
        let rendered = evaluation.report.render_text();

        assert!(rendered.contains("repository-release: external"));
        assert!(rendered.contains("network not performed"));
        assert!(rendered.contains("external publication not verified"));
        assert!(!rendered.contains("published successfully"));
        assert!(evaluation.findings.iter().any(|finding| {
            finding.rule.rule_id == "EGOLINT_RELEASE_AETHER_DECLARATION"
                && finding.severity == Severity::Warning
        }));
    }

    #[test]
    fn unpinned_workflow_dependency_is_a_stable_blocking_finding() {
        let inventory = declaration("active", "required");
        let mut entries = inventory.entries().to_vec();
        let workflow = entries
            .iter_mut()
            .find(|entry| entry.path == Path::new(".github/workflows/release.yml"))
            .expect("workflow fixture");
        workflow.content = b"name: Release\non:\n  workflow_dispatch:\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n".to_vec();
        let inventory = RepositoryInventory::from_entries(entries).expect("hostile inventory");

        let evaluation = RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(&inventory, None)
            .expect("valid evaluation");
        let finding = evaluation
            .findings
            .iter()
            .find(|finding| finding.rule.rule_id == "EGOLINT_RELEASE_MANUAL_WORKFLOW")
            .expect("workflow finding");

        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding
                .location
                .as_ref()
                .map(|location| location.path.as_path()),
            Some(Path::new(".github/workflows/release.yml"))
        );
        assert!(finding.fingerprint.is_some());
        assert_eq!(evaluation.report.state, ReleaseEvidenceState::Invalid);
    }

    #[test]
    fn advisory_rollout_preserves_failed_checks_as_warnings() {
        let inventory = declaration("incubating", "required");
        let mut entries = inventory.entries().to_vec();
        entries.retain(|entry| entry.path != Path::new("CHANGELOG.md"));
        let inventory = RepositoryInventory::from_entries(entries).expect("advisory inventory");

        let evaluation = RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(&inventory, None)
            .expect("valid evaluation");

        assert_eq!(evaluation.report.state, ReleaseEvidenceState::Advisory);
        assert!(
            evaluation
                .findings
                .iter()
                .all(|finding| finding.severity == Severity::Warning)
        );
        assert!(
            evaluation
                .findings
                .iter()
                .any(|finding| finding.rule.rule_id == "EGOLINT_RELEASE_CHANGELOG")
        );
    }

    #[test]
    fn explicit_required_adoption_makes_a_missing_declaration_blocking() {
        let evaluation = RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(
                &RepositoryInventory::default(),
                Some(ReleaseAdoptionState::Required),
            )
            .expect("valid evaluation");

        assert_eq!(evaluation.report.state, ReleaseEvidenceState::Unavailable);
        assert_eq!(evaluation.findings.len(), 1);
        assert_eq!(evaluation.findings[0].severity, Severity::Error);
        assert_eq!(
            evaluation.findings[0].rule.rule_id,
            "EGOLINT_RELEASE_AETHER_DECLARATION"
        );
    }

    #[test]
    fn safe_single_component_version_drift_is_blocking() {
        let inventory = declaration("active", "required");
        let mut entries = inventory.entries().to_vec();
        let declaration = entries
            .iter_mut()
            .find(|entry| entry.path == Path::new(DECLARATION_PATH))
            .expect("declaration fixture");
        let text = String::from_utf8(declaration.content.clone()).expect("UTF-8 fixture");
        declaration.content = text
            .replace("\"state\": \"unreleased\"", "\"state\": \"released\"")
            .into_bytes();
        let manifest = entries
            .iter_mut()
            .find(|entry| entry.path == Path::new("Cargo.toml"))
            .expect("manifest fixture");
        manifest.content = b"[package]\nname = \"egolint\"\nversion = \"0.2.0\"\n".to_vec();
        let inventory = RepositoryInventory::from_entries(entries).expect("drift inventory");

        let evaluation = RepositoryReleaseEvaluator::bundled()
            .expect("bundled policy")
            .evaluate(&inventory, None)
            .expect("valid evaluation");

        assert!(evaluation.findings.iter().any(|finding| {
            finding.rule.rule_id == "EGOLINT_RELEASE_VERSION_AUTHORITY"
                && finding.severity == Severity::Error
        }));
        assert_eq!(evaluation.report.state, ReleaseEvidenceState::Invalid);
    }
}
