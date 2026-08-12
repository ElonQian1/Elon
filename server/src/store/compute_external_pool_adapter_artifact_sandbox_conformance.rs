//! Immutable V239 verifier-signed sandbox conformance evidence for exact V236 artifacts.

mod read;
mod types;
mod write;

pub(crate) use types::{
    CreateExternalPoolAdapterSandboxConformance, ExternalPoolAdapterSandboxConformanceCurrentness,
    ExternalPoolAdapterSandboxConformanceWriteReceipt,
    GetExternalPoolAdapterSandboxConformanceChallenge,
};
