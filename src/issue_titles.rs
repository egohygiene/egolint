//! Offline issue-title validation and explicitly reviewed subject formatting.
//!
//! The organization owns the mapping and semantics. This module consumes one
//! checksum-verified candidate bundle; it does not classify issues or grant writes.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::{EgolintError, Result};

const SOURCE_LOCK: &str = include_str!("../.config/rules/issue-title-sources.v1.json");
const BUNDLE: [(&str, &str); 5] = [
    (
        ".github/issues/title-contract.v1.json",
        include_str!("../vendor/github/issue-titles/.github/issues/title-contract.v1.json"),
    ),
    (
        ".github/issues/schema/title-contract.v1.schema.json",
        include_str!(
            "../vendor/github/issue-titles/.github/issues/schema/title-contract.v1.schema.json"
        ),
    ),
    (
        ".github/labels/catalog.v1.json",
        include_str!("../vendor/github/issue-titles/.github/labels/catalog.v1.json"),
    ),
    (
        "docs/issue-titles.md",
        include_str!("../vendor/github/issue-titles/docs/issue-titles.md"),
    ),
    (
        "fixtures/issue-titles/cases.v1.json",
        include_str!("../vendor/github/issue-titles/fixtures/issue-titles/cases.v1.json"),
    ),
];

/// Explicit normalized input from a provider adapter; no omitted-label fallback.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueTitleSnapshot {
    /// Only version 1 is supported.
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    /// Caller attests the title and full label set were observed together.
    pub complete: bool,
    /// Observed title, preserved without cleanup or inference.
    pub title: String,
    /// Exact observed label names; duplicates are treated as a set.
    pub labels: Vec<String>,
}

/// The bundled upstream definition is proposed, not an accepted rollout policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IssueTitleAuthority {
    /// Requires upstream review and explicit consumer adoption before enforcement.
    Candidate,
}

/// Stable provenance for all five source artifacts; no runtime clock or host path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueTitleProvenance {
    /// Organization contract identifier.
    pub contract_id: String,
    /// Selected semantic contract version.
    pub contract_version: String,
    /// Owning repository, independent of the consumer.
    pub repository: String,
    /// Full immutable commit selected for every bundled source.
    pub revision: String,
    /// Review status of the selected definition.
    pub authority: IssueTitleAuthority,
    /// Source-relative path to SHA-256 digest, in stable key order.
    pub source_digests: BTreeMap<String, String>,
}

/// Classification is evaluated before title syntax, in the upstream order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IssueTitleStatus {
    /// Exact mapped prefix and a nonempty, trimmed, single-line subject.
    Conformant,
    /// A unique primary label exists, but the title does not match its format.
    Nonconformant,
    /// No known primary type label was observed.
    NeedsClassification,
    /// More than one distinct known primary type was observed.
    Conflict,
    /// At least one `type:*` label is outside the selected mapping.
    UnsupportedType,
    /// Input was explicitly incomplete and cannot prove any title status.
    Unavailable,
}

/// Result describes syntax under the candidate, never provider enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueTitleReport {
    /// Version of this Egolint result shape.
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    /// Exact definition and source evidence used by this check.
    pub contract: IssueTitleProvenance,
    /// Deterministic classification/format result.
    pub status: IssueTitleStatus,
    /// Unique known primary type, when classification succeeded.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Stable diagnostic that does not echo issue content.
    pub message: String,
}

