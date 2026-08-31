use std::{fs, path::PathBuf};

use super::super::a2b2_cases::{
    validate_unmap_report_payload, JointCloseSelector, UnmapActual, UnmapActualCounts,
    UnmapActualCustody, UnmapActualIdentity, UnmapActualTarget, UnmapActualTopology, UnmapCallback,
    UnmapCause, UnmapDmsCustody, UnmapFailureClass, UnmapLogicalRoutePhase, UnmapMode, UnmapNode,
    UnmapPath, UnmapPhase, UnmapRegistrationPhase, UnmapRegistryRoutePhase, UnmapRole,
    UnmapSelector, UnmapSqliteOutcome, UnmapTargetScope, UnmapTiming, UnmapTopology,
};
use super::{
    capture::drain_capped_for_test,
    child::{
        validate_payload_for_test, validated_receipt_for_record_test, ChildIdentityFingerprint,
        RegistrationCommitment, RootCommitment, ValidatedChildProcessReceipt,
    },
    cleanup::ValidatedParentCleanupReceipt,
    environment::{capture_root_binding, validate_git_sha, validate_sqlite_version},
    record::ValidatedUnmapCandidateRecord,
    SanitizedChildReport, WindowsDynamicEnvironment,
};

const NONCE: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn child_report_v2_extracts_one_bound_line_from_libtest_framing() {
    let root = test_root("parse");
    fs::create_dir_all(&root).expect("create report root");
    let actual = actual_payload(7);
    let line = SanitizedChildReport::encode_for_current_child(NONCE, &root, 7, &actual)
        .expect("encode bound child report");
    let stdout = format!("running 1 test\n{line}\ntest child ... ok\n");
    let report = SanitizedChildReport::parse_captured_stdout(stdout.as_bytes())
        .expect("parse bound child report");
    assert_eq!(report.actual_payload(), actual.as_str());
    fs::remove_dir_all(root).expect("remove report root");
}

#[test]
fn child_report_rejects_duplicate_or_unsafe_actual_fields() {
    let root = test_root("reject");
    fs::create_dir_all(&root).expect("create rejection root");
    let actual = actual_payload(7);
    let line = SanitizedChildReport::encode_for_current_child(NONCE, &root, 7, &actual)
        .expect("encode bound child report");
    let duplicate = format!("{line}\n{line}\n");
    assert_eq!(
        SanitizedChildReport::parse_captured_stdout(duplicate.as_bytes()).err(),
        Some("A2_DYNAMIC_CHILD_REPORT_DUPLICATE")
    );
    assert_eq!(
        validate_payload_for_test("a2b2rs1,success,0"),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2rs1", "fence-before", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2br1", "outstanding-callback-gate", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert!(validate_payload_for_test(&payload("a2b2br1", "fence-before", 7)).is_ok());
    assert!(validate_payload_for_test(&payload(
        "a2b2rl1",
        "registry-route-removal-publish-native",
        7,
    ))
    .is_ok());
    assert_eq!(
        validate_payload_for_test(&payload("a2b2rl1", "success", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert!(
        validate_payload_for_test(&payload("a2b2un1", "shared-delete-request-validation", 7,))
            .is_ok()
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2un1", "success", 7)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload("a2b2u1", "shared-delete-request-validation", 7,)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")
    );
    let mut noncanonical = vec!["0".to_owned(); 81];
    noncanonical[0] = "00".to_owned();
    assert_eq!(
        validate_payload_for_test(&format!("a2b2br1,fence-before,{}", noncanonical.join(","))),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    noncanonical[0] = "18446744073709551616".to_owned();
    assert_eq!(
        validate_payload_for_test(&format!("a2b2br1,fence-before,{}", noncanonical.join(","))),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    assert_eq!(
        SanitizedChildReport::encode_for_current_child(NONCE, &root, 0, &actual).err(),
        Some("A2_DYNAMIC_REGISTRATION_ID_INVALID")
    );
    fs::remove_dir_all(root).expect("remove rejection root");
}

#[test]
fn child_payload_accepts_every_frozen_unmap_selector() {
    for selector in UnmapSelector::ALL {
        validate_payload_for_test(&payload("a2b2un1", selector.report_name(), 7))
            .expect("accept one allow-listed canonical Unmap payload");
    }
}

#[test]
fn child_payload_accepts_exact_joint_close_width_without_widening_other_families() {
    for selector in JointCloseSelector::ALL {
        let payload = payload_with_count("a2b2jc1", selector.report_name(), 83);
        assert_eq!(payload.split(',').count(), 85);
        validate_payload_for_test(&payload)
            .expect("accept one allow-listed canonical JointClose payload");
    }

    for field_count in [81, 82, 84] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count(
                "a2b2jc1",
                JointCloseSelector::RawStateTakeRejected.report_name(),
                field_count,
            )),
            Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
        );
    }
    assert_eq!(
        validate_payload_for_test(&payload_with_count(
            "a2b2un1",
            UnmapSelector::SharedDeleteRequestValidation.report_name(),
            83,
        )),
        Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload_with_count("a2b2jc1", "unknown-member", 83)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
}

#[test]
fn child_payload_accepts_exact_map_request_budget_family_without_legacy_widening() {
    for selector in [
        "region-size-budget-completed",
        "region-count-budget-completed",
        "logical-size-budget-completed",
    ] {
        validate_payload_for_test(&payload_with_count("a2mapq2", selector, 67))
            .expect("accept one exact Map request-budget payload");
    }
    for field_count in [66, 68] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count(
                "a2mapq2",
                "region-count-budget-completed",
                field_count,
            )),
            Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
        );
    }
    assert_eq!(
        validate_payload_for_test(&payload_with_count(
            "a2mapq2",
            "allocation-granularity-completed",
            67,
        )),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload_with_count(
            "a2mapq1",
            "region-count-budget-completed",
            66,
        )),
        Err("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")
    );
}

