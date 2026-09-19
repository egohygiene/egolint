//! Isolated Git execution with bounded files instead of potentially deadlocking pipes.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{GitignoreWinningRule, MAX_PROBES, safe_path};

type Checked<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub(super) struct Match {
    pub ignored: bool,
    pub rule: Option<GitignoreWinningRule>,
}

pub(super) struct Git {
    executable: PathBuf,
    temporary: tempfile::TempDir,
    empty: PathBuf,
}

impl Git {
    pub fn new(executable: &Path) -> Checked<Self> {
        let temporary = tempfile::tempdir()
            .map_err(|_| "cannot allocate isolated Git evidence workspace".to_owned())?;
        let empty = temporary.path().join("empty-config");
        std::fs::write(&empty, b"")
            .map_err(|_| "cannot create isolated Git configuration".to_owned())?;
        Ok(Self {
            executable: executable.to_path_buf(),
            temporary,
            empty,
        })
    }

    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(&self.executable);
        // Keep platform process requirements (notably SystemRoot on Windows),
        // but remove every inherited Git override, tracing, helper, and index path.
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                command.env_remove(key);
            }
        }
        command
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &self.empty)
            .env("GIT_CONFIG_SYSTEM", &self.empty)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("LC_ALL", "C")
            .args([
                "--no-optional-locks",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.ignoreCase=false",
                "-c",
            ])
            .arg(format!("core.excludesFile={}", self.empty.display()))
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    fn run(
        &self,
        root: &Path,
        args: &[&str],
        input: Option<&[u8]>,
        accepted: &[i32],
    ) -> Checked<Vec<u8>> {
        let mut stdin = tempfile::tempfile().map_err(|_| "cannot allocate Git input".to_owned())?;
        if let Some(input) = input {
            stdin
                .write_all(input)
                .map_err(|_| "cannot write bounded Git input".to_owned())?;
        }
        stdin
            .rewind()
            .map_err(|_| "cannot rewind Git input".to_owned())?;
        let mut stdout =
            tempfile::tempfile().map_err(|_| "cannot allocate Git evidence".to_owned())?;
        let output_handle = stdout
            .try_clone()
            .map_err(|_| "cannot open Git evidence handle".to_owned())?;
        let mut child = self
            .command(root)
            .args(args)
            .stdin(stdin)
            .stdout(output_handle)
            .spawn()
            .map_err(|_| {
                "Git is unavailable; install Git and rerun native validation".to_owned()
            })?;
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None)
                    if started.elapsed() < Duration::from_secs(30)
                        && stdout
                            .metadata()
                            .is_ok_and(|metadata| metadata.len() <= 32 * 1024 * 1024) =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(
                        "Git evidence exceeded its execution budget or could not be inspected"
                            .to_owned(),
                    );
                }
            }
        };
        if !status.code().is_some_and(|code| accepted.contains(&code)) {
            return Err("Git evidence is unavailable or unsupported; inspect repository topology and policy syntax".to_owned());
        }
        let size = stdout
            .metadata()
            .map_err(|_| "cannot inspect Git evidence".to_owned())?
            .len();
        if size > 32 * 1024 * 1024 {
            return Err("Git evidence exceeded 32 MiB".to_owned());
        }
        stdout
            .rewind()
            .map_err(|_| "cannot rewind Git evidence".to_owned())?;
        let mut output = Vec::new();
        stdout
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut output)
            .map_err(|_| "cannot read Git evidence".to_owned())?;
        Ok(output)
    }

    pub fn tracked(&self, workspace: &Path) -> Checked<BTreeSet<String>> {
        let top = self.run(workspace, &["rev-parse", "--show-toplevel"], None, &[0])?;
        let top = std::str::from_utf8(&top)
            .map_err(|_| "Git root is not UTF-8".to_owned())?
            .trim();
        let canonical = workspace
            .canonicalize()
            .map_err(|_| "consumer root is unavailable".to_owned())?;
        if Path::new(top).canonicalize().ok().as_ref() != Some(&canonical) {
            return Err(
                "workspace must be its own Git worktree root, not a subdirectory".to_owned(),
            );
        }
        let raw = self.run(workspace, &["ls-files", "--stage", "-z"], None, &[0])?;
        let mut paths = BTreeSet::new();
        for record in raw
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            let text =
                std::str::from_utf8(record).map_err(|_| "index path is not UTF-8".to_owned())?;
            let (metadata, path) = text
                .split_once('\t')
                .ok_or_else(|| "malformed Git index evidence".to_owned())?;
            safe_path(path)?;
            let fields: Vec<_> = metadata.split_ascii_whitespace().collect();
            if fields.len() != 3 || fields[2] != "0" || !matches!(fields[0], "100644" | "100755") {
                return Err(
                    "unmerged index, symlink, or submodule requires separate tracked-file evidence"
                        .to_owned(),
                );
            }
            paths.insert(path.to_owned());
            if paths.len() > MAX_PROBES {
                return Err("tracked path inventory exceeded its limit".to_owned());
            }
        }
        Ok(paths)
    }

    pub fn evaluate(
        &self,
        name: &str,
        policies: &BTreeMap<String, Vec<u8>>,
        directories: &BTreeSet<String>,
        paths: &BTreeSet<String>,
    ) -> Checked<BTreeMap<String, Match>> {
        let root = self.temporary.path().join(name);
        let template = self.temporary.path().join("empty-template");
        std::fs::create_dir_all(&root)
            .and_then(|()| std::fs::create_dir_all(&template))
            .map_err(|_| "cannot create disposable Git repository".to_owned())?;
        self.run(
            &root,
            &[
                "init",
                "--quiet",
                "--initial-branch=main",
                &format!("--template={}", template.display()),
            ],
            None,
            &[0],
        )?;
        for directory in directories {
            safe_path(directory)?;
            std::fs::create_dir_all(root.join(directory))
                .map_err(|_| "cannot reproduce directory topology".to_owned())?;
        }
        for (path, raw) in policies {
            safe_path(path)?;
            let destination = root.join(path);
            std::fs::create_dir_all(
                destination
                    .parent()
                    .ok_or_else(|| "invalid policy parent".to_owned())?,
            )
            .and_then(|()| std::fs::write(destination, raw))
            .map_err(|_| "cannot reproduce ignore policy topology".to_owned())?;
        }
        let mut input = Vec::new();
        for path in paths {
            safe_path(path)?;
            let destination = root.join(path);
            if !destination.exists() {
                std::fs::create_dir_all(
                    destination
                        .parent()
                        .ok_or_else(|| "invalid probe parent".to_owned())?,
                )
                .and_then(|()| std::fs::write(&destination, b""))
                .map_err(|_| "probe conflicts with file/directory topology".to_owned())?;
            }
            input.extend_from_slice(path.as_bytes());
            input.push(0);
        }
        let output = self.run(
            &root,
            &[
                "check-ignore",
                "--no-index",
                "--verbose",
                "--non-matching",
                "--stdin",
                "-z",
            ],
            Some(&input),
            &[0, 1],
        )?;
        parse_matches(&output, policies, paths)
    }
}

