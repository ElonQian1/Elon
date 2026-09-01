//! Source-closure checks for q18 holder release on every post-arm exit.

use std::collections::BTreeSet;

use super::super::runner_admission::{
    NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_CREATED_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1,
};

fn scoped_source_v1(scope: &'static [(&'static str, &'static str)], path: &str) -> &'static str {
    scope
        .iter()
        .find(|(candidate, _)| *candidate == path)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("missing scoped q18 source {path}"))
}

fn assert_markers_in_order(source: &str, markers: &[&str]) {
    let mut offset = 0;
    for marker in markers {
        let relative = source[offset..]
            .find(marker)
            .unwrap_or_else(|| panic!("missing ordered q18 source marker {marker}"));
        offset += relative + marker.len();
    }
}

#[test]
fn q18_source_closure_aborts_every_post_arm_error_before_the_only_terminal_finish() {
    let q18 = NATIVE_ACQUIRE_CREATED_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;
    assert_eq!(q18.len(), 12);
    assert_eq!(
        q18.iter()
            .map(|(path, _)| *path)
            .collect::<BTreeSet<_>>()
            .len(),
        12
    );
    let inherited = NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1;
    let connection = scoped_source_v1(inherited, "managed_vfs/connection/lock_initialization.rs");
    let managed = scoped_source_v1(
        inherited,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime.rs",
    );
    let fixture = scoped_source_v1(
        q18,
        "managed_vfs/a2_dynamic_evidence/lock_runner/created_first_shared_busy_close_succeeded/fixture.rs",
    );
    let controller = scoped_source_v1(
        q18,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/controller/created_first_shared_busy_close_succeeded.rs",
    );
    assert_markers_in_order(
        connection,
        &[
            "let after_arm = (|| {",
            "match after_arm",
            "Err(error) => {",
            "observer.abort_created_first_shared_busy_close_succeeded_observation_v1()?;",
            "return Err(error);",
        ],
    );
    assert_markers_in_order(
        connection,
        &[
            "fn finish_after_terminal_custody_observed",
            "finish_created_first_shared_busy_close_succeeded_observation_v1",
            "Err(error) => {",
            "abort_created_first_shared_busy_close_succeeded_observation_v1",
            "return Err(error);",
        ],
    );
    assert_markers_in_order(
        managed,
        &[
            "fn abort_created_first_shared_busy_close_succeeded_observation_v1",
            "Err(poisoned) => {",
            "poisoned.into_inner()",
            "q18_abort_and_release(target)",
            "Err(CONTROLLER_POISONED)",
        ],
    );
    assert_markers_in_order(
        managed,
        &[
            "fn finish_created_first_shared_busy_close_succeeded_observation_v1",
            "let snapshot = match",
            "abort_created_first_shared_busy_close_succeeded_observation_v1()?;",
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_SNAPSHOT_FAILED",
            "let requested_lock = match requested_lock",
            "abort_created_first_shared_busy_close_succeeded_observation_v1()?;",
            "return Err(error);",
            "let finished = match",
            "Err(poisoned) => {",
            "poisoned.into_inner()",
            "q18_abort_and_release(target)",
            "return Err(CONTROLLER_POISONED);",
            "match finished",
            "Err(error) => {",
            "abort_created_first_shared_busy_close_succeeded_observation_v1();",
            "Err(error)",
        ],
    );
    assert_markers_in_order(
        controller,
        &[
            "pub(super) fn finish(",
            "let mut active = self",
            ".holder",
            ".release_explicit()?;",
            "validate_target(&active, target)?;",
            "validate_counts(&active.counts)?;",
            "validate_terminal(terminal)?;",
        ],
    );
    assert_markers_in_order(
        fixture,
        &[
            "let pending = fixture",
            "observe_main_shm_lock_created_first_shared_busy_close_succeeded_v1",
            "let inspection = (|| {",
            "terminal_custody_test_snapshot()",
            "match inspection",
            "Err(error) => {",
            "abort_after_inspection_failure()",
            "return Err(error);",
            "finish_after_terminal_custody_observed()",
        ],
    );
    assert_eq!(
        fixture.matches("abort_after_inspection_failure()").count(),
        1
    );
    assert_eq!(
        fixture
            .matches("finish_after_terminal_custody_observed()")
            .count(),
        1
    );
}

#[test]
fn q18_source_closure_uses_real_same_file_contention_and_never_synthesizes_busy() {
    let q18 = NATIVE_ACQUIRE_CREATED_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;
    let holder = scoped_source_v1(
        q18,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/created_first_shared_busy_close_succeeded.rs",
    );
    let target = scoped_source_v1(
        q18,
        "node_agent_managed_fs/sqlite_namespace_shm/node_initialization/created_first_shared_busy_close_succeeded.rs",
    );
    let fixture = scoped_source_v1(
        q18,
        "managed_vfs/a2_dynamic_evidence/lock_runner/created_first_shared_busy_close_succeeded/fixture.rs",
    );
    assert_markers_in_order(
        holder,
        &[
            "same_file_identity(target_identity, holder_identity)",
            "holder.as_raw_handle() == file.file.as_raw_handle()",
            "try_lock_sqlite_byte_range(&holder, SHM_DMS_OFFSET, 1, true)",
            "PlatformManagedSqliteLockAttempt::Acquired",
        ],
    );
    assert_markers_in_order(
        target,
        &[
            "ManagedSqliteShmTestQ18DmsHolderLeaseV1::acquire(target, &file)",
            "try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, false)",
            "Ok(PlatformManagedSqliteLockAttempt::Contended)",
            "record_q18_target_shared_contended(target)",
            "record_q18_target_close_attempt(target)",
            "file.close()",
            "record_q18_target_close_succeeded(target, receipt.kind())",
            "ManagedSqliteShmFailureClass::BusyAfterKnownMutation",
        ],
    );
    assert_eq!(target.matches("try_lock_sqlite_byte_range").count(), 1);
    assert_eq!(
        target
            .matches("ManagedSqliteShmTestQ18DmsHolderLeaseV1::acquire")
            .count(),
        1
    );
    assert_eq!(target.matches("file.close()").count(), 1);
    assert!(!target.contains("begin_test_fault"));
    assert!(!target.contains("activate_after_test_fault"));
    assert!(!target.contains("SQLITE_BUSY"));
    assert!(fixture.contains("value.callback.result_code() != ffi::SQLITE_IOERR_SHMLOCK"));
    assert!(!fixture.contains("ffi::SQLITE_BUSY"));
}
