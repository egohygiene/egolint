//! Defensive normalization of `MegaLinter` v10 evidence.
//!
//! Raw `MegaLinter` reports are intentionally treated as private adapter
//! artifacts. This module whitelists a small set of fields, replaces raw tool
//! messages with bounded generic diagnostics, and accepts only normalized
//! workspace-relative source locations before data enters Egolint's stable
//! report contract.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::Profile;
use crate::contracts::{
    CONTRACT_VERSION, Enforcement, EvidenceKind, EvidenceReference, Finding, RuleIdentity,
    RuleOwnership, Severity, SourceLocation, ToolResult, ToolStatus,
};
use crate::error::{EgolintError, Result};
use crate::report::ReportCompleteness;

/// Fixed raw `MegaLinter` JSON report path beneath a consumer workspace.
pub const RAW_JSON_REPORT: &str = ".reports/egolint/mega-linter-report.json";
/// Fixed raw `MegaLinter` SARIF report path beneath a consumer workspace.
pub const RAW_SARIF_REPORT: &str = ".reports/egolint/mega-linter-report.sarif";

const POLICY_MATRIX: &str = include_str!("../.config/megalinter/tool-matrix.json");
const MAX_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ACTIVE_LINTERS: usize = 512;
const MAX_SARIF_RUNS: usize = 512;
const MAX_SARIF_RESULTS: usize = 100_000;
const SUPPORTED_SARIF_VERSION: &str = "2.1.0";

/// Whitelisted adapter data ready for `RunReport::set_normalized`.
#[derive(Debug)]
pub struct NormalizedMegaLinter {
    /// One normalized result for every observed active linter.
    pub tool_results: Vec<ToolResult>,
    /// Sanitized source-located or aggregate findings.
    pub findings: Vec<Finding>,
    /// Reviewed paths to the private adapter inputs.
    pub evidence: Vec<EvidenceReference>,
    /// Honest coverage of the available adapter output.
    pub completeness: ReportCompleteness,
}

impl Default for NormalizedMegaLinter {
    fn default() -> Self {
        Self {
            tool_results: Vec::new(),
            findings: Vec::new(),
            evidence: Vec::new(),
            completeness: ReportCompleteness::AdapterExitOnly,
        }
    }
}

