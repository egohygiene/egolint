//! Stable SARIF 2.1.0 projection of normalized Egolint findings.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::contracts::{Finding, Severity, SuppressionState};
use crate::error::{EgolintError, Result};
use crate::report::{RunReport, RunStatus};

/// Canonical workspace-relative SARIF output path.
pub const EGOLINT_SARIF_REPORT: &str = ".reports/egolint/egolint.sarif";

/// Project a validated `RunReport` into deterministic SARIF 2.1.0.
///
/// The result contains only Egolint-normalized messages and locations. It does
/// not copy raw `MegaLinter` SARIF properties, source excerpts, stdout, or
/// environment state.
///
/// # Errors
///
/// Returns an error when the report or one of its normalized locations is
/// invalid.
pub fn to_sarif(report: &RunReport) -> Result<Value> {
    report.validate()?;
    let mut rules = BTreeMap::<String, Value>::new();
    let mut results = Vec::with_capacity(report.findings.len());
    let mut finding_occurrences = BTreeMap::<String, u64>::new();
    let suppression_states = report
        .suppressions
        .iter()
        .map(|suppression| (suppression.id.as_str(), suppression.state))
        .collect::<BTreeMap<_, _>>();

    for finding in &report.findings {
        let sarif_rule_id = sarif_rule_id(finding);
        let occurrence = finding_occurrences.entry(finding.id.clone()).or_default();
        *occurrence += 1;
        let occurrence = occurrence.to_string();
        let result_id = stable_sarif_id(
            "EGOLINT_RESULT",
            &[finding.id.as_str(), occurrence.as_str()],
        );
        rules
            .entry(sarif_rule_id.clone())
            .or_insert_with(|| sarif_rule(finding, &sarif_rule_id));
        results.push(sarif_result(
            finding,
            &sarif_rule_id,
            &result_id,
            &suppression_states,
        )?);
    }

    let execution_successful = report.status != RunStatus::ExecutionError;
    let rule_values = rules.into_values().collect::<Vec<_>>();
    Ok(json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Egolint",
                    "informationUri": "https://egolint.egohygiene.io",
                    "semanticVersion": env!("CARGO_PKG_VERSION"),
                    "rules": rule_values,
                },
            },
            "invocations": [{
                "executionSuccessful": execution_successful,
                "exitCode": report.egolint_exit_code,
                "properties": {
                    "egolintCompleteness": format!("{:?}", report.completeness).to_ascii_lowercase(),
                    "egolintProfile": report.profile.name.as_str(),
                },
            }],
            "results": results,
        }],
    }))
}

fn sarif_rule(finding: &Finding, sarif_rule_id: &str) -> Value {
    json!({
        "id": sarif_rule_id,
        "name": finding.rule.rule_id,
        "shortDescription": {
            "text": format!(
                "{} rule {}",
                finding.rule.tool_id,
                finding.rule.rule_id,
            ),
        },
        "properties": {
            "egolintToolId": finding.rule.tool_id,
            "egolintOwner": finding.ownership.owner,
            "egolintPolicySource": finding.ownership.policy_source,
            "egolintSeverity": severity_name(finding.severity),
        },
    })
}

fn sarif_result(
    finding: &Finding,
    sarif_rule_id: &str,
    result_id: &str,
    suppression_states: &BTreeMap<&str, SuppressionState>,
) -> Result<Value> {
    let mut result = Map::new();
    result.insert("ruleId".to_owned(), Value::String(sarif_rule_id.to_owned()));
    result.insert(
        "level".to_owned(),
        Value::String(sarif_level(finding.severity).to_owned()),
    );
    result.insert("message".to_owned(), json!({"text": finding.message}));
    if let Some(location) = &finding.location {
        result.insert("locations".to_owned(), sarif_locations(location)?);
    }
    if let Some(fingerprint) = &finding.fingerprint {
        result.insert(
            "partialFingerprints".to_owned(),
            json!({
                "egolint/v2": stable_sarif_id(
                    "EGOLINT_FINGERPRINT",
                    &[finding.id.as_str(), fingerprint.as_str(), result_id],
                ),
            }),
        );
    }
    if let Some(suppression_id) = &finding.suppressed_by {
        let state = suppression_states
            .get(suppression_id.as_str())
            .copied()
            .unwrap_or(SuppressionState::Invalid);
        result.insert(
            "suppressions".to_owned(),
            json!([{
                "kind": "external",
                "status": suppression_status(state),
                "justification": format!(
                    "Egolint suppression {suppression_id} ({})",
                    suppression_state_name(state),
                ),
            }]),
        );
    }
    result.insert(
        "properties".to_owned(),
        json!({
            "egolintFindingId": finding.id,
            "egolintResultId": result_id,
            "egolintToolId": finding.rule.tool_id,
            "egolintOwner": finding.ownership.owner,
            "egolintSuppressionId": finding.suppressed_by,
        }),
    );
    Ok(Value::Object(result))
}

