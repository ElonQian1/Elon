use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_credential_verification::{
        canonical_credential_verification_receipt_json_and_digest, credential_locator_commitment,
        credential_ref_scheme, credential_verification_challenge,
        validate_credential_verification_receipt, CREDENTIAL_VERIFICATION_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_credential_verifier_key::credential_verifier_key_record_authority_on,
        compute_external_pool_adapter_release::admission_by_id_on,
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
        Store,
    },
};

use super::types::{
    report_is_current_at, validate_checked_at,
    CurrentExternalPoolAdapterCredentialVerificationAuthority,
    ExternalPoolAdapterCredentialVerificationCurrentness,
    HistoricalExternalPoolAdapterCredentialVerificationAuthority,
    StoredExternalPoolAdapterCredentialVerification,
};

pub(super) fn receipt_by_report_on(
    conn: &Connection,
    verifier_report_id: &str,
) -> Result<Option<StoredExternalPoolAdapterCredentialVerification>> {
    receipt_on(conn, "verifier_report_id=?1", params![verifier_report_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterCredentialVerification>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn receipt_by_id_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterCredentialVerification>> {
    receipt_on(
        conn,
        "credential_verification_receipt_id=?1",
        params![receipt_id],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterCredentialVerification>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json FROM compute_external_pool_adapter_credential_verification_receipts WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredExternalPoolAdapterCredentialVerification {
                receipt,
                receipt_json,
            })
        },
    )
    .optional()?
    .map(|stored| audit_receipt(conn, stored))
    .transpose()
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredExternalPoolAdapterCredentialVerification,
) -> Result<StoredExternalPoolAdapterCredentialVerification> {
    validate_credential_verification_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_credential_verification_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.verification;
    let binding = &item.binding;
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &binding.application_id,
        &binding.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential verification lost onboarding root"))?;
    let admission = admission_by_id_on(conn, &binding.admission_id)?
        .ok_or_else(|| anyhow::anyhow!("credential verification lost admission root"))?;
    let verifier = credential_verifier_key_record_authority_on(
        conn,
        &binding.credential_verifier_key_record_id,
        &binding.credential_verifier_key_record_digest,
        &binding.credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential verification lost verifier key root"))?;
    let challenge = credential_verification_challenge(binding.clone())?;
    let message = STANDARD.decode(challenge.signature_message_base64)?;
    let signature_bytes = STANDARD.decode(&item.signature_base64)?;
    if STANDARD.encode(&signature_bytes) != item.signature_base64
        || hex::encode(Sha256::digest(&signature_bytes)) != item.signature_digest
    {
        bail!("credential verification signature encoding or digest is not canonical");
    }
    let public = RsaPublicKey::from_public_key_pem(verifier.public_key_pem())
        .context("decode historical credential verifier public key")?;
    let signature = RsaSignature::try_from(signature_bytes.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("historical credential verification signature failed")?;
    let provider = onboarding.provider();
    let adapter = provider
        .adapter
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("credential verification Provider lost Adapter root"))?;
    let locator = onboarding.non_bearer_credential_ref();
    if json != stored.receipt_json
        || digest != stored.receipt.credential_verification_receipt_digest
        || onboarding.provider_digest() != binding.provider_digest
        || provider.provider_id != binding.provider_id
        || provider.provider_kind != binding.provider_kind
        || provider.owner_account_id != binding.provider_owner_account_id
        || provider.settlement_account_id.as_deref() != Some(&binding.settlement_account_id)
        || provider.policy_revision != binding.provider_policy_revision
        || provider.status != binding.provider_status
        || adapter.adapter_id != binding.adapter_id
        || adapter.adapter_version != binding.adapter_release_version
        || adapter.config_revision != binding.adapter_config_revision
        || adapter.config_digest != binding.adapter_config_digest
        || credential_ref_scheme(locator)? != binding.credential_ref_scheme
        || credential_locator_commitment(locator) != binding.credential_locator_commitment
        || admission.admission_digest != binding.admission_digest
        || admission.adapter_id != binding.adapter_id
        || admission.release_version != binding.adapter_release_version
        || admission.declared_implementation_sha256 != binding.declared_implementation_sha256
        || admission.capability_set_digest != binding.capability_set_digest
        || admission.expected_credential_verifier != binding.expected_credential_verifier
        || verifier.verifier_record_id() != binding.credential_verifier_record_id
        || verifier.verifier_record_digest() != binding.credential_verifier_record_digest
        || verifier.verification_kind() != binding.expected_credential_verifier.verification_kind
        || verifier.verifier_id() != binding.expected_credential_verifier.verifier_id
        || verifier.verifier_revision() != binding.expected_credential_verifier.verifier_revision
        || verifier.verifier_digest() != binding.expected_credential_verifier.verifier_digest
        || challenge.signature_message_digest != item.signature_message_digest
        || !exact_projection(conn, &stored)?
    {
        bail!("credential verification failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterCredentialVerification,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.verification;
    let binding = &item.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verification_receipts
              WHERE credential_verification_receipt_id=?1
                AND credential_verification_receipt_digest=?2 AND receipt_json=?3
                AND verification_material_digest=?4 AND application_id=?5
                AND application_digest=?6 AND provider_id=?7
                AND provider_policy_revision=?8 AND provider_digest=?9
                AND adapter_id=?10 AND adapter_release_version=?11
                AND adapter_config_revision=?12 AND adapter_config_digest=?13
                AND credential_ref_scheme=?14 AND credential_locator_commitment=?15
                AND admission_id=?16 AND admission_digest=?17
                AND credential_verifier_key_record_id=?18
                AND credential_verifier_key_record_digest=?19
                AND credential_verifier_key_id=?20
                AND credential_verifier_record_id=?21
                AND credential_verifier_record_digest=?22 AND verifier_report_id=?23
                AND report_expires_at=?24 AND provider_response_evidence_digest=?25
                AND signature_message_digest=?26 AND signature_base64=?27
                AND signature_digest=?28 AND recorded_by_admin_user_id=?29
                AND confirmation=?30 AND idempotency_scope=?31 AND idempotency_key=?32
                AND verified_at=?33 AND recorded_at=?34 AND evidence_scope=?35
                AND credential_effect=?36 AND adapter_effect=?37 AND route_effect=?38
                AND execution_effect=?39 AND settlement_effect=?40",
            params![
                receipt.credential_verification_receipt_id,
                receipt.credential_verification_receipt_digest,
                stored.receipt_json,
                receipt.verification_material_digest,
                binding.application_id,
                binding.application_digest,
                binding.provider_id,
                binding.provider_policy_revision,
                binding.provider_digest,
                binding.adapter_id,
                binding.adapter_release_version,
                binding.adapter_config_revision,
                binding.adapter_config_digest,
                binding.credential_ref_scheme,
                binding.credential_locator_commitment,
                binding.admission_id,
                binding.admission_digest,
                binding.credential_verifier_key_record_id,
                binding.credential_verifier_key_record_digest,
                binding.credential_verifier_key_id,
                binding.credential_verifier_record_id,
                binding.credential_verifier_record_digest,
                binding.verifier_report_id,
                binding.report_expires_at,
                binding.provider_response_evidence_digest,
                item.signature_message_digest,
                item.signature_base64,
                item.signature_digest,
                item.recorded_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.verified_at,
                item.recorded_at,
                item.evidence_scope,
                item.credential_effect,
                item.adapter_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn currentness_on(
    conn: &Connection,
    receipt_id: &str,
    checked_at: &str,
) -> Result<Option<ExternalPoolAdapterCredentialVerificationCurrentness>> {
    validate_checked_at(checked_at)?;
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    let statuses: (String, String, String, String) = conn.query_row(
        "SELECT onboarding_status,provider_status,admission_status,verifier_key_status
           FROM compute_external_pool_adapter_credential_verification_current
          WHERE credential_verification_receipt_id=?1
            AND credential_verification_receipt_digest=?2",
        params![
            receipt_id,
            stored.receipt.credential_verification_receipt_digest
        ],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let report_is_current = report_is_current_at(
        &stored.receipt.verification.binding.report_expires_at,
        checked_at,
    )?;
    let upstreams_are_current = statuses.0 == "exact"
        && statuses.1 == "exact_registering"
        && statuses.2 == "staged"
        && statuses.3 == "active";
    Ok(Some(ExternalPoolAdapterCredentialVerificationCurrentness {
        schema: CREDENTIAL_VERIFICATION_CURRENTNESS_SCHEMA,
        credential_verification: stored.summary(),
        current_status: if upstreams_are_current && report_is_current {
            "verified_current"
        } else {
            "historical_only"
        }
        .to_string(),
        onboarding_status: statuses.0,
        provider_status: statuses.1,
        admission_status: statuses.2,
        verifier_key_status: statuses.3,
        report_validity_status: if report_is_current {
            "current"
        } else {
            "expired"
        }
        .to_string(),
    }))
}

pub(in crate::store) fn current_external_pool_adapter_credential_verification_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialVerificationAuthority>> {
    let Some(currentness) = currentness_on(conn, receipt_id, checked_at)? else {
        return Ok(None);
    };
    if currentness.current_status != "verified_current"
        || currentness
            .credential_verification
            .credential_verification_receipt_digest
            != expected_receipt_digest
    {
        bail!("credential verification authority is not current and exact");
    }
    let stored = receipt_by_id_on(conn, receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("credential verification disappeared after currentness"))?;
    let binding = &stored.receipt.verification.binding;
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &binding.application_id,
        &binding.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential verification lost onboarding authority"))?;
    Ok(Some(
        CurrentExternalPoolAdapterCredentialVerificationAuthority::new(
            stored.receipt,
            onboarding.non_bearer_credential_ref().to_string(),
            checked_at.to_string(),
        )?,
    ))
}

pub(in crate::store) fn external_pool_adapter_credential_verification_receipt_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterCredentialVerificationAuthority>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.credential_verification_receipt_digest != expected_receipt_digest {
        bail!("credential verification receipt authority is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterCredentialVerificationAuthority::new(stored.receipt),
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_verification_currentness(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ExternalPoolAdapterCredentialVerificationCurrentness>> {
        let connection = self.conn()?;
        let checked_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        currentness_on(&connection, receipt_id, &checked_at)
    }
}
