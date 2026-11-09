//! Versioned, metadata-only capability discovery and deterministic selection.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Profile;
use crate::error::{EgolintError, Result};

const MATRIX: &str = include_str!("../.config/megalinter/tool-matrix.json");
const CATALOG: &str = include_str!("../.config/megalinter/v10-catalog.json");
const COMPLEMENTARY: &str = include_str!("../.config/toolchain/complementary-tools.json");
const MAX_FILES: usize = 100_000;

/// Repository overrides may narrow the registry; declarations do not authorize shell commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CapabilityConfig {
    /// Disable adaptation explicitly to retain the original profile selection.
    pub enabled: bool,
    /// Exact capability IDs to exclude in addition to organization policy.
    pub disable: BTreeSet<String>,
    /// Explicitly authorized optional capabilities, never executable text.
    pub declarations: BTreeSet<String>,
    /// Named services made available by the caller; no endpoint is probed.
    pub services: BTreeSet<String>,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            disable: BTreeSet::new(),
            declarations: BTreeSet::new(),
            services: BTreeSet::new(),
        }
    }
}

/// A capability's owning execution boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    FullImage,
    Native,
    HostAdapter,
    ConfigurationOnly,
    SpecializedImage,
}

/// Closed applicability predicates; no repository script is evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    /// At least one inventoried regular file matches a portable glob.
    Path { pattern: String },
    /// A bounded JSON/TOML manifest contains a nonempty top-level metadata key.
    Metadata { pattern: String, key: String },
    /// Caller declared a named service.
    Service { id: String },
    /// Caller authorized this optional capability.
    Declaration { id: String },
    /// The observed caller event must match; this grants no external authority.
    Lifecycle {
        event: crate::enforcement::Invocation,
    },
    /// A required host runtime must be available without executing it.
    Runtime { executable: String },
    /// Release work needs both an explicit lifecycle and a locally verified comparison.
    ReleaseComparison,
    /// All nested requirements must hold.
    All { predicates: Vec<Self> },
    /// At least one nested requirement must hold.
    Any { predicates: Vec<Self> },
    /// Safe repository-wide metadata/static inspection.
    Repository,
}

/// A capability declaration describes availability separately from applicability.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDefinition {
    pub id: String,
    pub version: String,
    pub provider: String,
    pub placement: Placement,
    pub predicate: Predicate,
    pub profiles: BTreeSet<String>,
    pub policy_source: String,
    pub runtime: Option<String>,
}

/// Exact compiled registry assembled from existing catalog and policy owners.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRegistry {
    #[schemars(schema_with = "crate::contracts::contract_version_schema")]
    pub schema_version: u32,
    pub policy_sha256: String,
    pub capabilities: Vec<CapabilityDefinition>,
}

/// Selection is not evidence that an executable actually ran or passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    Applicable,
    Selected,
    Skipped,
    Unavailable,
    Invalid,
}

/// Public-safe explanation; never contains manifest values or file contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDecision {
    pub id: String,
    pub version: String,
    pub provider: String,
    pub placement: Placement,
    pub available: bool,
    pub applicable: bool,
    pub state: CapabilityState,
    pub reason: String,
}

/// Invocation facts are supplied by the caller, not inferred from installed scanners.
#[derive(Debug, Clone, Default)]
pub struct CapabilityContext {
    pub invocation: crate::enforcement::Invocation,
    pub paths: BTreeSet<String>,
    pub declarations: BTreeSet<String>,
    pub services: BTreeSet<String>,
    pub release_comparison: bool,
    pub network_allowed: bool,
}

impl CapabilityContext {
    /// Enumerate Git-visible regular paths, or a bounded non-Git directory tree.
    /// Symlinks and cache directories never become source inputs.
    /// # Errors
    /// Returns an error for unsafe paths, unreadable metadata or inventory limits.
    pub fn discover(workspace: &Path, config: &CapabilityConfig) -> Result<Self> {
        let output = Command::new("git")
            .args([
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ])
            .current_dir(workspace)
            .output();
        let mut paths = BTreeSet::new();
        if let Some(output) = output.ok().filter(|output| output.status.success()) {
            for raw in output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|raw| !raw.is_empty())
            {
                let path = std::str::from_utf8(raw)
                    .map_err(|_| invalid("capability paths must be UTF-8"))?;
                validate_path(path)?;
                if is_source_path(path)
                    && workspace
                        .join(path)
                        .symlink_metadata()
                        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
                {
                    paths.insert(path.to_owned());
                }
                if paths.len() > MAX_FILES {
                    return Err(invalid("capability inventory exceeds 100000 files"));
                }
            }
        } else {
            collect_paths(workspace, Path::new(""), &mut paths)?;
        }
        Ok(Self {
            paths,
            declarations: config.declarations.clone(),
            services: config.services.clone(),
            ..Self::default()
        })
    }
}