/// Normalize fixed-name `MegaLinter` artifacts from a validated workspace.
///
/// Missing adapter files are not an error: callers still retain the process
/// status in an adapter-exit-only `RunReport`. Existing files must be regular,
/// bounded JSON files beneath the exact workspace-relative report paths.
///
/// # Errors
///
/// Returns an error for unsafe report files, oversized/unparseable inputs,
/// unsupported result counts, or contract-invalid normalized output.
// Keep this trust-boundary sequence linear so envelope, reconciliation, and
// completeness transitions can be audited in execution order.
#[allow(clippy::too_many_lines)]
pub fn normalize_workspace(
    workspace: &Path,
    profile: Profile,
) -> Result<Option<NormalizedMegaLinter>> {
    let policy = PolicyIndex::load()?;
    let raw_json_path = workspace.join(RAW_JSON_REPORT);
    let raw_sarif_path = workspace.join(RAW_SARIF_REPORT);
    let json = read_optional_json(&raw_json_path)?;
    let sarif = read_optional_json(&raw_sarif_path)?;
    if json.is_none() && sarif.is_none() {
        return Ok(None);
    }

    let mut normalized = NormalizedMegaLinter {
        completeness: ReportCompleteness::Partial,
        ..NormalizedMegaLinter::default()
    };
    if json.is_some() {
        normalized.evidence.push(adapter_evidence(
            RAW_JSON_REPORT,
            "Private MegaLinter detailed JSON; only whitelisted fields were normalized.",
        ));
    }
    if sarif.is_some() {
        normalized.evidence.push(adapter_evidence(
            RAW_SARIF_REPORT,
            "Private upstream SARIF; source locations and rule identities were revalidated.",
        ));
    }

    let mut detailed_by_tool = BTreeMap::<String, DetailedCounts>::new();
    if let Some(document) = sarif.as_ref() {
        normalize_sarif(
            document,
            &policy,
            profile,
            &mut normalized.findings,
            &mut detailed_by_tool,
        )?;
    }

    let mut fully_covered = sarif.is_some();
    if let Some(document) = json.as_ref() {
        let raw: RawMegaLinter = serde_json::from_value(document.clone()).map_err(|_| {
            EgolintError::RuntimeExecution(
                "MegaLinter JSON report does not match the supported v10 adapter envelope"
                    .to_owned(),
            )
        })?;
        if raw.linters.len() > MAX_ACTIVE_LINTERS {
            return Err(EgolintError::RuntimeExecution(format!(
                "MegaLinter report contains more than {MAX_ACTIVE_LINTERS} linter records"
            )));
        }
        let mut observed_tools = BTreeSet::new();
        let mut active_tools = BTreeSet::new();
        let overall_return_code = raw.return_code;
        let overall_status_failed = raw
            .status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("error"));
        for raw_linter in raw.linters.into_iter().filter(|linter| linter.is_active) {
            let policy_entry = resolve_raw_policy(&policy, &raw_linter);
            let tool_id = policy_entry.map_or_else(
                || opaque_adapter_identifier("MEGALINTER_UNMAPPED_TOOL", raw_linter.identity()),
                |entry| entry.id.clone(),
            );
            if !observed_tools.insert(tool_id.clone()) {
                fully_covered = false;
            }
            active_tools.insert(tool_id.clone());
            let errors = raw_linter.total_number_errors.unwrap_or_default();
            let warnings = raw_linter.total_number_warnings.unwrap_or_default();
            let detailed = detailed_by_tool.get(&tool_id).copied().unwrap_or_default();
            let counts_present = raw_linter.total_number_errors.is_some()
                && raw_linter.total_number_warnings.is_some();
            if !counts_present
                || detailed.errors != errors
                || detailed.warnings != warnings
                || detailed.runs > 1
            {
                fully_covered = false;
                normalized.findings.push(aggregate_finding(
                    &tool_id,
                    policy_entry,
                    profile,
                    errors,
                    warnings,
                ));
            }
            normalized.tool_results.push(normalize_tool_result(
                &raw_linter,
                &tool_id,
                policy_entry,
                profile,
            ));
        }
        if detailed_by_tool
            .keys()
            .any(|tool_id| !active_tools.contains(tool_id))
        {
            fully_covered = false;
        }
        let has_observed_failure = normalized.tool_results.iter().any(|tool| {
            matches!(
                tool.status,
                ToolStatus::ConfigurationError
                    | ToolStatus::ExecutionError
                    | ToolStatus::TimedOut
                    | ToolStatus::FailedFindings
            )
        });
        if overall_return_code != 0 && !has_observed_failure {
            fully_covered = false;
        }
        if overall_status_failed && !has_observed_failure {
            fully_covered = false;
        }
    } else {
        fully_covered = false;
    }

    uniquify_findings(&mut normalized.findings);

    normalized
        .tool_results
        .sort_by(|left, right| left.tool_id.cmp(&right.tool_id));
    normalized.findings.sort_by(finding_order);
    if fully_covered {
        normalized.completeness = ReportCompleteness::Normalized;
    }
    for tool in &normalized.tool_results {
        tool.validate()?;
    }
    for finding in &normalized.findings {
        finding.validate()?;
    }
    Ok(Some(normalized))
}

