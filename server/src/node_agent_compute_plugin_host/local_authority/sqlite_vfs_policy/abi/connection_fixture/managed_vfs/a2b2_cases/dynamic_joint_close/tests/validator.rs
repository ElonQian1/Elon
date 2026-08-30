use std::collections::BTreeSet;

use super::super::super::{case_key::CaseKey, model::EvidenceKind};
use super::super::{
    actual::*,
    validate::{
        frozen_joint_close_cases, select_frozen_case, validate_frozen_joint_close_exact_set,
        validate_joint_close_report_payload,
    },
};
use super::support::{actual_from_case, main_lock_actual, replace_field, sample_actual};

#[test]
fn joint_close_selectors_are_an_exact_bijection_over_frozen_static_keys() {
    let cases = frozen_joint_close_cases();
    validate_frozen_joint_close_exact_set(&cases).expect("JointClose exact static set");
    let mut keys = BTreeSet::new();
    for selector in JointCloseSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique JointClose Case");
        assert_eq!(case.evidence, EvidenceKind::StaticContract);
        assert!(keys.insert(CaseKey::from(case)));
    }
    assert_eq!(keys.len(), 36);
}

#[test]
fn joint_close_exact_set_rejects_static_key_drift_and_missing_members() {
    let mut cases = frozen_joint_close_cases();
    cases[0].target.route_ordinal = 2;
    assert!(validate_frozen_joint_close_exact_set(&cases).is_err());

    let mut cases = frozen_joint_close_cases();
    cases.pop();
    assert!(validate_frozen_joint_close_exact_set(&cases).is_err());
}

#[test]
fn joint_close_validator_matches_all_thirty_six_static_records() {
    let cases = frozen_joint_close_cases();
    for selector in JointCloseSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique JointClose Case");
        let actual = actual_from_case(selector, case, 43);
        let payload = actual.to_report_payload();
        let observation = validate_joint_close_report_payload(selector, &payload)
            .expect("static-shaped independently encoded JointClose actual");
        assert_eq!(observation.selector(), selector);
        assert_eq!(observation.registration_id(), 43);
        let (key, sealed) = observation.into_evidence_parts();
        assert_eq!(key, CaseKey::from(case));
        assert!(sealed.matches_exact(&payload));
    }
}

#[test]
fn joint_close_validator_rejects_parent_selector_and_every_numeric_field_mutation() {
    let selector = JointCloseSelector::PhysicalSuccess;
    let payload = sample_actual(selector).to_report_payload();
    assert!(validate_joint_close_report_payload(
        JointCloseSelector::MainFileCloseAfterKnown,
        &payload,
    )
    .is_err());

    for index in 2..85 {
        let fields: Vec<_> = payload.split(',').collect();
        let replacement = if fields[index] == "0" { "1" } else { "0" };
        assert!(
            validate_joint_close_report_payload(
                selector,
                &replace_field(&payload, index, replacement),
            )
            .is_err(),
            "numeric field {index} drift was accepted"
        );
    }
}

#[test]
fn main_lock_native_uncertain_requires_variant_prestate_offset_and_call_receipt() {
    let mut shared = main_lock_actual(
        JointCloseSelector::MainLockReleaseNativeUncertainShared,
        0,
        JointCloseMainLockPrestate::Shared,
        JointCloseMainLockOffsetClass::SharedRange,
    );
    let shared_payload = shared.to_report_payload();
    let shared_fields: Vec<_> = shared_payload.split(',').collect();
    assert_eq!(
        (shared_fields[6], shared_fields[9], shared_fields[10]),
        ("0", "1", "1")
    );
    assert_eq!(shared_fields[62], "0");
    assert_eq!(shared_fields[65], "1");
    assert!(validate_joint_close_report_payload(shared.selector, &shared_payload).is_ok());

    let reserved = main_lock_actual(
        JointCloseSelector::MainLockReleaseNativeUncertainReserved,
        1,
        JointCloseMainLockPrestate::ReservedShared,
        JointCloseMainLockOffsetClass::ReservedByte,
    );
    let reserved_payload = reserved.to_report_payload();
    let reserved_fields: Vec<_> = reserved_payload.split(',').collect();
    assert_eq!(
        (reserved_fields[6], reserved_fields[9], reserved_fields[10]),
        ("1", "2", "2")
    );
    assert!(validate_joint_close_report_payload(reserved.selector, &reserved_payload).is_ok());

    shared.identity.main_lock_offset_class = JointCloseMainLockOffsetClass::ReservedByte;
    assert!(
        validate_joint_close_report_payload(shared.selector, &shared.to_report_payload()).is_err()
    );
    shared.identity.main_lock_offset_class = JointCloseMainLockOffsetClass::SharedRange;
    shared.identity.main_lock_prestate = JointCloseMainLockPrestate::ReservedShared;
    assert!(
        validate_joint_close_report_payload(shared.selector, &shared.to_report_payload()).is_err()
    );
    shared.identity.main_lock_prestate = JointCloseMainLockPrestate::Shared;
    shared.identity.variant = 1;
    assert!(
        validate_joint_close_report_payload(shared.selector, &shared.to_report_payload()).is_err()
    );
    shared.identity.variant = 0;
    shared.counts.main_unlock_attempt = 0;
    assert!(
        validate_joint_close_report_payload(shared.selector, &shared.to_report_payload()).is_err()
    );
}
