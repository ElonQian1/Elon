use super::super::terminal_descriptor::{
    InitializationFaultSiteV1, InitializationPathV1, InitializationProfileV1,
    InitializationStimulusV1, OccurrenceV1, PhaseV1, TimingV1,
};
use super::{
    init, windows_locking, CustodyState, DmsLockCustody, FailureClass, FailureShape, InitBuilder,
    MutationState, TerminalDisposition,
};

#[derive(Debug, Clone, Copy)]
struct Prefix {
    label: &'static str,
    profile: InitializationProfileV1,
    path: InitializationPathV1,
    mutation: MutationState,
    native_lock: u16,
    native_unlock: u16,
}

pub(super) fn build_exclusive(builder: &mut InitBuilder, from: &str, created: bool, branch: &str) {
    let label = if created { "created" } else { "existing" };
    let native = builder.decision(
        &format!("dms.{label}.exclusive-acquire"),
        windows_locking(
            "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
            "LockFileEx(",
        ),
    );
    builder.edge(from, &native, branch);
    add_exclusive_error(builder, &native, created, label);
    build_shared(
        builder,
        &native,
        Prefix {
            label: if created {
                "created-joiner"
            } else {
                "existing-joiner"
            },
            profile: if created {
                InitializationProfileV1::CreatedJoinerShared
            } else {
                InitializationProfileV1::ExistingJoinerShared
            },
            path: if created {
                InitializationPathV1::CreatedJoiner
            } else {
                InitializationPathV1::ExistingJoiner
            },
            mutation: if created {
                MutationState::Known
            } else {
                MutationState::None
            },
            native_lock: 1,
            native_unlock: 0,
        },
        "exclusive_contended_joiner",
    );
    build_first_process(
        builder,
        &native,
        created,
        "exclusive_acquired_first_process",
    );
}

fn add_exclusive_error(builder: &mut InitBuilder, native: &str, created: bool, label: &str) {
    if created {
        close_after_failure(
            builder,
            native,
            &format!("{label}.exclusive-error-known-mutation"),
            "exclusive_error_after_file_create",
            FailureShape {
                phase: PhaseV1::DmsExclusiveAcquire,
                stimulus: initialization_stimulus(
                    InitializationFaultSiteV1::DmsExclusiveAcquire,
                    InitializationPathV1::Created,
                ),
                timing: TimingV1::AtCall,
                occurrence: OccurrenceV1::Natural,
                class: FailureClass::MutatedButKnown,
                mutation: MutationState::Known,
                lock_uncertain: false,
                disposition: TerminalDisposition::Returned,
                file: CustodyState::Released,
                dms_lock: DmsLockCustody::Released,
                native_lock: 1,
                native_unlock: 0,
            },
        );
        return;
    }
    let class = builder.decision(
        &format!("dms.{label}.exclusive-error-class"),
        init(
            "fn classify_platform",
            "error.kind() == io::ErrorKind::Unsupported",
        ),
    );
    builder.edge(native, &class, "exclusive_error_before_mutation");
    for (kind, failure) in [
        ("io", FailureClass::IoBeforeMutation),
        ("unsupported", FailureClass::PlatformUnsupported),
    ] {
        close_after_failure(
            builder,
            &class,
            &format!("{label}.exclusive-error-{kind}"),
            &format!("classified_{kind}"),
            FailureShape {
                phase: PhaseV1::DmsExclusiveAcquire,
                stimulus: initialization_stimulus(
                    InitializationFaultSiteV1::DmsExclusiveAcquire,
                    InitializationPathV1::Existing,
                ),
                timing: TimingV1::AtCall,
                occurrence: OccurrenceV1::Natural,
                class: failure,
                mutation: MutationState::None,
                lock_uncertain: false,
                disposition: TerminalDisposition::Returned,
                file: CustodyState::Released,
                dms_lock: DmsLockCustody::Released,
                native_lock: 1,
                native_unlock: 0,
            },
        );
    }
}

