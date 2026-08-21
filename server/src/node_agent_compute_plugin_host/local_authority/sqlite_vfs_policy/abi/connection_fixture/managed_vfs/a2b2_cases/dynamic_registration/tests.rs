use std::collections::BTreeSet;

use super::super::{
    case_key::CaseKey,
    invariants,
    model::{EvidenceKind, RegistryRoutePhase},
};
use super::{
    actual::{
        RegistrationShutdownActual, RegistrationShutdownActualCounts,
        RegistrationShutdownActualCustody, RegistrationShutdownActualIdentity,
        RegistrationShutdownActualTarget, RegistrationShutdownActualTopology,
        RegistrationShutdownDmsCustody, RegistrationShutdownFailureClass,
        RegistrationShutdownLogicalRoutePhase, RegistrationShutdownPhase,
        RegistrationShutdownRegistrationPhase, RegistrationShutdownRegistryRoutePhase,
        RegistrationShutdownSelector, RegistrationShutdownTiming,
    },
    validate::{select_frozen_case, validate_registration_shutdown_report_payload},
};

#[test]
fn eight_selectors_are_a_bijection_over_static_registration_cases() {
    let cases = invariants::inventory();
    invariants::validate(&cases).expect("frozen A2b2 inventory");
    let mut keys = BTreeSet::new();
    for selector in RegistrationShutdownSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique selector");
        assert_eq!(case.evidence, EvidenceKind::StaticContract);
        assert!(keys.insert(CaseKey::from(case)));
    }
    assert_eq!(keys.len(), 8);
}

#[test]
fn canonical_success_payload_is_parsed_and_cross_bound() {
    let actual = success_actual();
    let payload = actual.to_report_payload();
    let observation = validate_registration_shutdown_report_payload(
        RegistrationShutdownSelector::Success,
        &payload,
    )
    .expect("complete success observation");
    assert_eq!(
        observation.selector(),
        RegistrationShutdownSelector::Success
    );
    let (_key, validated_payload) = observation.into_evidence_parts();
    assert!(validated_payload.matches_exact(&payload));
    assert!(!validated_payload.matches_exact("a2b2rs1,success"));
}

#[test]
fn eight_independently_authored_actuals_validate_against_their_fixed_selectors() {
    let actuals = [
        gate_actual(
            RegistrationShutdownSelector::OutstandingCallbackGate,
            RegistrationShutdownPhase::OutstandingCallbackGate,
            1,
            1,
            false,
        ),
        gate_actual(
            RegistrationShutdownSelector::LiveRouteGate,
            RegistrationShutdownPhase::LiveRouteGate,
            2,
            0,
            false,
        ),
        gate_actual(
            RegistrationShutdownSelector::QuarantinedCustodyGate,
            RegistrationShutdownPhase::QuarantinedCustodyGate,
            3,
            0,
            false,
        ),
        gate_actual(
            RegistrationShutdownSelector::RouteIndexObservation,
            RegistrationShutdownPhase::RouteIndexObservation,
            4,
            0,
            true,
        ),
        unregister_actual(
            RegistrationShutdownSelector::VfsUnregisterBeforeCall,
            RegistrationShutdownTiming::BeforeCall,
            0,
            0,
            true,
            true,
        ),
        unregister_actual(
            RegistrationShutdownSelector::VfsUnregisterNativeRetryable,
            RegistrationShutdownTiming::NativeRetryable,
            1,
            0,
            true,
            false,
        ),
        unregister_actual(
            RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown,
            RegistrationShutdownTiming::AfterSuccessKnown,
            1,
            1,
            true,
            true,
        ),
        success_actual(),
    ];
    for actual in actuals {
        let selector = actual.selector;
        let payload = actual.to_report_payload();
        validate_registration_shutdown_report_payload(selector, &payload)
            .expect("independently authored RegistrationShutdown actual");
    }
}

#[test]
fn fixed_selector_and_every_reported_count_remain_strict() {
    let payload = success_actual().to_report_payload();
    assert!(validate_registration_shutdown_report_payload(
        RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown,
        &payload,
    )
    .is_err());

    let mut wrong = success_actual();
    wrong.counts.vfs_unregister_success = 0;
    assert!(validate_registration_shutdown_report_payload(
        RegistrationShutdownSelector::Success,
        &wrong.to_report_payload(),
    )
    .is_err());
}

