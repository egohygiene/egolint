//! Native, versioned Egolint rule packs.

pub mod inventory;
pub mod portability;
pub mod repository_contract;
pub mod repository_intelligence;
pub mod repository_presentation;
pub mod suppressions;

pub use inventory::{RepositoryEntry, RepositoryEntryKind, RepositoryInventory};
pub use portability::{PortabilityRuleDefinition, PortabilityRuleSet};
pub use repository_contract::{
    ContractRequirement, ContractSource, RepositoryContract, RepositoryContractEvaluator,
    RequirementKind, RequirementOwnership, SourceRevisionKind,
};
pub use repository_intelligence::{
    AdoptionState, CommitHistory, CommitRecord, IntelligenceEnforcement, IntelligenceEvaluation,
    IntelligenceProfile, IntelligenceValidationStatus,
    REPORT_PATH as REPOSITORY_INTELLIGENCE_REPORT, RepositoryIntelligenceEvaluator,
    RepositoryIntelligencePolicy, RepositoryIntelligenceReport, RepresentedCommit,
    collect_commit_history, write_intelligence_report_atomic,
};
pub use repository_presentation::{
    PresentationEvaluation, PresentationIdentityLock, PresentationMarkers, PresentationMode,
    PresentationProfileLock, PresentationValidationStatus,
    REPORT_PATH as REPOSITORY_PRESENTATION_REPORT, RepositoryPresentationEvaluator,
    RepositoryPresentationPolicy, RepositoryPresentationReport, write_presentation_report_atomic,
};
pub use suppressions::apply_suppressions;
