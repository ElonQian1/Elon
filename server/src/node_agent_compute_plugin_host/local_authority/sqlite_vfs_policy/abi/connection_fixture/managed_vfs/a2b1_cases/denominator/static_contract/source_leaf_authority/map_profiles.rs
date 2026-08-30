//! Independently reviewed Map loop profiles and ordinal domains.
//!
//! The 21 rows below are literal authority data. They must not be generated from, or replaced by
//! calls into, the graph's `LoopSpec`, initialization expansion or post-initialization helpers.

use std::collections::BTreeSet;

use super::{canonical, model::Digest32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapModeV1 {
    Observe,
    Extend,
}

impl MapModeV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Extend => "extend",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapInitializationProfileV1 {
    NodeLive,
    CreatedFirstShared,
    CreatedJoinerShared,
    ExistingFirstShared,
    ExistingJoinerShared,
}

impl MapInitializationProfileV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::NodeLive => "node-live",
            Self::CreatedFirstShared => "created-first-shared",
            Self::CreatedJoinerShared => "created-joiner-shared",
            Self::ExistingFirstShared => "existing-first-shared",
            Self::ExistingJoinerShared => "existing-joiner-shared",
        }
    }

    const fn mutated(self) -> bool {
        matches!(
            self,
            Self::CreatedFirstShared | Self::CreatedJoinerShared | Self::ExistingFirstShared
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapRegionPrestateV1 {
    Empty,
    NonemptyTargetMissing,
}

impl MapRegionPrestateV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Empty => "regions-empty",
            Self::NonemptyTargetMissing => "regions-nonempty-target-missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapRegionSizeArmV1 {
    Same,
    UnsetAssigned,
}

impl MapRegionSizeArmV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Same => "region-size-same",
            Self::UnsetAssigned => "region-size-unset-assigned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MapFilePathV1 {
    SizeSufficient,
    GrowSucceeded,
}

impl MapFilePathV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::SizeSufficient => "size-sufficient",
            Self::GrowSucceeded => "grow-succeeded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OrdinalDomainV1 {
    pub(crate) first: u16,
    pub(crate) last_inclusive: u16,
}

