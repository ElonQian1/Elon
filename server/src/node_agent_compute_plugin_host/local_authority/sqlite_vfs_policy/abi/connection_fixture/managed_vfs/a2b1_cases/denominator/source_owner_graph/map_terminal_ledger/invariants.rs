mod anchors;
mod effects;
mod graph;
mod partition;
mod traces;

use super::{map, model::MapSourceStep};

pub(super) fn validate() -> Result<(), &'static str> {
    let steps = all_steps();
    anchors::validate(&steps)?;
    partition::validate(&steps)?;
    effects::validate(&steps)?;
    graph::validate(&steps)?;
    traces::validate(&steps)?;
    Ok(())
}

fn all_steps() -> Vec<MapSourceStep> {
    map::STEP_TABLES
        .iter()
        .flat_map(|table| table.iter().copied())
        .collect()
}
