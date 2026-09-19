//! Mechanical, offline checks for the source-pinned repository-release policy.

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation,
};
use crate::error::{EgolintError, Result};

use super::{
    AETHER_CONTRACT_ID, AETHER_SCHEMA_URL, DECLARATION_PATH, POLICY_PATH, ReleaseCheckResult,
    ReleaseCheckState, ReleaseLifecycle, ReleasePolicyReference, ReleaseRequirement,
    ReleaseSlotApplicability, RepositoryEntryKind, RepositoryInventory, RepositoryReleaseProfile,
    TOOL_ID, digest, stable_fingerprint, valid_repository,
};

const AGENTS_RULE: &str = "EGOLINT_RELEASE_AGENTS_PROFILE_POINTER";
const DECLARATION_RULE: &str = "EGOLINT_RELEASE_AETHER_DECLARATION";
const CHANGELOG_RULE: &str = "EGOLINT_RELEASE_CHANGELOG";
const WORKFLOW_RULE: &str = "EGOLINT_RELEASE_MANUAL_WORKFLOW";
const ROLLBACK_RULE: &str = "EGOLINT_RELEASE_ROLLBACK_DOCS";
const TASKS_RULE: &str = "EGOLINT_RELEASE_TASK_HANDOFFS";
const VERSION_RULE: &str = "EGOLINT_RELEASE_VERSION_AUTHORITY";