impl CapabilityRegistry {
    /// Existing policy/matrix remain authoritative for tool versions and profiles.
    /// # Errors
    /// Returns an error when compiled policy/catalog data cannot be decoded.
    pub fn built_in() -> Result<Self> {
        let matrix: serde_json::Value = serde_json::from_str(MATRIX)?;
        let catalog: serde_json::Value = serde_json::from_str(CATALOG)?;
        let complementary: serde_json::Value = serde_json::from_str(COMPLEMENTARY)?;
        let mut capabilities = Vec::new();
        for tool in matrix["tools"]
            .as_array()
            .ok_or_else(|| invalid("invalid bundled tool matrix"))?
        {
            let id = tool["id"]
                .as_str()
                .ok_or_else(|| invalid("bundled tool has no id"))?;
            let raw = &catalog["tools"][id];
            let profiles = tool["profiles"]
                .as_object()
                .into_iter()
                .flat_map(|map| map.iter())
                .filter(|(_, state)| state.as_str() == Some("selected"))
                .map(|(name, _)| name.clone())
                .collect();
            capabilities.push(CapabilityDefinition {
                id: id.to_owned(),
                version: tool["version"].as_str().unwrap_or("unknown").to_owned(),
                provider: "megalinter".to_owned(),
                placement: Placement::FullImage,
                predicate: tool_predicate(id, raw, tool),
                profiles,
                policy_source: ".config/megalinter/policy.yml".to_owned(),
                runtime: None,
            });
        }
        for (id, predicate) in [
            ("EGOLINT_PORTABILITY", Predicate::Repository),
            (
                "EGOLINT_REPOSITORY_RELEASE",
                path(".egohygiene/release.json"),
            ),
            (
                "EGOLINT_REPOSITORY_CONTRACT",
                declared("EGOLINT_REPOSITORY_CONTRACT"),
            ),
            (
                "EGOLINT_REPOSITORY_CONTINUITY",
                declared("EGOLINT_REPOSITORY_CONTINUITY"),
            ),
            (
                "EGOLINT_REPOSITORY_INTELLIGENCE",
                declared("EGOLINT_REPOSITORY_INTELLIGENCE"),
            ),
            (
                "EGOLINT_REPOSITORY_PRESENTATION",
                declared("EGOLINT_REPOSITORY_PRESENTATION"),
            ),
        ] {
            capabilities.push(CapabilityDefinition {
                id: id.to_owned(),
                version: "1".to_owned(),
                provider: "egolint".to_owned(),
                placement: Placement::Native,
                predicate,
                profiles: all_profiles(),
                policy_source: "egolint/native/v1".to_owned(),
                runtime: None,
            });
        }
        append_complementary(&mut capabilities, &complementary);
        capabilities.push(CapabilityDefinition {
            id: "AST_GREP".to_owned(),
            version: crate::structural::VERSION.to_owned(),
            provider: "egolint".to_owned(),
            placement: Placement::HostAdapter,
            predicate: Predicate::All {
                predicates: vec![
                    declared("AST_GREP"),
                    Predicate::Any {
                        predicates: vec![
                            path("**/*.js"),
                            path("**/*.ts"),
                            path("**/*.py"),
                            path("**/*.rs"),
                        ],
                    },
                ],
            },
            profiles: all_profiles(),
            policy_source: "egolint/structural/v1".to_owned(),
            runtime: Some("ast-grep".to_owned()),
        });
        capabilities.sort_by(|a, b| a.id.cmp(&b.id));
        let mut hash = Sha256::new();
        hash.update(serde_json::to_vec(&capabilities)?);
        hash.update(MATRIX);
        hash.update(CATALOG);
        hash.update(COMPLEMENTARY);
        Ok(Self {
            schema_version: 1,
            policy_sha256: format!("{:x}", hash.finalize()),
            capabilities,
        })
    }

