//! Versioned domain contracts shared by the CLI, reports, and integrations.

use std::path::{Component, Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::Profile;
use crate::error::{EgolintError, Result};

/// Current version for machine-readable Egolint domain and output contracts.
pub const CONTRACT_VERSION: u32 = 1;

/// Versioned description of one selected lint profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProfileDefinition {
    /// Profile-contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Stable built-in profile name.
    pub name: Profile,
    /// Repository surface evaluated by the profile.
    pub scope: ProfileScope,
    /// Human-readable reason the profile exists.
    pub purpose: String,
    /// Logical identifier for the policy that defines the profile.
    pub policy_source: String,
}

impl ProfileDefinition {
    /// Return the canonical definition for a built-in profile.
    #[must_use]
    pub fn built_in(profile: Profile) -> Self {
        let (scope, purpose) = match profile {
            Profile::Fast => (
                ProfileScope::ChangedFilesWithRepositoryPolicy,
                "Deterministic pull-request and local feedback.",
            ),
            Profile::Holistic => (
                ProfileScope::CompleteRepository,
                "Complete scheduled, manual, and trusted-branch inspection.",
            ),
            Profile::Security => (
                ProfileScope::CompleteRepository,
                "Focused secret, static-analysis, and infrastructure security inspection.",
            ),
            Profile::DependencyDebt => (
                ProfileScope::CompleteRepository,
                "Focused dependency vulnerability and software-inventory inspection.",
            ),
        };
        Self {
            schema_version: CONTRACT_VERSION,
            name: profile,
            scope,
            purpose: purpose.to_owned(),
            policy_source: format!(
                ".config/megalinter/policy.yml#profiles/{}",
                profile.as_str()
            ),
        }
    }

    /// Validate this profile contract.
    ///
    /// # Errors
    ///
    /// Returns an error when its version or required provenance is invalid.
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.schema_version, "profile")?;
        validate_text("profile purpose", &self.purpose, 512)?;
        validate_text("profile policy-source", &self.policy_source, 2_048)
    }
}

/// Repository surface selected by a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileScope {
    /// Files changed relative to the adapter's comparison base.
    ///
    /// This value remains accepted for version-1 documents produced before
    /// Egolint added repository-wide native policy checks to the fast profile.
    ChangedFiles,
    /// Changed-file adapter analysis plus complete-repository native policy.
    ChangedFilesWithRepositoryPolicy,
    /// The complete repository surface.
    CompleteRepository,
}

/// Stable identity of a tool-owned rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RuleIdentity {
    /// Canonical Egolint or `MegaLinter` tool identifier.
    pub tool_id: String,
    /// Tool-native or Egolint-owned rule identifier.
    pub rule_id: String,
}

impl RuleIdentity {
    fn validate(&self) -> Result<()> {
        validate_identifier("tool-id", &self.tool_id)?;
        validate_identifier("rule-id", &self.rule_id)
    }
}

/// Explicit ownership and policy provenance for a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RuleOwnership {
    /// Repository, team, or external system accountable for the rule.
    pub owner: String,
    /// Logical identifier for the policy decision that selected the rule.
    pub policy_source: String,
    /// Optional workspace-relative configuration implementing the rule.
    pub configuration_path: Option<PathBuf>,
}

impl RuleOwnership {
    fn validate(&self) -> Result<()> {
        validate_text("rule owner", &self.owner, 256)?;
        validate_text("rule policy-source", &self.policy_source, 2_048)?;
        if let Some(path) = &self.configuration_path {
            validate_relative_path(path, "rule configuration-path")?;
        }
        Ok(())
    }
}

/// Versioned normalized lint finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Finding {
    /// Finding-contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Stable finding identifier within the report.
    pub id: String,
    /// Tool and rule that produced the finding.
    pub rule: RuleIdentity,
    /// Normalized severity independent of adapter-specific labels.
    pub severity: Severity,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional normalized source location.
    pub location: Option<SourceLocation>,
    /// Accountable owner and policy provenance.
    pub ownership: RuleOwnership,
    /// Optional stable adapter or Egolint fingerprint.
    pub fingerprint: Option<String>,
    /// Sanitized evidence supporting this finding.
    pub evidence: Vec<EvidenceReference>,
    /// Suppression identifier applied to this finding, when any.
    pub suppressed_by: Option<String>,
}

