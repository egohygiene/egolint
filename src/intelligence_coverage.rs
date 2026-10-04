//! Offline collection-coverage validation, composed with source and graph validation.
//!
//! This check does not certify the full projection schema or graph semantics. Its
//! fixed diagnostics never echo provider identities, counts, payloads or paths.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contracts::validate_contract_date;
use crate::error::{EgolintError, Result};

/// Maximum JSON input size for both library and CLI callers.
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
/// Exact immutable Hygiene revision shared with Observatory alpha.2.
pub const SOURCE_REVISION: &str = "639a003d5ddc4d242c2cf190eeb59a9fc522d199";
const CONTRACT: &str = "egohygiene.repository-intelligence/v1";
const LOCK: &str =
    include_str!("../.config/rules/repository-intelligence-coverage-sources.v1.json");
const BUNDLE: [(&str, &str); 4] = [
    (
        "schemas/repository-intelligence.v1.schema.json",
        include_str!("../vendor/hygiene/intelligence/repository-intelligence.v1.schema.json"),
    ),
    (
        "schemas/repository-intelligence.alpha2.schema.json",
        include_str!("../vendor/hygiene/intelligence/repository-intelligence.alpha2.schema.json"),
    ),
    (
        "catalog/repository-intelligence-vocabulary.json",
        include_str!("../vendor/hygiene/intelligence/repository-intelligence-vocabulary.json"),
    ),
    (
        "docs/ecosystem/REPOSITORY_INTELLIGENCE_COVERAGE.md",
        include_str!("../vendor/hygiene/intelligence/REPOSITORY_INTELLIGENCE_COVERAGE.md"),
    ),
];

/// Fixed domain keys from the Hygiene alpha.2 contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    /// Canonical roadmap inventory.
    Roadmap,
    /// Canonical ADR inventory.
    Decisions,
    /// Reachable commit inventory.
    Git,
    /// All authorized issue states, not just referenced or open issues.
    Issues,
    /// All authorized pull-request states.
    PullRequests,
    /// Checks for the represented revision.
    Checks,
    /// Release inventory.
    Releases,
    /// Deployment inventory.
    Deployments,
    /// Lifecycle event history, independent of current inventories.
    History,
}

const DOMAINS: [(Domain, &str, &str); 9] = [
    (Domain::Roadmap, "roadmap", "roadmap_step"),
    (Domain::Decisions, "decisions", "architecture_decision"),
    (Domain::Git, "git", "commit"),
    (Domain::Issues, "issues", "issue"),
    (Domain::PullRequests, "pull_requests", "pull_request"),
    (Domain::Checks, "checks", "check"),
    (Domain::Releases, "releases", "release"),
    (Domain::Deployments, "deployments", "deployment"),
    (Domain::History, "history", ""),
];

/// A preserved claim whose values are checked against the pinned owner schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectionClaim {
    /// Collection state; independent of freshness and conformance.
    pub collection: String,
    /// Collector-supplied freshness, never refreshed by this check.
    pub freshness: String,
    /// Fixed owner-defined reason code; no provider message is retained.
    pub reason: String,
    /// Observation time, or explicit null when no observation was obtained.
    pub observed_at: Option<String>,
}

/// Semantic validity of collection claims only, never whole-projection conformance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    /// Supported claims are consistent with represented root-domain records.
    Valid,
    /// Malformed, unsupported or contradictory coverage input.
    Invalid,
    /// A protected input was withheld without identities or counts.
    Unavailable,
}

/// Whether a compatible projection provides any collection-completeness claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverageKnowledge {
    /// Alpha.2 explicitly declares all domains, which may themselves be partial.
    Explicit,
    /// Alpha.1 says nothing about collection completeness.
    LegacyUnknown,
    /// Input cannot supply usable coverage evidence.
    Unavailable,
}

