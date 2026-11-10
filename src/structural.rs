//! Optional pinned ast-grep host adapter. Only embedded rules execute, without rewriting source.

use std::collections::BTreeSet;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::capabilities::{CapabilityConfig, CapabilityContext};
use crate::contracts::{
    Enforcement, Finding, RuleIdentity, RuleOwnership, Severity, SourceLocation, ToolResult,
    ToolStatus,
};
use crate::error::{EgolintError, Result};

pub const TOOL_ID: &str = "AST_GREP";
pub const VERSION: &str = "0.51.1";
const MAX_OUTPUT: usize = 2 * 1024 * 1024;

struct Rule {
    id: &'static str,
    extensions: &'static [&'static str],
    yaml: &'static str,
    message: &'static str,
}
const RULES: &[Rule] = &[
    Rule {
        id: "EGO-AST-JS-EVAL-001",
        extensions: &["js", "mjs", "cjs", "jsx"],
        yaml: include_str!("../.config/structural/javascript-no-eval.yml"),
        message: "Review direct eval; prefer an explicit parser or dispatcher.",
    },
    Rule {
        id: "EGO-AST-TS-EVAL-001",
        extensions: &["ts"],
        yaml: include_str!("../.config/structural/typescript-no-eval.yml"),
        message: "Review direct eval; prefer an explicit parser or dispatcher.",
    },
    Rule {
        id: "EGO-AST-TS-EVAL-001",
        extensions: &["tsx"],
        yaml: include_str!("../.config/structural/tsx-no-eval.yml"),
        message: "Review direct eval; prefer an explicit parser or dispatcher.",
    },
    Rule {
        id: "EGO-AST-PY-SHELL-001",
        extensions: &["py"],
        yaml: include_str!("../.config/structural/python-no-shell-true.yml"),
        message: "Review subprocess.run with shell=True; prefer an explicit argument vector.",
    },
    Rule {
        id: "EGO-AST-RS-EXIT-001",
        extensions: &["rs"],
        yaml: include_str!("../.config/structural/rust-no-process-exit.yml"),
        message: "Review direct process exit; prefer returning a typed result from reusable code.",
    },
];

pub struct StructuralResult {
    pub tool: ToolResult,
    pub findings: Vec<Finding>,
}

/// Run the exact installed version against inventoried regular sources with embedded rules.
/// Repository sgconfig, custom parsers, scripts and fix directives are never loaded.
/// # Errors
/// Returns an error when inventory discovery or temporary storage is unavailable.
pub fn run(workspace: &Path) -> Result<StructuralResult> {
    let context = CapabilityContext::discover(workspace, &CapabilityConfig::default())?;
    if !context.paths.iter().any(|path| {
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|extension| {
                RULES
                    .iter()
                    .any(|rule| rule.extensions.contains(&extension))
            })
    }) {
        return Ok(StructuralResult {
            tool: tool_result(ToolStatus::NotApplicable),
            findings: Vec::new(),
        });
    }
    let started = Instant::now();
    let deadline = started + Duration::from_secs(30);
    let temporary = tempfile::tempdir().map_err(|source| EgolintError::Filesystem {
        path: PathBuf::from("<structural temporary directory>"),
        source,
    })?;
    let mut version_command = Command::new("ast-grep");
    version_command
        .arg("--version")
        .current_dir(temporary.path());
    let (code, version) = match capture(version_command, deadline) {
        Ok(output) => output,
        Err(error) => return Ok(failed(capture_failure(&error, deadline))),
    };
    if code != Some(0) {
        return Ok(failed(ToolStatus::ExecutionError));
    }
    if std::str::from_utf8(&version).ok().map(str::trim) != Some(&format!("ast-grep {VERSION}")) {
        return Ok(failed(ToolStatus::ConfigurationError));
    }
    let mut findings = Vec::new();
    let mut scanned = 0;
    for rule in RULES {
        let files = context
            .paths
            .iter()
            .filter(|p| {
                Path::new(p)
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| rule.extensions.contains(&e))
            })
            .map(|p| workspace.join(p))
            .collect::<Vec<_>>();
        scanned += files.len();
        if scanned > 20_000 {
            return Ok(failed(ToolStatus::ConfigurationError));
        }
        for chunk in files.chunks(100) {
            if !valid_chunk(chunk) {
                return Ok(failed(ToolStatus::ConfigurationError));
            }
            let mut command = Command::new("ast-grep");
            command
                .args(["scan", "--json=compact", "--inline-rules", rule.yaml, "--"])
                .args(chunk)
                .current_dir(temporary.path());
            let (status, bytes) = match capture(command, deadline) {
                Ok(output) => output,
                Err(error) => return Ok(failed(capture_failure(&error, deadline))),
            };
            if !matches!(status, Some(0 | 1)) {
                return Ok(failed(ToolStatus::ExecutionError));
            }
            let Ok(normalized) = normalize(workspace, rule, chunk, &bytes) else {
                return Ok(failed(ToolStatus::ExecutionError));
            };
            findings.extend(normalized);
            if findings.len() > 50_000 {
                return Ok(failed(ToolStatus::ExecutionError));
            }
        }
    }
    findings.sort_by(|a, b| a.id.cmp(&b.id));
    let mut tool = tool_result(if scanned == 0 {
        ToolStatus::NotApplicable
    } else if findings.is_empty() {
        ToolStatus::Passed
    } else {
        ToolStatus::PassedWithWarnings
    });
    tool.warning_count = findings.len() as u64;
    tool.duration_ms = Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
    Ok(StructuralResult { tool, findings })
}

