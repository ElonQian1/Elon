//! Source-bound specification for the exact positive Lock lifecycle programs.
//!
//! Admission uses only the typed dynamic key and exact expected vector; leaf ids, test names and frozen-manifest strings stay outside this matcher.

use sha2::{Digest, Sha256};

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, LockModeV1,
        MutationStateV1, ObservableCountsV1, RootOperationV1, SqliteResultV1,
        TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, InitializationProfileV1, LockActionV1,
        LockAxesV1, LockCompletionV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1,
        ObserverV1, OccurrenceV1, PhaseV1, PrestateV1, ReachabilityV1, SourceSiteV1, StimulusV1,
        TimingV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::LockRunnerExecutionViolationV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockLifecyclePathSpecV1 {
    NativeAcquire,
    NativeRelease,
    SharedLocalAcquire,
    SharedLocalRelease,
}

#[derive(Clone, Copy)]
pub(super) struct LockLifecycleProgramSpecV1 {
    #[cfg(windows)]
    pub(super) path: LockLifecyclePathSpecV1,
    #[cfg(windows)]
    pub(super) action: LockActionV1,
    #[cfg(windows)]
    pub(super) first: u8,
    #[cfg(windows)]
    pub(super) count: u8,
    #[cfg(windows)]
    pub(super) mask: u8,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockLifecycleProgramSpecV1, LockRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(LockRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let DynamicAxesV1::Lock(axes) = key.axes else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let (
        ReachabilityV1::Reached(action),
        ReachabilityV1::Reached(first),
        ReachabilityV1::Reached(count),
        ReachabilityV1::Reached(mask),
    ) = (axes.action, axes.first, axes.count, axes.mask)
    else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(path) = classify_path_v1(key) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if !valid_action_prestate_v1(path, action, key.prestate) {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    let Some(expected_mask) = range_mask_v1(first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(implementation_tag) = implementation_tag_v1(path, action, first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if mask != expected_mask
        || axes != expected_axes_v1(path, action, first, count, mask)
        || key.expected != expected_v1(path, action, mask)
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockLifecycleProgramSpecV1 {
        #[cfg(windows)]
        path,
        #[cfg(windows)]
        action,
        #[cfg(windows)]
        first,
        #[cfg(windows)]
        count,
        #[cfg(windows)]
        mask,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(implementation_tag),
    })
}

fn classify_path_v1(key: &DynamicClassKeyV1) -> Option<LockLifecyclePathSpecV1> {
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.phase != PhaseV1::Success
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
    {
        return None;
    }
    use LockLifecyclePathSpecV1 as P;
    match (
        key.source_site,
        key.stimulus,
        key.prestate,
        key.operation,
        key.timing,
        key.recipe.fixture,
        key.recipe.fault_seam,
    ) {
        (
            SourceSiteV1::LockNativeAcquire,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
            PrestateV1::Lock(LockPrestateV1::NoHeldLocks),
            DynamicOperationV1::Lock(LockOperationV1::NativeAcquire),
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(P::NativeAcquire),
        (
            SourceSiteV1::LockNativeRelease,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeRelease),
            PrestateV1::Lock(LockPrestateV1::OwnSharedHeld | LockPrestateV1::OwnExclusiveHeld),
            DynamicOperationV1::Lock(LockOperationV1::NativeRelease),
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(P::NativeRelease),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced),
            DynamicOperationV1::Lock(LockOperationV1::LocalAcquire),
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(P::SharedLocalAcquire),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced),
            DynamicOperationV1::Lock(LockOperationV1::LocalRelease),
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(P::SharedLocalRelease),
        _ => None,
    }
}

fn expected_axes_v1(
    path: LockLifecyclePathSpecV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> LockAxesV1 {
    use LockLifecyclePathSpecV1 as P;
    let (initialization, held_shared, held_exclusive, sibling_shared) = match path {
        P::NativeAcquire => (
            ReachabilityV1::Reached(InitializationProfileV1::NodeLive),
            0,
            0,
            0,
        ),
        P::NativeRelease => (
            ReachabilityV1::NotReached,
            if action == LockActionV1::UnlockShared {
                mask
            } else {
                0
            },
            if action == LockActionV1::UnlockExclusive {
                mask
            } else {
                0
            },
            0,
        ),
        P::SharedLocalAcquire => (ReachabilityV1::NotReached, 0, 0, mask),
        P::SharedLocalRelease => (ReachabilityV1::NotReached, mask, 0, mask),
    };
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization,
        held_shared_mask: ReachabilityV1::Reached(held_shared),
        held_exclusive_mask: ReachabilityV1::Reached(held_exclusive),
        sibling_shared_mask: ReachabilityV1::Reached(sibling_shared),
        sibling_exclusive_mask: ReachabilityV1::Reached(0),
        completion: ReachabilityV1::Reached(LockCompletionV1::Completed),
    }
}

fn valid_action_prestate_v1(
    path: LockLifecyclePathSpecV1,
    action: LockActionV1,
    prestate: PrestateV1,
) -> bool {
    use LockLifecyclePathSpecV1 as P;
    matches!(
        (path, action, prestate),
        (
            P::NativeAcquire,
            LockActionV1::LockShared | LockActionV1::LockExclusive,
            PrestateV1::Lock(LockPrestateV1::NoHeldLocks)
        ) | (
            P::NativeRelease,
            LockActionV1::UnlockShared,
            PrestateV1::Lock(LockPrestateV1::OwnSharedHeld)
        ) | (
            P::NativeRelease,
            LockActionV1::UnlockExclusive,
            PrestateV1::Lock(LockPrestateV1::OwnExclusiveHeld)
        ) | (
            P::SharedLocalAcquire,
            LockActionV1::LockShared,
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced)
        ) | (
            P::SharedLocalRelease,
            LockActionV1::UnlockShared,
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced)
        )
    )
}

