use super::{
    super::model::{
        CustodyState, DecisionStage, DmsLockCustody, ExclusionProof, TerminalDisposition,
    },
    builder::MapGraphBuilder,
    managed, projection, witnesses as w, MapMode,
};

pub(super) fn build(graph: &mut MapGraphBuilder, typed_entry: &str, mode: MapMode) {
    let prefix = format!("map.{}.typed", mode.name());

    let adapter = graph.decision(
        &format!("{prefix}.adapter-dispatch"),
        w::adapter("self.file.shm_map(region, region_size, mode).map_err(drop)?"),
    );
    graph.edge(
        typed_entry,
        &adapter,
        DecisionStage::Adapter,
        "protected_call_normal_execution",
    );
    let callback = graph.decision(
        &format!("{prefix}.callback-admission"),
        w::registry(
            "fn with_shm<T>",
            ".begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)",
        ),
    );
    graph.edge(
        &adapter,
        &callback,
        DecisionStage::CallbackAdmission,
        "registry_shm_dispatch",
    );
    add_custody_expect_exclusion(graph, &callback, &prefix);
    projection::callback_admission_failure(
        graph,
        &callback,
        &format!("{prefix}.admission-rejected"),
    );

    for (branch, needle) in [
        (
            "unsupported-file-role",
            "ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole",
        ),
        (
            "shm-detached",
            "ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached",
        ),
    ] {
        let cause = graph.decision(
            &format!("{prefix}.{branch}"),
            w::registry("fn with_shm<T>", needle),
        );
        graph.edge(&callback, &cause, DecisionStage::CallbackAdmission, branch);
        projection::operation_failure(
            graph,
            &cause,
            &format!("{prefix}.{branch}.projection"),
            projection::FailureSpec {
                phase: "CallbackAdmission",
                failure: super::super::model::FailureClass::RegistryRejected,
                mutation: super::super::model::MutationState::None,
                disposition: TerminalDisposition::Returned,
                file: CustodyState::Unchanged,
                mapping: CustodyState::NotReached,
                view: CustodyState::NotReached,
                payload: CustodyState::NotReached,
                counts: Default::default(),
                quarantine: false,
                lock_outcome_uncertain: false,
                dms_lock: DmsLockCustody::NotReached,
            },
        );
    }

    let admitted = graph.decision(
        &format!("{prefix}.managed-operation"),
        w::registry("fn with_shm<T>", "operation(shm).map_err"),
    );
    graph.edge(
        &callback,
        &admitted,
        DecisionStage::CallbackAdmission,
        "callback_lease_acquired_live_wal_main",
    );
    managed::build(graph, &admitted, mode);
}

fn add_custody_expect_exclusion(graph: &mut MapGraphBuilder, callback: &str, prefix: &str) {
    let excluded = graph.excluded(
        &format!("{prefix}.excluded.live-file-custody-missing"),
        ExclusionProof::TypeInvariant(
            "a live pinned-file operation owns Some(custody); only consuming close/Drop can take it, after which no mutable Map call remains possible",
        ),
        w::registry(
            "fn with_shm<T>",
            ".expect(\"live pinned file operation must retain exact custody\")",
        ),
    );
    graph.edge(
        callback,
        &excluded,
        DecisionStage::CallbackAdmission,
        "callback_acquired_but_live_file_custody_missing",
    );
}
