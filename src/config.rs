//! Deterministic configuration discovery and layering.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Component, Path, PathBuf};

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{EgolintError, Result};

/// Current configuration contract version.
pub const CONFIG_VERSION: u32 = 1;

const ENVIRONMENT_OVERRIDE_NAMES: [&str; 6] = [
    "EGOLINT_IMAGE",
    "EGOLINT_PROFILE",
    "EGOLINT_RUNTIME",
    "EGOLINT_PULL_POLICY",
    "EGOLINT_NETWORK",
    "EGOLINT_MEGALINTER_CONFIG",
];

const RESERVED_ADAPTER_ENVIRONMENT_NAMES: [&str; 7] = [
    "APPLY_FIXES",
    "DISABLE_LINTERS",
    "ENABLE_LINTERS",
    "GITHUB_WORKSPACE",
    "MEGALINTER_CONFIG",
    "REPORT_OUTPUT_FOLDER",
    "VALIDATE_ALL_CODEBASE",
];

/// Built-in lint profiles.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Small, deterministic changed-file feedback surface.
    #[default]
    Fast,
    /// Complete repository and security inspection.
    Holistic,
}

impl Profile {
    /// Return the configuration path embedded in `egolint-full`.
    #[must_use]
    pub const fn image_config(self) -> &'static str {
        match self {
            Self::Fast => "/opt/egolint/profiles/fast.yml",
            Self::Holistic => "/opt/egolint/profiles/holistic.yml",
        }
    }
}

/// Supported container runtimes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    /// Prefer Docker, then Podman.
    #[default]
    Auto,
    /// Use Docker.
    Docker,
    /// Use Podman.
    Podman,
}

impl Runtime {
    /// Return the executable name for an explicit runtime.
    #[must_use]
    pub const fn executable(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Docker => Some("docker"),
            Self::Podman => Some("podman"),
        }
    }
}

/// Container image pull behavior.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum PullPolicy {
    /// Pull only when the image is absent.
    #[default]
    Missing,
    /// Always request the current tag.
    Always,
    /// Never access a registry.
    Never,
}

impl PullPolicy {
    /// Return the Docker/Podman command value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

/// Network access granted to a lint container.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    /// Disable network access.
    #[default]
    None,
    /// Use the runtime's default bridge network.
    Bridge,
}

impl NetworkMode {
    /// Return the runtime command value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bridge => "bridge",
        }
    }
}

/// Fully resolved Egolint configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
#[schemars(transform = require_config_version)]
pub struct Config {
    /// Configuration contract version.
    #[schemars(schema_with = "config_version_schema")]
    pub config_version: u32,
    /// Default profile when no CLI override is supplied.
    pub profile: Profile,
    /// Container runtime selection.
    pub runtime: Runtime,
    /// Full Egolint image reference.
    pub image: String,
    /// Image pull behavior.
    pub pull_policy: PullPolicy,
    /// Container network mode.
    pub network: NetworkMode,
    /// Optional repository-local `MegaLinter` configuration file.
    pub megalinter_config: Option<PathBuf>,
    /// Explicit, non-secret environment additions for the adapter.
    pub environment: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            profile: Profile::Fast,
            runtime: Runtime::Auto,
            image: "ghcr.io/egohygiene/egolint-full:edge".to_owned(),
            pull_policy: PullPolicy::Missing,
            network: NetworkMode::None,
            megalinter_config: None,
            environment: BTreeMap::new(),
        }
    }
}

/// Optional values contributed by the command line.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// Profile override.
    pub profile: Option<Profile>,
    /// Runtime override.
    pub runtime: Option<Runtime>,
    /// Image override.
    pub image: Option<String>,
    /// Pull-policy override.
    pub pull_policy: Option<PullPolicy>,
    /// Network-mode override.
    pub network: Option<NetworkMode>,
    /// Repository-local `MegaLinter` configuration override.
    pub megalinter_config: Option<PathBuf>,
}