fn expected_v1(path: LockLifecyclePathSpecV1, action: LockActionV1, mask: u8) -> DynamicExpectedV1 {
    use LockLifecyclePathSpecV1 as P;
    let native = matches!(path, P::NativeAcquire | P::NativeRelease);
    let mode = if matches!(
        action,
        LockActionV1::LockShared | LockActionV1::UnlockShared
    ) {
        LockModeV1::Shared
    } else {
        LockModeV1::Exclusive
    };
    let lock_effect = if matches!(path, P::NativeAcquire | P::SharedLocalAcquire) {
        LockEffectV1::Acquired { mode, mask, native }
    } else {
        LockEffectV1::Released { mode, mask, native }
    };
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::Ok,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::Success,
        failure: FailureClassV1::None,
        mutation: MutationStateV1::Known,
        lock_outcome_uncertain: false,
        lock_effect,
        dms_lock: DmsLockCustodyV1::ExistingShared,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Unchanged,
        callback: CustodyStateV1::Released,
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: if path == P::NativeAcquire { 1 } else { 0 },
            native_unlock: if path == P::NativeRelease { 1 } else { 0 },
            ..ObservableCountsV1::default()
        },
    }
}

fn implementation_tag_v1(
    path: LockLifecyclePathSpecV1,
    action: LockActionV1,
    first: u8,
    count: u8,
) -> Option<u8> {
    use LockLifecyclePathSpecV1 as P;
    // Stable one-based tags: acquire shared 1..8 then exclusive 9..44, release shared 45..52
    // then exclusive 53..88, local acquire 89..96, and local release 97..104.
    match (path, action) {
        (P::NativeAcquire, LockActionV1::LockShared) => {
            shared_slot_ordinal_v1(first, count).map(|ordinal| ordinal + 1)
        }
        (P::NativeAcquire, LockActionV1::LockExclusive) => {
            exclusive_range_ordinal_v1(first, count).map(|ordinal| ordinal + 1)
        }
        (P::NativeRelease, LockActionV1::UnlockShared) => {
            shared_slot_ordinal_v1(first, count).map(|ordinal| 45 + ordinal)
        }
        (P::NativeRelease, LockActionV1::UnlockExclusive) => {
            exclusive_range_ordinal_v1(first, count).map(|ordinal| 45 + ordinal)
        }
        (P::SharedLocalAcquire, LockActionV1::LockShared) => {
            shared_slot_ordinal_v1(first, count).map(|ordinal| 89 + ordinal)
        }
        (P::SharedLocalRelease, LockActionV1::UnlockShared) => {
            shared_slot_ordinal_v1(first, count).map(|ordinal| 97 + ordinal)
        }
        _ => None,
    }
}

fn shared_slot_ordinal_v1(first: u8, count: u8) -> Option<u8> {
    (count == 1 && first < 8).then_some(first)
}

fn exclusive_range_ordinal_v1(first: u8, count: u8) -> Option<u8> {
    let end = first.checked_add(count)?;
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    let preceding_counts = (count - 1) * (18 - count) / 2;
    Some(8 + preceding_counts + first)
}

fn range_mask_v1(first: u8, count: u8) -> Option<u8> {
    let end = first.checked_add(count)?;
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

fn digest_implementation_v1(implementation_tag: u8) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-positive-lifecycle-implementation-v1\0");
    for source in [
        include_str!("../lock_program.rs"),
        include_str!("execution_receipt.rs"),
        include_str!("source_program.rs"),
        include_str!("lifecycle.rs"),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/fixture.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/lifecycle/payload.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_lifecycle.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence.rs"
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
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/multi_connection.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"
        )),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations/shm.rs")),
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
            "/src/node_agent_managed_fs.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_api.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows_sqlite_locking.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/windows_sqlite_shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_snapshot.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs"
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
            "/src/node_agent_managed_fs/sqlite_namespace_shm.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/node_agent_managed_fs/sqlite_namespace_shm/test_lock_runtime.rs"
        )),
    ] {
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([implementation_tag]);
    Digest32(hasher.finalize().into())
}
