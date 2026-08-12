//! Bounded static inspection for one signed external-pool Adapter package.
//!
//! This module never extracts to disk, executes an entrypoint, opens the network, or grants
//! Adapter/route authority. It only proves an exact ZIP/manifest shape against prior authorities.

mod canonical;
mod inspection;
mod types;
mod validation;

pub(crate) use canonical::{
    canonical_artifact_package_receipt_json_and_digest, package_material_digest,
};
pub(crate) use inspection::inspect_external_pool_adapter_artifact_package;
pub(crate) use types::*;
pub(crate) use validation::{
    validate_artifact_package_inspection, validate_artifact_package_receipt, validate_identifier,
};
