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
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::MapRunnerExecutionViolationV1;

#[derive(Clone, Copy)]
pub(super) enum ProgramModeV1 {
    Observe,
    Extend,
}

#[derive(Clone, Copy)]
pub(super) struct MapProgramSpecV1 {
    pub(super) mode: ProgramModeV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
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
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Map
        || plan.root != RootOperationV1::Map
        || key.source_site != SourceSiteV1::ManagedRequestValidation
        || key.stimulus != StimulusV1::MapManaged(MapManagedStimulusV1::RegionCountBudget)
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
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(),
    })
}

fn digest_implementation_v1() -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-region-count-budget-completed-implementation-v1\0");
    for source in [
        include_str!("../map_program.rs"),
        include_str!("request_budget.rs"),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"
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
    hasher.update(256u32.to_le_bytes());
    hasher.update(32_768u32.to_le_bytes());
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
