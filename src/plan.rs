//! Safe container execution-plan construction.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus};

use schemars::JsonSchema;
use serde::Serialize;

use crate::config::{CONFIG_VERSION, Profile, ResolvedConfig, Runtime};
use crate::error::{EgolintError, Result};

const CONTAINER_WORKSPACE: &str = "/tmp/lint";
/// Fixed workspace-relative location for all engine and Egolint reports.
pub const REPORT_DIRECTORY: &str = ".reports/egolint";

/// Requested lint operation and its workspace write capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Analyze without granting broad workspace writes.
    Check,
    /// Explicitly allow tools to update workspace files.
    Fix,
}

/// Per-run options that do not belong in persistent configuration.
#[derive(Debug, Clone, Default)]
pub struct PlanOptions {
    /// Force changed-file behavior independently of the named profile.
    pub changed_only: bool,
    /// Explicit `MegaLinter` linter selections.
    pub enable_linters: Vec<String>,
    /// Explicit `MegaLinter` linter exclusions.
    pub disable_linters: Vec<String>,
}

/// Public, redacted execution plan suitable for JSON output.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PlanView {
    /// Output contract version.
    pub schema_version: u32,
    /// Check or fix capability.
    pub operation: Operation,
    /// Selected named profile.
    pub profile: Profile,
    /// Container runtime executable.
    pub runtime: String,
    /// Immutable or tagged image reference requested by configuration.
    pub image: String,
    /// Canonical host workspace.
    pub workspace: PathBuf,
    /// Workspace-relative report directory.
    pub report_directory: PathBuf,
    /// Ordered configuration provenance.
    pub config_sources: Vec<String>,
    /// Redacted argument vector. This is evidence, not shell syntax.
    pub argv: Vec<String>,
}

/// Internal plan containing both public evidence and unredacted argv.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Shareable plan view.
    pub view: PlanView,
    argv: Vec<OsString>,
    report_path: PathBuf,
}

impl ExecutionPlan {
    /// Build a safe plan without invoking a shell.
    ///
    /// # Errors
    ///
    /// Returns an error when configuration is unsafe, a path escapes the
    /// workspace, a requested file is unavailable, or no required runtime is
    /// installed.
    pub fn build(
        workspace: &Path,
        resolved: &ResolvedConfig,
        operation: Operation,
        options: &PlanOptions,
        require_runtime: bool,
    ) -> Result<Self> {
        let workspace = canonical_directory(workspace)?;
        if resolved.config.config_version != CONFIG_VERSION {
            return Err(EgolintError::Configuration(format!(
                "config-version must equal {CONFIG_VERSION}"
            )));
        }
        crate::config::validate_image_reference(&resolved.config.image)?;
        crate::config::validate_adapter_environment(&resolved.config.environment)?;
        let runtime = resolve_runtime(resolved.config.runtime, require_runtime)?;
        let report_relative = PathBuf::from(REPORT_DIRECTORY);
        let report_path = workspace.join(&report_relative);
        validate_report_directory(&workspace, &report_path)?;
        let megalinter_config = resolve_megalinter_config(
            &workspace,
            resolved.config.megalinter_config.as_deref(),
            resolved.config.profile,
        )?;

        let container_report = container_relative_path(&report_relative, "report-directory")?;
        let environment = build_environment(
            resolved,
            operation,
            options,
            megalinter_config,
            &container_report,
        )?;
        let workspace_mount = bind_mount(
            &workspace,
            CONTAINER_WORKSPACE,
            operation == Operation::Check,
        )?;
        let report_mount = bind_mount(
            &report_path,
            &format!("{CONTAINER_WORKSPACE}/{container_report}"),
            false,
        )?;
        let argv = build_runtime_argv(
            runtime,
            resolved,
            workspace_mount,
            report_mount,
            &environment,
        );

        let redacted = redact_argv(&argv);
        let view = PlanView {
            schema_version: 1,
            operation,
            profile: resolved.config.profile,
            runtime: runtime.to_owned(),
            image: resolved.config.image.clone(),
            workspace,
            report_directory: report_relative,
            config_sources: resolved.sources.clone(),
            argv: redacted,
        };

        Ok(Self {
            view,
            argv,
            report_path,
        })
    }

