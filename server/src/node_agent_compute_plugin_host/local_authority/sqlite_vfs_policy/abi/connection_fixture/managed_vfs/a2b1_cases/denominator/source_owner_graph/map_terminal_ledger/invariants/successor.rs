mod abi;
mod graph;
mod raw;
mod raw_fragment;
mod shared;
mod source;
mod typed_fragment;
mod typed_witnesses;

use super::super::model::MapSourceStep;

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    source::validate(steps)?;
    abi::validate()?;
    raw::validate()?;
    raw_fragment::validate()?;
    typed_fragment::validate(steps)?;
    graph::validate(steps)
}
