use std::collections::BTreeMap;

use super::super::{
    super::model as graph,
    map_profiles::{
        validate_map_profiles, MapFilePathV1, MapInitializationProfileV1, MapLoopProfileV1,
        MapRegionSizeArmV1, MAP_LOOP_PROFILES,
    },
};
use super::{GraphIndex, RequiredKind};

const REQUIRED_EXCLUSIONS: &[&str] = &[
    "node-missing-before-map",
    "node-missing-during-map",
    "region-index-overflow",
    "region-offset-overflow",
    "view-shift-overflow",
    "region-length-overflow",
    "view-length-overflow",
    "node-missing-before-budget",
    "mapped-total-overflow",
    "node-missing-before-create",
    "mapped-size-budget",
    "mapping-size-zero",
    "mapping-size-above-i64",
    "view-length-zero",
    "view-offset-unaligned",
    "cached-granularity-failed",
    "null-view",
    "node-missing-after-view-map",
];

pub(super) fn validate(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
) -> Result<(), String> {
    validate_map_profiles()?;
    let mut authorized = BTreeMap::new();
    for profile in MAP_LOOP_PROFILES {
        let prefix = graph_prefix(profile);
        if authorized.insert(prefix.clone(), profile).is_some() {
            return Err(format!(
                "Map authority maps two profiles to graph prefix {prefix:?}"
            ));
        }
    }
    scan_graph_coordinates(contract, &authorized)?;
    for (prefix, profile) in authorized {
        validate_profile(contract, index, &prefix, profile)?;
    }
    Ok(())
}

pub(super) fn graph_prefix(profile: &MapLoopProfileV1) -> String {
    let initialization = match profile.initialization {
        MapInitializationProfileV1::NodeLive => "node-live",
        MapInitializationProfileV1::CreatedFirstShared => "created-first-shared",
        MapInitializationProfileV1::CreatedJoinerShared => "created-joiner-shared",
        MapInitializationProfileV1::ExistingFirstShared => "existing-first-shared",
        MapInitializationProfileV1::ExistingJoinerShared => "existing-joiner-shared",
    };
    let region_size = match profile.region_size_arm {
        MapRegionSizeArmV1::Same => "region-size-same",
        MapRegionSizeArmV1::UnsetAssigned => "region-size-unset",
    };
    let file_path = match profile.file_path {
        MapFilePathV1::SizeSufficient => "size-sufficient",
        MapFilePathV1::GrowSucceeded => "extend-grow.succeeded",
    };
    format!(
        "map.{}.managed.initialization.success.{initialization}.post-init.{}.{}.{file_path}.region-loop",
        profile.mode.canonical_name(),
        profile.prestate.canonical_name(),
        region_size,
    )
}

