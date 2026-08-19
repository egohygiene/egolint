//! Error types and stable process exit categories.

use std::path::PathBuf;

/// Stable process exit codes returned by Egolint itself.
///
/// Wrapped linter status is normalized: adapter `0` maps to [`CLEAN`], adapter
/// `1` maps to [`FINDINGS`], and every other status (including termination by a
/// signal) maps to [`RUNTIME`]. Raw adapter status remains available in the run
/// report and is never returned directly as an undocumented Egolint code.
pub mod exit_code {
    /// Checks completed without blocking findings.
    pub const CLEAN: i32 = 0;
    /// Checks completed and reported blocking findings.
    pub const FINDINGS: i32 = 1;
    /// CLI usage, configuration, schema, or requested-path error.
    pub const CONFIGURATION: i32 = 2;
    /// Container runtime unavailable or wrapped execution failed.
    pub const RUNTIME: i32 = 3;
    /// Egolint filesystem, serialization, or internal failure.
    pub const INTERNAL: i32 = 4;
}

/// Result alias used throughout Egolint.
pub type Result<T> = std::result::Result<T, EgolintError>;

/// Errors whose variants map to Egolint's documented exit-code contract.
#[derive(Debug, thiserror::Error)]
pub enum EgolintError {
    /// Invalid CLI or configuration input.
    #[error("configuration error: {0}")]
    Configuration(String),
    /// A requested file was not available.
    #[error("required path does not exist: {0}")]
    MissingPath(PathBuf),
    /// No usable container runtime could be found.
    #[error("container runtime is unavailable: {0}")]
    RuntimeUnavailable(String),
    /// A runtime command could not be started or inspected.
    #[error("runtime execution failed: {0}")]
    RuntimeExecution(String),
    /// A filesystem operation failed.
    #[error("filesystem operation failed for {path}: {source}")]
    Filesystem {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// A TOML configuration file could not be decoded.
    #[error("invalid TOML configuration at {path}: {source}")]
    Toml {
        /// Invalid configuration path.
        path: PathBuf,
        /// Decoder error.
        source: toml::de::Error,
    },
    /// A repository-local `MegaLinter` configuration could not be decoded.
    #[error("invalid YAML configuration at {path}: {source}")]
    Yaml {
        /// Invalid configuration path.
        path: PathBuf,
        /// Decoder error.
        source: serde_yaml::Error,
    },
    /// A report could not be serialized.
    #[error("report serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl EgolintError {
    /// Return the stable process exit code for this failure category.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Configuration(_)
            | Self::MissingPath(_)
            | Self::Toml { .. }
            | Self::Yaml { .. } => exit_code::CONFIGURATION,
            Self::RuntimeUnavailable(_) | Self::RuntimeExecution(_) => exit_code::RUNTIME,
            Self::Filesystem { .. } | Self::Serialization(_) => exit_code::INTERNAL,
        }
    }
}
