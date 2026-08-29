use std::collections::BTreeSet;

use super::super::{barrier, case_key::CaseKey, model::EvidenceKind};
use super::{
    actual::*,
    validate::{
        select_frozen_case, validate_barrier_report_payload, validate_frozen_barrier_exact_set,
    },
};

#[test]
fn barrier_eight_selectors_are_an_exact_bijection_over_frozen_cases() {
    let cases = barrier::cases();
    validate_frozen_barrier_exact_set(&cases).expect("Barrier selector exact set");
    let mut keys = BTreeSet::new();
    for selector in BarrierSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique Barrier selector");
        assert_eq!(case.evidence, EvidenceKind::StaticContract);
        assert!(keys.insert(CaseKey::from(case)));
    }
    assert_eq!(keys.len(), 8);
}

#[test]
fn barrier_exact_set_rejects_case_key_drift_outside_selector_signature() {
    let mut cases = barrier::cases();
    let success = cases.last_mut().expect("Barrier success Case");
    success.target.route_ordinal = 2;
    assert!(validate_frozen_barrier_exact_set(&cases).is_err());
}

#[test]
fn barrier_eight_independently_authored_actuals_match_all_frozen_fields() {
    for selector in BarrierSelector::ALL {
        let actual = independently_observed_actual(selector, 37);
        let payload = actual.to_report_payload();
        assert_eq!(payload.split(',').count(), 83);
        let observation = validate_barrier_report_payload(selector, &payload)
            .expect("independently authored Barrier actual");
        assert_eq!(observation.selector(), selector);
        assert_eq!(observation.registration_id(), 37);
        let (_key, sealed) = observation.into_evidence_parts();
        assert!(sealed.matches_exact(&payload));
        assert_eq!(sealed.exact_payload(), payload);
    }
}

#[test]
fn barrier_codec_rejects_wrong_version_and_field_quantity() {
    let payload = independently_observed_actual(BarrierSelector::Success, 41).to_report_payload();
    let wrong_version = payload.replacen("a2b2br1", "a2b2br2", 1);
    assert!(validate_barrier_report_payload(BarrierSelector::Success, &wrong_version).is_err());

    let mut fields: Vec<_> = payload.split(',').collect();
    fields.pop();
    assert!(validate_barrier_report_payload(BarrierSelector::Success, &fields.join(",")).is_err());
    fields.push("0");
    fields.push("0");
    assert!(validate_barrier_report_payload(BarrierSelector::Success, &fields.join(",")).is_err());
}

#[test]
fn barrier_codec_rejects_leading_zero_and_selector_drift() {
    let payload = independently_observed_actual(BarrierSelector::Success, 43).to_report_payload();
    let mut fields: Vec<_> = payload.split(',').collect();
    fields[14] = "01";
    assert!(validate_barrier_report_payload(BarrierSelector::Success, &fields.join(",")).is_err());

    assert!(validate_barrier_report_payload(BarrierSelector::FenceBefore, &payload).is_err());
    let unknown = payload.replacen(",success,", ",barrier-success,", 1);
    assert!(validate_barrier_report_payload(BarrierSelector::Success, &unknown).is_err());
}

#[test]
fn barrier_validator_rejects_identity_state_custody_and_field_drift() {
    let selector = BarrierSelector::FenceAfter;

    let mut identity = independently_observed_actual(selector, 47);
    identity.identity.target.registration_id = 0;
    assert!(validate_barrier_report_payload(selector, &identity.to_report_payload()).is_err());

    let mut state = independently_observed_actual(selector, 47);
    state.domain_terminal = false;
    assert!(validate_barrier_report_payload(selector, &state.to_report_payload()).is_err());

    let mut custody = independently_observed_actual(selector, 47);
    custody.retained.shm_lease = false;
    assert!(validate_barrier_report_payload(selector, &custody.to_report_payload()).is_err());

    let mut counts = independently_observed_actual(selector, 47);
    counts.counts.selected_action_success = 0;
    assert!(validate_barrier_report_payload(selector, &counts.to_report_payload()).is_err());
}