impl Finding {
    /// Validate this normalized finding.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported versions, unsafe paths, or missing
    /// identity, ownership, and evidence metadata.
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.schema_version, "finding")?;
        validate_identifier("finding id", &self.id)?;
        self.rule.validate()?;
        validate_text("finding message", &self.message, 16_384)?;
        reject_control_characters("finding message", &self.message)?;
        self.ownership.validate()?;
        if let Some(location) = &self.location {
            location.validate()?;
        }
        if let Some(fingerprint) = &self.fingerprint {
            validate_text("finding fingerprint", fingerprint, 1_024)?;
        }
        if let Some(suppression) = &self.suppressed_by {
            validate_identifier("finding suppression id", suppression)?;
        }
        for evidence in &self.evidence {
            evidence.validate()?;
        }
        Ok(())
    }
}

/// Normalized finding severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational observation.
    Info,
    /// Nonblocking warning.
    Warning,
    /// Blocking correctness or policy failure.
    Error,
    /// Urgent high-impact security or safety failure.
    Critical,
}

/// Workspace-relative source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SourceLocation {
    /// Normalized path relative to the inspected workspace.
    pub path: PathBuf,
    /// One-based starting line.
    pub start_line: Option<u32>,
    /// One-based starting column.
    pub start_column: Option<u32>,
    /// One-based ending line.
    pub end_line: Option<u32>,
    /// One-based ending column.
    pub end_column: Option<u32>,
}

impl SourceLocation {
    /// Validate this source location.
    ///
    /// # Errors
    ///
    /// Returns an error when the path escapes the workspace or coordinates
    /// are zero or reversed.
    pub fn validate(&self) -> Result<()> {
        validate_relative_path(&self.path, "finding location path")?;
        for (name, value) in [
            ("start-line", self.start_line),
            ("start-column", self.start_column),
            ("end-line", self.end_line),
            ("end-column", self.end_column),
        ] {
            if value == Some(0) {
                return Err(EgolintError::Configuration(format!(
                    "finding {name} must be one-based"
                )));
            }
        }
        if let (Some(start), Some(end)) = (self.start_line, self.end_line) {
            if end < start {
                return Err(EgolintError::Configuration(
                    "finding end-line may not precede start-line".to_owned(),
                ));
            }
        }
        if self.start_line == self.end_line {
            if let (Some(start), Some(end)) = (self.start_column, self.end_column) {
                if end < start {
                    return Err(EgolintError::Configuration(
                        "finding end-column may not precede start-column".to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Versioned, reviewable exception to one rule or finding fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Suppression {
    /// Suppression-contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Stable suppression identifier.
    pub id: String,
    /// Rule selected by this suppression.
    pub rule: RuleIdentity,
    /// Optional workspace-relative path scope.
    pub path: Option<PathBuf>,
    /// Optional exact finding fingerprint scope.
    pub fingerprint: Option<String>,
    /// Person, team, or repository accountable for review.
    pub owner: String,
    /// Nonempty justification for accepting the exception.
    pub justification: String,
    /// Required calendar expiry in `YYYY-MM-DD` form.
    pub expires_on: String,
    /// Observed state for this suppression in the report.
    pub state: SuppressionState,
    /// Sanitized evidence supporting the exception.
    pub evidence: Vec<EvidenceReference>,
}

impl Suppression {
    /// Validate this suppression contract.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, ownership, expiry, selector, or paths
    /// are invalid. Time-based state evaluation belongs to the rule engine.
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.schema_version, "suppression")?;
        validate_identifier("suppression id", &self.id)?;
        self.rule.validate()?;
        validate_text("suppression owner", &self.owner, 256)?;
        validate_text("suppression justification", &self.justification, 4_096)?;
        reject_control_characters("suppression justification", &self.justification)?;
        validate_contract_date(&self.expires_on)?;
        if let Some(path) = &self.path {
            validate_relative_path(path, "suppression path")?;
        }
        if let Some(fingerprint) = &self.fingerprint {
            validate_text("suppression fingerprint", fingerprint, 1_024)?;
        }
        for evidence in &self.evidence {
            evidence.validate()?;
        }
        Ok(())
    }
}

/// Observed disposition of a declared suppression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionState {
    /// The suppression matched at least one finding and remained valid.
    Applied,
    /// The suppression did not match a finding.
    Unmatched,
    /// The suppression passed its required expiry date.
    Expired,
    /// The suppression could not be evaluated safely.
    Invalid,
}

/// Versioned reference to sanitized, workspace-owned evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EvidenceReference {
    /// Evidence-contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Kind of evidence referenced.
    pub kind: EvidenceKind,
    /// Normalized workspace-relative evidence path.
    pub path: PathBuf,
    /// Optional lowercase hexadecimal SHA-256 digest.
    pub sha256: Option<String>,
    /// Optional bounded human-readable context.
    pub description: Option<String>,
}

impl EvidenceReference {
    /// Validate this evidence reference.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported versions, unsafe paths, malformed
    /// digests, or control characters.
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.schema_version, "evidence")?;
        validate_relative_path(&self.path, "evidence path")?;
        if let Some(digest) = &self.sha256 {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(EgolintError::Configuration(
                    "evidence sha256 must contain 64 lowercase hexadecimal characters".to_owned(),
                ));
            }
        }
        if let Some(description) = &self.description {
            validate_text("evidence description", description, 2_048)?;
            reject_control_characters("evidence description", description)?;
        }
        Ok(())
    }
}

