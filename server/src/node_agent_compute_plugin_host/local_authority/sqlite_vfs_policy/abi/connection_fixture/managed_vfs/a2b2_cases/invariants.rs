use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::{
    barrier, close_physical, close_registry, expected,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, LogicalRoutePhase,
        NodePrecondition, Path, Phase, RegistrationPhase, RegistryRoutePhase, SqliteOutcome,
        TargetScope, Timing, TopologyKind, UnmapMode,
    },
    registration, unmap_delete, unmap_nonfinal, unmap_teardown,
};

pub(super) fn inventory() -> Vec<Case> {
    let mut cases = Vec::new();
    cases.extend(barrier::cases());
    cases.extend(unmap_nonfinal::cases());
    cases.extend(unmap_teardown::cases());
    cases.extend(unmap_delete::cases());
    cases.extend(close_physical::cases());
    cases.extend(close_registry::cases());
    cases.extend(registration::cases());
    cases
}

pub(super) fn validate(cases: &[Case]) -> Result<(), &'static str> {
    if cases.len() != 117
        || count(cases, Path::Barrier) != 8
        || count(cases, Path::Unmap) != 49
        || count(cases, Path::JointClose) != 36
        || count(cases, Path::RegistryLifecycle) != 16
        || count(cases, Path::RegistrationShutdown) != 8
    {
        return Err("A2b2 exact path inventory count changed");
    }
    for case in cases {
        validate_case(case)?;
    }
    expected::validate(cases)
}

fn count(cases: &[Case], path: Path) -> usize {
    cases.iter().filter(|case| case.path == path).count()
}

fn validate_case(case: &Case) -> Result<(), &'static str> {
    let registration_target = case.path == Path::RegistrationShutdown
        && case.target.scope == TargetScope::Registration
        && case.target.route_ordinal == 0
        && case.target.runtime_generation == 0
        && case.target.shm_connection_id == 0
        && case.target.role.is_none()
        && case.target.callback.is_none();
    let exact_route_callback = matches!(
        (case.path, case.target.callback),
        (Path::Barrier | Path::Unmap, Some(CallbackKind::Shm))
            | (
                Path::JointClose | Path::RegistryLifecycle,
                Some(CallbackKind::Close)
            )
    );
    let route_target = case.path != Path::RegistrationShutdown
        && case.target.scope == TargetScope::RouteMain
        && case.target.route_ordinal != 0
        && case.target.runtime_generation != 0
        && case.target.shm_connection_id != 0
        && case.target.role == Some(ManagedSqliteLogicalFileRole::Main)
        && exact_route_callback;
    if case.target.registration_id == 0
        || !(registration_target || route_target)
        || case.target.occurrence == 0
        || case.evidence != EvidenceKind::StaticContract
        || case.pre.logical_names % 3 != 0
        || case.post.logical_names % 3 != 0
        || case.retained.views > case.retained.mappings
        || case.counts.selected_action_success > case.counts.selected_action_attempt
        || case.counts.raw_state_take_success > case.counts.raw_state_take_attempt
        || case.counts.callback_complete_success > case.counts.callback_complete_attempt
        || case.counts.callback_complete_attempt > case.counts.callback_begin
        || case.counts.main_unlock_success > case.counts.main_unlock_attempt
        || case.counts.main_file_close_success > case.counts.main_file_close_attempt
        || case.counts.registry_close_success > case.counts.registry_close_attempt
        || case.counts.connection_observe_success > case.counts.connection_observe_attempt
        || case.counts.registry_route_remove_success > case.counts.registry_route_remove_attempt
        || case.counts.logical_names_remove_success > case.counts.logical_names_remove_attempt
        || case.counts.vfs_unregister_success > case.counts.vfs_unregister_attempt
        || case.counts.physical_retry != 0
        || (case.path == Path::RegistrationShutdown
            && case.phase == Phase::VfsUnregister
            && case.mutation_may_have_occurred != (case.timing == Timing::AfterSuccessKnown))
    {
        return Err("A2b2 identity, mutation, custody or exact-count conservation failed");
    }
    validate_sqlite_channel(case)?;
    validate_one_shot(case)?;
    validate_terminal_state(case)?;
    validate_reachable_boundary(case)?;
    unmap_teardown::validate_dms_receipt(case)?;
    if case.path != Path::RegistrationShutdown
        && case.retained.callback_leases
            != case
                .counts
                .callback_begin
                .saturating_sub(case.counts.callback_complete_success)
    {
        return Err("callback lease custody does not match begin/completion counts");
    }
    if case.counts.raw_state_abandon != 0
        && (case.path != Path::Barrier || case.counts.methods_clear != 1)
    {
        return Err("only the void barrier failure path abandons installed raw state");
    }
    if case.counts.raw_state_take_success != 0
        && (case.path != Path::JointClose || case.counts.methods_clear != 1)
    {
        return Err("only xClose linearly takes and clears raw state");
    }
    if case.counts.logical_names_remove != 0 && case.counts.logical_names_remove != 3 {
        return Err("one exact route owns exactly three logical names");
    }
    validate_root_release(case)
}