/// A proposal built only from an explicit type and reviewed subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueTitleProposal {
    /// Version of this Egolint proposal shape.
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    /// Exact definition and source evidence used by this formatter.
    pub contract: IssueTitleProvenance,
    /// Explicitly selected primary type; this does not mutate provider labels.
    #[serde(rename = "type")]
    pub kind: String,
    /// Required provider label for the selected primary type.
    pub required_label: String,
    /// Canonical prefix followed by the reviewed subject unchanged.
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLock {
    schema_version: u32,
    contract: String,
    authority: IssueTitleAuthority,
    sources: Vec<PinnedArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PinnedArtifact {
    repository: String,
    revision: String,
    source_path: String,
    vendored_path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    #[serde(rename = "$schema")]
    schema: String,
    schema_version: u32,
    contract_id: String,
    contract_version: String,
    owner: String,
    label_catalog: CatalogReference,
    format: String,
    types: Vec<TypeMapping>,
    rules: Value,
    adoption: Value,
    semantics: String,
    conformance_examples: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogReference {
    path: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeMapping {
    #[serde(rename = "type")]
    kind: String,
    emoji: String,
    label: String,
}

impl TypeMapping {
    fn prefix(&self) -> String {
        format!("{} [{}] ", self.emoji, self.kind)
    }
}

/// Verified policy with a private constructor: callers cannot substitute silent defaults.
#[derive(Debug)]
pub struct IssueTitlePolicy {
    definition: Definition,
    provenance: IssueTitleProvenance,
}

impl IssueTitlePolicy {
    /// Load and verify every bundled artifact without network access.
    ///
    /// # Errors
    /// Returns a configuration error identifying unavailable/incompatible source evidence.
    pub fn bundled() -> Result<Self> {
        Self::from_bundle(SOURCE_LOCK, &BUNDLE)
    }

    fn from_bundle(lock: &str, bundle: &[(&str, &str)]) -> Result<Self> {
        let lock: SourceLock =
            serde_json::from_str(lock).map_err(|_| unavailable("source lock cannot be decoded"))?;
        let provenance = verify_sources(&lock, bundle)?;
        let definition: Definition = serde_json::from_str(bundle[0].1)
            .map_err(|_| unavailable("definition cannot be decoded"))?;
        verify_definition(&definition, bundle[2].1)?;
        Ok(Self {
            definition,
            provenance,
        })
    }

    /// Validate an observed snapshot without changing it or inferring its classification.
    ///
    /// # Errors
    /// Returns a configuration error for an unsupported snapshot version.
    pub fn validate(&self, input: &IssueTitleSnapshot) -> Result<IssueTitleReport> {
        if input.schema_version != 1 {
            return Err(EgolintError::Configuration(
                "issue-title snapshot schema_version must equal 1".into(),
            ));
        }
        let (status, mapping, message) = if input.complete {
            self.classify(input)
        } else {
            (
                IssueTitleStatus::Unavailable,
                None,
                "A complete title and label snapshot is required.",
            )
        };
        Ok(IssueTitleReport {
            schema_version: 1,
            contract: self.provenance.clone(),
            status,
            kind: mapping.map(|item| item.kind.clone()),
            message: message.into(),
        })
    }

    fn classify<'a>(
        &'a self,
        input: &IssueTitleSnapshot,
    ) -> (IssueTitleStatus, Option<&'a TypeMapping>, &'static str) {
        let labels: BTreeSet<&str> = input.labels.iter().map(String::as_str).collect();
        if labels.iter().any(|label| {
            label.starts_with("type:")
                && !self
                    .definition
                    .types
                    .iter()
                    .any(|item| item.label == *label)
        }) {
            return (
                IssueTitleStatus::UnsupportedType,
                None,
                "An observed type label is outside the pinned mapping.",
            );
        }
        let selected: Vec<_> = self
            .definition
            .types
            .iter()
            .filter(|item| labels.contains(item.label.as_str()))
            .collect();
        match selected.as_slice() {
            [] => (
                IssueTitleStatus::NeedsClassification,
                None,
                "Select one known primary type label through review.",
            ),
            [mapping] => {
                let valid = input
                    .title
                    .strip_prefix(&mapping.prefix())
                    .is_some_and(valid_subject);
                if valid {
                    (
                        IssueTitleStatus::Conformant,
                        Some(mapping),
                        "Title matches the selected candidate definition; adoption is not assessed.",
                    )
                } else {
                    (
                        IssueTitleStatus::Nonconformant,
                        Some(mapping),
                        "Expected the exact mapped prefix and a nonempty trimmed single-line subject.",
                    )
                }
            }
            _ => (
                IssueTitleStatus::Conflict,
                None,
                "Multiple distinct primary type labels require review.",
            ),
        }
    }

    /// Format an explicit type and reviewed subject, preserving the subject byte-for-byte.
    ///
    /// This is not an existing-title parser. No emoji, bracket text, or identifiers are stripped.
    ///
    /// # Errors
    /// Returns a configuration error for an unknown type or invalid subject.
    pub fn format(&self, kind: &str, reviewed_subject: &str) -> Result<IssueTitleProposal> {
        let mapping = self
            .definition
            .types
            .iter()
            .find(|item| item.kind == kind)
            .ok_or_else(|| EgolintError::Configuration("unknown issue-title type".into()))?;
        if !valid_subject(reviewed_subject) {
            return Err(EgolintError::Configuration(
                "reviewed subject must be nonempty, trimmed, and single-line".into(),
            ));
        }
        Ok(IssueTitleProposal {
            schema_version: 1,
            contract: self.provenance.clone(),
            kind: mapping.kind.clone(),
            required_label: mapping.label.clone(),
            title: format!("{}{reviewed_subject}", mapping.prefix()),
        })
    }
}

fn valid_subject(subject: &str) -> bool {
    !subject.is_empty()
        && subject.trim() == subject
        && !subject.contains([
            '\n', '\r', '\u{000b}', '\u{000c}', '\u{0085}', '\u{2028}', '\u{2029}',
        ])
}

fn unavailable(reason: &str) -> EgolintError {
    EgolintError::Configuration(format!("issue-title contract unavailable: {reason}"))
}

fn verify_sources(lock: &SourceLock, bundle: &[(&str, &str)]) -> Result<IssueTitleProvenance> {
    if lock.schema_version != 1
        || lock.contract != "egolint.issue-title-sources/v1"
        || lock.sources.len() != BUNDLE.len()
        || bundle.len() != BUNDLE.len()
    {
        return Err(unavailable("unsupported source lock or incomplete bundle"));
    }
    let expected_paths: BTreeSet<_> = BUNDLE.iter().map(|(path, _)| *path).collect();
    let paths: BTreeSet<_> = lock
        .sources
        .iter()
        .map(|source| source.source_path.as_str())
        .collect();
    if paths != expected_paths
        || bundle
            .iter()
            .zip(BUNDLE)
            .any(|((path, _), (expected, _))| *path != expected)
    {
        return Err(unavailable(
            "source paths do not identify the complete supported bundle",
        ));
    }
    let revision = &lock.sources[0].revision;
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(unavailable(
            "source revision must be a full immutable commit",
        ));
    }
    let mut digests = BTreeMap::new();
    for source in &lock.sources {
        let expected_path = format!("vendor/github/issue-titles/{}", source.source_path);
        if source.repository != "egohygiene/.github"
            || source.revision != *revision
            || source.vendored_path != expected_path
        {
            return Err(unavailable(
                "source ownership, revision, or vendored path mismatch",
            ));
        }
        let (_, content) = bundle
            .iter()
            .find(|(path, _)| *path == source.source_path)
            .ok_or_else(|| unavailable("required source bytes are missing"))?;
        let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
        if source.sha256 != digest {
            return Err(unavailable("source digest mismatch"));
        }
        digests.insert(source.source_path.clone(), digest);
    }
    Ok(IssueTitleProvenance {
        contract_id: "egohygiene.issue-title/v1".into(),
        contract_version: "1.0.0".into(),
        repository: "egohygiene/.github".into(),
        revision: revision.clone(),
        authority: lock.authority,
        source_digests: digests,
    })
}

fn verify_definition(definition: &Definition, catalog: &str) -> Result<()> {
    let supported_rules = json!({
        "primary_type": "exactly-one-known-type-label",
        "missing_type": "needs-classification",
        "multiple_types": "conflict",
        "unknown_type": "unsupported-type",
        "subject": "nonempty-trimmed-single-line",
        "separators": "single-ascii-space",
        "emoji_matching": "exact-codepoints",
        "subject_case": "preserve",
        "existing_subject": "preserve-wording-and-identifiers",
        "migration": "reviewed-explicit-subject",
        "normalization": "idempotent",
        "unknown_prefix": "preserve-until-reviewed",
        "relationships": "native-identifiers-not-title-parsing"
    });
    if definition.schema_version != 1
        || definition.contract_id != "egohygiene.issue-title/v1"
        || definition.contract_version != "1.0.0"
        || definition.owner != "egohygiene/.github"
        || definition.format != "{emoji} [{type}] {subject}"
        || definition.schema != "./schema/title-contract.v1.schema.json"
        || definition.label_catalog.path != "../labels/catalog.v1.json"
        || definition.label_catalog.version != "1.0.0"
        || definition.semantics != "../../docs/issue-titles.md"
        || definition.conformance_examples != "../../fixtures/issue-titles/cases.v1.json"
        || definition.rules != supported_rules
        || definition.adoption
            != json!({
                "scope": "explicit-repository-adoption",
                "default_mode": "observe",
                "reference": "immutable-commit",
                "unavailable_contract": "report-unavailable"
            })
    {
        return Err(unavailable("definition requires unsupported semantics"));
    }
    let catalog: Value = serde_json::from_str(catalog)
        .map_err(|_| unavailable("label catalog cannot be decoded"))?;
    if catalog["catalog_version"] != definition.label_catalog.version {
        return Err(unavailable("label catalog version mismatch"));
    }
    let universal = catalog["universal"]
        .as_array()
        .ok_or_else(|| unavailable("universal label catalog is missing"))?;
    let catalog_types: BTreeSet<_> = universal
        .iter()
        .filter(|item| item["category"] == "type")
        .filter_map(|item| item["name"].as_str())
        .collect();
    let labels: BTreeSet<_> = definition
        .types
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    let kinds: BTreeSet<_> = definition
        .types
        .iter()
        .map(|item| item.kind.as_str())
        .collect();
    if labels != catalog_types
        || definition.types.is_empty()
        || labels.len() != definition.types.len()
        || kinds.len() != definition.types.len()
        || definition.types.iter().any(|item| {
            item.label != format!("type:{}", item.kind)
                || item.kind.is_empty()
                || !item.kind.bytes().all(|byte| byte.is_ascii_lowercase())
                || !valid_subject(&item.emoji)
        })
    {
        return Err(unavailable("type mapping is incompatible with the catalog"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_title_source_bundle_rejects_missing_changed_and_mixed_inputs() {
        assert!(IssueTitlePolicy::from_bundle(SOURCE_LOCK, &BUNDLE[..4]).is_err());
        for index in 0..BUNDLE.len() {
            let mut changed = BUNDLE;
            changed[index].1 = "changed bytes";
            assert!(IssueTitlePolicy::from_bundle(SOURCE_LOCK, &changed).is_err());
        }
        for (field, value) in [
            ("revision", "main"),
            ("repository", "other/repository"),
            ("vendored_path", "elsewhere.json"),
            ("revision", "0000000000000000000000000000000000000000"),
        ] {
            let mut lock: Value = serde_json::from_str(SOURCE_LOCK).unwrap();
            lock["sources"][0][field] = json!(value);
            assert!(IssueTitlePolicy::from_bundle(&lock.to_string(), &BUNDLE).is_err());
        }
    }

    #[test]
    fn issue_title_source_bundle_rejects_unsupported_contract_semantics() {
        let mut definition: Definition = serde_json::from_str(BUNDLE[0].1).unwrap();
        definition.rules["missing_type"] = json!("silently-infer");
        assert!(verify_definition(&definition, BUNDLE[2].1).is_err());
        let mut definition: Definition = serde_json::from_str(BUNDLE[0].1).unwrap();
        definition.contract_version = "2.0.0".into();
        assert!(verify_definition(&definition, BUNDLE[2].1).is_err());
    }
}
