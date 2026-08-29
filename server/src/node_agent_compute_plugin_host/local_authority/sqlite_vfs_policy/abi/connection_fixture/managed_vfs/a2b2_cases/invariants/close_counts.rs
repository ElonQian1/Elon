//! Joint-close and RegistryLifecycle count/custody invariants.

use super::super::model::{Case, Path, Phase, RegistryRoutePhase, Timing, TopologyKind};

pub(super) fn validate(case: &Case) -> Result<(), &'static str> {
    let direct_unmap_terminal = case.path == Path::Unmap && case.domain_terminal;
    let joint_physical_failure = case.path == Path::JointClose
        && matches!(
            case.phase,
            Phase::ShmUnmapLift | Phase::MainLockRelease | Phase::MainFileClose
        )
        && !(case.phase == Phase::MainFileClose && case.variant == 1);
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
        && case.lock_outcome_uncertain != (case.timing == Timing::NativeUncertain)
    {
        return Err("main unlock uncertainty differs from its exact native receipt boundary");
    }
    if case.path == Path::JointClose
        && case.phase == Phase::MainFileClose
        && case.lock_outcome_uncertain
    {
        return Err("main file close cannot invent a main unlock uncertainty");
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
    validate_registry_lifecycle(case)
}

fn validate_registry_lifecycle(case: &Case) -> Result<(), &'static str> {
    if case.path != Path::RegistryLifecycle {
        return Ok(());
    }
    if case.topology_kind == TopologyKind::SharedNonFinal && case.phase != Phase::Success {
        return Err("RegistryLifecycle shared topology is valid only for success");
    }
    let expected_pre_sqlite = 1 + u8::from(case.topology_kind == TopologyKind::SharedNonFinal);
    let expected_post_sqlite = u8::from(case.topology_kind == TopologyKind::SharedNonFinal);
    if case.pre.sqlite_connections != expected_pre_sqlite
        || case.post.sqlite_connections != expected_post_sqlite
    {
        return Err("RegistryLifecycle SQLite connection topology is not exact");
    }
    let timing_success = u8::from(case.timing == Timing::AfterSuccessKnown);
    match case.phase {
        Phase::ConnectionObservation
            if case.counts.connection_observe_attempt
                != u8::from(case.variant == 1 || timing_success == 1)
                || case.counts.connection_observe_success != timing_success =>
        {
            Err("connection-observation attempt/success counts are not exact")
        }
        Phase::RegistryRouteRemoval
            if case.counts.registry_route_remove_attempt
                != u8::from(case.timing != Timing::BeforeCall)
                || case.counts.registry_route_remove_success
                    != u8::from(timing_success == 1 || case.variant == 2) =>
        {
            Err("registry-route removal attempt/success counts are not exact")
        }
        Phase::LogicalRouteRemoval
            if case.post.sqlite_connections != 0
                || case.counts.logical_names_remove_attempt
                    != u8::from(timing_success == 1 || case.variant == 2)
                || case.counts.logical_names_remove_success != timing_success
                || case.counts.logical_names_remove != timing_success * 3 =>
        {
            Err("logical-name removal attempt/success/count is not exact")
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
            Err("registry lifecycle success is missing a receipt-chain step")
        }
        _ => Ok(()),
    }
}
