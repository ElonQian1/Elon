mod symbol_span;

use std::collections::{BTreeMap, BTreeSet};

use self::symbol_span::symbol_span;
use super::super::{
    super::owners::{self, OWNERS},
    model::{MapSourceStep, MAP_BOTH},
    scope::OPEN_SOURCE_REVIEW_BOUNDARIES,
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let owner_ids = OWNERS.iter().map(|owner| owner.id).collect::<BTreeSet<_>>();
    let primary_counts = steps.iter().fold(BTreeMap::new(), |mut counts, step| {
        *counts.entry(step.anchor).or_insert(0usize) += 1;
        counts
    });
    let mut exact_anchors = BTreeSet::new();
    for step in steps {
        let anchor = step.anchor;
        if !owner_ids.contains(&anchor.owner) {
            return Err("Map review anchor is detached from the commit-bound owner table");
        }
        if anchor.symbol.is_empty() || anchor.needle.is_empty() || anchor.occurrence == 0 {
            return Err("Map review anchor has an empty symbol/needle or zero occurrence");
        }
        validate_anchor(anchor)?;
        if primary_counts.get(&anchor).copied().unwrap_or(0) > 1 && step.call_context.is_none() {
            return Err("shared Map source branch lacks an explicit call-site context anchor");
        }
        if let Some(context) = step.call_context {
            validate_anchor(context)?;
        }
        if !exact_anchors.insert((anchor, step.call_context)) {
            return Err("Map review ledger contains an unqualified duplicate source anchor");
        }
        if step.ops.is_empty()
            || step.ops.iter().any(|op| !MAP_BOTH.contains(op))
            || step.ops.iter().copied().collect::<BTreeSet<_>>().len() != step.ops.len()
        {
            return Err("Map review step has an empty, duplicated or non-Map operation scope");
        }
        let _reviewed_shape = (
            step.site,
            step.epoch,
            step.effect,
            step.value_flow,
            step.kind,
        );
    }
    for boundary in OPEN_SOURCE_REVIEW_BOUNDARIES {
        if boundary.anchors.is_empty() || boundary.note.is_empty() {
            return Err("Map open source-review boundary lacks anchors or an honest note");
        }
        for anchor in boundary.anchors {
            validate_anchor(*anchor)?;
        }
    }
    Ok(())
}

fn validate_anchor(anchor: super::super::model::SourceAnchor) -> Result<(), &'static str> {
    if anchor.symbol.is_empty() || anchor.needle.is_empty() || anchor.occurrence == 0 {
        return Err("Map review anchor has an empty symbol/needle or zero occurrence");
    }
    if !OWNERS.iter().any(|owner| owner.id == anchor.owner) {
        return Err("Map review anchor is detached from the commit-bound owner table");
    }
    let source = owners::source_content(anchor.owner);
    let span = symbol_span(source, anchor.symbol)
        .ok_or("Map review anchor symbol is absent from its owner snapshot")?;
    if span.matches(anchor.needle).count() < usize::from(anchor.occurrence) {
        return Err("Map review anchor occurrence is absent from its symbol span");
    }
    Ok(())
}
