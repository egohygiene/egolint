//! Bounded, reviewable, and reversible fix previews.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use sha2::{Digest, Sha256};

use crate::ResolvedConfig;
use crate::error::{EgolintError, Result, exit_code};
use crate::plan::{ExecutionPlan, Operation, PlanOptions};

/// Fixed reviewable patch path beneath the original workspace.
pub const FIX_PATCH_PATH: &str = ".reports/egolint/fixes.patch";

const MAX_PATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TREE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TREE_ENTRIES: usize = 200_000;
const MAX_GIT_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_TREE_LIST_BYTES: usize = 64 * 1024 * 1024;

/// Outcome of an isolated fix-preview run.
#[derive(Debug)]
pub struct FixOutcome {
    /// Raw adapter exit status from the isolated repository.
    pub adapter_exit_code: Option<i32>,
    /// Whether the selected linters proposed any source change.
    pub changed: bool,
    /// Stable workspace-relative patch path.
    pub patch_path: PathBuf,
    /// Lowercase SHA-256 of the exact patch bytes.
    pub patch_sha256: String,
    /// Original immutable Git commit against which the preview was created.
    pub base_commit: String,
    /// Exact Git tree expected after applying the reviewed patch.
    pub post_tree: String,
}

/// Run selected fixes in an isolated copy and write a reviewable patch.
///
/// The lint container never receives write access to the original repository.
/// Its mutable Git metadata is discarded before any host-side comparison. A
/// separately initialized comparison repository, never mounted into the
/// container, creates the patch. The adapter report boundary is removed before
/// comparison whether or not the consumer ignores it.
///
/// Applying is intentionally a separate operation: use [`apply_reviewed_fix`]
/// with the printed patch digest and base commit after reviewing the patch.
///
/// # Errors
///
/// Returns an error for an unbounded selection, dirty/non-root worktree,
/// unsupported symlink/submodule repository, unsafe report boundary, adapter
/// failure, or a copy/patch that exceeds its review budget.
// Keep the immutable-snapshot, untrusted-adapter, trusted-comparison, and
// publication sequence linear so the capability boundary remains auditable.
#[allow(clippy::too_many_lines)]
pub fn run_isolated_fix(
    workspace: &Path,
    resolved: &ResolvedConfig,
    options: &PlanOptions,
) -> Result<FixOutcome> {
    if options.enable_linters.is_empty() {
        return Err(EgolintError::Configuration(
            "fix requires at least one explicit --enable-linter selection".to_owned(),
        ));
    }
    if options.changed_only {
        return Err(EgolintError::Configuration(
            "fix previews require a full immutable snapshot; --changed-only is unsupported"
                .to_owned(),
        ));
    }
    let base_commit = require_clean_repository_root(workspace)?;
    let object_format = repository_object_format(workspace)?;
    validate_oid(&base_commit, &object_format, "reviewed fix base")?;
    reject_unsupported_repository_shapes(workspace)?;
    let base_tree = rev_parse_tree(workspace, &base_commit, &object_format)?;

    let original_plan =
        ExecutionPlan::build(workspace, resolved, Operation::Check, options, false)?;
    original_plan.prepare_report_directory()?;

    let temporary = tempfile::tempdir().map_err(|source| EgolintError::Filesystem {
        path: std::env::temp_dir(),
        source,
    })?;
    let candidate = temporary.path().join("candidate");
    let comparison = temporary.path().join("comparison");
    let candidate_executables =
        materialize_commit_tree(workspace, &candidate, &base_commit, &object_format)?;
    let comparison_executables =
        materialize_commit_tree(workspace, &comparison, &base_commit, &object_format)?;
    initialize_repository(&candidate, &object_format, &candidate_executables)?;
    initialize_repository(&comparison, &object_format, &comparison_executables)?;
    for snapshot in [&candidate, &comparison] {
        let observed = write_tree(snapshot, &object_format)?;
        if observed != base_tree {
            return Err(EgolintError::Configuration(format!(
                "immutable fix snapshot changed the base tree: expected {base_tree}, observed {observed}"
            )));
        }
    }

    let isolated_resolved = resolved_for_isolated_copy(resolved, workspace, &candidate)?;
    let isolated_plan = ExecutionPlan::build(
        &candidate,
        &isolated_resolved,
        Operation::Fix,
        options,
        true,
    )?;
    let status = isolated_plan.execute()?;
    let adapter_exit_code = status.code();
    if !matches!(
        adapter_exit_code,
        Some(exit_code::CLEAN | exit_code::FINDINGS)
    ) {
        return Err(EgolintError::RuntimeExecution(format!(
            "isolated fix adapter exited with {}",
            adapter_exit_code.map_or_else(|| "no status".to_owned(), |code| code.to_string())
        )));
    }

    let observed_base = require_clean_repository_root(workspace)?;
    if observed_base != base_commit {
        return Err(EgolintError::Configuration(
            "repository changed while the isolated fix preview was running".to_owned(),
        ));
    }

    remove_isolated_metadata(&candidate)?;
    replace_comparison_worktree(&comparison, &candidate)?;
    run_git_checked(
        &comparison,
        &[
            OsStr::new("add"),
            OsStr::new("--force"),
            OsStr::new("--all"),
            OsStr::new("--"),
        ],
    )?;
    let post_tree = write_tree(&comparison, &object_format)?;
    let patch = git_output_bounded(
        &comparison,
        &[
            OsStr::new("diff"),
            OsStr::new("--cached"),
            OsStr::new("--binary"),
            OsStr::new("--full-index"),
            OsStr::new("--no-ext-diff"),
            OsStr::new("--no-textconv"),
            OsStr::new("--src-prefix=a/"),
            OsStr::new("--dst-prefix=b/"),
            OsStr::new("--"),
        ],
        MAX_PATCH_BYTES,
    )?;

    original_plan.validate_report_path()?;
    let observed_base = require_clean_repository_root(workspace)?;
    if observed_base != base_commit {
        return Err(EgolintError::Configuration(
            "repository changed before the fix preview could be published".to_owned(),
        ));
    }
    let patch_path = original_plan.report_path().join("fixes.patch");
    write_bytes_atomic(&patch, &patch_path)?;
    original_plan.validate_report_path()?;
    Ok(FixOutcome {
        adapter_exit_code,
        changed: !patch.is_empty(),
        patch_path: PathBuf::from(FIX_PATCH_PATH),
        patch_sha256: sha256_hex(&patch),
        base_commit,
        post_tree,
    })
}