/// Sanitized evidence category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Canonical policy selection or ownership record.
    Policy,
    /// Tool or rule configuration.
    Configuration,
    /// Positive, negative, or compatibility fixture.
    Fixture,
    /// Machine-readable adapter report.
    AdapterReport,
    /// Sanitized adapter log.
    AdapterLog,
    /// Other reviewed evidence.
    Other,
}

/// Versioned normalized result for one selected tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ToolResult {
    /// Tool-result contract version.
    #[schemars(schema_with = "contract_version_schema")]
    pub schema_version: u32,
    /// Canonical Egolint or `MegaLinter` tool identifier.
    pub tool_id: String,
    /// Repository, team, or external system accountable for the tool policy.
    pub owner: String,
    /// Logical identifier for the policy that selected the tool.
    pub policy_source: String,
    /// Precise observed execution or selection state.
    pub status: ToolStatus,
    /// Whether this result participates in the blocking quality gate.
    pub enforcement: Enforcement,
    /// Number of normalized findings attributed to the tool.
    pub finding_count: u64,
    /// Number of normalized warnings attributed to the tool.
    pub warning_count: u64,
    /// Adapter-reported elapsed time, when available.
    pub duration_ms: Option<u64>,
    /// Sanitized evidence supporting the result.
    pub evidence: Vec<EvidenceReference>,
}

impl ToolResult {
    /// Validate this normalized tool result.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported versions, missing ownership, or unsafe
    /// evidence.
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.schema_version, "tool-result")?;
        validate_identifier("tool-result tool-id", &self.tool_id)?;
        validate_text("tool-result owner", &self.owner, 256)?;
        validate_text("tool-result policy-source", &self.policy_source, 2_048)?;
        for evidence in &self.evidence {
            evidence.validate()?;
        }
        Ok(())
    }
}

/// Precise normalized state for one tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Selected but not yet executed.
    Selected,
    /// Not relevant to the inspected repository.
    NotApplicable,
    /// Excluded by the selected profile.
    DisabledByProfile,
    /// Deliberately disabled by policy configuration.
    DisabledByConfiguration,
    /// Selected but absent from the adapter image.
    MissingFromImage,
    /// Tool configuration could not be loaded.
    ConfigurationError,
    /// Completed without findings or warnings.
    Passed,
    /// Completed with nonblocking warnings.
    PassedWithWarnings,
    /// Completed with lint findings.
    FailedFindings,
    /// Failed to execute normally.
    ExecutionError,
    /// Exceeded its execution budget.
    TimedOut,
}