#[derive(Debug, Deserialize)]
struct RawMegaLinter {
    /// `MegaLinter` v10's JSON reporter emits this exact top-level collection.
    linters: Vec<RawLinter>,
    /// Required by the supported v10 reporter envelope.
    return_code: i32,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawLinter {
    #[serde(default)]
    descriptor_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    linter_name: Option<String>,
    is_active: bool,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    return_code: Option<i32>,
    #[serde(default)]
    total_number_errors: Option<u64>,
    #[serde(default)]
    total_number_warnings: Option<u64>,
    #[serde(default)]
    elapsed_time_s: Option<f64>,
    #[serde(default)]
    config_file_error: Option<Value>,
    #[serde(default)]
    activation_skip_reason: Option<String>,
}

impl RawLinter {
    fn identity(&self) -> &str {
        self.name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                self.linter_name
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                self.descriptor_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or("unspecified")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DetailedCounts {
    errors: u64,
    warnings: u64,
    runs: u64,
}

#[derive(Debug, Deserialize)]
struct RawPolicyMatrix {
    tools: Vec<PolicyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolicyEntry {
    id: String,
    name: String,
    owner: String,
    policy_source: String,
    enforcement: Enforcement,
    #[serde(default)]
    profile_enforcement: BTreeMap<String, Enforcement>,
    #[serde(default)]
    configuration_path: Option<PathBuf>,
}

#[derive(Debug)]
struct PolicyIndex {
    by_id: BTreeMap<String, PolicyEntry>,
    names: BTreeMap<String, String>,
}

impl PolicyIndex {
    fn load() -> Result<Self> {
        let matrix: RawPolicyMatrix = serde_json::from_str(POLICY_MATRIX)?;
        let mut by_id = BTreeMap::new();
        let mut names = BTreeMap::new();
        let mut ambiguous_names = BTreeSet::new();
        for entry in matrix.tools {
            if by_id.contains_key(&entry.id) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate MegaLinter policy id {}",
                    entry.id
                )));
            }
            let folded_name = entry.name.to_ascii_lowercase();
            if names
                .insert(folded_name.clone(), entry.id.clone())
                .is_some()
            {
                ambiguous_names.insert(folded_name);
            }
            by_id.insert(entry.id.clone(), entry);
        }
        for name in ambiguous_names {
            names.remove(&name);
        }
        Ok(Self { by_id, names })
    }

    fn resolve(&self, value: &str) -> Option<&PolicyEntry> {
        self.by_id.get(value).or_else(|| {
            self.names
                .get(&value.to_ascii_lowercase())
                .and_then(|id| self.by_id.get(id))
        })
    }
}

fn resolve_raw_policy<'a>(policy: &'a PolicyIndex, linter: &RawLinter) -> Option<&'a PolicyEntry> {
    for candidate in [linter.name.as_deref(), linter.linter_name.as_deref()]
        .into_iter()
        .flatten()
    {
        if let Some(entry) = policy.resolve(candidate) {
            return Some(entry);
        }
    }
    if let (Some(descriptor), Some(linter_name)) = (
        linter.descriptor_id.as_deref(),
        linter.linter_name.as_deref(),
    ) {
        let combined = format!("{descriptor}_{linter_name}").to_ascii_uppercase();
        return policy.resolve(&combined);
    }
    None
}

fn normalize_tool_result(
    raw: &RawLinter,
    tool_id: &str,
    policy: Option<&PolicyEntry>,
    profile: Profile,
) -> ToolResult {
    let errors = raw.total_number_errors.unwrap_or_default();
    let warnings = raw.total_number_warnings.unwrap_or_default();
    let status_text = raw
        .status
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let has_config_error = raw
        .config_file_error
        .as_ref()
        .is_some_and(|value| !value.is_null() && value != "");
    let status = if has_config_error {
        ToolStatus::ConfigurationError
    } else if status_text.contains("timeout") {
        ToolStatus::TimedOut
    } else if raw
        .activation_skip_reason
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        ToolStatus::NotApplicable
    } else if errors > 0 {
        ToolStatus::FailedFindings
    } else if raw.return_code.is_some_and(|code| code != 0) || status_text == "error" {
        ToolStatus::ExecutionError
    } else if warnings > 0 {
        ToolStatus::PassedWithWarnings
    } else {
        ToolStatus::Passed
    };
    ToolResult {
        schema_version: CONTRACT_VERSION,
        tool_id: tool_id.to_owned(),
        owner: policy
            .map_or("egolint:adapter-unmapped", |entry| entry.owner.as_str())
            .to_owned(),
        policy_source: policy
            .map_or(".config/megalinter/tool-matrix.json", |entry| {
                entry.policy_source.as_str()
            })
            .to_owned(),
        status,
        enforcement: effective_enforcement(policy, profile),
        finding_count: errors,
        warning_count: warnings,
        duration_ms: duration_milliseconds(raw.elapsed_time_s),
        evidence: vec![adapter_evidence(
            RAW_JSON_REPORT,
            "Whitelisted MegaLinter tool status; raw output remains private.",
        )],
    }
}

