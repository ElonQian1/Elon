//! Read-only ESK/compute contract; no payment, migration or dispatch authority.
#[path = "model.rs"]
mod model;
#[path = "quote.rs"]
mod quote;
pub(crate) use model::*;
pub(crate) use quote::*;