fn build_first_process(builder: &mut InitBuilder, from: &str, created: bool, branch: &str) {
    let label = if created {
        "created-first"
    } else {
        "existing-first"
    };
    let truncate = builder.decision(
        &format!("dms.{label}.truncate"),
        init("fn open_node", "if let Err(error) = file.truncate(0)"),
    );
    builder.edge(from, &truncate, branch);
    let cleanup_release = builder.decision(
        &format!("dms.{label}.truncate-error-release"),
        windows_locking(
            "pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range(",
            "UnlockFileEx(",
        ),
    );
    builder.edge(
        &truncate,
        &cleanup_release,
        "truncate_failed_release_exclusive",
    );
    builder.failure(
        &cleanup_release,
        &format!("dms.{label}.truncate-error.release-ok"),
        "exclusive_release_succeeded",
        init("fn open_node", "ManagedSqliteShmFailurePhase::DmsTruncate"),
        FailureShape {
            phase: PhaseV1::DmsTruncate,
            stimulus: initialization_stimulus(
                InitializationFaultSiteV1::DmsTruncate,
                first_path(created),
            ),
            timing: TimingV1::AtCall,
            occurrence: OccurrenceV1::Natural,
            class: FailureClass::OutcomeUncertainPoisoned,
            mutation: MutationState::Uncertain,
            lock_uncertain: false,
            disposition: TerminalDisposition::Quarantined,
            file: CustodyState::Retained,
            dms_lock: DmsLockCustody::Released,
            native_lock: 1,
            native_unlock: 1,
        },
    );
    builder.failure(
        &cleanup_release,
        &format!("dms.{label}.truncate-error.release-failed"),
        "exclusive_release_failed",
        init(
            "fn open_node",
            "ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain",
        ),
        FailureShape {
            phase: PhaseV1::DmsExclusiveRelease,
            stimulus: initialization_stimulus(
                InitializationFaultSiteV1::DmsExclusiveRelease,
                first_path(created),
            ),
            timing: TimingV1::Cleanup,
            occurrence: OccurrenceV1::Natural,
            class: FailureClass::OutcomeUncertainPoisoned,
            mutation: MutationState::Uncertain,
            lock_uncertain: true,
            disposition: TerminalDisposition::CleanupRewritten,
            file: CustodyState::Retained,
            dms_lock: DmsLockCustody::ExclusiveOutcomeUncertain,
            native_lock: 1,
            native_unlock: 1,
        },
    );

    let release = builder.decision(
        &format!("dms.{label}.exclusive-release"),
        windows_locking(
            "pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range(",
            "UnlockFileEx(",
        ),
    );
    builder.edge(&truncate, &release, "truncate_succeeded_release_exclusive");
    builder.failure(
        &release,
        &format!("dms.{label}.exclusive-release-error"),
        "exclusive_release_failed",
        init(
            "fn open_node",
            "ManagedSqliteShmFailurePhase::DmsExclusiveRelease",
        ),
        FailureShape {
            phase: PhaseV1::DmsExclusiveRelease,
            stimulus: initialization_stimulus(
                InitializationFaultSiteV1::DmsExclusiveRelease,
                first_path(created),
            ),
            timing: TimingV1::AtCall,
            occurrence: OccurrenceV1::Natural,
            class: FailureClass::OutcomeUncertainPoisoned,
            mutation: MutationState::Uncertain,
            lock_uncertain: true,
            disposition: TerminalDisposition::Quarantined,
            file: CustodyState::Retained,
            dms_lock: DmsLockCustody::ExclusiveOutcomeUncertain,
            native_lock: 1,
            native_unlock: 1,
        },
    );
    build_shared(
        builder,
        &release,
        Prefix {
            label,
            profile: if created {
                InitializationProfileV1::CreatedFirstShared
            } else {
                InitializationProfileV1::ExistingFirstShared
            },
            path: first_path(created),
            mutation: MutationState::Known,
            native_lock: 1,
            native_unlock: 1,
        },
        "exclusive_release_succeeded",
    );
}