#[test]
fn child_payload_accepts_exact_lock_request_validation_family_without_widening() {
    for selector in [
        "range-overflow-lock-shared-completed",
        "range-overflow-lock-exclusive-completed",
        "range-overflow-unlock-shared-completed",
        "range-overflow-unlock-exclusive-completed",
        "end-past-eight-lock-shared-completed",
        "end-past-eight-lock-exclusive-completed",
        "end-past-eight-unlock-shared-completed",
        "end-past-eight-unlock-exclusive-completed",
        "shared-multi-slot-lock-shared-completed",
        "shared-multi-slot-unlock-shared-completed",
    ] {
        validate_payload_for_test(&payload_with_count("a2lockq1", selector, 51))
            .expect("accept one exact Lock request-validation payload");
    }
    for field_count in [50, 52] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count(
                "a2lockq1",
                "range-overflow-lock-shared-completed",
                field_count,
            )),
            Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
        );
    }
    for selector in [
        "shared-multi-slot-lock-exclusive-completed",
        "shared-multi-slot-unlock-exclusive-completed",
        "range-overflow-lock-unknown-completed",
    ] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count("a2lockq1", selector, 51)),
            Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
        );
    }
    assert_eq!(
        validate_payload_for_test(&payload_with_count(
            "a2lockq0",
            "range-overflow-lock-shared-completed",
            51,
        )),
        Err("A2_DYNAMIC_CHILD_ACTUAL_VERSION_INVALID")
    );
}

#[test]
fn child_payload_accepts_exact_lock_lifecycle_family_with_frozen_width() {
    let selectors = lock_lifecycle_selectors();
    assert_eq!(selectors.len(), 104);
    for selector in selectors {
        let payload = payload_with_count("a2lockq2", &selector, 103);
        assert_eq!(payload.split(',').count(), 105);
        validate_payload_for_test(&payload)
            .expect("accept one exact canonical Lock lifecycle payload");
    }
}