pub(super) fn rule_id_for_slot(slot: &str) -> &'static str {
    match slot {
        "agents_profile_pointer" => AGENTS_RULE,
        "aether_declaration" => DECLARATION_RULE,
        "changelog" => CHANGELOG_RULE,
        "manual_workflow" => WORKFLOW_RULE,
        "release_rollback_docs" => ROLLBACK_RULE,
        "task_handoffs" => TASKS_RULE,
        "version_authority" => VERSION_RULE,
        _ => "EGOLINT_RELEASE_UNKNOWN_POLICY_SLOT",
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseDeclaration {
    #[serde(rename = "$schema")]
    schema: Option<String>,
    schema_version: String,
    pub(super) repository: DeclarationRepository,
    pub(super) release: DeclarationRelease,
    pub(super) changelog: DeclarationChangelog,
    pub(super) components: Vec<DeclarationComponent>,
    delivery: DeclarationDelivery,
    pub(super) evidence: DeclarationEvidence,
    pub(super) automation: DeclarationAutomation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationRepository {
    pub(super) id: String,
    pub(super) lifecycle: ReleaseLifecycle,
    pub(super) release_profile: RepositoryReleaseProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationRelease {
    pub(super) state: ReleaseState,
    tag_prefix: String,
    immutable_tags: bool,
    major_alias: MajorAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReleaseState {
    Unreleased,
    Released,
    Frozen,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum MajorAlias {
    Disabled,
    Optional,
    Enabled,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationChangelog {
    pub(super) path: PathBuf,
    format: String,
    unreleased_heading: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationComponent {
    id: String,
    kind: ComponentKind,
    pub(super) version_authority: VersionAuthority,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ComponentKind {
    Repository,
    Crate,
    PythonPackage,
    NpmPackage,
    ContainerImage,
    StaticSite,
    Publication,
    Catalog,
    Workspace,
    Internal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VersionAuthority {
    pub(super) kind: VersionAuthorityKind,
    pub(super) path: Option<PathBuf>,
    pub(super) selector: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum VersionAuthorityKind {
    GitTag,
    CargoManifest,
    PyprojectProject,
    PackageJson,
    ContainerTag,
    PublicationMetadata,
    CatalogRecord,
    WorkspaceManifest,
    External,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationDelivery {
    channels: Vec<DeliveryChannel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliveryChannel {
    kind: DeliveryKind,
    state: DeliveryState,
    relay_profile: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DeliveryKind {
    GithubRelease,
    PackageRegistry,
    ContainerRegistry,
    SiteDeployment,
    PublicationArchive,
    InternalDistribution,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DeliveryState {
    Planned,
    Configured,
    External,
    Unavailable,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EvidenceState {
    Required,
    Available,
    External,
    Unavailable,
    NotApplicable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationEvidence {
    source: EvidenceState,
    change: EvidenceState,
    provenance: EvidenceState,
    sbom: EvidenceState,
    signature: EvidenceState,
    pub(super) rollback: Rollback,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rollback {
    strategy: RollbackStrategy,
    instructions: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RollbackStrategy {
    RevertAndSuccessorTag,
    RevokeChannel,
    RedeployPriorArtifact,
    Freeze,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationAutomation {
    pub(super) taskfile_path: PathBuf,
    pub(super) tasks: DeclarationTasks,
    pub(super) github: DeclarationGithub,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationTasks {
    plan: String,
    prepare: String,
    verify: String,
    publish: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationGithub {
    manual_dispatch_required: bool,
    pub(super) workflow_path: PathBuf,
    pub(super) state: GithubAutomationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum GithubAutomationState {
    Planned,
    Configured,
    Unavailable,
}

pub(super) fn parse_declaration(
    value: JsonValue,
) -> std::result::Result<ReleaseDeclaration, String> {
    let document: ReleaseDeclaration = serde_json::from_value(value).map_err(|_| {
        "the release declaration does not match the closed Aether object shape".to_owned()
    })?;
    validate_declaration(&document)?;
    Ok(document)
}

fn validate_declaration(document: &ReleaseDeclaration) -> std::result::Result<(), String> {
    if document
        .schema
        .as_deref()
        .is_some_and(|value| value != AETHER_SCHEMA_URL)
        || document.schema_version != AETHER_CONTRACT_ID
        || !valid_repository(&document.repository.id)
        || document.release.tag_prefix != "v"
        || !document.release.immutable_tags
        || document.changelog.format != "keep-a-changelog/1.1"
        || document.changelog.unreleased_heading != "Unreleased"
        || document.components.is_empty()
        || document.delivery.channels.is_empty()
        || !document.automation.github.manual_dispatch_required
        || document.automation.tasks.plan != "release:plan"
        || document.automation.tasks.prepare != "release:prepare"
        || document.automation.tasks.verify != "release:verify"
        || document.automation.tasks.publish != "release:publish"
    {
        return Err(
            "the release declaration violates an Aether constant or required value".to_owned(),
        );
    }
    validate_schema_path(&document.changelog.path, "changelog path")?;
    validate_schema_path(&document.automation.taskfile_path, "Taskfile path")?;
    validate_relative_path(&document.automation.github.workflow_path, "workflow path")?;
    let workflow = document.automation.github.workflow_path.to_string_lossy();
    let Some(workflow_name) = workflow.strip_prefix(".github/workflows/") else {
        return Err("the release workflow path must name a GitHub Actions YAML file".to_owned());
    };
    if workflow_name.is_empty()
        || workflow_name.contains('/')
        || !workflow_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || !(workflow_name.ends_with(".yml") || workflow_name.ends_with(".yaml"))
    {
        return Err("the release workflow path must name a GitHub Actions YAML file".to_owned());
    }
    if document.evidence.rollback.instructions.trim().is_empty()
        || document.evidence.rollback.instructions.len() > 1_000
        || document
            .evidence
            .rollback
            .instructions
            .chars()
            .any(char::is_control)
    {
        return Err("rollback instructions must contain bounded human-readable text".to_owned());
    }
    for channel in &document.delivery.channels {
        if let Some(profile) = &channel.relay_profile {
            if !valid_relay_profile(profile) {
                return Err("a delivery relay profile is invalid".to_owned());
            }
        }
        if channel
            .notes
            .as_ref()
            .is_some_and(|notes| notes.len() > 500 || notes.chars().any(char::is_control))
        {
            return Err("delivery notes must contain bounded human-readable text".to_owned());
        }
        let _ = (channel.kind, channel.state);
    }
    let mut component_ids = std::collections::BTreeSet::new();
    for component in &document.components {
        if !valid_component_id(&component.id) || !component_ids.insert(component.id.as_str()) {
            return Err("component identifiers must be valid and unique".to_owned());
        }
        if let Some(path) = &component.version_authority.path {
            validate_schema_path(path, "version authority path")?;
        }
        if component
            .version_authority
            .selector
            .as_ref()
            .is_some_and(|selector| {
                selector.is_empty()
                    || selector.len() > 256
                    || selector.chars().any(char::is_control)
            })
        {
            return Err("a version authority selector is invalid".to_owned());
        }
        let _ = component.kind;
    }
    let _ = (
        document.release.major_alias,
        document.evidence.source,
        document.evidence.change,
        document.evidence.provenance,
        document.evidence.sbom,
        document.evidence.signature,
        document.evidence.rollback.strategy,
    );
    Ok(())
}

pub(super) fn evaluate(
    inventory: &RepositoryInventory,
    declaration: Option<&ReleaseDeclaration>,
    slots: &[ReleaseSlotApplicability],
    policy: &ReleasePolicyReference,
) -> Result<Vec<ReleaseCheckResult>> {
    let changelog = declaration.map(|document| analyze_changelog(inventory, document));
    let mut results = Vec::with_capacity(slots.len());
    for slot in slots {
        let rule_id = rule_id_for_slot(&slot.id);
        if rule_id == "EGOLINT_RELEASE_UNKNOWN_POLICY_SLOT" {
            return Err(EgolintError::Configuration(format!(
                "repository-release slot {} has no native rule mapping",
                slot.id
            )));
        }
        let outcome = if slot.requirement == ReleaseRequirement::NotApplicable {
            Outcome::not_applicable(
                PathBuf::from(DECLARATION_PATH),
                "The accepted policy marks this release check not applicable.",
                "No repository change is required while this applicability decision remains in force.",
            )
        } else if let Some(document) = declaration {
            match slot.id.as_str() {
                "agents_profile_pointer" => check_agents(inventory),
                "aether_declaration" => Outcome::passed(
                    PathBuf::from(DECLARATION_PATH),
                    "The repository declaration satisfies the pinned Aether object contract.",
                    "Keep the declaration synchronized with the pinned Aether contract.",
                    local_evidence(inventory, Path::new(DECLARATION_PATH)),
                ),
                "changelog" => changelog
                    .as_ref()
                    .expect("a parsed declaration always has changelog analysis")
                    .outcome
                    .clone(),
                "manual_workflow" => check_workflow(inventory, document),
                "release_rollback_docs" => check_rollback(document),
                "task_handoffs" => check_taskfile(inventory, document),
                "version_authority" => check_versions(inventory, document, changelog.as_ref()),
                _ => unreachable!("rule mapping was checked above"),
            }
        } else {
            Outcome::unavailable(
                PathBuf::from(DECLARATION_PATH),
                "The release declaration is unavailable, so this check cannot resolve repository-owned facts.",
                format!("Create {DECLARATION_PATH} from the pinned Aether contract."),
                local_evidence(inventory, Path::new(DECLARATION_PATH)),
            )
        };
        let mut evidence = vec![EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Policy,
            path: PathBuf::from(POLICY_PATH),
            sha256: Some(policy.hygiene_source.sha256.clone()),
            description: Some(format!("Source-pinned Hygiene policy slot {}.", slot.id)),
        }];
        evidence.extend(outcome.evidence);
        results.push(ReleaseCheckResult {
            slot_id: slot.id.clone(),
            rule_id: rule_id.to_owned(),
            requirement: slot.requirement,
            authority: slot.authority.clone(),
            state: outcome.state,
            message: outcome.message,
            remediation: outcome.remediation,
            location: SourceLocation {
                path: outcome.path,
                start_line: None,
                start_column: None,
                end_line: None,
                end_column: None,
            },
            evidence,
        });
    }
    Ok(results)
}

pub(super) fn findings(checks: &[ReleaseCheckResult]) -> Vec<Finding> {
    checks
        .iter()
        .filter_map(|check| {
            if matches!(
                check.state,
                ReleaseCheckState::Passed | ReleaseCheckState::NotApplicable
            ) {
                return None;
            }
            let severity = if check.requirement == ReleaseRequirement::Required
                && matches!(
                    check.state,
                    ReleaseCheckState::Failed | ReleaseCheckState::Unavailable
                ) {
                Severity::Error
            } else {
                Severity::Warning
            };
            let message = format!("{} Remediation: {}", check.message, check.remediation);
            let fingerprint = stable_fingerprint(&check.rule_id, &check.location, &message);
            Some(Finding {
                schema_version: CONTRACT_VERSION,
                id: format!("{}-{fingerprint}", check.rule_id),
                rule: RuleIdentity {
                    tool_id: TOOL_ID.to_owned(),
                    rule_id: check.rule_id.clone(),
                },
                severity,
                message,
                location: Some(check.location.clone()),
                ownership: RuleOwnership {
                    owner: "egohygiene/egolint".to_owned(),
                    policy_source: format!("{POLICY_PATH}#{}", check.slot_id),
                    configuration_path: Some(PathBuf::from(POLICY_PATH)),
                },
                fingerprint: Some(fingerprint),
                evidence: check.evidence.clone(),
                suppressed_by: None,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct Outcome {
    state: ReleaseCheckState,
    path: PathBuf,
    message: String,
    remediation: String,
    evidence: Vec<EvidenceReference>,
}

impl Outcome {
    fn passed(
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Self {
        Self::new(
            ReleaseCheckState::Passed,
            path,
            message,
            remediation,
            evidence,
        )
    }

    fn failed(
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Self {
        Self::new(
            ReleaseCheckState::Failed,
            path,
            message,
            remediation,
            evidence,
        )
    }

    fn unavailable(
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Self {
        Self::new(
            ReleaseCheckState::Unavailable,
            path,
            message,
            remediation,
            evidence,
        )
    }

    fn external(
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Self {
        Self::new(
            ReleaseCheckState::External,
            path,
            message,
            remediation,
            evidence,
        )
    }

    fn not_applicable(
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self::new(
            ReleaseCheckState::NotApplicable,
            path,
            message,
            remediation,
            Vec::new(),
        )
    }

    fn new(
        state: ReleaseCheckState,
        path: PathBuf,
        message: impl Into<String>,
        remediation: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Self {
        Self {
            state,
            path,
            message: message.into(),
            remediation: remediation.into(),
            evidence,
        }
    }
}

fn check_agents(inventory: &RepositoryInventory) -> Outcome {
    let path = PathBuf::from("AGENTS.md");
    let evidence = local_evidence(inventory, &path);
    let Some(entry) = inventory.get(&path) else {
        return Outcome::unavailable(
            path,
            "AGENTS.md is unavailable, so agents cannot discover the repository release profile.",
            format!("Create AGENTS.md and point it to {DECLARATION_PATH}."),
            evidence,
        );
    };
    if entry.kind != RepositoryEntryKind::File {
        return Outcome::failed(
            path,
            "AGENTS.md is not a regular file.",
            format!("Replace AGENTS.md with a regular file that points to {DECLARATION_PATH}."),
            evidence,
        );
    }
    let Ok(text) = std::str::from_utf8(&entry.content) else {
        return Outcome::failed(
            path,
            "AGENTS.md is not UTF-8 and cannot provide a reliable release-profile pointer.",
            format!("Encode AGENTS.md as UTF-8 and point it to {DECLARATION_PATH}."),
            evidence,
        );
    };
    if text.contains(DECLARATION_PATH) {
        Outcome::passed(
            path,
            "AGENTS.md points agents to the repository-owned release declaration.",
            "Keep the release-profile pointer current.",
            evidence,
        )
    } else {
        Outcome::failed(
            path,
            "AGENTS.md does not point agents to the repository-owned release declaration.",
            format!("Add a guidance pointer to {DECLARATION_PATH} in AGENTS.md."),
            evidence,
        )
    }
}

#[derive(Debug, Clone)]
struct ChangelogAnalysis {
    outcome: Outcome,
    latest_version: Option<String>,
}

fn analyze_changelog(
    inventory: &RepositoryInventory,
    declaration: &ReleaseDeclaration,
) -> ChangelogAnalysis {
    let path = declaration.changelog.path.clone();
    let evidence = local_evidence(inventory, &path);
    if path != Path::new("CHANGELOG.md") {
        return ChangelogAnalysis {
            outcome: Outcome::failed(
                path,
                "The Aether specification requires the changelog at repository-root CHANGELOG.md.",
                "Set changelog.path to CHANGELOG.md and preserve existing history there.",
                evidence,
            ),
            latest_version: None,
        };
    }
    let Some(entry) = inventory.get(&path) else {
        return ChangelogAnalysis {
            outcome: Outcome::unavailable(
                path,
                "The declared root changelog is unavailable.",
                "Create CHANGELOG.md with a # Changelog title and ## [Unreleased] section without inventing history.",
                evidence,
            ),
            latest_version: None,
        };
    };
    if entry.kind != RepositoryEntryKind::File {
        return ChangelogAnalysis {
            outcome: Outcome::failed(
                path,
                "The declared changelog is not a regular file.",
                "Replace CHANGELOG.md with a regular UTF-8 file.",
                evidence,
            ),
            latest_version: None,
        };
    }
    let Ok(text) = std::str::from_utf8(&entry.content) else {
        return ChangelogAnalysis {
            outcome: Outcome::failed(
                path,
                "The declared changelog is not UTF-8.",
                "Encode CHANGELOG.md as UTF-8.",
                evidence,
            ),
            latest_version: None,
        };
    };
    if !text.lines().any(|line| line.trim() == "# Changelog")
        || !text.lines().any(|line| line.trim() == "## [Unreleased]")
    {
        return ChangelogAnalysis {
            outcome: Outcome::failed(
                path,
                "CHANGELOG.md is missing the exact title or Unreleased heading required by the pinned specification.",
                "Add # Changelog and ## [Unreleased] headings while preserving existing history.",
                evidence,
            ),
            latest_version: None,
        };
    }
    let mut versions = Vec::new();
    for heading in text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("## "))
    {
        if heading == "## [Unreleased]" {
            continue;
        }
        let Some(rest) = heading.strip_prefix("## [") else {
            return invalid_changelog_heading(path, evidence);
        };
        let Some((version, date)) = rest.split_once("] - ") else {
            return invalid_changelog_heading(path, evidence);
        };
        if !valid_semver(version) || !valid_date(date) {
            return invalid_changelog_heading(path, evidence);
        }
        versions.push(version.to_owned());
    }
    if matches!(
        declaration.release.state,
        ReleaseState::Released | ReleaseState::Frozen
    ) && versions.is_empty()
    {
        return ChangelogAnalysis {
            outcome: Outcome::failed(
                path,
                "A released or frozen repository must retain at least one promoted semantic-version changelog entry.",
                "Add the reviewed release heading in ## [X.Y.Z] - YYYY-MM-DD form; do not invent prior releases.",
                evidence,
            ),
            latest_version: None,
        };
    }
    ChangelogAnalysis {
        outcome: Outcome::passed(
            path,
            "The root changelog has an Unreleased section and valid promoted semantic-version headings.",
            "Keep promoted headings in ## [X.Y.Z] - YYYY-MM-DD form.",
            evidence,
        ),
        latest_version: versions.first().cloned(),
    }
}

fn invalid_changelog_heading(path: PathBuf, evidence: Vec<EvidenceReference>) -> ChangelogAnalysis {
    ChangelogAnalysis {
        outcome: Outcome::failed(
            path,
            "A promoted changelog heading does not use semantic version and calendar-date syntax.",
            "Use ## [X.Y.Z] - YYYY-MM-DD for every promoted release heading.",
            evidence,
        ),
        latest_version: None,
    }
}

fn check_workflow(inventory: &RepositoryInventory, declaration: &ReleaseDeclaration) -> Outcome {
    let path = declaration.automation.github.workflow_path.clone();
    let evidence = local_evidence(inventory, &path);
    match declaration.automation.github.state {
        GithubAutomationState::Planned => {
            return Outcome::unavailable(
                path,
                "The declaration records the manual release workflow as planned, so it cannot be validated yet.",
                "Add the declared workflow, then set automation.github.state to configured.",
                evidence,
            );
        }
        GithubAutomationState::Unavailable => {
            return Outcome::unavailable(
                path,
                "The declaration explicitly records the manual release workflow as unavailable.",
                "Restore the manual workflow or retain an authorized, reviewable exception.",
                evidence,
            );
        }
        GithubAutomationState::Configured => {}
    }
    let Some(entry) = inventory.get(&path) else {
        return Outcome::unavailable(
            path,
            "The configured manual release workflow is unavailable.",
            "Add the workflow at the exact declared path.",
            evidence,
        );
    };
    if entry.kind != RepositoryEntryKind::File {
        return Outcome::failed(
            path,
            "The configured manual release workflow is not a regular file.",
            "Replace the workflow with a regular YAML file.",
            evidence,
        );
    }
    let Ok(document) = serde_yaml::from_slice::<YamlValue>(&entry.content) else {
        return Outcome::failed(
            path,
            "The configured manual release workflow is not valid YAML.",
            "Repair the workflow YAML and preserve manual dispatch.",
            evidence,
        );
    };
    let Some(root) = document.as_mapping() else {
        return Outcome::failed(
            path,
            "The configured manual release workflow must be a YAML mapping.",
            "Define a workflow with on.workflow_dispatch and jobs.",
            evidence,
        );
    };
    let Some(triggers) = yaml_mapping_value(root, "on") else {
        return workflow_trigger_failure(path, evidence);
    };
    if !has_yaml_key_or_value(triggers, "workflow_dispatch")
        || has_yaml_key_or_value(triggers, "push")
    {
        return workflow_trigger_failure(path, evidence);
    }
    let mut unpinned = Vec::new();
    collect_unpinned_uses(&document, &mut unpinned);
    if !unpinned.is_empty() {
        return Outcome::failed(
            path,
            format!(
                "The manual release workflow has {} dependency reference(s) that are not pinned to immutable revisions.",
                unpinned.len()
            ),
            "Pin every external action to a full 40-character Git commit and every docker action to an image digest.",
            evidence,
        );
    }
    Outcome::passed(
        path,
        "The declared workflow is manually dispatched, has no push trigger, and pins external dependencies immutably.",
        "Keep publication behind reviewed manual dispatch and immutable dependency revisions.",
        evidence,
    )
}

fn workflow_trigger_failure(path: PathBuf, evidence: Vec<EvidenceReference>) -> Outcome {
    Outcome::failed(
        path,
        "The release workflow must expose workflow_dispatch and must not publish from a push trigger.",
        "Configure on.workflow_dispatch and remove the push trigger from the release workflow.",
        evidence,
    )
}

fn check_rollback(declaration: &ReleaseDeclaration) -> Outcome {
    Outcome::passed(
        PathBuf::from(DECLARATION_PATH),
        "The declaration names an accepted rollback strategy with nonempty repository-owned instructions.",
        "Keep rollback instructions current and review them with each release-path change.",
        local_declaration_evidence(),
    )
}

fn check_taskfile(inventory: &RepositoryInventory, declaration: &ReleaseDeclaration) -> Outcome {
    let path = declaration.automation.taskfile_path.clone();
    let evidence = local_evidence(inventory, &path);
    let Some(entry) = inventory.get(&path) else {
        return Outcome::unavailable(
            path,
            "The declared Taskfile is unavailable.",
            "Add the declared Taskfile with the four bounded release handoffs.",
            evidence,
        );
    };
    if entry.kind != RepositoryEntryKind::File {
        return Outcome::failed(
            path,
            "The declared Taskfile is not a regular file.",
            "Replace the Taskfile with a regular YAML file.",
            evidence,
        );
    }
    let Ok(document) = serde_yaml::from_slice::<YamlValue>(&entry.content) else {
        return Outcome::failed(
            path,
            "The declared Taskfile is not valid YAML.",
            "Repair the Taskfile YAML and preserve the four release handoffs.",
            evidence,
        );
    };
    let Some(tasks) = document
        .as_mapping()
        .and_then(|root| yaml_mapping_value(root, "tasks"))
        .and_then(YamlValue::as_mapping)
    else {
        return Outcome::failed(
            path,
            "The declared Taskfile does not define a tasks mapping.",
            "Define release:plan, release:prepare, release:verify, and release:publish tasks.",
            evidence,
        );
    };
    let declared_tasks = [
        declaration.automation.tasks.plan.as_str(),
        declaration.automation.tasks.prepare.as_str(),
        declaration.automation.tasks.verify.as_str(),
        declaration.automation.tasks.publish.as_str(),
    ];
    if declared_tasks
        .iter()
        .any(|name| yaml_mapping_value(tasks, name).is_none())
    {
        return Outcome::failed(
            path,
            "The Taskfile does not expose every declared bounded release handoff.",
            "Define release:plan, release:prepare, release:verify, and release:publish exactly as declared.",
            evidence,
        );
    }
    let publish = yaml_mapping_value(tasks, declaration.automation.tasks.publish.as_str())
        .expect("declared task presence was checked above");
    let publish_text = serde_yaml::to_string(publish).unwrap_or_default();
    let workflow = declaration
        .automation
        .github
        .workflow_path
        .to_string_lossy();
    if !publish_text.contains(workflow.as_ref()) {
        return Outcome::unavailable(
            path,
            "The release:publish task exists, but its handoff to the declared manual workflow cannot be established statically.",
            "Make release:publish explicitly reference the declared workflow path without embedding credentials.",
            evidence,
        );
    }
    Outcome::passed(
        path,
        "The Taskfile exposes all four release handoffs and explicitly names the declared manual workflow.",
        "Keep release:publish as a credential-free handoff to the reviewed manual workflow.",
        evidence,
    )
}

fn check_versions(
    inventory: &RepositoryInventory,
    declaration: &ReleaseDeclaration,
    changelog: Option<&ChangelogAnalysis>,
) -> Outcome {
    let mut versions = Vec::new();
    let mut has_external = false;
    let mut evidence = local_declaration_evidence();
    for component in &declaration.components {
        let authority = &component.version_authority;
        match authority.kind {
            VersionAuthorityKind::GitTag => continue,
            VersionAuthorityKind::External => {
                has_external = true;
                continue;
            }
            _ => {}
        }
        let Some(path) = authority.path.as_ref() else {
            return Outcome::failed(
                PathBuf::from(DECLARATION_PATH),
                format!(
                    "Component {} declares a local version authority without a path.",
                    component.id
                ),
                "Add the repository-relative authority path required by this authority kind.",
                evidence,
            );
        };
        evidence.extend(local_evidence(inventory, path));
        let Some(entry) = inventory.get(path) else {
            return Outcome::unavailable(
                path.clone(),
                format!(
                    "The version authority for component {} is unavailable.",
                    component.id
                ),
                "Restore the declared authority file or update the reviewed declaration.",
                evidence,
            );
        };
        if entry.kind != RepositoryEntryKind::File {
            return Outcome::failed(
                path.clone(),
                format!(
                    "The version authority for component {} is not a regular file.",
                    component.id
                ),
                "Use a regular repository-owned authority file.",
                evidence,
            );
        }
        let selector = authority
            .selector
            .as_deref()
            .or_else(|| default_selector(authority.kind));
        let Some(selector) = selector else {
            return Outcome::failed(
                PathBuf::from(DECLARATION_PATH),
                format!(
                    "Component {} requires an explicit version selector.",
                    component.id
                ),
                "Add the selector that identifies the version field in the authority document.",
                evidence,
            );
        };
        let version = match extract_version(path, &entry.content, selector, authority.kind) {
            Ok(version) => version,
            Err(reason) => {
                return Outcome::failed(
                    path.clone(),
                    format!(
                        "The version authority for component {} is invalid: {reason}.",
                        component.id
                    ),
                    "Repair the authority document or its declared selector so it yields one semantic version.",
                    evidence,
                );
            }
        };
        if !valid_semver(&version) {
            return Outcome::failed(
                path.clone(),
                format!(
                    "The version authority for component {} does not contain a valid semantic version.",
                    component.id
                ),
                "Use MAJOR.MINOR.PATCH syntax with valid optional pre-release or build identifiers.",
                evidence,
            );
        }
        versions.push(version);
    }
    if declaration.components.len() == 1
        && matches!(
            declaration.release.state,
            ReleaseState::Released | ReleaseState::Frozen
        )
        && versions.len() == 1
    {
        let Some(changelog) = changelog else {
            return Outcome::unavailable(
                PathBuf::from(DECLARATION_PATH),
                "Static version drift cannot be checked without changelog analysis.",
                "Restore a valid root changelog before evaluating release-version drift.",
                evidence,
            );
        };
        if changelog.outcome.state != ReleaseCheckState::Passed {
            return Outcome::unavailable(
                declaration.changelog.path.clone(),
                "Static version drift cannot be checked safely because the changelog is not valid and available.",
                "Repair the root changelog before evaluating release-version drift.",
                evidence,
            );
        }
        if changelog.latest_version.as_ref() != versions.first() {
            return Outcome::failed(
                declaration.changelog.path.clone(),
                "The single component's authoritative version differs from the latest promoted changelog version.",
                "Reconcile the reviewed component version and latest promoted changelog heading.",
                evidence,
            );
        }
    }
    if has_external {
        return Outcome::external(
            PathBuf::from(DECLARATION_PATH),
            "One or more components assign version authority to an external owner; all local authorities were validated without querying external systems.",
            "Retain explicit external ownership or provide repository-local Aether version authorities.",
            evidence,
        );
    }
    Outcome::passed(
        PathBuf::from(DECLARATION_PATH),
        "Every component has one valid declared version authority; safe single-component drift checks agree.",
        "Keep each component authority explicit and make version decisions through review.",
        evidence,
    )
}

fn extract_version(
    path: &Path,
    bytes: &[u8],
    selector: &str,
    kind: VersionAuthorityKind,
) -> std::result::Result<String, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the authority is not UTF-8")?;
    match kind {
        VersionAuthorityKind::CargoManifest
        | VersionAuthorityKind::PyprojectProject
        | VersionAuthorityKind::WorkspaceManifest => {
            let value: toml::Value =
                toml::from_str(text).map_err(|_| "the authority is not valid TOML")?;
            select_toml(&value, selector)
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
                .ok_or("the selector does not resolve to a string")
        }
        VersionAuthorityKind::PackageJson => {
            let value: JsonValue =
                serde_json::from_str(text).map_err(|_| "the authority is not valid JSON")?;
            select_json(&value, selector)
                .and_then(JsonValue::as_str)
                .map(str::to_owned)
                .ok_or("the selector does not resolve to a string")
        }
        VersionAuthorityKind::ContainerTag
        | VersionAuthorityKind::PublicationMetadata
        | VersionAuthorityKind::CatalogRecord => {
            match path.extension().and_then(|value| value.to_str()) {
                Some("json") => {
                    let value: JsonValue = serde_json::from_str(text)
                        .map_err(|_| "the authority is not valid JSON")?;
                    select_json(&value, selector)
                        .and_then(JsonValue::as_str)
                        .map(str::to_owned)
                        .ok_or("the selector does not resolve to a string")
                }
                Some("toml") => {
                    let value: toml::Value =
                        toml::from_str(text).map_err(|_| "the authority is not valid TOML")?;
                    select_toml(&value, selector)
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned)
                        .ok_or("the selector does not resolve to a string")
                }
                Some("yaml" | "yml") => {
                    let value: YamlValue = serde_yaml::from_str(text)
                        .map_err(|_| "the authority is not valid YAML")?;
                    select_yaml(&value, selector)
                        .and_then(YamlValue::as_str)
                        .map(str::to_owned)
                        .ok_or("the selector does not resolve to a string")
                }
                _ => Err("the generic authority extension is unsupported"),
            }
        }
        VersionAuthorityKind::GitTag | VersionAuthorityKind::External => {
            Err("this authority does not use a local selector")
        }
    }
}

fn default_selector(kind: VersionAuthorityKind) -> Option<&'static str> {
    match kind {
        VersionAuthorityKind::CargoManifest => Some("package.version"),
        VersionAuthorityKind::PyprojectProject => Some("project.version"),
        VersionAuthorityKind::PackageJson => Some("version"),
        _ => None,
    }
}

fn select_json<'a>(value: &'a JsonValue, selector: &str) -> Option<&'a JsonValue> {
    selector
        .split('.')
        .try_fold(value, |current, key| current.get(key))
}

fn select_toml<'a>(value: &'a toml::Value, selector: &str) -> Option<&'a toml::Value> {
    selector
        .split('.')
        .try_fold(value, |current, key| current.get(key))
}

fn select_yaml<'a>(value: &'a YamlValue, selector: &str) -> Option<&'a YamlValue> {
    selector.split('.').try_fold(value, |current, key| {
        current.as_mapping()?.get(YamlValue::String(key.to_owned()))
    })
}

fn local_declaration_evidence() -> Vec<EvidenceReference> {
    vec![EvidenceReference {
        schema_version: CONTRACT_VERSION,
        kind: EvidenceKind::Configuration,
        path: PathBuf::from(DECLARATION_PATH),
        sha256: None,
        description: Some("Repository-owned Aether release declaration.".to_owned()),
    }]
}

fn local_evidence(inventory: &RepositoryInventory, path: &Path) -> Vec<EvidenceReference> {
    vec![EvidenceReference {
        schema_version: CONTRACT_VERSION,
        kind: EvidenceKind::Configuration,
        path: path.to_path_buf(),
        sha256: inventory
            .get(path)
            .filter(|entry| entry.kind == RepositoryEntryKind::File)
            .map(|entry| digest(&entry.content)),
        description: Some(
            "Repository-local release evidence inspected without network access.".to_owned(),
        ),
    }]
}

fn validate_relative_path(path: &Path, name: &str) -> std::result::Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.to_str().is_none()
    {
        Err(format!(
            "the {name} must be a safe repository-relative UTF-8 path"
        ))
    } else {
        Ok(())
    }
}

fn validate_schema_path(path: &Path, name: &str) -> std::result::Result<(), String> {
    validate_relative_path(path, name)?;
    let value = path
        .to_str()
        .ok_or_else(|| format!("the {name} must be a safe repository-relative UTF-8 path"))?;
    if !value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'/' | b'-'))
    {
        Err(format!("the {name} does not match the Aether path syntax"))
    } else {
        Ok(())
    }
}

fn valid_component_id(value: &str) -> bool {
    (2..=64).contains(&value.len())
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_relay_profile(value: &str) -> bool {
    value == "unavailable"
        || (!value.is_empty()
            && value.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            }))
}

fn valid_semver(value: &str) -> bool {
    let (without_build, build) = value
        .split_once('+')
        .map_or((value, None), |(left, right)| (left, Some(right)));
    if build.is_some_and(|part| !valid_semver_identifiers(part, false)) {
        return false;
    }
    let (core, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(left, right)| (left, Some(right)));
    if prerelease.is_some_and(|part| !valid_semver_identifiers(part, true)) {
        return false;
    }
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!reject_numeric_leading_zero
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || part == "0"
                    || !part.starts_with('0'))
        })
}

fn valid_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let Ok(year) = parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=maximum).contains(&day)
}

fn yaml_mapping_value<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a YamlValue> {
    mapping.get(YamlValue::String(key.to_owned()))
}

fn has_yaml_key_or_value(value: &YamlValue, target: &str) -> bool {
    match value {
        YamlValue::String(value) => value == target,
        YamlValue::Sequence(values) => values.iter().any(|value| value.as_str() == Some(target)),
        YamlValue::Mapping(mapping) => yaml_mapping_value(mapping, target).is_some(),
        _ => false,
    }
}

fn collect_unpinned_uses(value: &YamlValue, unpinned: &mut Vec<String>) {
    match value {
        YamlValue::Mapping(mapping) => {
            for (key, value) in mapping {
                if key.as_str() == Some("uses") {
                    if let Some(reference) = value.as_str() {
                        if !immutable_action_reference(reference) {
                            unpinned.push(reference.to_owned());
                        }
                    } else {
                        unpinned.push("non-string uses value".to_owned());
                    }
                }
                collect_unpinned_uses(value, unpinned);
            }
        }
        YamlValue::Sequence(values) => {
            for value in values {
                collect_unpinned_uses(value, unpinned);
            }
        }
        _ => {}
    }
}

fn immutable_action_reference(reference: &str) -> bool {
    if reference.starts_with("./") {
        return true;
    }
    if let Some(image) = reference.strip_prefix("docker://") {
        return image.split_once("@sha256:").is_some_and(|(_, digest)| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    }
    reference.rsplit_once('@').is_some_and(|(_, revision)| {
        revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