fn build_shared(builder: &mut InitBuilder, from: &str, prefix: Prefix, branch: &str) {
    let native = builder.decision(
        &format!("dms.{}.shared-acquire", prefix.label),
        windows_locking(
            "pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range",
            "LockFileEx(",
        ),
    );
    builder.edge(from, &native, branch);
    builder.success(
        &native,
        success_label(prefix.label),
        prefix.profile,
        "shared_acquired",
        true,
        prefix.mutation,
        DmsLockCustody::AcquiredShared,
        prefix.native_lock + 1,
        prefix.native_unlock,
    );
    let busy = if prefix.mutation == MutationState::None {
        FailureClass::BusyNoMutation
    } else {
        FailureClass::BusyAfterKnownMutation
    };
    close_after_failure(
        builder,
        &native,
        &format!("{}.shared-busy", prefix.label),
        "shared_contended",
        shared_failure(prefix, busy),
    );
    if prefix.mutation != MutationState::None {
        close_after_failure(
            builder,
            &native,
            &format!("{}.shared-error-known-mutation", prefix.label),
            "shared_error_after_known_mutation",
            shared_failure(prefix, FailureClass::MutatedButKnown),
        );
    } else {
        let class = builder.decision(
            &format!("dms.{}.shared-error-class", prefix.label),
            init(
                "fn classify_platform",
                "error.kind() == io::ErrorKind::Unsupported",
            ),
        );
        builder.edge(&native, &class, "shared_error_before_mutation");
        for (kind, failure) in [
            ("io", FailureClass::IoBeforeMutation),
            ("unsupported", FailureClass::PlatformUnsupported),
        ] {
            close_after_failure(
                builder,
                &class,
                &format!("{}.shared-error-{kind}", prefix.label),
                &format!("classified_{kind}"),
                shared_failure(prefix, failure),
            );
        }
    }
}

fn shared_failure(prefix: Prefix, class: FailureClass) -> FailureShape {
    FailureShape {
        phase: PhaseV1::DmsSharedAcquire,
        stimulus: initialization_stimulus(InitializationFaultSiteV1::DmsSharedAcquire, prefix.path),
        timing: TimingV1::AtCall,
        occurrence: OccurrenceV1::Natural,
        class,
        mutation: prefix.mutation,
        lock_uncertain: false,
        disposition: TerminalDisposition::Returned,
        file: CustodyState::Released,
        dms_lock: DmsLockCustody::Released,
        native_lock: prefix.native_lock + 1,
        native_unlock: prefix.native_unlock,
    }
}

const fn first_path(created: bool) -> InitializationPathV1 {
    if created {
        InitializationPathV1::CreatedFirst
    } else {
        InitializationPathV1::ExistingFirst
    }
}

const fn initialization_stimulus(
    fault_site: InitializationFaultSiteV1,
    path: InitializationPathV1,
) -> InitializationStimulusV1 {
    InitializationStimulusV1 {
        fault_site,
        path,
        cleanup_rewrite: false,
    }
}

fn success_label(prefix: &str) -> &'static str {
    match prefix {
        "created-first" => "created-first-shared",
        "created-joiner" => "created-joiner-shared",
        "existing-first" => "existing-first-shared",
        "existing-joiner" => "existing-joiner-shared",
        _ => unreachable!("fixed DMS prefix"),
    }
}

fn close_after_failure(
    builder: &mut InitBuilder,
    from: &str,
    cell: &str,
    branch: &str,
    shape: FailureShape,
) {
    let close = builder.decision(
        &format!("dms.{cell}.close-failed-open-file"),
        init("fn close_failed_open_file", "match file.close()"),
    );
    builder.edge(from, &close, branch);
    builder.failure(
        &close,
        &format!("dms.{cell}.close-ok"),
        "file_close_succeeded",
        init("fn close_failed_open_file", "Ok(_) => original"),
        shape,
    );
    builder.failure(
        &close,
        &format!("dms.{cell}.close-failed"),
        "file_close_failed_cleanup_rewrite",
        init("fn close_failed_open_file", "Err(close_failure) =>"),
        shape.cleanup_rewrite(),
    );
}