// Keep the bounded SARIF walk linear so every raw-field access is visibly
// paired with its sanitization and count-reconciliation step.
#[allow(clippy::too_many_lines)]
fn normalize_sarif(
    document: &Value,
    policy: &PolicyIndex,
    profile: Profile,
    findings: &mut Vec<Finding>,
    detailed_by_tool: &mut BTreeMap<String, DetailedCounts>,
) -> Result<()> {
    if document.get("version").and_then(Value::as_str) != Some(SUPPORTED_SARIF_VERSION) {
        return Err(EgolintError::RuntimeExecution(format!(
            "MegaLinter SARIF must use supported version {SUPPORTED_SARIF_VERSION}"
        )));
    }
    let runs = document
        .get("runs")
        .and_then(Value::as_array)
        .ok_or_else(|| EgolintError::RuntimeExecution("SARIF runs must be an array".to_owned()))?;
    if runs.len() > MAX_SARIF_RUNS {
        return Err(EgolintError::RuntimeExecution(format!(
            "SARIF contains more than {MAX_SARIF_RUNS} runs"
        )));
    }
    let mut total_results = 0usize;
    for run in runs {
        let driver_name = run
            .pointer("/tool/driver/name")
            .and_then(Value::as_str)
            .unwrap_or("MEGALINTER_SARIF");
        let policy_entry = policy.resolve(driver_name);
        let tool_id = policy_entry.map_or_else(
            || opaque_adapter_identifier("MEGALINTER_UNMAPPED_TOOL", driver_name),
            |entry| entry.id.clone(),
        );
        let detailed_counts = detailed_by_tool.entry(tool_id.clone()).or_default();
        detailed_counts.runs = detailed_counts.runs.saturating_add(1);
        let results = match run.get("results") {
            Some(results) => results.as_array().map(Vec::as_slice).ok_or_else(|| {
                EgolintError::RuntimeExecution("SARIF results must be an array".to_owned())
            })?,
            None => &[],
        };
        total_results = total_results.saturating_add(results.len());
        if total_results > MAX_SARIF_RESULTS {
            return Err(EgolintError::RuntimeExecution(format!(
                "SARIF contains more than {MAX_SARIF_RESULTS} results"
            )));
        }
        for result in results {
            if !sarif_result_is_finding(result)? {
                continue;
            }
            let rule_id = match result.get("ruleId") {
                Some(Value::String(value)) => opaque_adapter_identifier("MEGALINTER_RULE", value),
                None => "MEGALINTER_RULE_UNSPECIFIED".to_owned(),
                Some(_) => {
                    return Err(EgolintError::RuntimeExecution(
                        "SARIF ruleId must be a string".to_owned(),
                    ));
                }
            };
            let location = sarif_location(result)?;
            let reported_severity = match result.get("level") {
                Some(Value::String(level)) if level == "error" => Severity::Error,
                Some(Value::String(level)) if level == "warning" => Severity::Warning,
                Some(Value::String(level)) if level == "note" => Severity::Info,
                None => Severity::Warning,
                Some(_) => {
                    return Err(EgolintError::RuntimeExecution(
                        "SARIF result uses an unsupported level".to_owned(),
                    ));
                }
            };
            let severity = if effective_enforcement(policy_entry, profile) == Enforcement::Advisory
                && matches!(reported_severity, Severity::Error | Severity::Critical)
            {
                Severity::Warning
            } else {
                reported_severity
            };
            let fingerprint = stable_fingerprint(
                &tool_id,
                &rule_id,
                location.as_ref().map(|value| value.path.as_path()),
                location.as_ref().and_then(|value| value.start_line),
                location.as_ref().and_then(|value| value.start_column),
            );
            findings.push(Finding {
                schema_version: CONTRACT_VERSION,
                id: format!("MEGALINTER-SARIF-{fingerprint}"),
                rule: RuleIdentity {
                    tool_id: tool_id.clone(),
                    rule_id: rule_id.clone(),
                },
                severity,
                message: format!(
                    "MegaLinter reported a normalized finding for {tool_id}. Review the private adapter artifact for tool-native detail."
                ),
                location,
                ownership: ownership(policy_entry),
                fingerprint: Some(fingerprint),
                evidence: vec![adapter_evidence(
                    RAW_SARIF_REPORT,
                    "Sanitized rule identity and source location from private upstream SARIF.",
                )],
                suppressed_by: None,
            });
            let counts = detailed_by_tool.entry(tool_id.clone()).or_default();
            match reported_severity {
                Severity::Error | Severity::Critical => {
                    counts.errors = counts.errors.saturating_add(1);
                }
                Severity::Warning => {
                    counts.warnings = counts.warnings.saturating_add(1);
                }
                Severity::Info => {}
            }
        }
    }
    Ok(())
}