#[test]
fn noncanonical_payload_cannot_reach_the_validator() {
    let mut payload = success_actual().to_report_payload();
    payload.push_str(",0");
    assert!(validate_registration_shutdown_report_payload(
        RegistrationShutdownSelector::Success,
        &payload,
    )
    .is_err());
}

#[test]
fn quarantined_custody_gate_is_registration_level_not_route_quarantine() {
    let cases = invariants::inventory();
    let case = select_frozen_case(&cases, RegistrationShutdownSelector::QuarantinedCustodyGate)
        .expect("registration-level custody gate");
    assert_eq!(case.registry_route_phase, RegistryRoutePhase::Active);
    assert!(!case.domain_terminal);
    assert!(case.later_callback_allowed);
}

fn success_actual() -> RegistrationShutdownActual {
    RegistrationShutdownActual {
        selector: RegistrationShutdownSelector::Success,
        identity: RegistrationShutdownActualIdentity {
            path_is_registration_shutdown: true,
            topology_is_registration_only: true,
            unmap_is_not_applicable: true,
            node_is_not_applicable: true,
            variant: 0,
            pre_shared_mask: 0,
            pre_exclusive_mask: 0,
            phase: RegistrationShutdownPhase::Success,
            cause_phase_is_none: true,
            timing: RegistrationShutdownTiming::Success,
            class: RegistrationShutdownFailureClass::None,
            target: RegistrationShutdownActualTarget {
                scope_is_registration: true,
                registration_id: 1,
                route_ordinal_is_not_applicable: true,
                runtime_generation_is_not_applicable: true,
                shm_connection_id_is_not_applicable: true,
                role_is_none: true,
                callback_is_none: true,
                occurrence: 1,
            },
            sqlite_outcome_is_not_applicable: true,
        },
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: RegistrationShutdownRegistryRoutePhase::Removed,
        logical_route_phase: RegistrationShutdownLogicalRoutePhase::Removed,
        registration_phase: RegistrationShutdownRegistrationPhase::Unregistered,
        later_callback_allowed: false,
        pre: RegistrationShutdownActualTopology {
            sqlite_connections: 0,
            shm_connections: 0,
            registry_routes: 0,
            logical_names: 0,
        },
        post: RegistrationShutdownActualTopology {
            sqlite_connections: 0,
            shm_connections: 0,
            registry_routes: 0,
            logical_names: 0,
        },
        retained: RegistrationShutdownActualCustody {
            node: false,
            views: 0,
            mappings: 0,
            dms: RegistrationShutdownDmsCustody::Absent,
            shm_file: false,
            main_file: false,
            main_lock_owner: false,
            main_lease: false,
            shm_lease: false,
            callback_leases: 0,
            registry_entry: false,
            logical_names: 0,
            vfs_table: false,
            vfs_name: false,
            vfs_context: false,
            root_deletable: true,
        },
        counts: RegistrationShutdownActualCounts {
            vfs_unregister_attempt: 1,
            vfs_unregister_success: 1,
            ..zero_counts()
        },
    }
}

fn gate_actual(
    selector: RegistrationShutdownSelector,
    phase: RegistrationShutdownPhase,
    variant: u8,
    callback_leases: u8,
    route_observed: bool,
) -> RegistrationShutdownActual {
    RegistrationShutdownActual {
        selector,
        identity: registration_identity(
            phase,
            if route_observed {
                RegistrationShutdownTiming::NativeUncertain
            } else {
                RegistrationShutdownTiming::Validation
            },
            RegistrationShutdownFailureClass::RegistrationRetained,
            variant,
        ),
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: RegistrationShutdownRegistryRoutePhase::Active,
        logical_route_phase: RegistrationShutdownLogicalRoutePhase::Retained,
        registration_phase: RegistrationShutdownRegistrationPhase::RetainedRegistered,
        later_callback_allowed: true,
        pre: one_topology(),
        post: one_topology(),
        retained: RegistrationShutdownActualCustody {
            node: true,
            views: 1,
            mappings: 1,
            dms: RegistrationShutdownDmsCustody::Shared,
            shm_file: true,
            main_file: true,
            main_lock_owner: true,
            main_lease: true,
            shm_lease: true,
            callback_leases,
            registry_entry: true,
            logical_names: 3,
            vfs_table: true,
            vfs_name: true,
            vfs_context: true,
            root_deletable: false,
        },
        counts: RegistrationShutdownActualCounts {
            fault_observe: u8::from(route_observed),
            custody_retain: 1,
            ..zero_counts()
        },
    }
}

