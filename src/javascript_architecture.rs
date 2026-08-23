//! Egolint-owned JavaScript/TypeScript architecture policy with a dependency-cruiser adapter.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::contracts::{
    CONTRACT_VERSION, Finding, RuleIdentity, RuleOwnership, Severity, SourceLocation,
    validate_contract_date,
};
use crate::error::{EgolintError, Result, exit_code};

/// Current Egolint JavaScript architecture profile contract version.
pub const JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION: u32 = 1;
/// dependency-cruiser version reviewed for this profile contract.
pub const DEPENDENCY_CRUISER_VERSION: &str = "18.2.0";
/// Stable Egolint tool identity for dependency graph findings.
pub const ARCHITECTURE_TOOL_ID: &str = "DEPENDENCY_CRUISER";
/// Canonical policy path embedded into the Egolint binary.
pub const BUILTIN_PROFILE_PATH: &str = ".config/rules/javascript-architecture.v1.json";

const BUILTIN_PROFILE: &str = include_str!("../.config/rules/javascript-architecture.v1.json");

/// Versioned architecture profile owned by Egolint rather than dependency-cruiser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JavascriptArchitectureProfile {
    /// Profile schema version.
    pub schema_version: u32,
    /// Stable profile identifier.
    pub id: String,
    /// Semantic profile version.
    pub version: String,
    /// Canonical policy source identifier.
    pub policy_source: String,
    /// Tool adapter declaration.
    pub adapter: ArchitectureAdapter,
    /// Workspace-relative roots dependency-cruiser should inspect when present.
    pub roots: Vec<String>,
    /// Ordered Egolint-owned architecture rules.
    pub rules: Vec<ArchitectureRule>,
}

/// Reviewed adapter contract for one external graph engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureAdapter {
    /// Adapter implementation identifier.
    pub name: String,
    /// Exact reviewed dependency-cruiser version.
    pub version: String,
}

/// One Egolint architecture rule translated mechanically to dependency-cruiser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureRule {
    /// Stable Egolint rule identifier.
    pub id: String,
    /// Normalized severity.
    pub severity: ArchitectureSeverity,
    /// Human-readable policy intent.
    pub description: String,
    /// Source-module restriction.
    pub from: ArchitectureRestriction,
    /// Target-module restriction.
    pub to: ArchitectureRestriction,
    /// Actionable remediation guidance.
    pub remediation: String,
}

/// Supported restriction surface intentionally narrower than dependency-cruiser's full schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureRestriction {
    /// Regexes that must match a module path.
    #[serde(default)]
    pub path: Vec<String>,
    /// Regexes that must not match a module path.
    #[serde(default)]
    pub path_not: Vec<String>,
    /// Match circular dependencies.
    #[serde(default)]
    pub circular: Option<bool>,
    /// Match orphan modules. Valid only for a source restriction.
    #[serde(default)]
    pub orphan: Option<bool>,
    /// Match dependencies dependency-cruiser cannot resolve.
    #[serde(default)]
    pub could_not_resolve: Option<bool>,
    /// Match dependency types such as `core`, `local`, or `npm`.
    #[serde(default)]
    pub dependency_types: Vec<String>,
    /// Exclude dependency types.
    #[serde(default)]
    pub dependency_types_not: Vec<String>,
}

/// Normalized Egolint severity accepted by architecture rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureSeverity {
    /// Informational evidence.
    Info,
    /// Advisory violation.
    Warning,
    /// Blocking violation.
    Error,
}

/// Repository-owned overlay that extends a canonical profile without replacing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JavascriptArchitectureOverlay {
    /// Overlay schema version.
    pub schema_version: u32,
    /// Canonical profile this overlay targets.
    pub profile_id: String,
    /// Additional repository-specific rules.
    #[serde(default)]
    pub add_rules: Vec<ArchitectureRule>,
    /// Owned, expiring exceptions evaluated by Egolint after adapter execution.
    #[serde(default)]
    pub exceptions: Vec<ArchitectureException>,
}

/// Owned exception to a normalized architecture violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureException {
    /// Stable exception identity.
    pub id: String,
    /// Egolint rule selected by this exception.
    pub rule_id: String,
    /// Accountable person, team, or repository.
    pub owner: String,
    /// Reviewable justification.
    pub reason: String,
    /// Required expiry date (`YYYY-MM-DD`).
    pub expires_on: String,
    /// Optional exact source-module selector.
    pub from: Option<String>,
    /// Optional exact target-module selector.
    pub to: Option<String>,
}

