//! V277 permanent history for one external-pool Provider atomic activation.
//!
//! This module is domain-only. It does not expose an HTTP DTO, a Store facade, a route
//! constructor, or a production worker connection.

mod active_carrier;
mod canonical;
mod projected_binding;
mod types;
mod validation;

pub(crate) use active_carrier::*;
pub(crate) use canonical::*;
pub(crate) use projected_binding::*;
pub(crate) use types::*;
pub(crate) use validation::*;