/// Apply the exact reviewed preview after revalidating its digest and base.
///
/// The worktree must still be clean and at `expected_base_commit`. The fixed
/// patch path is read into bounded memory, matched against
/// `expected_patch_sha256`, checked by Git, and only then applied. The same
/// patch remains available for `git apply --reverse` rollback.
///
/// # Errors
///
/// Returns an error when the patch was replaced, the repository moved or
/// changed, or Git cannot verify and apply it exactly.
pub fn apply_reviewed_fix(
    workspace: &Path,
    expected_patch_sha256: &str,
    expected_base_commit: &str,
    expected_post_tree: &str,
) -> Result<()> {
    validate_sha256(expected_patch_sha256)?;
    let object_format = repository_object_format(workspace)?;
    validate_oid(expected_base_commit, &object_format, "reviewed fix base")?;
    validate_oid(expected_post_tree, &object_format, "reviewed fix post-tree")?;
    reject_unsupported_repository_shapes(workspace)?;
    let observed_base = require_clean_repository_root(workspace)?;
    if observed_base != expected_base_commit {
        return Err(EgolintError::Configuration(format!(
            "reviewed fix base changed: expected {expected_base_commit}, observed {observed_base}"
        )));
    }
    let patch_path = workspace.join(FIX_PATCH_PATH);
    validate_fixed_report_parent(workspace, &patch_path)?;
    let (validated_patch_path, _) = crate::sarif::validated_report_target(&patch_path)?;
    if validated_patch_path != patch_path {
        return Err(EgolintError::Configuration(
            "reviewed fix path changed during validation".to_owned(),
        ));
    }
    let patch = read_regular_file_bounded(&validated_patch_path, MAX_PATCH_BYTES)?;
    let observed_digest = sha256_hex(&patch);
    if observed_digest != expected_patch_sha256 {
        return Err(EgolintError::Configuration(format!(
            "reviewed fix digest changed: expected {expected_patch_sha256}, observed {observed_digest}"
        )));
    }
    apply_patch(workspace, &patch, true, false)?;
    let final_base = require_clean_repository_root(workspace)?;
    if final_base != expected_base_commit {
        return Err(EgolintError::Configuration(
            "repository changed while the reviewed patch was being verified".to_owned(),
        ));
    }
    reject_unsupported_repository_shapes(workspace)?;
    apply_patch(workspace, &patch, false, false)?;
    if let Err(verification) = verify_applied_worktree(workspace) {
        let rollback = apply_patch(workspace, &patch, false, true);
        return Err(EgolintError::RuntimeExecution(format!(
            "reviewed patch failed its final worktree verification ({verification}); rollback {}",
            if rollback.is_ok() {
                "succeeded"
            } else {
                "failed"
            }
        )));
    }
    let observed_post_tree = write_tree(workspace, &object_format)?;
    if observed_post_tree != expected_post_tree {
        let rollback = apply_patch(workspace, &patch, false, true);
        return Err(EgolintError::RuntimeExecution(format!(
            "reviewed patch produced tree {observed_post_tree}, expected {expected_post_tree}; rollback {}",
            if rollback.is_ok() {
                "succeeded"
            } else {
                "failed"
            }
        )));
    }
    Ok(())
}

fn resolved_for_isolated_copy(
    resolved: &ResolvedConfig,
    workspace: &Path,
    candidate: &Path,
) -> Result<ResolvedConfig> {
    let mut isolated = resolved.clone();
    if let Some(config) = resolved.config.megalinter_config.as_deref() {
        let relative = config.strip_prefix(workspace).map_err(|_| {
            EgolintError::Configuration(
                "MegaLinter config must remain inside the fix-preview workspace".to_owned(),
            )
        })?;
        isolated.config.megalinter_config = Some(candidate.join(relative));
    }
    Ok(isolated)
}

