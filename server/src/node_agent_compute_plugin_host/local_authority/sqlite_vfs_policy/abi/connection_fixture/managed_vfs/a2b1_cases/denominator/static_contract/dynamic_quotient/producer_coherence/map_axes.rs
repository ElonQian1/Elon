use super::super::super::terminal_descriptor::{
    InitializationProfileV1, MapFilePathV1, MapModeV1, MapProfileV1, MapRegionPrestateV1,
    MapRegionSizeArmV1, MapTerminalDescriptorV1, OccurrenceV1, ReachabilityV1,
};
use super::super::projector::{ProjectionErrorV1, ProjectionViolationV1};
use super::invalid;

pub(super) fn validate(value: MapTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    match (value.axes.ordinal, value.occurrence) {
        (ReachabilityV1::NotReached, OccurrenceV1::Natural)
        | (ReachabilityV1::Reached(_), OccurrenceV1::Exact(_)) => {}
        _ => return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch)),
    }
    if let (ReachabilityV1::Reached(ordinal), OccurrenceV1::Exact(occurrence)) =
        (value.axes.ordinal, value.occurrence)
    {
        if ordinal != occurrence {
            return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch));
        }
    }
    if let ReachabilityV1::Reached(profile) = value.axes.profile {
        if !valid_profile(profile) {
            return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch));
        }
        if let ReachabilityV1::Reached(ordinal) = value.axes.ordinal {
            let valid_loop_profile = matches!(
                profile.file_path,
                MapFilePathV1::SizeSufficient | MapFilePathV1::GrowSucceeded
            ) && matches!(
                profile.prestate,
                MapRegionPrestateV1::Empty | MapRegionPrestateV1::NonemptyTargetMissing
            ) && (profile.prestate
                != MapRegionPrestateV1::NonemptyTargetMissing
                || ordinal <= 255);
            if !valid_loop_profile {
                return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch));
            }
        }
    }
    Ok(())
}

fn valid_profile(profile: MapProfileV1) -> bool {
    let initialization_prestate = match (profile.initialization, profile.prestate) {
        (InitializationProfileV1::NodeLive, MapRegionPrestateV1::Empty) => {
            !profile.preexisting_mapping
        }
        (
            InitializationProfileV1::NodeLive,
            MapRegionPrestateV1::NonemptyTargetMissing | MapRegionPrestateV1::Reuse,
        ) => profile.preexisting_mapping,
        (
            InitializationProfileV1::CreatedFirstShared
            | InitializationProfileV1::CreatedJoinerShared
            | InitializationProfileV1::ExistingFirstShared,
            MapRegionPrestateV1::Empty,
        ) => !profile.preexisting_mapping,
        (InitializationProfileV1::ExistingJoinerShared, MapRegionPrestateV1::Empty) => {
            !profile.preexisting_mapping
        }
        _ => false,
    };
    let region_arm = match profile.initialization {
        InitializationProfileV1::NodeLive => match profile.prestate {
            MapRegionPrestateV1::Empty => matches!(
                profile.region_size_arm,
                MapRegionSizeArmV1::Changed
                    | MapRegionSizeArmV1::Same
                    | MapRegionSizeArmV1::UnsetAssigned
            ),
            MapRegionPrestateV1::NonemptyTargetMissing | MapRegionPrestateV1::Reuse => matches!(
                profile.region_size_arm,
                MapRegionSizeArmV1::Changed | MapRegionSizeArmV1::Same
            ),
            MapRegionPrestateV1::ObserveNotPresent => false,
        },
        _ => {
            profile.prestate == MapRegionPrestateV1::Empty
                && profile.region_size_arm == MapRegionSizeArmV1::UnsetAssigned
        }
    };
    let file_path = match profile.region_size_arm {
        MapRegionSizeArmV1::Changed => profile.file_path == MapFilePathV1::NotReached,
        MapRegionSizeArmV1::Same | MapRegionSizeArmV1::UnsetAssigned => match profile.file_path {
            MapFilePathV1::NotReached | MapFilePathV1::SizeSufficient => true,
            MapFilePathV1::ObserveNotPresent => profile.mode == MapModeV1::Observe,
            MapFilePathV1::GrowAttempted | MapFilePathV1::GrowSucceeded => {
                profile.mode == MapModeV1::Extend
            }
        },
        MapRegionSizeArmV1::NotReached => false,
    };
    let prior_mutation = profile.prior_mutation
        == (profile.preexisting_mapping
            || matches!(
                profile.initialization,
                InitializationProfileV1::CreatedFirstShared
                    | InitializationProfileV1::CreatedJoinerShared
                    | InitializationProfileV1::ExistingFirstShared
            )
            || profile.file_path == MapFilePathV1::GrowSucceeded);
    initialization_prestate && region_arm && file_path && prior_mutation
}
