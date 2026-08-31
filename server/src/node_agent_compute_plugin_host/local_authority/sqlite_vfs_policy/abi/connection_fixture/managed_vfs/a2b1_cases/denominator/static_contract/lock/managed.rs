mod acquire;
mod helpers;
mod local;
mod release;

use super::super::terminal_descriptor::{
    FaultSeamV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1, SourceSiteV1, TimingV1,
};
use super::{
    super::{
        model::{
            DecisionStage, DmsLockCustody, ExclusionProof, FailureClass, LockEffect, MutationState,
        },
        poison,
        source::{witness, ProductionOwner},
    },
    builder::Builder,
    input::ValidRequest,
    outcome::{self, Shape},
};
use helpers::{
    add_admission_failures, locking_witness, registry_operations, safe_branch, unsafe_branch,
};

pub(super) fn expand(builder: &mut Builder, request: ValidRequest) {
    let admission = builder.decision(
        format!("{}.callback-admission", request.prefix),
        witness(
            ProductionOwner::RegistryProcessOwner,
            "pub(super) fn begin_callback",
            "let lease = self.apply_route(route, |routes| routes.begin_callback(route, kind))?;",
            1,
        ),
    );
    builder.edge(
        &request.node,
        &admission,
        DecisionStage::CallbackAdmission,
        "begin_shm_callback",
    );
    add_admission_failures(builder, &admission, &request);

    let custody_present = builder.decision(
        format!("{}.custody-present", request.prefix),
        registry_operations(
            "fn with_shm<T>",
            "expect(\"live pinned file operation must retain exact custody\")",
        ),
    );
    builder.edge(
        &admission,
        &custody_present,
        DecisionStage::CallbackAdmission,
        "admitted",
    );
    outcome::caught_unwind(
        builder,
        &custody_present,
        &format!("{}.custody-invariant-unwind", request.prefix),
        registry_operations(
            "fn with_shm<T>",
            "expect(\"live pinned file operation must retain exact custody\")",
        ),
        MutationState::None,
        false,
        0,
        0,
        LockEffect::NotReached,
        true,
    );

    let custody = builder.decision(
        format!("{}.shm-file-role", request.prefix),
        registry_operations(
            "fn with_shm<T>",
            "let ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } = custody else",
        ),
    );
    builder.edge(
        &custody_present,
        &custody,
        DecisionStage::CallbackAdmission,
        "custody_present",
    );
    safe_branch(
        builder,
        &custody,
        &format!("{}.unsupported-file-role", request.prefix),
        DecisionStage::CallbackAdmission,
        "not_wal_main",
        registry_operations("fn with_shm<T>", "UnsupportedFileRole"),
        Shape::failure(
            request.descriptor(
                SourceSiteV1::AdapterDispatch,
                LockManagedStimulusV1::UnsupportedFileRole,
                LockPrestateV1::NotReached,
                LockOperationV1::CallbackAdmission,
                super::super::terminal_descriptor::PhaseV1::CallbackAdmission,
                TimingV1::BeforeCall,
                FaultSeamV1::RegistryAdmission,
            ),
            "CallbackAdmission",
            FailureClass::RegistryRejected,
            MutationState::None,
            false,
            0,
            0,
        )
        .with_lock_effect(LockEffect::NotReached),
    );

    let shm = builder.decision(
        format!("{}.shm-attached", request.prefix),
        registry_operations("fn with_shm<T>", "let Some(shm) = file.shm_mut() else"),
    );
    builder.edge(&custody, &shm, DecisionStage::CallbackAdmission, "wal_main");
    safe_branch(
        builder,
        &shm,
        &format!("{}.shm-detached", request.prefix),
        DecisionStage::CallbackAdmission,
        "wal_main_without_shm",
        registry_operations("fn with_shm<T>", "ShmDetached"),
        Shape::failure(
            request.descriptor(
                SourceSiteV1::AdapterDispatch,
                LockManagedStimulusV1::ShmDetached,
                LockPrestateV1::NotReached,
                LockOperationV1::CallbackAdmission,
                super::super::terminal_descriptor::PhaseV1::CallbackAdmission,
                TimingV1::BeforeCall,
                FaultSeamV1::RegistryAdmission,
            ),
            "CallbackAdmission",
            FailureClass::RegistryRejected,
            MutationState::None,
            false,
            0,
            0,
        )
        .with_lock_effect(LockEffect::NotReached),
    );

    let active = builder.decision(
        format!("{}.connection-active", request.prefix),
        locking_witness("pub(crate) fn lock", "if !self.active"),
    );
    builder.edge(&shm, &active, DecisionStage::Coordination, "shm_attached");
    let inactive = builder.excluded(
        format!("{}.excluded.connection-inactive", request.prefix),
        ExclusionProof::ControlFlow(
            "with_shm can reach this pinned WAL-main connection only while its SHM connection remains active; detach removes SHM custody before another callback can be admitted",
        ),
        locking_witness("pub(crate) fn lock", "NODE_MANAGED_SQLITE_SHM_CONNECTION_INACTIVE"),
    );
    builder.edge(&active, &inactive, DecisionStage::Coordination, "inactive");

    let coordinator = builder.decision(
        format!("{}.coordinator-state", request.prefix),
        locking_witness(
            "pub(super) fn lock_connection",
            "let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;",
        ),
    );
    builder.edge(&active, &coordinator, DecisionStage::Coordination, "active");
    let mutex_poisoned = builder.excluded(
        format!("{}.coordinator-mutex-poisoned", request.prefix),
        poison::coordinator_mutex_poison_proof(),
        witness(
            ProductionOwner::ManagedCoordinator,
            "pub(super) fn poisoned_failure",
            "NODE_MANAGED_SQLITE_SHM_COORDINATOR_POISONED",
            1,
        ),
    );
    builder.edge(
        &coordinator,
        &mutex_poisoned,
        DecisionStage::Coordination,
        "mutex_poisoned",
    );
    poison::validate_manifest();
    for cell in poison::STORED_POISON_CELLS {
        let label = cell.label();
        unsafe_branch(
            builder,
            &coordinator,
            &format!("{}.domain-already-poisoned.{label}", request.prefix),
            DecisionStage::Coordination,
            &format!("domain_poisoned.{label}"),
            locking_witness(
                "pub(super) fn lock_connection",
                "return Err(poison.failure());",
            ),
            Shape::failure(
                request.descriptor(
                    SourceSiteV1::CoordinatorState,
                    LockManagedStimulusV1::StoredPoison,
                    LockPrestateV1::StoredPoison,
                    LockOperationV1::Quarantine,
                    cell.typed_phase,
                    TimingV1::BeforeCall,
                    FaultSeamV1::Natural,
                ),
                cell.phase,
                FailureClass::OutcomeUncertainPoisoned,
                cell.mutation,
                cell.lock_outcome_uncertain,
                0,
                0,
            )
            .with_dms_lock(DmsLockCustody::UnobservedRetained),
        );
    }
    let not_attached = builder.excluded(
        format!("{}.excluded.connection-not-attached", request.prefix),
        ExclusionProof::ControlFlow(
            "an active pinned SHM connection is inserted into coordinator.connections at attach and cannot be removed before the same connection is deactivated and its custody detached",
        ),
        locking_witness(
            "pub(super) fn lock_connection",
            "NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED",
        ),
    );
    builder.edge(
        &coordinator,
        &not_attached,
        DecisionStage::Coordination,
        "connection_missing",
    );

    let local = builder.decision(
        format!("{}.local-state", request.prefix),
        locking_witness("pub(super) fn lock_connection", "match request.action()"),
    );
    builder.edge(
        &coordinator,
        &local,
        DecisionStage::Coordination,
        "state_ready",
    );
    local::expand(builder, &local, &request);
}
