//! Source-bound specification for the deterministic Lock managed-request-validation programs.

use sha2::{Digest, Sha256};

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, LockActionV1, LockAxesV1, LockCompletionV1,
        LockManagedStimulusV1, LockOperationV1, LockPrestateV1, ObserverV1, OccurrenceV1, PhaseV1,
        PrestateV1, ReachabilityV1, SourceSiteV1, StimulusV1, TimingV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::LockRunnerExecutionViolationV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockRequestValidationGuardV1 {
    RangeOverflow,
    EndPastEight,
    SharedMultiSlot,
}

impl LockRequestValidationGuardV1 {
    fn from_stimulus(value: LockManagedStimulusV1) -> Option<Self> {
        match value {
            LockManagedStimulusV1::RangeOverflow => Some(Self::RangeOverflow),
            LockManagedStimulusV1::EndPastEight => Some(Self::EndPastEight),
            LockManagedStimulusV1::SharedMultiSlot => Some(Self::SharedMultiSlot),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct LockProgramSpecV1 {
    pub(super) action: LockActionV1,
    #[cfg(windows)]
    pub(super) guard: LockRequestValidationGuardV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockProgramSpecV1, LockRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(LockRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let DynamicAxesV1::Lock(axes) = key.axes else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let ReachabilityV1::Reached(action) = axes.action else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let StimulusV1::LockManaged(stimulus) = key.stimulus else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(guard) = LockRequestValidationGuardV1::from_stimulus(stimulus) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(implementation_tag) = implementation_tag_v1(action, guard) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let expected_axes = LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
        ..LockAxesV1::NOT_REACHED
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || plan.root != RootOperationV1::Lock
        || key.source_site != SourceSiteV1::ManagedRequestValidation
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::ManagedRequest)
        || key.phase != PhaseV1::RequestValidation
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::ManagedRequest
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || axes != expected_axes
        || key.expected != expected_v1()
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockProgramSpecV1 {
        action,
        #[cfg(windows)]
        guard,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(implementation_tag),
    })
}

const fn implementation_tag_v1(
    action: LockActionV1,
    guard: LockRequestValidationGuardV1,
) -> Option<u8> {
    match (action, guard) {
        (LockActionV1::LockShared, LockRequestValidationGuardV1::RangeOverflow) => Some(1),
        (LockActionV1::LockShared, LockRequestValidationGuardV1::EndPastEight) => Some(2),
        (LockActionV1::LockShared, LockRequestValidationGuardV1::SharedMultiSlot) => Some(3),
        (LockActionV1::LockExclusive, LockRequestValidationGuardV1::RangeOverflow) => Some(4),
        (LockActionV1::LockExclusive, LockRequestValidationGuardV1::EndPastEight) => Some(5),
        (LockActionV1::UnlockShared, LockRequestValidationGuardV1::RangeOverflow) => Some(6),
        (LockActionV1::UnlockShared, LockRequestValidationGuardV1::EndPastEight) => Some(7),
        (LockActionV1::UnlockShared, LockRequestValidationGuardV1::SharedMultiSlot) => Some(8),
        (LockActionV1::UnlockExclusive, LockRequestValidationGuardV1::RangeOverflow) => Some(9),
        (LockActionV1::UnlockExclusive, LockRequestValidationGuardV1::EndPastEight) => Some(10),
        (
            LockActionV1::LockExclusive | LockActionV1::UnlockExclusive,
            LockRequestValidationGuardV1::SharedMultiSlot,
        ) => None,
    }
}

fn digest_implementation_v1(implementation_tag: u8) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-managed-request-validation-direct-implementation-v1\0");
    for source in [
        include_str!("../lock_program.rs"),
        include_str!("request_validation.rs"),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/request_validation.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_request_validation.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/live_registration.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"
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
    ] {
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([implementation_tag]);
    Digest32(hasher.finalize().into())
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::RequestValidation,
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::Unchanged,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::NotReached,
        callback: CustodyStateV1::NotReached,
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1::default(),
    }
}