fn sarif_locations(location: &crate::SourceLocation) -> Result<Value> {
    location.validate()?;
    let mut physical = Map::new();
    physical.insert(
        "artifactLocation".to_owned(),
        json!({"uri": portable_path(&location.path)?}),
    );
    let mut region = Map::new();
    insert_number(&mut region, "startLine", location.start_line);
    insert_number(&mut region, "startColumn", location.start_column);
    insert_number(&mut region, "endLine", location.end_line);
    insert_number(&mut region, "endColumn", location.end_column);
    if !region.is_empty() {
        physical.insert("region".to_owned(), Value::Object(region));
    }
    Ok(json!([{"physicalLocation": Value::Object(physical)}]))
}

/// Atomically write canonical SARIF without following an existing target link.
///
/// # Errors
///
/// Returns an error when the report is invalid or the destination cannot be
/// written and replaced durably.
pub fn write_sarif_atomic(report: &RunReport, path: &Path) -> Result<()> {
    let document = to_sarif(report)?;
    write_json_atomic(&document, path)
}

/// Atomically write one reviewed JSON value beside other Egolint evidence.
///
/// This helper is shared by the compact debt projection. The caller must use a
/// previously validated report directory. Existing links are rejected.
///
/// # Errors
///
/// Returns an error for a non-directory parent or any serialization/write
/// failure.
pub fn write_json_atomic(value: &Value, path: &Path) -> Result<()> {
    let (validated_path, parent) = validated_report_target(path)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(&parent).map_err(|source| EgolintError::Filesystem {
            path: parent.clone(),
            source,
        })?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), value)?;
    temporary
        .as_file_mut()
        .write_all(b"\n")
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|source| EgolintError::Filesystem {
            path: validated_path.clone(),
            source,
        })?;
    let (revalidated_path, revalidated_parent) = validated_report_target(&validated_path)?;
    if revalidated_path != validated_path || revalidated_parent != parent {
        return Err(EgolintError::RuntimeExecution(
            "JSON report destination changed before persistence".to_owned(),
        ));
    }
    temporary
        .persist(&validated_path)
        .map_err(|error| EgolintError::Filesystem {
            path: validated_path,
            source: error.error,
        })?;
    Ok(())
}

fn sarif_rule_id(finding: &Finding) -> String {
    stable_sarif_id(
        "EGOLINT_RULE",
        &[finding.rule.tool_id.as_str(), finding.rule.rule_id.as_str()],
    )
}

fn stable_sarif_id(prefix: &str, components: &[&str]) -> String {
    let mut digest = Sha256::new();
    for component in components {
        let length = u64::try_from(component.len()).unwrap_or(u64::MAX);
        digest.update(length.to_le_bytes());
        digest.update(component.as_bytes());
    }
    format!("{prefix}_{:x}", digest.finalize())
}

const fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "note",
        Severity::Warning => "warning",
        Severity::Error | Severity::Critical => "error",
    }
}

const fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Critical => "critical",
    }
}

const fn suppression_status(state: SuppressionState) -> &'static str {
    match state {
        SuppressionState::Applied => "accepted",
        SuppressionState::Unmatched | SuppressionState::Expired | SuppressionState::Invalid => {
            "underReview"
        }
    }
}

const fn suppression_state_name(state: SuppressionState) -> &'static str {
    match state {
        SuppressionState::Applied => "applied",
        SuppressionState::Unmatched => "unmatched",
        SuppressionState::Expired => "expired",
        SuppressionState::Invalid => "invalid",
    }
}

fn portable_path(path: &Path) -> Result<String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(EgolintError::Configuration(
            "SARIF output path must be workspace-relative".to_owned(),
        ));
    }
    let mut components = Vec::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(EgolintError::Configuration(
                "SARIF output path must be normalized".to_owned(),
            ));
        };
        let text = component.to_str().ok_or_else(|| {
            EgolintError::Configuration("SARIF output path must contain valid UTF-8".to_owned())
        })?;
        if text.chars().any(char::is_control) || text.contains(['/', '\\']) {
            return Err(EgolintError::Configuration(
                "SARIF output path contains unsupported characters".to_owned(),
            ));
        }
        components.push(percent_encode_uri_component(text));
    }
    Ok(components.join("/"))
}

fn percent_encode_uri_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for &byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    char::from(b"0123456789ABCDEF"[usize::from(value & 0x0f)])
}

