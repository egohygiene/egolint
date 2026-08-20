//! Offline validation of pinned repository-profile and ecosystem-context projections.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation,
};
use crate::error::{EgolintError, Result};

use super::inventory::{RepositoryEntryKind, RepositoryInventory};

const TOOL_ID: &str = "EGOLINT_REPOSITORY_CONTRACT";
const SOURCE_RULE: &str = "EGO-CONTRACT-SOURCE-001";
const FILE_RULE: &str = "EGO-CONTRACT-FILE-001";
const CONTEXT_RULE: &str = "EGO-CONTRACT-CONTEXT-001";

/// Versioned, offline repository contract consumed by Egolint.
///
/// Empathy and Hygiene own the meaning and contents of their projections;
/// Egolint owns only deterministic validation of this local, pinned envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RepositoryContract {
    /// Contract envelope version.
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    /// Stable contract identifier.
    pub id: String,
    /// Source-owned contract version.
    pub version: String,
    /// Source-owned profile/context name.
    pub profile: String,
    /// Whether this is compatibility evidence for an unfinished upstream contract.
    pub provisional: bool,
    /// Immutable upstream provenance.
    pub source: ContractSource,
    /// Required repository surfaces.
    pub requirements: Vec<ContractRequirement>,
}

impl RepositoryContract {
    /// Decode and validate a TOML contract fixture or repository projection.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed TOML, unsafe paths, mutable provenance,
    /// duplicate requirements, or incomplete generated-file expectations.
    pub fn from_toml(contents: &str, source_path: &Path) -> Result<Self> {
        let contract: Self = toml::from_str(contents).map_err(|source| EgolintError::Toml {
            path: source_path.to_path_buf(),
            source,
        })?;
        contract.validate()?;
        Ok(contract)
    }

    /// Validate the contract envelope independently from a repository.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract is unsafe, incomplete, or ambiguous.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CONTRACT_VERSION {
            return Err(EgolintError::Configuration(format!(
                "repository contract schema-version must equal {CONTRACT_VERSION}"
            )));
        }
        for (name, value) in [
            ("contract id", self.id.as_str()),
            ("contract version", self.version.as_str()),
            ("contract profile", self.profile.as_str()),
        ] {
            if value.trim().is_empty() || value.chars().any(char::is_control) {
                return Err(EgolintError::Configuration(format!(
                    "{name} must be nonempty and contain no control characters"
                )));
            }
        }
        self.source.validate()?;
        if self.requirements.is_empty() {
            return Err(EgolintError::Configuration(
                "repository contract requires at least one file or directory".to_owned(),
            ));
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for requirement in &self.requirements {
            requirement.validate()?;
            if !ids.insert(requirement.id.as_str()) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate repository requirement id {}",
                    requirement.id
                )));
            }
            if !paths.insert(requirement.path.as_path()) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate repository requirement path {}",
                    portable_path(&requirement.path)
                )));
            }
        }
        Ok(())
    }
}

/// Immutable source information for an upstream profile or context projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContractSource {
    /// Canonical `owner/repository` identifier.
    pub repository: String,
    /// Exact 40-character lowercase Git commit or blob identifier.
    pub revision: String,
    /// Whether `revision` names a commit or a single source blob.
    pub revision_kind: SourceRevisionKind,
    /// Repository-relative upstream source path.
    pub path: PathBuf,
    /// Human-review issue or decision describing provisional status.
    pub decision: String,
}

impl ContractSource {
    fn validate(&self) -> Result<()> {
        if self.repository.split('/').count() != 2
            || self.repository.starts_with('/')
            || self.repository.ends_with('/')
        {
            return Err(EgolintError::Configuration(
                "contract source repository must use owner/name form".to_owned(),
            ));
        }
        if self.revision.len() != 40
            || !self
                .revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(EgolintError::Configuration(
                "contract source revision must be an immutable 40-character lowercase Git id"
                    .to_owned(),
            ));
        }
        validate_relative_path(&self.path, "contract source path")?;
        if !self.decision.starts_with("https://") {
            return Err(EgolintError::Configuration(
                "contract source decision must be an HTTPS reference".to_owned(),
            ));
        }
        Ok(())
    }

    fn policy_source(&self) -> String {
        match self.revision_kind {
            SourceRevisionKind::GitCommit => format!(
                "https://github.com/{}/blob/{}/{}",
                self.repository,
                self.revision,
                portable_path(&self.path)
            ),
            SourceRevisionKind::GitBlob => format!(
                "https://api.github.com/repos/{}/git/blobs/{}",
                self.repository, self.revision
            ),
        }
    }
}

