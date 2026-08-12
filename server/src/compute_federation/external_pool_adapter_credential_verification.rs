//! Short-lived verifier-signed credential evidence for one exact external-pool onboarding root.
//!
//! The server commits, but never exposes, the non-bearer credential locator. An independently
//! registered V241/V242 verifier signs the exact V221 Provider and V222 admission binding. The
//! resulting receipt is evidence only: it grants no Adapter adoption, route, execution, or
//! settlement authority.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
