use super::{
    dms, failure_custody, namespace, namespace_close, namespace_types, shm_root, CustodyState,
    DmsLockCustody, ExclusionProof, FailureClass, FailureShape, InitBuilder, MutationState,
    TerminalDisposition,
};

pub(super) fn build(builder: &mut InitBuilder, from: &str) {
    let call = builder.decision(
        "open.open-shm-for-wal",
        shm_root("fn open_shm_for_wal", "self.open_exact("),
    );
    builder.edge(from, &call, "node_missing_open_shm_for_wal");
    let argument_gate = builder.decision(
        "open.arguments",
        namespace(
            "fn open_exact",
            "mode == ManagedSqliteOpenMode::OpenOrCreate && access != ManagedSqliteAccess::ReadWrite",
        ),
    );
    builder.edge(&call, &argument_gate, "fixed_arguments_supplied");
    let invalid = builder.excluded(
        "open.invalid-fixed-arguments",
        ExclusionProof::ControlFlow(
            "open_shm_for_wal always supplies Shm, ReadWrite, and OpenOrCreate constants",
        ),
        shm_root("fn open_shm_for_wal", "ManagedSqliteOpenMode::OpenOrCreate"),
    );
    builder.edge(&argument_gate, &invalid, "fixed_arguments_invalid");

    let parent_validation = builder.decision(
        "open.parent-validation-before-open",
        namespace("fn open_exact", "self.validate_parent().map_err(|error|"),
    );
    builder.edge(&argument_gate, &parent_validation, "fixed_arguments_valid");
    not_opened_failure(
        builder,
        &parent_validation,
        "parent-validation-before-open",
        "parent_validation_failed",
    );

    let parent_handle = builder.decision(
        "open.parent-handle",
        namespace(
            "fn open_exact",
            "let parent = self.parent().map_err(|error|",
        ),
    );
    builder.edge(
        &parent_validation,
        &parent_handle,
        "parent_validation_succeeded",
    );
    not_opened_failure(
        builder,
        &parent_handle,
        "parent-handle",
        "parent_handle_failed",
    );

    let native_open = builder.decision(
        "open.platform-open",
        namespace(
            "fn open_exact",
            "platform::open_sqlite_file_relative(parent, kind, access, mode)",
        ),
    );
    builder.edge(&parent_handle, &native_open, "parent_handle_acquired");
    not_opened_failure(
        builder,
        &native_open,
        "platform-open",
        "platform_open_failed",
    );

    let completion = builder.decision(
        "open.completion-validation",
        namespace(
            "fn open_exact",
            "if let Err(error) = validate_open_completion",
        ),
    );
    builder.edge(&native_open, &completion, "platform_open_succeeded");
    rejected_open_failure(
        builder,
        &completion,
        "completion-validation",
        "open_completion_rejected",
    );

    let identity = builder.decision(
        "open.file-validation",
        namespace(
            "fn open_exact",
            "let identity = match self.validate_file(&file)",
        ),
    );
    builder.edge(&completion, &identity, "open_completion_valid");
    rejected_open_failure(
        builder,
        &identity,
        "file-validation",
        "file_validation_rejected",
    );

    let parent_revalidation = builder.decision(
        "open.parent-validation-after-open",
        namespace(
            "fn open_exact",
            "if let Err(error) = self.validate_parent()",
        ),
    );
    builder.edge(&identity, &parent_revalidation, "file_validation_succeeded");
    rejected_open_failure(
        builder,
        &parent_revalidation,
        "parent-validation-after-open",
        "parent_revalidation_rejected",
    );

    let created = builder.decision(
        "open.created-state",
        namespace(
            "fn open_exact",
            "created: opened.information == FILE_CREATED",
        ),
    );
    builder.edge(
        &parent_revalidation,
        &created,
        "parent_revalidation_succeeded",
    );
    dms::build_exclusive(builder, &created, false, "opened_existing");
    dms::build_exclusive(builder, &created, true, "opened_created");
}

fn not_opened_failure(builder: &mut InitBuilder, from: &str, cell: &str, branch: &str) {
    let live = builder.decision(
        &format!("open.failure.{cell}.live-custody"),
        failure_custody(
            "fn retain_handle_custody",
            "if let Some(file) = custody.live",
        ),
    );
    builder.edge(from, &live, branch);
    let impossible_live = builder.excluded(
        &format!("open.failure.{cell}.live-custody-present"),
        ExclusionProof::TypeInvariant(
            "ManagedSqliteFileOpenFailure::not_opened constructs both custody fields as None",
        ),
        namespace("fn open_exact", "ManagedSqliteFileOpenFailure::not_opened"),
    );
    builder.edge(&live, &impossible_live, "live_custody_present");
    finish_absent_close_custody(builder, &live, cell, "live_custody_absent");
}

