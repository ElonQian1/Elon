use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeExternalPoolAdapterReleaseAdmissionBinding,
    ComputeExternalPoolAdapterReleaseAdmissionTerminal,
    ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
    ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding,
};

const MAX_TERMINAL_RECEIPT_JSON_BYTES: usize = 512 * 1024;
const TERMINAL_RECEIPT_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-ADMISSION-TERMINAL-RECEIPT-V1";
const TERMINAL_REQUEST_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-ADMISSION-TERMINAL-REQUEST-V1";

/// Returns full JCS JSON and the domain-separated digest with the receipt digest blanked.
pub(crate) fn canonical_external_pool_adapter_release_admission_terminal_json_and_digest(
    receipt: &ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let object = value.as_object().ok_or_else(|| {
        anyhow::anyhow!("external-pool Adapter admission terminal receipt is not an object")
    })?;
    let mut projection = object.clone();
    if projection
        .insert(
            "terminal_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("external-pool Adapter admission terminal receipt lacks its digest field");
    }
    let digest = domain_digest(TERMINAL_RECEIPT_DIGEST_DOMAIN, &projection)?;
    let json = canonical_json(receipt)?;
    if !receipt.terminal_receipt_digest.is_empty() && receipt.terminal_receipt_digest != digest {
        bail!("external-pool Adapter admission terminal receipt digest mismatch");
    }
    Ok((json, digest))
}

/// Stable replay material. Server timestamps and fixed effects do not change request identity.
pub(crate) fn canonical_external_pool_adapter_release_admission_terminal_request_digest(
    terminal: &ComputeExternalPoolAdapterReleaseAdmissionTerminal,
) -> Result<String> {
    #[derive(Serialize)]
    struct RequestProjection<'a> {
        admission: &'a ComputeExternalPoolAdapterReleaseAdmissionBinding,
        terminal_status: &'a str,
        successor_admission: &'a Option<ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding>,
        actor_kind: &'a str,
        actor_id: &'a str,
        reason: &'a str,
        confirmation: &'a str,
        idempotency_scope: &'a str,
        idempotency_key: &'a str,
    }

    domain_digest(
        TERMINAL_REQUEST_DIGEST_DOMAIN,
        &RequestProjection {
            admission: &terminal.admission,
            terminal_status: &terminal.terminal_status,
            successor_admission: &terminal.successor_admission,
            actor_kind: &terminal.actor_kind,
            actor_id: &terminal.actor_id,
            reason: &terminal.reason,
            confirmation: &terminal.confirmation,
            idempotency_scope: &terminal.idempotency_scope,
            idempotency_key: &terminal.idempotency_key,
        },
    )
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_TERMINAL_RECEIPT_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