/// Observed state of a repository architecture exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureExceptionState {
    /// Exception matched at least one finding and remained current.
    Applied,
    /// Exception was current but matched no finding.
    Unmatched,
    /// Exception expired and did not suppress findings.
    Expired,
}

/// Exception plus its evaluated state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvaluatedArchitectureException {
    /// Declared exception.
    #[serde(flatten)]
    pub exception: ArchitectureException,
    /// Evaluated state.
    pub state: ArchitectureExceptionState,
}

/// Graph-specific metadata wrapped around the shared Egolint finding contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureFinding {
    /// Shared normalized Egolint finding.
    #[serde(flatten)]
    pub finding: Finding,
    /// Source module reported by dependency-cruiser.
    pub source_module: String,
    /// Target module reported by dependency-cruiser, when applicable.
    pub target_module: Option<String>,
    /// Dependency/cycle path when dependency-cruiser provides one.
    pub dependency_path: Vec<String>,
    /// Adapter version that produced the evidence.
    pub dependency_cruiser_version: String,
    /// Egolint remediation associated with the canonical rule.
    pub remediation: String,
}

/// Deterministic architecture report emitted for JSON/SARIF consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JavascriptArchitectureReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Canonical profile identifier.
    pub profile_id: String,
    /// Canonical profile version.
    pub profile_version: String,
    /// Canonical policy source.
    pub policy_source: String,
    /// Exact adapter version observed at runtime.
    pub dependency_cruiser_version: String,
    /// Evaluated findings in deterministic order.
    pub findings: Vec<ArchitectureFinding>,
    /// Evaluated repository exceptions.
    pub exceptions: Vec<EvaluatedArchitectureException>,
    /// Deterministic report summary.
    pub summary: ArchitectureSummary,
}

/// Summary counts used by local and CI gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureSummary {
    /// Total normalized findings, including valid suppressions.
    pub findings: u64,
    /// Unsuppressed error findings.
    pub errors: u64,
    /// Unsuppressed warning findings.
    pub warnings: u64,
    /// Findings suppressed by current owned exceptions.
    pub suppressed: u64,
    /// Expired exceptions observed during evaluation.
    pub expired_exceptions: u64,
}

/// Inputs for one architecture adapter execution.
pub struct ArchitectureRunOptions<'a> {
    /// Repository workspace.
    pub workspace: &'a Path,
    /// Optional repository-local profile override. The built-in profile is canonical by default.
    pub profile_path: Option<&'a Path>,
    /// Repository overlay files.
    pub overlay_paths: &'a [PathBuf],
    /// Auditable date for exception expiry evaluation.
    pub evaluation_date: &'a str,
    /// JSON report path relative to the workspace.
    pub json_output: &'a Path,
    /// SARIF report path relative to the workspace.
    pub sarif_output: &'a Path,
    /// Optional DOT graph path relative to the workspace.
    pub graph_output: Option<&'a Path>,
}

/// Run the pinned dependency-cruiser adapter and emit normalized Egolint outputs.
///
/// # Errors
///
/// Returns configuration errors for invalid profiles/overlays, runtime errors for
/// missing or incompatible tooling, and filesystem errors for output failures.
pub fn run_javascript_architecture(options: &ArchitectureRunOptions<'_>) -> Result<i32> {
    validate_contract_date(options.evaluation_date)?;
    let (profile_path, profile) = load_profile(options.workspace, options.profile_path)?;
    validate_profile(&profile)?;
    let overlays = load_overlays(options.workspace, options.overlay_paths, &profile.id)?;
    let roots = existing_roots(options.workspace, &profile.roots)?;

    let observed_version = dependency_cruiser_version(options.workspace)?;
    if observed_version != profile.adapter.version {
        return Err(EgolintError::RuntimeUnavailable(format!(
            "dependency-cruiser version mismatch: profile requires {}, found {observed_version}",
            profile.adapter.version
        )));
    }

    let (rules, exceptions) = resolve_policy(&profile, &overlays)?;
    let generated_config = dependency_cruiser_config(&rules);
    let raw_report = run_dependency_cruiser(
        options.workspace,
        &roots,
        &generated_config,
        "json",
    )?;
    let mut findings = normalize_violations(
        &raw_report,
        &rules,
        &profile_path,
        &observed_version,
    )?;
    let evaluated_exceptions = apply_architecture_exceptions(
        &mut findings,
        &exceptions,
        options.evaluation_date,
    )?;
    findings.sort_by(|left, right| {
        (
            left.finding.rule.rule_id.as_str(),
            left.source_module.as_str(),
            left.target_module.as_deref().unwrap_or(""),
            left.finding.id.as_str(),
        )
            .cmp(&(
                right.finding.rule.rule_id.as_str(),
                right.source_module.as_str(),
                right.target_module.as_deref().unwrap_or(""),
                right.finding.id.as_str(),
            ))
    });

    let report = build_report(&profile, observed_version, findings, evaluated_exceptions);
    write_json(options.workspace, options.json_output, &report)?;
    write_sarif(options.workspace, options.sarif_output, &report)?;

    if let Some(graph_output) = options.graph_output {
        let dot = run_dependency_cruiser(
            options.workspace,
            &roots,
            &generated_config,
            "dot",
        )?;
        write_bytes(options.workspace, graph_output, dot.as_bytes())?;
    }

    Ok(if report.summary.errors > 0 || report.summary.expired_exceptions > 0 {
        exit_code::FINDINGS
    } else {
        exit_code::CLEAN
    })
}