#[test]
fn child_payload_rejects_lock_lifecycle_widening_and_q1_q2_cross_family_headers() {
    let canonical = "native-acquire-lock-shared-first0-count1-completed";
    for field_count in [102, 104] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count("a2lockq2", canonical, field_count,)),
            Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
        );
    }

    for selector in [
        "native-acquire-lock-shared-first0-count2-completed",
        "native-acquire-lock-exclusive-first7-count2-completed",
        "native-release-unlock-exclusive-first0-count0-completed",
        "shared-local-acquire-lock-exclusive-first0-count1-completed",
        "shared-local-release-unlock-shared-first8-count1-completed",
        "native-acquire-unlock-shared-first0-count1-completed",
        "unknown-lock-lifecycle-member",
    ] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count("a2lockq2", selector, 103)),
            Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
        );
    }

    assert_eq!(
        validate_payload_for_test(&payload_with_count("a2lockq3", canonical, 103)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload_with_count("a2lockq1", canonical, 103)),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
    assert_eq!(
        validate_payload_for_test(&payload_with_count(
            "a2lockq2",
            "range-overflow-lock-shared-completed",
            103,
        )),
        Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
    );
}

#[test]
fn child_payload_accepts_exact_lock_stored_poison_width_without_header_widening() {
    let canonical = "lock-exclusive-first0-count1-gate-none-lock-certain-retention-succeeded";
    validate_payload_for_test(&payload_with_count("a2lockq3", canonical, 135))
        .expect("accept one exact canonical Lock stored-poison payload header");
    for field_count in [134, 136] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count("a2lockq3", canonical, field_count,)),
            Err("A2_DYNAMIC_CHILD_ACTUAL_FIELDS_INVALID")
        );
    }
    for selector in [
        "lock-shared-first0-count2-gate-none-lock-certain-retention-succeeded",
        "lock-exclusive-first7-count2-gate-none-lock-certain-retention-succeeded",
        "lock-exclusive-first0-count1-gate-none-lock-certain-route-unknown",
        "unknown-lock-stored-poison-member",
    ] {
        assert_eq!(
            validate_payload_for_test(&payload_with_count("a2lockq3", selector, 135)),
            Err("A2_DYNAMIC_CHILD_ACTUAL_SELECTOR_INVALID")
        );
    }
}

