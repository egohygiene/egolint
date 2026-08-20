//! Core configuration, planning, and execution primitives for Egolint.

pub mod config;
pub mod contracts;
pub mod debt;
pub mod error;
pub mod fix;
pub mod megalinter;
pub mod plan;
pub mod report;
pub mod rules;
pub mod sarif;

pub use config::{
    Config, ConfigOverrides, NetworkMode, Profile, PullPolicy, ResolvedConfig, Runtime,
};
pub use contracts::{
    CONTRACT_VERSION, Enforcement, EvidenceKind, EvidenceReference, Finding, ProfileDefinition,
    ProfileScope, RuleIdentity, RuleOwnership, Severity, SourceLocation, Suppression,
    SuppressionState, ToolResult, ToolStatus, validate_contract_date,
};
pub use error::{EgolintError, Result};
pub use fix::{FIX_PATCH_PATH, FixOutcome, apply_reviewed_fix, run_isolated_fix};
pub use megalinter::{NormalizedMegaLinter, normalize_workspace};
pub use plan::{ExecutionPlan, Operation, PlanOptions};
pub use report::{ReportCompleteness, ReportSummary, RunReport, RunStatus};