fn sarif_result_is_finding(result: &Value) -> Result<bool> {
    match result.get("kind") {
        None => Ok(true),
        Some(Value::String(kind)) if matches!(kind.as_str(), "fail" | "open" | "review") => {
            Ok(true)
        }
        Some(Value::String(kind))
            if matches!(kind.as_str(), "pass" | "notApplicable" | "informational") =>
        {
            Ok(false)
        }
        Some(_) => Err(EgolintError::RuntimeExecution(
            "SARIF result uses an unsupported kind".to_owned(),
        )),
    }
}

fn sarif_location(result: &Value) -> Result<Option<SourceLocation>> {
    let Some(physical) = result.pointer("/locations/0/physicalLocation") else {
        return Ok(None);
    };
    let Some(uri) = physical
        .pointer("/artifactLocation/uri")
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let path = normalize_sarif_uri(uri)?;
    let region = physical.get("region").unwrap_or(&Value::Null);
    let location = SourceLocation {
        path,
        start_line: json_u32(region.get("startLine"))?,
        start_column: json_u32(region.get("startColumn"))?,
        end_line: json_u32(region.get("endLine"))?,
        end_column: json_u32(region.get("endColumn"))?,
    };
    location.validate()?;
    Ok(Some(location))
}

fn normalize_sarif_uri(uri: &str) -> Result<PathBuf> {
    if uri.len() > 8_192 || uri.chars().any(char::is_control) {
        return Err(EgolintError::RuntimeExecution(
            "SARIF artifact URI is oversized or contains control characters".to_owned(),
        ));
    }
    let decoded = percent_decode(uri)?;
    let relative = decoded
        .strip_prefix("file:///tmp/lint/")
        .or_else(|| decoded.strip_prefix("/tmp/lint/"))
        .or_else(|| decoded.strip_prefix("file://tmp/lint/"))
        .unwrap_or(decoded.as_str());
    if relative.contains('\\') || relative.contains(':') {
        return Err(EgolintError::RuntimeExecution(
            "SARIF artifact URI is not a portable workspace-relative path".to_owned(),
        ));
    }
    let path = PathBuf::from(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::RuntimeExecution(
            "SARIF artifact URI escapes the workspace".to_owned(),
        ));
    }
    Ok(path)
}

fn aggregate_finding(
    tool_id: &str,
    policy: Option<&PolicyEntry>,
    profile: Profile,
    errors: u64,
    warnings: u64,
) -> Finding {
    let severity = if errors > 0 && effective_enforcement(policy, profile) == Enforcement::Blocking
    {
        Severity::Error
    } else {
        Severity::Warning
    };
    let fingerprint = stable_fingerprint(tool_id, "ADAPTER_SUMMARY", None, None, None);
    Finding {
        schema_version: CONTRACT_VERSION,
        id: format!("MEGALINTER-SUMMARY-{fingerprint}"),
        rule: RuleIdentity {
            tool_id: tool_id.to_owned(),
            rule_id: "ADAPTER_SUMMARY".to_owned(),
        },
        severity,
        message: format!(
            "MegaLinter reported {errors} error(s) and {warnings} warning(s) without complete normalized source locations. Review the private adapter artifact."
        ),
        location: None,
        ownership: ownership(policy),
        fingerprint: Some(fingerprint),
        evidence: vec![adapter_evidence(
            RAW_JSON_REPORT,
            "Sanitized aggregate counts from private MegaLinter JSON.",
        )],
        suppressed_by: None,
    }
}

fn effective_enforcement(policy: Option<&PolicyEntry>, profile: Profile) -> Enforcement {
    policy.map_or(Enforcement::Advisory, |entry| {
        entry
            .profile_enforcement
            .get(profile.as_str())
            .copied()
            .unwrap_or(entry.enforcement)
    })
}

