use super::super::super::terminal_descriptor::{
    FaultSeamV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1, PhaseV1, SourceSiteV1,
    TimingV1,
};
use super::{
    super::{
        super::{
            initialization::{self, InitializationFailure, InitializationSuccess},
            model::{DecisionStage, ExclusionProof, FailureClass, LockEffect, MutationState},
            source::{witness, ProductionOwner},
        },
        builder::Builder,
        dynamic::SeedV1,
        input::ValidRequest,
        outcome::{self, Shape},
    },
    helpers::{locking_witness, safe_branch, unsafe_branch},
};

pub(super) fn expand(builder: &mut Builder, from: &str, request: &ValidRequest, branch: &str) {
    let prefix = format!("{}.native-acquire", request.prefix);
    let consistency = builder.decision(
        format!("{prefix}.action-consistency"),
        locking_witness(
            "fn try_os_lock",
            "NODE_MANAGED_SQLITE_SHM_LOCK_ACTION_CHANGED",
        ),
    );
    builder.edge(from, &consistency, DecisionStage::NativeCall, branch);
    let action_changed = builder.excluded(
        format!("{prefix}.excluded.action-changed"),
        ExclusionProof::ControlFlow(
            "lock_connection dispatches try_os_lock from the immutable request.action arm with its matching exclusive constant",
        ),
        locking_witness(
            "fn try_os_lock",
            "NODE_MANAGED_SQLITE_SHM_LOCK_ACTION_CHANGED",
        ),
    );
    builder.edge(
        &consistency,
        &action_changed,
        DecisionStage::NativeCall,
        "action_or_exclusive_changed",
    );

    let init_prefix = format!("{prefix}.initialization");
    let expansion = initialization::build(&init_prefix);
    let entry = expansion.entry.clone();
    let (successes, failures) = builder.merge_initialization(expansion);
    builder.edge(
        &consistency,
        &entry,
        DecisionStage::Initialization,
        "action_consistent_ensure_node",
    );
    assert_eq!(
        successes.len(),
        5,
        "Lock acquire initialization success partition drift"
    );
    for success in successes {
        expand_after_initialization(builder, request, &prefix, &success);
    }
    project_initialization_failures(builder, request, failures);
}

fn project_initialization_failures(
    builder: &mut Builder,
    request: &ValidRequest,
    failures: Vec<InitializationFailure>,
) {
    for failure in failures {
        let mut shape = Shape::failure(
            SeedV1::initialization_failure(
                request.action,
                request.range,
                failure.stimulus,
                failure.typed_phase,
                failure.timing,
                failure.occurrence,
            ),
            failure.phase,
            failure.class,
            failure.mutation,
            failure.lock_uncertain,
            failure.native_lock,
            failure.native_unlock,
        )
        .with_dms_lock(failure.dms_lock);
        shape.disposition = failure.disposition;
        shape.file = failure.file;
        let unsafe_failure = failure.class == FailureClass::OutcomeUncertainPoisoned
            || failure.mutation != MutationState::None
            || failure.lock_uncertain;
        if unsafe_failure {
            outcome::unsafe_failure(builder, &failure.node, &failure.projection_prefix, shape);
        } else {
            outcome::complete(builder, &failure.node, &failure.projection_prefix, shape);
        }
    }
}