fn scan_graph_coordinates(
    contract: &graph::ContractGraph,
    authorized: &BTreeMap<String, &MapLoopProfileV1>,
) -> Result<(), String> {
    for id in contract
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .chain(contract.source_leaf_universe.iter().map(String::as_str))
    {
        let Some((prefix, coordinate)) = parse_coordinate(id)? else {
            continue;
        };
        let profile = authorized.get(prefix).ok_or_else(|| {
            format!("Map graph contains an unauthorized loop profile prefix {prefix:?}")
        })?;
        match coordinate {
            LoopCoordinate::Cell(ordinal)
                if ordinal >= profile.ordinals.first
                    && ordinal <= profile.ordinals.last_inclusive => {}
            LoopCoordinate::Boundary(ordinal) if ordinal == profile.ordinals.last_inclusive => {}
            LoopCoordinate::Cell(ordinal) | LoopCoordinate::Boundary(ordinal) => {
                return Err(format!(
                    "Map graph contains unauthorized ordinal {ordinal} for {}",
                    profile.id
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopCoordinate {
    Cell(u16),
    Boundary(u16),
}

fn parse_coordinate(id: &str) -> Result<Option<(&str, LoopCoordinate)>, String> {
    const MARKER: &str = ".region-loop";
    let mut matches = id.match_indices(MARKER);
    let Some((offset, _)) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(format!("Map graph id repeats the loop marker: {id:?}"));
    }
    let prefix_end = offset + MARKER.len();
    let suffix = &id[prefix_end..];
    if let Some(rest) = suffix.strip_prefix(".ordinal-") {
        let ordinal = parse_ordinal(rest, true, id)?;
        return Ok(Some((&id[..prefix_end], LoopCoordinate::Cell(ordinal))));
    }
    if let Some(rest) = suffix.strip_prefix(".excluded.target-after-ordinal-") {
        let ordinal = parse_ordinal(rest, false, id)?;
        return Ok(Some((&id[..prefix_end], LoopCoordinate::Boundary(ordinal))));
    }
    Err(format!("Map graph has a malformed loop coordinate: {id:?}"))
}

fn parse_ordinal(rest: &str, requires_suffix: bool, id: &str) -> Result<u16, String> {
    let digits = rest
        .get(..3)
        .filter(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| format!("Map graph has an invalid ordinal: {id:?}"))?;
    let tail = &rest[3..];
    if (requires_suffix && !tail.starts_with('.')) || (!requires_suffix && !tail.is_empty()) {
        return Err(format!("Map graph has a malformed loop coordinate: {id:?}"));
    }
    digits
        .parse::<u16>()
        .map_err(|_| format!("Map graph has an invalid ordinal: {id:?}"))
}

fn validate_profile(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    prefix: &str,
    profile: &MapLoopProfileV1,
) -> Result<(), String> {
    for ordinal in profile.ordinals.first..=profile.ordinals.last_inclusive {
        validate_cell(contract, index, prefix, profile, ordinal)?;
    }
    Ok(())
}

fn validate_cell(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    prefix: &str,
    profile: &MapLoopProfileV1,
    ordinal: u16,
) -> Result<(), String> {
    let cell = format!("{prefix}.ordinal-{ordinal:03}");
    for suffix in [
        "iteration",
        "mapping-create",
        "view-map",
        "view-base",
        "record-region",
        "loop-control",
        "target.selection",
    ] {
        index.require_node(&format!("{cell}.{suffix}"), RequiredKind::Decision)?;
    }
    for suffix in REQUIRED_EXCLUSIONS {
        index.require_leaf(
            contract,
            &format!("{cell}.excluded.{suffix}"),
            RequiredKind::Excluded,
        )?;
    }
    index.require_leaf(
        contract,
        &format!("{cell}.target.excluded.region-custody-missing"),
        RequiredKind::Excluded,
    )?;
    validate_failure_shape(index, &cell, profile.prior_mutation || ordinal > 1)?;
    validate_success(contract, index, &cell, profile, ordinal)?;
    validate_edges(contract, index, prefix, &cell, profile, ordinal)
}

fn validate_failure_shape(
    index: &GraphIndex<'_>,
    cell: &str,
    mutated_before: bool,
) -> Result<(), String> {
    let variants = if mutated_before {
        [
            ("after-known-mutation", true),
            ("io-before-mutation", false),
            ("platform-unsupported", false),
        ]
    } else {
        [
            ("after-known-mutation", false),
            ("io-before-mutation", true),
            ("platform-unsupported", true),
        ]
    };
    for (variant, expected) in variants {
        for (operation, suffix) in [("mapping-create", "cause"), ("view-map", "mapping-close")] {
            let id = format!("{cell}.{operation}-{variant}.{suffix}");
            if index.has_node(&id) != expected {
                return Err(format!(
                    "Map profile mutation ledger disagrees with graph branch {id:?}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_success(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    cell: &str,
    profile: &MapLoopProfileV1,
    ordinal: u16,
) -> Result<(), String> {
    let id = format!("{cell}.target.projection.terminal.success");
    let node = index.require_leaf(contract, &id, RequiredKind::Terminal)?;
    let graph::NodeKind::Terminal { expected, .. } = &node.kind else {
        unreachable!("required terminal was checked")
    };
    let (native_lock, native_unlock, dms_lock) = initialization_observables(profile);
    let counts = expected.counts;
    if expected.sqlite != graph::SqliteResult::Ok
        || expected.disposition != graph::TerminalDisposition::Returned
        || expected.phase != "Success"
        || expected.failure != graph::FailureClass::None
        || expected.mutation != graph::MutationState::Known
        || expected.dms_lock != dms_lock
        || counts.callback_begin != 1
        || counts.callback_complete != 1
        || counts.native_lock != native_lock
        || counts.native_unlock != native_unlock
        || counts.file_grow != profile.file_grow_count
        || counts.mapping_create != ordinal
        || counts.view_map != ordinal
    {
        return Err(format!(
            "Map profile/ordinal ledger disagrees with target success {id:?}"
        ));
    }
    Ok(())
}

fn initialization_observables(profile: &MapLoopProfileV1) -> (u16, u16, graph::DmsLockCustody) {
    match profile.initialization {
        MapInitializationProfileV1::NodeLive => (0, 0, graph::DmsLockCustody::ExistingShared),
        MapInitializationProfileV1::CreatedFirstShared
        | MapInitializationProfileV1::ExistingFirstShared => {
            (2, 1, graph::DmsLockCustody::AcquiredShared)
        }
        MapInitializationProfileV1::CreatedJoinerShared
        | MapInitializationProfileV1::ExistingJoinerShared => {
            (2, 0, graph::DmsLockCustody::AcquiredShared)
        }
    }
}

fn validate_edges(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    prefix: &str,
    cell: &str,
    profile: &MapLoopProfileV1,
    ordinal: u16,
) -> Result<(), String> {
    index.require_edge(
        &format!("{cell}.iteration"),
        &format!("{cell}.mapping-create"),
        graph::DecisionStage::NativeCall,
        &format!("ordinal_{ordinal:03}_validated_create_mapping"),
    )?;
    index.require_edge(
        &format!("{cell}.loop-control"),
        &format!("{cell}.target.selection"),
        graph::DecisionStage::Coordination,
        &format!("requested_target_reached_at_ordinal_{ordinal:03}"),
    )?;
    let branch = format!("requested_target_after_ordinal_{ordinal:03}");
    if ordinal < profile.ordinals.last_inclusive {
        index.require_edge(
            &format!("{cell}.loop-control"),
            &format!("{prefix}.ordinal-{:03}.iteration", ordinal + 1),
            graph::DecisionStage::Coordination,
            &branch,
        )
    } else {
        let beyond = format!("{prefix}.excluded.target-after-ordinal-{ordinal:03}");
        index.require_leaf(contract, &beyond, RequiredKind::Excluded)?;
        index.require_edge(
            &format!("{cell}.loop-control"),
            &beyond,
            graph::DecisionStage::ManagedRequest,
            &branch,
        )
    }
}
