//! Native, versioned Egolint rule packs.

pub mod inventory;
pub mod portability;
pub mod repository_contract;
pub mod suppressions;

pub use inventory::{RepositoryEntry, RepositoryEntryKind, RepositoryInventory};
pub use portability::{PortabilityRuleDefinition, PortabilityRuleSet};
pub use repository_contract::{
    ContractRequirement, ContractSource, RepositoryContract, RepositoryContractEvaluator,
    RequirementKind, RequirementOwnership, SourceRevisionKind,
};
pub use suppressions::apply_suppressions;
