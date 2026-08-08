//! First-party privileged component supply-chain contracts for the Windows node.
//!
//! This module is intentionally separate from the compute-plugin Publisher/InstallPlan authority:
//! a third-party plugin signature must never authorize kernel code. The current code only defines
//! strict wire shapes and non-authorizing validation. It does not download, install, load, trust or
//! report a driver as usable, and it cannot construct a managed namespace mutation fence.

pub(crate) mod contract;
pub(crate) mod validation;
