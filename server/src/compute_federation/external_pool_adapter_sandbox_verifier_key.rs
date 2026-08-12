//! Independent V237 trust roots for future Adapter sandbox-conformance evidence.
//!
//! Registration and activation identify who may sign a later sandbox report. They do not
//! execute an artifact or grant conformance, Adapter, route, credential, or settlement authority.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