    /// Decide every capability without invoking a runtime or active scanner.
    /// # Errors
    /// Returns an error for unknown overrides or malformed required project metadata.
    pub fn decide(
        &self,
        workspace: &Path,
        config: &CapabilityConfig,
        context: &CapabilityContext,
        profile: Profile,
        enable: &[String],
        disable: &[String],
    ) -> Result<Vec<CapabilityDecision>> {
        let ids = self
            .capabilities
            .iter()
            .map(|c| c.id.as_str())
            .collect::<BTreeSet<_>>();
        for id in config
            .disable
            .iter()
            .chain(config.declarations.iter())
            .chain(enable.iter())
            .chain(disable.iter())
        {
            if !ids.contains(id.as_str()) {
                return Err(invalid("unknown capability override"));
            }
        }
        self.capabilities
            .iter()
            .map(|capability| {
                let disabled_by_configuration =
                    config.disable.contains(&capability.id) || disable.contains(&capability.id);
                let disabled_by_profile = !capability.profiles.contains(profile.as_str())
                    && !enable.contains(&capability.id);
                // Disabled tools must not read unrelated project manifests. Applicability
                // is evaluated only for capabilities admitted by the selected policy.
                let applicable = !disabled_by_configuration
                    && !disabled_by_profile
                    && matches_predicate(&capability.predicate, workspace, context)?;
                let available = match capability.placement {
                    Placement::ConfigurationOnly | Placement::SpecializedImage => false,
                    Placement::HostAdapter => capability
                        .runtime
                        .as_deref()
                        .is_some_and(executable_available),
                    _ => true,
                };
                let (state, reason) = if disabled_by_configuration {
                    (CapabilityState::Skipped, "disabled_by_configuration")
                } else if disabled_by_profile {
                    (CapabilityState::Skipped, "disabled_by_profile")
                } else if !applicable {
                    (
                        CapabilityState::Skipped,
                        "applicability_predicate_not_satisfied",
                    )
                } else if !available {
                    (
                        CapabilityState::Unavailable,
                        "not_available_in_this_execution_boundary",
                    )
                } else {
                    (
                        CapabilityState::Selected,
                        "applicable_and_allowed_by_policy",
                    )
                };
                Ok(CapabilityDecision {
                    id: capability.id.clone(),
                    version: capability.version.clone(),
                    provider: capability.provider.clone(),
                    placement: capability.placement,
                    available,
                    applicable,
                    state,
                    reason: reason.to_owned(),
                })
            })
            .collect()
    }
}

fn append_complementary(
    capabilities: &mut Vec<CapabilityDefinition>,
    complementary: &serde_json::Value,
) {
    for tool in complementary["tools"].as_array().into_iter().flatten() {
        let Some(id) = tool["id"].as_str() else {
            continue;
        };
        // Development integrations are catalogued, never launched as arbitrary commands.
        capabilities.push(CapabilityDefinition {
            id: format!("complementary:{id}"),
            version: tool["version"].as_str().unwrap_or("unknown").to_owned(),
            provider: "complementary".to_owned(),
            placement: Placement::ConfigurationOnly,
            predicate: declared(&format!("complementary:{id}")),
            profiles: all_profiles(),
            policy_source: ".config/toolchain/complementary-tools.json".to_owned(),
            runtime: tool["runtime"].as_str().map(str::to_owned),
        });
    }
}

