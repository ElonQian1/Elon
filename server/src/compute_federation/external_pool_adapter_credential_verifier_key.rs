//! V242 public-key roots bound to exact active V241 credential-verifier implementations.
//!
//! A key can authenticate future verification reports. It cannot read credentials, prove an
//! endpoint is usable, issue a credential receipt, adopt an Adapter, or authorize execution.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
