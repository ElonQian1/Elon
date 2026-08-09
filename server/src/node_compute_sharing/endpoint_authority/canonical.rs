use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{bounded_identifier, is_sha256};

const MAX_CANONICAL_BYTES: usize = 256 * 1024;
const INSTALLATION_BINDING_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_INSTALLATION_BINDING_V1";
const SECRET_VERIFIER_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_SECRET_VERIFIER_V1";

pub(super) fn canonical_domain_json_and_digest<T: Serialize + ?Sized>(
    domain: &[u8],
    value: &T,
) -> Result<(String, String)> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_CANONICAL_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok((json, hex::encode(digest.finalize())))
}

pub(super) fn deterministic_identifier<T: Serialize + ?Sized>(
    prefix: &str,
    domain: &[u8],
    value: &T,
) -> Result<String> {
    let (_, digest) = canonical_domain_json_and_digest(domain, value)?;
    Ok(format!("{prefix}{digest}"))
}

pub(super) fn installation_binding_digest(
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
) -> Result<String> {
    if !bounded_identifier(agent_id, 160)
        || !bounded_identifier(owner_user_id, 160)
        || !bounded_identifier(install_id, 512)
    {
        bail!("NODE_ENDPOINT_INSTALLATION_BINDING_INVALID");
    }
    #[derive(Serialize)]
    struct Binding<'a> {
        agent_id: &'a str,
        owner_user_id: &'a str,
        install_id: &'a str,
    }
    let (_, digest) = canonical_domain_json_and_digest(
        INSTALLATION_BINDING_DOMAIN,
        &Binding {
            agent_id,
            owner_user_id,
            install_id,
        },
    )?;
    Ok(digest)
}

pub(super) fn secret_verifier_digest(secret_hash: &[u8; 32]) -> String {
    let mut digest = Sha256::new();
    digest.update(SECRET_VERIFIER_DOMAIN);
    digest.update([0]);
    digest.update(secret_hash);
    hex::encode(digest.finalize())
}

pub(super) fn utc_nanos(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(super) fn parse_utc_nanos(value: &str, code: &'static str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!(code))?
        .with_timezone(&Utc);
    if utc_nanos(parsed) != value {
        bail!(code);
    }
    Ok(parsed)
}

pub(super) fn ensure_time_order(
    earlier: DateTime<Utc>,
    later: DateTime<Utc>,
    code: &'static str,
) -> Result<()> {
    if earlier > later {
        bail!(code);
    }
    Ok(())
}

pub(super) fn ensure_canonical_readback<T: Serialize + ?Sized>(
    domain: &[u8],
    value: &T,
    stored_json: &str,
    stored_digest: &str,
) -> Result<()> {
    if !is_sha256(stored_digest) {
        bail!("NODE_ENDPOINT_STORED_DIGEST_INVALID");
    }
    let (expected_json, expected_digest) = canonical_domain_json_and_digest(domain, value)?;
    if expected_json != stored_json || expected_digest != stored_digest {
        bail!("NODE_ENDPOINT_CANONICAL_READBACK_MISMATCH");
    }
    Ok(())
}
