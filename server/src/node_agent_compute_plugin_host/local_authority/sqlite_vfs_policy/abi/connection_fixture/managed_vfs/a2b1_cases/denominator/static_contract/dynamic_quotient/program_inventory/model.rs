use super::super::super::source_leaf_authority::{Digest32, RootOperationV1};
use super::super::{
    descriptor_binding::DescriptorBindingContextDriftV1,
    runner_admission::{ExecutionProgramInventoryStatusV1, ExecutionProgramInventoryViolationV1},
    DynamicClassKeyV1, ProjectionErrorV1, StaticMemberSealV1,
};

pub(in super::super) const EXECUTION_PROGRAM_INVENTORY_SCHEMA_V1: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ExecutionProgramInventoryContextV1 {
    pub(in super::super) schema_version: u16,
    pub(in super::super) root: RootOperationV1,
    pub(in super::super) static_source_baseline_sha1: String,
    pub(in super::super) static_source_scope_sha256: Digest32,
    pub(in super::super) static_ledger_sha256: Digest32,
    pub(in super::super) static_manifest_sha256: Digest32,
    pub(in super::super) static_member_pair_set_sha256: Digest32,
    pub(in super::super) static_included_count: u64,
    pub(in super::super) static_excluded_count: u64,
    pub(in super::super) static_source_universe_count: u64,
    pub(in super::super) projector_schema_sha256: Digest32,
    pub(in super::super) projector_source_scope_sha256: Digest32,
    pub(in super::super) descriptor_binding_sha256: Digest32,
    pub(in super::super) inventory_source_scope_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ExecutionProgramGroupV1 {
    pub(in super::super) normalized_key: DynamicClassKeyV1,
    pub(in super::super) program_id: Digest32,
    pub(in super::super) plan_sha256: Digest32,
    pub(in super::super) status: ExecutionProgramInventoryStatusV1,
    pub(in super::super) member_count: u64,
    pub(in super::super) member_set_sha256: Digest32,
    pub(in super::super) members: Vec<StaticMemberSealV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) struct ExecutionProgramMembershipV1 {
    pub(in super::super) member: StaticMemberSealV1,
    pub(in super::super) program_id: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ExecutionProgramInventoryV1 {
    pub(in super::super) context: ExecutionProgramInventoryContextV1,
    pub(in super::super) member_count: u64,
    pub(in super::super) program_group_count: u64,
    pub(in super::super) source_present_member_count: u64,
    pub(in super::super) source_present_group_count: u64,
    pub(in super::super) planned_missing_member_count: u64,
    pub(in super::super) planned_missing_group_count: u64,
    pub(in super::super) membership_sha256: Digest32,
    pub(in super::super) program_catalog_sha256: Digest32,
    pub(in super::super) inventory_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ExecutionProgramInventoryBundleV1 {
    pub(in super::super) inventory: ExecutionProgramInventoryV1,
    pub(in super::super) groups: Vec<ExecutionProgramGroupV1>,
    pub(in super::super) reverse_index: Vec<ExecutionProgramMembershipV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ExecutionProgramProjectionFailureV1 {
    pub(in super::super) member: StaticMemberSealV1,
    pub(in super::super) error: ProjectionErrorV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum ExecutionProgramInventoryErrorV1 {
    StaticIngress(String),
    StaticBindingDrift,
    RootMismatch,
    OutcomeMismatch,
    DuplicateStaticMember(StaticMemberSealV1),
    ExcludedMemberProjected(StaticMemberSealV1),
    ProjectedMemberDigestMismatch(StaticMemberSealV1),
    DescriptorBindingContextDrift(DescriptorBindingContextDriftV1),
    DescriptorBindingCommitmentDrift {
        expected: Digest32,
        actual: Digest32,
    },
    ProjectionFailed(ExecutionProgramProjectionFailureV1),
    ProgramInventoryAdmissionFailed {
        member: StaticMemberSealV1,
        error: ExecutionProgramInventoryViolationV1,
    },
    ProgramIdentityMismatch(StaticMemberSealV1),
    ProgramDigestCollision(Digest32),
    ProgramContractMismatch(Digest32),
    ProgramMembershipMismatch,
    StaticMemberCountMismatch,
    StaticExcludedCountMismatch,
    StaticUniverseMismatch,
    StaticMemberSetMismatch,
    EmptyProgramGroup,
    CountOverflow,
}
