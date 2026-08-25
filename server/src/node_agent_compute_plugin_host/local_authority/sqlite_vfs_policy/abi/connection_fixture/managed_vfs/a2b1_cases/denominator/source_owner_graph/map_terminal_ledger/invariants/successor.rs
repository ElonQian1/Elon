mod abi;
mod graph;
mod raw;
mod shared;
mod source;

use super::super::model::MapSourceStep;

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    source::validate(steps)?;
    abi::validate()?;
    raw::validate()?;
    graph::validate(steps)
}
