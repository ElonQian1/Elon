//! Signed provenance for one exact quarantined external-pool Adapter Artifact.
//!
//! A verified receipt proves only an RSA signature over immutable release/source/key bindings.
//! It does not validate the Artifact format or create Adapter, credential, route, or execution
//! authority.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
