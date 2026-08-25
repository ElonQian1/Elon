mod abi;
mod adapter_projection_fragment;
mod adapter_projection_source_shapes;
mod adapter_projection_witnesses;
mod graph;
mod raw;
mod raw_fragment;
mod route_callback_fragment;
mod route_callback_source_shapes;
mod route_callback_witnesses;
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
    route_callback_fragment::validate(steps)?;
    adapter_projection_fragment::validate(steps)?;
    graph::validate(steps)
}
