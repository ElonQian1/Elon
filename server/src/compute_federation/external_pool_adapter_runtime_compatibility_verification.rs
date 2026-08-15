//! V268 provider-neutral signed runtime-compatibility verification contract.
//!
//! This domain publishes the V2 profile and canonical durable evidence shapes. Process launch,
//! retained-file custody, challenge consumption, persistence, and HTTP redaction remain owned by
//! the Store/service layers. Nothing here grants Provider, route, activation, execution, usage,
//! market, or settlement authority.

#![allow(dead_code)]

mod canonical;
mod evidence_validation;
mod handoff_types;
mod input_types;
mod policy;
mod profile;
mod release_types;
mod summary_types;
mod types;
mod validation;
mod validation_support;

pub(crate) use canonical::*;
pub(crate) use evidence_validation::*;
pub(crate) use handoff_types::*;
pub(crate) use input_types::*;
pub(crate) use policy::*;
pub(crate) use profile::*;
pub(crate) use release_types::*;
pub(crate) use summary_types::*;
pub(crate) use types::*;
pub(crate) use validation::*;
pub(crate) use validation_support::canonical_runtime_compatibility_timestamp;
