use std::collections::BTreeSet;

use super::super::{case_key::CaseKey, close_registry, model::EvidenceKind};
use super::{
    actual::*,
    validate::{
        select_frozen_case, validate_frozen_registry_lifecycle_exact_set,
        validate_registry_lifecycle_report_payload,
    },
};

#[test]
fn registry_lifecycle_sixteen_selectors_exactly_cover_the_frozen_family() {
    let cases = close_registry::cases();
    validate_frozen_registry_lifecycle_exact_set(&cases).expect("RegistryLifecycle exact set");
    let mut keys = BTreeSet::new();
    for selector in RegistryLifecycleSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique RegistryLifecycle Case");
        assert_eq!(case.evidence, EvidenceKind::StaticContract);
        assert!(keys.insert(CaseKey::from(case)));
    }
    assert_eq!(keys.len(), 16);
}

#[test]
fn registry_lifecycle_exact_set_rejects_case_key_drift() {
    let mut cases = close_registry::cases();
    cases
        .iter_mut()
        .find(|case| case.path == super::super::model::Path::RegistryLifecycle)
        .expect("RegistryLifecycle Case")
        .target
        .route_ordinal = 2;
    assert!(validate_frozen_registry_lifecycle_exact_set(&cases).is_err());
}

#[test]
fn registry_lifecycle_independent_actuals_match_all_sixteen_cases() {
    for selector in RegistryLifecycleSelector::ALL {
        let actual = independently_observed_actual(selector, 37);
        let payload = actual.to_report_payload();
        assert_eq!(payload.split(',').count(), 83);
        let observation = validate_registry_lifecycle_report_payload(selector, &payload)
            .expect("independently authored RegistryLifecycle actual");
        assert_eq!(observation.selector(), selector);
        assert_eq!(observation.registration_id(), 37);
        let (_key, sealed) = observation.into_evidence_parts();
        assert!(sealed.matches_exact(&payload));
        assert_eq!(sealed.exact_payload(), payload);
    }
}

#[test]
fn registry_lifecycle_codec_rejects_version_quantity_and_noncanonical_fields() {
    let payload = independently_observed_actual(RegistryLifecycleSelector::SuccessFinal, 41)
        .to_report_payload();
    assert!(validate_registry_lifecycle_report_payload(
        RegistryLifecycleSelector::SuccessFinal,
        &payload.replacen("a2b2rl1", "a2b2rl2", 1),
    )
    .is_err());

    let mut fields: Vec<_> = payload.split(',').collect();
    fields.pop();
    assert!(validate_registry_lifecycle_report_payload(
        RegistryLifecycleSelector::SuccessFinal,
        &fields.join(","),
    )
    .is_err());

    let payload = independently_observed_actual(RegistryLifecycleSelector::SuccessFinal, 43)
        .to_report_payload();
    let mut fields: Vec<_> = payload.split(',').collect();
    fields[2] = "01";
    assert!(validate_registry_lifecycle_report_payload(
        RegistryLifecycleSelector::SuccessFinal,
        &fields.join(","),
    )
    .is_err());
}

#[test]
fn registry_lifecycle_validator_rejects_selector_and_field_drift() {
    let selector = RegistryLifecycleSelector::SuccessFinal;
    let actual = independently_observed_actual(selector, 47);
    let payload = actual.to_report_payload();
    assert!(validate_registry_lifecycle_report_payload(
        RegistryLifecycleSelector::SuccessSharedNonFinal,
        &payload,
    )
    .is_err());

    for index in [2usize, 14, 22, 47, 80] {
        let mut fields: Vec<_> = payload.split(',').collect();
        fields[index] = if fields[index] == "0" { "1" } else { "0" };
        assert!(validate_registry_lifecycle_report_payload(selector, &fields.join(",")).is_err());
    }
}

