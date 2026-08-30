//! Offline validation for the Hygiene repository-presentation profile.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation,
};
use crate::error::{EgolintError, Result};

use super::{AdoptionState, RepositoryEntryKind, RepositoryInventory, RepresentedCommit};

/// Stable tool identifier used by normalized findings and tool results.
pub const TOOL_ID: &str = "EGOLINT_REPOSITORY_PRESENTATION";
/// Dedicated privacy-safe validation artifact.
pub const REPORT_PATH: &str = ".reports/egolint/repository-presentation.json";

const POLICY_CONTRACT: &str = "egolint.repository-presentation-validation/v1";
const REPORT_CONTRACT: &str = "egolint.repository-presentation-report/v1";
const CATALOG_PATH: &str = ".config/rules/repository-presentation.v1.toml";
const CATALOG_SOURCE: &str = include_str!("../../.config/rules/repository-presentation.v1.toml");
const PROFILE_ID: &str = "egohygiene.repository-presentation-profile/v1";
const EVIDENCE_ID: &str = "egohygiene.repository-presentation-evidence/v1";
const PACKAGE_ID: &str = "identity.repository-presentation-package/v1";
const MANIFEST_ID: &str = "identity.repository-presentation-package-manifest/v1";

const CONTRACT_RULE: &str = "EGO-PRESENT-CONTRACT-001";
const APPLICABILITY_RULE: &str = "EGO-PRESENT-APPLICABILITY-001";
const README_RULE: &str = "EGO-PRESENT-README-001";
const BANNER_RULE: &str = "EGO-PRESENT-BANNER-001";
const BADGE_RULE: &str = "EGO-PRESENT-BADGE-001";
const EVIDENCE_RULE: &str = "EGO-PRESENT-EVIDENCE-001";
const LINK_RULE: &str = "EGO-PRESENT-LINK-001";
const MANIFEST_RULE: &str = "EGO-PRESENT-MANIFEST-001";
const GENERATED_RULE: &str = "EGO-PRESENT-GENERATED-001";
const EXCEPTION_RULE: &str = "EGO-PRESENT-EXCEPTION-001";
const EXTERNAL_RULE: &str = "EGO-PRESENT-EXTERNAL-001";

const EXPECTED_RULE_IDS: [&str; 11] = [
    CONTRACT_RULE,
    APPLICABILITY_RULE,
    README_RULE,
    BANNER_RULE,
    BADGE_RULE,
    EVIDENCE_RULE,
    LINK_RULE,
    MANIFEST_RULE,
    GENERATED_RULE,
    EXCEPTION_RULE,
    EXTERNAL_RULE,
];

/// Whether presentation findings fail the enclosing Egolint run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PresentationMode {
    /// Error-level diagnostics remain blocking.
    Blocking,
    /// All diagnostics are capped at warning severity.
    Advisory,
}

/// Exact Hygiene profile lock selected by the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PresentationProfileLock {
    pub id: String,
    pub version: String,
    pub status: String,
    pub owner: String,
    pub source_repository: String,
    pub source_revision: String,
    pub source_path: PathBuf,
    pub digest: String,
}

/// Exact Identity package contract selected by the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PresentationIdentityLock {
    pub package_schema: String,
    pub manifest_schema: String,
    pub version: String,
    pub source_repository: String,
    pub source_revision: String,
}

/// Generated-region ownership markers. Egolint validates but never inserts them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PresentationMarkers {
    pub owner: String,
    pub begin: String,
    pub end: String,
}

/// Repository-owned, versioned presentation validation policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RepositoryPresentationPolicy {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub id: String,
    pub repository: String,
    pub mode: PresentationMode,
    pub repository_type: String,
    pub visibility: String,
    pub lifecycle: String,
    pub readme: PathBuf,
    pub profile_path: PathBuf,
    pub evidence_path: PathBuf,
    pub package_path: PathBuf,
    pub manifest_path: PathBuf,
    pub exceptions_path: Option<PathBuf>,
    pub profile_lock: PresentationProfileLock,
    pub identity_lock: PresentationIdentityLock,
    pub markers: PresentationMarkers,
}

