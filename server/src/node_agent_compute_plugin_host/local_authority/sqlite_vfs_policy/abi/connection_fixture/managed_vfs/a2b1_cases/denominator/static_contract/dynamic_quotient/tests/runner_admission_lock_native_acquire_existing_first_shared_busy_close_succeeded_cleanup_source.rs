//! Source-closure checks for q19 holder release on every post-arm exit.

use std::collections::BTreeSet;

use super::super::runner_admission::{
    NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1,
    NATIVE_ACQUIRE_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1,
};

fn scoped_source_v1(scope: &'static [(&'static str, &'static str)], path: &str) -> &'static str {
    scope
        .iter()
        .find(|(candidate, _)| *candidate == path)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("missing scoped q19 source {path}"))
}

fn assert_markers_in_order(source: &str, markers: &[&str]) {
    let mut offset = 0;
    for marker in markers {
        let relative = source[offset..]
            .find(marker)
            .unwrap_or_else(|| panic!("missing ordered q19 source marker {marker}"));
        offset += relative + marker.len();
    }
}

#[test]
fn q19_source_closure_aborts_every_post_arm_error_before_the_only_terminal_finish() {
    let q19 = NATIVE_ACQUIRE_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;
    assert_eq!(q19.len(), 14);
    assert_eq!(
        q19.iter()
            .map(|(path, _)| *path)
            .collect::<BTreeSet<_>>()
            .len(),
        14
    );
    let inherited = NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1;
    let connection = scoped_source_v1(inherited, "managed_vfs/connection/lock_initialization.rs");
    let managed = scoped_source_v1(
        q19,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/existing_first_shared_busy_close_succeeded.rs",
    );
    let fixture = scoped_source_v1(
        q19,
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_shared_busy_close_succeeded/fixture.rs",
    );
    let controller = scoped_source_v1(
        q19,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/controller/existing_first_shared_busy_close_succeeded.rs",
    );
    assert_markers_in_order(
        connection,
        &[
            "let after_arm = (|| {",
            "match after_arm",
            "Err(error) => {",
            "observer.abort_existing_first_shared_busy_close_succeeded_observation_v1()?;",
            "return Err(error);",
        ],
    );
    assert_markers_in_order(
        connection,
        &[
            "fn finish_after_terminal_custody_observed",
            "finish_existing_first_shared_busy_close_succeeded_observation_v1",
            "Err(error) => {",
            "abort_existing_first_shared_busy_close_succeeded_observation_v1",
            "return Err(error);",
        ],
    );
    assert_markers_in_order(
        managed,
        &[
            "fn abort_existing_first_shared_busy_close_succeeded_observation_v1",
            "Err(poisoned) => {",
            "poisoned.into_inner()",
            "q19_abort_and_release(target)",
            "Err(CONTROLLER_POISONED)",
        ],
    );
    assert_markers_in_order(
        managed,
        &[
            "fn finish_existing_first_shared_busy_close_succeeded_observation_v1",
            "let snapshot = match",
            "abort_existing_first_shared_busy_close_succeeded_observation_v1()?;",
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_SNAPSHOT_FAILED",
            "let requested_lock = match requested_lock",
            "abort_existing_first_shared_busy_close_succeeded_observation_v1()?;",
            "return Err(error);",
            "let finished = match",
            "Err(poisoned) => {",
            "poisoned.into_inner()",
            "q19_abort_and_release(target)",
            "return Err(CONTROLLER_POISONED);",
            "match finished",
            "Err(error) => {",
            "abort_existing_first_shared_busy_close_succeeded_observation_v1();",
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
            "active.validate_target(target)?;",
            "validation::validate_completion(&active)?;",
            "validation::validate_terminal(terminal)?;",
            "validation::validate_requested_lock(&active, requested_lock)?;",
            "validation::validate_holder_values(target, holder_values)?;",
        ],
    );
    assert_markers_in_order(
        fixture,
        &[
            "let pending = fixture",
            "observe_main_shm_lock_existing_first_shared_busy_close_succeeded_v1",
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
fn q19_source_closure_uses_real_same_file_contention_and_never_synthesizes_busy() {
    let q19 = NATIVE_ACQUIRE_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;
    let holder = scoped_source_v1(
        q19,
        "node_agent_managed_fs/sqlite_namespace_shm/test_initialization_runtime/existing_first_shared_busy_close_succeeded.rs",
    );
    let target = scoped_source_v1(
        q19,
        "node_agent_managed_fs/sqlite_namespace_shm/node_initialization/existing_first_shared_busy_close_succeeded.rs",
    );
    let fixture = scoped_source_v1(
        q19,
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_shared_busy_close_succeeded/fixture.rs",
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
            "ManagedSqliteShmTestQ19DmsHolderLeaseV1::acquire(target, &file)",
            "try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, false)",
            "Ok(PlatformManagedSqliteLockAttempt::Contended)",
            "record_q19_target_shared_contended(target)",
            "record_q19_target_close_attempt(target)",
            "file.close()",
            "record_q19_target_close_succeeded(target, receipt.kind())",
            "ManagedSqliteShmFailureClass::BusyAfterKnownMutation",
        ],
    );
    assert_eq!(target.matches("try_lock_sqlite_byte_range").count(), 1);
    assert_eq!(
        target
            .matches("ManagedSqliteShmTestQ19DmsHolderLeaseV1::acquire")
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

#[test]
fn q19_source_closure_propagates_holder_release_uncertainty_on_every_cleanup_path() {
    let q19 = NATIVE_ACQUIRE_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;
    let target = scoped_source_v1(
        q19,
        "node_agent_managed_fs/sqlite_namespace_shm/node_initialization/existing_first_shared_busy_close_succeeded.rs",
    );
    let shared_node_initialization = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs"
    ));
    assert_markers_in_order(
        target,
        &[
            "if let Err(code) = self.store_q19_holder(target, holder) {",
            "code == \"NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_RELEASE_FAILED\"",
            "self.abort_q19_and_close(",
            "code,",
            "holder_release_failed,",
        ],
    );
    assert_markers_in_order(
        target,
        &[
            "if let Err(code) = self.record_q19_target_close_succeeded(target, receipt.kind())",
            "let release_failed = self.release_q19_holder(target).is_err();",
            "self.mark_poisoned(",
            "release_failed,",
            "ManagedSqliteShmFailure::poisoned_code(",
            "release_failed,",
        ],
    );
    assert_markers_in_order(
        target,
        &[
            "Err(close_failure) => {",
            "let release_failed = self.release_q19_holder(target).is_err();",
            "self.mark_poisoned(",
            "release_failed,",
            "ManagedSqliteShmFailure::poisoned(",
            "release_failed,",
        ],
    );
    assert_markers_in_order(
        target,
        &[
            "fn abort_q19_and_close(",
            "let release_failed = self.release_q19_holder(target).is_err();",
            "let uncertain = lock_outcome_uncertain || release_failed;",
            "self.mark_poisoned(state, phase, true, uncertain);",
            "ManagedSqliteShmFailure::poisoned_code(phase, code, true, uncertain)",
        ],
    );
    assert_markers_in_order(
        shared_node_initialization,
        &[
            "fn close_failed_open_file(",
            "let mutation = original.mutation_may_have_occurred();",
            "let lock_outcome_uncertain = original.lock_outcome_uncertain();",
            "self.mark_poisoned(",
            "mutation,",
            "lock_outcome_uncertain,",
            "ManagedSqliteShmFailure::poisoned(",
            "mutation,",
            "lock_outcome_uncertain,",
        ],
    );
}