fn ownership(policy: Option<&PolicyEntry>) -> RuleOwnership {
    RuleOwnership {
        owner: policy
            .map_or("egolint:adapter-unmapped", |entry| entry.owner.as_str())
            .to_owned(),
        policy_source: policy
            .map_or(".config/megalinter/tool-matrix.json", |entry| {
                entry.policy_source.as_str()
            })
            .to_owned(),
        configuration_path: policy.and_then(|entry| entry.configuration_path.clone()),
    }
}

fn adapter_evidence(path: &str, description: &str) -> EvidenceReference {
    EvidenceReference {
        schema_version: CONTRACT_VERSION,
        kind: EvidenceKind::AdapterReport,
        path: PathBuf::from(path),
        sha256: None,
        description: Some(description.to_owned()),
    }
}

fn duration_milliseconds(seconds: Option<f64>) -> Option<u64> {
    let seconds = seconds?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let milliseconds = seconds * 1_000.0;
    format!("{milliseconds:.0}").parse().ok()
}

fn read_optional_json(path: &Path) -> Result<Option<Value>> {
    let Some((validated_path, metadata)) = validated_report_file(path)? else {
        return Ok(None);
    };
    if metadata.len() > MAX_REPORT_BYTES {
        return Err(EgolintError::RuntimeExecution(format!(
            "adapter report exceeds the {MAX_REPORT_BYTES}-byte normalization limit: {}",
            path.display()
        )));
    }
    let Some((validated_path, opened_metadata)) = validated_report_file(&validated_path)? else {
        return Err(EgolintError::RuntimeExecution(
            "adapter report disappeared before it could be read".to_owned(),
        ));
    };
    if opened_metadata.len() > MAX_REPORT_BYTES {
        return Err(EgolintError::RuntimeExecution(format!(
            "adapter report exceeds the {MAX_REPORT_BYTES}-byte normalization limit"
        )));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(opened_metadata.len()).unwrap_or_default());
    File::open(&validated_path)
        .and_then(|file| file.take(MAX_REPORT_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|source| EgolintError::Filesystem {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return Err(EgolintError::RuntimeExecution(format!(
            "adapter report grew beyond the normalization limit: {}",
            path.display()
        )));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| EgolintError::RuntimeExecution("adapter report is not valid JSON".to_owned()))
}

fn validated_report_file(path: &Path) -> Result<Option<(PathBuf, std::fs::Metadata)>> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| EgolintError::Filesystem {
                path: PathBuf::from("."),
                source,
            })?
            .join(path)
    };
    let parent = absolute.parent().ok_or_else(|| {
        EgolintError::RuntimeExecution("adapter report path has no parent".to_owned())
    })?;
    let mut current = PathBuf::new();
    for component in parent.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
            Component::CurDir | Component::ParentDir => {
                return Err(EgolintError::RuntimeExecution(
                    "adapter report path is not normalized".to_owned(),
                ));
            }
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(EgolintError::RuntimeExecution(
                    "adapter report parent must contain only real directories".to_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(EgolintError::Filesystem {
                    path: current,
                    source,
                });
            }
        }
    }
    match std::fs::symlink_metadata(&absolute) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(EgolintError::RuntimeExecution(
                "adapter report must be a regular file, not a link".to_owned(),
            ))
        }
        Ok(metadata) => Ok(Some((absolute, metadata))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(EgolintError::Filesystem {
            path: absolute,
            source,
        }),
    }
}

fn json_u32(value: Option<&Value>) -> Result<Option<u32>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(number) = value.as_u64() else {
        return Err(EgolintError::RuntimeExecution(
            "SARIF source coordinates must be unsigned integers".to_owned(),
        ));
    };
    u32::try_from(number).map(Some).map_err(|_| {
        EgolintError::RuntimeExecution("SARIF source coordinate exceeds u32".to_owned())
    })
}

fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let Some(high) = bytes.get(index + 1).and_then(|byte| hex_value(*byte)) else {
            return Err(EgolintError::RuntimeExecution(
                "SARIF artifact URI contains invalid percent encoding".to_owned(),
            ));
        };
        let Some(low) = bytes.get(index + 2).and_then(|byte| hex_value(*byte)) else {
            return Err(EgolintError::RuntimeExecution(
                "SARIF artifact URI contains invalid percent encoding".to_owned(),
            ));
        };
        decoded.push((high << 4) | low);
        index += 3;
    }
    let decoded = String::from_utf8(decoded).map_err(|_| {
        EgolintError::RuntimeExecution("SARIF artifact URI is not valid UTF-8".to_owned())
    })?;
    if decoded.chars().any(char::is_control) {
        return Err(EgolintError::RuntimeExecution(
            "SARIF artifact URI decodes to control characters".to_owned(),
        ));
    }
    Ok(decoded)
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn opaque_adapter_identifier(prefix: &str, value: &str) -> String {
    format!("{prefix}_{:x}", Sha256::digest(value.as_bytes()))
}

