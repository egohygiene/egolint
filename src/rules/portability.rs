//! Evidence-backed repository portability rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation, Suppression,
};
use crate::error::{EgolintError, Result};

use super::inventory::{RepositoryEntry, RepositoryEntryKind, RepositoryInventory};

const CATALOG_PATH: &str = ".config/rules/portability.toml";
const CATALOG_SOURCE: &str = include_str!("../../.config/rules/portability.toml");
const TOOL_ID: &str = "EGOLINT_PORTABILITY";

const CASE_COLLISION: &str = "EGO-PORT-CASE-001";
const WINDOWS_PATH: &str = "EGO-PORT-PATH-001";
const MIXED_EOL: &str = "EGO-PORT-EOL-001";
const AUTOMATION_EOL: &str = "EGO-PORT-EOL-002";
const EXECUTABLE_MODE: &str = "EGO-PORT-EXEC-001";
const HARDCODED_HOME: &str = "EGO-PORT-HOME-001";
const PLATFORM_COMMAND: &str = "EGO-PORT-CMD-001";
const WORKFLOW_SHELL: &str = "EGO-PORT-WORKFLOW-001";

const EXPECTED_RULE_IDS: [&str; 8] = [
    CASE_COLLISION,
    WINDOWS_PATH,
    MIXED_EOL,
    AUTOMATION_EOL,
    EXECUTABLE_MODE,
    HARDCODED_HOME,
    PLATFORM_COMMAND,
    WORKFLOW_SHELL,
];

/// One documented portability rule loaded from the bundled policy catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PortabilityRuleDefinition {
    /// Stable rule identifier.
    pub id: String,
    /// Human-readable rule name.
    pub title: String,
    /// Portability surface covered by the rule.
    pub category: String,
    /// Default normalized severity.
    pub severity: Severity,
    /// Whether a reviewed, expiring suppression may match the rule.
    pub suppressible: bool,
    /// Bounded explanation of the behavior being checked.
    pub description: String,
    /// Authoritative references supporting the rule.
    pub evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct PortabilityCatalog {
    schema_version: u32,
    owner: String,
    policy_source: String,
    rules: Vec<PortabilityRuleDefinition>,
}

/// Bundled portability rules and their deterministic evaluator.
#[derive(Debug)]
pub struct PortabilityRuleSet {
    owner: String,
    policy_source: String,
    definitions: BTreeMap<String, PortabilityRuleDefinition>,
}

impl PortabilityRuleSet {
    /// Load and validate the policy catalog embedded in the Egolint binary.
    ///
    /// # Errors
    ///
    /// Returns an error when the catalog is malformed, incomplete, or contains
    /// non-HTTPS external evidence references.
    pub fn bundled() -> Result<Self> {
        let catalog: PortabilityCatalog =
            toml::from_str(CATALOG_SOURCE).map_err(|source| EgolintError::Toml {
                path: PathBuf::from(CATALOG_PATH),
                source,
            })?;
        if catalog.schema_version != CONTRACT_VERSION {
            return Err(EgolintError::Configuration(format!(
                "portability catalog schema-version must equal {CONTRACT_VERSION}"
            )));
        }
        if catalog.owner.trim().is_empty() || catalog.policy_source.trim().is_empty() {
            return Err(EgolintError::Configuration(
                "portability catalog owner and policy-source are required".to_owned(),
            ));
        }
        let mut definitions = BTreeMap::new();
        for definition in catalog.rules {
            if definition.id.trim().is_empty()
                || definition.title.trim().is_empty()
                || definition.category.trim().is_empty()
                || definition.description.trim().is_empty()
                || definition.evidence.is_empty()
            {
                return Err(EgolintError::Configuration(
                    "every portability rule requires identity, text, and evidence".to_owned(),
                ));
            }
            if definition
                .evidence
                .iter()
                .any(|reference| !reference.starts_with("https://"))
            {
                return Err(EgolintError::Configuration(format!(
                    "portability rule {} contains a non-HTTPS evidence reference",
                    definition.id
                )));
            }
            let id = definition.id.clone();
            if definitions.insert(id.clone(), definition).is_some() {
                return Err(EgolintError::Configuration(format!(
                    "duplicate portability rule id {id}"
                )));
            }
        }
        let observed = definitions
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = EXPECTED_RULE_IDS.into_iter().collect::<BTreeSet<_>>();
        if observed != expected {
            return Err(EgolintError::Configuration(format!(
                "portability catalog rule set drifted: expected {expected:?}, observed {observed:?}"
            )));
        }
        Ok(Self {
            owner: catalog.owner,
            policy_source: catalog.policy_source,
            definitions,
        })
    }