    /// Execute the exact argv plan and return its status.
    ///
    /// # Errors
    ///
    /// Returns an error when the report directory cannot be prepared safely or
    /// when the selected container runtime cannot be started.
    pub fn execute(&self) -> Result<ExitStatus> {
        validate_report_directory(&self.view.workspace, &self.report_path)?;
        std::fs::create_dir_all(&self.report_path).map_err(|source| EgolintError::Filesystem {
            path: self.report_path.clone(),
            source,
        })?;
        validate_report_directory(&self.view.workspace, &self.report_path)?;

        let executable = self
            .argv
            .first()
            .ok_or_else(|| EgolintError::RuntimeExecution("empty execution plan".to_owned()))?;
        let mut process = Command::new(executable);
        process.args(&self.argv[1..]);
        process
            .status()
            .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))
    }

    /// Return the fixed host report directory used by this plan.
    #[must_use]
    pub fn report_path(&self) -> &Path {
        &self.report_path
    }

    /// Revalidate that the report path has not become a symlink or alias.
    ///
    /// # Errors
    ///
    /// Returns an error when any existing component is not the exact expected
    /// directory beneath the canonical workspace.
    pub fn validate_report_path(&self) -> Result<()> {
        validate_report_directory(&self.view.workspace, &self.report_path)
    }

    /// Check whether the selected runtime daemon is responsive.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime cannot be started or reports an
    /// unhealthy daemon.
    pub fn doctor(&self) -> Result<()> {
        let status = Command::new(&self.view.runtime)
            .arg("info")
            .status()
            .map_err(|error| EgolintError::RuntimeExecution(error.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(EgolintError::RuntimeUnavailable(format!(
                "{} info exited with {}",
                self.view.runtime, status
            )))
        }
    }
}

fn build_environment(
    resolved: &ResolvedConfig,
    operation: Operation,
    options: &PlanOptions,
    megalinter_config: String,
    container_report: &str,
) -> Result<BTreeMap<String, String>> {
    let mut environment = resolved.config.environment.clone();
    environment.insert(
        "GITHUB_WORKSPACE".to_owned(),
        CONTAINER_WORKSPACE.to_owned(),
    );
    environment.insert("MEGALINTER_CONFIG".to_owned(), megalinter_config);
    environment.insert(
        "REPORT_OUTPUT_FOLDER".to_owned(),
        format!("{CONTAINER_WORKSPACE}/{container_report}"),
    );
    environment.insert(
        "VALIDATE_ALL_CODEBASE".to_owned(),
        if options.changed_only || resolved.config.profile == Profile::Fast {
            "false"
        } else {
            "true"
        }
        .to_owned(),
    );
    environment.insert(
        "APPLY_FIXES".to_owned(),
        if operation == Operation::Fix {
            "all"
        } else {
            "none"
        }
        .to_owned(),
    );
    if !options.enable_linters.is_empty() {
        environment.insert(
            "ENABLE_LINTERS".to_owned(),
            normalize_linter_list(&options.enable_linters)?,
        );
    }
    if !options.disable_linters.is_empty() {
        environment.insert(
            "DISABLE_LINTERS".to_owned(),
            normalize_linter_list(&options.disable_linters)?,
        );
    }
    Ok(environment)
}

fn build_runtime_argv(
    runtime: &str,
    resolved: &ResolvedConfig,
    workspace_mount: OsString,
    report_mount: OsString,
    environment: &BTreeMap<String, String>,
) -> Vec<OsString> {
    let mut argv = vec![
        OsString::from(runtime),
        OsString::from("run"),
        OsString::from("--rm"),
        OsString::from("--pull"),
        OsString::from(resolved.config.pull_policy.as_str()),
        OsString::from("--network"),
        OsString::from(resolved.config.network.as_str()),
        OsString::from("--cap-drop"),
        OsString::from("ALL"),
        OsString::from("--security-opt"),
        OsString::from("no-new-privileges"),
        OsString::from("--pids-limit"),
        OsString::from("512"),
        OsString::from("--mount"),
        workspace_mount,
        OsString::from("--mount"),
        report_mount,
        OsString::from("--workdir"),
        OsString::from(CONTAINER_WORKSPACE),
    ];
    for (name, value) in environment {
        argv.push(OsString::from("--env"));
        argv.push(OsString::from(format!("{name}={value}")));
    }
    argv.push(OsString::from(&resolved.config.image));
    argv
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Err(EgolintError::MissingPath(path.to_path_buf()));
    }
    path.canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: path.to_path_buf(),
            source,
        })
}

