use super::super::super::terminal_descriptor::{
    FaultSeamV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1, PhaseV1, SourceSiteV1,
    TimingV1,
};
use super::super::{
    super::model::{DecisionStage, DmsLockCustody, ExclusionProof, LockEffect},
    builder::Builder,
    input::ValidRequest,
    outcome::Shape,
    range::Action,
};
use super::{
    acquire,
    helpers::{locking_witness, protocol, safe_branch},
    release,
};

pub(super) fn expand(builder: &mut Builder, local: &str, request: &ValidRequest) {
    match request.action {
        Action::LockShared => lock_shared(builder, local, request),
        Action::LockExclusive => lock_exclusive(builder, local, request),
        Action::UnlockShared => unlock_shared(builder, local, request),
        Action::UnlockExclusive => unlock_exclusive(builder, local, request),
    }
}

fn lock_shared(builder: &mut Builder, local: &str, request: &ValidRequest) {
    safe_branch(
        builder,
        local,
        &format!("{}.own-overlap", request.prefix),
        DecisionStage::Coordination,
        "own_shared_or_exclusive_overlap",
        locking_witness(
            "fn require_unlocked",
            "NODE_MANAGED_SQLITE_SHM_LOCK_TRANSITION_NOT_UNLOCKED",
        ),
        protocol(
            local_descriptor(
                request,
                LockPrestateV1::OwnOverlap,
                LockOperationV1::LocalAcquire,
                PhaseV1::RequestValidation,
            ),
            "RequestValidation",
            0,
            0,
        ),
    );
    safe_branch(
        builder,
        local,
        &format!("{}.sibling-exclusive-busy", request.prefix),
        DecisionStage::Coordination,
        "own_clear_sibling_exclusive_overlap",
        locking_witness(
            "pub(super) fn lock_connection",
            "sibling.exclusive_mask & mask != 0",
        ),
        Shape::busy(
            local_descriptor(
                request,
                LockPrestateV1::SiblingExclusiveContention,
                LockOperationV1::LocalAcquire,
                PhaseV1::LockAcquire,
            ),
            false,
            0,
            0,
        )
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );
    let local_shared = builder.decision(
        format!("{}.shared-local-connection", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "state.connections.get_mut(&connection_id).ok_or_else",
        ),
    );
    builder.edge(
        local,
        &local_shared,
        DecisionStage::Coordination,
        "own_clear_sibling_shared_overlap",
    );
    add_connection_disappeared_exclusion(builder, &local_shared, request, "shared-lock");
    safe_branch(
        builder,
        &local_shared,
        &format!("{}.shared-local-success", request.prefix),
        DecisionStage::Coordination,
        "connection_present_update_shared_mask",
        locking_witness("pub(super) fn lock_connection", "held.shared_mask |= mask"),
        Shape::success(
            local_descriptor(
                request,
                LockPrestateV1::SiblingSharedCoalesced,
                LockOperationV1::LocalAcquire,
                PhaseV1::Success,
            ),
            0,
            0,
        )
        .with_lock_effect(LockEffect::Acquired {
            mode: request.action.mode(),
            mask: request.range.mask(),
            native: false,
        })
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );
    acquire::expand(builder, local, request, "own_clear_no_sibling_overlap");
}

fn lock_exclusive(builder: &mut Builder, local: &str, request: &ValidRequest) {
    safe_branch(
        builder,
        local,
        &format!("{}.own-overlap", request.prefix),
        DecisionStage::Coordination,
        "own_shared_or_exclusive_overlap",
        locking_witness(
            "fn require_unlocked",
            "NODE_MANAGED_SQLITE_SHM_LOCK_TRANSITION_NOT_UNLOCKED",
        ),
        protocol(
            local_descriptor(
                request,
                LockPrestateV1::OwnOverlap,
                LockOperationV1::LocalAcquire,
                PhaseV1::RequestValidation,
            ),
            "RequestValidation",
            0,
            0,
        ),
    );
    safe_branch(
        builder,
        local,
        &format!("{}.sibling-overlap-busy", request.prefix),
        DecisionStage::Coordination,
        "own_clear_sibling_shared_or_exclusive_overlap",
        locking_witness(
            "pub(super) fn lock_connection",
            "(sibling.shared_mask | sibling.exclusive_mask) & mask != 0",
        ),
        Shape::busy(
            local_descriptor(
                request,
                LockPrestateV1::SiblingAnyContention,
                LockOperationV1::LocalAcquire,
                PhaseV1::LockAcquire,
            ),
            false,
            0,
            0,
        )
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );
    acquire::expand(builder, local, request, "own_clear_no_sibling_overlap");
}