fn load_profile(
    workspace: &Path,
    requested: Option<&Path>,
) -> Result<(PathBuf, JavascriptArchitectureProfile)> {
    if let Some(requested) = requested {
        let relative = validate_workspace_path(requested, "architecture profile")?;
        let path = workspace.join(&relative);
        let contents = fs::read(&path).map_err(|source| EgolintError::Filesystem {
            path: path.clone(),
            source,
        })?;
        let profile = serde_json::from_slice(&contents).map_err(|error| {
            EgolintError::Configuration(format!(
                "invalid JavaScript architecture profile at {}: {error}",
                relative.display()
            ))
        })?;
        return Ok((relative, profile));
    }
    let profile = serde_json::from_str(BUILTIN_PROFILE).map_err(|error| {
        EgolintError::Configuration(format!(
            "embedded JavaScript architecture profile is invalid: {error}"
        ))
    })?;
    Ok((PathBuf::from(BUILTIN_PROFILE_PATH), profile))
}

fn load_overlays(
    workspace: &Path,
    requested: &[PathBuf],
    profile_id: &str,
) -> Result<Vec<JavascriptArchitectureOverlay>> {
    let mut overlays = Vec::new();
    for overlay_path in requested {
        let relative = validate_workspace_path(overlay_path, "architecture overlay")?;
        let path = workspace.join(&relative);
        let contents = fs::read(&path).map_err(|source| EgolintError::Filesystem {
            path: path.clone(),
            source,
        })?;
        let overlay: JavascriptArchitectureOverlay = serde_json::from_slice(&contents).map_err(|error| {
            EgolintError::Configuration(format!(
                "invalid JavaScript architecture overlay at {}: {error}",
                relative.display()
            ))
        })?;
        if overlay.schema_version != JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION {
            return Err(EgolintError::Configuration(format!(
                "architecture overlay {} schema_version must equal {}",
                relative.display(), JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION
            )));
        }
        if overlay.profile_id != profile_id {
            return Err(EgolintError::Configuration(format!(
                "architecture overlay {} targets profile {}, expected {profile_id}",
                relative.display(), overlay.profile_id
            )));
        }
        overlays.push(overlay);
    }
    Ok(overlays)
}

fn validate_profile(profile: &JavascriptArchitectureProfile) -> Result<()> {
    if profile.schema_version != JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION {
        return Err(EgolintError::Configuration(format!(
            "JavaScript architecture profile schema_version must equal {JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION}"
        )));
    }
    if !valid_identifier(&profile.id)
        || profile.version.trim().is_empty()
        || profile.policy_source.trim().is_empty()
    {
        return Err(EgolintError::Configuration(
            "JavaScript architecture profile id, version, and policy_source are invalid".to_owned(),
        ));
    }
    if profile.adapter.name != "dependency-cruiser"
        || profile.adapter.version != DEPENDENCY_CRUISER_VERSION
    {
        return Err(EgolintError::Configuration(format!(
            "JavaScript architecture profile must use reviewed dependency-cruiser {DEPENDENCY_CRUISER_VERSION}"
        )));
    }
    if profile.roots.is_empty() {
        return Err(EgolintError::Configuration(
            "JavaScript architecture profile roots may not be empty".to_owned(),
        ));
    }
    for root in &profile.roots {
        validate_workspace_path(Path::new(root), "architecture root")?;
    }
    let mut ids = BTreeSet::new();
    for rule in &profile.rules {
        validate_rule(rule)?;
        if !ids.insert(rule.id.clone()) {
            return Err(EgolintError::Configuration(format!(
                "duplicate architecture rule id: {}", rule.id
            )));
        }
    }
    Ok(())
}