impl OrdinalDomainV1 {
    pub(crate) const fn width(self) -> u16 {
        self.last_inclusive - self.first + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MapLoopProfileV1 {
    pub(crate) id: &'static str,
    pub(crate) mode: MapModeV1,
    pub(crate) initialization: MapInitializationProfileV1,
    pub(crate) prestate: MapRegionPrestateV1,
    pub(crate) region_size_arm: MapRegionSizeArmV1,
    pub(crate) file_path: MapFilePathV1,
    pub(crate) ordinals: OrdinalDomainV1,
    pub(crate) prior_mutation: bool,
    pub(crate) preexisting_mapping: bool,
    pub(crate) file_grow_count: u16,
}

const EMPTY_ORDINALS: OrdinalDomainV1 = OrdinalDomainV1 {
    first: 1,
    last_inclusive: 256,
};
const MISSING_ORDINALS: OrdinalDomainV1 = OrdinalDomainV1 {
    first: 1,
    last_inclusive: 255,
};

const fn profile(
    id: &'static str,
    mode: MapModeV1,
    initialization: MapInitializationProfileV1,
    prestate: MapRegionPrestateV1,
    region_size_arm: MapRegionSizeArmV1,
    file_path: MapFilePathV1,
    prior_mutation: bool,
) -> MapLoopProfileV1 {
    let preexisting_mapping = matches!(prestate, MapRegionPrestateV1::NonemptyTargetMissing);
    MapLoopProfileV1 {
        id,
        mode,
        initialization,
        prestate,
        region_size_arm,
        file_path,
        ordinals: if preexisting_mapping {
            MISSING_ORDINALS
        } else {
            EMPTY_ORDINALS
        },
        prior_mutation,
        preexisting_mapping,
        file_grow_count: if matches!(file_path, MapFilePathV1::GrowSucceeded) {
            1
        } else {
            0
        },
    }
}

macro_rules! loop_profile {
    ($id:literal, $mode:ident, $init:ident, $pre:ident, $arm:ident, $file:ident, $mutated:literal) => {
        profile(
            $id,
            MapModeV1::$mode,
            MapInitializationProfileV1::$init,
            MapRegionPrestateV1::$pre,
            MapRegionSizeArmV1::$arm,
            MapFilePathV1::$file,
            $mutated,
        )
    };
}

pub(crate) const MAP_LOOP_PROFILES: &[MapLoopProfileV1; 21] = &[
    loop_profile!(
        "map.observe.node-live.empty.same.sufficient",
        Observe,
        NodeLive,
        Empty,
        Same,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.observe.node-live.empty.unset.sufficient",
        Observe,
        NodeLive,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.observe.node-live.missing.same.sufficient",
        Observe,
        NodeLive,
        NonemptyTargetMissing,
        Same,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.observe.created-first.empty.unset.sufficient",
        Observe,
        CreatedFirstShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.observe.created-joiner.empty.unset.sufficient",
        Observe,
        CreatedJoinerShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.observe.existing-first.empty.unset.sufficient",
        Observe,
        ExistingFirstShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.observe.existing-joiner.empty.unset.sufficient",
        Observe,
        ExistingJoinerShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.extend.node-live.empty.same.sufficient",
        Extend,
        NodeLive,
        Empty,
        Same,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.extend.node-live.empty.unset.sufficient",
        Extend,
        NodeLive,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.extend.node-live.missing.same.sufficient",
        Extend,
        NodeLive,
        NonemptyTargetMissing,
        Same,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.extend.created-first.empty.unset.sufficient",
        Extend,
        CreatedFirstShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.extend.created-joiner.empty.unset.sufficient",
        Extend,
        CreatedJoinerShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.extend.existing-first.empty.unset.sufficient",
        Extend,
        ExistingFirstShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        true
    ),
    loop_profile!(
        "map.extend.existing-joiner.empty.unset.sufficient",
        Extend,
        ExistingJoinerShared,
        Empty,
        UnsetAssigned,
        SizeSufficient,
        false
    ),
    loop_profile!(
        "map.extend.node-live.empty.same.grow",
        Extend,
        NodeLive,
        Empty,
        Same,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.node-live.empty.unset.grow",
        Extend,
        NodeLive,
        Empty,
        UnsetAssigned,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.node-live.missing.same.grow",
        Extend,
        NodeLive,
        NonemptyTargetMissing,
        Same,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.created-first.empty.unset.grow",
        Extend,
        CreatedFirstShared,
        Empty,
        UnsetAssigned,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.created-joiner.empty.unset.grow",
        Extend,
        CreatedJoinerShared,
        Empty,
        UnsetAssigned,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.existing-first.empty.unset.grow",
        Extend,
        ExistingFirstShared,
        Empty,
        UnsetAssigned,
        GrowSucceeded,
        true
    ),
    loop_profile!(
        "map.extend.existing-joiner.empty.unset.grow",
        Extend,
        ExistingJoinerShared,
        Empty,
        UnsetAssigned,
        GrowSucceeded,
        true
    ),
];

pub(crate) fn validate_map_profiles() -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for profile in MAP_LOOP_PROFILES {
        if profile.id.is_empty() || !ids.insert(profile.id) {
            return Err("Map authority repeats or empties a loop profile id".to_owned());
        }
        if profile.ordinals.first != 1
            || profile.ordinals.last_inclusive
                != if profile.preexisting_mapping {
                    255
                } else {
                    256
                }
        {
            return Err(format!("{} has the wrong ordinal domain", profile.id));
        }
        if matches!(profile.mode, MapModeV1::Observe)
            && !matches!(profile.file_path, MapFilePathV1::SizeSufficient)
        {
            return Err(format!("{} lets Observe grow the file", profile.id));
        }
        let expected_prior_mutation = profile.initialization.mutated()
            || profile.preexisting_mapping
            || matches!(profile.file_path, MapFilePathV1::GrowSucceeded);
        if profile.prior_mutation != expected_prior_mutation
            || profile.file_grow_count
                != u16::from(matches!(profile.file_path, MapFilePathV1::GrowSucceeded))
        {
            return Err(format!(
                "{} has a wrong mutation/grow projection",
                profile.id
            ));
        }
    }
    let observe = MAP_LOOP_PROFILES
        .iter()
        .filter(|profile| profile.mode == MapModeV1::Observe)
        .count();
    let extend = MAP_LOOP_PROFILES.len() - observe;
    let ordinal_cells: usize = MAP_LOOP_PROFILES
        .iter()
        .map(|profile| usize::from(profile.ordinals.width()))
        .sum();
    if (observe, extend, ordinal_cells) != (7, 14, 5_373) {
        return Err(
            "Map authority lost its exact 7/14 profile or 5,373 ordinal partition".to_owned(),
        );
    }
    Ok(())
}

pub(crate) fn map_profile_set_sha256() -> Digest32 {
    canonical::digest_map_profile_set(MAP_LOOP_PROFILES)
}

pub(crate) fn map_ordinal_domain_sha256() -> Digest32 {
    canonical::digest_map_ordinal_domains(MAP_LOOP_PROFILES)
}