fn require_clean_repository_root(workspace: &Path) -> Result<String> {
    let root = git_output_bounded(
        workspace,
        &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let root = String::from_utf8(root).map_err(|_| {
        EgolintError::Configuration("Git repository root must contain valid UTF-8".to_owned())
    })?;
    let root = root.trim_end_matches(['\r', '\n']);
    let canonical_root =
        Path::new(root)
            .canonicalize()
            .map_err(|source| EgolintError::Filesystem {
                path: PathBuf::from(root),
                source,
            })?;
    let canonical_workspace =
        workspace
            .canonicalize()
            .map_err(|source| EgolintError::Filesystem {
                path: workspace.to_path_buf(),
                source,
            })?;
    if canonical_root != canonical_workspace {
        return Err(EgolintError::Configuration(
            "fix workspace must equal the Git repository root".to_owned(),
        ));
    }
    let status = git_output_bounded(
        workspace,
        &[
            OsStr::new("status"),
            OsStr::new("--porcelain=v1"),
            OsStr::new("--untracked-files=all"),
            OsStr::new("-z"),
        ],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if status_records_outside_report(&status) {
        return Err(EgolintError::Configuration(
            "fix preview/apply requires a clean worktree outside .reports/egolint".to_owned(),
        ));
    }
    let commit = git_output_bounded(
        workspace,
        &[OsStr::new("rev-parse"), OsStr::new("HEAD")],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let commit = String::from_utf8(commit).map_err(|_| {
        EgolintError::Configuration("Git commit id must contain valid UTF-8".to_owned())
    })?;
    let commit = commit.trim_end_matches(['\r', '\n']).to_owned();
    validate_git_oid_syntax(&commit, "Git HEAD")?;
    Ok(commit)
}

fn status_records_outside_report(status: &[u8]) -> bool {
    status
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .any(|record| {
            let is_untracked = record.starts_with(b"?? ");
            let path = if record.len() >= 3 && record[2] == b' ' {
                &record[3..]
            } else {
                record
            };
            !is_untracked
                || (path != b".reports/egolint" && !path.starts_with(b".reports/egolint/"))
        })
}

fn reject_unsupported_repository_shapes(workspace: &Path) -> Result<()> {
    let sparse = git_output_bounded(
        workspace,
        &[
            OsStr::new("config"),
            OsStr::new("--type=bool"),
            OsStr::new("--default=false"),
            OsStr::new("--get"),
            OsStr::new("core.sparseCheckout"),
        ],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if sparse == b"true\n" || sparse == b"true\r\n" {
        return Err(EgolintError::Configuration(
            "fix preview/apply currently rejects sparse checkouts; lint remains available"
                .to_owned(),
        ));
    }
    let flags = git_output_bounded(
        workspace,
        &[OsStr::new("ls-files"), OsStr::new("-v"), OsStr::new("-z")],
        MAX_TREE_LIST_BYTES,
    )?;
    if flags
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .any(|record| record.first() != Some(&b'H'))
    {
        return Err(EgolintError::Configuration(
            "fix preview/apply rejects assume-unchanged, skip-worktree, and other special index flags"
                .to_owned(),
        ));
    }
    Ok(())
}

fn verify_applied_worktree(workspace: &Path) -> Result<()> {
    reject_unsupported_repository_shapes(workspace)?;
    run_git_checked(
        workspace,
        &[
            OsStr::new("diff"),
            OsStr::new("--quiet"),
            OsStr::new("--no-ext-diff"),
            OsStr::new("--no-textconv"),
            OsStr::new("--"),
        ],
    )
    .map_err(|_| {
        EgolintError::Configuration(
            "reviewed patch left tracked worktree content different from the staged index"
                .to_owned(),
        )
    })?;

    let untracked = git_output_bounded(
        workspace,
        &[
            OsStr::new("ls-files"),
            OsStr::new("--others"),
            OsStr::new("--exclude-standard"),
            OsStr::new("-z"),
        ],
        MAX_TREE_LIST_BYTES,
    )?;
    if untracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .any(|path| path != b".reports/egolint" && !path.starts_with(b".reports/egolint/"))
    {
        return Err(EgolintError::Configuration(
            "reviewed patch left an untracked path outside .reports/egolint".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct TreeEntry {
    mode: String,
    oid: String,
    path: PathBuf,
}

fn materialize_commit_tree(
    source: &Path,
    destination: &Path,
    commit: &str,
    object_format: &str,
) -> Result<Vec<PathBuf>> {
    let listing = git_output_bounded(
        source,
        &[
            OsStr::new("ls-tree"),
            OsStr::new("--full-tree"),
            OsStr::new("-r"),
            OsStr::new("-z"),
            OsStr::new(commit),
        ],
        MAX_TREE_LIST_BYTES,
    )?;
    let entries = parse_tree_entries(&listing, object_format)?;
    std::fs::create_dir(destination).map_err(|source_error| EgolintError::Filesystem {
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    let mut budget = CopyBudget::default();
    let mut stderr_file = tempfile::tempfile().map_err(|source| EgolintError::Filesystem {
        path: std::env::temp_dir(),
        source,
    })?;
    let stderr_writer = stderr_file
        .try_clone()
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    let mut child = hardened_git_command(source)
        .arg("cat-file")
        .arg("--batch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| EgolintError::RuntimeExecution("Git object input unavailable".to_owned()))?;
    let output = child.stdout.take().ok_or_else(|| {
        EgolintError::RuntimeExecution("Git object output unavailable".to_owned())
    })?;
    let mut output = BufReader::new(output);
    for entry in &entries {
        writeln!(input, "{}", entry.oid)
            .and_then(|()| input.flush())
            .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
        materialize_batch_blob(&mut output, destination, entry, object_format, &mut budget)?;
    }
    drop(input);
    let status = child
        .wait()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if status.success() {
        return Ok(entries
            .into_iter()
            .filter(|entry| entry.mode == "100755")
            .map(|entry| entry.path)
            .collect());
    }
    stderr_file
        .seek(SeekFrom::Start(0))
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    let mut stderr = Vec::new();
    stderr_file
        .take(4_096)
        .read_to_end(&mut stderr)
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    Err(EgolintError::RuntimeExecution(format!(
        "Git object materialization failed: {}",
        bounded_stderr(&stderr)
    )))
}

fn parse_tree_entries(listing: &[u8], object_format: &str) -> Result<Vec<TreeEntry>> {
    let mut entries = Vec::new();
    for record in listing
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
    {
        if entries.len() >= MAX_TREE_ENTRIES {
            return Err(EgolintError::Configuration(format!(
                "fix preview exceeds the {MAX_TREE_ENTRIES}-entry snapshot budget"
            )));
        }
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| {
                EgolintError::RuntimeExecution(
                    "Git tree entry omitted its path separator".to_owned(),
                )
            })?;
        let (header, path_with_separator) = record.split_at(separator);
        let path = &path_with_separator[1..];
        let header = std::str::from_utf8(header).map_err(|_| {
            EgolintError::RuntimeExecution("Git tree metadata was not UTF-8".to_owned())
        })?;
        let mut fields = header.split_ascii_whitespace();
        let mode = fields.next().unwrap_or_default();
        let kind = fields.next().unwrap_or_default();
        let oid = fields.next().unwrap_or_default();
        if fields.next().is_some() || kind != "blob" || !matches!(mode, "100644" | "100755") {
            return Err(EgolintError::Configuration(format!(
                "fix preview rejects symlinks, gitlinks, and unsupported tree mode {mode}/{kind}"
            )));
        }
        validate_oid(oid, object_format, "Git tree object")?;
        let path = std::str::from_utf8(path).map_err(|_| {
            EgolintError::Configuration("fix snapshot paths must contain valid UTF-8".to_owned())
        })?;
        let path = PathBuf::from(path);
        validate_copy_path(&path)?;
        if path
            .components()
            .next()
            .is_some_and(|component| component.as_os_str() == ".reports")
        {
            return Err(EgolintError::Configuration(
                "fix preview rejects committed .reports content; remove generated evidence from Git"
                    .to_owned(),
            ));
        }
        entries.push(TreeEntry {
            mode: mode.to_owned(),
            oid: oid.to_owned(),
            path,
        });
    }
    Ok(entries)
}

fn materialize_batch_blob<R: BufRead>(
    output: &mut R,
    destination: &Path,
    entry: &TreeEntry,
    object_format: &str,
    budget: &mut CopyBudget,
) -> Result<()> {
    let mut header = String::new();
    output
        .read_line(&mut header)
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    let mut fields = header.split_ascii_whitespace();
    let observed_oid = fields.next().unwrap_or_default();
    let kind = fields.next().unwrap_or_default();
    let size = fields
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            EgolintError::RuntimeExecution("Git blob response omitted a valid size".to_owned())
        })?;
    if fields.next().is_some() || observed_oid != entry.oid || kind != "blob" {
        return Err(EgolintError::RuntimeExecution(
            "Git blob response did not match the requested immutable object".to_owned(),
        ));
    }
    validate_oid(observed_oid, object_format, "materialized Git object")?;
    if size > MAX_FILE_BYTES {
        return Err(EgolintError::Configuration(format!(
            "fix preview file exceeds the {MAX_FILE_BYTES}-byte limit: {}",
            entry.path.display()
        )));
    }
    budget.entries += 1;
    budget.bytes = budget.bytes.saturating_add(size);
    if budget.entries > MAX_TREE_ENTRIES || budget.bytes > MAX_TREE_BYTES {
        return Err(EgolintError::Configuration(
            "fix preview immutable snapshot exceeded its review budget".to_owned(),
        ));
    }
    let target = destination.join(&entry.path);
    let parent = target.parent().ok_or_else(|| {
        EgolintError::Configuration("fix snapshot file omitted a parent".to_owned())
    })?;
    std::fs::create_dir_all(parent).map_err(|source| EgolintError::Filesystem {
        path: parent.to_path_buf(),
        source,
    })?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&target)
        .map_err(|source| EgolintError::Filesystem {
            path: target.clone(),
            source,
        })?;
    let copied = std::io::copy(&mut output.take(size), &mut file).map_err(|source| {
        EgolintError::Filesystem {
            path: target.clone(),
            source,
        }
    })?;
    if copied != size {
        return Err(EgolintError::RuntimeExecution(format!(
            "Git blob ended early for {}",
            entry.path.display()
        )));
    }
    let mut delimiter = [0_u8; 1];
    output
        .read_exact(&mut delimiter)
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if delimiter != [b'\n'] {
        return Err(EgolintError::RuntimeExecution(
            "Git blob response omitted its record delimiter".to_owned(),
        ));
    }
    file.sync_all().map_err(|source| EgolintError::Filesystem {
        path: target.clone(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = if entry.mode == "100755" { 0o755 } else { 0o644 };
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(permissions)).map_err(
            |source| EgolintError::Filesystem {
                path: target,
                source,
            },
        )?;
    }
    Ok(())
}

#[derive(Default)]
struct CopyBudget {
    entries: usize,
    bytes: u64,
}

fn copy_directory(
    source_root: &Path,
    destination_root: &Path,
    relative: &Path,
    budget: &mut CopyBudget,
) -> Result<()> {
    let source_directory = source_root.join(relative);
    let entries =
        std::fs::read_dir(&source_directory).map_err(|source| EgolintError::Filesystem {
            path: source_directory.clone(),
            source,
        })?;
    let mut entries = entries
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|source| EgolintError::Filesystem {
            path: source_directory.clone(),
            source,
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let child_relative = relative.join(entry.file_name());
        if child_relative == Path::new(".git") || child_relative == Path::new(".reports") {
            continue;
        }
        validate_copy_path(&child_relative)?;
        budget.entries += 1;
        if budget.entries > MAX_TREE_ENTRIES {
            return Err(EgolintError::Configuration(format!(
                "fix preview exceeds the {MAX_TREE_ENTRIES}-entry copy budget"
            )));
        }
        let source_path = source_root.join(&child_relative);
        let destination_path = destination_root.join(&child_relative);
        let metadata =
            std::fs::symlink_metadata(&source_path).map_err(|source| EgolintError::Filesystem {
                path: source_path.clone(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(EgolintError::Configuration(format!(
                "fix preview currently rejects symbolic links: {}",
                child_relative.display()
            )));
        }
        if metadata.is_dir() {
            std::fs::create_dir(&destination_path).map_err(|source| EgolintError::Filesystem {
                path: destination_path.clone(),
                source,
            })?;
            copy_directory(source_root, destination_root, &child_relative, budget)?;
        } else if metadata.is_file() {
            copy_regular_file(&source_path, &destination_path, &child_relative, budget)?;
        } else {
            return Err(EgolintError::Configuration(format!(
                "fix preview rejects non-file repository entries: {}",
                child_relative.display()
            )));
        }
    }
    Ok(())
}

fn copy_regular_file(
    source: &Path,
    destination: &Path,
    relative: &Path,
    budget: &mut CopyBudget,
) -> Result<()> {
    let metadata = source
        .metadata()
        .map_err(|source_error| EgolintError::Filesystem {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(EgolintError::Configuration(format!(
            "fix preview file exceeds the {MAX_FILE_BYTES}-byte limit: {}",
            relative.display()
        )));
    }
    budget.bytes = budget.bytes.saturating_add(metadata.len());
    if budget.bytes > MAX_TREE_BYTES {
        return Err(EgolintError::Configuration(format!(
            "fix preview exceeds the {MAX_TREE_BYTES}-byte copy budget"
        )));
    }
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|source_error| EgolintError::Filesystem {
            path: destination.to_path_buf(),
            source: source_error,
        })
}

fn validate_copy_path(path: &Path) -> Result<()> {
    if path.to_str().is_none()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "fix preview path must be normalized UTF-8: {}",
            path.display()
        )));
    }
    if path
        .components()
        .any(|component| component.as_os_str() == OsStr::new(".git"))
    {
        return Err(EgolintError::Configuration(format!(
            "fix preview rejects nested Git metadata: {}",
            path.display()
        )));
    }
    Ok(())
}

fn initialize_repository(
    workspace: &Path,
    object_format: &str,
    executable_paths: &[PathBuf],
) -> Result<()> {
    let format = format!("--object-format={object_format}");
    run_git_checked(
        workspace,
        &[
            OsStr::new("init"),
            OsStr::new("--quiet"),
            OsStr::new(&format),
        ],
    )?;
    for paths in executable_paths.chunks(100) {
        let mut arguments = vec![OsStr::new("update-index"), OsStr::new("--chmod=+x")];
        arguments.extend(paths.iter().map(|path| path.as_os_str()));
        run_git_checked(workspace, &arguments)?;
    }
    run_git_checked(
        workspace,
        &[
            OsStr::new("add"),
            OsStr::new("--force"),
            OsStr::new("--all"),
            OsStr::new("--"),
        ],
    )?;
    run_git_checked(
        workspace,
        &[
            OsStr::new("-c"),
            OsStr::new("user.name=Egolint"),
            OsStr::new("-c"),
            OsStr::new("user.email=egolint@invalid.example"),
            OsStr::new("commit"),
            OsStr::new("--quiet"),
            OsStr::new("--allow-empty"),
            OsStr::new("--no-gpg-sign"),
            OsStr::new("--message"),
            OsStr::new("Egolint trusted baseline"),
        ],
    )
}

fn repository_object_format(workspace: &Path) -> Result<String> {
    let output = git_output_bounded(
        workspace,
        &[OsStr::new("rev-parse"), OsStr::new("--show-object-format")],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let value = String::from_utf8(output).map_err(|_| {
        EgolintError::Configuration("Git object format must contain valid UTF-8".to_owned())
    })?;
    let value = value.trim_end_matches(['\r', '\n']).to_owned();
    if matches!(value.as_str(), "sha1" | "sha256") {
        Ok(value)
    } else {
        Err(EgolintError::Configuration(format!(
            "unsupported Git object format: {value}"
        )))
    }
}

fn rev_parse_tree(workspace: &Path, commit: &str, object_format: &str) -> Result<String> {
    let expression = format!("{commit}^{{tree}}");
    let output = git_output_bounded(
        workspace,
        &[OsStr::new("rev-parse"), OsStr::new(&expression)],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let tree = String::from_utf8(output).map_err(|_| {
        EgolintError::Configuration("Git tree id must contain valid UTF-8".to_owned())
    })?;
    let tree = tree.trim_end_matches(['\r', '\n']).to_owned();
    validate_oid(&tree, object_format, "Git base tree")?;
    Ok(tree)
}

fn write_tree(workspace: &Path, object_format: &str) -> Result<String> {
    let output = git_output_bounded(workspace, &[OsStr::new("write-tree")], MAX_GIT_OUTPUT_BYTES)?;
    let tree = String::from_utf8(output).map_err(|_| {
        EgolintError::Configuration("Git tree id must contain valid UTF-8".to_owned())
    })?;
    let tree = tree.trim_end_matches(['\r', '\n']).to_owned();
    validate_oid(&tree, object_format, "Git written tree")?;
    Ok(tree)
}

fn remove_isolated_metadata(candidate: &Path) -> Result<()> {
    remove_entry(&candidate.join(".git"))?;
    remove_entry(&candidate.join(".reports"))
}

fn replace_comparison_worktree(comparison: &Path, candidate: &Path) -> Result<()> {
    for entry in std::fs::read_dir(comparison).map_err(|source| EgolintError::Filesystem {
        path: comparison.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| EgolintError::Filesystem {
            path: comparison.to_path_buf(),
            source,
        })?;
        if entry.file_name() != OsStr::new(".git") {
            remove_entry(&entry.path())?;
        }
    }
    let mut budget = CopyBudget::default();
    copy_directory(candidate, comparison, Path::new(""), &mut budget)
}

fn remove_entry(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(EgolintError::Filesystem {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let result = if metadata.is_dir() && !metadata.file_type().is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    result.map_err(|source| EgolintError::Filesystem {
        path: path.to_path_buf(),
        source,
    })
}

fn apply_patch(workspace: &Path, patch: &[u8], check_only: bool, reverse: bool) -> Result<()> {
    let mut stderr_file = tempfile::tempfile().map_err(|source| EgolintError::Filesystem {
        path: std::env::temp_dir(),
        source,
    })?;
    let stderr_writer = stderr_file
        .try_clone()
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    let mut command = hardened_git_command(workspace);
    command
        .arg("apply")
        .arg("--binary")
        .arg("--index")
        .arg("--whitespace=nowarn");
    if check_only {
        command.arg("--check");
    }
    if reverse {
        command.arg("--reverse");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| EgolintError::RuntimeExecution("Git apply stdin unavailable".to_owned()))?
        .write_all(patch)
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    let status = child
        .wait()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        stderr_file
            .seek(SeekFrom::Start(0))
            .map_err(|source| EgolintError::Filesystem {
                path: std::env::temp_dir(),
                source,
            })?;
        let mut stderr = Vec::new();
        stderr_file
            .take(4_096)
            .read_to_end(&mut stderr)
            .map_err(|source| EgolintError::Filesystem {
                path: std::env::temp_dir(),
                source,
            })?;
        Err(EgolintError::RuntimeExecution(format!(
            "Git apply {} failed: {}",
            if check_only {
                "verification"
            } else {
                "execution"
            },
            bounded_stderr(&stderr)
        )))
    }
}

fn run_git_checked(workspace: &Path, arguments: &[&OsStr]) -> Result<()> {
    git_output_bounded(workspace, arguments, MAX_GIT_OUTPUT_BYTES).map(|_| ())
}

fn git_output_bounded(
    workspace: &Path,
    arguments: &[&OsStr],
    maximum_bytes: usize,
) -> Result<Vec<u8>> {
    let mut stderr_file = tempfile::tempfile().map_err(|source| EgolintError::Filesystem {
        path: std::env::temp_dir(),
        source,
    })?;
    let stderr_writer = stderr_file
        .try_clone()
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    let mut child = hardened_git_command(workspace)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    let mut output = Vec::with_capacity(maximum_bytes.min(1024 * 1024));
    child
        .stdout
        .take()
        .ok_or_else(|| EgolintError::RuntimeExecution("Git stdout unavailable".to_owned()))?
        .take((maximum_bytes + 1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if output.len() > maximum_bytes {
        let _ = child.kill();
        let _ = child.wait();
        return Err(EgolintError::RuntimeExecution(format!(
            "Git output exceeds the {maximum_bytes}-byte review limit"
        )));
    }
    let status = child
        .wait()
        .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
    if status.success() {
        return Ok(output);
    }
    stderr_file
        .seek(SeekFrom::Start(0))
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    let mut stderr = Vec::new();
    stderr_file
        .take(4_096)
        .read_to_end(&mut stderr)
        .map_err(|source| EgolintError::Filesystem {
            path: std::env::temp_dir(),
            source,
        })?;
    Err(EgolintError::RuntimeExecution(format!(
        "Git command failed: {}",
        bounded_stderr(&stderr)
    )))
}

fn hardened_git_command(workspace: &Path) -> Command {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-c")
        .arg("core.autocrlf=false")
        .arg("-c")
        .arg("core.safecrlf=false")
        .arg("-c")
        .arg("diff.external=")
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-c")
        .arg("core.untrackedCache=false")
        .arg("-c")
        .arg(format!("core.hooksPath={}", null_device()))
        .arg("-c")
        .arg(format!("core.excludesFile={}", null_device()))
        .arg("-C")
        .arg(workspace)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1");
    command
}

const fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

fn bounded_stderr(value: &[u8]) -> String {
    String::from_utf8_lossy(&value[..value.len().min(4_096)])
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>()
        .trim()
        .to_owned()
}

fn validate_fixed_report_parent(workspace: &Path, path: &Path) -> Result<()> {
    if path != workspace.join(FIX_PATCH_PATH) {
        return Err(EgolintError::Configuration(
            "fix patch path must use the fixed Egolint report boundary".to_owned(),
        ));
    }
    let mut current = workspace.to_path_buf();
    for component in [".reports", "egolint"] {
        current.push(component);
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|source| EgolintError::Filesystem {
                path: current.clone(),
                source,
            })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(EgolintError::Configuration(format!(
                "fix report boundary contains a non-directory or link: {}",
                current.display()
            )));
        }
        let canonical = current
            .canonicalize()
            .map_err(|source| EgolintError::Filesystem {
                path: current.clone(),
                source,
            })?;
        if canonical != current {
            return Err(EgolintError::Configuration(format!(
                "fix report boundary contains a canonical alias: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn read_regular_file_bounded(path: &Path, maximum_bytes: usize) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| EgolintError::Filesystem {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(EgolintError::Configuration(format!(
            "reviewed patch must be a regular file, not a link: {}",
            path.display()
        )));
    }
    if metadata.len() > maximum_bytes as u64 {
        return Err(EgolintError::Configuration(format!(
            "reviewed patch exceeds the {maximum_bytes}-byte limit"
        )));
    }
    let mut contents = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(maximum_bytes));
    OpenOptions::new()
        .read(true)
        .open(path)
        .and_then(|file| {
            file.take((maximum_bytes + 1) as u64)
                .read_to_end(&mut contents)
        })
        .map_err(|source| EgolintError::Filesystem {
            path: path.to_path_buf(),
            source,
        })?;
    if contents.len() > maximum_bytes {
        return Err(EgolintError::Configuration(format!(
            "reviewed patch grew beyond the {maximum_bytes}-byte limit"
        )));
    }
    Ok(contents)
}

fn write_bytes_atomic(contents: &[u8], path: &Path) -> Result<()> {
    let (path, parent) = crate::sarif::validated_report_target(path)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(&parent).map_err(|source| EgolintError::Filesystem {
            path: parent.clone(),
            source,
        })?;
    temporary
        .as_file_mut()
        .write_all(contents)
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|source| EgolintError::Filesystem {
            path: path.clone(),
            source,
        })?;
    let (revalidated_path, revalidated_parent) = crate::sarif::validated_report_target(&path)?;
    if revalidated_path != path || revalidated_parent != parent {
        return Err(EgolintError::RuntimeExecution(
            "fix patch destination changed before persistence".to_owned(),
        ));
    }
    temporary
        .persist(&path)
        .map_err(|error| EgolintError::Filesystem {
            path,
            source: error.error,
        })?;
    Ok(())
}

fn sha256_hex(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(EgolintError::Configuration(
            "reviewed patch SHA-256 must contain 64 lowercase hexadecimal characters".to_owned(),
        ))
    }
}

fn validate_git_oid_syntax(value: &str, name: &str) -> Result<()> {
    if matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(EgolintError::Configuration(format!(
            "{name} must be a full lowercase SHA-1 or SHA-256 Git object id"
        )))
    }
}

fn validate_oid(value: &str, object_format: &str, name: &str) -> Result<()> {
    validate_git_oid_syntax(value, name)?;
    let expected = if object_format == "sha256" { 64 } else { 40 };
    if value.len() == expected {
        Ok(())
    } else {
        Err(EgolintError::Configuration(format!(
            "{name} must contain {expected} lowercase hexadecimal characters for {object_format}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed_repository() -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::write(directory.path().join(".gitignore"), ".env\n.reports/\n")
            .expect("gitignore");
        std::fs::write(directory.path().join("example.txt"), "before\n").expect("tracked file");
        initialize_repository(directory.path(), "sha1", &[]).expect("repository initialization");
        directory
    }

    #[test]
    fn dirty_or_non_repository_workspaces_are_rejected() {
        let directory = tempfile::tempdir().expect("temporary directory");
        assert!(require_clean_repository_root(directory.path()).is_err());
    }

    #[test]
    fn report_only_status_is_allowed_but_other_status_is_dirty() {
        assert!(!status_records_outside_report(
            b"?? .reports/egolint/fixes.patch\0"
        ));
        assert!(status_records_outside_report(
            b" M .reports/egolint/tracked.json\0"
        ));
        assert!(status_records_outside_report(b" M src/lib.rs\0"));
    }

    #[test]
    fn bounded_stderr_removes_binary_control_characters() {
        assert_eq!(bounded_stderr(b"bad\0value\nnext"), "badvalue\nnext");
    }

    #[test]
    fn sha256_contract_uses_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(validate_sha256(&sha256_hex(b"abc")).is_ok());
    }

    #[test]
    fn immutable_materialization_never_copies_ignored_worktree_secrets() {
        let repository = committed_repository();
        std::fs::write(repository.path().join(".env"), "IGNORED_SECRET=value\n")
            .expect("ignored live secret");
        let commit = require_clean_repository_root(repository.path()).expect("clean repository");
        let destination = tempfile::tempdir().expect("destination parent");
        let snapshot = destination.path().join("snapshot");

        materialize_commit_tree(repository.path(), &snapshot, &commit, "sha1")
            .expect("immutable materialization");

        assert_eq!(
            std::fs::read_to_string(snapshot.join("example.txt")).expect("snapshot file"),
            "before\n"
        );
        assert!(!snapshot.join(".env").exists());
    }

    #[test]
    fn candidate_copy_excludes_every_report_subtree() {
        let source = tempfile::tempdir().expect("source");
        let destination = tempfile::tempdir().expect("destination parent");
        let destination = destination.path().join("copy");
        std::fs::create_dir_all(source.path().join(".reports/untrusted"))
            .expect("report directory");
        std::fs::write(
            source.path().join(".reports/untrusted/private.log"),
            "secret",
        )
        .expect("private report");
        std::fs::write(source.path().join("kept.txt"), "kept").expect("ordinary candidate file");
        std::fs::create_dir(&destination).expect("copy destination");

        let mut budget = CopyBudget::default();
        copy_directory(source.path(), &destination, Path::new(""), &mut budget)
            .expect("candidate copy");

        assert_eq!(
            std::fs::read_to_string(destination.join("kept.txt")).expect("kept file"),
            "kept"
        );
        assert!(!destination.join(".reports").exists());
    }

    #[test]
    fn reviewed_apply_is_bound_to_digest_base_and_post_tree() {
        let repository = committed_repository();
        let patch = b"diff --git a/example.txt b/example.txt\n--- a/example.txt\n+++ b/example.txt\n@@ -1 +1 @@\n-before\n+after\n";
        let base = require_clean_repository_root(repository.path()).expect("base commit");
        apply_patch(repository.path(), patch, false, false).expect("reference apply");
        let post_tree = write_tree(repository.path(), "sha1").expect("post tree");
        apply_patch(repository.path(), patch, false, true).expect("reference rollback");
        let report = repository.path().join(".reports/egolint");
        std::fs::create_dir_all(&report).expect("report directory");
        std::fs::write(report.join("fixes.patch"), patch).expect("reviewed patch");

        apply_reviewed_fix(repository.path(), &sha256_hex(patch), &base, &post_tree)
            .expect("reviewed apply");

        assert_eq!(
            std::fs::read_to_string(repository.path().join("example.txt")).expect("applied file"),
            "after\n"
        );
        assert_eq!(
            write_tree(repository.path(), "sha1").expect("observed post tree"),
            post_tree
        );
    }

    #[test]
    fn reviewed_apply_rejects_hidden_assume_unchanged_mutation() {
        let repository = committed_repository();
        std::fs::write(repository.path().join("hidden.txt"), "trusted\n")
            .expect("second tracked file");
        run_git_checked(
            repository.path(),
            &[
                OsStr::new("add"),
                OsStr::new("--"),
                OsStr::new("hidden.txt"),
            ],
        )
        .expect("stage second file");
        run_git_checked(
            repository.path(),
            &[
                OsStr::new("-c"),
                OsStr::new("user.name=Egolint"),
                OsStr::new("-c"),
                OsStr::new("user.email=egolint@invalid.example"),
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("--no-gpg-sign"),
                OsStr::new("--message"),
                OsStr::new("Add hidden-state fixture"),
            ],
        )
        .expect("commit second file");

        let patch = b"diff --git a/example.txt b/example.txt\n--- a/example.txt\n+++ b/example.txt\n@@ -1 +1 @@\n-before\n+after\n";
        let base = require_clean_repository_root(repository.path()).expect("base commit");
        apply_patch(repository.path(), patch, false, false).expect("reference apply");
        let post_tree = write_tree(repository.path(), "sha1").expect("post tree");
        apply_patch(repository.path(), patch, false, true).expect("reference rollback");
        let report = repository.path().join(".reports/egolint");
        std::fs::create_dir_all(&report).expect("report directory");
        std::fs::write(report.join("fixes.patch"), patch).expect("reviewed patch");

        run_git_checked(
            repository.path(),
            &[
                OsStr::new("update-index"),
                OsStr::new("--assume-unchanged"),
                OsStr::new("hidden.txt"),
            ],
        )
        .expect("hide tracked path from status");
        std::fs::write(repository.path().join("hidden.txt"), "unreviewed\n")
            .expect("hidden worktree mutation");
        assert_eq!(
            require_clean_repository_root(repository.path()).expect("status is deceptively clean"),
            base
        );

        let error = apply_reviewed_fix(repository.path(), &sha256_hex(patch), &base, &post_tree)
            .expect_err("special index flags must fail closed");

        assert!(error.to_string().contains("special index flags"));
        assert_eq!(
            std::fs::read_to_string(repository.path().join("example.txt"))
                .expect("reviewed path remains unchanged"),
            "before\n"
        );
        assert_eq!(
            std::fs::read_to_string(repository.path().join("hidden.txt"))
                .expect("hidden mutation retained for caller recovery"),
            "unreviewed\n"
        );
    }
}