/// Resolved configuration plus its ordered provenance.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Effective values.
    pub config: Config,
    /// Applied sources in increasing precedence order.
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct ConfigLayer {
    config_version: Option<u32>,
    profile: Option<Profile>,
    runtime: Option<Runtime>,
    image: Option<String>,
    pull_policy: Option<PullPolicy>,
    network: Option<NetworkMode>,
    megalinter_config: Option<PathBuf>,
    environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct ResolutionEnvironment {
    ci: bool,
    user_config_path: Option<PathBuf>,
    overrides: BTreeMap<&'static str, String>,
}

impl ResolutionEnvironment {
    fn from_process() -> Result<Self> {
        let mut overrides = BTreeMap::new();
        for name in ENVIRONMENT_OVERRIDE_NAMES {
            match env::var(name) {
                Ok(value) => {
                    overrides.insert(name, value);
                }
                Err(env::VarError::NotPresent) => {}
                Err(env::VarError::NotUnicode(_)) => {
                    return Err(EgolintError::Configuration(format!(
                        "{name} must contain valid Unicode"
                    )));
                }
            }
        }

        Ok(Self {
            ci: env::var_os("CI").is_some_and(|value| environment_value_truthy(&value)),
            user_config_path: user_config_path(),
            overrides,
        })
    }
}

impl Config {
    /// Resolve configuration in this fixed precedence order: compiled defaults,
    /// user, `egolint.toml`, `.egolint.toml`, local, explicit file, environment,
    /// and CLI. CI suppresses the implicit user and local layers; an explicitly
    /// requested file remains explicit even when it has a local-style name.
    ///
    /// # Errors
    ///
    /// Returns an error when a source cannot be read or decoded, a requested
    /// path escapes the workspace, an override uses a protected adapter key, or
    /// the resulting configuration violates the versioned contract.
    pub fn resolve(
        workspace: &Path,
        explicit: Option<&Path>,
        overrides: &ConfigOverrides,
    ) -> Result<ResolvedConfig> {
        let environment = ResolutionEnvironment::from_process()?;
        Self::resolve_with_environment(workspace, explicit, overrides, &environment)
    }

    fn resolve_with_environment(
        workspace: &Path,
        explicit: Option<&Path>,
        overrides: &ConfigOverrides,
        environment: &ResolutionEnvironment,
    ) -> Result<ResolvedConfig> {
        let workspace = canonical_workspace(workspace)?;
        let mut config = Self::default();
        let mut sources = vec!["compiled defaults".to_owned()];

        if !environment.ci {
            if let Some(user_path) = &environment.user_config_path {
                if user_path.is_file() {
                    apply_file(&mut config, user_path)?;
                    sources.push(user_path.display().to_string());
                }
            }
        }

        for repository_path in [
            workspace.join("egolint.toml"),
            workspace.join(".egolint.toml"),
        ] {
            if repository_path.is_file() {
                apply_file(&mut config, &repository_path)?;
                sources.push(repository_path.display().to_string());
            }
        }

        let local_path = workspace.join(".egolint.local.toml");
        if !environment.ci && local_path.is_file() {
            apply_file(&mut config, &local_path)?;
            sources.push(local_path.display().to_string());
        }

        if let Some(path) = explicit {
            let explicit_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace.join(path)
            };
            if !explicit_path.is_file() {
                return Err(EgolintError::MissingPath(explicit_path));
            }
            apply_file(&mut config, &explicit_path)?;
            sources.push(explicit_path.display().to_string());
        }

        apply_environment(&mut config, &environment.overrides)?;
        if !environment.overrides.is_empty() {
            sources.push("EGOLINT_* environment".to_owned());
        }
        apply_overrides(&mut config, overrides);
        if overrides_present(overrides) {
            sources.push("command line".to_owned());
        }
        validate_and_normalize(&workspace, &mut config)?;

        Ok(ResolvedConfig { config, sources })
    }
}