/// Kind of immutable Git identifier used by a contract source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SourceRevisionKind {
    /// Complete source repository revision.
    GitCommit,
    /// Content API blob identifier for a provisional source document.
    GitBlob,
}

/// One exact-case repository requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContractRequirement {
    /// Stable source-owned requirement identifier.
    pub id: String,
    /// Exact-case workspace-relative path.
    pub path: PathBuf,
    /// Required filesystem object kind.
    pub kind: RequirementKind,
    /// Which system may control the content.
    pub ownership: RequirementOwnership,
    /// Required Git executable mode for a file.
    #[serde(default)]
    pub executable: bool,
    /// Required literal content markers for generated/context files.
    #[serde(default)]
    pub markers: Vec<String>,
}

impl ContractRequirement {
    fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() || self.id.chars().any(char::is_control) {
            return Err(EgolintError::Configuration(
                "repository requirement id must be nonempty and safe".to_owned(),
            ));
        }
        validate_relative_path(&self.path, "repository requirement path")?;
        if self.executable && self.kind != RequirementKind::File {
            return Err(EgolintError::Configuration(format!(
                "directory requirement {} cannot require executable mode",
                self.id
            )));
        }
        if self.ownership == RequirementOwnership::Generated && self.markers.is_empty() {
            return Err(EgolintError::Configuration(format!(
                "generated requirement {} needs at least one deterministic marker",
                self.id
            )));
        }
        if self
            .markers
            .iter()
            .any(|marker| marker.is_empty() || marker.chars().any(char::is_control))
        {
            return Err(EgolintError::Configuration(format!(
                "repository requirement {} contains an empty or unsafe marker",
                self.id
            )));
        }
        Ok(())
    }
}

/// Required repository object kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementKind {
    /// Regular file, not a symbolic link.
    File,
    /// Directory represented by one or more inventory entries.
    Directory,
}

/// Content-control boundary for a required repository artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RequirementOwnership {
    /// Materialized from the pinned upstream profile/context source.
    Generated,
    /// Required by the upstream contract, with locally chosen content.
    Required,
    /// Owned and evolved by the consumer repository.
    RepositoryOwned,
}

/// Evaluator retaining the local contract path as report evidence.
#[derive(Debug)]
pub struct RepositoryContractEvaluator<'a> {
    contract: &'a RepositoryContract,
    contract_path: PathBuf,
}

impl<'a> RepositoryContractEvaluator<'a> {
    /// Construct an evaluator for one validated local contract projection.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest path escapes the workspace or the
    /// contract itself is invalid.
    pub fn new(contract: &'a RepositoryContract, contract_path: &Path) -> Result<Self> {
        contract.validate()?;
        validate_relative_path(contract_path, "repository contract evidence path")?;
        Ok(Self {
            contract,
            contract_path: contract_path.to_path_buf(),
        })
    }

