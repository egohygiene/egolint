//! Deterministic, Git-aware repository inventory.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::error::{EgolintError, Result};

/// Kind of repository entry presented to a rule pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryEntryKind {
    /// Regular file with optional readable bytes.
    File,
    /// Symbolic link. `content` contains the link text, never target contents.
    Symlink,
}

/// One normalized repository entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryEntry {
    /// Workspace-relative path using `/` separators.
    pub path: PathBuf,
    /// Git index mode when the entry is tracked.
    pub git_mode: Option<u32>,
    /// Entry kind without following symbolic links.
    pub kind: RepositoryEntryKind,
    /// File bytes or symlink target bytes.
    pub content: Vec<u8>,
}

impl RepositoryEntry {
    /// Construct a regular entry for a rule unit test or adapter.
    #[must_use]
    pub fn file(path: impl Into<PathBuf>, git_mode: Option<u32>, content: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            git_mode,
            kind: RepositoryEntryKind::File,
            content,
        }
    }

    /// Return the normalized path as UTF-8.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when an adapter supplied a non-UTF-8 path.
    pub fn normalized_path(&self) -> Result<&str> {
        self.path.to_str().ok_or_else(|| {
            EgolintError::Configuration("repository paths must contain valid UTF-8".to_owned())
        })
    }
}

/// Sorted snapshot of the current repository worktree and tracked Git modes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepositoryInventory {
    entries: Vec<RepositoryEntry>,
}

impl RepositoryInventory {
    /// Build an inventory from already-normalized entries.
    ///
    /// # Errors
    ///
    /// Returns an error for an absolute, parent-traversing, duplicate, or
    /// non-UTF-8 path. Backslashes remain valid input so the portability rules
    /// can report them as Windows-incompatible repository names.
    pub fn from_entries(mut entries: Vec<RepositoryEntry>) -> Result<Self> {
        for entry in &entries {
            validate_relative_path(&entry.path)?;
            entry.normalized_path()?;
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        for pair in entries.windows(2) {
            if pair[0].path == pair[1].path {
                return Err(EgolintError::Configuration(format!(
                    "repository inventory contains duplicate path {}",
                    pair[0].path.display()
                )));
            }
        }
        Ok(Self { entries })
    }

    /// Discover tracked and non-ignored untracked files without parsing
    /// newline-delimited Git output or trusting platform filesystem modes.
    ///
    /// # Errors
    ///
    /// Returns an error when Git cannot enumerate the repository, an entry path
    /// is unsafe or non-UTF-8, or file metadata/content cannot be read.
    pub fn discover(workspace: &Path) -> Result<Self> {
        let workspace = workspace
            .canonicalize()
            .map_err(|source| EgolintError::Filesystem {
                path: workspace.to_path_buf(),
                source,
            })?;
        let modes = tracked_modes(&workspace)?;
        let paths = run_git(
            &workspace,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        )?;
        let mut entries = Vec::new();
        for path_bytes in nul_records(&paths) {
            let path_string = std::str::from_utf8(path_bytes).map_err(|_| {
                EgolintError::Configuration(
                    "Git repository paths must contain valid UTF-8".to_owned(),
                )
            })?;
            let relative = PathBuf::from(path_string);
            validate_relative_path(&relative)?;
            if relative == Path::new(".reports/egolint") || relative.starts_with(".reports/egolint")
            {
                continue;
            }
            let absolute = workspace.join(&relative);
            let metadata = match std::fs::symlink_metadata(&absolute) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(EgolintError::Filesystem {
                        path: absolute,
                        source,
                    });
                }
            };
            let (kind, content) = if metadata.file_type().is_symlink() {
                let target =
                    std::fs::read_link(&absolute).map_err(|source| EgolintError::Filesystem {
                        path: absolute.clone(),
                        source,
                    })?;
                (
                    RepositoryEntryKind::Symlink,
                    target.to_string_lossy().as_bytes().to_vec(),
                )
            } else if metadata.is_file() {
                let content =
                    std::fs::read(&absolute).map_err(|source| EgolintError::Filesystem {
                        path: absolute.clone(),
                        source,
                    })?;
                (RepositoryEntryKind::File, content)
            } else {
                continue;
            };
            entries.push(RepositoryEntry {
                git_mode: modes.get(path_string).copied(),
                path: relative,
                kind,
                content,
            });
        }
        Self::from_entries(entries)
    }

    /// Return entries in normalized lexical order.
    #[must_use]
    pub fn entries(&self) -> &[RepositoryEntry] {
        &self.entries
    }

    /// Find one entry using exact path casing.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<&RepositoryEntry> {
        self.entries
            .binary_search_by(|entry| entry.path.as_path().cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// Return whether an exact-case directory exists in the inventory.
    #[must_use]
    pub fn contains_directory(&self, directory: &Path) -> bool {
        self.entries.iter().any(|entry| {
            entry
                .path
                .ancestors()
                .skip(1)
                .any(|ancestor| ancestor == directory)
        })
    }
}

fn tracked_modes(workspace: &Path) -> Result<BTreeMap<String, u32>> {
    let output = run_git(workspace, &["ls-files", "--stage", "-z"])?;
    let mut modes = BTreeMap::new();
    for record in nul_records(&output) {
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| {
                EgolintError::Configuration("Git index record is missing a path".to_owned())
            })?;
        let header = std::str::from_utf8(&record[..tab]).map_err(|_| {
            EgolintError::Configuration("Git index metadata must contain valid UTF-8".to_owned())
        })?;
        let mode = header
            .split_ascii_whitespace()
            .next()
            .ok_or_else(|| {
                EgolintError::Configuration("Git index record is missing a mode".to_owned())
            })?
            .parse::<u32>()
            .map_err(|_| {
                EgolintError::Configuration("Git index mode must be octal digits".to_owned())
            })?;
        let path = std::str::from_utf8(&record[tab + 1..]).map_err(|_| {
            EgolintError::Configuration("Git repository paths must contain valid UTF-8".to_owned())
        })?;
        modes.insert(path.to_owned(), mode);
    }
    Ok(modes)
}

fn run_git(workspace: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(EgolintError::Configuration(format!(
            "Git repository inventory failed: {message}"
        )))
    }
}

fn nul_records(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(EgolintError::Configuration(
            "repository inventory paths must be non-empty and relative".to_owned(),
        ));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(EgolintError::Configuration(format!(
                    "repository inventory path must be normalized: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_sorts_entries_and_preserves_git_modes() {
        let inventory = RepositoryInventory::from_entries(vec![
            RepositoryEntry::file("scripts/z.sh", Some(100_755), b"#!/bin/sh\n".to_vec()),
            RepositoryEntry::file("README.md", Some(100_644), b"# Example\n".to_vec()),
        ])
        .expect("valid inventory");

        assert_eq!(inventory.entries()[0].path, PathBuf::from("README.md"));
        assert_eq!(inventory.entries()[1].git_mode, Some(100_755));
        assert!(inventory.contains_directory(Path::new("scripts")));
    }

    #[test]
    fn inventory_rejects_unsafe_or_duplicate_paths() {
        assert!(
            RepositoryInventory::from_entries(vec![RepositoryEntry::file(
                "../escape",
                None,
                Vec::new(),
            )])
            .is_err()
        );
        assert!(
            RepositoryInventory::from_entries(vec![
                RepositoryEntry::file("same", None, Vec::new()),
                RepositoryEntry::file("same", None, Vec::new()),
            ])
            .is_err()
        );
    }
}
