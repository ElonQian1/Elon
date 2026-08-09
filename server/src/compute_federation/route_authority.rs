//! Sealed Provider route, Adapter, credential, and service-actor authority facts.
//!
//! Producers and Store gates are intentionally outside this module. The wrappers expose only
//! authenticated or validated custody and have no public construction or deserialization path.

mod canonical;
mod types;
mod validated;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validated::*;