fn validate_rule(rule: &ArchitectureRule) -> Result<()> {
    if !valid_identifier(&rule.id)
        || rule.description.trim().is_empty()
        || rule.remediation.trim().is_empty()
    {
        return Err(EgolintError::Configuration(
            "architecture rules require a kebab-case id, description, and remediation".to_owned(),
        ));
    }
    if rule.to.orphan.is_some() {
        return Err(EgolintError::Configuration(format!(
            "architecture rule {} may use orphan only in from", rule.id
        )));
    }
    Ok(())
}

fn resolve_policy(
    profile: &JavascriptArchitectureProfile,
    overlays: &[JavascriptArchitectureOverlay],
) -> Result<(Vec<ArchitectureRule>, Vec<ArchitectureException>)> {
    let mut rules = profile.rules.clone();
    let mut rule_ids = rules.iter().map(|rule| rule.id.clone()).collect::<BTreeSet<_>>();
    let mut exceptions = Vec::new();
    let mut exception_ids = BTreeSet::new();
    for overlay in overlays {
        for rule in &overlay.add_rules {
            validate_rule(rule)?;
            if !rule_ids.insert(rule.id.clone()) {
                return Err(EgolintError::Configuration(format!(
                    "repository overlay may not replace canonical architecture rule {}", rule.id
                )));
            }
            rules.push(rule.clone());
        }
        for exception in &overlay.exceptions {
            validate_exception(exception)?;
            if !rule_ids.contains(&exception.rule_id) {
                return Err(EgolintError::Configuration(format!(
                    "architecture exception {} references unknown rule {}",
                    exception.id, exception.rule_id
                )));
            }
            if !exception_ids.insert(exception.id.clone()) {
                return Err(EgolintError::Configuration(format!(
                    "duplicate architecture exception id: {}", exception.id
                )));
            }
            exceptions.push(exception.clone());
        }
    }
    rules.sort_by(|left, right| left.id.cmp(&right.id));
    exceptions.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((rules, exceptions))
}

fn validate_exception(exception: &ArchitectureException) -> Result<()> {
    if !valid_identifier(&exception.id)
        || !valid_identifier(&exception.rule_id)
        || exception.owner.trim().is_empty()
        || exception.reason.trim().is_empty()
    {
        return Err(EgolintError::Configuration(
            "architecture exceptions require kebab-case id/rule_id, owner, and reason".to_owned(),
        ));
    }
    validate_contract_date(&exception.expires_on)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || (byte == b'-' && index > 0 && index + 1 < value.len())
            })
        && !value.contains("--")
}

fn existing_roots(workspace: &Path, declared: &[String]) -> Result<Vec<String>> {
    let roots = declared
        .iter()
        .filter(|root| workspace.join(root.as_str()).is_dir())
        .cloned()
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(EgolintError::Configuration(format!(
            "none of the JavaScript architecture roots exist: {}",
            declared.join(", ")
        )));
    }
    Ok(roots)
}

fn dependency_cruiser_version(workspace: &Path) -> Result<String> {
    let output = Command::new("pnpm")
        .current_dir(workspace)
        .args(["exec", "depcruise", "--version"])
        .output()
        .map_err(|error| EgolintError::RuntimeUnavailable(format!(
            "could not start pnpm/dependency-cruiser: {error}"
        )))?;
    if !output.status.success() {
        return Err(EgolintError::RuntimeUnavailable(format!(
            "dependency-cruiser is unavailable; install the profile's pinned dev dependency (stderr: {})",
            bounded_text(&String::from_utf8_lossy(&output.stderr))
        )));
    }
    normalize_dependency_cruiser_version(String::from_utf8_lossy(&output.stdout).trim())
}

fn normalize_dependency_cruiser_version(raw: &str) -> Result<String> {
    let candidate = raw
        .split_whitespace()
        .last()
        .unwrap_or(raw)
        .rsplit('@')
        .next()
        .unwrap_or(raw)
        .trim_start_matches('v');
    if candidate.split('.').count() == 3
        && candidate
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        Ok(candidate.to_owned())
    } else {
        Err(EgolintError::RuntimeExecution(format!(
            "could not parse dependency-cruiser version from: {}",
            bounded_text(raw)
        )))
    }
}

