//! Core configuration, planning, and execution primitives for Egolint.

pub mod config;
pub mod error;
pub mod plan;
pub mod report;

pub use config::{
    Config, ConfigOverrides, NetworkMode, Profile, PullPolicy, ResolvedConfig, Runtime,
};
pub use error::{EgolintError, Result};
pub use plan::{ExecutionPlan, Operation, PlanOptions};
pub use report::{RunReport, RunStatus};