#[derive(Deserialize)]
struct RawMatch {
    #[serde(rename = "ruleId")]
    rule_id: String,
    file: PathBuf,
    range: RawRange,
}
#[derive(Deserialize)]
struct RawRange {
    start: Position,
    end: Position,
}
#[derive(Deserialize)]
struct Position {
    line: u32,
    column: u32,
}

fn normalize(
    workspace: &Path,
    rule: &Rule,
    inputs: &[PathBuf],
    bytes: &[u8],
) -> Result<Vec<Finding>> {
    let matches: Vec<RawMatch> = serde_json::from_slice(bytes)?;
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for item in matches {
        if item.rule_id != rule.id {
            return Err(invalid("unexpected structural rule id"));
        }
        if !inputs.contains(&item.file) {
            return Err(invalid(
                "structural result does not identify a scanned input",
            ));
        }
        let path = item
            .file
            .strip_prefix(workspace)
            .map_err(|_| invalid("structural result escapes workspace"))?
            .to_path_buf();
        let identity = serde_json::to_vec(&(
            &item.rule_id,
            &path,
            item.range.start.line,
            item.range.start.column,
        ))?;
        let fingerprint = format!("{:x}", Sha256::digest(identity));
        if !seen.insert(fingerprint.clone()) {
            return Err(invalid("duplicate structural diagnostic"));
        }
        let finding = Finding {
            schema_version: 1,
            id: format!("{}-{fingerprint}", rule.id),
            rule: RuleIdentity {
                tool_id: TOOL_ID.to_owned(),
                rule_id: rule.id.to_owned(),
            },
            severity: Severity::Warning,
            message: rule.message.to_owned(),
            location: Some(SourceLocation {
                path,
                start_line: item.range.start.line.checked_add(1),
                start_column: item.range.start.column.checked_add(1),
                end_line: item.range.end.line.checked_add(1),
                end_column: item.range.end.column.checked_add(1),
            }),
            ownership: RuleOwnership {
                owner: "egohygiene/egolint".to_owned(),
                policy_source: "egolint/structural/v1".to_owned(),
                configuration_path: None,
            },
            fingerprint: Some(fingerprint),
            evidence: Vec::new(),
            suppressed_by: None,
        };
        if finding.location.as_ref().is_some_and(|l| {
            l.start_line.is_none()
                || l.start_column.is_none()
                || l.end_line.is_none()
                || l.end_column.is_none()
        }) {
            return Err(invalid("structural span overflow"));
        }
        finding.validate()?;
        findings.push(finding);
    }
    Ok(findings)
}

fn valid_chunk(paths: &[PathBuf]) -> bool {
    paths.iter().all(|path| {
        path.canonicalize().ok().as_ref() == Some(path)
            && path
                .symlink_metadata()
                .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
    })
}

fn capture(mut command: Command, deadline: Instant) -> Result<(Option<i32>, Vec<u8>)> {
    if Instant::now() >= deadline {
        return Err(invalid("structural execution timed out"));
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                EgolintError::RuntimeUnavailable("structural executable unavailable".to_owned())
            } else {
                EgolintError::RuntimeExecution("cannot start structural executable".to_owned())
            }
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("structural stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| invalid("structural stderr unavailable"))?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(2);
    let output_sender = sender.clone();
    std::thread::spawn(move || {
        let _ = output_sender.send((true, bounded_read(stdout)));
    });
    std::thread::spawn(move || {
        let _ = sender.send((false, bounded_read(stderr)));
    });
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| invalid("cannot inspect structural process"))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child
                .wait()
                .map_err(|_| invalid("cannot stop structural process"))?;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if timed_out {
        return Err(invalid("structural execution timed out"));
    }
    let mut bytes = Vec::new();
    for _ in 0..2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let (is_stdout, output) = receiver
            .recv_timeout(remaining)
            .map_err(|_| invalid("structural output unavailable before deadline"))?;
        if is_stdout {
            bytes = output?;
        } else {
            output?;
        }
    }
    Ok((status.code(), bytes))
}

fn capture_failure(error: &EgolintError, deadline: Instant) -> ToolStatus {
    if Instant::now() >= deadline {
        ToolStatus::TimedOut
    } else if matches!(error, EgolintError::RuntimeUnavailable(_)) {
        ToolStatus::MissingFromImage
    } else {
        ToolStatus::ExecutionError
    }
}