fn validate_sqlite_channel(case: &Case) -> Result<(), &'static str> {
    if (case.path == Path::Barrier) != (case.sqlite_outcome == SqliteOutcome::VoidNoResultCode) {
        return Err("xShmBarrier must not invent a SQLite result-code channel");
    }
    let expected = match (case.path, case.phase, case.class) {
        (Path::Barrier, _, _) => SqliteOutcome::VoidNoResultCode,
        (Path::RegistrationShutdown, _, _) | (_, Phase::LogicalRouteRemoval, _) => {
            SqliteOutcome::NotApplicable
        }
        (_, _, FailureClass::None) => SqliteOutcome::Ok,
        (Path::JointClose | Path::RegistryLifecycle, _, _) => SqliteOutcome::IoerrClose,
        (Path::Unmap, _, _) => SqliteOutcome::Ioerr,
    };
    if case.sqlite_outcome != expected {
        return Err("A2b2 record uses the wrong SQLite result channel");
    }
    Ok(())
}

fn validate_one_shot(case: &Case) -> Result<(), &'static str> {
    let not_found = case.path == Path::Unmap
        && case.unmap_mode == UnmapMode::Delete
        && case.phase == Phase::Success
        && case.variant == 1;
    let native_observation = (case.path == Path::JointClose
        && case.phase == Phase::RegistryWalMainClose
        && case.timing == Timing::NativeUncertain)
        || matches!(
            (case.path, case.phase),
            (Path::RegistrationShutdown, Phase::RouteIndexObservation)
        )
        || (matches!(
            (case.path, case.phase),
            (Path::Barrier, Phase::CallbackCompletion)
                | (Path::RegistryLifecycle, Phase::CallbackCompletion)
        ) && case.timing == Timing::NativeUncertain)
        || (case.path == Path::RegistryLifecycle
            && case.phase == Phase::RegistryRouteRemoval
            && case.timing == Timing::NativeUncertain
            && case.variant == 1)
        || (case.path == Path::RegistryLifecycle
            && case.phase == Phase::LogicalRouteRemoval
            && case.timing == Timing::NativeUncertain
            && case.variant == 2)
        || (case.path == Path::JointClose
            && matches!(case.phase, Phase::MainLockRelease | Phase::MainFileClose)
            && matches!(
                case.timing,
                Timing::NativeRetryable | Timing::NativeUncertain
            ))
        || (case.path == Path::RegistryLifecycle
            && case.phase == Phase::ConnectionObservation
            && case.variant == 1)
        || (case.path == Path::RegistrationShutdown
            && case.phase == Phase::VfsUnregister
            && case.timing == Timing::NativeRetryable);
    let no_selector = matches!(
        (case.path, case.phase),
        (
            Path::Barrier | Path::Unmap | Path::JointClose,
            Phase::CallbackAdmission
        ) | (Path::JointClose, Phase::BeginConnectionClose)
    );
    let injected = !no_selector
        && matches!(
            case.timing,
            Timing::BeforeCall | Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
        );
    let expected = if not_found {
        (1, 0, 1)
    } else if native_observation {
        (1, 0, 0)
    } else if injected {
        (1, 1, 0)
    } else {
        (0, 0, 0)
    };
    if (
        case.counts.fault_observe,
        case.counts.fault_trigger,
        case.counts.fault_pending,
    ) != expected
    {
        return Err("one-shot observe/trigger/pending counts differ from the exact seam");
    }
    Ok(())
}

