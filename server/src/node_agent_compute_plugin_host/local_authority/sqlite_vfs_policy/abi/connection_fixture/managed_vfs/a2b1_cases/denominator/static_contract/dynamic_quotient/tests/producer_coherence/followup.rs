use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::terminal_descriptor::MapStoredPoisonPrestateV1;

#[test]
fn map_stored_poison_uses_managed_request_seam() {
    let mut descriptor = map_file_descriptor(true);
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.source_site = SourceSiteV1::CoordinatorState;
    value.stimulus = StimulusV1::MapManaged(MapManagedStimulusV1::StoredPoison);
    value.prestate = PrestateV1::Map(MapPrestateV1::StoredPoison(
        MapStoredPoisonPrestateV1::NoNode,
    ));
    value.operation = MapOperationV1::ManagedRequest;
    value.phase = PhaseV1::Gate;
    value.timing = TimingV1::BeforeCall;
    value.recipe.fault_seam = FaultSeamV1::ManagedRequest;
    value.axes.profile = ReachabilityV1::NotReached;

    let record = record(RootOperationV1::Map, PhaseV1::Gate);
    assert_missing(&record, &descriptor);
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.recipe.fault_seam = FaultSeamV1::Natural;
    assert_invalid(
        &record,
        &descriptor,
        ProjectionViolationV1::MapProducerTupleMismatch,
    );
}

#[test]
fn capability_gap_is_root_specific_before_runner_gate() {
    let map_record = record(RootOperationV1::Map, PhaseV1::FileSize);
    for capability in [
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        RunnerCapabilityV1::Supported,
    ] {
        let mut descriptor = map_file_descriptor(false);
        let TerminalDescriptorV1::Map(value) = &mut descriptor else {
            unreachable!()
        };
        value.recipe.capability = capability;
        assert_invalid(
            &map_record,
            &descriptor,
            ProjectionViolationV1::MapProducerRecipeMismatch,
        );
    }

    let lock_record = record(RootOperationV1::Lock, PhaseV1::LockAcquire);
    for capability in [
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
        RunnerCapabilityV1::Supported,
    ] {
        let mut descriptor = lock_local_descriptor();
        let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
            unreachable!()
        };
        value.recipe.capability = capability;
        assert_invalid(
            &lock_record,
            &descriptor,
            ProjectionViolationV1::LockProducerRecipeMismatch,
        );
    }
}

#[test]
fn map_prior_mutation_is_closed_over_profile_origin() {
    let mut descriptor = map_loop_descriptor();
    let record = record(RootOperationV1::Map, PhaseV1::MappingCreate);
    assert_missing(&record, &descriptor);
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    let ReachabilityV1::Reached(profile) = &mut value.axes.profile else {
        unreachable!()
    };
    profile.prior_mutation = true;
    assert_invalid(
        &record,
        &descriptor,
        ProjectionViolationV1::MapProducerAxesMismatch,
    );

    let mut created = map_loop_descriptor();
    let TerminalDescriptorV1::Map(value) = &mut created else {
        unreachable!()
    };
    let ReachabilityV1::Reached(profile) = &mut value.axes.profile else {
        unreachable!()
    };
    profile.initialization = InitializationProfileV1::CreatedFirstShared;
    profile.region_size_arm = MapRegionSizeArmV1::UnsetAssigned;
    profile.prior_mutation = true;
    assert_missing(&record, &created);
    let TerminalDescriptorV1::Map(value) = &mut created else {
        unreachable!()
    };
    let ReachabilityV1::Reached(profile) = &mut value.axes.profile else {
        unreachable!()
    };
    profile.prior_mutation = false;
    assert_invalid(
        &record,
        &created,
        ProjectionViolationV1::MapProducerAxesMismatch,
    );
}
