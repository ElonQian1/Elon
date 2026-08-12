//! V233 deterministic SBOM, license declaration, and static safety inspection.
//!
//! This boundary never executes package bytes and does not consult a network vulnerability feed.

mod canonical;
mod inspection;
mod types;
mod validation;

pub(crate) use canonical::{
    canonical_artifact_security_receipt_json_and_digest, canonical_sbom, security_material_digest,
};
pub(crate) use inspection::scan_external_pool_adapter_artifact_security;
pub(crate) use types::*;
pub(crate) use validation::{
    validate_artifact_security_inspection, validate_artifact_security_receipt, validate_sbom,
};
