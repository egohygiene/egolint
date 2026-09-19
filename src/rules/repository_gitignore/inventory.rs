//! Bounded policy and pathname inventory that never reads ordinary file payloads.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use super::{MAX_ENTRIES, MAX_FILE_BYTES, MAX_POLICIES, MAX_POLICY_BYTES, safe_path};

type Checked<T> = std::result::Result<T, String>;

pub(super) fn read_file(workspace: &Path, relative: &str) -> Checked<Vec<u8>> {
    safe_path(relative)?;
    let mut path = workspace.to_path_buf();
    for component in relative.split('/') {
        path.push(component);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| "required policy/input is unavailable".to_owned())?;
        if metadata.file_type().is_symlink() {
            return Err("symlink policy/input paths are unsupported".to_owned());
        }
    }
    let metadata =
        std::fs::symlink_metadata(&path).map_err(|_| "cannot inspect policy/input".to_owned())?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES as u64 {
        return Err("policy/input must be a regular file no larger than 1 MiB".to_owned());
    }
    let file = std::fs::File::open(&path).map_err(|_| "cannot read policy/input".to_owned())?;
    let mut raw = Vec::new();
    file.take(MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|_| "cannot read policy/input".to_owned())?;
    if raw.len() > MAX_FILE_BYTES {
        return Err("policy/input exceeded its byte limit".to_owned());
    }
    Ok(raw)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub policies: BTreeMap<String, Vec<u8>>,
    pub paths: BTreeSet<String>,
    pub directories: BTreeSet<String>,
    pub problems: Vec<(Option<String>, String)>,
}

// A single bounded traversal makes skipped boundaries and budget exhaustion explicit.
#[allow(clippy::too_many_lines)]
pub(super) fn collect(workspace: &Path) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let mut pending = vec![String::new()];
    let mut entries_seen = 0;
    let mut policy_bytes = 0;
    while let Some(relative) = pending.pop() {
        let directory = workspace.join(&relative);
        if !relative.is_empty() && std::fs::symlink_metadata(directory.join(".git")).is_ok() {
            snapshot.problems.push((
                Some(relative),
                "nested Git repository/submodule requires separate validation".to_owned(),
            ));
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&directory) else {
            snapshot.problems.push((
                if relative.is_empty() {
                    None
                } else {
                    Some(relative)
                },
                "directory inventory is unavailable".to_owned(),
            ));
            continue;
        };
        // Limit before sorting so an enormous directory cannot consume an
        // unbounded allocation. An exhausted budget always prevents a pass.
        let mut entries: Vec<_> = entries.take(MAX_ENTRIES + 1).collect();
        entries.sort_by_key(|entry| entry.as_ref().ok().map(std::fs::DirEntry::file_name));
        for entry in entries {
            entries_seen += 1;
            if entries_seen > MAX_ENTRIES {
                snapshot
                    .problems
                    .push((None, "metadata inventory exceeded 50000 entries".to_owned()));
                return snapshot;
            }
            let Ok(entry) = entry else {
                snapshot
                    .problems
                    .push((None, "directory entry is unreadable".to_owned()));
                continue;
            };
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                snapshot.problems.push((
                    None,
                    "non-UTF-8 path prevents complete policy inventory".to_owned(),
                ));
                continue;
            };
            if name == ".git" {
                continue;
            }
            let path = if relative.is_empty() {
                name.clone()
            } else {
                format!("{relative}/{name}")
            };
            if path == ".reports/egolint" {
                continue;
            }
            if safe_path(&path).is_err() {
                snapshot.problems.push((
                    None,
                    "unsupported pathname prevents complete policy inventory".to_owned(),
                ));
                continue;
            }
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                snapshot
                    .problems
                    .push((Some(path), "path metadata is unavailable".to_owned()));
                continue;
            };
            if metadata.file_type().is_symlink() {
                snapshot.problems.push((
                    Some(path),
                    "symlink inventory boundary is not followed".to_owned(),
                ));
            } else if metadata.is_dir() {
                snapshot.directories.insert(path.clone());
                pending.push(path);
            } else if metadata.is_file() {
                snapshot.paths.insert(path.clone());
                if name.eq_ignore_ascii_case(".gitignore") {
                    if name != ".gitignore" {
                        snapshot.problems.push((
                            Some(path),
                            "mis-cased ignore policy is unsupported".to_owned(),
                        ));
                    } else if snapshot.policies.len() >= MAX_POLICIES {
                        snapshot
                            .problems
                            .push((Some(path), "ignore policy count exceeded 256".to_owned()));
                    } else {
                        match read_file(workspace, &path) {
                            Ok(raw) if policy_bytes + raw.len() <= MAX_POLICY_BYTES => {
                                policy_bytes += raw.len();
                                snapshot.policies.insert(path, raw);
                            }
                            Ok(_) => snapshot.problems.push((
                                Some(path),
                                "ignore policies exceeded 8 MiB total".to_owned(),
                            )),
                            Err(message) => snapshot.problems.push((Some(path), message)),
                        }
                    }
                }
            } else {
                snapshot.problems.push((
                    Some(path),
                    "non-regular filesystem entry is unsupported".to_owned(),
                ));
            }
        }
    }
    snapshot.problems.sort();
    snapshot
}