fn apply_file(config: &mut Config, path: &Path) -> Result<()> {
    let contents = std::fs::read_to_string(path).map_err(|source| EgolintError::Filesystem {
        path: path.to_path_buf(),
        source,
    })?;
    let layer: ConfigLayer = toml::from_str(&contents).map_err(|source| EgolintError::Toml {
        path: path.to_path_buf(),
        source,
    })?;
    if layer.config_version != Some(CONFIG_VERSION) {
        return Err(EgolintError::Configuration(format!(
            "{} must declare config-version = {CONFIG_VERSION}",
            path.display()
        )));
    }
    apply_layer(config, layer);
    Ok(())
}

fn apply_layer(config: &mut Config, layer: ConfigLayer) {
    if let Some(value) = layer.profile {
        config.profile = value;
    }
    if let Some(value) = layer.runtime {
        config.runtime = value;
    }
    if let Some(value) = layer.image {
        config.image = value;
    }
    if let Some(value) = layer.pull_policy {
        config.pull_policy = value;
    }
    if let Some(value) = layer.network {
        config.network = value;
    }
    if let Some(value) = layer.megalinter_config {
        config.megalinter_config = Some(value);
    }
    config.environment.extend(layer.environment);
}

fn apply_environment(
    config: &mut Config,
    environment: &BTreeMap<&'static str, String>,
) -> Result<()> {
    if let Some(value) = environment.get("EGOLINT_IMAGE") {
        config.image.clone_from(value);
    }
    if let Some(value) = environment.get("EGOLINT_PROFILE") {
        config.profile = parse_environment_enum("EGOLINT_PROFILE", value, &["fast", "holistic"])?;
    }
    if let Some(value) = environment.get("EGOLINT_RUNTIME") {
        config.runtime =
            parse_environment_enum("EGOLINT_RUNTIME", value, &["auto", "docker", "podman"])?;
    }
    if let Some(value) = environment.get("EGOLINT_PULL_POLICY") {
        config.pull_policy = parse_environment_enum(
            "EGOLINT_PULL_POLICY",
            value,
            &["missing", "always", "never"],
        )?;
    }
    if let Some(value) = environment.get("EGOLINT_NETWORK") {
        config.network = parse_environment_enum("EGOLINT_NETWORK", value, &["none", "bridge"])?;
    }
    if let Some(value) = environment.get("EGOLINT_MEGALINTER_CONFIG") {
        config.megalinter_config = Some(PathBuf::from(value));
    }
    Ok(())
}

fn parse_environment_enum<T>(name: &str, value: &str, variants: &[&str]) -> Result<T>
where
    T: ValueEnum,
{
    T::from_str(value, true).map_err(|_| {
        EgolintError::Configuration(format!("{name} must be one of: {}", variants.join(", ")))
    })
}

fn apply_overrides(config: &mut Config, overrides: &ConfigOverrides) {
    if let Some(value) = overrides.profile {
        config.profile = value;
    }
    if let Some(value) = overrides.runtime {
        config.runtime = value;
    }
    if let Some(value) = &overrides.image {
        config.image.clone_from(value);
    }
    if let Some(value) = overrides.pull_policy {
        config.pull_policy = value;
    }
    if let Some(value) = overrides.network {
        config.network = value;
    }
    if let Some(value) = &overrides.megalinter_config {
        config.megalinter_config = Some(value.clone());
    }
}

fn validate_and_normalize(workspace: &Path, config: &mut Config) -> Result<()> {
    if config.config_version != CONFIG_VERSION {
        return Err(EgolintError::Configuration(format!(
            "config-version must equal {CONFIG_VERSION}"
        )));
    }
    validate_image_reference(&config.image)?;

    if let Some(path) = &config.megalinter_config {
        let relative = resolve_repository_file(workspace, path, "megalinter-config")?;
        validate_megalinter_config(workspace, &workspace.join(&relative))?;
        config.megalinter_config = Some(relative);
    }

    validate_adapter_environment(&config.environment)?;
    Ok(())
}