fn bounded_read(mut reader: impl Read) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut chunk = [0_u8; 8192];
    let mut overflow = false;
    loop {
        let count = reader
            .read(&mut chunk)
            .map_err(|_| invalid("cannot read structural output"))?;
        if count == 0 {
            break;
        }
        let retained = count.min(MAX_OUTPUT.saturating_sub(output.len()));
        output.extend_from_slice(&chunk[..retained]);
        overflow |= retained < count;
    }
    if overflow {
        return Err(invalid("structural output exceeded limit"));
    }
    Ok(output)
}
fn tool_result(status: ToolStatus) -> ToolResult {
    ToolResult {
        schema_version: 1,
        tool_id: TOOL_ID.to_owned(),
        owner: "egohygiene/egolint".to_owned(),
        policy_source: "egolint/structural/v1".to_owned(),
        status,
        enforcement: Enforcement::Advisory,
        finding_count: 0,
        warning_count: 0,
        duration_ms: None,
        evidence: Vec::new(),
    }
}
fn failed(status: ToolStatus) -> StructuralResult {
    StructuralResult {
        tool: tool_result(status),
        findings: Vec::new(),
    }
}
fn invalid(message: &str) -> EgolintError {
    EgolintError::Configuration(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raw_match(path: &Path) -> serde_json::Value {
        json!({
            "ruleId": RULES[0].id,
            "file": path,
            "range": {"start": {"line": 1, "column": 2}, "end": {"line": 1, "column": 8}},
            "text": "private matched text",
            "message": "untrusted provider message"
        })
    }

    #[test]
    fn normalization_binds_findings_to_the_current_scanned_inputs() {
        let workspace = tempfile::tempdir().unwrap();
        let selected = workspace.path().join("selected.js");
        let other = workspace.path().join("other.js");
        let inputs = std::slice::from_ref(&selected);
        let normalize_match = |item| {
            normalize(
                workspace.path(),
                &RULES[0],
                inputs,
                &serde_json::to_vec(&vec![item]).unwrap(),
            )
        };
        let result = normalize_match(raw_match(&selected)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].location.as_ref().unwrap().path,
            Path::new("selected.js")
        );
        assert_eq!(result[0].location.as_ref().unwrap().start_line, Some(2));
        assert_eq!(result[0].message, RULES[0].message);
        let encoded = serde_json::to_string(&result).unwrap();
        assert!(!encoded.contains("private matched text"));
        assert!(!encoded.contains("untrusted provider message"));
        assert!(normalize_match(raw_match(&other)).is_err());
        assert!(normalize_match(raw_match(Path::new("selected.js"))).is_err());
        let mut wrong_rule = raw_match(&selected);
        wrong_rule["ruleId"] = json!("UNKNOWN-RULE");
        assert!(normalize_match(wrong_rule).is_err());
    }

    #[test]
    fn normalization_rejects_duplicate_and_invalid_spans() {
        let workspace = tempfile::tempdir().unwrap();
        let selected = workspace.path().join("selected.js");
        let item = raw_match(&selected);
        let bytes = serde_json::to_vec(&vec![item.clone(), item.clone()]).unwrap();
        assert!(
            normalize(
                workspace.path(),
                &RULES[0],
                std::slice::from_ref(&selected),
                &bytes,
            )
            .is_err()
        );
        for span in [
            json!({"start": {"line": u32::MAX, "column": 0}, "end": {"line": u32::MAX, "column": 1}}),
            json!({"start": {"line": 2, "column": 0}, "end": {"line": 1, "column": 1}}),
        ] {
            let mut item = item.clone();
            item["range"] = span;
            let bytes = serde_json::to_vec(&vec![item]).unwrap();
            assert!(
                normalize(
                    workspace.path(),
                    &RULES[0],
                    std::slice::from_ref(&selected),
                    &bytes,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn bounded_output_rejects_excess_without_retaining_it() {
        let at_limit = vec![b'x'; MAX_OUTPUT];
        assert_eq!(bounded_read(at_limit.as_slice()).unwrap().len(), MAX_OUTPUT);
        let oversized = vec![b'x'; MAX_OUTPUT + 1];
        assert!(bounded_read(oversized.as_slice()).is_err());
    }

    #[test]
    fn missing_executable_retains_its_failure_category() {
        let root = tempfile::tempdir().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let error =
            capture(Command::new(root.path().join("absent-ast-grep")), deadline).unwrap_err();
        assert_eq!(
            capture_failure(&error, deadline),
            ToolStatus::MissingFromImage
        );
    }

    #[cfg(unix)]
    #[test]
    fn child_timeout_and_inherited_output_handles_obey_the_deadline() {
        for script in ["sleep 1", "sleep 1 &"] {
            let started = Instant::now();
            let deadline = started + Duration::from_millis(50);
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script]);
            let error = capture(command, deadline).unwrap_err();
            assert_eq!(capture_failure(&error, deadline), ToolStatus::TimedOut);
            assert!(started.elapsed() < Duration::from_millis(750));
        }
    }
}
