use super::super::super::terminal_descriptor::{
    CapabilityGapV1, InitializationProfileV1, MapAxesV1, MapCompletionV1, MapFilePathV1, MapModeV1,
    MapPrestateV1, MapProfileV1, MapRegionPrestateV1, MapRegionSizeArmV1, PrestateV1,
    ReachabilityV1, RunnerCapabilityV1, TerminalDescriptorV1,
};
use super::super::{project_dynamic_class_v1, ProjectionErrorV1, ProjectionViolationV1};
use super::{descriptor, record};

fn with_map_axes(
    mut descriptor: TerminalDescriptorV1,
    prestate: MapPrestateV1,
    axes: MapAxesV1,
) -> TerminalDescriptorV1 {
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!("test helper only accepts Map descriptors")
    };
    value.prestate = PrestateV1::Map(prestate);
    value.axes = axes;
    descriptor
}

fn profile(prestate: MapRegionPrestateV1, preexisting_mapping: bool) -> MapProfileV1 {
    MapProfileV1 {
        mode: MapModeV1::Extend,
        initialization: InitializationProfileV1::NodeLive,
        prestate,
        region_size_arm: MapRegionSizeArmV1::Same,
        file_path: MapFilePathV1::SizeSufficient,
        prior_mutation: preexisting_mapping,
        preexisting_mapping,
    }
}

fn profile_axes(profile: MapProfileV1) -> MapAxesV1 {
    MapAxesV1 {
        mode: ReachabilityV1::Reached(profile.mode),
        profile: ReachabilityV1::Reached(profile),
        completion: ReachabilityV1::Reached(MapCompletionV1::Direct),
        ..MapAxesV1::NOT_REACHED
    }
}

fn missing_descriptor() -> TerminalDescriptorV1 {
    descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ))
}

#[test]
fn completion_is_required_before_missing_capability() {
    let record = record("map-completion", "invalid-region");
    let descriptor = with_map_axes(
        missing_descriptor(),
        MapPrestateV1::NotReached,
        MapAxesV1::NOT_REACHED,
    );
    assert_eq!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::MapCompletionNotReached,
        ))
    );
}

#[test]
fn loop_axes_are_exact_and_bounded_before_missing_capability() {
    let record = record("map-loop-axes", "invalid-region");
    let base = profile_axes(profile(MapRegionPrestateV1::Empty, false));
    for (ordinal, regions, violation) in [
        (
            ReachabilityV1::Reached(1),
            ReachabilityV1::NotReached,
            ProjectionViolationV1::PartialMapLoopAxes,
        ),
        (
            ReachabilityV1::Reached(1),
            ReachabilityV1::Reached(2),
            ProjectionViolationV1::MapOrdinalRegionsMismatch,
        ),
        (
            ReachabilityV1::Reached(0),
            ReachabilityV1::Reached(0),
            ProjectionViolationV1::MapLoopOrdinalOutOfRange,
        ),
        (
            ReachabilityV1::Reached(257),
            ReachabilityV1::Reached(257),
            ProjectionViolationV1::MapLoopOrdinalOutOfRange,
        ),
    ] {
        let descriptor = with_map_axes(
            missing_descriptor(),
            MapPrestateV1::RegionsEmpty,
            MapAxesV1 {
                ordinal,
                regions_to_create: regions,
                ..base
            },
        );
        assert_eq!(
            project_dynamic_class_v1(&record, &descriptor),
            Err(ProjectionErrorV1::Invalid(violation))
        );
    }
}

#[test]
fn profile_must_match_mode_prestate_and_mapping_presence() {
    let record = record("map-profile", "invalid-region");
    for (prestate, profile, mode, violation) in [
        (
            MapPrestateV1::TargetMissing,
            profile(MapRegionPrestateV1::Empty, false),
            MapModeV1::Extend,
            ProjectionViolationV1::MapProfilePrestateMismatch,
        ),
        (
            MapPrestateV1::RegionsEmpty,
            profile(MapRegionPrestateV1::Empty, true),
            MapModeV1::Extend,
            ProjectionViolationV1::MapProfilePrestateMismatch,
        ),
        (
            MapPrestateV1::RegionsEmpty,
            profile(MapRegionPrestateV1::ObserveNotPresent, false),
            MapModeV1::Extend,
            ProjectionViolationV1::MapProfilePrestateMismatch,
        ),
        (
            MapPrestateV1::RegionsEmpty,
            profile(MapRegionPrestateV1::Empty, false),
            MapModeV1::Observe,
            ProjectionViolationV1::MapProfileModeMismatch,
        ),
    ] {
        let profile_axes = profile_axes(profile);
        let descriptor = with_map_axes(
            missing_descriptor(),
            prestate,
            MapAxesV1 {
                mode: ReachabilityV1::Reached(mode),
                ..profile_axes
            },
        );
        assert_eq!(
            project_dynamic_class_v1(&record, &descriptor),
            Err(ProjectionErrorV1::Invalid(violation))
        );
    }
}