/// Deterministic machine-readable result for the standalone coverage check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoverageReport {
    /// Version of this `EgoLint` report shape.
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    /// Explicit scope: this supplements full source/schema/graph validation.
    pub scope: String,
    /// Owner contract identifier, not consumer repository identity.
    pub contract: String,
    /// Exact supported input version, absent for unavailable/unsupported input.
    pub contract_version: Option<String>,
    /// Exact owner definition revision.
    pub source_revision: String,
    /// Contract implementation remains proposed, regardless of merge state.
    pub authority: String,
    /// Validity of the declared coverage claims.
    pub status: CoverageStatus,
    /// Explicit claims, legacy uncertainty, or unavailable evidence.
    pub coverage: CoverageKnowledge,
    /// Preserved claims only after the entire coverage check succeeds.
    pub domains: Option<BTreeMap<Domain, CollectionClaim>>,
    /// One stable diagnostic, without untrusted provider data.
    pub diagnostic: String,
}

impl CoverageReport {
    fn empty(status: CoverageStatus, diagnostic: &str) -> Self {
        Self {
            schema_version: 1,
            scope: "collection-coverage".into(),
            contract: CONTRACT.into(),
            contract_version: None,
            source_revision: SOURCE_REVISION.into(),
            authority: "proposed".into(),
            status,
            coverage: CoverageKnowledge::Unavailable,
            domains: None,
            diagnostic: diagnostic.into(),
        }
    }
}

/// Validate the pinned definition and collection claims without I/O or mutation.
///
/// Alpha.1 returns valid compatibility with `legacy_unknown` and no domain
/// claims. A valid alpha.2 report does not imply complete or current collection.
/// Full Hygiene schema/graph validation and existing ADR/roadmap source lint
/// remain separate required checks before publication.
///
/// # Errors
/// Returns an error only if the bundled owner definition or source lock drifts.
/// Invalid user inputs return sanitized, deterministic reports.
pub fn validate_coverage(bytes: &[u8]) -> Result<CoverageReport> {
    verify_bundle()?;
    let invalid = |code| CoverageReport::empty(CoverageStatus::Invalid, code);
    if bytes.len() > MAX_INPUT_BYTES {
        return Ok(invalid("EGO-INTEL-COVERAGE-INPUT"));
    }
    let value = match serde_json::from_slice::<UniqueJson>(bytes) {
        Ok(value) => value.0,
        Err(_) => return Ok(invalid("EGO-INTEL-COVERAGE-INPUT")),
    };
    // Privacy is checked before any graph context or coverage is returned.
    if value.get("visibility").and_then(Value::as_str) != Some("public") {
        return Ok(CoverageReport::empty(
            CoverageStatus::Unavailable,
            "EGO-INTEL-COVERAGE-VISIBILITY",
        ));
    }
    let Some(version @ ("1.0.0-alpha.1" | "1.0.0-alpha.2")) =
        value.get("contract_version").and_then(Value::as_str)
    else {
        return Ok(invalid("EGO-INTEL-COVERAGE-CONTRACT"));
    };
    if value.get("schema").and_then(Value::as_str) != Some(CONTRACT) {
        return Ok(invalid("EGO-INTEL-COVERAGE-CONTRACT"));
    }
    let Some(context) = Context::parse(&value) else {
        return Ok(invalid("EGO-INTEL-COVERAGE-CONTEXT"));
    };
    for collection in ["sources", "entities", "events"] {
        let Some(records) = value[collection].as_array() else {
            return Ok(invalid("EGO-INTEL-COVERAGE-CONTEXT"));
        };
        if records
            .iter()
            .any(|record| record["visibility"].as_str() != Some("public"))
        {
            return Ok(CoverageReport::empty(
                CoverageStatus::Unavailable,
                "EGO-INTEL-COVERAGE-VISIBILITY",
            ));
        }
    }
    let mut report = CoverageReport::empty(CoverageStatus::Valid, "EGO-INTEL-COVERAGE-VALID");
    report.contract_version = Some(version.into());
    if version == "1.0.0-alpha.1" {
        if value.get("collection_coverage").is_some() {
            return Ok(invalid("EGO-INTEL-COVERAGE-CONTRACT"));
        }
        report.coverage = CoverageKnowledge::LegacyUnknown;
        report.diagnostic = "EGO-INTEL-COVERAGE-LEGACY-UNKNOWN".into();
        return Ok(report);
    }
    let Some(coverage) = value["collection_coverage"].as_object() else {
        return Ok(invalid("EGO-INTEL-COVERAGE-DOMAINS"));
    };
    if coverage.len() != DOMAINS.len()
        || DOMAINS
            .iter()
            .any(|(_, key, _)| !coverage.contains_key(*key))
    {
        return Ok(invalid("EGO-INTEL-COVERAGE-DOMAINS"));
    }
    let schema: Value = serde_json::from_str(BUNDLE[1].1)?;
    let mut domains = BTreeMap::new();
    for (domain, key, kind) in DOMAINS {
        let Some(claim) = claim(&coverage[key], &schema, &context.observed_at) else {
            return Ok(invalid("EGO-INTEL-COVERAGE-CLAIM"));
        };
        let present = if domain == Domain::History {
            context.root_events
        } else {
            context.root_kinds.contains(kind)
        };
        if (present
            && matches!(
                claim.collection.as_str(),
                "observed_empty" | "uncollected" | "unavailable" | "failed" | "not_applicable"
            ))
            || (!present && claim.collection == "observed")
        {
            return Ok(invalid("EGO-INTEL-COVERAGE-RECORDS"));
        }
        domains.insert(domain, claim);
    }
    report.coverage = CoverageKnowledge::Explicit;
    report.domains = Some(domains);
    Ok(report)
}

