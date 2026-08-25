mod abi;
mod cleanup;
mod initialization;
mod mapping;
mod raw;
mod route;
mod route_callback;
mod success;
mod validation;

use super::model::{MapSourceStep, MapSuccessFamilyRecord};

pub(super) const STEP_TABLES: &[&[MapSourceStep]] = &[
    abi::STEPS,
    raw::STEPS,
    route::STEPS,
    route_callback::STEPS,
    validation::STEPS,
    initialization::STEPS,
    initialization::FINALIZATION_STEPS,
    mapping::STEPS,
    cleanup::STEPS,
];

pub(super) const SUCCESS_FAMILY_CANDIDATES: &[MapSuccessFamilyRecord] = success::CANDIDATES;
