use anyhow::{bail, Result};
use ring::constant_time;

use crate::node_compute_sharing::endpoint_authority::PresentedNodeEndpointCredentialSecret;

pub(super) fn verify_presented_secret(
    stored_secret_hash: Option<&str>,
    presented: &PresentedNodeEndpointCredentialSecret,
) -> Result<()> {
    let mut stored = [0_u8; 32];
    let valid_encoding = stored_secret_hash
        .map(|value| hex::decode_to_slice(value, &mut stored).is_ok())
        .unwrap_or(false);
    let equal = constant_time::verify_slices_are_equal(&stored, presented.secret_hash()).is_ok();
    if !valid_encoding || !equal {
        bail!("NODE_ENDPOINT_CREDENTIAL_AUTHENTICATION_FAILED");
    }
    Ok(())
}

pub(super) fn ensure_secret_hash_exact(stored: &str, expected: &str) -> Result<()> {
    let mut stored_bytes = [0_u8; 32];
    let mut expected_bytes = [0_u8; 32];
    let stored_valid = hex::decode_to_slice(stored, &mut stored_bytes).is_ok();
    let expected_valid = hex::decode_to_slice(expected, &mut expected_bytes).is_ok();
    let equal = constant_time::verify_slices_are_equal(&stored_bytes, &expected_bytes).is_ok();
    if !stored_valid || !expected_valid || !equal {
        bail!("NODE_ENDPOINT_CREDENTIAL_SECRET_READBACK_MISMATCH");
    }
    Ok(())
}