fn independently_observed_actual(selector: BarrierSelector, registration_id: u64) -> BarrierActual {
    let topology = BarrierActualTopology {
        sqlite_connections: 2,
        shm_connections: 2,
        registry_routes: 2,
        logical_names: 6,
    };
    let retained = BarrierActualCustody {
        node: true,
        views: 1,
        mappings: 1,
        dms: BarrierDmsCustody::Shared,
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
    };
    let success_counts = BarrierActualCounts {
        callback_begin: 1,
        callback_complete_attempt: 1,
        callback_complete_success: 1,
        selected_action_attempt: 1,
        selected_action_success: 1,
        ..BarrierActualCounts::default()
    };
    let mut actual = BarrierActual {
        selector,
        identity: BarrierActualIdentity {
            path_is_barrier: true,
            topology_is_shared_non_final: true,
            unmap_is_not_applicable: true,
            node_is_live: true,
            variant: 0,
            pre_shared_mask: 0,
            pre_exclusive_mask: 0,
            phase: BarrierPhase::Success,
            cause_phase_is_none: true,
            timing: BarrierTiming::Success,
            class: BarrierFailureClass::None,
            target: BarrierActualTarget {
                scope_is_route_main: true,
                registration_id,
                route_ordinal: 1,
                runtime_generation: 1,
                shm_connection_id: 1,
                role_is_main: true,
                callback_is_shm: true,
                occurrence: 1,
            },
            sqlite_outcome_is_void_no_result_code: true,
        },
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: BarrierRegistryRoutePhase::Active,
        logical_route_phase: BarrierLogicalRoutePhase::Indexed,
        registration_phase: BarrierRegistrationPhase::Registered,
        later_callback_allowed: true,
        pre: topology,
        post: topology,
        retained,
        counts: success_counts,
    };
    if selector != BarrierSelector::Success {
        actual.registry_route_phase = BarrierRegistryRoutePhase::TerminalQuarantine;
        actual.logical_route_phase = BarrierLogicalRoutePhase::Retained;
        actual.later_callback_allowed = false;
        actual.counts = BarrierActualCounts {
            raw_state_abandon: 1,
            methods_clear: 1,
            custody_retain: 1,
            ..BarrierActualCounts::default()
        };
    }
    match selector {
        BarrierSelector::AdmissionRejected => failure_identity(
            &mut actual,
            BarrierPhase::CallbackAdmission,
            BarrierTiming::BeforeCall,
            BarrierFailureClass::RegistryRejected,
        ),
        BarrierSelector::WrapperBefore => {
            failure_identity(
                &mut actual,
                BarrierPhase::BarrierFence,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::IoBeforeMutation,
            );
            actual.identity.variant = 1;
            observe_trigger(&mut actual);
        }
        BarrierSelector::FenceBefore => {
            failure_identity(
                &mut actual,
                BarrierPhase::BarrierFence,
                BarrierTiming::BeforeCall,
                BarrierFailureClass::IoBeforeMutation,
            );
            actual.domain_terminal = true;
            actual.counts.callback_begin = 1;
            actual.retained.callback_leases = 1;
            observe_trigger(&mut actual);
        }
        BarrierSelector::FenceAfter => {
            failure_identity(
                &mut actual,
                BarrierPhase::BarrierFence,
                BarrierTiming::AfterSuccessUncertain,
                BarrierFailureClass::OutcomeUncertainPoisoned,
            );
            actual.domain_terminal = true;
            actual.counts.callback_begin = 1;
            actual.counts.selected_action_attempt = 1;
            actual.counts.selected_action_success = 1;
            actual.retained.callback_leases = 1;
            observe_trigger(&mut actual);
        }
        BarrierSelector::CompletionBefore => {
            completion(&mut actual, BarrierTiming::BeforeCall, 0, 0, 1);
            observe_trigger(&mut actual);
        }
        BarrierSelector::CompletionNativeUncertain => {
            completion(&mut actual, BarrierTiming::NativeUncertain, 1, 0, 1);
            actual.counts.fault_observe = 1;
        }
        BarrierSelector::CompletionAfterSuccessKnown => {
            completion(&mut actual, BarrierTiming::AfterSuccessKnown, 1, 1, 0);
            observe_trigger(&mut actual);
        }
        BarrierSelector::Success => {}
    }
    actual
}

fn failure_identity(
    actual: &mut BarrierActual,
    phase: BarrierPhase,
    timing: BarrierTiming,
    class: BarrierFailureClass,
) {
    actual.identity.phase = phase;
    actual.identity.timing = timing;
    actual.identity.class = class;
}

fn observe_trigger(actual: &mut BarrierActual) {
    actual.counts.fault_observe = 1;
    actual.counts.fault_trigger = 1;
}

fn completion(
    actual: &mut BarrierActual,
    timing: BarrierTiming,
    complete_attempt: u8,
    complete_success: u8,
    callback_leases: u8,
) {
    failure_identity(
        actual,
        BarrierPhase::CallbackCompletion,
        timing,
        BarrierFailureClass::RegistryRejected,
    );
    actual.counts.callback_begin = 1;
    actual.counts.callback_complete_attempt = complete_attempt;
    actual.counts.callback_complete_success = complete_success;
    actual.counts.selected_action_attempt = 1;
    actual.counts.selected_action_success = 1;
    actual.retained.callback_leases = callback_leases;
}