fn rejected_open_failure(builder: &mut InitBuilder, from: &str, cell: &str, branch: &str) {
    let live = builder.decision(
        &format!("open.failure.{cell}.live-custody"),
        failure_custody(
            "fn retain_handle_custody",
            "if let Some(file) = custody.live",
        ),
    );
    builder.edge(from, &live, branch);
    let impossible_absent = builder.excluded(
        &format!("open.failure.{cell}.live-custody-absent"),
        ExclusionProof::TypeInvariant(
            "open_exact uses ManagedSqliteFileOpenFailure::opened_rejected after native open, which stores Some live custody",
        ),
        namespace(
            "fn rejected_open",
            "ManagedSqliteFileOpenFailure::opened_rejected",
        ),
    );
    builder.edge(&live, &impossible_absent, "live_custody_absent");
    let close = builder.decision(
        &format!("open.failure.{cell}.live-close"),
        namespace_close(
            "self.close_with(platform::close_sqlite_file)",
            "platform::close_sqlite_file",
        ),
    );
    builder.edge(&live, &close, "live_custody_present_close");
    finish_absent_close_custody(
        builder,
        &close,
        &format!("{cell}.close-ok"),
        "live_close_succeeded",
    );
    finish_close_failure(
        builder,
        &close,
        &format!("{cell}.close-failed"),
        "live_close_failed",
    );
}

fn finish_absent_close_custody(builder: &mut InitBuilder, from: &str, cell: &str, branch: &str) {
    let close_custody = builder.decision(
        &format!("open.failure.{cell}.prior-close-custody"),
        failure_custody(
            "fn retain_handle_custody",
            "if let Some(failure) = custody.close_failure",
        ),
    );
    builder.edge(from, &close_custody, branch);
    let impossible = builder.excluded(
        &format!("open.failure.{cell}.prior-close-custody-present"),
        ExclusionProof::ControlFlow(
            "open_exact returns not_opened or opened_rejected and never constructs the close_failed variant used only by observation-open paths",
        ),
        namespace(
            "fn open_exact",
            "ManagedSqliteFileOpenFailure::not_opened",
        ),
    );
    builder.edge(&close_custody, &impossible, "prior_close_custody_present");
    builder.failure(
        &close_custody,
        &format!("open.{cell}.exact-sibling-open"),
        "no_close_failure",
        failure_custody(
            "fn consume_open_failure",
            "ManagedSqliteShmFailurePhase::ExactSiblingOpen",
        ),
        open_failure_shape("ExactSiblingOpen", CustodyState::Released),
    );
}

fn finish_close_failure(builder: &mut InitBuilder, from: &str, cell: &str, branch: &str) {
    let close_custody = builder.decision(
        &format!("open.failure.{cell}.prior-close-custody"),
        failure_custody(
            "fn retain_handle_custody",
            "if let Some(failure) = custody.close_failure",
        ),
    );
    builder.edge(from, &close_custody, branch);
    let impossible = builder.excluded(
        &format!("open.failure.{cell}.prior-close-custody-present"),
        ExclusionProof::ControlFlow(
            "opened_rejected stores live custody only; a second pre-existing close failure cannot accompany the live close result",
        ),
        namespace_types("fn opened_rejected", "close_custody: None"),
    );
    builder.edge(&close_custody, &impossible, "prior_close_custody_present");
    builder.failure(
        &close_custody,
        &format!("open.{cell}.file-close"),
        "prior_close_custody_absent",
        failure_custody(
            "fn consume_open_failure",
            "ManagedSqliteShmFailurePhase::FileClose",
        ),
        open_failure_shape("FileClose", CustodyState::Quarantined),
    );
}

fn open_failure_shape(phase: &'static str, file: CustodyState) -> FailureShape {
    FailureShape {
        phase,
        class: FailureClass::OutcomeUncertainPoisoned,
        mutation: MutationState::Uncertain,
        lock_uncertain: false,
        disposition: if phase == "FileClose" {
            TerminalDisposition::CleanupRewritten
        } else {
            TerminalDisposition::Quarantined
        },
        file,
        dms_lock: DmsLockCustody::NotReached,
        native_lock: 0,
        native_unlock: 0,
    }
}
