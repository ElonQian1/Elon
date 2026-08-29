use std::collections::BTreeSet;

use super::super::{case_key::CaseKey, model::*};
use super::{
    actual::*,
    validate::{
        frozen_unmap_cases, select_frozen_case, validate_frozen_unmap_exact_set,
        validate_unmap_report_payload,
    },
};

#[test]
fn unmap_forty_nine_selectors_exactly_cover_the_frozen_family() {
    let cases = frozen_unmap_cases();
    validate_frozen_unmap_exact_set(&cases).expect("Unmap exact set");
    let mut keys = BTreeSet::new();
    let mut names = BTreeSet::new();
    for selector in UnmapSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique Unmap Case");
        assert_eq!(case.evidence, EvidenceKind::StaticContract);
        assert!(keys.insert(CaseKey::from(case)));
        assert!(names.insert(selector.report_name()));
    }
    assert_eq!(keys.len(), 49);
    assert_eq!(names.len(), 49);
}

#[test]
fn unmap_exact_set_rejects_case_key_drift() {
    let mut cases = frozen_unmap_cases();
    cases[0].target.route_ordinal = 2;
    assert!(validate_frozen_unmap_exact_set(&cases).is_err());
    let mut cases = frozen_unmap_cases();
    cases.pop();
    assert!(validate_frozen_unmap_exact_set(&cases).is_err());
}

#[test]
fn unmap_codec_and_validator_cover_all_forty_nine_cases() {
    let cases = frozen_unmap_cases();
    for selector in UnmapSelector::ALL {
        let case = select_frozen_case(&cases, selector).expect("unique Unmap Case");
        let actual = actual_from_case(selector, case, 37);
        let payload = actual.to_report_payload();
        assert_eq!(payload.split(',').count(), 83);
        let observation = validate_unmap_report_payload(selector, &payload)
            .expect("static-shaped independently encoded Unmap actual");
        assert_eq!(observation.selector(), selector);
        assert_eq!(observation.registration_id(), 37);
        let (_key, sealed) = observation.into_evidence_parts();
        assert!(sealed.matches_exact(&payload));
        assert_eq!(sealed.exact_payload(), payload);
    }
}

#[test]
fn unmap_codec_rejects_version_quantity_canonical_and_unknown_selector() {
    let selector = UnmapSelector::FinalKeepSuccessLiveNode;
    let cases = frozen_unmap_cases();
    let payload = actual_from_case(
        selector,
        select_frozen_case(&cases, selector).expect("success Case"),
        41,
    )
    .to_report_payload();

    assert!(
        validate_unmap_report_payload(selector, &payload.replacen("a2b2un1", "a2b2un2", 1),)
            .is_err()
    );
    let mut fields: Vec<_> = payload.split(',').collect();
    fields.pop();
    assert!(validate_unmap_report_payload(selector, &fields.join(",")).is_err());
    let mut fields: Vec<_> = payload.split(',').collect();
    fields[2] = "00";
    assert!(validate_unmap_report_payload(selector, &fields.join(",")).is_err());
    let mut fields: Vec<_> = payload.split(',').collect();
    fields[1] = "unknown-unmap-selector";
    assert!(validate_unmap_report_payload(selector, &fields.join(",")).is_err());
}

#[test]
fn unmap_validator_rejects_parent_selector_and_every_field_mismatch() {
    let selector = UnmapSelector::FinalDeleteSiblingAfterKnown;
    let cases = frozen_unmap_cases();
    let payload = actual_from_case(
        selector,
        select_frozen_case(&cases, selector).expect("delete Case"),
        43,
    )
    .to_report_payload();
    assert!(validate_unmap_report_payload(UnmapSelector::SharedKeepSuccess, &payload).is_err());

    for index in 2..83 {
        let mut fields: Vec<_> = payload.split(',').collect();
        fields[index] = if index == 14 || fields[index] != "0" {
            "0"
        } else {
            "1"
        };
        assert!(
            validate_unmap_report_payload(selector, &fields.join(",")).is_err(),
            "field {index} drift was accepted"
        );
    }
}