/// Quality-gate behavior for one tool or rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    /// Findings fail the quality gate.
    Blocking,
    /// Findings remain visible without failing the gate.
    Advisory,
    /// Policy deliberately disables the tool or rule.
    Disabled,
}

/// JSON Schema for an exact contract-version field.
#[must_use]
pub fn contract_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "const": CONTRACT_VERSION
    })
}

fn validate_contract_version(version: u32, contract: &str) -> Result<()> {
    if version == CONTRACT_VERSION {
        Ok(())
    } else {
        Err(EgolintError::Configuration(format!(
            "{contract} schema-version must equal {CONTRACT_VERSION}"
        )))
    }
}

fn validate_text(name: &str, value: &str, maximum_length: usize) -> Result<()> {
    if value.trim().is_empty() {
        Err(EgolintError::Configuration(format!(
            "{name} may not be empty"
        )))
    } else if value.len() > maximum_length {
        Err(EgolintError::Configuration(format!(
            "{name} may not exceed {maximum_length} bytes"
        )))
    } else {
        Ok(())
    }
}

fn validate_identifier(name: &str, value: &str) -> Result<()> {
    validate_text(name, value, 256)?;
    if value.chars().any(char::is_control) {
        return Err(EgolintError::Configuration(format!(
            "{name} may not contain control characters"
        )));
    }
    Ok(())
}

fn reject_control_characters(name: &str, value: &str) -> Result<()> {
    if value
        .chars()
        .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        Err(EgolintError::Configuration(format!(
            "{name} contains unsupported control characters"
        )))
    } else {
        Ok(())
    }
}