pub(crate) fn validate_image_reference(image: &str) -> Result<()> {
    if image.trim().is_empty() {
        return Err(EgolintError::Configuration(
            "image cannot be empty".to_owned(),
        ));
    }
    if image.starts_with('-')
        || image
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(EgolintError::Configuration(
            "image must be one container-image argv value and may not begin with '-'".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_adapter_environment(environment: &BTreeMap<String, String>) -> Result<()> {
    for (name, value) in environment {
        if !valid_environment_name(name) {
            return Err(EgolintError::Configuration(format!(
                "invalid environment variable name: {name}"
            )));
        }
        if RESERVED_ADAPTER_ENVIRONMENT_NAMES
            .iter()
            .any(|reserved| name.eq_ignore_ascii_case(reserved))
            || dangerous_megalinter_key(name)
        {
            return Err(EgolintError::Configuration(format!(
                "{name} is controlled by Egolint and cannot be overridden through environment"
            )));
        }
        if value.contains('\0') {
            return Err(EgolintError::Configuration(format!(
                "environment variable {name} contains a NUL byte"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_megalinter_config(workspace: &Path, path: &Path) -> Result<()> {
    let workspace = canonical_workspace(workspace)?;
    let path = path
        .canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: path.to_path_buf(),
            source,
        })?;
    if !path.starts_with(&workspace) {
        return Err(EgolintError::Configuration(
            "MegaLinter configuration must remain inside the workspace".to_owned(),
        ));
    }
    validate_megalinter_config_inner(&workspace, &path, &mut BTreeSet::new())
}

fn validate_megalinter_config_inner(
    workspace: &Path,
    path: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if !visited.insert(path.to_path_buf()) {
        return Err(EgolintError::Configuration(format!(
            "MegaLinter EXTENDS cycle detected at {}",
            path.display()
        )));
    }
    let contents = std::fs::read_to_string(path).map_err(|source| EgolintError::Filesystem {
        path: path.to_path_buf(),
        source,
    })?;
    let value: serde_yaml::Value =
        serde_yaml::from_str(&contents).map_err(|source| EgolintError::Yaml {
            path: path.to_path_buf(),
            source,
        })?;
    let mapping = value.as_mapping().ok_or_else(|| {
        EgolintError::Configuration(format!(
            "{} must contain one top-level YAML mapping",
            path.display()
        ))
    })?;
    for key in mapping.keys() {
        let key = key.as_str().ok_or_else(|| {
            EgolintError::Configuration(format!(
                "{} contains a non-string top-level key",
                path.display()
            ))
        })?;
        if key == "<<" || dangerous_megalinter_key(key) {
            return Err(EgolintError::Configuration(format!(
                "{} contains prohibited MegaLinter key {key}",
                path.display()
            )));
        }
    }
    if let Some(extends) = mapping.get(serde_yaml::Value::String("EXTENDS".to_owned())) {
        let mut values = Vec::new();
        match extends {
            serde_yaml::Value::String(value) => values.push(value.as_str()),
            serde_yaml::Value::Sequence(sequence) => {
                for value in sequence {
                    values.push(value.as_str().ok_or_else(|| {
                        EgolintError::Configuration(format!(
                            "{} contains a non-string EXTENDS entry",
                            path.display()
                        ))
                    })?);
                }
            }
            _ => {
                return Err(EgolintError::Configuration(format!(
                    "{} EXTENDS must be a string or list of strings",
                    path.display()
                )));
            }
        }
        for value in values {
            validate_megalinter_extension(workspace, path, value, visited)?;
        }
    }
    visited.remove(path);
    Ok(())
}

fn validate_megalinter_extension(
    workspace: &Path,
    including: &Path,
    value: &str,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let extension = Path::new(value);
    if value.contains("://") || value.contains('\\') {
        return Err(EgolintError::Configuration(format!(
            "{} may extend only repository-local files using Linux path separators",
            including.display()
        )));
    }
    let relative = normalize_relative_path(extension, "MegaLinter EXTENDS")?;
    let candidate = workspace.join(relative);
    if !candidate.is_file() {
        return Err(EgolintError::MissingPath(candidate));
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: candidate.clone(),
            source,
        })?;
    if !canonical.starts_with(workspace) {
        return Err(EgolintError::Configuration(format!(
            "MegaLinter EXTENDS target {} resolves outside the workspace",
            candidate.display()
        )));
    }
    validate_megalinter_config_inner(workspace, &canonical, visited)
}

fn dangerous_megalinter_key(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    matches!(name.as_str(), "PLUGINS" | "PRE_COMMANDS" | "POST_COMMANDS")
        || name.ends_with("_PRE_COMMANDS")
        || name.ends_with("_POST_COMMANDS")
}

fn canonical_workspace(workspace: &Path) -> Result<PathBuf> {
    if !workspace.is_dir() {
        return Err(EgolintError::MissingPath(workspace.to_path_buf()));
    }
    workspace
        .canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: workspace.to_path_buf(),
            source,
        })
}

fn normalize_relative_path(path: &Path, name: &str) -> Result<PathBuf> {
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

fn resolve_repository_file(workspace: &Path, path: &Path, name: &str) -> Result<PathBuf> {
    let normalized = normalize_relative_path(path, name)?;
    let candidate = workspace.join(&normalized);
    if !candidate.is_file() {
        return Err(EgolintError::MissingPath(candidate));
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: candidate.clone(),
            source,
        })?;
    canonical
        .strip_prefix(workspace)
        .map(Path::to_path_buf)
        .map_err(|_| EgolintError::Configuration(format!("{name} resolves outside the workspace")))
}

fn valid_environment_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn user_config_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("egolint/config.toml"))
}

fn environment_value_truthy(value: &std::ffi::OsStr) -> bool {
    value.to_str().is_some_and(|value| {
        !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    })
}

fn overrides_present(overrides: &ConfigOverrides) -> bool {
    overrides.profile.is_some()
        || overrides.runtime.is_some()
        || overrides.image.is_some()
        || overrides.pull_policy.is_some()
        || overrides.network.is_some()
        || overrides.megalinter_config.is_some()
}

fn config_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "const": CONFIG_VERSION
    })
}