fn validate_terminal_state(case: &Case) -> Result<(), &'static str> {
    let expected_domain = match (case.path, case.phase) {
        (Path::Barrier, Phase::BarrierFence) => case.variant == 0,
        (Path::Unmap, Phase::DeleteAuthorization) => case.lock_outcome_uncertain,
        (Path::Unmap, Phase::ViewUnmap) => case.timing != Timing::BeforeCall,
        (
            Path::Unmap,
            Phase::MappingClose
            | Phase::DmsSharedRelease
            | Phase::ShmFileClose
            | Phase::ExactSiblingDelete,
        ) => true,
        (Path::Unmap, Phase::ConnectionDetach) => {
            case.topology_kind == TopologyKind::FinalConnection
                || case.unmap_mode == UnmapMode::Delete
                || matches!(
                    case.timing,
                    Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
                )
        }
        (Path::JointClose, Phase::ShmUnmapLift) => {
            !(case.cause_phase == Some(Phase::ViewUnmap) && case.timing == Timing::BeforeCall)
        }
        _ => false,
    };
    let expected_route = match (case.path, case.phase) {
        (Path::Barrier, _) => {
            if case.class == FailureClass::None {
                RegistryRoutePhase::Active
            } else {
                RegistryRoutePhase::TerminalQuarantine
            }
        }
        (Path::Unmap, Phase::CallbackAdmission | Phase::CallbackCompletion) => {
            RegistryRoutePhase::TerminalQuarantine
        }
        (Path::Unmap, _) => {
            if expected_domain {
                RegistryRoutePhase::TerminalQuarantine
            } else {
                RegistryRoutePhase::Active
            }
        }
        (Path::JointClose, Phase::RawStateTake) => RegistryRoutePhase::Active,
        (Path::JointClose, Phase::Success) => RegistryRoutePhase::Closing,
        (Path::JointClose, _) => RegistryRoutePhase::TerminalQuarantine,
        (Path::RegistryLifecycle, Phase::Success | Phase::LogicalRouteRemoval) => {
            RegistryRoutePhase::Removed
        }
        (Path::RegistryLifecycle, Phase::RegistryRouteRemoval)
            if case.timing == Timing::AfterSuccessKnown || case.variant == 2 =>
        {
            RegistryRoutePhase::Removed
        }
        (Path::RegistryLifecycle, _) => RegistryRoutePhase::TerminalQuarantine,
        (Path::RegistrationShutdown, Phase::VfsUnregister | Phase::Success) => {
            RegistryRoutePhase::Removed
        }
        (Path::RegistrationShutdown, _) => RegistryRoutePhase::Active,
    };
    let expected_later_callback = matches!(
        expected_route,
        RegistryRoutePhase::Active | RegistryRoutePhase::Closing
    );
    if case.domain_terminal != expected_domain
        || case.registry_route_phase != expected_route
        || case.later_callback_allowed != expected_later_callback
        || (case.lock_outcome_uncertain && case.class != FailureClass::OutcomeUncertainPoisoned)
    {
        return Err("physical domain, exact route and callback authority disagree");
    }
    Ok(())
}

