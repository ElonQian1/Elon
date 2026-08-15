use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::ExternalPoolAdapterTaskProductionSessionRoots;

const ROOTS_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.production.session.roots.v1\0";
const KDF_SALT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.production.session.kdf_salt.v1\0";

pub(crate) const TASK_PRODUCTION_ROOT_ARGUMENT_PREFIXES: [&str; 8] = [
    "--elon-task-production-policy=",
    "--elon-task-production-runtime-profile=",
    "--elon-task-production-protocol-profile=",
    "--elon-task-production-target=",
    "--elon-task-production-companion=",
    "--elon-task-production-launch-image=",
    "--elon-task-production-secret-delivery=",
    "--elon-task-production-conformance-receipt=",
];

pub(crate) fn task_production_session_roots_digest(
    roots: &ExternalPoolAdapterTaskProductionSessionRoots,
) -> Result<String> {
    Ok(hex::encode(session_digest(ROOTS_DOMAIN, roots, None)?))
}

pub(crate) fn task_production_session_kdf_salt(
    roots: &ExternalPoolAdapterTaskProductionSessionRoots,
    host_nonce: &[u8; 32],
    child_nonce: &[u8; 32],
) -> Result<[u8; 32]> {
    session_digest(KDF_SALT_DOMAIN, roots, Some((host_nonce, child_nonce)))
}

pub(crate) fn task_production_session_root_arguments(
    roots: &ExternalPoolAdapterTaskProductionSessionRoots,
) -> Result<[String; 8]> {
    let values = ordered_root_text(roots);
    for value in values {
        decode_digest(value)?;
    }
    Ok(std::array::from_fn(|index| {
        format!(
            "{}{}",
            TASK_PRODUCTION_ROOT_ARGUMENT_PREFIXES[index], values[index]
        )
    }))
}

fn session_digest(
    domain: &[u8],
    roots: &ExternalPoolAdapterTaskProductionSessionRoots,
    nonces: Option<(&[u8; 32], &[u8; 32])>,
) -> Result<[u8; 32]> {
    let mut digest = Sha256::new();
    digest.update(domain);
    for value in ordered_root_text(roots) {
        digest.update(decode_digest(value)?);
    }
    if let Some((host_nonce, child_nonce)) = nonces {
        digest.update(host_nonce);
        digest.update(child_nonce);
    }
    Ok(digest.finalize().into())
}

fn ordered_root_text(roots: &ExternalPoolAdapterTaskProductionSessionRoots) -> [&str; 8] {
    [
        &roots.supervisor_session_policy_digest,
        &roots.runtime_launch_profile_digest,
        &roots.task_protocol_profile_digest,
        &roots.upstream_transport_target_digest,
        &roots.supervisor_session_policy_companion_digest,
        &roots.launch_image_sha256,
        &roots.ephemeral_task_secret_delivery_root,
        &roots.task_protocol_conformance_run_receipt_digest,
    ]
}

fn decode_digest(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("task production session root is not lowercase SHA-256")
    }
    let mut output = [0_u8; 32];
    hex::decode_to_slice(value, &mut output)?;
    if output.iter().all(|byte| *byte == 0) {
        bail!("task production session root must not be all zero")
    }
    Ok(output)
}