fn validate_relative_path(path: &Path, name: &str) -> Result<()> {
    let Some(text) = path.to_str() else {
        return Err(EgolintError::Configuration(format!(
            "{name} must contain valid Unicode"
        )));
    };
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || text.contains('\\')
        || text.as_bytes().get(1) == Some(&b':')
    {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a nonempty portable path relative to the workspace"
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

/// Validate a calendar date used by a versioned contract.
///
/// # Errors
///
/// Returns an error unless `value` is a real Gregorian date in `YYYY-MM-DD`
/// form.
pub fn validate_contract_date(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let shape_is_valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !shape_is_valid {
        return Err(EgolintError::Configuration(
            "contract date must use YYYY-MM-DD".to_owned(),
        ));
    }
    let year = value[0..4].parse::<u32>().unwrap_or_default();
    let month = value[5..7].parse::<u32>().unwrap_or_default();
    let day = value[8..10].parse::<u32>().unwrap_or_default();
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days_in_month {
        return Err(EgolintError::Configuration(
            "contract date is not a real Gregorian date".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(path: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Fixture,
            path: PathBuf::from(path),
            sha256: None,
            description: Some("Synthetic compatibility evidence.".to_owned()),
        }
    }

    #[test]
    fn built_in_profiles_are_versioned_and_explainable() {
        let fast = ProfileDefinition::built_in(Profile::Fast);
        let holistic = ProfileDefinition::built_in(Profile::Holistic);
        let security = ProfileDefinition::built_in(Profile::Security);
        let dependency_debt = ProfileDefinition::built_in(Profile::DependencyDebt);

        fast.validate().expect("valid fast profile");
        holistic.validate().expect("valid holistic profile");
        security.validate().expect("valid security profile");
        dependency_debt
            .validate()
            .expect("valid dependency-debt profile");
        assert_eq!(fast.scope, ProfileScope::ChangedFilesWithRepositoryPolicy);
        assert_eq!(holistic.scope, ProfileScope::CompleteRepository);
        assert_eq!(security.scope, ProfileScope::CompleteRepository);
        assert_eq!(dependency_debt.scope, ProfileScope::CompleteRepository);
    }

    #[test]
    fn fast_profile_reports_mixed_scope_and_accepts_legacy_scope() {
        assert_eq!(
            serde_json::to_string(&ProfileDefinition::built_in(Profile::Fast).scope)
                .expect("encode fast scope"),
            "\"changed_files_with_repository_policy\""
        );

        let legacy_scope: ProfileScope =
            serde_json::from_str("\"changed_files\"").expect("decode legacy fast scope");
        assert_eq!(legacy_scope, ProfileScope::ChangedFiles);
    }

    #[test]
    fn built_in_profile_purposes_match_canonical_policy() {
        let policy: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../.config/megalinter/policy.yml"))
                .expect("decode canonical MegaLinter policy");

        for profile in [
            Profile::Fast,
            Profile::Holistic,
            Profile::Security,
            Profile::DependencyDebt,
        ] {
            let policy_purpose = policy["profiles"][profile.as_str()]["purpose"]
                .as_str()
                .expect("profile policy purpose");
            assert_eq!(ProfileDefinition::built_in(profile).purpose, policy_purpose);
        }
    }

    #[test]
    fn evidence_and_locations_reject_workspace_escapes() {
        let mut unsafe_evidence = evidence("../outside.log");
        assert!(unsafe_evidence.validate().is_err());
        unsafe_evidence.path = PathBuf::from("C:\\Users\\person\\report.json");
        assert!(unsafe_evidence.validate().is_err());
        unsafe_evidence.path = PathBuf::from(".reports/egolint/tool.log");
        unsafe_evidence.sha256 = Some("A".repeat(64));
        assert!(unsafe_evidence.validate().is_err());

        let location = SourceLocation {
            path: PathBuf::from("/absolute/source.rs"),
            start_line: Some(1),
            start_column: Some(1),
            end_line: None,
            end_column: None,
        };
        assert!(location.validate().is_err());
    }

    #[test]
    fn suppression_requires_review_metadata_and_valid_paths() {
        let suppression = Suppression {
            schema_version: CONTRACT_VERSION,
            id: "SUP-001".to_owned(),
            rule: RuleIdentity {
                tool_id: "PYTHON_RUFF".to_owned(),
                rule_id: "F401".to_owned(),
            },
            path: Some(PathBuf::from("src/example.py")),
            fingerprint: None,
            owner: "egohygiene/empathy".to_owned(),
            justification: "Compatibility exception under active review.".to_owned(),
            expires_on: "2027-01-31".to_owned(),
            state: SuppressionState::Applied,
            evidence: vec![evidence(
                "tests/fixtures/compatibility/empathy-v1/suppression.json",
            )],
        };

        suppression.validate().expect("valid suppression");
    }

    #[test]
    fn suppression_expiry_uses_real_gregorian_dates() {
        assert!(validate_contract_date("2028-02-29").is_ok());
        assert!(validate_contract_date("2000-02-29").is_ok());
        assert!(validate_contract_date("2026-02-29").is_err());
        assert!(validate_contract_date("2100-02-29").is_err());
        assert!(validate_contract_date("2026-04-31").is_err());
    }

    #[test]
    fn publishable_text_fields_are_bounded() {
        assert!(validate_identifier("id", &"x".repeat(256)).is_ok());
        assert!(validate_identifier("id", &"x".repeat(257)).is_err());

        let mut reference = evidence("evidence.json");
        reference.description = Some("x".repeat(2_049));
        assert!(reference.validate().is_err());
    }

    #[test]
    fn every_emitted_contract_schema_pins_version_one() {
        for schema in [
            serde_json::to_value(schemars::schema_for!(ProfileDefinition)).expect("profile schema"),
            serde_json::to_value(schemars::schema_for!(Finding)).expect("finding schema"),
            serde_json::to_value(schemars::schema_for!(Suppression)).expect("suppression schema"),
            serde_json::to_value(schemars::schema_for!(EvidenceReference))
                .expect("evidence schema"),
            serde_json::to_value(schemars::schema_for!(ToolResult)).expect("tool-result schema"),
        ] {
            assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        }
    }
}
