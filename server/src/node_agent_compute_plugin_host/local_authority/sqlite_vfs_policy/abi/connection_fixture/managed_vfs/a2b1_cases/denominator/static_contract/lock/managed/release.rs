use super::super::super::terminal_descriptor::{
    FaultSeamV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1, PhaseV1, SourceSiteV1,
    TimingV1,
};
use super::{
    super::{
        super::{
            model::{
                DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, LockEffect,
                MutationState,
            },
            source::{witness, ProductionOwner},
        },
        builder::Builder,
        input::ValidRequest,
        outcome::Shape,
    },
    helpers::{locking_witness, safe_branch, unsafe_branch},
};

pub(super) fn expand(builder: &mut Builder, from: &str, request: &ValidRequest, branch: &str) {
    let prefix = format!("{}.native-release", request.prefix);
    let consistency = builder.decision(
        format!("{prefix}.action-consistency"),
        locking_witness(
            "fn unlock_os_range",
            "NODE_MANAGED_SQLITE_SHM_UNLOCK_ACTION_CHANGED",
        ),
    );
    builder.edge(from, &consistency, DecisionStage::NativeCall, branch);
    let action_changed = builder.excluded(
        format!("{prefix}.excluded.action-changed"),
        ExclusionProof::ControlFlow(
            "lock_connection dispatches unlock_os_range only from immutable unlock action arms",
        ),
        locking_witness(
            "fn unlock_os_range",
            "NODE_MANAGED_SQLITE_SHM_UNLOCK_ACTION_CHANGED",
        ),
    );
    builder.edge(
        &consistency,
        &action_changed,
        DecisionStage::NativeCall,
        "action_changed_to_acquire",
    );

    let node_gate = builder.decision(
        format!("{prefix}.node-present"),
        locking_witness("fn unlock_os_range", "match state.node.as_mut()"),
    );
    builder.edge(
        &consistency,
        &node_gate,
        DecisionStage::Coordination,
        "action_consistent",
    );
    let node_missing = builder.excluded(
        format!("{prefix}.excluded.node-missing"),
        ExclusionProof::ControlFlow(
            "a recorded shared or exclusive lock can only be installed after ensure_node returned a live node, and node teardown cannot run while this MutexGuard is held",
        ),
        locking_witness(
            "fn unlock_os_range",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_UNLOCK",
        ),
    );
    builder.edge(
        &node_gate,
        &node_missing,
        DecisionStage::NativeCall,
        "node_missing_before_native",
    );
    let native = builder.decision(
        format!("{prefix}.unlock-file-ex"),
        witness(
            ProductionOwner::WindowsLocking,
            "pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range(",
            "UnlockFileEx(",
            1,
        ),
    );
    builder.edge(
        &node_gate,
        &native,
        DecisionStage::NativeCall,
        "node_present_call_unlock_file_ex",
    );
    unsafe_branch(
        builder,
        &native,
        &format!("{prefix}.native-error"),
        DecisionStage::NativeCall,
        "unlock_file_ex_error",
        witness(
            ProductionOwner::WindowsLocking,
            "pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range(",
            "return Err(std::io::Error::last_os_error());",
            1,
        ),
        Shape::failure(
            release_descriptor(request, PhaseV1::LockRelease, TimingV1::AtCall),
            "LockRelease",
            FailureClass::OutcomeUncertainPoisoned,
            MutationState::None,
            true,
            0,
            1,
        )
        .with_lock_effect(LockEffect::OutcomeUncertain {
            mode: request.action.mode(),
            mask: request.range.mask(),
        })
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );

    let updated = builder.decision(
        format!("{prefix}.post-native-state"),
        locking_witness(
            "fn unlock_os_range",
            "let Some(held) = state.connections.get_mut(&connection_id) else",
        ),
    );
    builder.edge(
        &native,
        &updated,
        DecisionStage::NativeCall,
        "unlock_file_ex_succeeded",
    );
    let missing_after_unlock = builder.excluded(
        format!("{prefix}.excluded.connection-missing-after-unlock"),
        ExclusionProof::ControlFlow(
            "the coordinator MutexGuard is held from the connection check through UnlockFileEx and no intervening code removes the connection",
        ),
        locking_witness(
            "fn unlock_os_range",
            "NODE_MANAGED_SQLITE_SHM_CONNECTION_MISSING_AFTER_UNLOCK",
        ),
    );
    builder.edge(
        &updated,
        &missing_after_unlock,
        DecisionStage::Coordination,
        "connection_missing",
    );
    safe_branch(
        builder,
        &updated,
        &format!("{prefix}.released"),
        DecisionStage::Coordination,
        &format!(
            "connection_present_clear_{}_mask_{:02x}",
            request.action.label(),
            request.range.mask()
        ),
        locking_witness(
            "fn unlock_os_range",
            if request.action.is_shared() {
                "held.shared_mask &= !request.mask()"
            } else {
                "held.exclusive_mask &= !request.mask()"
            },
        ),
        Shape::success(
            release_descriptor(request, PhaseV1::Success, TimingV1::AfterSuccess),
            0,
            1,
        )
        .with_lock_effect(LockEffect::Released {
            mode: request.action.mode(),
            mask: request.range.mask(),
            native: true,
        })
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );
}

fn release_descriptor(
    request: &ValidRequest,
    phase: PhaseV1,
    timing: TimingV1,
) -> super::super::dynamic::SeedV1 {
    request.descriptor(
        SourceSiteV1::LockNativeRelease,
        LockManagedStimulusV1::NativeRelease,
        if request.action.is_shared() {
            LockPrestateV1::OwnSharedHeld
        } else {
            LockPrestateV1::OwnExclusiveHeld
        },
        LockOperationV1::NativeRelease,
        phase,
        timing,
        FaultSeamV1::NativeOperation,
    )
}