fn validate_reachable_boundary(case: &Case) -> Result<(), &'static str> {
    let shared_detached = case.path == Path::Unmap
        && case.topology_kind == TopologyKind::SharedNonFinal
        && (case.phase == Phase::Success
            || case.phase == Phase::CallbackCompletion
            || (case.phase == Phase::ConnectionDetach
                && matches!(
                    case.timing,
                    Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
                )));
    if shared_detached
        && (case.post.shm_connections != 1
            || !case.retained.node
            || case.retained.views != 1
            || case.retained.mappings != 1
            || case.retained.dms != DmsCustody::Shared
            || !case.retained.shm_file)
    {
        return Err("non-final unmap failed to preserve sibling shared custody");
    }
    let seam = if case.path == Path::Unmap {
        Some(case.phase)
    } else if case.path == Path::JointClose && case.phase == Phase::ShmUnmapLift {
        case.cause_phase
    } else {
        None
    };
    match seam {
        Some(Phase::ViewUnmap)
            if case.timing == Timing::BeforeCall
                && (!case.retained.node
                    || case.retained.views != 1
                    || case.retained.mappings != 1
                    || case.retained.dms != DmsCustody::Shared
                    || !case.retained.shm_file) =>
        {
            return Err("view-unmap before-call must retain the live node custody");
        }
        Some(Phase::MappingClose) if case.retained.views != 0 => {
            return Err("mapping-close seam still retains a prior view");
        }
        Some(Phase::DmsSharedRelease)
            if case.retained.views != 0 || case.retained.mappings != 0 =>
        {
            return Err("DMS-release seam still retains a prior mapping");
        }
        Some(Phase::ShmFileClose) => validate_file_close_custody(case)?,
        _ => {}
    }
    let direct_final_post_teardown = case.path == Path::Unmap
        && case.topology_kind == TopologyKind::FinalConnection
        && matches!(
            case.phase,
            Phase::ConnectionDetach | Phase::CallbackCompletion
        );
    if direct_final_post_teardown
        && (case.retained.node
            || case.retained.views != 0
            || case.retained.mappings != 0
            || case.retained.dms != DmsCustody::Absent
            || case.retained.shm_file)
    {
        return Err("final detach/completion retained already-consumed SHM custody");
    }
    validate_close_and_registry_counts(case)
}

fn validate_file_close_custody(case: &Case) -> Result<(), &'static str> {
    let selected_success = matches!(
        case.timing,
        Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    );
    let before = case.timing == Timing::BeforeCall;
    if case.retained.views != 0
        || case.retained.mappings != 0
        || (before
            && (!case.retained.node
                || case.retained.dms != DmsCustody::Released
                || !case.retained.shm_file))
        || (!before
            && (case.retained.node
                || case.retained.dms != DmsCustody::Absent
                || case.retained.shm_file == selected_success))
    {
        return Err("SHM file-close seam custody does not match before/native/after order");
    }
    Ok(())
}