fn actual_from_case(selector: UnmapSelector, case: &Case, registration_id: u64) -> UnmapActual {
    UnmapActual {
        selector,
        identity: UnmapActualIdentity {
            path: UnmapPath::Unmap,
            topology: match case.topology_kind {
                TopologyKind::SharedNonFinal => UnmapTopology::SharedNonFinal,
                TopologyKind::FinalConnection => UnmapTopology::FinalConnection,
                TopologyKind::RegistrationOnly => panic!("registration-only is not Unmap"),
            },
            mode: match case.unmap_mode {
                super::super::model::UnmapMode::Keep => super::actual::UnmapMode::Keep,
                super::super::model::UnmapMode::Delete => super::actual::UnmapMode::Delete,
                super::super::model::UnmapMode::NotApplicable => panic!("Unmap mode is required"),
            },
            node: match case.node_precondition {
                NodePrecondition::Live => UnmapNode::Live,
                NodePrecondition::Absent => UnmapNode::Absent,
                NodePrecondition::NotApplicable => panic!("Unmap node is required"),
            },
            variant: case.variant,
            pre_shared_mask: case.pre_shared_mask,
            pre_exclusive_mask: case.pre_exclusive_mask,
            phase: actual_phase(case.phase),
            cause: UnmapCause::None,
            timing: actual_timing(case.timing),
            class: actual_class(case.class),
            target: UnmapActualTarget {
                scope: UnmapTargetScope::RouteMain,
                registration_id,
                route_ordinal: case.target.route_ordinal,
                runtime_generation: case.target.runtime_generation,
                shm_connection_id: case.target.shm_connection_id,
                role: UnmapRole::Main,
                callback: UnmapCallback::Shm,
                occurrence: case.target.occurrence,
            },
            sqlite_outcome: match case.sqlite_outcome {
                SqliteOutcome::Ok => UnmapSqliteOutcome::Ok,
                SqliteOutcome::Ioerr => UnmapSqliteOutcome::Ioerr,
                _ => panic!("unsupported Unmap SQLite outcome"),
            },
        },
        mutation_may_have_occurred: case.mutation_may_have_occurred,
        lock_outcome_uncertain: case.lock_outcome_uncertain,
        domain_terminal: case.domain_terminal,
        registry_route_phase: match case.registry_route_phase {
            RegistryRoutePhase::Active => UnmapRegistryRoutePhase::Active,
            RegistryRoutePhase::TerminalQuarantine => UnmapRegistryRoutePhase::TerminalQuarantine,
            _ => panic!("unsupported Unmap registry phase"),
        },
        logical_route_phase: match case.logical_route_phase {
            LogicalRoutePhase::Indexed => UnmapLogicalRoutePhase::Indexed,
            LogicalRoutePhase::Retained => UnmapLogicalRoutePhase::Retained,
            _ => panic!("unsupported Unmap logical phase"),
        },
        registration_phase: UnmapRegistrationPhase::Registered,
        later_callback_allowed: case.later_callback_allowed,
        pre: topology(case.pre),
        post: topology(case.post),
        retained: custody(case.retained),
        counts: counts(case.counts),
    }
}

fn actual_phase(value: Phase) -> UnmapPhase {
    match value {
        Phase::RequestValidation => UnmapPhase::RequestValidation,
        Phase::CallbackAdmission => UnmapPhase::CallbackAdmission,
        Phase::HeldLockGate => UnmapPhase::HeldLockGate,
        Phase::ConnectionDetach => UnmapPhase::ConnectionDetach,
        Phase::ViewUnmap => UnmapPhase::ViewUnmap,
        Phase::MappingClose => UnmapPhase::MappingClose,
        Phase::DmsSharedRelease => UnmapPhase::DmsSharedRelease,
        Phase::ShmFileClose => UnmapPhase::ShmFileClose,
        Phase::DeleteAuthorization => UnmapPhase::DeleteAuthorization,
        Phase::ExactSiblingDelete => UnmapPhase::ExactSiblingDelete,
        Phase::CallbackCompletion => UnmapPhase::CallbackCompletion,
        Phase::Success => UnmapPhase::Success,
        _ => panic!("unsupported Unmap phase"),
    }
}

