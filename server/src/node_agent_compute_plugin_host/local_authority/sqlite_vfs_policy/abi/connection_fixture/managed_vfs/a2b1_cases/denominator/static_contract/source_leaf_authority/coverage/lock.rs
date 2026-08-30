use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    super::model as graph,
    lock_ranges::{validate_lock_ranges, LockActionV1, LockRangeV1, LOCK_RANGES},
};
use super::{GraphIndex, RequiredKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Coordinate {
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
}

pub(super) fn validate(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
) -> Result<(), String> {
    validate_lock_ranges()?;
    let authorized = LOCK_RANGES
        .iter()
        .map(|range| (coordinate(range), range))
        .collect::<BTreeMap<_, _>>();
    if authorized.len() != LOCK_RANGES.len() {
        return Err("Lock authority repeats a graph range coordinate".to_owned());
    }
    scan_graph_coordinates(contract, &authorized)?;
    for range in LOCK_RANGES {
        validate_range(contract, index, range)?;
    }
    Ok(())
}

pub(super) fn graph_prefix(range: &LockRangeV1) -> String {
    format!(
        "lock.request.{}.first{}.count{}.mask{:02x}",
        range.action.canonical_name(),
        range.first,
        range.count,
        range.mask
    )
}

fn coordinate(range: &LockRangeV1) -> Coordinate {
    Coordinate {
        action: range.action,
        first: range.first,
        count: range.count,
        mask: range.mask,
    }
}

fn scan_graph_coordinates(
    contract: &graph::ContractGraph,
    authorized: &BTreeMap<Coordinate, &LockRangeV1>,
) -> Result<(), String> {
    let mut observed = BTreeSet::new();
    for id in contract
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .chain(contract.source_leaf_universe.iter().map(String::as_str))
    {
        let Some(found) = parse_coordinate(id)? else {
            continue;
        };
        if !authorized.contains_key(&found) {
            return Err(format!(
                "Lock graph contains unauthorized action/range coordinate {found:?}"
            ));
        }
        observed.insert(found);
    }
    let expected = authorized.keys().copied().collect::<BTreeSet<_>>();
    if observed != expected {
        let missing = expected.difference(&observed).take(8).collect::<Vec<_>>();
        return Err(format!(
            "Lock graph does not cover every independent range; missing={missing:?}"
        ));
    }
    Ok(())
}

fn parse_coordinate(id: &str) -> Result<Option<Coordinate>, String> {
    let Some(rest) = id.strip_prefix("lock.request.") else {
        return Ok(None);
    };
    let (action_text, rest) = rest
        .split_once('.')
        .ok_or_else(|| format!("Lock graph has a malformed request id: {id:?}"))?;
    let action = parse_action(action_text)
        .ok_or_else(|| format!("Lock graph has an unauthorized request action: {id:?}"))?;
    if rest == "validation" || rest.starts_with("rejected.") {
        return Ok(None);
    }
    let mut parts = rest.split('.');
    let first = parse_decimal(parts.next(), "first", id)?;
    let count = parse_decimal(parts.next(), "count", id)?;
    let mask_text = parts
        .next()
        .and_then(|part| part.strip_prefix("mask"))
        .filter(|text| {
            text.len() == 2
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| format!("Lock graph has a malformed range mask: {id:?}"))?;
    if parts.next().is_none() {
        return Err(format!("Lock graph range id has no node suffix: {id:?}"));
    }
    let mask = u8::from_str_radix(mask_text, 16)
        .map_err(|_| format!("Lock graph has an invalid range mask: {id:?}"))?;
    Ok(Some(Coordinate {
        action,
        first,
        count,
        mask,
    }))
}

fn parse_action(value: &str) -> Option<LockActionV1> {
    match value {
        "lock-shared" => Some(LockActionV1::LockShared),
        "lock-exclusive" => Some(LockActionV1::LockExclusive),
        "unlock-shared" => Some(LockActionV1::UnlockShared),
        "unlock-exclusive" => Some(LockActionV1::UnlockExclusive),
        _ => None,
    }
}

fn parse_decimal(part: Option<&str>, prefix: &str, id: &str) -> Result<u8, String> {
    let text = part
        .and_then(|part| part.strip_prefix(prefix))
        .filter(|text| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| format!("Lock graph has a malformed {prefix} coordinate: {id:?}"))?;
    let value = text
        .parse::<u8>()
        .map_err(|_| format!("Lock graph has an invalid {prefix} coordinate: {id:?}"))?;
    if text != value.to_string() {
        return Err(format!(
            "Lock graph has a non-canonical {prefix} coordinate: {id:?}"
        ));
    }
    Ok(value)
}

fn validate_range(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    range: &LockRangeV1,
) -> Result<(), String> {
    let prefix = graph_prefix(range);
    let valid = format!("{prefix}.valid");
    index.require_node(&valid, RequiredKind::Continuation)?;
    index.require_edge(
        &format!("lock.request.{}.validation", range.action.canonical_name()),
        &valid,
        graph::DecisionStage::ManagedRequest,
        &format!(
            "valid-orbit.first{}.count{}.mask{:02x}",
            range.first, range.count, range.mask
        ),
    )?;
    index.require_leaf(
        contract,
        &format!("{prefix}.admission-rejected.excluded.owner-poisoned"),
        RequiredKind::Excluded,
    )?;
    index.require_leaf(
        contract,
        &format!("{prefix}.admission-rejected.excluded.identity-mismatch"),
        RequiredKind::Excluded,
    )?;
    validate_terminal_effects(contract, index, range, &prefix)
}

fn validate_terminal_effects(
    contract: &graph::ContractGraph,
    index: &GraphIndex<'_>,
    range: &LockRangeV1,
    prefix: &str,
) -> Result<(), String> {
    let mut found_intended_effect = false;
    let mode = if range.action.is_shared() {
        graph::LockMode::Shared
    } else {
        graph::LockMode::Exclusive
    };
    for id in contract
        .source_leaf_universe
        .range(prefix.to_owned()..)
        .take_while(|id| id.starts_with(prefix))
    {
        let node = index.require_node(
            id,
            match index_node_kind(index, id)? {
                RequiredKind::Terminal => RequiredKind::Terminal,
                RequiredKind::Excluded => continue,
                _ => return Err(format!("Lock source leaf is not final: {id:?}")),
            },
        )?;
        let graph::NodeKind::Terminal { expected, .. } = &node.kind else {
            unreachable!("terminal kind was checked")
        };
        match expected.lock_effect {
            graph::LockEffect::NotReached | graph::LockEffect::Unchanged => {}
            graph::LockEffect::Acquired {
                mode: actual_mode,
                mask,
                ..
            } => {
                validate_effect_coordinate(id, actual_mode, mask, mode, range.mask)?;
                if !matches!(
                    range.action,
                    LockActionV1::LockShared | LockActionV1::LockExclusive
                ) {
                    return Err(format!(
                        "Lock unlock range {prefix:?} has an acquired terminal effect"
                    ));
                }
                found_intended_effect = true;
            }
            graph::LockEffect::Released {
                mode: actual_mode,
                mask,
                ..
            } => {
                validate_effect_coordinate(id, actual_mode, mask, mode, range.mask)?;
                if !matches!(
                    range.action,
                    LockActionV1::UnlockShared | LockActionV1::UnlockExclusive
                ) {
                    return Err(format!(
                        "Lock acquire range {prefix:?} has a released terminal effect"
                    ));
                }
                found_intended_effect = true;
            }
            graph::LockEffect::OutcomeUncertain {
                mode: actual_mode,
                mask,
            } => validate_effect_coordinate(id, actual_mode, mask, mode, range.mask)?,
        }
    }
    if !found_intended_effect {
        return Err(format!(
            "Lock range {prefix:?} has no terminal carrying its intended action effect"
        ));
    }
    Ok(())
}

fn index_node_kind(index: &GraphIndex<'_>, id: &str) -> Result<RequiredKind, String> {
    for kind in [RequiredKind::Terminal, RequiredKind::Excluded] {
        if index.require_node(id, kind).is_ok() {
            return Ok(kind);
        }
    }
    Err(format!(
        "Lock source leaf is absent from graph nodes: {id:?}"
    ))
}

fn validate_effect_coordinate(
    id: &str,
    actual_mode: graph::LockMode,
    actual_mask: u8,
    mode: graph::LockMode,
    mask: u8,
) -> Result<(), String> {
    if (actual_mode, actual_mask) == (mode, mask) {
        Ok(())
    } else {
        Err(format!(
            "Lock terminal {id:?} effect disagrees with independent range mode/mask"
        ))
    }
}