fn independently_observed_actual(
    selector: RegistryLifecycleSelector,
    registration_id: u64,
) -> RegistryLifecycleActual {
    use RegistryLifecycleSelector as S;
    let shared = selector == S::SuccessSharedNonFinal;
    let mut actual = success_actual(selector, registration_id, shared);
    match selector {
        S::CallbackCompletionBefore => {
            callback_failure(&mut actual, RegistryLifecycleTiming::BeforeCall, 0, 0);
            injected(&mut actual);
        }
        S::CallbackCompletionNativeUncertain => {
            callback_failure(&mut actual, RegistryLifecycleTiming::NativeUncertain, 1, 0);
            actual.counts.fault_observe = 1;
        }
        S::CallbackCompletionAfterSuccessKnown => {
            callback_failure(
                &mut actual,
                RegistryLifecycleTiming::AfterSuccessKnown,
                1,
                1,
            );
            injected(&mut actual);
        }
        S::ConnectionObservationBefore => {
            observation_failure(&mut actual, RegistryLifecycleTiming::BeforeCall, 0, 0);
            injected(&mut actual);
        }
        S::ConnectionObservationOutstandingSidecar => {
            observation_failure(&mut actual, RegistryLifecycleTiming::Validation, 1, 0);
            actual.identity.variant = 1;
            actual.counts.fault_observe = 1;
        }
        S::ConnectionObservationAfterSuccessKnown => {
            observation_failure(
                &mut actual,
                RegistryLifecycleTiming::AfterSuccessKnown,
                1,
                1,
            );
            injected(&mut actual);
        }
        S::RegistryRouteRemovalBefore => {
            route_failure(&mut actual, RegistryLifecycleTiming::BeforeCall, 0, 0, 0);
            injected(&mut actual);
        }
        S::RegistryRouteRemovalOwnerNative => {
            route_failure(
                &mut actual,
                RegistryLifecycleTiming::NativeUncertain,
                1,
                1,
                0,
            );
            actual.counts.fault_observe = 1;
        }
        S::RegistryRouteRemovalPublishNative => {
            route_failure(
                &mut actual,
                RegistryLifecycleTiming::NativeUncertain,
                2,
                1,
                1,
            );
        }
        S::RegistryRouteRemovalAfterSuccessKnown => {
            route_failure(
                &mut actual,
                RegistryLifecycleTiming::AfterSuccessKnown,
                0,
                1,
                1,
            );
            injected(&mut actual);
        }
        S::LogicalRouteRemovalBefore => {
            logical_failure(&mut actual, RegistryLifecycleTiming::BeforeCall, 0, 0, 0, 0);
            injected(&mut actual);
        }
        S::LogicalRouteRemovalClaimNative => {
            logical_failure(
                &mut actual,
                RegistryLifecycleTiming::NativeUncertain,
                1,
                0,
                0,
                0,
            );
        }
        S::LogicalRouteRemovalIndexNative => {
            logical_failure(
                &mut actual,
                RegistryLifecycleTiming::NativeUncertain,
                2,
                1,
                0,
                0,
            );
            actual.counts.fault_observe = 1;
        }
        S::LogicalRouteRemovalAfterSuccessKnown => {
            logical_failure(
                &mut actual,
                RegistryLifecycleTiming::AfterSuccessKnown,
                0,
                1,
                1,
                3,
            );
            actual.logical_route_phase = RegistryLifecycleLogicalRoutePhase::Removed;
            actual.post.logical_names = 0;
            actual.retained.logical_names = 0;
            injected(&mut actual);
        }
        S::SuccessSharedNonFinal | S::SuccessFinal => {}
    }
    actual
}

fn success_actual(
    selector: RegistryLifecycleSelector,
    registration_id: u64,
    shared: bool,
) -> RegistryLifecycleActual {
    let one = RegistryLifecycleActualTopology {
        sqlite_connections: 1,
        shm_connections: 1,
        registry_routes: 1,
        logical_names: 3,
    };
    let empty = RegistryLifecycleActualTopology {
        sqlite_connections: 0,
        shm_connections: 0,
        registry_routes: 0,
        logical_names: 0,
    };
    let mut pre = one;
    let mut post = empty;
    if shared {
        pre = RegistryLifecycleActualTopology {
            sqlite_connections: 2,
            shm_connections: 2,
            registry_routes: 2,
            logical_names: 6,
        };
        post = one;
    }
    RegistryLifecycleActual {
        selector,
        identity: RegistryLifecycleActualIdentity {
            path_is_registry_lifecycle: true,
            topology_is_shared_non_final: shared,
            unmap_is_keep: true,
            node_is_live: true,
            variant: 0,
            pre_shared_mask: 0,
            pre_exclusive_mask: 0,
            phase: RegistryLifecyclePhase::Success,
            cause_phase_is_none: true,
            timing: RegistryLifecycleTiming::Success,
            class: RegistryLifecycleFailureClass::None,
            target: RegistryLifecycleActualTarget {
                scope_is_route_main: true,
                registration_id,
                route_ordinal: 1,
                runtime_generation: 1,
                shm_connection_id: 1,
                role_is_main: true,
                callback_is_close: true,
                occurrence: 1,
            },
            sqlite_outcome: RegistryLifecycleSqliteOutcome::Ok,
        },
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: RegistryLifecycleRegistryRoutePhase::Removed,
        logical_route_phase: RegistryLifecycleLogicalRoutePhase::Removed,
        registration_phase: RegistryLifecycleRegistrationPhase::Registered,
        later_callback_allowed: false,
        pre,
        post,
        retained: RegistryLifecycleActualCustody {
            node: false,
            views: 0,
            mappings: 0,
            dms: RegistryLifecycleDmsCustody::Absent,
            shm_file: false,
            main_file: false,
            main_lock_owner: false,
            main_lease: false,
            shm_lease: false,
            callback_leases: 0,
            registry_entry: false,
            logical_names: 0,
            vfs_table: true,
            vfs_name: true,
            vfs_context: true,
            root_deletable: false,
        },
        counts: RegistryLifecycleActualCounts {
            raw_state_take_attempt: 1,
            raw_state_take_success: 1,
            methods_clear: 1,
            callback_begin: 1,
            callback_complete_attempt: 1,
            callback_complete_success: 1,
            shm_detach: 1,
            main_unlock_attempt: 1,
            main_unlock_success: 1,
            main_file_close_attempt: 1,
            main_file_close_success: 1,
            registry_close_attempt: 1,
            registry_close_success: 1,
            connection_observe_attempt: 1,
            connection_observe_success: 1,
            registry_route_remove_attempt: 1,
            registry_route_remove_success: 1,
            logical_names_remove_attempt: 1,
            logical_names_remove_success: 1,
            logical_names_remove: 3,
            ..RegistryLifecycleActualCounts::default()
        },
    }
}

