use super::super::{
    super::{
        model::{
            CustodyState, DecisionStage, Expected, FailureClass, ObservableCounts, RootOperation,
            SqliteResult, TerminalDisposition,
        },
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
};
use super::{abi_projection, Shape};

pub(super) fn expected(shape: Shape) -> Expected {
    let mut expected = Expected::unavailable(RootOperation::Lock, shape.phase);
    expected.sqlite = shape.sqlite;
    expected.disposition = shape.disposition;
    expected.failure = shape.failure;
    expected.mutation = shape.mutation;
    expected.lock_outcome_uncertain = shape.lock_uncertain;
    expected.lock_effect = shape.lock_effect;
    expected.dms_lock = shape.dms_lock;
    expected.raw_slots = CustodyState::Unchanged;
    expected.route = shape.route;
    expected.callback = CustodyState::Released;
    expected.file = shape.file;
    expected.counts = ObservableCounts {
        callback_begin: 1,
        callback_complete: 1,
        native_lock: shape.native_lock,
        native_unlock: shape.native_unlock,
        ..ObservableCounts::default()
    };
    expected
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Completion {
    Completed,
    RouteUnknown,
}

pub(super) fn add_completion_terminal(
    builder: &mut Builder,
    gate: &str,
    prefix: &str,
    branch: &str,
    shape: Shape,
    completion: Completion,
    source: SourceWitness,
) {
    let outcome = builder.continuation(
        format!("{prefix}.completion-outcome.{branch}"),
        "callback completion result",
        source,
    );
    builder.edge(gate, &outcome, DecisionStage::CallbackCompletion, branch);
    let mut expected = expected(shape);
    if !matches!(completion, Completion::Completed) {
        if shape.sqlite != SqliteResult::LockUnavailable {
            expected.sqlite = SqliteResult::LockUnavailable;
            expected.failure = FailureClass::RegistryRejected;
        }
        expected.disposition = TerminalDisposition::Quarantined;
        expected.callback = CustodyState::Retained;
        expected.route = match completion {
            Completion::RouteUnknown => CustodyState::Quarantined,
            Completion::Completed => unreachable!(),
        };
    }
    let sqlite = expected.sqlite;
    let projection = adapter_projection(builder, &format!("{prefix}.projection.{branch}"), sqlite);
    builder.edge(
        &outcome,
        &projection,
        DecisionStage::CallbackCompletion,
        "project_completion_outcome",
    );
    add_abi_terminal(
        builder,
        &projection,
        &format!("{prefix}.terminal.{branch}"),
        expected,
    );
}

pub(super) fn adapter_projection(builder: &mut Builder, id: &str, sqlite: SqliteResult) -> String {
    let needle = match sqlite {
        SqliteResult::Ok => {
            "ManagedSqliteShmLockAttempt::Acquired => HandleBoundSqliteAbiAttempt::Acquired"
        }
        SqliteResult::Busy => {
            "ManagedSqliteShmLockAttempt::Contended => HandleBoundSqliteAbiAttempt::Busy"
        }
        SqliteResult::LockUnavailable | SqliteResult::MapUnavailable => ".map_err(drop)",
    };
    builder.continuation(
        id,
        "outer SQLite result-code projection",
        witness(ProductionOwner::RegistryAbiFile, "fn shm_lock", needle, 1),
    )
}

pub(super) fn add_abi_terminal(builder: &mut Builder, from: &str, id: &str, expected: Expected) {
    let sqlite = expected.sqlite;
    let terminal = builder.terminal(id, expected, abi_projection(sqlite));
    builder.edge(
        from,
        &terminal,
        DecisionStage::AbiProjection,
        match sqlite {
            SqliteResult::Ok => "sqlite_ok",
            SqliteResult::Busy => "sqlite_busy",
            SqliteResult::LockUnavailable | SqliteResult::MapUnavailable => "shm_lock_unavailable",
        },
    );
}
