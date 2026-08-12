use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use sha2::Sha256;

use crate::{
    compute_federation::external_pool_adapter_sandbox_reattestation::*,
    store::{
        compute_external_pool_adapter_registry::{
            current_external_pool_adapter_registry_release_authority_on,
            historical_external_pool_adapter_registry_release_authority_on,
        },
        compute_external_pool_adapter_sandbox_verifier_key::{
            current_sandbox_verifier_key_authority_on, sandbox_verifier_key_record_authority_on,
        },
        compute_external_pool_adapter_vulnerability_reattestation::{
            current_external_pool_adapter_vulnerability_reattestation_authority_on,
            historical_external_pool_adapter_vulnerability_reattestation_authority_on,
        },
        Store,
    },
};

use super::{
    challenge_audit::challenge_by_id_on,
    receipt_projection_audit::{exact_receipt_projection, exact_revocation_projection},
    types::*,
};

pub(super) fn receipt_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredSandboxReattestation>> {
    receipt_on(conn, "reattestation_receipt_id=?1", params![id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSandboxReattestation>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn receipt_by_challenge_on(
    conn: &Connection,
    challenge_id: &str,
) -> Result<Option<StoredSandboxReattestation>> {
    receipt_on(conn, "challenge_id=?1", params![challenge_id])
}

pub(super) fn head_by_release_on(
    conn: &Connection,
    release_id: &str,
) -> Result<Option<StoredSandboxReattestation>> {
    receipt_on(
        conn,
        "registry_release_id=?1 AND NOT EXISTS(
           SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts successor
            WHERE successor.predecessor_receipt_id=
                  compute_external_pool_adapter_sandbox_reattestation_receipts.reattestation_receipt_id)",
        params![release_id],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSandboxReattestation>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json FROM compute_external_pool_adapter_sandbox_reattestation_receipts WHERE {filter}"
        ),
        values,
        |row| decode_receipt(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_receipt(conn, stored))
    .transpose()
}

fn decode_receipt(json: String) -> rusqlite::Result<StoredSandboxReattestation> {
    let receipt = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredSandboxReattestation {
        receipt,
        receipt_json: json,
    })
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredSandboxReattestation,
) -> Result<StoredSandboxReattestation> {
    validate_sandbox_reattestation_receipt(&stored.receipt)?;
    let (json, digest) = sandbox_reattestation_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.reattestation;
    let b = &item.binding;
    let release = historical_external_pool_adapter_registry_release_authority_on(
        conn,
        &b.registry_release_id,
        &b.registry_release_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation lost V249 release history"))?;
    let vulnerability = historical_external_pool_adapter_vulnerability_reattestation_authority_on(
        conn,
        &b.vulnerability_reattestation_receipt_id,
        &b.vulnerability_reattestation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation lost V250 history"))?;
    let verifier = sandbox_verifier_key_record_authority_on(
        conn,
        &b.sandbox_verifier_key_record_id,
        &b.sandbox_verifier_key_record_digest,
        &b.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation lost V237 verifier history"))?;
    let challenge = sandbox_reattestation_challenge(b.clone())?;
    let durable_challenge = challenge_by_id_on(conn, &b.challenge_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation lost durable challenge"))?;
    let message = STANDARD.decode(&challenge.signature_message_base64)?;
    let signature = STANDARD.decode(&item.signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(verifier.public_key_pem())
        .context("decode historical V237 sandbox verifier public key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("sandbox re-attestation historical RSA signature audit failed")?;
    audit_roots(release.release(), vulnerability.receipt(), b)?;
    if verifier.verifier_operator() != b.sandbox_verifier_operator
        || verifier.verifier_product() != b.sandbox_verifier_product
    {
        bail!("sandbox re-attestation verifier identity drifted");
    }
    audit_predecessor(conn, b)?;
    if json != stored.receipt_json
        || digest != stored.receipt.reattestation_receipt_digest
        || challenge.signature_message_digest != item.signature_message_digest
        || durable_challenge != challenge
        || !exact_receipt_projection(conn, &stored)?
    {
        bail!("sandbox re-attestation failed exact historical audit");
    }
    Ok(stored)
}

fn audit_roots(
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    vulnerability: &crate::compute_federation::external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationReceipt,
    b: &ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    let r = &release.release;
    let v = &vulnerability.reattestation.binding;
    if release.registry_release_id != b.registry_release_id
        || release.registry_release_digest != b.registry_release_digest
        || release.registry_release_material_digest != b.registry_release_material_digest
        || r.admission_id != b.admission_id
        || r.admission_digest != b.admission_digest
        || r.package_receipt_id != b.package_receipt_id
        || r.package_receipt_digest != b.package_receipt_digest
        || r.source_receipt_id != b.source_receipt_id
        || r.source_receipt_digest != b.source_receipt_digest
        || r.adapter_id != b.adapter_id
        || r.release_version != b.release_version
        || r.route_kind != b.route_kind
        || r.supported_provider_kinds != b.supported_provider_kinds
        || r.implementation_digest != b.implementation_digest
        || r.declared_implementation_sha256 != b.declared_implementation_sha256
        || r.supported_capabilities != b.supported_capabilities
        || r.capability_set_digest != b.capability_set_digest
        || r.credential_verifier != b.expected_credential_verifier
        || r.credential_verifier_digest != b.credential_verifier_digest
        || r.archive_sha256 != b.archive_sha256
        || r.archive_size_bytes != b.archive_size_bytes
        || r.manifest_digest != b.manifest_digest
        || r.entry_inventory_digest != b.entry_inventory_digest
        || r.entry_count != b.entry_count
        || r.total_uncompressed_bytes != b.total_uncompressed_bytes
        || r.installation_content_digest != b.installation_content_digest
        || vulnerability.reattestation_receipt_id != b.vulnerability_reattestation_receipt_id
        || vulnerability.reattestation_receipt_digest
            != b.vulnerability_reattestation_receipt_digest
        || vulnerability.reattestation_material_digest
            != b.vulnerability_reattestation_material_digest
        || v.registry_release_id != b.registry_release_id
        || v.registry_release_digest != b.registry_release_digest
        || v.sequence != b.vulnerability_reattestation_sequence
        || vulnerability.reattestation.verified_at != b.vulnerability_reattestation_verified_at
        || v.intelligence.snapshot_digest != b.vulnerability_intelligence_snapshot_digest
        || v.intelligence.expires_at != b.vulnerability_intelligence_expires_at
        || v.security_receipt_id != b.security_receipt_id
        || v.security_receipt_digest != b.security_receipt_digest
        || v.security_material_digest != b.security_material_digest
        || v.sbom_digest != b.sbom_digest
        || v.component_inventory_digest != b.component_inventory_digest
        || v.component_count != b.component_count
        || v.dependency_inventory_digest != b.dependency_inventory_digest
    {
        bail!("sandbox re-attestation exact roots drifted");
    }
    Ok(())
}

fn audit_predecessor(
    conn: &Connection,
    b: &ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    if let (Some(id), Some(digest)) = (
        b.predecessor_receipt_id.as_deref(),
        b.predecessor_receipt_digest.as_deref(),
    ) {
        let predecessor = receipt_by_id_on(conn, id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation lost predecessor history"))?;
        let predecessor_binding = &predecessor.receipt.reattestation.binding;
        if predecessor.receipt.reattestation_receipt_digest != digest
            || predecessor_binding.registry_release_id != b.registry_release_id
            || predecessor_binding.sequence.checked_add(1) != Some(b.sequence)
        {
            bail!("sandbox re-attestation predecessor lineage is not exact");
        }
    }
    Ok(())
}

pub(in crate::store) fn historical_external_pool_adapter_sandbox_reattestation_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterSandboxReattestationAuthority>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.reattestation_receipt_digest != expected_digest {
        bail!("sandbox re-attestation history is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterSandboxReattestationAuthority::new(stored.receipt),
    ))
}

pub(in crate::store) fn current_external_pool_adapter_sandbox_reattestation_authority_on(
    conn: &Connection,
    release_id: &str,
    expected_receipt_id: &str,
    expected_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterSandboxReattestationAuthority>> {
    let Some(stored) = head_by_release_on(conn, release_id)? else {
        return Ok(None);
    };
    let b = &stored.receipt.reattestation.binding;
    let checked = canonical_checked_at(checked_at)?;
    let verified = DateTime::parse_from_rfc3339(&stored.receipt.reattestation.verified_at)?;
    let expires = DateTime::parse_from_rfc3339(&b.report_expires_at)?;
    if stored.receipt.reattestation_receipt_id != expected_receipt_id
        || stored.receipt.reattestation_receipt_digest != expected_receipt_digest
        || checked < verified
        || checked >= expires
        || revocation_by_receipt_on(conn, expected_receipt_id)?.is_some()
    {
        bail!("sandbox re-attestation is not current and exact");
    }
    let release = current_external_pool_adapter_registry_release_authority_on(
        conn,
        release_id,
        &b.registry_release_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V249 release was not found"))?;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        conn,
        release_id,
        &b.vulnerability_reattestation_receipt_id,
        &b.vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V250 authority was not found"))?;
    if release.checked_at() != checked_at || vulnerability.checked_at() != checked_at {
        bail!("sandbox re-attestation current roots used different checked_at anchors");
    }
    let verifier = current_sandbox_verifier_key_authority_on(
        conn,
        &b.sandbox_verifier_key_record_id,
        &b.sandbox_verifier_key_record_digest,
        &b.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active V237 verifier was not found"))?;
    audit_roots(release.release(), vulnerability.receipt(), b)?;
    if verifier.verifier_operator() != b.sandbox_verifier_operator
        || verifier.verifier_product() != b.sandbox_verifier_product
    {
        bail!("sandbox re-attestation current verifier root drifted");
    }
    Ok(Some(
        CurrentExternalPoolAdapterSandboxReattestationAuthority::new(
            stored.receipt,
            checked_at.to_string(),
        ),
    ))
}

pub(super) fn revocation_by_receipt_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredSandboxReattestationRevocation>> {
    revocation_on(conn, "reattestation_receipt_id=?1", params![receipt_id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSandboxReattestationRevocation>> {
    revocation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn revocation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSandboxReattestationRevocation>> {
    conn.query_row(
        &format!("SELECT receipt_json FROM compute_external_pool_adapter_sandbox_reattestation_revocations WHERE {filter}"),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredSandboxReattestationRevocation { receipt, receipt_json })
        },
    )
    .optional()?
    .map(|stored| audit_revocation(conn, stored))
    .transpose()
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredSandboxReattestationRevocation,
) -> Result<StoredSandboxReattestationRevocation> {
    validate_sandbox_reattestation_revocation_receipt(&stored.receipt)?;
    let (json, digest) = sandbox_reattestation_revocation_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.revocation;
    let target = receipt_by_id_on(conn, &item.reattestation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation revocation lost target"))?;
    let binding = &target.receipt.reattestation.binding;
    if json != stored.receipt_json
        || digest != stored.receipt.revocation_receipt_digest
        || item.reattestation_receipt_digest != target.receipt.reattestation_receipt_digest
        || item.registry_release_id != binding.registry_release_id
        || item.registry_release_digest != binding.registry_release_digest
        || DateTime::parse_from_rfc3339(&item.revoked_at)?
            < DateTime::parse_from_rfc3339(&target.receipt.reattestation.verified_at)?
        || !exact_revocation_projection(conn, &stored)?
    {
        bail!("sandbox re-attestation revocation failed exact historical audit");
    }
    Ok(stored)
}

fn canonical_checked_at(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = canonical_sandbox_reattestation_timestamp(value)?;
    if parsed > Utc::now() + Duration::minutes(5) {
        bail!("sandbox re-attestation checked_at is future-dated");
    }
    Ok(parsed)
}

impl Store {
    pub(crate) fn external_pool_adapter_sandbox_reattestation_challenge_exists(
        &self,
        challenge_id: &str,
        release_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_challenges
              WHERE challenge_id=?1 AND registry_release_id=?2",
                params![challenge_id, release_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub(crate) fn external_pool_adapter_sandbox_reattestation_exists(
        &self,
        receipt_id: &str,
        release_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts
              WHERE reattestation_receipt_id=?1 AND registry_release_id=?2",
                params![receipt_id, release_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}