fn require_config_version(schema: &mut schemars::Schema) {
    schema.insert("required".to_owned(), serde_json::json!(["config-version"]));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(path: &Path, body: &str) {
        std::fs::write(path, format!("config-version = 1\n{body}"))
            .expect("write test configuration");
    }

    fn resolve_for_test(
        workspace: &Path,
        explicit: Option<&Path>,
        overrides: &ConfigOverrides,
        environment: &ResolutionEnvironment,
    ) -> Result<ResolvedConfig> {
        Config::resolve_with_environment(workspace, explicit, overrides, environment)
    }

    #[test]
    fn precedence_is_defaults_user_repository_local_explicit_environment_cli() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path();
        let user = repository.join("user.toml");
        write_config(
            &user,
            "image = \"example.invalid/user\"\n[environment]\nA = \"user\"\n",
        );
        write_config(
            &repository.join("egolint.toml"),
            "image = \"example.invalid/repository\"\n[environment]\nA = \"repository\"\n",
        );
        write_config(
            &repository.join(".egolint.toml"),
            "image = \"example.invalid/hidden\"\n[environment]\nB = \"hidden\"\n",
        );
        write_config(
            &repository.join(".egolint.local.toml"),
            "image = \"example.invalid/local\"\n[environment]\nC = \"local\"\n",
        );
        write_config(
            &repository.join("explicit.toml"),
            "image = \"example.invalid/explicit\"\n[environment]\nD = \"explicit\"\n",
        );

        let environment = ResolutionEnvironment {
            ci: false,
            user_config_path: Some(user),
            overrides: BTreeMap::from([(
                "EGOLINT_IMAGE",
                "example.invalid/environment".to_owned(),
            )]),
        };
        let overrides = ConfigOverrides {
            image: Some("example.invalid/cli".to_owned()),
            ..ConfigOverrides::default()
        };
        let resolved = resolve_for_test(
            repository,
            Some(Path::new("explicit.toml")),
            &overrides,
            &environment,
        )
        .expect("resolved configuration");

        assert_eq!(resolved.config.image, "example.invalid/cli");
        assert_eq!(
            resolved.config.environment.get("A").map(String::as_str),
            Some("repository")
        );
        assert_eq!(
            resolved.config.environment.get("B").map(String::as_str),
            Some("hidden")
        );
        assert_eq!(
            resolved.config.environment.get("C").map(String::as_str),
            Some("local")
        );
        assert_eq!(
            resolved.config.environment.get("D").map(String::as_str),
            Some("explicit")
        );
        assert_eq!(
            resolved.sources,
            vec![
                "compiled defaults".to_owned(),
                repository.join("user.toml").display().to_string(),
                repository
                    .canonicalize()
                    .expect("canonical workspace")
                    .join("egolint.toml")
                    .display()
                    .to_string(),
                repository
                    .canonicalize()
                    .expect("canonical workspace")
                    .join(".egolint.toml")
                    .display()
                    .to_string(),
                repository
                    .canonicalize()
                    .expect("canonical workspace")
                    .join(".egolint.local.toml")
                    .display()
                    .to_string(),
                repository
                    .canonicalize()
                    .expect("canonical workspace")
                    .join("explicit.toml")
                    .display()
                    .to_string(),
                "EGOLINT_* environment".to_owned(),
                "command line".to_owned(),
            ]
        );
    }

    #[test]
    fn ci_suppresses_user_and_local_configuration() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path();
        let user = repository.join("user.toml");
        write_config(&user, "image = \"example.invalid/user\"\n");
        write_config(
            &repository.join("egolint.toml"),
            "image = \"example.invalid/repository\"\n",
        );
        write_config(
            &repository.join(".egolint.local.toml"),
            "image = \"example.invalid/local\"\n",
        );
        let environment = ResolutionEnvironment {
            ci: true,
            user_config_path: Some(user),
            overrides: BTreeMap::new(),
        };

        let resolved =
            resolve_for_test(repository, None, &ConfigOverrides::default(), &environment)
                .expect("resolved CI configuration");

        assert_eq!(resolved.config.image, "example.invalid/repository");
        assert_eq!(resolved.sources.len(), 2);
        assert!(resolved.sources[1].ends_with("egolint.toml"));
    }

    #[test]
    fn cli_megalinter_config_overrides_repository_value() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path();
        std::fs::write(repository.join("base.yml"), "---\n{}\n").expect("base profile");
        std::fs::write(repository.join("override.yml"), "---\n{}\n").expect("override profile");
        write_config(
            &repository.join("egolint.toml"),
            "megalinter-config = \"base.yml\"\n",
        );
        let overrides = ConfigOverrides {
            megalinter_config: Some(PathBuf::from("override.yml")),
            ..ConfigOverrides::default()
        };

        let resolved = resolve_for_test(
            repository,
            None,
            &overrides,
            &ResolutionEnvironment::default(),
        )
        .expect("resolved configuration");

        assert_eq!(
            resolved.config.megalinter_config,
            Some(PathBuf::from("override.yml"))
        );
    }

    #[test]
    fn megalinter_config_cannot_traverse_the_workspace_mount() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut config = Config {
            megalinter_config: Some(PathBuf::from("../outside.yml")),
            ..Config::default()
        };
        assert!(validate_and_normalize(directory.path(), &mut config).is_err());
    }

    #[test]
    fn report_directory_is_not_a_user_configuration_field() {
        let directory = tempfile::tempdir().expect("temporary directory");
        write_config(
            &directory.path().join("egolint.toml"),
            "report-directory = \"src\"\n",
        );

        assert!(
            resolve_for_test(
                directory.path(),
                None,
                &ConfigOverrides::default(),
                &ResolutionEnvironment::default(),
            )
            .is_err()
        );
        assert!(!ENVIRONMENT_OVERRIDE_NAMES.contains(&"EGOLINT_REPORT_DIRECTORY"));
    }

    #[test]
    fn config_schema_requires_exact_contract_version_and_has_no_report_override() {
        let schema = serde_json::to_value(schemars::schema_for!(Config)).expect("config schema");
        let required = schema["required"]
            .as_array()
            .expect("required property list");

        assert!(required.iter().any(|value| value == "config-version"));
        assert_eq!(schema["properties"]["config-version"]["const"], 1);
        assert!(schema["properties"].get("report-directory").is_none());
    }

    #[test]
    fn protected_adapter_environment_cannot_be_overridden() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut config = Config {
            environment: BTreeMap::from([("APPLY_FIXES".to_owned(), "all".to_owned())]),
            ..Config::default()
        };
        assert!(validate_and_normalize(directory.path(), &mut config).is_err());
    }

    #[test]
    fn repository_megalinter_config_allows_extends_but_rejects_commands_and_plugins() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("override.yml");
        std::fs::write(directory.path().join("base.yml"), "---\n{}\n").expect("base policy");
        std::fs::write(
            &path,
            "---\nEXTENDS: base.yml\nENABLE_LINTERS:\n  - RUST_CLIPPY\n",
        )
        .expect("safe override");
        validate_megalinter_config(directory.path(), &path).expect("safe MegaLinter override");

        for dangerous in [
            "PRE_COMMANDS:\n  - command: echo unsafe\n",
            "POST_COMMANDS:\n  - command: echo unsafe\n",
            "PLUGINS:\n  - https://example.invalid/plugin.yml\n",
            "PYTHON_RUFF_PRE_COMMANDS:\n  - command: echo unsafe\n",
            "PYTHON_RUFF_POST_COMMANDS:\n  - command: echo unsafe\n",
            "<<: { PRE_COMMANDS: [] }\n",
        ] {
            std::fs::write(&path, format!("---\n{dangerous}")).expect("dangerous override");
            assert!(
                validate_megalinter_config(directory.path(), &path).is_err(),
                "{dangerous}"
            );
        }
    }

    #[test]
    fn megalinter_extends_is_recursive_and_cannot_fetch_or_escape() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("root.yml");
        let nested = directory.path().join("nested.yml");

        std::fs::write(&nested, "---\nPRE_COMMANDS: []\n").expect("nested override");
        std::fs::write(&root, "---\nEXTENDS: nested.yml\n").expect("root override");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());

        std::fs::write(
            &root,
            "---\nEXTENDS: https://example.invalid/untrusted.yml\n",
        )
        .expect("remote override");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());

        std::fs::write(&root, "---\nEXTENDS: /opt/egolint/profiles/holistic.yml\n")
            .expect("absolute override");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());

        std::fs::write(&root, "---\nEXTENDS: ../outside.yml\n").expect("escaping override");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());

        std::fs::write(&root, "---\nEXTENDS: nested\\policy.yml\n").expect("backslash override");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());

        std::fs::write(&nested, "---\nEXTENDS: root.yml\n").expect("nested cycle");
        std::fs::write(&root, "---\nEXTENDS: nested.yml\n").expect("root cycle");
        assert!(validate_megalinter_config(directory.path(), &root).is_err());
    }

    #[test]
    fn megalinter_extends_matches_workspace_root_runtime_resolution() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path();
        let policies = repository.join("policies");
        std::fs::create_dir(&policies).expect("policy directory");
        let override_path = policies.join("override.yml");
        std::fs::write(&override_path, "---\nEXTENDS: base.yml\n").expect("override policy");
        std::fs::write(policies.join("base.yml"), "---\n{}\n").expect("nested collision");
        std::fs::write(repository.join("base.yml"), "---\nPRE_COMMANDS: []\n")
            .expect("runtime-selected unsafe policy");

        assert!(validate_megalinter_config(repository, &override_path).is_err());

        std::fs::write(repository.join("base.yml"), "---\n{}\n")
            .expect("runtime-selected safe policy");
        std::fs::write(policies.join("base.yml"), "---\nPRE_COMMANDS: []\n")
            .expect("unused nested collision");

        validate_megalinter_config(repository, &override_path)
            .expect("validator and MegaLinter both select the workspace-root file");
    }
}