fn dependency_cruiser_config(rules: &[ArchitectureRule]) -> Value {
    let forbidden = rules
        .iter()
        .map(|rule| {
            json!({
                "name": rule.id,
                "comment": rule.description,
                "severity": match rule.severity {
                    ArchitectureSeverity::Info => "info",
                    ArchitectureSeverity::Warning => "warn",
                    ArchitectureSeverity::Error => "error",
                },
                "from": restriction_json(&rule.from, true),
                "to": restriction_json(&rule.to, false),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "forbidden": forbidden,
        "allowed": [],
        "required": [],
        "options": {
            "doNotFollow": {"path": "node_modules"},
            "exclude": "(^|/)node_modules(/|$)",
            "skipAnalysisNotInRules": true
        }
    })
}

fn restriction_json(restriction: &ArchitectureRestriction, allow_orphan: bool) -> Value {
    let mut object = serde_json::Map::new();
    if !restriction.path.is_empty() {
        object.insert("path".to_owned(), json!(restriction.path));
    }
    if !restriction.path_not.is_empty() {
        object.insert("pathNot".to_owned(), json!(restriction.path_not));
    }
    if let Some(circular) = restriction.circular {
        object.insert("circular".to_owned(), json!(circular));
    }
    if allow_orphan {
        if let Some(orphan) = restriction.orphan {
            object.insert("orphan".to_owned(), json!(orphan));
        }
    }
    if let Some(could_not_resolve) = restriction.could_not_resolve {
        object.insert("couldNotResolve".to_owned(), json!(could_not_resolve));
    }
    if !restriction.dependency_types.is_empty() {
        object.insert("dependencyTypes".to_owned(), json!(restriction.dependency_types));
    }
    if !restriction.dependency_types_not.is_empty() {
        object.insert(
            "dependencyTypesNot".to_owned(),
            json!(restriction.dependency_types_not),
        );
    }
    Value::Object(object)
}

fn run_dependency_cruiser(
    workspace: &Path,
    roots: &[String],
    config: &Value,
    output_type: &str,
) -> Result<String> {
    let mut config_file = NamedTempFile::new().map_err(|source| EgolintError::Filesystem {
        path: PathBuf::from("temporary-dependency-cruiser-config.json"),
        source,
    })?;
    serde_json::to_writer_pretty(&mut config_file, config)?;
    config_file.flush().map_err(|source| EgolintError::Filesystem {
        path: PathBuf::from("temporary-dependency-cruiser-config.json"),
        source,
    })?;
    let config_path = config_file.path().to_string_lossy().into_owned();
    let output = Command::new("pnpm")
        .current_dir(workspace)
        .args([
            "exec",
            "depcruise",
            "--config",
            &config_path,
            "--output-type",
            output_type,
            "--",
        ])
        .args(roots)
        .output()
        .map_err(|error| EgolintError::RuntimeExecution(format!(
            "could not start dependency-cruiser: {error}"
        )))?;
    let code = output.status.code().unwrap_or(exit_code::RUNTIME);
    if !matches!(code, 0 | 1) {
        return Err(EgolintError::RuntimeExecution(format!(
            "dependency-cruiser exited with {code}: {}",
            bounded_text(&String::from_utf8_lossy(&output.stderr))
        )));
    }
    String::from_utf8(output.stdout).map_err(|error| EgolintError::RuntimeExecution(format!(
        "dependency-cruiser emitted non-UTF-8 output: {error}"
    )))
}

fn normalize_violations(
    raw_report: &str,
    rules: &[ArchitectureRule],
    profile_path: &Path,
    adapter_version: &str,
) -> Result<Vec<ArchitectureFinding>> {
    let document: Value = serde_json::from_str(raw_report)?;
    let violations = document
        .pointer("/summary/violations")
        .and_then(Value::as_array)
        .ok_or_else(|| EgolintError::RuntimeExecution(
            "dependency-cruiser JSON omitted summary.violations".to_owned(),
        ))?;
    let rules_by_id = rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    let mut findings = Vec::new();
    for violation in violations {
        let rule_id = violation
            .pointer("/rule/name")
            .and_then(Value::as_str)
            .unwrap_or("unnamed");
        let Some(rule) = rules_by_id.get(rule_id) else {
            return Err(EgolintError::RuntimeExecution(format!(
                "dependency-cruiser returned undeclared rule {rule_id}"
            )));
        };
        let source = violation
            .get("from")
            .and_then(Value::as_str)
            .unwrap_or("unknown-module");
        let target = violation.get("to").and_then(Value::as_str).map(str::to_owned);
        let dependency_path = violation
            .get("cycle")
            .and_then(Value::as_array)
            .map(|cycle| {
                cycle
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let fingerprint = stable_hash(&[
            rule_id,
            source,
            target.as_deref().unwrap_or(""),
            &dependency_path.join("->"),
        ]);
        let id = format!("architecture-{}", &fingerprint[..16]);
        let message = target.as_ref().map_or_else(
            || format!("{}: {source}. {}", rule.description, rule.remediation),
            |target| {
                format!(
                    "{}: {source} -> {target}. {}",
                    rule.description, rule.remediation
                )
            },
        );
        let location = if source == "unknown-module" {
            None
        } else {
            let path = PathBuf::from(source);
            Some(SourceLocation {
                path,
                start_line: None,
                start_column: None,
                end_line: None,
                end_column: None,
            })
        };
        let architecture_finding = ArchitectureFinding {
            finding: Finding {
                schema_version: CONTRACT_VERSION,
                id,
                rule: RuleIdentity {
                    tool_id: ARCHITECTURE_TOOL_ID.to_owned(),
                    rule_id: rule.id.clone(),
                },
                severity: match rule.severity {
                    ArchitectureSeverity::Info => Severity::Info,
                    ArchitectureSeverity::Warning => Severity::Warning,
                    ArchitectureSeverity::Error => Severity::Error,
                },
                message,
                location,
                ownership: RuleOwnership {
                    owner: "egohygiene/egolint".to_owned(),
                    policy_source: profile_path.display().to_string(),
                    configuration_path: Some(profile_path.to_path_buf()),
                },
                fingerprint: Some(fingerprint),
                evidence: Vec::new(),
                suppressed_by: None,
            },
            source_module: source.to_owned(),
            target_module: target,
            dependency_path,
            dependency_cruiser_version: adapter_version.to_owned(),
            remediation: rule.remediation.clone(),
        };
        architecture_finding.finding.validate()?;
        findings.push(architecture_finding);
    }
    Ok(findings)
}

fn apply_architecture_exceptions(
    findings: &mut [ArchitectureFinding],
    exceptions: &[ArchitectureException],
    evaluation_date: &str,
) -> Result<Vec<EvaluatedArchitectureException>> {
    let mut evaluated = Vec::new();
    for exception in exceptions {
        let expired = evaluation_date > exception.expires_on.as_str();
        let mut matched = false;
        if !expired {
            for finding in findings.iter_mut() {
                if finding.finding.rule.rule_id != exception.rule_id {
                    continue;
                }
                if exception
                    .from
                    .as_ref()
                    .is_some_and(|from| from != &finding.source_module)
                {
                    continue;
                }
                if exception
                    .to
                    .as_ref()
                    .is_some_and(|to| finding.target_module.as_ref() != Some(to))
                {
                    continue;
                }
                finding.finding.suppressed_by = Some(exception.id.clone());
                matched = true;
            }
        }
        evaluated.push(EvaluatedArchitectureException {
            exception: exception.clone(),
            state: if expired {
                ArchitectureExceptionState::Expired
            } else if matched {
                ArchitectureExceptionState::Applied
            } else {
                ArchitectureExceptionState::Unmatched
            },
        });
    }
    Ok(evaluated)
}

fn build_report(
    profile: &JavascriptArchitectureProfile,
    dependency_cruiser_version: String,
    findings: Vec<ArchitectureFinding>,
    exceptions: Vec<EvaluatedArchitectureException>,
) -> JavascriptArchitectureReport {
    let suppressed = findings
        .iter()
        .filter(|finding| finding.finding.suppressed_by.is_some())
        .count() as u64;
    let errors = findings
        .iter()
        .filter(|finding| {
            finding.finding.suppressed_by.is_none()
                && finding.finding.severity == Severity::Error
        })
        .count() as u64;
    let warnings = findings
        .iter()
        .filter(|finding| {
            finding.finding.suppressed_by.is_none()
                && finding.finding.severity == Severity::Warning
        })
        .count() as u64;
    let expired_exceptions = exceptions
        .iter()
        .filter(|exception| exception.state == ArchitectureExceptionState::Expired)
        .count() as u64;
    JavascriptArchitectureReport {
        schema_version: JAVASCRIPT_ARCHITECTURE_SCHEMA_VERSION,
        profile_id: profile.id.clone(),
        profile_version: profile.version.clone(),
        policy_source: profile.policy_source.clone(),
        dependency_cruiser_version,
        summary: ArchitectureSummary {
            findings: findings.len() as u64,
            errors,
            warnings,
            suppressed,
            expired_exceptions,
        },
        findings,
        exceptions,
    }
}

fn write_json(
    workspace: &Path,
    relative: &Path,
    report: &JavascriptArchitectureReport,
) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(report)?;
    bytes.push(b'\n');
    write_bytes(workspace, relative, &bytes)
}

fn write_sarif(
    workspace: &Path,
    relative: &Path,
    report: &JavascriptArchitectureReport,
) -> Result<()> {
    let mut rules = BTreeMap::new();
    let results = report
        .findings
        .iter()
        .map(|finding| {
            let rule_id = format!(
                "{}:{}",
                ARCHITECTURE_TOOL_ID, finding.finding.rule.rule_id
            );
            rules.entry(rule_id.clone()).or_insert_with(|| {
                json!({
                    "id": rule_id,
                    "name": finding.finding.rule.rule_id,
                    "shortDescription": {"text": finding.remediation},
                    "properties": {
                        "egolintToolId": ARCHITECTURE_TOOL_ID,
                        "egolintPolicySource": report.policy_source,
                        "dependencyCruiserVersion": report.dependency_cruiser_version,
                    }
                })
            });
            let level = match finding.finding.severity {
                Severity::Info => "note",
                Severity::Warning => "warning",
                Severity::Error | Severity::Critical => "error",
            };
            let mut result = json!({
                "ruleId": rule_id,
                "level": level,
                "message": {"text": finding.finding.message},
                "properties": {
                    "sourceModule": finding.source_module,
                    "targetModule": finding.target_module,
                    "dependencyPath": finding.dependency_path,
                    "remediation": finding.remediation,
                }
            });
            if let Some(location) = &finding.finding.location {
                result["locations"] = json!([{
                    "physicalLocation": {
                        "artifactLocation": {"uri": location.path.to_string_lossy()}
                    }
                }]);
            }
            if let Some(suppression) = &finding.finding.suppressed_by {
                result["suppressions"] = json!([{
                    "kind": "external",
                    "justification": format!("Egolint architecture exception {suppression}")
                }]);
            }
            result
        })
        .collect::<Vec<_>>();
    let document = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {"driver": {
                "name": "Egolint",
                "informationUri": "https://egolint.egohygiene.io",
                "semanticVersion": env!("CARGO_PKG_VERSION"),
                "rules": rules.into_values().collect::<Vec<_>>()
            }},
            "results": results,
            "properties": {
                "egolintProfile": report.profile_id,
                "egolintProfileVersion": report.profile_version,
                "dependencyCruiserVersion": report.dependency_cruiser_version,
            }
        }]
    });
    let mut bytes = serde_json::to_vec_pretty(&document)?;
    bytes.push(b'\n');
    write_bytes(workspace, relative, &bytes)
}

