//! Optional pinned ast-grep host adapter. Only embedded rules execute, without rewriting source.

use std::io::Read;
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
        extensions: &["ts", "tsx"],
        yaml: include_str!("../.config/structural/typescript-no-eval.yml"),
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
    let Ok((code, version)) = capture(version_command, deadline) else {
        return Ok(failed(ToolStatus::MissingFromImage));
    };
    if code != Some(0)
        || std::str::from_utf8(&version).ok().map(str::trim) != Some(&format!("ast-grep {VERSION}"))
    {
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
            let Ok((status, bytes)) = capture(command, deadline) else {
                return Ok(failed(if Instant::now() >= deadline {
                    ToolStatus::TimedOut
                } else {
                    ToolStatus::ExecutionError
                }));
            };
            if !matches!(status, Some(0 | 1)) {
                return Ok(failed(ToolStatus::ExecutionError));
            }
            let Ok(normalized) = normalize(workspace, rule, &bytes) else {
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

fn normalize(workspace: &Path, rule: &Rule, bytes: &[u8]) -> Result<Vec<Finding>> {
    let matches: Vec<RawMatch> = serde_json::from_slice(bytes)?;
    let mut findings = Vec::new();
    for item in matches {
        if item.rule_id != rule.id {
            return Err(invalid("unexpected structural rule id"));
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
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| invalid("structural executable unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("structural stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| invalid("structural stderr unavailable"))?;
    let out = std::thread::spawn(move || bounded_read(stdout));
    let err = std::thread::spawn(move || bounded_read(stderr));
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
    let bytes = out
        .join()
        .map_err(|_| invalid("structural output reader failed"))??;
    let _ = err
        .join()
        .map_err(|_| invalid("structural error reader failed"))??;
    if timed_out {
        return Err(invalid("structural execution timed out"));
    }
    Ok((status.code(), bytes))
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