fn unregister_actual(
    selector: RegistrationShutdownSelector,
    timing: RegistrationShutdownTiming,
    unregister_attempt: u8,
    unregister_success: u8,
    fault_observe: bool,
    fault_trigger: bool,
) -> RegistrationShutdownActual {
    let after_success = timing == RegistrationShutdownTiming::AfterSuccessKnown;
    RegistrationShutdownActual {
        selector,
        identity: registration_identity(
            RegistrationShutdownPhase::VfsUnregister,
            timing,
            RegistrationShutdownFailureClass::RegistrationRetained,
            0,
        ),
        mutation_may_have_occurred: after_success,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: RegistrationShutdownRegistryRoutePhase::Removed,
        logical_route_phase: RegistrationShutdownLogicalRoutePhase::Removed,
        registration_phase: if after_success {
            RegistrationShutdownRegistrationPhase::RetainedAfterUnregister
        } else {
            RegistrationShutdownRegistrationPhase::RetainedRegistered
        },
        later_callback_allowed: false,
        pre: empty_topology(),
        post: empty_topology(),
        retained: retained_vfs_custody(),
        counts: RegistrationShutdownActualCounts {
            vfs_unregister_attempt: unregister_attempt,
            vfs_unregister_success: unregister_success,
            fault_observe: u8::from(fault_observe),
            fault_trigger: u8::from(fault_trigger),
            custody_retain: 1,
            ..zero_counts()
        },
    }
}

fn registration_identity(
    phase: RegistrationShutdownPhase,
    timing: RegistrationShutdownTiming,
    class: RegistrationShutdownFailureClass,
    variant: u8,
) -> RegistrationShutdownActualIdentity {
    RegistrationShutdownActualIdentity {
        path_is_registration_shutdown: true,
        topology_is_registration_only: true,
        unmap_is_not_applicable: true,
        node_is_not_applicable: true,
        variant,
        pre_shared_mask: 0,
        pre_exclusive_mask: 0,
        phase,
        cause_phase_is_none: true,
        timing,
        class,
        target: RegistrationShutdownActualTarget {
            scope_is_registration: true,
            registration_id: 1,
            route_ordinal_is_not_applicable: true,
            runtime_generation_is_not_applicable: true,
            shm_connection_id_is_not_applicable: true,
            role_is_none: true,
            callback_is_none: true,
            occurrence: 1,
        },
        sqlite_outcome_is_not_applicable: true,
    }
}

fn one_topology() -> RegistrationShutdownActualTopology {
    RegistrationShutdownActualTopology {
        sqlite_connections: 1,
        shm_connections: 1,
        registry_routes: 1,
        logical_names: 3,
    }
}

fn empty_topology() -> RegistrationShutdownActualTopology {
    RegistrationShutdownActualTopology {
        sqlite_connections: 0,
        shm_connections: 0,
        registry_routes: 0,
        logical_names: 0,
    }
}

fn retained_vfs_custody() -> RegistrationShutdownActualCustody {
    RegistrationShutdownActualCustody {
        node: false,
        views: 0,
        mappings: 0,
        dms: RegistrationShutdownDmsCustody::Absent,
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
    }
}

fn zero_counts() -> RegistrationShutdownActualCounts {
    RegistrationShutdownActualCounts {
        raw_state_take_attempt: 0,
        raw_state_take_success: 0,
        raw_state_abandon: 0,
        methods_clear: 0,
        callback_begin: 0,
        callback_complete_attempt: 0,
        callback_complete_success: 0,
        selected_action_attempt: 0,
        selected_action_success: 0,
        shm_detach: 0,
        main_unlock_attempt: 0,
        main_unlock_success: 0,
        main_file_close_attempt: 0,
        main_file_close_success: 0,
        registry_close_attempt: 0,
        registry_close_success: 0,
        connection_observe_attempt: 0,
        connection_observe_success: 0,
        registry_route_remove_attempt: 0,
        registry_route_remove_success: 0,
        logical_names_remove_attempt: 0,
        logical_names_remove_success: 0,
        logical_names_remove: 0,
        vfs_unregister_attempt: 0,
        vfs_unregister_success: 0,
        fault_observe: 0,
        fault_trigger: 0,
        fault_pending: 0,
        custody_retain: 0,
        physical_retry: 0,
    }
}