impl RepositoryPresentationPolicy {
    /// Decode and structurally validate a repository policy.
    ///
    /// # Errors
    ///
    /// Returns an error when TOML decoding fails or the policy contains an
    /// unsafe path, unsupported identifier, or malformed immutable pin.
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
                "repository-presentation policy must use schema-version {CONTRACT_VERSION} and id {POLICY_CONTRACT}"
            )));
        }
        if !valid_repository(&self.repository) {
            return Err(EgolintError::Configuration(
                "repository-presentation repository must use owner/name form".to_owned(),
            ));
        }
        for (name, path) in [
            ("README", &self.readme),
            ("profile", &self.profile_path),
            ("evidence", &self.evidence_path),
            ("package", &self.package_path),
            ("manifest", &self.manifest_path),
            ("profile source", &self.profile_lock.source_path),
        ] {
            validate_relative_path(path, name)?;
        }
        if let Some(path) = &self.exceptions_path {
            validate_relative_path(path, "exceptions")?;
        }
        if !valid_commit(&self.profile_lock.source_revision)
            || !valid_commit(&self.identity_lock.source_revision)
        {
            return Err(EgolintError::Configuration(
                "presentation source revisions must be full lowercase Git SHAs".to_owned(),
            ));
        }
        if !valid_digest(&self.profile_lock.digest) {
            return Err(EgolintError::Configuration(
                "profile-lock digest must be a lowercase SHA-256".to_owned(),
            ));
        }
        if self.markers.owner.trim().is_empty()
            || self.markers.begin == self.markers.end
            || !self
                .markers
                .begin
                .contains(&format!("owner={}", self.markers.owner))
            || !self
                .markers
                .begin
                .contains(&format!("profile={}", self.profile_lock.version))
        {
            return Err(EgolintError::Configuration(
                "generated begin marker must name its owner and pinned profile version".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Semantic validity independent of advisory/blocking enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresentationValidationStatus {
    Valid,
    Invalid,
    Incomplete,
    NotApplicable,
}

/// One privacy-safe diagnostic. No source excerpts are retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PresentationDiagnostic {
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

/// Exact bounded counts for Observatory and Repository Intelligence consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PresentationValidationSummary {
    pub diagnostics: u64,
    pub blocking_diagnostics: u64,
    pub required_slots: u64,
    pub present_required_slots: u64,
    pub exceptions_applied: u64,
    pub local_references_checked: u64,
    pub external_references_not_checked: u64,
    pub manifest_files_checked: u64,
}

/// Versioned, privacy-safe evidence artifact for Relay and Observatory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RepositoryPresentationReport {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub contract: String,
    pub catalog_version: String,
    pub repository: String,
    pub policy_path: PathBuf,
    pub mode: PresentationMode,
    pub profile: PresentationProfileLock,
    pub represented_commit: RepresentedCommit,
    pub repository_type: String,
    pub visibility: String,
    pub lifecycle: String,
    pub evidence_state: String,
    pub external_reachability: String,
    pub status: PresentationValidationStatus,
    pub summary: PresentationValidationSummary,
    pub diagnostics: Vec<PresentationDiagnostic>,
}

/// Normalized findings plus the dedicated presentation report.
pub struct PresentationEvaluation {
    pub findings: Vec<Finding>,
    pub report: RepositoryPresentationReport,
}

/// Atomically write the dedicated presentation report in Egolint's report boundary.
///
/// # Errors
///
/// Returns an error when the destination escapes the report boundary or the
/// report cannot be serialized, synchronized, or persisted.
pub fn write_presentation_report_atomic(
    report: &RepositoryPresentationReport,
    path: &Path,
) -> Result<()> {
    if path != Path::new(REPORT_PATH) && !path.ends_with(REPORT_PATH) {
        return Err(EgolintError::Configuration(format!(
            "repository-presentation report path must end with {REPORT_PATH}"
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CatalogContract {
    id: String,
    version: String,
    authority: String,
    source_repository: String,
    source_revision: String,
    source_path: PathBuf,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CatalogRule {
    id: String,
    #[allow(dead_code)]
    title: String,
    default_severity: Severity,
    contracts: Vec<String>,
    remediation: String,
}

struct BundledCatalog {
    version: String,
    owner: String,
    tool_id: String,
    policy_source: String,
    contracts: Vec<CatalogContract>,
    rules: BTreeMap<String, CatalogRule>,
}

impl BundledCatalog {
    fn load() -> Result<Self> {
        let source: BundledCatalogSource = toml::from_str(CATALOG_SOURCE).map_err(|error| {
            EgolintError::Configuration(format!(
                "bundled repository-presentation catalog is invalid: {error}"
            ))
        })?;
        let rule_ids = source
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<BTreeSet<_>>();
        if source.schema_version != CONTRACT_VERSION
            || source.tool_id != TOOL_ID
            || rule_ids != EXPECTED_RULE_IDS.into_iter().collect()
        {
            return Err(EgolintError::Configuration(
                "bundled repository-presentation catalog identity or rule set is invalid"
                    .to_owned(),
            ));
        }
        Ok(Self {
            version: source.catalog_version,
            owner: source.owner,
            tool_id: source.tool_id,
            policy_source: source.policy_source,
            contracts: source.upstream_contracts,
            rules: source
                .rules
                .into_iter()
                .map(|rule| (rule.id.clone(), rule))
                .collect(),
        })
    }
}

#[derive(Default)]
struct EvaluationContext {
    diagnostics: Vec<PresentationDiagnostic>,
    required_slots: usize,
    present_required_slots: usize,
    exceptions_applied: usize,
    local_references_checked: usize,
    external_references_not_checked: usize,
    manifest_files_checked: usize,
    evidence_state: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ExceptionDocument {
    schema: String,
    profile_version: String,
    represented_commit: String,
    exceptions: Vec<SlotException>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotException {
    slot: String,
    reason: String,
    evidence: String,
}

/// Deterministic validator for local presentation structure and evidence linkage.
pub struct RepositoryPresentationEvaluator<'a> {
    policy: &'a RepositoryPresentationPolicy,
    policy_path: PathBuf,
    represented_commit: RepresentedCommit,
    catalog: BundledCatalog,
}

impl<'a> RepositoryPresentationEvaluator<'a> {
    /// Construct an evaluator. Contract drift remains a reportable diagnostic.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy path is unsafe or the bundled catalog
    /// cannot be decoded and verified.
    pub fn new(
        policy: &'a RepositoryPresentationPolicy,
        policy_path: &Path,
        represented_commit: RepresentedCommit,
    ) -> Result<Self> {
        validate_relative_path(policy_path, "repository-presentation policy")?;
        Ok(Self {
            policy,
            policy_path: policy_path.to_path_buf(),
            represented_commit,
            catalog: BundledCatalog::load()?,
        })
    }

    /// Validate README structure, local references, Hygiene evidence, and Identity integrity.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured input cannot be evaluated safely or
    /// a normalized finding/report cannot satisfy its machine contract.
    #[allow(clippy::too_many_lines)]
    pub fn evaluate(&self, inventory: &RepositoryInventory) -> Result<PresentationEvaluation> {
        let mut context = EvaluationContext::default();
        self.validate_catalog_pins(&mut context)?;

        let profile = self.json_entry(
            inventory,
            &self.policy.profile_path,
            CONTRACT_RULE,
            &mut context,
        );
        let evidence = self.json_entry(
            inventory,
            &self.policy.evidence_path,
            EVIDENCE_RULE,
            &mut context,
        );
        let package = self.json_entry(
            inventory,
            &self.policy.package_path,
            MANIFEST_RULE,
            &mut context,
        );
        let manifest = self.json_entry(
            inventory,
            &self.policy.manifest_path,
            MANIFEST_RULE,
            &mut context,
        );
        let readme = self.utf8_entry(inventory, &self.policy.readme, README_RULE, &mut context);
        let exceptions = self.load_exceptions(inventory, &mut context)?;

        let requirements = profile.as_ref().map_or_else(BTreeMap::new, |profile| {
            self.validate_profile(profile, inventory, &mut context)
        });
        let state_messages = profile
            .as_ref()
            .map_or_else(BTreeMap::new, profile_state_messages);
        if let Some(readme) = readme.as_deref() {
            self.validate_readme(readme, inventory, &requirements, &exceptions, &mut context)?;
        }
        if let Some(evidence) = evidence.as_ref() {
            self.validate_evidence(
                evidence,
                &requirements,
                &exceptions,
                &state_messages,
                &mut context,
            )?;
        }
        if let (Some(package), Some(manifest)) = (package.as_ref(), manifest.as_ref()) {
            self.validate_identity_package(
                package,
                manifest,
                inventory,
                readme.as_deref(),
                &mut context,
            )?;
        }

        context.diagnostics.sort_by(diagnostic_order);
        let invalid = context
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id != EXTERNAL_RULE);
        let incomplete_state = matches!(
            context.evidence_state.as_str(),
            "" | "unknown" | "evaluating" | "advisory" | "partial" | "stale" | "blocked"
        ) || self.represented_commit.state == AdoptionState::Unknown;
        let status = if invalid || context.evidence_state == "failing" {
            PresentationValidationStatus::Invalid
        } else if context.evidence_state == "not_applicable" {
            PresentationValidationStatus::NotApplicable
        } else if incomplete_state {
            PresentationValidationStatus::Incomplete
        } else {
            PresentationValidationStatus::Valid
        };
        let findings = context
            .diagnostics
            .iter()
            .map(|diagnostic| self.normalized_finding(diagnostic))
            .collect::<Vec<_>>();
        for finding in &findings {
            finding.validate()?;
        }
        let report = RepositoryPresentationReport {
            schema_version: CONTRACT_VERSION,
            contract: REPORT_CONTRACT.to_owned(),
            catalog_version: self.catalog.version.clone(),
            repository: self.policy.repository.clone(),
            policy_path: self.policy_path.clone(),
            mode: self.policy.mode,
            profile: self.policy.profile_lock.clone(),
            represented_commit: self.represented_commit.clone(),
            repository_type: self.policy.repository_type.clone(),
            visibility: self.policy.visibility.clone(),
            lifecycle: self.policy.lifecycle.clone(),
            evidence_state: context.evidence_state.clone(),
            external_reachability: if context.external_references_not_checked == 0 {
                "not_applicable".to_owned()
            } else {
                "not_checked".to_owned()
            },
            status,
            summary: PresentationValidationSummary {
                diagnostics: context.diagnostics.len() as u64,
                blocking_diagnostics: context
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        matches!(diagnostic.severity, Severity::Error | Severity::Critical)
                    })
                    .count() as u64,
                required_slots: context.required_slots as u64,
                present_required_slots: context.present_required_slots as u64,
                exceptions_applied: context.exceptions_applied as u64,
                local_references_checked: context.local_references_checked as u64,
                external_references_not_checked: context.external_references_not_checked as u64,
                manifest_files_checked: context.manifest_files_checked as u64,
            },
            diagnostics: context.diagnostics,
        };
        Ok(PresentationEvaluation { findings, report })
    }

    fn validate_catalog_pins(&self, context: &mut EvaluationContext) -> Result<()> {
        let hygiene = self
            .catalog
            .contracts
            .iter()
            .find(|contract| contract.id == PROFILE_ID)
            .expect("catalog requires Hygiene profile");
        let identity = self
            .catalog
            .contracts
            .iter()
            .find(|contract| contract.id == MANIFEST_ID)
            .expect("catalog requires Identity manifest");
        let lock = &self.policy.profile_lock;
        if lock.id != hygiene.id
            || lock.version != hygiene.version
            || lock.status != hygiene.authority
            || lock.source_repository != hygiene.source_repository
            || lock.source_revision != hygiene.source_revision
            || lock.source_path != hygiene.source_path
            || Some(lock.digest.as_str()) != hygiene.digest.as_deref()
        {
            self.push(
                context,
                CONTRACT_RULE,
                Some(location(&self.policy_path)),
                "exact supported Hygiene profile pin",
                "contract pin drift",
                "The policy profile lock differs from Egolint's supported immutable Hygiene input.",
            )?;
        }
        let lock = &self.policy.identity_lock;
        if lock.manifest_schema != identity.id
            || lock.package_schema != PACKAGE_ID
            || lock.version != identity.version
            || lock.source_repository != identity.source_repository
            || lock.source_revision != identity.source_revision
        {
            self.push(
                context,
                CONTRACT_RULE,
                Some(location(&self.policy_path)),
                "exact supported Identity package contracts",
                "contract pin drift",
                "The policy Identity lock differs from Egolint's supported immutable package input.",
            )?;
        }
        Ok(())
    }

    fn validate_profile(
        &self,
        profile: &Value,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> BTreeMap<String, String> {
        let entry = inventory.get(&self.policy.profile_path);
        let digest = entry.map_or_else(String::new, |entry| normalized_digest(&entry.content));
        let metadata_matches = string(profile, "schema")
            == Some(self.policy.profile_lock.id.as_str())
            && string(profile, "version") == Some(self.policy.profile_lock.version.as_str())
            && string(profile, "status") == Some(self.policy.profile_lock.status.as_str())
            && string(profile, "owner") == Some(self.policy.profile_lock.owner.as_str())
            && digest == self.policy.profile_lock.digest;
        if !metadata_matches {
            let _ = self.push(
                context,
                CONTRACT_RULE,
                Some(location(&self.policy.profile_path)),
                "profile metadata and normalized digest equal the immutable lock",
                "profile bytes or metadata drift",
                "The vendored Hygiene profile is not the exact policy-locked input.",
            );
        }
        let repository_types = string_array(profile.get("repository_types"));
        let visibilities = string_array(profile.get("visibilities"));
        let lifecycles = string_array(profile.get("lifecycles"));
        for (name, value, allowed) in [
            (
                "repository type",
                self.policy.repository_type.as_str(),
                &repository_types,
            ),
            ("visibility", self.policy.visibility.as_str(), &visibilities),
            ("lifecycle", self.policy.lifecycle.as_str(), &lifecycles),
        ] {
            if !allowed.contains(value) {
                let _ = self.push(
                    context,
                    APPLICABILITY_RULE,
                    Some(location(&self.policy_path)),
                    format!("Hygiene-supported {name}"),
                    value,
                    format!("The declared {name} is absent from the pinned Hygiene vocabulary."),
                );
            }
        }
        let mut requirements = BTreeMap::new();
        if let Some(slots) = profile.get("slots").and_then(Value::as_array) {
            for slot in slots {
                if let (Some(id), Some(requirement)) =
                    (string(slot, "id"), string(slot, "default_requirement"))
                {
                    requirements.insert(id.to_owned(), requirement.to_owned());
                }
            }
        }
        for (group, axis) in [
            ("type_overrides", self.policy.repository_type.as_str()),
            ("visibility_overrides", self.policy.visibility.as_str()),
            ("lifecycle_overrides", self.policy.lifecycle.as_str()),
        ] {
            if let Some(overrides) = profile
                .get(group)
                .and_then(|value| value.get(axis))
                .and_then(Value::as_object)
            {
                for (slot, requirement) in overrides {
                    if let Some(requirement) = requirement.as_str() {
                        requirements.insert(slot.clone(), requirement.to_owned());
                    }
                }
            }
        }
        let policy = profile.get("claim_policy");
        let claim_contract_supported = policy.and_then(|value| string(value, "badge_label"))
            == Some("Hygienic")
            && policy
                .and_then(|value| value.get("represented_commit_required"))
                .and_then(Value::as_bool)
                == Some(true)
            && policy
                .and_then(|value| value.get("evidence_url_required"))
                .and_then(Value::as_bool)
                == Some(true)
            && policy
                .and_then(|value| value.get("unknown_fails_closed"))
                .and_then(Value::as_bool)
                == Some(true)
            && !profile_state_messages(profile).is_empty();
        if !claim_contract_supported {
            let _ = self.push(
                context,
                CONTRACT_RULE,
                Some(location(&self.policy.profile_path)),
                "supported Hygiene evidence and claim policy",
                "unsupported profile semantics",
                "The pinned profile does not expose the evidence-bound fail-closed contract Egolint requires.",
            );
        }
        requirements
    }

    #[allow(clippy::too_many_lines)]
    fn validate_readme(
        &self,
        readme: &str,
        inventory: &RepositoryInventory,
        requirements: &BTreeMap<String, String>,
        exceptions: &BTreeMap<String, SlotException>,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let headings = markdown_headings(readme);
        for slot in exceptions.keys() {
            if !requirements.contains_key(slot) {
                self.push(
                    context,
                    EXCEPTION_RULE,
                    Some(location(
                        &self.policy.exceptions_path.clone().unwrap_or_default(),
                    )),
                    "exception names a slot in the pinned profile",
                    "unknown slot",
                    "An exception names a slot that the pinned Hygiene profile does not define.",
                )?;
            }
        }
        for (slot, requirement) in requirements {
            if requirement != "required" {
                continue;
            }
            context.required_slots += 1;
            let marker = format!("<!-- repository-presentation:slot {slot} -->");
            let present = readme.contains(&marker)
                || headings.contains(&slot.replace('_', " "))
                || headings.contains(&slot_title_fallback(slot));
            if present {
                context.present_required_slots += 1;
            } else if exceptions.contains_key(slot) {
                context.exceptions_applied += 1;
            } else {
                self.push(
                    context,
                    README_RULE,
                    Some(location(&self.policy.readme)),
                    format!("required slot {slot} is marked or headed"),
                    "missing",
                    format!("The resolved Hygiene profile requires the {slot} README slot."),
                )?;
            }
        }

        let begins = readme.matches(&self.policy.markers.begin).count();
        let ends = readme.matches(&self.policy.markers.end).count();
        if begins != 1 || ends != 1 {
            self.push(
                context,
                GENERATED_RULE,
                Some(location(&self.policy.readme)),
                "one balanced owner/profile-marked generated region",
                format!("{begins} begin markers and {ends} end markers"),
                "Generated presentation blocks are missing, duplicated, or unbalanced.",
            )?;
        } else if readme.find(&self.policy.markers.begin) > readme.find(&self.policy.markers.end) {
            self.push(
                context,
                GENERATED_RULE,
                Some(location(&self.policy.readme)),
                "begin marker precedes end marker",
                "reversed marker order",
                "The generated presentation region has invalid marker order.",
            )?;
        }

        let references = markdown_references(readme);
        for reference in references {
            if reference.starts_with("https://") {
                context.external_references_not_checked += 1;
                continue;
            }
            if reference.starts_with('#') || reference.starts_with("mailto:") {
                continue;
            }
            context.local_references_checked += 1;
            if let Some(path) = resolve_reference(&self.policy.readme, &reference) {
                if inventory.get(&path).is_none() && !inventory.contains_directory(&path) {
                    self.push(
                        context,
                        LINK_RULE,
                        Some(location(&self.policy.readme)),
                        "existing repository-relative destination",
                        portable_path(&path),
                        "A local README destination does not resolve in the repository inventory.",
                    )?;
                }
            } else {
                self.push(
                    context,
                    LINK_RULE,
                    Some(location(&self.policy.readme)),
                    "safe repository-relative, anchor, mailto, or HTTPS destination",
                    "unsupported destination",
                    "A README destination is unsafe or uses an unsupported scheme.",
                )?;
            }
        }
        if context.external_references_not_checked > 0 {
            self.push(
                context,
                EXTERNAL_RULE,
                Some(location(&self.policy.readme)),
                "separate authorized network validation",
                format!(
                    "{} hosted destinations intentionally not checked",
                    context.external_references_not_checked
                ),
                "Deterministic validation does not claim that hosted destinations are reachable.",
            )?;
        }
        Ok(())
    }

    fn validate_evidence(
        &self,
        evidence: &Value,
        requirements: &BTreeMap<String, String>,
        exceptions: &BTreeMap<String, SlotException>,
        messages: &BTreeMap<String, String>,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let profile = evidence.get("profile");
        let repository = evidence.get("repository");
        let assessment = evidence.get("assessment");
        let badge = evidence.get("badge");
        let state = badge.and_then(|value| string(value, "state")).unwrap_or("");
        state.clone_into(&mut context.evidence_state);
        let represented = self.represented_commit.revision.as_deref();
        let expected_message = messages.get(state).map(String::as_str);
        let valid = string(evidence, "schema") == Some(EVIDENCE_ID)
            && profile.and_then(|value| string(value, "id"))
                == Some(self.policy.profile_lock.id.as_str())
            && profile.and_then(|value| string(value, "version"))
                == Some(self.policy.profile_lock.version.as_str())
            && profile.and_then(|value| string(value, "status"))
                == Some(self.policy.profile_lock.status.as_str())
            && repository.and_then(|value| string(value, "name"))
                == Some(self.policy.repository.as_str())
            && repository.and_then(|value| string(value, "type"))
                == Some(self.policy.repository_type.as_str())
            && repository.and_then(|value| string(value, "visibility"))
                == Some(self.policy.visibility.as_str())
            && repository.and_then(|value| string(value, "lifecycle"))
                == Some(self.policy.lifecycle.as_str())
            && represented.is_none_or(|revision| {
                repository.and_then(|value| string(value, "represented_commit")) == Some(revision)
            })
            && assessment.and_then(|value| string(value, "state")) == Some(state)
            && badge.and_then(|value| string(value, "label")) == Some("Hygienic")
            && badge.and_then(|value| string(value, "message")) == expected_message
            && badge.and_then(|value| string(value, "profile_version"))
                == Some(self.policy.profile_lock.version.as_str())
            && represented.is_none_or(|revision| {
                badge.and_then(|value| string(value, "represented_commit")) == Some(revision)
            })
            && badge
                .and_then(|value| string(value, "evidence_url"))
                .is_some_and(valid_reference);
        if !valid {
            self.push(
                context,
                EVIDENCE_RULE,
                Some(location(&self.policy.evidence_path)),
                "evidence, profile, axes, state, badge, URL, and represented commit agree",
                "inconsistent evidence binding",
                "The Hygiene evidence document is malformed or disagrees with its policy inputs.",
            )?;
        }
        if state == "passing" {
            let slots = evidence
                .get("slots")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for (slot, requirement) in requirements {
                if requirement != "required" || exceptions.contains_key(slot) {
                    continue;
                }
                let item = slots.iter().find(|item| string(item, "id") == Some(slot));
                let current = item.is_some_and(|item| {
                    string(item, "requirement") == Some(requirement)
                        && string(item, "state") == Some("passing")
                        && item
                            .get("evidence")
                            .and_then(Value::as_array)
                            .is_some_and(|records| {
                                !records.is_empty()
                                    && records.iter().all(|record| {
                                        string(record, "freshness") == Some("current")
                                    })
                            })
                });
                if !current {
                    self.push(
                        context,
                        BADGE_RULE,
                        Some(location(&self.policy.evidence_path)),
                        format!("current passing evidence for required slot {slot}"),
                        "missing, stale, or non-passing",
                        "A passing badge cannot be derived while a required slot lacks current passing evidence.",
                    )?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn validate_identity_package(
        &self,
        package: &Value,
        manifest: &Value,
        inventory: &RepositoryInventory,
        readme: Option<&str>,
        context: &mut EvaluationContext,
    ) -> Result<()> {
        let profile = package.get("profile");
        let project = package.get("project");
        let badge = package.get("badge");
        let boundary = package.get("consumerBoundary");
        let manifest_profile = manifest.get("profile");
        let manifest_evidence = manifest.get("evidence");
        let represented = self.represented_commit.revision.as_deref();
        let structurally_bound = string(package, "schema") == Some(PACKAGE_ID)
            && string(package, "version") == Some(self.policy.identity_lock.version.as_str())
            && string(manifest, "schema") == Some(MANIFEST_ID)
            && string(manifest, "version") == Some(self.policy.identity_lock.version.as_str())
            && string(manifest, "projectionSchema") == Some(PACKAGE_ID)
            && project.and_then(|value| string(value, "visibility"))
                == Some(self.policy.visibility.as_str())
            && profile.and_then(|value| string(value, "id"))
                == Some(self.policy.profile_lock.id.as_str())
            && profile.and_then(|value| string(value, "version"))
                == Some(self.policy.profile_lock.version.as_str())
            && profile.and_then(|value| string(value, "status"))
                == Some(self.policy.profile_lock.status.as_str())
            && profile.and_then(|value| string(value, "commit"))
                == Some(self.policy.profile_lock.source_revision.as_str())
            && profile.and_then(|value| string(value, "sha256"))
                == Some(self.policy.profile_lock.digest.as_str())
            && manifest_profile.and_then(|value| string(value, "digest"))
                == Some(self.policy.profile_lock.digest.as_str())
            && badge.and_then(|value| string(value, "label")) == Some("Hygienic")
            && badge.and_then(|value| string(value, "profileVersion"))
                == Some(self.policy.profile_lock.version.as_str())
            && badge.and_then(|value| string(value, "state"))
                == manifest_evidence.and_then(|value| string(value, "state"))
            && badge.and_then(|value| string(value, "evidenceUrl"))
                == manifest_evidence.and_then(|value| string(value, "url"))
            && represented.is_none_or(|revision| {
                badge.and_then(|value| string(value, "representedCommit")) == Some(revision)
            })
            && represented.is_none_or(|revision| {
                manifest_evidence.and_then(|value| string(value, "representedCommit"))
                    == Some(revision)
            })
            && boundary
                .and_then(|value| value.get("editsReadme"))
                .and_then(Value::as_bool)
                == Some(false)
            && boundary
                .and_then(|value| value.get("evaluatesEvidence"))
                .and_then(Value::as_bool)
                == Some(false)
            && boundary
                .and_then(|value| value.get("networkRequired"))
                .and_then(Value::as_bool)
                == Some(false)
            && boundary
                .and_then(|value| value.get("generatedRegionsOnly"))
                .and_then(Value::as_bool)
                == Some(true);
        if !structurally_bound {
            self.push(
                context,
                MANIFEST_RULE,
                Some(location(&self.policy.manifest_path)),
                "package and manifest bind exact profile, evidence, visibility, and consumer boundary",
                "binding mismatch",
                "Identity package metadata is stale, unsupported, or inconsistent with repository policy.",
            )?;
        }

        let package_directory = self
            .policy
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new(""));
        if let Some(files) = manifest.get("files").and_then(Value::as_object) {
            for (relative, metadata) in files {
                let Some(path) = safe_join(package_directory, relative) else {
                    self.push(
                        context,
                        MANIFEST_RULE,
                        Some(location(&self.policy.manifest_path)),
                        "safe package-relative manifest path",
                        "unsafe path",
                        "Identity manifest contains an unsafe file path.",
                    )?;
                    continue;
                };
                context.manifest_files_checked += 1;
                let matches = inventory.get(&path).is_some_and(|entry| {
                    let digest = sha256(&entry.content);
                    string(metadata, "sha256") == Some(digest.as_str())
                        && metadata.get("bytes").and_then(Value::as_u64)
                            == Some(entry.content.len() as u64)
                });
                if !matches {
                    self.push(
                        context,
                        MANIFEST_RULE,
                        Some(location(&path)),
                        "manifest-bound bytes and SHA-256",
                        "missing or manually edited file",
                        "A generated Identity file no longer matches its integrity manifest.",
                    )?;
                }
            }
        } else {
            self.push(
                context,
                MANIFEST_RULE,
                Some(location(&self.policy.manifest_path)),
                "non-empty manifest file map",
                "missing",
                "Identity manifest does not enumerate generated files.",
            )?;
        }

        if let (Some(readme), Some(banner), Some(badge)) = (readme, package.get("banner"), badge) {
            let banner_path = banner
                .get("variants")
                .and_then(Value::as_array)
                .and_then(|variants| variants.first())
                .and_then(|variant| string(variant, "svg"))
                .and_then(|path| safe_join(package_directory, path));
            let badge_path =
                string(badge, "svg").and_then(|path| safe_join(package_directory, path));
            let alt_ok = string(banner, "altText").is_some_and(|value| !value.trim().is_empty());
            let fallback_ok =
                string(banner, "fallbackText").is_some_and(|value| !value.trim().is_empty());
            let banner_used = banner_path.as_ref().is_some_and(|path| {
                markdown_images(readme).iter().any(|(alt, destination)| {
                    !alt.trim().is_empty()
                        && resolve_reference(&self.policy.readme, destination).as_ref()
                            == Some(path)
                })
            });
            if !alt_ok || !fallback_ok || !banner_used {
                self.push(context, BANNER_RULE, Some(location(&self.policy.readme)), "manifest-bound local banner with alt text and fallback", "missing or inaccessible banner binding", "README banner presentation does not match accessible Identity package metadata.")?;
            }
            let badge_used = badge_path.as_ref().is_some_and(|path| {
                markdown_images(readme).iter().any(|(_, destination)| {
                    resolve_reference(&self.policy.readme, destination).as_ref() == Some(path)
                })
            });
            let badge_bound = string(badge, "label") == Some("Hygienic")
                && string(badge, "profileVersion")
                    == Some(self.policy.profile_lock.version.as_str())
                && string(badge, "evidenceUrl").is_some_and(valid_reference);
            if !badge_used || !badge_bound {
                self.push(context, BADGE_RULE, Some(location(&self.policy.readme)), "local Identity badge bound to profile and evidence", "missing or inconsistent badge binding", "README badge presentation is not tied to the Identity descriptor and Hygiene evidence.")?;
            }
        }
        Ok(())
    }

    fn load_exceptions(
        &self,
        inventory: &RepositoryInventory,
        context: &mut EvaluationContext,
    ) -> Result<BTreeMap<String, SlotException>> {
        let Some(path) = &self.policy.exceptions_path else {
            return Ok(BTreeMap::new());
        };
        let Some(entry) = inventory.get(path) else {
            self.push(
                context,
                EXCEPTION_RULE,
                Some(location(path)),
                "declared versioned exception document",
                "missing",
                "The policy names an exception document that is absent.",
            )?;
            return Ok(BTreeMap::new());
        };
        let Ok(document) = serde_json::from_slice::<ExceptionDocument>(&entry.content) else {
            self.push(
                context,
                EXCEPTION_RULE,
                Some(location(path)),
                "valid privacy-safe exception JSON",
                "malformed",
                "The exception document does not match the supported structure.",
            )?;
            return Ok(BTreeMap::new());
        };
        let represented_matches = self
            .represented_commit
            .revision
            .as_deref()
            .is_none_or(|revision| document.represented_commit == revision);
        if document.schema != "egolint.repository-presentation-exceptions/v1"
            || document.profile_version != self.policy.profile_lock.version
            || !represented_matches
        {
            self.push(
                context,
                EXCEPTION_RULE,
                Some(location(path)),
                "exception document bound to profile and represented commit",
                "stale or unsupported",
                "The exception envelope does not bind the current validation inputs.",
            )?;
            return Ok(BTreeMap::new());
        }
        let mut exceptions = BTreeMap::new();
        for exception in document.exceptions {
            if exception.reason.trim().len() < 8
                || !valid_reference(&exception.evidence)
                || exceptions.contains_key(&exception.slot)
            {
                self.push(
                    context,
                    EXCEPTION_RULE,
                    Some(location(path)),
                    "unique slot exception with durable reason and evidence",
                    "invalid exception",
                    "An exception is duplicated, unevidenced, or lacks a meaningful reason.",
                )?;
            } else {
                exceptions.insert(exception.slot.clone(), exception);
            }
        }
        Ok(exceptions)
    }

    fn utf8_entry(
        &self,
        inventory: &RepositoryInventory,
        path: &Path,
        rule: &str,
        context: &mut EvaluationContext,
    ) -> Option<String> {
        let Some(entry) = inventory.get(path) else {
            let _ = self.push(
                context,
                rule,
                Some(location(path)),
                "existing UTF-8 file",
                "missing",
                "A required presentation input is absent.",
            );
            return None;
        };
        if entry.kind != RepositoryEntryKind::File {
            let _ = self.push(
                context,
                rule,
                Some(location(path)),
                "regular file",
                "symbolic link",
                "Presentation policy inputs must be repository-owned regular files.",
            );
            return None;
        }
        let Ok(value) = std::str::from_utf8(&entry.content) else {
            let _ = self.push(
                context,
                rule,
                Some(location(path)),
                "UTF-8 file",
                "non-UTF-8 bytes",
                "Presentation metadata and README files must be UTF-8.",
            );
            return None;
        };
        Some(value.to_owned())
    }

    fn json_entry(
        &self,
        inventory: &RepositoryInventory,
        path: &Path,
        rule: &str,
        context: &mut EvaluationContext,
    ) -> Option<Value> {
        let value = self.utf8_entry(inventory, path, rule, context)?;
        let Ok(value) = serde_json::from_str(&value) else {
            let _ = self.push(
                context,
                rule,
                Some(location(path)),
                "valid JSON document",
                "malformed JSON",
                "The presentation input cannot be decoded safely.",
            );
            return None;
        };
        Some(value)
    }

    fn push(
        &self,
        context: &mut EvaluationContext,
        rule_id: &str,
        location: Option<SourceLocation>,
        expected_state: impl Into<String>,
        actual_state: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<()> {
        let definition = self.catalog.rules.get(rule_id).ok_or_else(|| {
            EgolintError::Configuration(format!("unknown bundled presentation rule {rule_id}"))
        })?;
        let severity = match self.policy.mode {
            PresentationMode::Blocking => definition.default_severity,
            PresentationMode::Advisory => match definition.default_severity {
                Severity::Info => Severity::Info,
                Severity::Warning | Severity::Error | Severity::Critical => Severity::Warning,
            },
        };
        let expected_state = expected_state.into();
        let actual_state = actual_state.into();
        let message = message.into();
        let fingerprint =
            stable_fingerprint(rule_id, location.as_ref(), &expected_state, &actual_state);
        context.diagnostics.push(PresentationDiagnostic {
            id: format!("{rule_id}-{fingerprint}"),
            rule_id: rule_id.to_owned(),
            severity,
            location,
            expected_state,
            actual_state,
            message,
            remediation: definition.remediation.clone(),
            contracts: definition.contracts.clone(),
        });
        Ok(())
    }

    fn normalized_finding(&self, diagnostic: &PresentationDiagnostic) -> Finding {
        Finding {
            schema_version: CONTRACT_VERSION,
            id: diagnostic.id.clone(),
            rule: RuleIdentity {
                tool_id: self.catalog.tool_id.clone(),
                rule_id: diagnostic.rule_id.clone(),
            },
            severity: diagnostic.severity,
            message: format!(
                "{} Remediation: {}",
                diagnostic.message, diagnostic.remediation
            ),
            location: diagnostic.location.clone(),
            ownership: RuleOwnership {
                owner: self.catalog.owner.clone(),
                policy_source: format!("{}#{}", self.catalog.policy_source, diagnostic.rule_id),
                configuration_path: Some(self.policy_path.clone()),
            },
            fingerprint: Some(stable_fingerprint(
                &diagnostic.rule_id,
                diagnostic.location.as_ref(),
                &diagnostic.expected_state,
                &diagnostic.actual_state,
            )),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Policy,
                path: PathBuf::from(CATALOG_PATH),
                sha256: None,
                description: Some(format!(
                    "Egolint presentation rule mapped to {}.",
                    diagnostic.contracts.join(", ")
                )),
            }],
            suppressed_by: None,
        }
    }
}

fn string<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn string_array(value: Option<&Value>) -> BTreeSet<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn profile_state_messages(profile: &Value) -> BTreeMap<String, String> {
    profile
        .get("claim_policy")
        .and_then(|value| value.get("state_messages"))
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(state, message)| {
            message
                .as_str()
                .map(|message| (state.clone(), message.to_owned()))
        })
        .collect()
}

fn normalized_digest(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    sha256(text.as_bytes())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_none()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
}

fn validate_relative_path(path: &Path, name: &str) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.to_string_lossy().contains('\\') {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a safe repository-relative path"
        )));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "{name} must not contain traversal or dot components"
        )));
    }
    Ok(())
}