fn failure_base(actual: &mut RegistryLifecycleActual) {
    actual.identity.topology_is_shared_non_final = false;
    actual.identity.class = RegistryLifecycleFailureClass::RegistryRejected;
    actual.identity.sqlite_outcome = RegistryLifecycleSqliteOutcome::IoerrClose;
    actual.mutation_may_have_occurred = true;
    actual.registry_route_phase = RegistryLifecycleRegistryRoutePhase::TerminalQuarantine;
    actual.logical_route_phase = RegistryLifecycleLogicalRoutePhase::Retained;
    actual.pre = RegistryLifecycleActualTopology {
        sqlite_connections: 1,
        shm_connections: 1,
        registry_routes: 1,
        logical_names: 3,
    };
    actual.post = RegistryLifecycleActualTopology {
        sqlite_connections: 0,
        shm_connections: 0,
        registry_routes: 1,
        logical_names: 3,
    };
    actual.retained.registry_entry = true;
    actual.retained.logical_names = 3;
    actual.counts.connection_observe_attempt = 0;
    actual.counts.connection_observe_success = 0;
    actual.counts.registry_route_remove_attempt = 0;
    actual.counts.registry_route_remove_success = 0;
    actual.counts.logical_names_remove_attempt = 0;
    actual.counts.logical_names_remove_success = 0;
    actual.counts.logical_names_remove = 0;
    actual.counts.custody_retain = 1;
}

fn callback_failure(
    actual: &mut RegistryLifecycleActual,
    timing: RegistryLifecycleTiming,
    attempt: u8,
    success: u8,
) {
    failure_base(actual);
    actual.identity.phase = RegistryLifecyclePhase::CallbackCompletion;
    actual.identity.timing = timing;
    actual.counts.callback_complete_attempt = attempt;
    actual.counts.callback_complete_success = success;
    actual.retained.callback_leases = 1 - success;
}

fn observation_failure(
    actual: &mut RegistryLifecycleActual,
    timing: RegistryLifecycleTiming,
    attempt: u8,
    success: u8,
) {
    failure_base(actual);
    actual.identity.phase = RegistryLifecyclePhase::ConnectionObservation;
    actual.identity.timing = timing;
    actual.counts.connection_observe_attempt = attempt;
    actual.counts.connection_observe_success = success;
}

fn route_failure(
    actual: &mut RegistryLifecycleActual,
    timing: RegistryLifecycleTiming,
    variant: u8,
    attempt: u8,
    success: u8,
) {
    observation_failure(actual, timing, 1, 1);
    actual.identity.phase = RegistryLifecyclePhase::RegistryRouteRemoval;
    actual.identity.variant = variant;
    actual.counts.registry_route_remove_attempt = attempt;
    actual.counts.registry_route_remove_success = success;
    if success == 1 {
        actual.registry_route_phase = RegistryLifecycleRegistryRoutePhase::Removed;
        actual.post.registry_routes = 0;
        actual.retained.registry_entry = false;
    }
}

fn logical_failure(
    actual: &mut RegistryLifecycleActual,
    timing: RegistryLifecycleTiming,
    variant: u8,
    attempt: u8,
    success: u8,
    removed: u8,
) {
    route_failure(actual, timing, variant, 1, 1);
    actual.identity.phase = RegistryLifecyclePhase::LogicalRouteRemoval;
    actual.identity.sqlite_outcome = RegistryLifecycleSqliteOutcome::NotApplicable;
    actual.post.sqlite_connections = 0;
    actual.counts.logical_names_remove_attempt = attempt;
    actual.counts.logical_names_remove_success = success;
    actual.counts.logical_names_remove = removed;
}

fn injected(actual: &mut RegistryLifecycleActual) {
    actual.counts.fault_observe = 1;
    actual.counts.fault_trigger = 1;
}