fn stable_fingerprint(
    tool_id: &str,
    rule_id: &str,
    path: Option<&Path>,
    line: Option<u32>,
    column: Option<u32>,
) -> String {
    let mut digest = Sha256::new();
    digest.update(tool_id.as_bytes());
    digest.update([0]);
    digest.update(rule_id.as_bytes());
    digest.update([0]);
    digest.update(
        path.unwrap_or_else(|| Path::new(""))
            .to_string_lossy()
            .as_bytes(),
    );
    digest.update([0]);
    digest.update(line.unwrap_or_default().to_le_bytes());
    digest.update(column.unwrap_or_default().to_le_bytes());
    format!("megalinter-v1-{:x}", digest.finalize())
}

fn uniquify_findings(findings: &mut [Finding]) {
    let mut seen = BTreeMap::<String, u64>::new();
    for finding in findings {
        let occurrence = seen.entry(finding.id.clone()).or_default();
        *occurrence += 1;
        if *occurrence > 1 {
            finding.id = format!("{}-{occurrence}", finding.id);
            if let Some(fingerprint) = finding.fingerprint.as_mut() {
                fingerprint.push_str(&format!("-{occurrence}"));
            }
        }
    }
}

fn finding_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_path = left
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    let right_path = right
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    left.rule
        .tool_id
        .cmp(&right.rule.tool_id)
        .then_with(|| left.rule.rule_id.cmp(&right.rule.rule_id))
        .then_with(|| left_path.cmp(right_path))
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
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sarif_paths_are_workspace_relative_and_portable() {
        assert_eq!(
            normalize_sarif_uri("file:///tmp/lint/src/lib.rs").expect("container path"),
            PathBuf::from("src/lib.rs")
        );
        assert_eq!(
            normalize_sarif_uri("docs/space%20name.md").expect("encoded relative path"),
            PathBuf::from("docs/space name.md")
        );
        for unsafe_uri in [
            "../../outside",
            "/etc/passwd",
            "file:///etc/passwd",
            "C:%5CUsers%5Calice%5Csecret",
            "src%2F..%2Foutside",
        ] {
            assert!(normalize_sarif_uri(unsafe_uri).is_err(), "{unsafe_uri}");
        }
    }

    #[test]
    fn raw_messages_are_not_copied_into_normalized_findings() {
        let policy = PolicyIndex::load().expect("policy matrix");
        let document = serde_json::json!({
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {"name": "ruff"}},
                "results": [{
                    "ruleId": "F401",
                    "level": "error",
                    "message": {"text": "TOP-SECRET-SOURCE-EXCERPT"},
                    "locations": [{"physicalLocation": {
                        "artifactLocation": {"uri": "src/example.py"},
                        "region": {"startLine": 3, "startColumn": 2}
                    }}]
                }]
            }]
        });
        let mut findings = Vec::new();
        let mut counts = BTreeMap::new();
        normalize_sarif(
            &document,
            &policy,
            Profile::Holistic,
            &mut findings,
            &mut counts,
        )
        .expect("normalized SARIF");

        assert_eq!(findings.len(), 1);
        assert!(!findings[0].message.contains("TOP-SECRET"));
        assert_eq!(
            findings[0].location.as_ref().unwrap().path,
            Path::new("src/example.py")
        );
        assert_eq!(
            counts.get("PYTHON_RUFF"),
            Some(&DetailedCounts {
                errors: 1,
                warnings: 0,
                runs: 1,
            })
        );
        assert_ne!(findings[0].rule.rule_id, "F401");
    }

    #[test]
    fn tool_status_distinguishes_findings_from_execution_errors() {
        let policy = PolicyIndex::load().expect("policy matrix");
        let entry = policy.resolve("PYTHON_RUFF");
        let findings = RawLinter {
            descriptor_id: Some("PYTHON".to_owned()),
            name: Some("PYTHON_RUFF".to_owned()),
            linter_name: Some("ruff".to_owned()),
            is_active: true,
            status: Some("error".to_owned()),
            return_code: Some(1),
            total_number_errors: Some(2),
            total_number_warnings: Some(0),
            elapsed_time_s: Some(0.125),
            config_file_error: None,
            activation_skip_reason: None,
        };
        let failure = RawLinter {
            total_number_errors: Some(0),
            return_code: Some(2),
            ..findings
        };

        assert_eq!(
            normalize_tool_result(&failure, "PYTHON_RUFF", entry, Profile::Holistic).status,
            ToolStatus::ExecutionError
        );
        assert_eq!(duration_milliseconds(Some(0.125)), Some(125));
    }

    #[test]
    fn supported_envelopes_reject_empty_or_wrong_version_documents() {
        assert!(serde_json::from_value::<RawMegaLinter>(serde_json::json!({})).is_err());
        let policy = PolicyIndex::load().expect("policy matrix");
        let mut findings = Vec::new();
        let mut counts = BTreeMap::new();
        assert!(
            normalize_sarif(
                &serde_json::json!({"version": "2.0.0", "runs": []}),
                &policy,
                Profile::Holistic,
                &mut findings,
                &mut counts,
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_adapter_identifiers_are_opaque_and_duplicates_are_unique() {
        let policy = PolicyIndex::load().expect("policy matrix");
        let secret = "PRIVATE-TOKEN-LIKE-TOOL";
        let document = serde_json::json!({
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {"name": secret}},
                "results": [
                    {"ruleId": secret, "level": "warning"},
                    {"ruleId": secret, "level": "warning"}
                ]
            }]
        });
        let mut findings = Vec::new();
        let mut counts = BTreeMap::new();
        normalize_sarif(
            &document,
            &policy,
            Profile::Holistic,
            &mut findings,
            &mut counts,
        )
        .expect("supported SARIF");
        uniquify_findings(&mut findings);

        assert_eq!(findings.len(), 2);
        assert_ne!(findings[0].id, findings[1].id);
        let serialized = serde_json::to_string(&findings).expect("findings JSON");
        assert!(!serialized.contains(secret));
    }

    #[test]
    fn incomplete_counts_remain_partial() {
        let temporary = tempfile::tempdir().expect("temporary workspace");
        let report_directory = temporary.path().join(".reports/egolint");
        std::fs::create_dir_all(&report_directory).expect("report directory");
        std::fs::write(
            report_directory.join("mega-linter-report.json"),
            serde_json::to_vec(&serde_json::json!({
                "return_code": 1,
                "status": "error",
                "linters": [{
                    "descriptor_id": "PYTHON",
                    "name": "PYTHON_RUFF",
                    "linter_name": "ruff",
                    "is_active": true,
                    "status": "error",
                    "return_code": 1,
                    "total_number_errors": 2,
                    "total_number_warnings": 0
                }]
            }))
            .expect("JSON"),
        )
        .expect("raw JSON");
        std::fs::write(
            report_directory.join("mega-linter-report.sarif"),
            serde_json::to_vec(&serde_json::json!({
                "version": "2.1.0",
                "runs": [{
                    "tool": {"driver": {"name": "ruff"}},
                    "results": [{"ruleId": "F401", "level": "error"}]
                }]
            }))
            .expect("SARIF"),
        )
        .expect("raw SARIF");

        let normalized = normalize_workspace(temporary.path(), Profile::Holistic)
            .expect("normalization")
            .expect("adapter evidence");
        assert_eq!(normalized.completeness, ReportCompleteness::Partial);
        assert!(
            normalized
                .findings
                .iter()
                .any(|finding| finding.rule.rule_id == "ADAPTER_SUMMARY")
        );
    }

    #[test]
    fn focused_profile_enforcement_can_reenable_a_baseline_disabled_tool() {
        let policy = PolicyIndex::load().expect("policy matrix");
        let osv = policy.resolve("REPOSITORY_OSV_SCANNER");
        assert_eq!(
            effective_enforcement(osv, Profile::Holistic),
            Enforcement::Disabled
        );
        assert_eq!(
            effective_enforcement(osv, Profile::DependencyDebt),
            Enforcement::Blocking
        );
    }
}