fn validate_report_directory(workspace: &Path, report_path: &Path) -> Result<()> {
    let relative = report_path.strip_prefix(workspace).map_err(|_| {
        EgolintError::Configuration(
            "fixed report directory must remain inside the workspace".to_owned(),
        )
    })?;
    let mut current = workspace.to_path_buf();

    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(EgolintError::Configuration(
                "fixed report directory must be a normalized workspace path".to_owned(),
            ));
        };
        current.push(part);

        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(source) => {
                return Err(EgolintError::Filesystem {
                    path: current,
                    source,
                });
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(EgolintError::Configuration(format!(
                "fixed report directory may not contain symlinks: {}",
                current.display()
            )));
        }
        if !metadata.is_dir() {
            return Err(EgolintError::Configuration(format!(
                "fixed report path component must be a directory: {}",
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
                "fixed report directory may not use a canonical alias: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn resolve_runtime(runtime: Runtime, require_runtime: bool) -> Result<&'static str> {
    if let Some(executable) = runtime.executable() {
        if !require_runtime || executable_in_path(executable) {
            return Ok(executable);
        }
        return Err(EgolintError::RuntimeUnavailable(executable.to_owned()));
    }
    for executable in ["docker", "podman"] {
        if executable_in_path(executable) {
            return Ok(executable);
        }
    }
    if require_runtime {
        Err(EgolintError::RuntimeUnavailable(
            "neither docker nor podman was found".to_owned(),
        ))
    } else {
        Ok("docker")
    }
}

fn executable_in_path(executable: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|directory| {
        let candidate = directory.join(executable);
        if candidate.is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            return directory.join(format!("{executable}.exe")).is_file();
        }
        #[cfg(not(windows))]
        false
    })
}

fn resolve_megalinter_config(
    workspace: &Path,
    explicit: Option<&Path>,
    profile: Profile,
) -> Result<String> {
    let Some(explicit) = explicit else {
        return Ok(profile.image_config().to_owned());
    };
    let relative = safe_relative_path(explicit, "megalinter-config")?;
    let candidate = workspace.join(&relative);
    if !candidate.is_file() {
        return Err(EgolintError::MissingPath(candidate));
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: candidate.clone(),
            source,
        })?;
    let relative = canonical.strip_prefix(workspace).map_err(|_| {
        EgolintError::Configuration(
            "MegaLinter configuration must remain inside the workspace".to_owned(),
        )
    })?;
    crate::config::validate_megalinter_config(workspace, &canonical)?;
    Ok(format!(
        "{CONTAINER_WORKSPACE}/{}",
        container_relative_path(relative, "megalinter-config")?
    ))
}

fn safe_relative_path(path: &Path, name: &str) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(EgolintError::Configuration(format!(
            "{name} must be a non-empty path relative to the workspace"
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(EgolintError::Configuration(format!(
                    "{name} must remain inside the workspace"
                )));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(EgolintError::Configuration(format!(
            "{name} may not resolve to the workspace root"
        )));
    }
    Ok(normalized)
}

fn container_relative_path(path: &Path, name: &str) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(EgolintError::Configuration(format!(
                "{name} is not a normalized relative path"
            )));
        };
        let part = part.to_str().ok_or_else(|| {
            EgolintError::Configuration(format!("{name} must contain valid Unicode"))
        })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn bind_mount(source: &Path, target: &str, read_only: bool) -> Result<OsString> {
    let source = source.to_str().ok_or_else(|| {
        EgolintError::Configuration("container mount paths must contain valid Unicode".to_owned())
    })?;
    if source.contains(',') || target.contains(',') {
        return Err(EgolintError::Configuration(
            "container mount paths may not contain commas".to_owned(),
        ));
    }
    let mut value = format!("type=bind,source={source},target={target}");
    if read_only {
        value.push_str(",readonly");
    }
    Ok(OsString::from(value))
}

fn normalize_linter_list(values: &[String]) -> Result<String> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_ascii_uppercase();
        if value.is_empty()
            || !value
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric())
        {
            return Err(EgolintError::Configuration(format!(
                "invalid MegaLinter identifier: {value}"
            )));
        }
        normalized.push(value);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized.join(","))
}