fn claim(value: &Value, schema: &Value, observed_at: &(String, String)) -> Option<CollectionClaim> {
    let object = value.as_object()?;
    if object.len() != 4
        || ["collection", "freshness", "reason", "observed_at"]
            .iter()
            .any(|key| !object.contains_key(*key))
    {
        return None;
    }
    let claim: CollectionClaim = serde_json::from_value(value.clone()).ok()?;
    let rules = schema["$defs"]["collectionDomain"]["allOf"].as_array()?;
    let rule = rules.iter().find(|rule| {
        rule["if"]["properties"]["collection"]["const"].as_str() == Some(&claim.collection)
    })?;
    let properties = &rule["then"]["properties"];
    for (field, actual) in [("reason", &claim.reason), ("freshness", &claim.freshness)] {
        if !properties[field]["enum"]
            .as_array()?
            .iter()
            .any(|item| item.as_str() == Some(actual))
        {
            return None;
        }
    }
    match properties["observed_at"]["type"].as_str()? {
        "string" => {
            if timestamp(claim.observed_at.as_deref()?)?.gt(observed_at) {
                return None;
            }
        }
        "null" if claim.observed_at.is_none() => {}
        _ => return None,
    }
    Some(claim)
}

struct Context<'a> {
    observed_at: (String, String),
    root_kinds: BTreeSet<&'a str>,
    root_events: bool,
}