fn validate_close_and_registry_counts(case: &Case) -> Result<(), &'static str> {
    let direct_unmap_terminal = case.path == Path::Unmap && case.domain_terminal;
    let joint_physical_failure = case.path == Path::JointClose
        && case.variant == 0
        && matches!(
            case.phase,
            Phase::ShmUnmapLift | Phase::MainLockRelease | Phase::MainFileClose
        );
    if (direct_unmap_terminal || joint_physical_failure)
        && (case.counts.callback_begin != 1
            || case.counts.callback_complete_attempt != 1
            || case.counts.callback_complete_success != 0
            || case.retained.callback_leases != 1)
    {
        return Err("physical route quarantine must reject completion and retain its lease");
    }
    if case.path == Path::JointClose && case.phase == Phase::Success {
        if case.registry_route_phase != RegistryRoutePhase::Closing
            || case.retained.callback_leases != 1
            || case.retained.main_file
            || case.retained.main_lock_owner
        {
            return Err("physical close success lacks its pending registry receipt boundary");
        }
    }
    if case.path == Path::JointClose
        && case.phase == Phase::MainLockRelease
        && case.timing == Timing::AfterSuccessKnown
        && (!case.retained.main_file || !case.retained.main_lock_owner)
    {
        return Err("main unlock receipt incorrectly consumed the live main owner");
    }
    if case.path == Path::JointClose
        && case.phase == Phase::MainFileClose
        && case.variant == 0
        && case.timing == Timing::AfterSuccessKnown
        && (case.retained.main_file || case.retained.main_lock_owner)
    {
        return Err("main file-close receipt failed to consume its owner");
    }
    if case.path == Path::JointClose
        && case.phase == Phase::RegistryWalMainClose
        && case.timing == Timing::NativeUncertain
        && (case.counts.registry_close_attempt != 1
            || case.counts.registry_close_success != 0
            || case.counts.callback_complete_attempt != 1
            || case.counts.callback_complete_success != 0)
    {
        return Err("native registry WAL-main rejection counts are not exact");
    }
    if case.path == Path::RegistryLifecycle {
        let timing_success = u8::from(case.timing == Timing::AfterSuccessKnown);
        match case.phase {
            Phase::ConnectionObservation
                if case.counts.connection_observe_attempt
                    != u8::from(case.variant == 1 || timing_success == 1)
                    || case.counts.connection_observe_success != timing_success =>
            {
                return Err("connection-observation attempt/success counts are not exact");
            }
            Phase::RegistryRouteRemoval
                if case.counts.registry_route_remove_attempt
                    != u8::from(case.timing != Timing::BeforeCall)
                    || case.counts.registry_route_remove_success
                        != u8::from(timing_success == 1 || case.variant == 2) =>
            {
                return Err("registry-route removal attempt/success counts are not exact");
            }
            Phase::LogicalRouteRemoval
                if case.post.sqlite_connections != 0
                    || case.counts.logical_names_remove_attempt
                        != u8::from(timing_success == 1 || case.variant == 2)
                    || case.counts.logical_names_remove_success != timing_success
                    || case.counts.logical_names_remove != timing_success * 3 =>
            {
                return Err("logical-name removal attempt/success/count is not exact");
            }
            Phase::Success
                if case.counts.connection_observe_attempt != 1
                    || case.counts.connection_observe_success != 1
                    || case.counts.registry_route_remove_attempt != 1
                    || case.counts.registry_route_remove_success != 1
                    || case.counts.logical_names_remove_attempt != 1
                    || case.counts.logical_names_remove_success != 1
                    || case.counts.logical_names_remove != 3 =>
            {
                return Err("registry lifecycle success is missing a receipt-chain step");
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_root_release(case: &Case) -> Result<(), &'static str> {
    if !case.retained.root_deletable {
        return Ok(());
    }
    if case.registration_phase != RegistrationPhase::Unregistered
        || case.registry_route_phase != RegistryRoutePhase::Removed
        || case.logical_route_phase != LogicalRoutePhase::Removed
        || case.post.sqlite_connections != 0
        || case.post.shm_connections != 0
        || case.post.registry_routes != 0
        || case.post.logical_names != 0
        || case.retained.node
        || case.retained.views != 0
        || case.retained.mappings != 0
        || case.retained.dms != DmsCustody::Absent
        || case.retained.shm_file
        || case.retained.main_file
        || case.retained.main_lock_owner
        || case.retained.main_lease
        || case.retained.shm_lease
        || case.retained.callback_leases != 0
        || case.retained.registry_entry
        || case.retained.logical_names != 0
        || case.retained.vfs_table
        || case.retained.vfs_name
        || case.retained.vfs_context
    {
        return Err("test root deletion lacks complete physical and registration release proof");
    }
    Ok(())
}