    /// Evaluate exact-case requirements without fetching an upstream repository.
    ///
    /// # Errors
    ///
    /// Returns an error only for an invalid contract or inventory. Repository
    /// drift is returned as normalized findings.
    pub fn evaluate(&self, inventory: &RepositoryInventory) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        if self.contract.provisional {
            findings.push(self.finding(
                SOURCE_RULE,
                &self.contract_path,
                Severity::Warning,
                format!(
                    "contract {} is provisional compatibility evidence; upstream decision remains {}",
                    self.contract.id, self.contract.source.decision
                ),
            ));
        }
        for requirement in &self.contract.requirements {
            self.evaluate_requirement(inventory, requirement, &mut findings);
        }
        findings.sort_by(|left, right| {
            let left_path = left
                .location
                .as_ref()
                .map_or_else(|| Path::new(""), |location| location.path.as_path());
            let right_path = right
                .location
                .as_ref()
                .map_or_else(|| Path::new(""), |location| location.path.as_path());
            left.rule
                .rule_id
                .cmp(&right.rule.rule_id)
                .then_with(|| left_path.cmp(right_path))
                .then_with(|| left.message.cmp(&right.message))
        });
        for finding in &findings {
            finding.validate()?;
        }
        Ok(findings)
    }

    fn evaluate_requirement(
        &self,
        inventory: &RepositoryInventory,
        requirement: &ContractRequirement,
        findings: &mut Vec<Finding>,
    ) {
        let exists = match requirement.kind {
            RequirementKind::File => inventory.get(&requirement.path).is_some(),
            RequirementKind::Directory => inventory.contains_directory(&requirement.path),
        };
        if !exists {
            let expected_path = portable_path(&requirement.path);
            let folded = expected_path.to_ascii_lowercase();
            let casing = inventory
                .entries()
                .iter()
                .find(|entry| portable_path(&entry.path).to_ascii_lowercase() == folded);
            let message = casing.map_or_else(
                || {
                    format!(
                        "required {:?} {} is missing",
                        requirement.kind, expected_path
                    )
                },
                |entry| {
                    format!(
                        "required path {} has incorrect casing; observed {}",
                        expected_path,
                        portable_path(&entry.path)
                    )
                },
            );
            findings.push(self.finding(FILE_RULE, &requirement.path, Severity::Error, message));
            return;
        }
        if requirement.kind == RequirementKind::File {
            let entry = inventory
                .get(&requirement.path)
                .expect("exact file existence checked above");
            if entry.kind != RepositoryEntryKind::File {
                findings.push(self.finding(
                    FILE_RULE,
                    &requirement.path,
                    Severity::Error,
                    format!(
                        "required file {} is a symbolic link",
                        portable_path(&requirement.path)
                    ),
                ));
                return;
            }
            if requirement.executable && entry.git_mode != Some(100_755) {
                findings.push(self.finding(
                    FILE_RULE,
                    &requirement.path,
                    Severity::Error,
                    format!(
                        "required executable {} has Git mode {}; expected 100755",
                        portable_path(&requirement.path),
                        entry.git_mode.unwrap_or_default()
                    ),
                ));
            }
            for marker in &requirement.markers {
                if !entry
                    .content
                    .windows(marker.len())
                    .any(|candidate| candidate == marker.as_bytes())
                {
                    findings.push(self.finding(
                        CONTEXT_RULE,
                        &requirement.path,
                        Severity::Error,
                        format!(
                            "generated/context file {} is missing required marker {:?}",
                            portable_path(&requirement.path),
                            marker
                        ),
                    ));
                }
            }
        }
    }

    fn finding(&self, rule_id: &str, path: &Path, severity: Severity, message: String) -> Finding {
        let fingerprint = stable_fingerprint(rule_id, path, &message);
        Finding {
            schema_version: CONTRACT_VERSION,
            id: format!("{rule_id}-{fingerprint}"),
            rule: RuleIdentity {
                tool_id: TOOL_ID.to_owned(),
                rule_id: rule_id.to_owned(),
            },
            severity,
            message,
            location: Some(SourceLocation {
                path: path.to_path_buf(),
                start_line: None,
                start_column: None,
                end_line: None,
                end_column: None,
            }),
            ownership: RuleOwnership {
                owner: self.contract.source.repository.clone(),
                policy_source: self.contract.source.policy_source(),
                configuration_path: Some(self.contract_path.clone()),
            },
            fingerprint: Some(fingerprint),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Fixture,
                path: self.contract_path.clone(),
                sha256: None,
                description: Some(format!(
                    "Pinned {} {:?} {} at {}",
                    self.contract.source.repository,
                    self.contract.source.revision_kind,
                    self.contract.source.revision,
                    portable_path(&self.contract.source.path)
                )),
            }],
            suppressed_by: None,
        }
    }
}