/// Resolve a report destination and reject links in every existing component.
///
/// This check is intentionally repeated immediately before persistence by all
/// atomic writers in this crate. It reduces link-following exposure while the
/// final rename still replaces, rather than opens, the destination.
pub(crate) fn validated_report_target(path: &Path) -> Result<(PathBuf, PathBuf)> {
    if path.as_os_str().is_empty() {
        return Err(EgolintError::Configuration(
            "report destination may not be empty".to_owned(),
        ));
    }
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
        EgolintError::Configuration("report destination must have a parent".to_owned())
    })?;
    let parent = parent.to_path_buf();
    let mut current = PathBuf::new();
    for component in parent.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
            Component::CurDir | Component::ParentDir => {
                return Err(EgolintError::Configuration(
                    "report destination must be normalized".to_owned(),
                ));
            }
        }
        let metadata = std::fs::symlink_metadata(&current).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                EgolintError::MissingPath(current.clone())
            } else {
                EgolintError::Filesystem {
                    path: current.clone(),
                    source,
                }
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(EgolintError::Configuration(
                "report destination parent must contain only real directories".to_owned(),
            ));
        }
    }
    match std::fs::symlink_metadata(&absolute) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(EgolintError::Configuration(
                "report destination may not be a link or non-file".to_owned(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(EgolintError::Filesystem {
                path: absolute,
                source,
            });
        }
    }
    Ok((absolute, parent))
}

fn insert_number(target: &mut Map<String, Value>, name: &str, value: Option<u32>) {
    if let Some(value) = value {
        target.insert(name.to_owned(), Value::from(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        CONTRACT_VERSION, EvidenceReference, ProfileDefinition, RuleIdentity, RuleOwnership,
        SourceLocation,
    };
    use crate::plan::Operation;
    use crate::report::{ReportCompleteness, ReportSummary};
    use crate::{Profile, RunStatus};
    use std::path::PathBuf;

    #[test]
    fn sarif_contains_only_normalized_relative_evidence() {
        let finding = Finding {
            schema_version: CONTRACT_VERSION,
            id: "F-1".to_owned(),
            rule: RuleIdentity {
                tool_id: "EGOLINT_PORTABILITY".to_owned(),
                rule_id: "EGO-PORT-CASE-001".to_owned(),
            },
            severity: Severity::Error,
            message: "Paths collide on a case-insensitive filesystem.".to_owned(),
            location: Some(SourceLocation {
                path: PathBuf::from("src/lib.rs"),
                start_line: Some(4),
                start_column: Some(2),
                end_line: Some(4),
                end_column: Some(8),
            }),
            ownership: RuleOwnership {
                owner: "egohygiene/egolint".to_owned(),
                policy_source: ".config/rules/portability.toml".to_owned(),
                configuration_path: None,
            },
            fingerprint: Some("portable-v1-example".to_owned()),
            evidence: Vec::<EvidenceReference>::new(),
            suppressed_by: None,
        };
        let report = RunReport {
            schema_version: CONTRACT_VERSION,
            generated_at_unix: 0,
            operation: Operation::Check,
            status: RunStatus::Findings,
            egolint_exit_code: 1,
            adapter_exit_code: Some(1),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            profile: ProfileDefinition::built_in(Profile::Fast),
            config_sources: vec!["compiled defaults".to_owned()],
            report_path: PathBuf::from(".reports/egolint/run.json"),
            completeness: ReportCompleteness::Normalized,
            summary: ReportSummary {
                normalized_tool_results: 0,
                normalized_findings: 1,
                normalized_suppressions: 0,
            },
            tool_results: Vec::new(),
            findings: vec![finding],
            suppressions: Vec::new(),
            evidence: Vec::new(),
        };

        let sarif = to_sarif(&report).expect("valid SARIF");
        assert_eq!(sarif["version"], "2.1.0");
        assert_eq!(
            sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
                ["uri"],
            "src/lib.rs"
        );
        assert_eq!(
            sarif["runs"][0]["results"][0]["properties"]["egolintFindingId"],
            "F-1"
        );
        let rule_id = sarif["runs"][0]["results"][0]["ruleId"]
            .as_str()
            .expect("SARIF rule id");
        assert!(rule_id.starts_with("EGOLINT_RULE_"));
        assert_ne!(rule_id, "EGOLINT_PORTABILITY/EGO-PORT-CASE-001");
    }

    #[test]
    fn portable_path_rejects_traversal_and_platform_separators() {
        assert!(portable_path(Path::new("../outside")).is_err());
        assert!(portable_path(Path::new("src\\windows.rs")).is_err());
        assert_eq!(
            portable_path(Path::new("docs/guide.md")).unwrap(),
            "docs/guide.md"
        );
        assert_eq!(
            portable_path(Path::new("docs/space #100%.md")).unwrap(),
            "docs/space%20%23100%25.md"
        );
        assert_eq!(
            portable_path(Path::new("docs/café.md")).unwrap(),
            "docs/caf%C3%A9.md"
        );
    }

    #[test]
    fn structured_sarif_ids_do_not_have_separator_collisions() {
        assert_ne!(
            stable_sarif_id("EGOLINT_RULE", &["tool/rule", "id"]),
            stable_sarif_id("EGOLINT_RULE", &["tool", "rule/id"]),
        );
    }

    #[cfg(unix)]
    #[test]
    fn report_target_rejects_linked_parent() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let real = temporary.path().join("real");
        std::fs::create_dir(&real).expect("real directory");
        let linked = temporary.path().join("linked");
        symlink(&real, &linked).expect("directory symlink");

        assert!(validated_report_target(&linked.join("report.json")).is_err());
    }
}