fn location(path: &Path) -> SourceLocation {
    SourceLocation {
        path: path.to_path_buf(),
        start_line: None,
        start_column: None,
        end_line: None,
        end_column: None,
    }
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn safe_join(base: &Path, value: &str) -> Option<PathBuf> {
    if value.is_empty() || value.contains('\\') {
        return None;
    }
    let mut result = base.to_path_buf();
    for component in Path::new(value).components() {
        match component {
            Component::Normal(component) => result.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(result)
}

fn resolve_reference(readme: &Path, reference: &str) -> Option<PathBuf> {
    let value = reference.trim().trim_matches(['<', '>']);
    let value = value.split(['#', '?']).next().unwrap_or("");
    if value.is_empty() || value.contains(':') || value.contains('%') {
        return None;
    }
    safe_join(readme.parent().unwrap_or_else(|| Path::new("")), value)
}

fn valid_reference(value: &str) -> bool {
    value.starts_with("https://")
        || (!value.is_empty()
            && !value.starts_with('/')
            && !value.contains("..")
            && !value.contains('\\')
            && !value.contains(':'))
}

fn markdown_references(readme: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = readme;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let destination = rest[..end]
            .split_ascii_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(['<', '>']);
        if !destination.is_empty() {
            values.push(destination.to_owned());
        }
        rest = &rest[end + 1..];
    }
    values
}

fn markdown_images(readme: &str) -> Vec<(String, String)> {
    let mut images = Vec::new();
    let mut rest = readme;
    while let Some(start) = rest.find("![") {
        rest = &rest[start + 2..];
        let Some(alt_end) = rest.find("](") else {
            break;
        };
        let alt = rest[..alt_end].to_owned();
        rest = &rest[alt_end + 2..];
        let Some(destination_end) = rest.find(')') else {
            break;
        };
        let destination = rest[..destination_end]
            .split_ascii_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(['<', '>'])
            .to_owned();
        images.push((alt, destination));
        rest = &rest[destination_end + 1..];
    }
    images
}

fn markdown_headings(readme: &str) -> BTreeSet<String> {
    readme
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('#'))
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .trim_end_matches('#')
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn slot_title_fallback(slot: &str) -> String {
    match slot {
        "identity_banner" => "repository identity banner",
        "maturity_status" => "maturity and status",
        "support_boundary" => "support boundary",
        "canonical_navigation" => "canonical navigation",
        "evidence_badges" => "evidence-backed badges",
        "development" => "local development",
        "license" => "license and notices",
        "security" => "security reporting",
        "generated_ownership" => "generated content ownership",
        value => value,
    }
    .to_owned()
}

fn stable_fingerprint(
    rule_id: &str,
    location: Option<&SourceLocation>,
    expected: &str,
    actual: &str,
) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let path = location.map_or_else(String::new, |value| portable_path(&value.path));
    for byte in rule_id
        .bytes()
        .chain([0])
        .chain(path.bytes())
        .chain([0])
        .chain(expected.bytes())
        .chain([0])
        .chain(actual.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("repository-presentation-v1-{hash:016x}")
}

fn diagnostic_order(
    left: &PresentationDiagnostic,
    right: &PresentationDiagnostic,
) -> std::cmp::Ordering {
    left.location
        .as_ref()
        .map(|value| &value.path)
        .cmp(&right.location.as_ref().map(|value| &value.path))
        .then_with(|| left.rule_id.cmp(&right.rule_id))
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(mode: &str) -> RepositoryPresentationPolicy {
        RepositoryPresentationPolicy::from_toml(
            &format!(
                r#"schema-version = 1
id = "egolint.repository-presentation-validation/v1"
repository = "egohygiene/example"
mode = "{mode}"
repository-type = "tool"
visibility = "public"
lifecycle = "active"
readme = "README.md"
profile-path = "vendor/hygiene/profile.json"
evidence-path = "evidence/presentation.json"
package-path = "assets/identity/repository-presentation.json"
manifest-path = "assets/identity/repository-presentation-manifest.json"

[profile-lock]
id = "egohygiene.repository-presentation-profile/v1"
version = "1.0.0-alpha.1"
status = "proposed"
owner = "egohygiene/hygiene"
source-repository = "egohygiene/hygiene"
source-revision = "cb2ed63425d29abada2d2bbb43a3b3e59d11aeb8"
source-path = "catalog/repository-presentation-profile.json"
digest = "44e0881519350e6747723995939c79c6fb4659e38a74b2c32e409866e7a186ba"

[identity-lock]
package-schema = "identity.repository-presentation-package/v1"
manifest-schema = "identity.repository-presentation-package-manifest/v1"
version = "1.0.0"
source-repository = "egohygiene/identity"
source-revision = "3c2fd3141371b355628e81f66f63159f19d63338"

[markers]
owner = "egohygiene/identity"
begin = "<!-- repository-presentation:begin owner=egohygiene/identity profile=1.0.0-alpha.1 -->"
end = "<!-- repository-presentation:end -->"
"#,
            ),
            Path::new(".config/egolint/repository-presentation.toml"),
        )
        .expect("valid policy")
    }

    #[test]
    fn advisory_mode_caps_semantic_diagnostics_without_changing_status() {
        let policy = policy("advisory");
        let evaluator = RepositoryPresentationEvaluator::new(
            &policy,
            Path::new(".config/egolint/repository-presentation.toml"),
            RepresentedCommit::parse("unknown").expect("represented commit"),
        )
        .expect("evaluator");
        let inventory = RepositoryInventory::from_entries(Vec::new()).expect("inventory");
        let evaluation = evaluator.evaluate(&inventory).expect("evaluation");
        assert_eq!(
            evaluation.report.status,
            PresentationValidationStatus::Invalid
        );
        assert!(
            evaluation
                .findings
                .iter()
                .all(|finding| !matches!(finding.severity, Severity::Error | Severity::Critical))
        );
        assert!(
            evaluation
                .report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message.len() < 512)
        );
    }

    #[test]
    fn fixture_names_cover_required_rollout_profiles() {
        let names = include_str!("../../tests/fixtures/repository-presentation/scenarios.txt");
        for expected in [
            "minimal",
            "customized",
            "private",
            "archived",
            "partial",
            "broken",
            "fully-conformant",
        ] {
            assert!(names.lines().any(|line| line == expected));
        }
    }
}