fn parse_matches(
    output: &[u8],
    policies: &BTreeMap<String, Vec<u8>>,
    paths: &BTreeSet<String>,
) -> Checked<BTreeMap<String, Match>> {
    let fields: Vec<_> = output.split(|byte| *byte == 0).collect();
    if fields.last() != Some(&&b""[..]) || (fields.len() - 1) % 4 != 0 {
        return Err("malformed Git winning-rule evidence".to_owned());
    }
    let mut matches = BTreeMap::new();
    for record in fields[..fields.len() - 1].chunks_exact(4) {
        let values: Vec<_> = record
            .iter()
            .map(|value| std::str::from_utf8(value))
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| "Git rule evidence is not UTF-8".to_owned())?;
        let path = values[3];
        if !paths.contains(path) || matches.contains_key(path) {
            return Err("Git returned an unexpected or duplicate probe".to_owned());
        }
        let rule = if values[0].is_empty() {
            None
        } else {
            if !policies.contains_key(values[0])
                || values[2].len() > 4096
                || values[2].chars().any(char::is_control)
            {
                return Err("Git returned an unsupported winning policy/rule".to_owned());
            }
            let line = values[1]
                .parse::<u32>()
                .ok()
                .filter(|line| *line > 0)
                .ok_or_else(|| "Git returned an invalid rule line".to_owned())?;
            Some(GitignoreWinningRule {
                path: values[0].to_owned(),
                line,
                pattern: values[2].to_owned(),
            })
        };
        matches.insert(
            path.to_owned(),
            Match {
                ignored: rule.is_some() && !values[2].starts_with('!'),
                rule,
            },
        );
    }
    if matches.len() != paths.len() {
        return Err("Git did not return every requested probe".to_owned());
    }
    Ok(matches)
}