fn tool_predicate(id: &str, raw: &serde_json::Value, policy: &serde_json::Value) -> Predicate {
    let descriptor = raw["descriptor"].as_str().unwrap_or("");
    let mut predicates = match descriptor {
        "ACTION" => vec![
            path(".github/workflows/*.yml"),
            path(".github/workflows/*.yaml"),
        ],
        "ANSIBLE" => vec![
            path("**/playbooks/*.yml"),
            path("**/playbooks/*.yaml"),
            path("**/roles/**/*.yml"),
            path("ansible/**/*.yml"),
        ],
        "DOCKERFILE" => vec![
            path("**/Dockerfile"),
            path("**/Dockerfile.*"),
            path("**/*.Dockerfile"),
            path("**/Containerfile"),
        ],
        _ => raw["file_extensions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
            .map(|extension| path(&format!("**/*{extension}")))
            .collect(),
    };
    for pattern in policy["applicability"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
    {
        if pattern.contains('*') || pattern.contains('/') {
            predicates.push(path(pattern));
        }
    }
    if predicates.is_empty() {
        // A manifest-only or repository-level tool is not a license to execute unrelated runtimes.
        predicates = match descriptor {
            "REPOSITORY" | "COPYPASTE" | "SPELL" => vec![Predicate::Repository],
            "JSON" => vec![path("**/*.json")],
            "YAML" => vec![path("**/*.yml"), path("**/*.yaml")],
            "MARKDOWN" => vec![path("**/*.md")],
            _ => vec![declared(id)],
        };
    }
    let mut predicate = Predicate::Any { predicates };
    if raw["cli_lint_mode"].as_str() == Some("project") {
        let metadata = match descriptor {
            "RUST" => Some(Predicate::Any {
                predicates: vec![
                    metadata("**/Cargo.toml", "package"),
                    metadata("**/Cargo.toml", "workspace"),
                ],
            }),
            "GO" => Some(path("**/go.mod")),
            "JAVASCRIPT" | "TYPESCRIPT" | "TSX" | "JSX" => {
                Some(metadata("**/package.json", "name"))
            }
            "JAVA" => Some(Predicate::Any {
                predicates: vec![
                    path("**/pom.xml"),
                    path("**/build.gradle"),
                    path("**/build.gradle.kts"),
                ],
            }),
            "CSHARP" => Some(path("**/*.csproj")),
            "PHP" => Some(metadata("**/composer.json", "name")),
            _ => None,
        };
        if let Some(metadata) = metadata {
            predicate = Predicate::All {
                predicates: vec![predicate, metadata],
            };
        }
    }
    if [
        "MARKDOWN_MARKDOWN_LINK_CHECK",
        "MARKDOWN_LYCHEE",
        "JSON_V8R",
        "YAML_V8R",
    ]
    .contains(&id)
    {
        predicate = Predicate::All {
            predicates: vec![
                predicate,
                declared(id),
                Predicate::Service {
                    id: "external-http".to_owned(),
                },
            ],
        };
    }
    predicate
}

fn matches_predicate(
    predicate: &Predicate,
    workspace: &Path,
    context: &CapabilityContext,
) -> Result<bool> {
    Ok(match predicate {
        Predicate::Repository => true,
        Predicate::Path { pattern } => context.paths.iter().any(|p| glob_matches(pattern, p)),
        Predicate::Declaration { id } => context.declarations.contains(id),
        Predicate::Lifecycle { event } => context.invocation == *event,
        Predicate::Runtime { executable } => executable_available(executable),
        Predicate::Service { id } => context.network_allowed && context.services.contains(id),
        Predicate::ReleaseComparison => context.release_comparison,
        Predicate::All { predicates } => {
            let mut matched = true;
            for p in predicates {
                if !matches_predicate(p, workspace, context)? {
                    matched = false;
                    break;
                }
            }
            matched
        }
        Predicate::Any { predicates } => {
            let mut matched = false;
            let mut failure = None;
            for p in predicates {
                match matches_predicate(p, workspace, context) {
                    Ok(true) => {
                        matched = true;
                        break;
                    }
                    Ok(false) => {}
                    Err(error) => failure = Some(error),
                }
            }
            if !matched {
                if let Some(error) = failure {
                    return Err(error);
                }
            }
            matched
        }
        Predicate::Metadata { pattern, key } => {
            let mut present = false;
            for path in context.paths.iter().filter(|p| glob_matches(pattern, p)) {
                let absolute = workspace.join(path);
                let meta =
                    absolute
                        .symlink_metadata()
                        .map_err(|source| EgolintError::Filesystem {
                            path: absolute.clone(),
                            source,
                        })?;
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 2 * 1024 * 1024
                {
                    return Err(invalid("project metadata must be a bounded regular file"));
                }
                let text = std::fs::read_to_string(&absolute).map_err(|source| {
                    EgolintError::Filesystem {
                        path: absolute,
                        source,
                    }
                })?;
                let document: serde_json::Value = if Path::new(path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
                {
                    let parsed: toml::Value = toml::from_str(&text)
                        .map_err(|_| invalid("invalid project TOML metadata"))?;
                    serde_json::to_value(parsed)?
                } else {
                    serde_json::from_str(&text)
                        .map_err(|_| invalid("invalid project JSON metadata"))?
                };
                if document.get(key).is_some_and(|v| !v.is_null()) {
                    present = true;
                    break;
                }
            }
            present
        }
    })
}

/// Portable bounded glob matcher, including zero-directory `**/` matches.
#[must_use]
pub fn glob_matches(pattern: &str, path: &str) -> bool {
    fn matches(p: &[u8], s: &[u8], memo: &mut BTreeMap<(usize, usize), bool>) -> bool {
        let key = (p.len(), s.len());
        if let Some(value) = memo.get(&key) {
            return *value;
        }
        let result = if p.is_empty() {
            s.is_empty()
        } else if p.starts_with(b"**/") {
            matches(&p[3..], s, memo)
                || s.iter()
                    .position(|byte| *byte == b'/')
                    .is_some_and(|index| matches(p, &s[index + 1..], memo))
        } else if p.starts_with(b"**") {
            matches(&p[2..], s, memo) || (!s.is_empty() && matches(p, &s[1..], memo))
        } else if p[0] == b'*' {
            matches(&p[1..], s, memo)
                || (!s.is_empty() && s[0] != b'/' && matches(p, &s[1..], memo))
        } else {
            !s.is_empty()
                && (p[0] == s[0] || (p[0] == b'?' && s[0] != b'/'))
                && matches(&p[1..], &s[1..], memo)
        };
        memo.insert(key, result);
        result
    }
    pattern.len() <= 512
        && path.len() <= 4096
        && matches(pattern.as_bytes(), path.as_bytes(), &mut BTreeMap::new())
}

fn collect_paths(root: &Path, relative: &Path, paths: &mut BTreeSet<String>) -> Result<()> {
    if relative.components().count() > 64 {
        return Err(invalid("capability directory depth exceeds limit"));
    }
    let directory = root.join(relative);
    for entry in std::fs::read_dir(&directory).map_err(|source| EgolintError::Filesystem {
        path: directory,
        source,
    })? {
        let entry = entry.map_err(|_| invalid("cannot read capability inventory"))?;
        let path = relative.join(entry.file_name());
        let name = path
            .to_str()
            .ok_or_else(|| invalid("capability paths must be UTF-8"))?;
        if !is_source_path(name) {
            continue;
        }
        let kind = entry
            .file_type()
            .map_err(|_| invalid("cannot inspect capability entry"))?;
        if kind.is_dir() {
            collect_paths(root, &path, paths)?;
        } else if kind.is_file() {
            paths.insert(name.to_owned());
        }
        if paths.len() > MAX_FILES {
            return Err(invalid("capability inventory exceeds 100000 files"));
        }
    }
    Ok(())
}

fn is_source_path(path: &str) -> bool {
    !path.split('/').any(|part| {
        [
            ".git",
            ".reports",
            "node_modules",
            "target",
            ".venv",
            "__pycache__",
        ]
        .contains(&part)
    })
}
fn validate_path(path: &str) -> Result<()> {
    if PathBuf::from(path).is_absolute() || path.split('/').any(|p| p == "..") {
        return Err(invalid("unsafe capability inventory path"));
    }
    Ok(())
}
fn path(pattern: &str) -> Predicate {
    Predicate::Path {
        pattern: pattern.to_owned(),
    }
}
fn metadata(pattern: &str, key: &str) -> Predicate {
    Predicate::Metadata {
        pattern: pattern.to_owned(),
        key: key.to_owned(),
    }
}
fn declared(id: &str) -> Predicate {
    Predicate::Declaration { id: id.to_owned() }
}
fn all_profiles() -> BTreeSet<String> {
    ["fast", "holistic", "security", "dependency-debt"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}
fn executable_available(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|p| p.join(name).is_file()))
}
fn invalid(message: &str) -> EgolintError {
    EgolintError::Configuration(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn globs_match_root_nested_and_directory_boundaries() {
        assert!(glob_matches("**/*.rs", "src/main.rs"));
        assert!(glob_matches("**/*.rs", "main.rs"));
        assert!(!glob_matches(".github/workflows/*.yml", "unrelated.yml"));
        assert!(!glob_matches("src/*.rs", "src/nested/main.rs"));
        assert!(!glob_matches("**/Cargo.toml", "unrelatedCargo.toml"));
    }
    #[test]
    fn registry_skips_unrelated_tools_and_requires_network_authorization() {
        let root = tempfile::tempdir().unwrap();
        let config = CapabilityConfig::default();
        let mut context = CapabilityContext {
            paths: ["source.py", "README.md", "schema.json"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ..CapabilityContext::default()
        };
        let registry = CapabilityRegistry::built_in().unwrap();
        let decisions = registry
            .decide(root.path(), &config, &context, Profile::Holistic, &[], &[])
            .unwrap();
        assert_eq!(
            decisions
                .iter()
                .find(|d| d.id == "PYTHON_RUFF")
                .unwrap()
                .state,
            CapabilityState::Selected
        );
        assert_eq!(
            decisions
                .iter()
                .find(|d| d.id == "RUST_CLIPPY")
                .unwrap()
                .state,
            CapabilityState::Skipped
        );
        assert_eq!(
            decisions.iter().find(|d| d.id == "JSON_V8R").unwrap().state,
            CapabilityState::Skipped
        );
        context.paths.clear();
        assert!(
            !registry
                .decide(root.path(), &config, &context, Profile::Fast, &[], &[],)
                .unwrap()
                .iter()
                .any(|d| d.id == "PYTHON_RUFF" && d.state == CapabilityState::Selected)
        );
    }

    #[test]
    fn discovery_tracks_evolving_polyglot_sources_and_ignores_generated_trees() {
        let root = tempfile::tempdir().unwrap();
        let config = CapabilityConfig::default();
        let registry = CapabilityRegistry::built_in().unwrap();
        let decisions = || {
            let context = CapabilityContext::discover(root.path(), &config).unwrap();
            registry
                .decide(root.path(), &config, &context, Profile::Holistic, &[], &[])
                .unwrap()
        };
        let selected = |decisions: &[CapabilityDecision], id: &str| {
            decisions
                .iter()
                .any(|d| d.id == id && d.state == CapabilityState::Selected)
        };
        assert!(!selected(&decisions(), "PYTHON_RUFF"));
        for directory in ["target", "node_modules", ".reports/egolint"] {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
            std::fs::write(root.path().join(directory).join("generated.py"), "").unwrap();
        }
        assert!(!selected(&decisions(), "PYTHON_RUFF"));
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/main.py"), "").unwrap();
        assert!(selected(&decisions(), "PYTHON_RUFF"));
        assert!(!selected(&decisions(), "RUST_CLIPPY"));
        std::fs::write(root.path().join("src/main.rs"), "").unwrap();
        assert!(!selected(&decisions(), "RUST_CLIPPY"));
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        let polyglot = decisions();
        assert!(selected(&polyglot, "PYTHON_RUFF"));
        assert!(selected(&polyglot, "RUST_CLIPPY"));
        assert!(!selected(&polyglot, "JSON_V8R"));
    }

    #[test]
    fn project_selection_requires_metadata_and_honors_explicit_exclusions() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        std::fs::create_dir(root.path().join("fixtures")).unwrap();
        std::fs::write(root.path().join("fixtures/Cargo.toml"), "invalid [toml").unwrap();
        let registry = CapabilityRegistry::built_in().unwrap();
        let mut config = CapabilityConfig::default();
        let mut context = CapabilityContext {
            paths: [
                "main.rs",
                "Cargo.toml",
                "fixtures/Cargo.toml",
                "Containerfile",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            ..CapabilityContext::default()
        };
        let select = |config: &CapabilityConfig, context: &CapabilityContext| {
            registry.decide(root.path(), config, context, Profile::Holistic, &[], &[])
        };
        let decisions = select(&config, &context).unwrap();
        for id in ["RUST_CLIPPY", "DOCKERFILE_HADOLINT"] {
            assert_eq!(
                decisions.iter().find(|d| d.id == id).unwrap().state,
                CapabilityState::Selected
            );
        }
        context.paths.remove("Cargo.toml");
        assert!(select(&config, &context).is_err());
        config.disable.insert("RUST_CLIPPY".to_owned());
        let decisions = select(&config, &context).unwrap();
        assert_eq!(
            decisions
                .iter()
                .find(|d| d.id == "RUST_CLIPPY")
                .unwrap()
                .reason,
            "disabled_by_configuration"
        );
        config.disable.clear();
        context.paths.remove("fixtures/Cargo.toml");
        let decisions = select(&config, &context).unwrap();
        assert_eq!(
            decisions
                .iter()
                .find(|d| d.id == "RUST_CLIPPY")
                .unwrap()
                .state,
            CapabilityState::Skipped
        );
    }
}