fn redact_argv(argv: &[OsString]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(argv.len());
    let mut redact_next = false;
    for argument in argv {
        let value = argument.to_string_lossy();
        if redact_next {
            let name = value.split_once('=').map_or("<redacted>", |(name, _)| name);
            redacted.push(format!("{name}=<redacted>"));
            redact_next = false;
        } else {
            redact_next = value == "--env";
            redacted.push(value.into_owned());
        }
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ResolvedConfig};

    fn resolved() -> ResolvedConfig {
        ResolvedConfig {
            config: Config {
                runtime: Runtime::Docker,
                ..Config::default()
            },
            sources: vec!["test".to_owned()],
        }
    }

    fn mount_for_target<'a>(plan: &'a ExecutionPlan, target: &str) -> &'a str {
        plan.view
            .argv
            .iter()
            .find(|argument| {
                argument.starts_with("type=bind,") && argument.contains(&format!("target={target}"))
            })
            .map(String::as_str)
            .expect("mount argument")
    }

    fn environment_value<'a>(plan: &'a ExecutionPlan, name: &str) -> &'a str {
        let prefix = format!("{name}=");
        plan.argv
            .iter()
            .filter_map(|argument| argument.to_str())
            .find(|argument| argument.starts_with(&prefix))
            .expect("environment argument")
    }

    #[test]
    fn check_plan_uses_read_only_workspace_and_redacts_environment() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let plan = ExecutionPlan::build(
            directory.path(),
            &resolved(),
            Operation::Check,
            &PlanOptions::default(),
            false,
        )
        .expect("execution plan");
        assert!(mount_for_target(&plan, CONTAINER_WORKSPACE).ends_with(",readonly"));
        assert!(!mount_for_target(&plan, "/tmp/lint/.reports/egolint").ends_with(",readonly"));
        assert_eq!(environment_value(&plan, "APPLY_FIXES"), "APPLY_FIXES=none");
        assert!(
            plan.view
                .argv
                .iter()
                .any(|part| part == "MEGALINTER_CONFIG=<redacted>")
        );
    }

    #[test]
    fn fix_plan_grants_explicit_workspace_writes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let plan = ExecutionPlan::build(
            directory.path(),
            &resolved(),
            Operation::Fix,
            &PlanOptions::default(),
            false,
        )
        .expect("execution plan");
        assert!(!mount_for_target(&plan, CONTAINER_WORKSPACE).ends_with(",readonly"));
        assert_eq!(environment_value(&plan, "APPLY_FIXES"), "APPLY_FIXES=all");
    }

    #[test]
    fn untrusted_environment_is_one_argv_value_and_public_plan_is_redacted() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let secret = "$(touch /tmp/egolint-must-not-run); secret value";
        let mut resolved = resolved();
        resolved
            .config
            .environment
            .insert("EXAMPLE_TOKEN".to_owned(), secret.to_owned());

        let plan = ExecutionPlan::build(
            directory.path(),
            &resolved,
            Operation::Check,
            &PlanOptions::default(),
            false,
        )
        .expect("execution plan");

        assert_eq!(
            environment_value(&plan, "EXAMPLE_TOKEN"),
            format!("EXAMPLE_TOKEN={secret}")
        );
        assert!(!plan.argv.iter().any(|argument| argument == "sh"));
        assert!(!plan.argv.iter().any(|argument| argument == "-c"));
        assert!(
            plan.view
                .argv
                .iter()
                .any(|argument| argument == "EXAMPLE_TOKEN=<redacted>")
        );
        assert!(
            !plan
                .view
                .argv
                .iter()
                .any(|argument| argument.contains(secret))
        );
    }

    #[test]
    fn manual_config_cannot_inject_runtime_options_or_control_environment() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut option_injection = resolved();
        option_injection.config.image = "--privileged".to_owned();
        assert!(
            ExecutionPlan::build(
                directory.path(),
                &option_injection,
                Operation::Check,
                &PlanOptions::default(),
                false,
            )
            .is_err()
        );

        let mut control_override = resolved();
        control_override
            .config
            .environment
            .insert("APPLY_FIXES".to_owned(), "all".to_owned());
        assert!(
            ExecutionPlan::build(
                directory.path(),
                &control_override,
                Operation::Check,
                &PlanOptions::default(),
                false,
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn internal_report_directory_symlink_is_rejected_during_plan_build() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(directory.path().join("src")).expect("source directory");
        std::fs::create_dir(directory.path().join(".reports")).expect("report parent");
        symlink("../src", directory.path().join(".reports/egolint")).expect("report link");

        assert!(
            ExecutionPlan::build(
                directory.path(),
                &resolved(),
                Operation::Check,
                &PlanOptions::default(),
                false,
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn internal_report_parent_symlink_is_rejected_during_plan_build() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir_all(directory.path().join("report-root/egolint"))
            .expect("aliased report directory");
        symlink("report-root", directory.path().join(".reports")).expect("report parent link");

        assert!(
            ExecutionPlan::build(
                directory.path(),
                &resolved(),
                Operation::Check,
                &PlanOptions::default(),
                false,
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn megalinter_config_symlink_cannot_escape_workspace() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        let outside = tempfile::NamedTempFile::new().expect("outside configuration");
        symlink(outside.path(), directory.path().join("override.yml")).expect("configuration link");
        let mut resolved = resolved();
        resolved.config.megalinter_config = Some(PathBuf::from("override.yml"));

        assert!(
            ExecutionPlan::build(
                directory.path(),
                &resolved,
                Operation::Check,
                &PlanOptions::default(),
                false,
            )
            .is_err()
        );
    }
}