fn expand_after_initialization(
    builder: &mut Builder,
    request: &ValidRequest,
    prefix: &str,
    success: &InitializationSuccess,
) {
    let initialization_mutated = success.mutation != MutationState::None;
    let cell = success.label;
    let node_gate = builder.decision(
        format!("{prefix}.{cell}.node-present"),
        locking_witness("fn try_os_lock", "let node = state"),
    );
    builder.edge(
        &success.node,
        &node_gate,
        DecisionStage::Coordination,
        "initialization_succeeded",
    );

    let node_missing = builder.excluded(
        format!("{prefix}.{cell}.excluded.node-missing"),
        ExclusionProof::ControlFlow(
            "ensure_node returned a live node under the same coordinator MutexGuard and no intervening code removes it",
        ),
        locking_witness(
            "fn try_os_lock",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_LOCK",
        ),
    );
    builder.edge(
        &node_gate,
        &node_missing,
        DecisionStage::NativeCall,
        "node_missing_before_native",
    );
    let native = builder.decision(
        format!("{prefix}.{cell}.lock-file-ex"),
        witness(
            ProductionOwner::WindowsLocking,
            "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
            "LockFileEx(",
            1,
        ),
    );
    builder.edge(
        &node_gate,
        &native,
        DecisionStage::NativeCall,
        "node_present_call_lock_file_ex",
    );
    safe_branch(
        builder,
        &native,
        &format!("{prefix}.{cell}.native-busy"),
        DecisionStage::NativeCall,
        "lock_file_ex_contended",
        witness(
            ProductionOwner::WindowsLocking,
            "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
            "Some(ERROR_LOCK_VIOLATION) =>",
            1,
        ),
        Shape::busy(
            native_descriptor(request, success, PhaseV1::LockAcquire, TimingV1::AtCall),
            initialization_mutated,
            success.native_lock + 1,
            success.native_unlock,
        )
        .with_dms_lock(success.dms_lock),
    );
    if initialization_mutated {
        unsafe_branch(
            builder,
            &native,
            &format!("{prefix}.{cell}.native-error-after-initialization"),
            DecisionStage::NativeCall,
            "lock_file_ex_error_after_known_initialization_mutation",
            witness(
                ProductionOwner::WindowsLocking,
                "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
                "_ => Err(error)",
                1,
            ),
            Shape::failure(
                native_descriptor(request, success, PhaseV1::LockAcquire, TimingV1::AtCall),
                "LockAcquire",
                FailureClass::MutatedButKnown,
                success.mutation,
                false,
                success.native_lock + 1,
                success.native_unlock,
            )
            .with_dms_lock(success.dms_lock),
        );
    } else {
        add_warm_native_error(builder, &native, request, prefix, cell, "io", success);
        add_warm_native_error(
            builder,
            &native,
            request,
            prefix,
            cell,
            "unsupported",
            success,
        );
    }

    let installed = builder.decision(
        format!("{prefix}.{cell}.post-native-state"),
        locking_witness(
            "fn try_os_lock",
            "let Some(held) = state.connections.get_mut(&connection_id) else",
        ),
    );
    builder.edge(
        &native,
        &installed,
        DecisionStage::NativeCall,
        "lock_file_ex_acquired",
    );
    let missing_after_lock = builder.excluded(
        format!("{prefix}.{cell}.excluded.connection-missing-after-lock"),
        ExclusionProof::ControlFlow(
            "the coordinator MutexGuard is held from the connection check through LockFileEx and no intervening code removes the connection",
        ),
        locking_witness(
            "fn try_os_lock",
            "NODE_MANAGED_SQLITE_SHM_CONNECTION_MISSING_AFTER_LOCK",
        ),
    );
    builder.edge(
        &installed,
        &missing_after_lock,
        DecisionStage::Coordination,
        "connection_missing",
    );
    safe_branch(
        builder,
        &installed,
        &format!("{prefix}.{cell}.acquired"),
        DecisionStage::Coordination,
        format!(
            "connection_present_update_{}_mask_{:02x}",
            request.action.label(),
            request.range.mask()
        )
        .as_str(),
        locking_witness(
            "fn try_os_lock",
            if request.action.is_shared() {
                "held.shared_mask |= request.mask()"
            } else {
                "held.exclusive_mask |= request.mask()"
            },
        ),
        Shape::success(
            native_descriptor(request, success, PhaseV1::Success, TimingV1::AfterSuccess),
            success.native_lock + 1,
            success.native_unlock,
        )
        .with_lock_effect(LockEffect::Acquired {
            mode: request.action.mode(),
            mask: request.range.mask(),
            native: true,
        })
        .with_dms_lock(success.dms_lock),
    );
}

fn add_warm_native_error(
    builder: &mut Builder,
    native: &str,
    request: &ValidRequest,
    prefix: &str,
    cell: &str,
    kind: &str,
    success: &InitializationSuccess,
) {
    let source = if kind == "unsupported" {
        locking_witness(
            "fn mutation_class",
            "error.kind() == io::ErrorKind::Unsupported",
        )
    } else {
        witness(
            ProductionOwner::WindowsLocking,
            "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
            "_ => Err(error)",
            1,
        )
    };
    let shape = Shape::failure(
        native_descriptor(request, success, PhaseV1::LockAcquire, TimingV1::AtCall),
        "LockAcquire",
        if kind == "unsupported" {
            FailureClass::PlatformUnsupported
        } else {
            FailureClass::IoBeforeMutation
        },
        MutationState::None,
        false,
        success.native_lock + 1,
        success.native_unlock,
    )
    .with_dms_lock(success.dms_lock);
    let outcome_prefix = format!("{prefix}.{cell}.native-{kind}");
    safe_branch(
        builder,
        native,
        &outcome_prefix,
        DecisionStage::NativeCall,
        &format!("lock_file_ex_error_{kind}"),
        source,
        shape,
    );
}

fn native_descriptor(
    request: &ValidRequest,
    success: &InitializationSuccess,
    phase: PhaseV1,
    timing: TimingV1,
) -> SeedV1 {
    request
        .descriptor(
            SourceSiteV1::LockNativeAcquire,
            LockManagedStimulusV1::NativeAcquire,
            LockPrestateV1::NoHeldLocks,
            LockOperationV1::NativeAcquire,
            phase,
            timing,
            FaultSeamV1::NativeOperation,
        )
        .with_initialization(success.profile)
}
