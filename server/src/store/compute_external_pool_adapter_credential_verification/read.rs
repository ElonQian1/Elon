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
use sha2::Sha256;

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
    CurrentExternalPoolAdapterCredentialVerificationAuthority,
    ExternalPoolAdapterCredentialVerificationCurrentness,
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
    let signature = STANDARD.decode(&item.signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(verifier.public_key_pem())
        .context("decode historical credential verifier public key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
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
                AND application_digest=?6 AND provider_id=?7 AND provider_digest=?8
                AND credential_locator_commitment=?9 AND admission_id=?10
                AND admission_digest=?11 AND credential_verifier_key_record_id=?12
                AND credential_verifier_key_record_digest=?13
                AND credential_verifier_key_id=?14 AND verifier_report_id=?15
                AND provider_response_evidence_digest=?16
                AND signature_message_digest=?17 AND signature_base64=?18
                AND signature_digest=?19 AND recorded_by_admin_user_id=?20
                AND confirmation=?21 AND idempotency_scope=?22 AND idempotency_key=?23
                AND verified_at=?24 AND recorded_at=?25 AND evidence_scope=?26
                AND credential_effect=?27 AND adapter_effect=?28 AND route_effect=?29
                AND execution_effect=?30 AND settlement_effect=?31",
            params![
                receipt.credential_verification_receipt_id,
                receipt.credential_verification_receipt_digest,
                stored.receipt_json,
                receipt.verification_material_digest,
                binding.application_id,
                binding.application_digest,
                binding.provider_id,
                binding.provider_digest,
                binding.credential_locator_commitment,
                binding.admission_id,
                binding.admission_digest,
                binding.credential_verifier_key_record_id,
                binding.credential_verifier_key_record_digest,
                binding.credential_verifier_key_id,
                binding.verifier_report_id,
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
) -> Result<Option<ExternalPoolAdapterCredentialVerificationCurrentness>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    let statuses: (String, String, String, String, String, String) = conn.query_row(
        "SELECT current_status,onboarding_status,provider_status,admission_status,
                verifier_key_status,report_validity_status
           FROM compute_external_pool_adapter_credential_verification_current
          WHERE credential_verification_receipt_id=?1
            AND credential_verification_receipt_digest=?2",
        params![
            receipt_id,
            stored.receipt.credential_verification_receipt_digest
        ],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    Ok(Some(ExternalPoolAdapterCredentialVerificationCurrentness {
        schema: CREDENTIAL_VERIFICATION_CURRENTNESS_SCHEMA,
        credential_verification: stored.summary(),
        current_status: statuses.0,
        onboarding_status: statuses.1,
        provider_status: statuses.2,
        admission_status: statuses.3,
        verifier_key_status: statuses.4,
        report_validity_status: statuses.5,
    }))
}

pub(in crate::store) fn current_external_pool_adapter_credential_verification_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialVerificationAuthority>> {
    let Some(currentness) = currentness_on(conn, receipt_id)? else {
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
    external_pool_adapter_credential_verification_receipt_authority_on(
        conn,
        receipt_id,
        expected_receipt_digest,
    )
}

pub(in crate::store) fn external_pool_adapter_credential_verification_receipt_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialVerificationAuthority>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.credential_verification_receipt_digest != expected_receipt_digest {
        bail!("credential verification receipt authority is not exact");
    }
    Ok(Some(
        CurrentExternalPoolAdapterCredentialVerificationAuthority::new(stored.receipt),
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_verification_currentness(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ExternalPoolAdapterCredentialVerificationCurrentness>> {
        let connection = self.conn()?;
        currentness_on(&connection, receipt_id)
    }
}