fn unlock_shared(builder: &mut Builder, local: &str, request: &ValidRequest) {
    safe_branch(
        builder,
        local,
        &format!("{}.shared-not-held", request.prefix),
        DecisionStage::Coordination,
        "shared_mask_missing",
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD",
        ),
        protocol(
            local_descriptor(
                request,
                LockPrestateV1::NoHeldLocks,
                LockOperationV1::LocalRelease,
                PhaseV1::RequestValidation,
            ),
            "RequestValidation",
            0,
            0,
        ),
    );
    let exclusive_check = builder.decision(
        format!("{}.shared-unlock-own-exclusive-check", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "current.exclusive_mask & mask != 0",
        ),
    );
    builder.edge(
        local,
        &exclusive_check,
        DecisionStage::Coordination,
        "shared_mask_held",
    );
    let impossible_overlap = builder.excluded(
        format!("{}.excluded.shared-unlock-own-exclusive-overlap", request.prefix),
        ExclusionProof::ControlFlow(
            "all lock acquisition paths require the same connection to be clear for the requested mask, so a reachable connection cannot hold overlapping shared and exclusive masks",
        ),
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD",
        ),
    );
    builder.edge(
        &exclusive_check,
        &impossible_overlap,
        DecisionStage::Coordination,
        "own_exclusive_overlap",
    );
    let local_release = builder.decision(
        format!("{}.shared-local-release-connection", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "state.connections.get_mut(&connection_id).ok_or_else",
        ),
    );
    builder.edge(
        &exclusive_check,
        &local_release,
        DecisionStage::Coordination,
        "own_exclusive_clear_sibling_shared_overlap",
    );
    add_connection_disappeared_exclusion(builder, &local_release, request, "shared-unlock");
    safe_branch(
        builder,
        &local_release,
        &format!("{}.shared-local-release-success", request.prefix),
        DecisionStage::Coordination,
        "connection_present_clear_shared_mask",
        locking_witness("pub(super) fn lock_connection", "held.shared_mask &= !mask"),
        Shape::success(
            local_descriptor(
                request,
                LockPrestateV1::SiblingSharedCoalesced,
                LockOperationV1::LocalRelease,
                PhaseV1::Success,
            ),
            0,
            0,
        )
        .with_lock_effect(LockEffect::Released {
            mode: request.action.mode(),
            mask: request.range.mask(),
            native: false,
        })
        .with_dms_lock(DmsLockCustody::ExistingShared),
    );
    release::expand(
        builder,
        &exclusive_check,
        request,
        "own_exclusive_clear_no_sibling_shared_overlap",
    );
}

fn unlock_exclusive(builder: &mut Builder, local: &str, request: &ValidRequest) {
    safe_branch(
        builder,
        local,
        &format!("{}.exclusive-not-held", request.prefix),
        DecisionStage::Coordination,
        "exclusive_mask_missing",
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_NOT_HELD",
        ),
        protocol(
            local_descriptor(
                request,
                LockPrestateV1::NoHeldLocks,
                LockOperationV1::LocalRelease,
                PhaseV1::RequestValidation,
            ),
            "RequestValidation",
            0,
            0,
        ),
    );
    let shared_check = builder.decision(
        format!("{}.exclusive-unlock-own-shared-check", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "current.shared_mask & mask != 0",
        ),
    );
    builder.edge(
        local,
        &shared_check,
        DecisionStage::Coordination,
        "exclusive_mask_held",
    );
    let impossible_overlap = builder.excluded(
        format!("{}.excluded.exclusive-unlock-own-shared-overlap", request.prefix),
        ExclusionProof::ControlFlow(
            "all lock acquisition paths require the same connection to be clear for the requested mask, so a reachable connection cannot hold overlapping exclusive and shared masks",
        ),
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_NOT_HELD",
        ),
    );
    builder.edge(
        &shared_check,
        &impossible_overlap,
        DecisionStage::Coordination,
        "own_shared_overlap",
    );
    let range_gate = builder.decision(
        format!("{}.exclusive-range-table", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "current.exclusive_ranges[usize::from(request.first())] != request.count()",
        ),
    );
    builder.edge(
        &shared_check,
        &range_gate,
        DecisionStage::Coordination,
        "own_shared_clear",
    );
    safe_branch(
        builder,
        &range_gate,
        &format!("{}.exclusive-range-mismatch", request.prefix),
        DecisionStage::Coordination,
        "range_count_mismatch",
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_RANGE_MISMATCH",
        ),
        protocol(
            local_descriptor(
                request,
                LockPrestateV1::ExclusiveRangeMismatch,
                LockOperationV1::LocalRelease,
                PhaseV1::RequestValidation,
            ),
            "RequestValidation",
            0,
            0,
        ),
    );
    let sibling_overlap = builder.excluded(
        format!("{}.excluded.exclusive-sibling-overlap", request.prefix),
        ExclusionProof::ControlFlow(
            "exclusive acquisition rejects every sibling overlap before installing the exclusive mask, and coordinator mutations are serialized by the same MutexGuard",
        ),
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_SIBLING_OVERLAP",
        ),
    );
    builder.edge(
        &range_gate,
        &sibling_overlap,
        DecisionStage::Coordination,
        "range_exact_sibling_shared_or_exclusive_overlap",
    );
    release::expand(
        builder,
        &range_gate,
        request,
        "range_exact_no_sibling_overlap",
    );
}

fn add_connection_disappeared_exclusion(
    builder: &mut Builder,
    from: &str,
    request: &ValidRequest,
    cell: &str,
) {
    let missing = builder.excluded(
        format!("{}.excluded.{cell}-connection-disappeared", request.prefix),
        ExclusionProof::ControlFlow(
            "lock_connection copied this connection from the map while holding the same coordinator MutexGuard, and no intervening operation can remove it",
        ),
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED",
        ),
    );
    builder.edge(
        from,
        &missing,
        DecisionStage::Coordination,
        "connection_disappeared",
    );
}

fn local_descriptor(
    request: &ValidRequest,
    prestate: LockPrestateV1,
    operation: LockOperationV1,
    phase: PhaseV1,
) -> super::super::dynamic::SeedV1 {
    request.descriptor(
        SourceSiteV1::LockLocalState,
        LockManagedStimulusV1::LocalState,
        prestate,
        operation,
        phase,
        TimingV1::Natural,
        FaultSeamV1::Natural,
    )
}
