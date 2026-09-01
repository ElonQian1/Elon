//! Source-bound specification for the deterministic Map request-budget program.

use sha2::{Digest, Sha256};

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, MapCompletionV1, MapManagedStimulusV1,
        MapModeV1, MapOperationV1, MapPrestateV1, ObserverV1, OccurrenceV1, PhaseV1, PrestateV1,
        ReachabilityV1, SourceSiteV1, StimulusV1, TimingV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::{MapProgramCaseV1, MapProgramSpecV1, MapRunnerExecutionViolationV1, ProgramModeV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapRequestBudgetGuardV1 {
    RegionSize,
    RegionCount,
    LogicalSize,
}

impl MapRequestBudgetGuardV1 {
    fn from_stimulus(value: MapManagedStimulusV1) -> Option<Self> {
        match value {
            MapManagedStimulusV1::RegionSizeBudget => Some(Self::RegionSize),
            MapManagedStimulusV1::RegionCountBudget => Some(Self::RegionCount),
            MapManagedStimulusV1::LogicalSizeBudget => Some(Self::LogicalSize),
            _ => None,
        }
    }

    const fn implementation_tag(self) -> u8 {
        match self {
            Self::RegionSize => 1,
            Self::RegionCount => 2,
            Self::LogicalSize => 3,
        }
    }
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapProgramSpecV1, MapRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(MapRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let DynamicAxesV1::Map(axes) = key.axes else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let mode = match axes.mode {
        ReachabilityV1::Reached(MapModeV1::Observe) => ProgramModeV1::Observe,
        ReachabilityV1::Reached(MapModeV1::Extend) => ProgramModeV1::Extend,
        ReachabilityV1::NotReached => {
            return Err(MapRunnerExecutionViolationV1::UnsupportedProgram)
        }
    };
    let StimulusV1::MapManaged(stimulus) = key.stimulus else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(guard) = MapRequestBudgetGuardV1::from_stimulus(stimulus) else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Map
        || plan.root != RootOperationV1::Map
        || key.source_site != SourceSiteV1::ManagedRequestValidation
        || key.prestate != PrestateV1::Map(MapPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Map(MapOperationV1::ManagedRequest)
        || key.phase != PhaseV1::RequestValidation
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmMap
        || key.recipe.fault_seam != FaultSeamV1::ManagedRequest
        || key.recipe.observer != ObserverV1::MapCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || axes.profile != ReachabilityV1::NotReached
        || axes.ordinal != ReachabilityV1::NotReached
        || axes.regions_to_create != ReachabilityV1::NotReached
        || axes.completion != ReachabilityV1::Reached(MapCompletionV1::Completed)
        || key.expected != expected_v1()
    {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(MapProgramSpecV1 {
        mode,
        case: MapProgramCaseV1::RequestBudget(guard),
        member: member_v1(mode, guard),
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(guard),
    })
}

const fn member_v1(mode: ProgramModeV1, guard: MapRequestBudgetGuardV1) -> StaticMemberSealV1 {
    let (case_key_sha256, full_record_sha256) = match (mode, guard) {
        (ProgramModeV1::Observe, MapRequestBudgetGuardV1::RegionSize) => (
            [
                0xe5, 0xd7, 0xbb, 0xe1, 0x98, 0x63, 0xc0, 0x4d, 0x49, 0x12, 0xbd, 0xa1, 0xab, 0xf5,
                0xe4, 0xb0, 0xce, 0x49, 0xd3, 0xfb, 0xfb, 0x8d, 0x49, 0xf8, 0xcd, 0xcf, 0x18, 0x71,
                0x64, 0xfc, 0xf1, 0xdc,
            ],
            [
                0x2e, 0x52, 0xcd, 0x4e, 0xb1, 0xe0, 0x16, 0x0f, 0xeb, 0x21, 0x98, 0xe0, 0x80, 0x7a,
                0xf5, 0x7c, 0x78, 0x21, 0xcf, 0x3a, 0x25, 0x63, 0x25, 0x5a, 0x3f, 0x11, 0x84, 0xa6,
                0x24, 0x44, 0xbb, 0xfe,
            ],
        ),
        (ProgramModeV1::Observe, MapRequestBudgetGuardV1::RegionCount) => (
            [
                0xa2, 0x8b, 0x96, 0xfe, 0x86, 0x69, 0x82, 0x86, 0xf8, 0x86, 0xa0, 0x1a, 0x79, 0xe1,
                0x51, 0xd7, 0x10, 0x46, 0x1d, 0x40, 0x77, 0x7f, 0x90, 0xdd, 0x23, 0x98, 0xf6, 0x33,
                0xa4, 0xaf, 0xd5, 0x5b,
            ],
            [
                0x94, 0xdf, 0x39, 0xd1, 0x7a, 0x9c, 0x64, 0xa3, 0xde, 0x68, 0xef, 0x15, 0x2e, 0xac,
                0x45, 0xdc, 0x8a, 0x11, 0x8e, 0x3f, 0xba, 0xe8, 0x20, 0x02, 0x1a, 0x57, 0x59, 0xc5,
                0xd8, 0x64, 0xdb, 0xe7,
            ],
        ),
        (ProgramModeV1::Observe, MapRequestBudgetGuardV1::LogicalSize) => (
            [
                0xc8, 0x47, 0x87, 0x02, 0x2a, 0xb9, 0x4a, 0x7a, 0x5f, 0x53, 0x8e, 0xf6, 0x55, 0x8b,
                0xc5, 0xd5, 0x2e, 0xc8, 0x54, 0x5f, 0x4c, 0x46, 0xea, 0x97, 0x3c, 0x93, 0xc5, 0x9f,
                0xc4, 0x2c, 0xec, 0x18,
            ],
            [
                0x5b, 0xf7, 0x03, 0x8f, 0x34, 0x29, 0xdd, 0x49, 0x0f, 0x13, 0x10, 0xfa, 0x5b, 0xad,
                0xa8, 0xfa, 0x4b, 0x6d, 0x69, 0xae, 0x39, 0xf1, 0xfa, 0x25, 0xc6, 0x45, 0x39, 0x59,
                0x37, 0xe9, 0xcc, 0x6b,
            ],
        ),
        (ProgramModeV1::Extend, MapRequestBudgetGuardV1::RegionSize) => (
            [
                0xbb, 0x4c, 0xa7, 0xc5, 0x56, 0x3f, 0x6f, 0x39, 0x74, 0x50, 0x3e, 0x7c, 0xfe, 0xc8,
                0x66, 0x27, 0x17, 0x16, 0x01, 0x51, 0x5c, 0x86, 0xb8, 0x96, 0x80, 0x94, 0x36, 0xa1,
                0xe8, 0xbf, 0xf3, 0xbb,
            ],
            [
                0xc2, 0x26, 0x18, 0x9d, 0xd5, 0xf3, 0x40, 0x56, 0xa6, 0xc4, 0x66, 0x89, 0xe5, 0xf4,
                0x18, 0x13, 0x24, 0x8c, 0xb4, 0xd5, 0xcb, 0xdf, 0x89, 0x9f, 0x6b, 0x2e, 0x44, 0x23,
                0x55, 0x90, 0xb0, 0x26,
            ],
        ),
        (ProgramModeV1::Extend, MapRequestBudgetGuardV1::RegionCount) => (
            [
                0xca, 0x6e, 0x79, 0xd4, 0xd2, 0x2c, 0xce, 0xdc, 0xa2, 0x75, 0xf3, 0xb1, 0x12, 0xf1,
                0x5d, 0x4f, 0x0c, 0x85, 0x59, 0xa8, 0xb9, 0xe1, 0x6e, 0x75, 0x9d, 0xd7, 0xec, 0x3e,
                0x4e, 0x1d, 0xd6, 0xf6,
            ],
            [
                0xac, 0xdb, 0x72, 0xd2, 0x70, 0x4c, 0x3a, 0x9f, 0x8b, 0xac, 0x74, 0xcb, 0x00, 0xbb,
                0xa8, 0xe2, 0xdd, 0x8a, 0xbf, 0x79, 0xa6, 0x4a, 0x10, 0xeb, 0xfc, 0x15, 0x62, 0x5b,
                0xbd, 0x93, 0xe0, 0xf6,
            ],
        ),
        (ProgramModeV1::Extend, MapRequestBudgetGuardV1::LogicalSize) => (
            [
                0xd8, 0x16, 0xfc, 0x70, 0x06, 0x78, 0xed, 0xd9, 0xae, 0x56, 0x8e, 0xa5, 0x01, 0xad,
                0x21, 0x87, 0x49, 0xa8, 0xd2, 0xa0, 0x34, 0x04, 0x89, 0x74, 0x84, 0x78, 0xd1, 0x46,
                0xf9, 0x41, 0x40, 0x54,
            ],
            [
                0x2a, 0x9e, 0xac, 0x13, 0x2e, 0x91, 0x2a, 0xeb, 0x94, 0x37, 0x44, 0x1c, 0xa2, 0x9f,
                0xfe, 0xef, 0xfd, 0xf1, 0xa8, 0xdf, 0x17, 0x5f, 0xb8, 0x54, 0x64, 0xc9, 0xaf, 0x8d,
                0x3a, 0xb6, 0x81, 0xf1,
            ],
        ),
    };
    StaticMemberSealV1 {
        case_key_sha256: Digest32(case_key_sha256),
        full_record_sha256: Digest32(full_record_sha256),
    }
}

fn digest_implementation_v1(guard: MapRequestBudgetGuardV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-managed-request-budget-completed-implementation-v1\0");
    for source in [
        include_str!("../map_program.rs"),
        include_str!("request_budget.rs"),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner/request_budget.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/payload.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/capture.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/environment.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/cleanup.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shared_namespace.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection/unmap.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/callbacks.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations/shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/types.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"
        )),
    ] {
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([guard.implementation_tag()]);
    Digest32(hasher.finalize().into())
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::MapUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::RequestValidation,
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Unchanged,
        callback: CustodyStateV1::Released,
        file: CustodyStateV1::Retained,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            ..ObservableCountsV1::default()
        },
    }
}
