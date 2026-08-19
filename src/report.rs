//! Stable run-report contract for Relay and Observatory consumers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::Serialize;

use crate::error::{EgolintError, Result, exit_code};
use crate::plan::{Operation, PlanView};

/// Normalized outcome independent of any individual linter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// All blocking checks passed.
    Clean,
    /// The wrapped lint engine reported findings.
    Findings,
    /// The engine could not complete normally.
    ExecutionError,
}

impl RunStatus {
    /// Normalize a wrapped adapter status into the stable Egolint contract.
    #[must_use]
    pub const fn from_adapter_exit_code(exit_code: Option<i32>) -> Self {
        match exit_code {
            Some(exit_code::CLEAN) => Self::Clean,
            Some(exit_code::FINDINGS) => Self::Findings,
            _ => Self::ExecutionError,
        }
    }

    /// Return the stable Egolint process code for this outcome.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Clean => exit_code::CLEAN,
            Self::Findings => exit_code::FINDINGS,
            Self::ExecutionError => exit_code::RUNTIME,
        }
    }
}

/// Minimal versioned report written beside raw adapter output.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RunReport {
    /// Output contract version.
    pub schema_version: u32,
    /// Unix timestamp in seconds.
    pub generated_at_unix: u64,
    /// Operation executed.
    pub operation: Operation,
    /// Normalized outcome.
    pub status: RunStatus,
    /// Stable Egolint process exit code.
    pub egolint_exit_code: i32,
    /// Raw wrapped-adapter exit code when available.
    pub adapter_exit_code: Option<i32>,
    /// Requested container image.
    pub image: String,
    /// Selected profile name.
    pub profile: String,
    /// Configuration provenance.
    pub config_sources: Vec<String>,
    /// Workspace-relative location of the report itself.
    pub report_path: PathBuf,
}

impl RunReport {
    /// Construct a report from a public plan and process status.
    #[must_use]
    pub fn from_plan(plan: &PlanView, adapter_exit_code: Option<i32>) -> Self {
        let status = RunStatus::from_adapter_exit_code(adapter_exit_code);
        Self {
            schema_version: 1,
            generated_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
            operation: plan.operation,
            status,
            egolint_exit_code: status.exit_code(),
            adapter_exit_code,
            image: plan.image.clone(),
            profile: format!("{:?}", plan.profile).to_ascii_lowercase(),
            config_sources: plan.config_sources.clone(),
            report_path: plan.report_directory.join("run.json"),
        }
    }

    /// Atomically persist this report without following an existing target
    /// symlink. The temporary file is created beside the destination so the
    /// final replacement stays on one filesystem.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization, durable temporary-file writing, or
    /// the final atomic replacement fails.
    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path.parent().ok_or_else(|| {
            EgolintError::Configuration("run report path must have a parent directory".to_owned())
        })?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|source| EgolintError::Filesystem {
                path: parent.to_path_buf(),
                source,
            })?;
        serde_json::to_writer_pretty(temporary.as_file_mut(), self)?;
        temporary
            .as_file_mut()
            .write_all(b"\n")
            .map_err(|source| EgolintError::Filesystem {
                path: path.to_path_buf(),
                source,
            })?;
        temporary
            .as_file_mut()
            .sync_all()
            .map_err(|source| EgolintError::Filesystem {
                path: path.to_path_buf(),
                source,
            })?;
        temporary
            .persist(path)
            .map_err(|error| EgolintError::Filesystem {
                path: path.to_path_buf(),
                source: error.error,
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

    #[test]
    fn adapter_exit_codes_are_normalized_without_losing_raw_status() {
        for (raw, expected_status, expected_egolint) in [
            (Some(0), RunStatus::Clean, exit_code::CLEAN),
            (Some(1), RunStatus::Findings, exit_code::FINDINGS),
            (Some(2), RunStatus::ExecutionError, exit_code::RUNTIME),
            (Some(127), RunStatus::ExecutionError, exit_code::RUNTIME),
            (None, RunStatus::ExecutionError, exit_code::RUNTIME),
        ] {
            let status = RunStatus::from_adapter_exit_code(raw);
            assert_eq!(status, expected_status);
            assert_eq!(status.exit_code(), expected_egolint);
        }
    }

    #[test]
    fn run_report_preserves_raw_adapter_status_and_stable_egolint_status() {
        let plan = PlanView {
            schema_version: 1,
            operation: Operation::Check,
            profile: Profile::Fast,
            runtime: "docker".to_owned(),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            workspace: PathBuf::from("/workspace"),
            report_directory: PathBuf::from(".reports/egolint"),
            config_sources: vec!["compiled defaults".to_owned()],
            argv: vec!["docker".to_owned(), "run".to_owned()],
        };

        let report = RunReport::from_plan(&plan, Some(42));

        assert_eq!(report.status, RunStatus::ExecutionError);
        assert_eq!(report.egolint_exit_code, exit_code::RUNTIME);
        assert_eq!(report.adapter_exit_code, Some(42));
    }

    #[test]
    fn atomic_report_write_replaces_an_existing_regular_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("run.json");
        std::fs::write(&path, "stale").expect("stale report");

        let report = RunReport::from_plan(&plan(), Some(0));
        report.write_atomic(&path).expect("atomic report write");

        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("persisted report"))
                .expect("valid report JSON");
        assert_eq!(value["status"], "clean");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_report_write_replaces_symlink_without_overwriting_its_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        let outside = tempfile::NamedTempFile::new().expect("outside target");
        std::fs::write(outside.path(), "sentinel").expect("outside sentinel");
        let path = directory.path().join("run.json");
        symlink(outside.path(), &path).expect("malicious report link");

        RunReport::from_plan(&plan(), Some(0))
            .write_atomic(&path)
            .expect("atomic report write");

        assert_eq!(
            std::fs::read_to_string(outside.path()).expect("outside target"),
            "sentinel"
        );
        assert!(
            !std::fs::symlink_metadata(&path)
                .expect("report metadata")
                .file_type()
                .is_symlink()
        );
    }

    fn plan() -> PlanView {
        PlanView {
            schema_version: 1,
            operation: Operation::Check,
            profile: Profile::Fast,
            runtime: "docker".to_owned(),
            image: "example.invalid/egolint@sha256:abc".to_owned(),
            workspace: PathBuf::from("/workspace"),
            report_directory: PathBuf::from(".reports/egolint"),
            config_sources: vec!["compiled defaults".to_owned()],
            argv: vec!["docker".to_owned(), "run".to_owned()],
        }
    }
}