fn stable_fingerprint(rule_id: &str, path: &Path, message: &str) -> String {
    let path = portable_path(path);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in rule_id
        .bytes()
        .chain([0])
        .chain(path.bytes())
        .chain([0])
        .chain(message.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("repository-contract-v1-{hash:016x}")
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::inventory::RepositoryEntry;

    const EMPATHY_CONTRACT: &str =
        include_str!("../../tests/fixtures/contracts/empathy-universal-provisional.toml");
    const HYGIENE_CONTRACT: &str =
        include_str!("../../tests/fixtures/contracts/hygiene-context-provisional.toml");

    fn contract(contents: &str, path: &str) -> RepositoryContract {
        RepositoryContract::from_toml(contents, Path::new(path)).expect("valid fixture contract")
    }

    #[test]
    fn provisional_contract_sources_are_immutable_and_visible() {
        let empathy = contract(EMPATHY_CONTRACT, "tests/fixtures/contracts/empathy.toml");
        let findings = RepositoryContractEvaluator::new(
            &empathy,
            Path::new("tests/fixtures/contracts/empathy.toml"),
        )
        .expect("valid evaluator")
        .evaluate(
            &RepositoryInventory::from_entries(vec![
                RepositoryEntry::file("README.md", Some(100_644), b"# Consumer\n".to_vec()),
                RepositoryEntry::file(".editorconfig", Some(100_644), b"root = true\n".to_vec()),
                RepositoryEntry::file(
                    ".gitattributes",
                    Some(100_644),
                    b"* text=auto eol=lf\n".to_vec(),
                ),
                RepositoryEntry::file(
                    "ARCHITECTURE.md",
                    Some(100_644),
                    b"# Architecture\n".to_vec(),
                ),
                RepositoryEntry::file("Taskfile.yml", Some(100_644), b"version: '3'\n".to_vec()),
            ])
            .expect("valid repository"),
        )
        .expect("evaluation");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule.rule_id, SOURCE_RULE);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn hygiene_context_checks_identity_release_and_agent_surface() {
        let hygiene = contract(HYGIENE_CONTRACT, "tests/fixtures/contracts/hygiene.toml");
        let evaluator = RepositoryContractEvaluator::new(
            &hygiene,
            Path::new("tests/fixtures/contracts/hygiene.toml"),
        )
        .expect("valid evaluator");
        let invalid = RepositoryInventory::from_entries(vec![
            RepositoryEntry::file("agents.md", Some(100_644), b"Read context.\n".to_vec()),
            RepositoryEntry::file(
                "docs/ecosystem/CONTEXT.md",
                Some(100_644),
                b"architecture_release: stale\nrepository: egohygiene/example\n".to_vec(),
            ),
        ])
        .expect("valid inventory");

        let findings = evaluator.evaluate(&invalid).expect("evaluation");
        let rules = findings
            .iter()
            .map(|finding| finding.rule.rule_id.as_str())
            .collect::<Vec<_>>();
        assert!(rules.contains(&FILE_RULE));
        assert!(rules.contains(&CONTEXT_RULE));
        assert!(rules.contains(&SOURCE_RULE));
        let source = findings
            .iter()
            .find(|finding| finding.rule.rule_id == SOURCE_RULE)
            .expect("provisional source finding");
        assert_eq!(
            source.ownership.policy_source,
            "https://api.github.com/repos/egohygiene/hygiene/git/blobs/feb8ae94e784913553d4ae72a14eebabfe4ecb5f"
        );
        assert!(
            source.evidence[0]
                .description
                .as_deref()
                .is_some_and(|description| description.contains("docs/ecosystem/AGENT_CONTEXT.md"))
        );
    }

    #[test]
    fn contract_rejects_mutable_sources_and_ambiguous_requirements() {
        let mutable = EMPATHY_CONTRACT.replace("560aff8430c2f170dadae9161a4603a71c41acbf", "main");
        assert!(
            RepositoryContract::from_toml(
                &mutable,
                Path::new("tests/fixtures/contracts/mutable.toml")
            )
            .is_err()
        );

        let duplicate = EMPATHY_CONTRACT.replace(
            "[[requirements]]\nid = \"editorconfig\"",
            "[[requirements]]\nid = \"readme\"",
        );
        assert!(
            RepositoryContract::from_toml(
                &duplicate,
                Path::new("tests/fixtures/contracts/duplicate.toml")
            )
            .is_err()
        );
    }

    #[test]
    fn repository_contract_schema_pins_version_one() {
        let schema = serde_json::to_value(schemars::schema_for!(RepositoryContract))
            .expect("repository-contract schema");

        assert_eq!(schema["properties"]["schema-version"]["const"], 1);
    }
}