fn actual_timing(value: Timing) -> UnmapTiming {
    match value {
        Timing::Validation => UnmapTiming::Validation,
        Timing::BeforeCall => UnmapTiming::BeforeCall,
        Timing::NativeRetryable => UnmapTiming::NativeRetryable,
        Timing::NativeUncertain => UnmapTiming::NativeUncertain,
        Timing::AfterSuccessKnown => UnmapTiming::AfterSuccessKnown,
        Timing::AfterSuccessUncertain => UnmapTiming::AfterSuccessUncertain,
        Timing::Success => UnmapTiming::Success,
    }
}

fn actual_class(value: FailureClass) -> UnmapFailureClass {
    match value {
        FailureClass::None => UnmapFailureClass::None,
        FailureClass::ProtocolViolation => UnmapFailureClass::ProtocolViolation,
        FailureClass::IoBeforeMutation => UnmapFailureClass::IoBeforeMutation,
        FailureClass::MutatedButKnown => UnmapFailureClass::MutatedButKnown,
        FailureClass::OutcomeUncertainPoisoned => UnmapFailureClass::OutcomeUncertainPoisoned,
        FailureClass::RegistryRejected => UnmapFailureClass::RegistryRejected,
        _ => panic!("unsupported Unmap failure class"),
    }
}

fn topology(value: Topology) -> UnmapActualTopology {
    UnmapActualTopology {
        sqlite_connections: value.sqlite_connections,
        shm_connections: value.shm_connections,
        registry_routes: value.registry_routes,
        logical_names: value.logical_names,
    }
}

fn custody(value: Custody) -> UnmapActualCustody {
    UnmapActualCustody {
        node: value.node,
        views: value.views,
        mappings: value.mappings,
        dms: match value.dms {
            DmsCustody::Absent => UnmapDmsCustody::Absent,
            DmsCustody::Shared => UnmapDmsCustody::Shared,
            DmsCustody::Released => UnmapDmsCustody::Released,
            DmsCustody::OutcomeUncertain => UnmapDmsCustody::OutcomeUncertain,
        },
        shm_file: value.shm_file,
        main_file: value.main_file,
        main_lock_owner: value.main_lock_owner,
        main_lease: value.main_lease,
        shm_lease: value.shm_lease,
        callback_leases: value.callback_leases,
        registry_entry: value.registry_entry,
        logical_names: value.logical_names,
        vfs_table: value.vfs_table,
        vfs_name: value.vfs_name,
        vfs_context: value.vfs_context,
        root_deletable: value.root_deletable,
    }
}

fn counts(value: Counts) -> UnmapActualCounts {
    UnmapActualCounts {
        raw_state_take_attempt: value.raw_state_take_attempt,
        raw_state_take_success: value.raw_state_take_success,
        raw_state_abandon: value.raw_state_abandon,
        methods_clear: value.methods_clear,
        callback_begin: value.callback_begin,
        callback_complete_attempt: value.callback_complete_attempt,
        callback_complete_success: value.callback_complete_success,
        selected_action_attempt: value.selected_action_attempt,
        selected_action_success: value.selected_action_success,
        shm_detach: value.shm_detach,
        main_unlock_attempt: value.main_unlock_attempt,
        main_unlock_success: value.main_unlock_success,
        main_file_close_attempt: value.main_file_close_attempt,
        main_file_close_success: value.main_file_close_success,
        registry_close_attempt: value.registry_close_attempt,
        registry_close_success: value.registry_close_success,
        connection_observe_attempt: value.connection_observe_attempt,
        connection_observe_success: value.connection_observe_success,
        registry_route_remove_attempt: value.registry_route_remove_attempt,
        registry_route_remove_success: value.registry_route_remove_success,
        logical_names_remove_attempt: value.logical_names_remove_attempt,
        logical_names_remove_success: value.logical_names_remove_success,
        logical_names_remove: value.logical_names_remove,
        vfs_unregister_attempt: value.vfs_unregister_attempt,
        vfs_unregister_success: value.vfs_unregister_success,
        fault_observe: value.fault_observe,
        fault_trigger: value.fault_trigger,
        fault_pending: value.fault_pending,
        custody_retain: value.custody_retain,
        physical_retry: value.physical_retry,
    }
}