#[test]
fn unmap_candidate_record_accepts_one_fully_bound_observation() {
    let selector = UnmapSelector::SharedKeepSuccess;
    let payload = unmap_success_payload(7);
    let observation = validate_unmap_report_payload(selector, &payload)
        .expect("validate exact Unmap observation");
    let child = validated_receipt_for_record_test(&payload, 7).expect("sealed Unmap child receipt");
    let (environment, cleanup) = record_witnesses(&child);
    let record = ValidatedUnmapCandidateRecord::validate(observation, environment, child, cleanup)
        .expect("bind exact Unmap candidate record");
    let report = record.report();
    assert_eq!(report.case_selector(), selector.report_name());
    assert_eq!(report.git_sha(), "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(report.target(), "elon-pc-node");
    assert_eq!(report.child_exit_code(), 0);
    assert!(report.parent_cleanup_deleted());
    assert!(report.actual_payload_commitment().starts_with("sha256:"));
}

#[test]
fn unmap_candidate_record_rejects_cross_family_registration_and_payload_splicing() {
    let selector = UnmapSelector::SharedKeepSuccess;
    let unmap_payload = unmap_success_payload(7);

    let observation = validate_unmap_report_payload(selector, &unmap_payload)
        .expect("validate exact Unmap observation");
    let child = validated_receipt_for_record_test(&payload("a2b2br1", "fence-before", 7), 7)
        .expect("sealed Barrier child receipt");
    let (environment, cleanup) = record_witnesses(&child);
    assert_eq!(
        ValidatedUnmapCandidateRecord::validate(observation, environment, child, cleanup).err(),
        Some("A2_DYNAMIC_PAYLOAD_FAMILY_BINDING_MISMATCH")
    );

    let observation = validate_unmap_report_payload(selector, &unmap_payload)
        .expect("validate exact Unmap observation");
    let child = validated_receipt_for_record_test(&unmap_payload, 8)
        .expect("sealed wrong-registration child receipt");
    let (environment, cleanup) = record_witnesses(&child);
    assert_eq!(
        ValidatedUnmapCandidateRecord::validate(observation, environment, child, cleanup).err(),
        Some("A2_DYNAMIC_REGISTRATION_ID_BINDING_MISMATCH")
    );

    let observation = validate_unmap_report_payload(selector, &unmap_payload)
        .expect("validate exact Unmap observation");
    let mut changed = unmap_payload.split(',').collect::<Vec<_>>();
    changed[22] = "1";
    let changed = changed.join(",");
    let child = validated_receipt_for_record_test(&changed, 7)
        .expect("sealed changed-payload child receipt");
    let (environment, cleanup) = record_witnesses(&child);
    assert_eq!(
        ValidatedUnmapCandidateRecord::validate(observation, environment, child, cleanup).err(),
        Some("A2_DYNAMIC_ACTUAL_PAYLOAD_BINDING_MISMATCH")
    );
}

#[test]
fn root_and_registration_commitments_bind_real_inputs_without_exposing_them() {
    let root = test_root("bindings");
    fs::create_dir_all(&root).expect("create binding root");
    let actual = actual_payload(7);
    let first = encoded_report(&root, 7, &actual);
    let different_registration = encoded_report(&root, 8, &actual);
    let changed_actual = encoded_report(&root, 7, &changed_actual_payload());
    assert!(first.root_commitment == different_registration.root_commitment);
    assert!(first.registration_commitment != different_registration.registration_commitment);
    assert!(first.registration_commitment != changed_actual.registration_commitment);
    let recomputed =
        capture_root_binding(&root, std::process::id(), NONCE).expect("recompute root binding");
    assert!(first.root_commitment == recomputed.commitment);
    fs::remove_dir_all(root).expect("remove binding root");
}

#[test]
fn capped_capture_drains_but_never_retains_beyond_the_limit() {
    assert_eq!(drain_capped_for_test(b"1234", 4), Ok(b"1234".to_vec()));
    assert_eq!(
        drain_capped_for_test(b"12345", 4),
        Err("A2_DYNAMIC_CHILD_TEST_TOO_LARGE")
    );
}

#[test]
fn environment_scalar_validators_reject_ambiguous_build_identity() {
    assert_eq!(
        validate_git_sha("0123456789abcdef0123456789abcdef01234567"),
        Ok("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(
        validate_git_sha("0123456"),
        Err("A2_DYNAMIC_GIT_SHA_INVALID")
    );
    assert!(validate_sqlite_version("3.45.0", 3_045_000).is_ok());
    assert_eq!(
        validate_sqlite_version("3.45.0", 3_046_000),
        Err("A2_DYNAMIC_SQLITE_VERSION_MISMATCH")
    );
}

fn encoded_report(
    root: &std::path::Path,
    registration_id: u64,
    actual: &str,
) -> SanitizedChildReport {
    let line = SanitizedChildReport::encode_for_current_child(NONCE, root, registration_id, actual)
        .expect("encode child report");
    SanitizedChildReport::parse_captured_stdout(line.as_bytes()).expect("parse child report")
}

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-a2-dynamic-evidence-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ))
}

fn actual_payload(registration_id: u64) -> String {
    payload("a2b2rs1", "success", registration_id)
}

fn payload(version: &str, selector: &str, registration_id: u64) -> String {
    let mut fields = vec!["0".to_owned(); 81];
    fields[12] = registration_id.to_string();
    format!("{version},{selector},{}", fields.join(","))
}

fn payload_with_count(version: &str, selector: &str, field_count: usize) -> String {
    format!("{version},{selector},{}", vec!["0"; field_count].join(","))
}

fn lock_lifecycle_selectors() -> std::collections::BTreeSet<String> {
    let mut selectors = std::collections::BTreeSet::new();
    for first in 0..8u8 {
        assert!(selectors.insert(format!(
            "native-acquire-lock-shared-first{first}-count1-completed"
        )));
        assert!(selectors.insert(format!(
            "native-release-unlock-shared-first{first}-count1-completed"
        )));
        assert!(selectors.insert(format!(
            "shared-local-acquire-lock-shared-first{first}-count1-completed"
        )));
        assert!(selectors.insert(format!(
            "shared-local-release-unlock-shared-first{first}-count1-completed"
        )));
    }
    for count in 1..=8u8 {
        for first in 0..=8 - count {
            assert!(selectors.insert(format!(
                "native-acquire-lock-exclusive-first{first}-count{count}-completed"
            )));
            assert!(selectors.insert(format!(
                "native-release-unlock-exclusive-first{first}-count{count}-completed"
            )));
        }
    }
    selectors
}