fn write_bytes(workspace: &Path, relative: &Path, bytes: &[u8]) -> Result<()> {
    let relative = validate_workspace_path(relative, "architecture output")?;
    if !relative.starts_with(".reports/egolint") {
        return Err(EgolintError::Configuration(format!(
            "architecture outputs must stay under .reports/egolint: {}",
            relative.display()
        )));
    }
    let path = workspace.join(&relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| EgolintError::Filesystem {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&path, bytes).map_err(|source| EgolintError::Filesystem { path, source })
}

fn validate_workspace_path(path: &Path, name: &str) -> Result<PathBuf> {
    let Some(text) = path.to_str() else {
        return Err(EgolintError::Configuration(format!(
            "{name} must contain UTF-8"
        )));
    };
    if text.is_empty()
        || path.is_absolute()
        || text.contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a normalized workspace-relative path: {}",
            path.display()
        )));
    }
    Ok(path.to_path_buf())
}

fn stable_hash(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

fn bounded_text(value: &str) -> String {
    const LIMIT: usize = 2_048;
    value
        .replace(['\r', '\n'], " ")
        .chars()
        .take(LIMIT)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str) -> ArchitectureRule {
        ArchitectureRule {
            id: id.to_owned(),
            severity: ArchitectureSeverity::Error,
            description: "Forbidden dependency".to_owned(),
            from: ArchitectureRestriction::default(),
            to: ArchitectureRestriction::default(),
            remediation: "Use the public boundary.".to_owned(),
        }
    }

    #[test]
    fn built_in_profile_is_valid_and_pinned() {
        let profile: JavascriptArchitectureProfile =
            serde_json::from_str(BUILTIN_PROFILE).expect("built-in profile parses");
        validate_profile(&profile).expect("built-in profile validates");
        assert_eq!(profile.adapter.version, DEPENDENCY_CRUISER_VERSION);
        assert!(profile.rules.iter().any(|rule| rule.id == "no-circular"));
        assert!(
            profile
                .rules
                .iter()
                .any(|rule| rule.id == "no-browser-to-node-core")
        );
    }

    #[test]
    fn generated_dependency_cruiser_config_is_deterministic() {
        let first = dependency_cruiser_config(&[rule("no-deep-import")]);
        let second = dependency_cruiser_config(&[rule("no-deep-import")]);
        assert_eq!(first, second);
        assert_eq!(first["forbidden"][0]["name"], "no-deep-import");
        assert_eq!(first["options"]["skipAnalysisNotInRules"], true);
    }

    #[test]
    fn adapter_version_output_is_normalized() {
        assert_eq!(
            normalize_dependency_cruiser_version("18.2.0").expect("plain version"),
            "18.2.0"
        );
        assert_eq!(
            normalize_dependency_cruiser_version("dependency-cruiser@18.2.0")
                .expect("package version"),
            "18.2.0"
        );
    }

    #[test]
    fn adapter_violations_normalize_into_shared_finding_contract() {
        let raw = json!({
            "modules": [],
            "summary": {
                "violations": [{
                    "type": "dependency",
                    "from": "apps/web/src/app.ts",
                    "to": "packages/ui/src/internal.ts",
                    "rule": {"severity": "error", "name": "no-deep-import"}
                }]
            }
        });
        let findings = normalize_violations(
            &raw.to_string(),
            &[rule("no-deep-import")],
            Path::new(BUILTIN_PROFILE_PATH),
            DEPENDENCY_CRUISER_VERSION,
        )
        .expect("normalize fixture");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule.tool_id, ARCHITECTURE_TOOL_ID);
        assert_eq!(findings[0].source_module, "apps/web/src/app.ts");
        assert_eq!(
            findings[0].target_module.as_deref(),
            Some("packages/ui/src/internal.ts")
        );
        findings[0]
            .finding
            .validate()
            .expect("shared finding contract");
    }

    #[test]
    fn overlay_cannot_replace_canonical_rule() {
        let profile: JavascriptArchitectureProfile =
            serde_json::from_str(BUILTIN_PROFILE).expect("profile");
        let overlay = JavascriptArchitectureOverlay {
            schema_version: 1,
            profile_id: profile.id.clone(),
            add_rules: vec![profile.rules[0].clone()],
            exceptions: Vec::new(),
        };
        let error = resolve_policy(&profile, &[overlay]).expect_err("replacement rejected");
        assert!(error.to_string().contains("may not replace canonical"));
    }

    #[test]
    fn owned_exception_suppresses_only_exact_selected_edge() {
        let raw = json!({
            "summary": {"violations": [{
                "from": "apps/web/a.ts",
                "to": "packages/ui/src/internal.ts",
                "rule": {"name": "no-deep-import"}
            }]}
        });
        let mut findings = normalize_violations(
            &raw.to_string(),
            &[rule("no-deep-import")],
            Path::new(BUILTIN_PROFILE_PATH),
            DEPENDENCY_CRUISER_VERSION,
        )
        .expect("normalize");
        let exception = ArchitectureException {
            id: "migration-001".to_owned(),
            rule_id: "no-deep-import".to_owned(),
            owner: "web-team".to_owned(),
            reason: "Bounded migration".to_owned(),
            expires_on: "2026-09-01".to_owned(),
            from: Some("apps/web/a.ts".to_owned()),
            to: Some("packages/ui/src/internal.ts".to_owned()),
        };
        let evaluated = apply_architecture_exceptions(
            &mut findings,
            &[exception],
            "2026-08-23",
        )
        .expect("evaluate exception");
        assert_eq!(evaluated[0].state, ArchitectureExceptionState::Applied);
        assert_eq!(
            findings[0].finding.suppressed_by.as_deref(),
            Some("migration-001")
        );
    }

    #[test]
    fn expired_exception_never_suppresses() {
        let raw = json!({
            "summary": {"violations": [{
                "from": "apps/web/a.ts",
                "to": "apps/admin/b.ts",
                "rule": {"name": "no-app-to-app"}
            }]}
        });
        let mut findings = normalize_violations(
            &raw.to_string(),
            &[rule("no-app-to-app")],
            Path::new(BUILTIN_PROFILE_PATH),
            DEPENDENCY_CRUISER_VERSION,
        )
        .expect("normalize");
        let exception = ArchitectureException {
            id: "old-migration".to_owned(),
            rule_id: "no-app-to-app".to_owned(),
            owner: "web-team".to_owned(),
            reason: "Migration ended".to_owned(),
            expires_on: "2026-08-22".to_owned(),
            from: None,
            to: None,
        };
        let evaluated = apply_architecture_exceptions(
            &mut findings,
            &[exception],
            "2026-08-23",
        )
        .expect("evaluate exception");
        assert_eq!(evaluated[0].state, ArchitectureExceptionState::Expired);
        assert!(findings[0].finding.suppressed_by.is_none());
    }
}