impl<'a> Context<'a> {
    fn parse(value: &'a Value) -> Option<Self> {
        let repository = value["repository"].as_str()?;
        let name = repository.strip_prefix("egohygiene/")?;
        if name != ".github"
            && (name.is_empty()
                || !name.as_bytes()[0].is_ascii_alphanumeric()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-".contains(&b)))
        {
            return None;
        }
        let commit = value["represented_commit"].as_str()?;
        if commit.len() != 40
            || !commit
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || value["projection_id"].as_str()? != format!("{repository}@{commit}")
        {
            return None;
        }
        let observed_at = timestamp(value["observed_at"].as_str()?)?;
        let mut all_ids = BTreeSet::new();
        let mut root_ids = BTreeSet::new();
        let mut root_kinds = BTreeSet::new();
        let mut roots = 0;
        for entity in value["entities"].as_array()? {
            let id = entity["id"].as_str()?;
            let kind = entity["kind"].as_str()?;
            let owner = entity["repository"].as_str()?;
            if id.is_empty()
                || !all_ids.insert(id)
                || (kind != "repository" && !DOMAINS.iter().any(|(_, _, k)| *k == kind))
            {
                return None;
            }
            if owner == repository {
                root_ids.insert(id);
                root_kinds.insert(kind);
                roots += usize::from(kind == "repository");
            }
        }
        if roots != 1 {
            return None;
        }
        let mut event_ids = BTreeSet::new();
        let mut root_events = false;
        for event in value["events"].as_array()? {
            let id = event["id"].as_str()?;
            let subject = event["subject"].as_str()?;
            if id.is_empty() || !event_ids.insert(id) || !all_ids.contains(subject) {
                return None;
            }
            root_events |= root_ids.contains(subject);
        }
        Some(Self {
            observed_at,
            root_kinds,
            root_events,
        })
    }
}

// Strict UTC, real Gregorian date, arbitrary decimal precision. Lexical comparison
// of equal-width seconds plus trailing-zero-normalized fractions is chronological.
fn timestamp(value: &str) -> Option<(String, String)> {
    if !value.is_ascii() || value.len() < 20 || !value.ends_with('Z') {
        return None;
    }
    validate_contract_date(value.get(..10)?).ok()?;
    let bytes = value.as_bytes();
    if bytes[10] != b'T' || bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    for (start, maximum) in [(11, 23), (14, 59), (17, 59)] {
        let part = value.get(start..start + 2)?;
        if !part.bytes().all(|b| b.is_ascii_digit()) || part.parse::<u8>().ok()? > maximum {
            return None;
        }
    }
    let suffix = value.get(19..value.len() - 1)?;
    let fraction = if suffix.is_empty() {
        ""
    } else {
        let fraction = suffix.strip_prefix('.')?;
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        fraction.trim_end_matches('0')
    };
    Some((value[..19].into(), fraction.into()))
}

fn verify_bundle() -> Result<()> {
    let lock: Value = serde_json::from_str(LOCK)?;
    if lock["schema_version"] != 1
        || lock["contract"] != CONTRACT
        || lock["authority"] != "proposed"
        || lock["repository"] != "egohygiene/hygiene"
        || lock["revision"] != SOURCE_REVISION
        || lock["versions"] != serde_json::json!(["1.0.0-alpha.1", "1.0.0-alpha.2"])
    {
        return Err(bundle_error());
    }
    let artifacts = lock["artifacts"].as_array().ok_or_else(bundle_error)?;
    if artifacts.len() != BUNDLE.len() {
        return Err(bundle_error());
    }
    for ((path, bytes), entry) in BUNDLE.iter().zip(artifacts) {
        if entry["source_path"] != *path
            || entry["sha256"] != format!("{:x}", Sha256::digest(bytes.as_bytes()))
        {
            return Err(bundle_error());
        }
    }
    Ok(())
}

fn bundle_error() -> EgolintError {
    EgolintError::Configuration(
        "bundled Intelligence coverage contract failed integrity verification".into(),
    )
}

// serde_json::Value otherwise accepts duplicate keys with last-value-wins semantics.
// Reject ambiguous provider captures before interpreting any collection claim.
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("unambiguous JSON")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, UniqueJson>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(UniqueJson(Value::Object(values)))
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> std::result::Result<Self::Value, S::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<UniqueJson>()? {
                    values.push(value.0);
                }
                Ok(UniqueJson(Value::Array(values)))
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                value: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::Bool(value)))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                value: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(value.into())))
            }
            fn visit_i64<E: serde::de::Error>(
                self,
                value: i64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(
                self,
                value: f64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(
                    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number),
                ))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
