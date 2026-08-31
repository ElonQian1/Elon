mod completion;
mod projection;
mod retention;

use super::super::terminal_descriptor::TerminalDescriptorV1;
use super::{
    super::{
        model::{
            CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, Expected, FailureClass,
            LockEffect, MutationState, RootOperation, SqliteResult, TerminalDisposition,
        },
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
    dynamic::{SeedV1, TerminalPathV1},
};
use projection::{adapter_projection, add_abi_terminal};

#[derive(Debug, Clone, Copy)]
pub(super) struct Shape {
    pub(super) descriptor: SeedV1,
    pub(super) sqlite: SqliteResult,
    pub(super) phase: &'static str,
    pub(super) failure: FailureClass,
    pub(super) mutation: MutationState,
    pub(super) lock_uncertain: bool,
    pub(super) lock_effect: LockEffect,
    pub(super) dms_lock: DmsLockCustody,
    pub(super) disposition: TerminalDisposition,
    pub(super) route: CustodyState,
    pub(super) file: CustodyState,
    pub(super) native_lock: u16,
    pub(super) native_unlock: u16,
}

impl Shape {
    pub(super) fn success(descriptor: SeedV1, native_lock: u16, native_unlock: u16) -> Self {
        Self::new(
            descriptor,
            SqliteResult::Ok,
            "Success",
            FailureClass::None,
            MutationState::Known,
        )
        .native(native_lock, native_unlock)
    }

    pub(super) fn busy(
        descriptor: SeedV1,
        mutated: bool,
        native_lock: u16,
        native_unlock: u16,
    ) -> Self {
        Self::new(
            descriptor,
            SqliteResult::Busy,
            "LockAcquire",
            if mutated {
                FailureClass::BusyAfterKnownMutation
            } else {
                FailureClass::BusyNoMutation
            },
            if mutated {
                MutationState::Known
            } else {
                MutationState::None
            },
        )
        .native(native_lock, native_unlock)
    }

    pub(super) fn failure(
        descriptor: SeedV1,
        phase: &'static str,
        failure: FailureClass,
        mutation: MutationState,
        lock_uncertain: bool,
        native_lock: u16,
        native_unlock: u16,
    ) -> Self {
        let mut shape = Self::new(
            descriptor,
            SqliteResult::LockUnavailable,
            phase,
            failure,
            mutation,
        )
        .native(native_lock, native_unlock);
        shape.lock_uncertain = lock_uncertain;
        shape
    }

    fn new(
        descriptor: SeedV1,
        sqlite: SqliteResult,
        phase: &'static str,
        failure: FailureClass,
        mutation: MutationState,
    ) -> Self {
        Self {
            descriptor,
            sqlite,
            phase,
            failure,
            mutation,
            lock_uncertain: false,
            lock_effect: LockEffect::Unchanged,
            dms_lock: DmsLockCustody::NotReached,
            disposition: TerminalDisposition::Returned,
            route: CustodyState::Unchanged,
            file: CustodyState::Unchanged,
            native_lock: 0,
            native_unlock: 0,
        }
    }

    fn native(mut self, lock: u16, unlock: u16) -> Self {
        self.native_lock = lock;
        self.native_unlock = unlock;
        self
    }

    pub(super) fn with_lock_effect(mut self, lock_effect: LockEffect) -> Self {
        self.lock_effect = lock_effect;
        self
    }

    pub(super) fn with_dms_lock(mut self, dms_lock: DmsLockCustody) -> Self {
        self.dms_lock = dms_lock;
        self
    }
}

/// Expands the production `match (result, callback.complete())`.  Operation errors win the SQLite
/// result, while completion failure still changes route and callback custody.
pub(super) fn complete(builder: &mut Builder, from: &str, prefix: &str, shape: Shape) {
    completion::expand(builder, from, prefix, shape);
}

/// Expands failure-custody retention for a SHM error whose class/mutation flags require quarantine.
/// The retain result is intentionally ignored in production, so all three custody results remain.
pub(super) fn unsafe_failure(builder: &mut Builder, from: &str, prefix: &str, shape: Shape) {
    retention::expand(builder, from, prefix, shape);
}

pub(super) fn direct(
    builder: &mut Builder,
    from: &str,
    prefix: &str,
    stage: DecisionStage,
    branch: &str,
    mut expected: Expected,
    descriptor: TerminalDescriptorV1,
    source: SourceWitness,
) {
    expected.sqlite = SqliteResult::LockUnavailable;
    let terminal = builder.terminal(format!("{prefix}.terminal"), expected, descriptor, source);
    builder.edge(from, &terminal, stage, branch);
}

pub(super) fn managed_direct(
    builder: &mut Builder,
    from: &str,
    prefix: &str,
    stage: DecisionStage,
    branch: &str,
    mut expected: Expected,
    descriptor: SeedV1,
    source: SourceWitness,
) {
    expected.sqlite = SqliteResult::LockUnavailable;
    let cause = builder.continuation(
        format!("{prefix}.cause"),
        "managed failure ABI projection",
        source,
    );
    builder.edge(from, &cause, stage, branch);
    let projection = adapter_projection(
        builder,
        &format!("{prefix}.adapter-projection"),
        SqliteResult::LockUnavailable,
    );
    builder.edge(
        &cause,
        &projection,
        DecisionStage::AbiProjection,
        "managed_error_to_unit",
    );
    add_abi_terminal(
        builder,
        &projection,
        &format!("{prefix}.terminal"),
        expected,
        descriptor,
        TerminalPathV1::Direct,
    );
}

pub(super) fn caught_unwind(
    builder: &mut Builder,
    from: &str,
    prefix: &str,
    panic_source: SourceWitness,
    _mutation: MutationState,
    _lock_uncertain: bool,
    _native_lock: u16,
    _native_unlock: u16,
    _lock_effect: LockEffect,
    _callback_started: bool,
) {
    let excluded = builder.excluded(
        format!("{prefix}.excluded"),
        ExclusionProof::ControlFlow(
            "the panic requires loss or substitution of private owned custody while the same value remains exclusively borrowed; no memory-safe callback input can create that state",
        ),
        panic_source,
    );
    builder.edge(
        from,
        &excluded,
        DecisionStage::RawAbandon,
        "private_custody_invariant_violation",
    );
}

pub(super) fn unavailable(phase: &'static str) -> Expected {
    Expected::unavailable(RootOperation::Lock, phase)
}

pub(super) fn abi_projection(sqlite: SqliteResult) -> SourceWitness {
    let needle = match sqlite {
        SqliteResult::Ok => "Ok(HandleBoundSqliteAbiAttempt::Acquired) => ffi::SQLITE_OK",
        SqliteResult::Busy => "Ok(HandleBoundSqliteAbiAttempt::Busy) => ffi::SQLITE_BUSY",
        SqliteResult::LockUnavailable | SqliteResult::MapUnavailable => {
            "Err(()) => result_codes::SHM_LOCK_UNAVAILABLE"
        }
    };
    witness(
        ProductionOwner::AbiIoShm,
        "unsafe extern \"C\" fn lock",
        needle,
        1,
    )
}

pub(super) fn registry_operations(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryOperations, symbol, needle, 1)
}
