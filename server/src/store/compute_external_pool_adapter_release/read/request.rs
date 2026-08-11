use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_release::{
    canonical_external_pool_adapter_release_request_json_and_digest,
    canonical_external_pool_adapter_release_request_material_digest,
    validate_external_pool_adapter_release_request_envelope,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA,
};

use super::super::{
    canonical::{canonical_capabilities_json_and_digest, canonical_json},
    types::StoredRequest,
};
use super::decode;

pub(super) fn request_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredRequest>> {
    request_on(conn, "WHERE request_id=?1", params![id])
}

pub(super) fn request_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRequest>> {
    request_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn request_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRequest>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT request_json, supported_provider_kinds_json, capabilities_json,
                        capability_set_digest, status, reviewed_by_admin_user_id, reviewed_at,
                        applied_by_admin_user_id, applied_at, idempotency_scope, idempotency_key,
                        created_at, updated_at
                   FROM compute_external_pool_adapter_release_requests {filter}"
            ),
            values,
            |row| {
                let request_json: String = row.get(0)?;
                Ok(StoredRequest {
                    envelope: decode(&request_json, 0)?,
                    request_json,
                    supported_provider_kinds_json: row.get(1)?,
                    capabilities_json: row.get(2)?,
                    capability_set_digest: row.get(3)?,
                    status: row.get(4)?,
                    reviewed_by_admin_user_id: row.get(5)?,
                    reviewed_at: row.get(6)?,
                    applied_by_admin_user_id: row.get(7)?,
                    applied_at: row.get(8)?,
                    idempotency_scope: row.get(9)?,
                    idempotency_key: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .optional()?;
    stored.map(|row| audit_request(conn, row)).transpose()
}

fn audit_request(conn: &Connection, stored: StoredRequest) -> Result<StoredRequest> {
    validate_external_pool_adapter_release_request_envelope(&stored.envelope)?;
    let (json, digest) =
        canonical_external_pool_adapter_release_request_json_and_digest(&stored.envelope)?;
    let material_digest =
        canonical_external_pool_adapter_release_request_material_digest(&stored.envelope.request)?;
    let release = &stored.envelope.request.release;
    let verifier = &release.expected_credential_verifier;
    let kinds_json = canonical_json(&release.supported_provider_kinds)?;
    let (capabilities_json, capability_digest) =
        canonical_capabilities_json_and_digest(&release.supported_capabilities)?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_release_requests
              WHERE request_id=?1 AND request_schema=?2 AND request_digest=?3
                AND request_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND request_material_digest=?7 AND adapter_id=?8 AND release_version=?9
                AND route_kind=?10 AND supported_provider_kinds_json=?11
                AND candidate_artifact_ref=?12 AND declared_implementation_sha256=?13
                AND capabilities_json=?14 AND capability_set_digest=?15
                AND verifier_verification_kind=?16 AND verifier_id=?17
                AND verifier_revision=?18 AND verifier_digest=?19
                AND submit_confirmation=?20 AND submit_note=?21
                AND submitted_by_admin_user_id=?22 AND submitted_at=?23 AND status=?24
                AND reviewed_by_admin_user_id IS ?25 AND reviewed_at IS ?26
                AND applied_by_admin_user_id IS ?27 AND applied_at IS ?28
                AND idempotency_scope=?29 AND idempotency_key=?30
                AND created_at=?31 AND updated_at=?32",
            params![
                stored.envelope.request_id,
                stored.envelope.schema,
                stored.envelope.request_digest,
                stored.request_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                stored.envelope.request_material_digest,
                release.adapter_id,
                release.release_version,
                release.route_kind,
                kinds_json,
                release.candidate_artifact_ref,
                release.declared_implementation_sha256,
                capabilities_json,
                release.capability_set_digest,
                verifier.verification_kind,
                verifier.verifier_id,
                verifier.verifier_revision,
                verifier.verifier_digest,
                stored.envelope.request.confirmation,
                stored.envelope.request.submission_note,
                stored.envelope.request.submitted_by_admin_user_id,
                stored.envelope.request.submitted_at,
                stored.status,
                stored.reviewed_by_admin_user_id,
                stored.reviewed_at,
                stored.applied_by_admin_user_id,
                stored.applied_at,
                stored.idempotency_scope,
                stored.idempotency_key,
                stored.created_at,
                stored.updated_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA
        || json != stored.request_json
        || digest != stored.envelope.request_digest
        || material_digest != stored.envelope.request_material_digest
        || kinds_json != stored.supported_provider_kinds_json
        || capabilities_json != stored.capabilities_json
        || capability_digest != stored.capability_set_digest
        || capability_digest != release.capability_set_digest
        || stored.idempotency_key != stored.envelope.request.idempotency_key
        || !stored.state_is_exact()
        || !projected
    {
        bail!("external-pool Adapter release request failed exact readback audit");
    }
    Ok(stored)
}