fn changed_actual_payload() -> String {
    let mut fields = vec!["0".to_owned(); 81];
    fields[12] = "7".to_owned();
    fields[20] = "1".to_owned();
    format!("a2b2rs1,success,{}", fields.join(","))
}

fn unmap_success_payload(registration_id: u64) -> String {
    UnmapActual {
        selector: UnmapSelector::SharedKeepSuccess,
        identity: UnmapActualIdentity {
            path: UnmapPath::Unmap,
            topology: UnmapTopology::SharedNonFinal,
            mode: UnmapMode::Keep,
            node: UnmapNode::Live,
            variant: 0,
            pre_shared_mask: 0,
            pre_exclusive_mask: 0,
            phase: UnmapPhase::Success,
            cause: UnmapCause::None,
            timing: UnmapTiming::Success,
            class: UnmapFailureClass::None,
            target: UnmapActualTarget {
                scope: UnmapTargetScope::RouteMain,
                registration_id,
                route_ordinal: 1,
                runtime_generation: 1,
                shm_connection_id: 1,
                role: UnmapRole::Main,
                callback: UnmapCallback::Shm,
                occurrence: 1,
            },
            sqlite_outcome: UnmapSqliteOutcome::Ok,
        },
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: UnmapRegistryRoutePhase::Active,
        logical_route_phase: UnmapLogicalRoutePhase::Indexed,
        registration_phase: UnmapRegistrationPhase::Registered,
        later_callback_allowed: true,
        pre: UnmapActualTopology {
            sqlite_connections: 2,
            shm_connections: 2,
            registry_routes: 2,
            logical_names: 6,
        },
        post: UnmapActualTopology {
            sqlite_connections: 2,
            shm_connections: 1,
            registry_routes: 2,
            logical_names: 6,
        },
        retained: UnmapActualCustody {
            node: true,
            views: 1,
            mappings: 1,
            dms: UnmapDmsCustody::Shared,
            shm_file: true,
            main_file: true,
            main_lock_owner: true,
            main_lease: true,
            shm_lease: true,
            callback_leases: 0,
            registry_entry: true,
            logical_names: 3,
            vfs_table: true,
            vfs_name: true,
            vfs_context: true,
            root_deletable: false,
        },
        counts: UnmapActualCounts {
            callback_begin: 1,
            callback_complete_attempt: 1,
            callback_complete_success: 1,
            selected_action_attempt: 1,
            selected_action_success: 1,
            shm_detach: 1,
            ..UnmapActualCounts::default()
        },
    }
    .to_report_payload()
}

fn record_witnesses(
    child: &ValidatedChildProcessReceipt,
) -> (WindowsDynamicEnvironment, ValidatedParentCleanupReceipt) {
    let child_fingerprint = child.fingerprint().0;
    let root_commitment = child.root_commitment.0;
    let registration_commitment = child.registration_commitment.0;
    (
        WindowsDynamicEnvironment {
            git_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            target: "elon-pc-node",
            windows_build: "test-build".to_owned(),
            architecture: "x86_64",
            volume_kind: "fixed",
            filesystem: "NTFS".to_owned(),
            bundled_sqlite: "3.45.0".to_owned(),
            child_fingerprint: ChildIdentityFingerprint(child_fingerprint),
            registration_commitment: RegistrationCommitment(registration_commitment),
            canonical_root: PathBuf::from(r"C:\sealed-record-test-root"),
            root_commitment: RootCommitment(root_commitment),
        },
        ValidatedParentCleanupReceipt {
            child_fingerprint: ChildIdentityFingerprint(child_fingerprint),
            root_commitment: RootCommitment(root_commitment),
            registration_commitment: RegistrationCommitment(registration_commitment),
        },
    )
}
