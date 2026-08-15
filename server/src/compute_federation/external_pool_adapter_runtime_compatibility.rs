//! Server-owned V266 interoperability contract for third-party Adapter authors.
//!
//! This module only publishes a canonical profile and validates bounded, unsigned candidate
//! reports. It never loads an artifact, launches a process, opens a network, resolves a secret,
//! or grants Adapter, Provider, route, activation, usage, market, or settlement authority.

#![allow(dead_code)]

mod canonical;
mod profile;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use profile::*;
pub(crate) use types::*;
pub(crate) use validation::*;

#[cfg(test)]
mod tests;