    /// Return rule definitions in stable identifier order.
    #[must_use]
    pub fn definitions(&self) -> Vec<&PortabilityRuleDefinition> {
        self.definitions.values().collect()
    }

    /// Return whether a reviewed suppression may target this portability rule.
    #[must_use]
    pub fn is_suppressible(&self, rule_id: &str) -> bool {
        self.definitions
            .get(rule_id)
            .is_some_and(|definition| definition.suppressible)
    }

    /// Evaluate every bundled rule against a deterministic repository snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid inventory paths or malformed workflow YAML.
    /// Invalid workflow YAML remains owned by the YAML/action linters and is
    /// skipped here rather than duplicated as a portability finding.
    pub fn evaluate(&self, inventory: &RepositoryInventory) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        self.case_collisions(inventory, &mut findings)?;
        for entry in inventory.entries() {
            self.windows_path(entry, &mut findings)?;
            self.line_endings(entry, &mut findings)?;
            self.executable_mode(entry, &mut findings)?;
            self.hardcoded_homes(entry, &mut findings)?;
            self.platform_commands(entry, &mut findings)?;
            self.workflow_shells(entry, &mut findings)?;
        }
        findings.sort_by(|left, right| finding_sort_key(left).cmp(&finding_sort_key(right)));
        for finding in &findings {
            finding.validate()?;
        }
        Ok(findings)
    }

    /// Apply reviewed suppressions while enforcing each rule's catalog policy.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid, broad, ambiguous, or non-suppressible
    /// declarations as defined by the shared suppression engine and this rule
    /// catalog. Expired declarations remain visible as blocking findings.
    pub fn apply_suppressions(
        &self,
        findings: &mut Vec<Finding>,
        suppressions: &mut [Suppression],
        today: &str,
    ) -> Result<()> {
        super::suppressions::apply_suppressions(findings, suppressions, today, |rule| {
            rule.tool_id == TOOL_ID
                && self
                    .definitions
                    .get(&rule.rule_id)
                    .is_some_and(|definition| definition.suppressible)
        })
    }

    fn case_collisions(
        &self,
        inventory: &RepositoryInventory,
        findings: &mut Vec<Finding>,
    ) -> Result<()> {
        let mut folded_paths = BTreeMap::<String, &str>::new();
        for entry in inventory.entries() {
            let path = entry.normalized_path()?;
            let folded = path.to_ascii_lowercase();
            if let Some(previous) = folded_paths.insert(folded, path) {
                if previous != path {
                    findings.push(self.finding(
                        CASE_COLLISION,
                        path,
                        None,
                        None,
                        format!(
                            "repository paths {} and {} collide on a case-insensitive filesystem",
                            safe_text(previous),
                            safe_text(path)
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    fn windows_path(&self, entry: &RepositoryEntry, findings: &mut Vec<Finding>) -> Result<()> {
        let path = entry.normalized_path()?;
        for segment in path.split('/') {
            if let Some(reason) = windows_segment_problem(segment) {
                findings.push(self.finding(
                    WINDOWS_PATH,
                    path,
                    None,
                    None,
                    format!(
                        "path segment {} is not portable to Windows: {reason}",
                        safe_text(segment)
                    ),
                ));
            }
        }
        Ok(())
    }

    fn line_endings(&self, entry: &RepositoryEntry, findings: &mut Vec<Finding>) -> Result<()> {
        if entry.kind != RepositoryEntryKind::File || entry.content.contains(&0) {
            return Ok(());
        }
        let path = entry.normalized_path()?;
        let endings = LineEndings::inspect(&entry.content);
        if endings.lone_carriage_return > 0 || (endings.crlf > 0 && endings.lf > 0) {
            let offset = first_mixed_ending_offset(&entry.content);
            let (line, column) = byte_position(&entry.content, offset);
            findings.push(self.finding(
                MIXED_EOL,
                path,
                Some(line),
                Some(column),
                "text contains mixed LF/CRLF endings or a lone carriage return".to_owned(),
            ));
        } else if endings.crlf > 0 && requires_lf(path) {
            findings.push(self.finding(
                AUTOMATION_EOL,
                path,
                Some(1),
                Some(1),
                "portable automation file uses CRLF; declare and store LF endings".to_owned(),
            ));
        }
        Ok(())
    }

    fn executable_mode(&self, entry: &RepositoryEntry, findings: &mut Vec<Finding>) -> Result<()> {
        if entry.kind == RepositoryEntryKind::File
            && entry.content.starts_with(b"#!")
            && entry.git_mode.is_some()
            && entry.git_mode != Some(100_755)
        {
            let path = entry.normalized_path()?;
            findings.push(self.finding(
                EXECUTABLE_MODE,
                path,
                Some(1),
                Some(1),
                format!(
                    "shebang script is tracked with Git mode {}; expected 100755",
                    entry.git_mode.unwrap_or_default()
                ),
            ));
        }
        Ok(())
    }

    fn hardcoded_homes(&self, entry: &RepositoryEntry, findings: &mut Vec<Finding>) -> Result<()> {
        let path = entry.normalized_path()?;
        if entry.kind != RepositoryEntryKind::File || !is_automation_surface(path) {
            return Ok(());
        }
        let Ok(content) = std::str::from_utf8(&entry.content) else {
            return Ok(());
        };
        for (line_index, line) in content.lines().enumerate() {
            if line.trim_start().starts_with('#') || line.trim_start().starts_with("//") {
                continue;
            }
            if let Some((column, pattern)) = hardcoded_home_pattern(line) {
                findings.push(self.finding(
                    HARDCODED_HOME,
                    path,
                    Some(one_based(line_index)),
                    Some(one_based(column)),
                    format!(
                        "portable automation embeds host home-path form {pattern}; resolve user state from the environment or a platform API"
                    ),
                ));
            }
        }
        Ok(())
    }

    fn platform_commands(
        &self,
        entry: &RepositoryEntry,
        findings: &mut Vec<Finding>,
    ) -> Result<()> {
        let path = entry.normalized_path()?;
        if entry.kind != RepositoryEntryKind::File || !is_posix_shell(path, &entry.content) {
            return Ok(());
        }
        let Ok(content) = std::str::from_utf8(&entry.content) else {
            return Ok(());
        };
        for (line_index, line) in content.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            if let Some((column, form)) = platform_specific_command(line) {
                findings.push(self.finding(
                    PLATFORM_COMMAND,
                    path,
                    Some(one_based(line_index)),
                    Some(one_based(column)),
                    format!(
                        "command form {form} has GNU/BSD portability differences; branch explicitly, use a portable form, or document platform scope"
                    ),
                ));
            }
        }
        Ok(())
    }

    fn workflow_shells(&self, entry: &RepositoryEntry, findings: &mut Vec<Finding>) -> Result<()> {
        let path = entry.normalized_path()?;
        if entry.kind != RepositoryEntryKind::File || !is_workflow(path) {
            return Ok(());
        }
        let Ok(document) = serde_yaml::from_slice::<Value>(&entry.content) else {
            return Ok(());
        };
        let Some(root) = document.as_mapping() else {
            return Ok(());
        };
        let workflow_shell = explicit_default_shell(root);
        let Some(jobs) = mapping_value(root, "jobs").and_then(Value::as_mapping) else {
            return Ok(());
        };
        let run_lines = run_step_lines(&entry.content);
        let mut run_index = 0usize;
        for job in jobs.values().filter_map(Value::as_mapping) {
            let portable_matrix = job_runs_on_multiple_operating_systems(job);
            let job_shell = explicit_default_shell(job).or(workflow_shell);
            let Some(steps) = mapping_value(job, "steps").and_then(Value::as_sequence) else {
                continue;
            };
            for step in steps.iter().filter_map(Value::as_mapping) {
                if mapping_value(step, "run").is_none() {
                    continue;
                }
                let line = run_lines.get(run_index).copied();
                run_index += 1;
                if portable_matrix && job_shell.is_none() && mapping_value(step, "shell").is_none()
                {
                    findings.push(self.finding(
                        WORKFLOW_SHELL,
                        path,
                        line,
                        Some(1),
                        "multi-OS workflow run step relies on platform-dependent default shell semantics"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn finding(
        &self,
        rule_id: &str,
        path: &str,
        line: Option<u32>,
        column: Option<u32>,
        message: String,
    ) -> Finding {
        let definition = self
            .definitions
            .get(rule_id)
            .expect("bundled rule identifier must be validated");
        let fingerprint = stable_fingerprint(rule_id, path, line, column, &message);
        Finding {
            schema_version: CONTRACT_VERSION,
            id: format!("{rule_id}-{fingerprint}"),
            rule: RuleIdentity {
                tool_id: TOOL_ID.to_owned(),
                rule_id: rule_id.to_owned(),
            },
            severity: definition.severity,
            message,
            location: Some(SourceLocation {
                path: PathBuf::from(path),
                start_line: line,
                start_column: column,
                end_line: line,
                end_column: None,
            }),
            ownership: RuleOwnership {
                owner: self.owner.clone(),
                policy_source: self.policy_source.clone(),
                configuration_path: Some(PathBuf::from(CATALOG_PATH)),
            },
            fingerprint: Some(fingerprint),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Policy,
                path: PathBuf::from(CATALOG_PATH),
                sha256: None,
                description: Some(format!(
                    "{} authoritative reference(s) are recorded in the bundled rule catalog.",
                    definition.evidence.len()
                )),
            }],
            suppressed_by: None,
        }
    }
}

fn windows_segment_problem(segment: &str) -> Option<&'static str> {
    if segment.ends_with(' ') || segment.ends_with('.') {
        return Some("segments may not end in a space or period");
    }
    if segment
        .chars()
        .any(|character| character.is_control() || r#"<>:"\|?*"#.contains(character))
    {
        return Some("segment contains a Windows-reserved character");
    }
    let basename = segment
        .split('.')
        .next()
        .unwrap_or(segment)
        .to_ascii_uppercase();
    if matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_numbered_device(&basename, "COM")
        || reserved_numbered_device(&basename, "LPT")
    {
        return Some("segment uses a reserved Windows device name");
    }
    None
}

fn reserved_numbered_device(name: &str, prefix: &str) -> bool {
    let Some(number) = name.strip_prefix(prefix) else {
        return false;
    };
    matches!(
        number,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
    )
}

#[derive(Debug, Default)]
struct LineEndings {
    crlf: usize,
    lf: usize,
    lone_carriage_return: usize,
}

impl LineEndings {
    fn inspect(content: &[u8]) -> Self {
        let mut result = Self::default();
        let mut index = 0;
        while index < content.len() {
            match content[index] {
                b'\r' if content.get(index + 1) == Some(&b'\n') => {
                    result.crlf += 1;
                    index += 2;
                }
                b'\r' => {
                    result.lone_carriage_return += 1;
                    index += 1;
                }
                b'\n' => {
                    result.lf += 1;
                    index += 1;
                }
                _ => index += 1,
            }
        }
        result
    }
}

fn first_mixed_ending_offset(content: &[u8]) -> usize {
    content
        .iter()
        .position(|byte| *byte == b'\r')
        .unwrap_or_default()
}

fn requires_lf(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "sh" | "bash" | "dash" | "ksh" | "zsh" | "fish" | "ps1"
    ) || is_workflow(path)
        || (path.starts_with(".devcontainer/")
            && matches!(extension.as_str(), "json" | "jsonc" | "sh"))
}

fn is_automation_surface(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "sh" | "bash"
            | "dash"
            | "ksh"
            | "zsh"
            | "fish"
            | "ps1"
            | "cmd"
            | "bat"
            | "yml"
            | "yaml"
            | "json"
            | "jsonc"
            | "toml"
    ) && (path.starts_with("scripts/")
        || path.starts_with(".github/")
        || path.starts_with(".devcontainer/")
        || path.starts_with("tasks/")
        || Path::new(path)
            .file_name()
            .is_some_and(|name| name == "Taskfile.yml" || name == "Taskfile.yaml"))
}

fn is_posix_shell(path: &str, content: &[u8]) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(extension.as_str(), "sh" | "bash" | "dash" | "ksh" | "zsh")
        || content
            .strip_prefix(b"#!")
            .and_then(|line| line.split(|byte| *byte == b'\n').next())
            .is_some_and(|line| line.windows(2).any(|window| window == b"sh"))
}

fn hardcoded_home_pattern(line: &str) -> Option<(usize, &'static str)> {
    const PATTERNS: [&str; 4] = ["/users/", "/home/", "c:\\users\\", "c:/users/"];

    let lowercase = line.to_ascii_lowercase();
    PATTERNS
        .iter()
        .filter_map(|pattern| {
            let column = lowercase.find(pattern)?;
            let remainder = &line[column + pattern.len()..];
            let segment = remainder.split(['/', '\\']).next().unwrap_or_default();
            let first = segment.chars().next();
            let literal = !segment.is_empty()
                && !first.is_some_and(|character| matches!(character, '$' | '%' | '{' | '<' | '~'))
                && segment
                    .chars()
                    .all(|character| character.is_alphanumeric() || "._-".contains(character));
            literal.then_some((column, *pattern))
        })
        .min_by_key(|(column, _)| *column)
}

fn platform_specific_command(line: &str) -> Option<(usize, &'static str)> {
    const FORMS: [(&str, &str); 10] = [
        ("sed -i", "sed -i"),
        ("readlink -f", "readlink -f"),
        ("realpath --relative-to", "realpath --relative-to"),
        ("date -d", "date -d"),
        ("date --date", "date --date"),
        ("stat -c", "stat -c"),
        ("grep -P", "grep -P"),
        ("xargs -r", "xargs -r"),
        ("mktemp --directory", "mktemp --directory"),
        ("sha256sum", "sha256sum"),
    ];
    FORMS
        .iter()
        .filter_map(|(needle, display)| line.find(needle).map(|column| (column, *display)))
        .min_by_key(|(column, _)| *column)
}

fn is_workflow(path: &str) -> bool {
    path.starts_with(".github/workflows/")
        && Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("yml") || extension.eq_ignore_ascii_case("yaml")
            })
}

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

fn explicit_default_shell(mapping: &Mapping) -> Option<&str> {
    mapping_value(mapping, "defaults")
        .and_then(Value::as_mapping)
        .and_then(|defaults| mapping_value(defaults, "run"))
        .and_then(Value::as_mapping)
        .and_then(|run| mapping_value(run, "shell"))
        .and_then(Value::as_str)
}

fn job_runs_on_multiple_operating_systems(job: &Mapping) -> bool {
    let Some(runs_on) = mapping_value(job, "runs-on").and_then(Value::as_str) else {
        return false;
    };
    let Some(matrix) = mapping_value(job, "strategy")
        .and_then(Value::as_mapping)
        .and_then(|strategy| mapping_value(strategy, "matrix"))
        .and_then(Value::as_mapping)
    else {
        return false;
    };
    for (key, values) in matrix {
        let Some(key) = key.as_str() else {
            continue;
        };
        let matrix_selector = format!("matrix.{key}");
        if !runs_on.contains(matrix_selector.as_str()) {
            continue;
        }
        let families = values
            .as_sequence()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(operating_system_family)
            .collect::<BTreeSet<_>>();
        if families.len() > 1 {
            return true;
        }
    }
    false
}

fn operating_system_family(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    if value.contains("windows") {
        Some("windows")
    } else if value.contains("macos") || value.contains("osx") {
        Some("macos")
    } else if value.contains("ubuntu") || value.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

fn run_step_lines(content: &[u8]) -> Vec<u32> {
    String::from_utf8_lossy(content)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim_start().trim_start_matches("- ");
            line.starts_with("run:").then(|| one_based(index))
        })
        .collect()
}

fn byte_position(content: &[u8], offset: usize) -> (u32, u32) {
    let prefix = &content[..offset.min(content.len())];
    let line = prefix.split(|byte| *byte == b'\n').count();
    let column = prefix
        .rsplit(|byte| *byte == b'\n')
        .next()
        .map_or(1, |segment| segment.len().saturating_add(1));
    (usize_to_u32(line), usize_to_u32(column))
}

fn one_based(zero_based: usize) -> u32 {
    usize_to_u32(zero_based.saturating_add(1))
}

fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn stable_fingerprint(
    rule_id: &str,
    path: &str,
    line: Option<u32>,
    column: Option<u32>,
    message: &str,
) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in rule_id
        .bytes()
        .chain([0])
        .chain(path.bytes())
        .chain([0])
        .chain(line.unwrap_or_default().to_le_bytes())
        .chain(column.unwrap_or_default().to_le_bytes())
        .chain(message.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("portability-v1-{hash:016x}")
}

fn safe_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

fn finding_sort_key(finding: &Finding) -> (&str, &Path, u32, u32, &str) {
    let location = finding.location.as_ref();
    (
        finding.rule.rule_id.as_str(),
        location.map_or_else(|| Path::new(""), |location| location.path.as_path()),
        location
            .and_then(|location| location.start_line)
            .unwrap_or_default(),
        location
            .and_then(|location| location.start_column)
            .unwrap_or_default(),
        finding.message.as_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::inventory::RepositoryEntry;

    fn evaluate(entries: Vec<RepositoryEntry>) -> Vec<Finding> {
        PortabilityRuleSet::bundled()
            .expect("valid bundled catalog")
            .evaluate(&RepositoryInventory::from_entries(entries).expect("valid inventory"))
            .expect("successful portability evaluation")
    }

    fn rule_ids(findings: &[Finding]) -> Vec<&str> {
        findings
            .iter()
            .map(|finding| finding.rule.rule_id.as_str())
            .collect()
    }

    #[test]
    fn bundled_catalog_is_complete_and_evidence_backed() {
        let rules = PortabilityRuleSet::bundled().expect("valid bundled catalog");

        assert_eq!(rules.definitions().len(), EXPECTED_RULE_IDS.len());
        assert!(
            rules
                .definitions()
                .iter()
                .all(|rule| !rule.evidence.is_empty())
        );
    }

    #[test]
    fn portable_repository_has_no_findings() {
        let findings = evaluate(vec![
            RepositoryEntry::file("README.md", Some(100_644), b"# Example\n".to_vec()),
            RepositoryEntry::file(
                "scripts/check.sh",
                Some(100_755),
                b"#!/usr/bin/env bash\nprintf '%s\\n' \"ok\"\n".to_vec(),
            ),
            RepositoryEntry::file(
                ".github/workflows/ci.yml",
                Some(100_644),
                b"jobs:\n  test:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: cargo test\n"
                    .to_vec(),
            ),
        ]);

        assert!(findings.is_empty(), "unexpected findings: {findings:#?}");
    }

    #[test]
    fn path_rules_detect_case_collisions_and_windows_names() {
        let findings = evaluate(vec![
            RepositoryEntry::file("Docs/Guide.md", Some(100_644), Vec::new()),
            RepositoryEntry::file("docs/guide.md", Some(100_644), Vec::new()),
            RepositoryEntry::file("assets/NUL.txt", Some(100_644), Vec::new()),
            RepositoryEntry::file("assets/trailing. ", Some(100_644), Vec::new()),
        ]);

        let ids = rule_ids(&findings);
        assert!(ids.contains(&CASE_COLLISION));
        assert_eq!(ids.iter().filter(|id| **id == WINDOWS_PATH).count(), 2);
    }

    #[test]
    fn line_endings_and_git_index_mode_are_checked_without_host_mode() {
        let findings = evaluate(vec![
            RepositoryEntry::file(
                "scripts/mixed.sh",
                Some(100_755),
                b"#!/bin/sh\r\nprintf ok\n".to_vec(),
            ),
            RepositoryEntry::file(
                "scripts/crlf.sh",
                Some(100_755),
                b"#!/bin/sh\r\nprintf ok\r\n".to_vec(),
            ),
            RepositoryEntry::file(
                "scripts/not-executable.sh",
                Some(100_644),
                b"#!/bin/sh\nprintf ok\n".to_vec(),
            ),
            RepositoryEntry::file(
                "scripts/windows.cmd",
                Some(100_644),
                b"echo ok\r\n".to_vec(),
            ),
        ]);

        let ids = rule_ids(&findings);
        assert!(ids.contains(&MIXED_EOL));
        assert!(ids.contains(&AUTOMATION_EOL));
        assert!(ids.contains(&EXECUTABLE_MODE));
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn automation_assumptions_are_advisory_and_located() {
        let findings = evaluate(vec![RepositoryEntry::file(
            "scripts/install.sh",
            Some(100_755),
            b"#!/usr/bin/env bash\ncp /Users/alice/tool .\nreadlink -f ./tool\n".to_vec(),
        )]);

        assert_eq!(rule_ids(&findings), vec![PLATFORM_COMMAND, HARDCODED_HOME]);
        assert!(
            findings
                .iter()
                .all(|finding| finding.severity == Severity::Warning)
        );
        assert!(findings.iter().all(|finding| {
            finding
                .location
                .as_ref()
                .and_then(|location| location.start_line)
                .is_some()
        }));
    }

    #[test]
    fn environment_resolved_home_paths_are_not_literals() {
        let findings = evaluate(vec![RepositoryEntry::file(
            "scripts/install.sh",
            Some(100_755),
            b"#!/usr/bin/env bash\nprintf '%s\\n' /home/$USER/tool /Users/${USER}/tool 'C:\\Users\\%USERNAME%\\tool'\n"
                .to_vec(),
        )]);

        assert!(findings.is_empty(), "unexpected findings: {findings:#?}");
    }

    #[test]
    fn multi_os_workflow_requires_an_explicit_shell() {
        let workflow = br"jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-24.04, windows-2025, macos-15]
    runs-on: ${{ matrix.os }}
    steps:
      - run: cargo test
      - shell: bash
        run: cargo check
";
        let findings = evaluate(vec![RepositoryEntry::file(
            ".github/workflows/portable.yml",
            Some(100_644),
            workflow.to_vec(),
        )]);

        assert_eq!(rule_ids(&findings), vec![WORKFLOW_SHELL]);
    }

    #[test]
    fn finding_order_and_fingerprints_are_deterministic() {
        let entries = vec![RepositoryEntry::file(
            "scripts/install.sh",
            Some(100_644),
            b"#!/bin/sh\nsed -i 's/a/b/' /home/alice/file\n".to_vec(),
        )];
        let first = evaluate(entries.clone());
        let second = evaluate(entries);

        assert_eq!(first, second);
        assert!(first.iter().all(|finding| finding.validate().is_ok()));
    }
}
