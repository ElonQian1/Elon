use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use sha2::Sha256;

use crate::{
    compute_federation::external_pool_adapter_artifact_sandbox_conformance::{
        canonical_sandbox_conformance_receipt_json_and_digest, sandbox_capability_test_plan,
        sandbox_conformance_challenge, sandbox_observation_inventory_digest,
        sandbox_test_plan_digest, validate_sandbox_conformance_receipt,
        SANDBOX_CONFORMANCE_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_artifact_vulnerability_report::historical_vulnerability_report_authority_on,
        compute_external_pool_adapter_release::admission_by_id_on,
        compute_external_pool_adapter_sandbox_verifier_key::sandbox_verifier_key_record_authority_on,
        Store,
    },
};

use super::types::{
    ExternalPoolAdapterSandboxConformanceCurrentness, StoredExternalPoolAdapterSandboxConformance,
};

pub(super) fn receipt_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredExternalPoolAdapterSandboxConformance>> {
    receipt_on(conn, "admission_id=?1", params![admission_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterSandboxConformance>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterSandboxConformance>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json FROM compute_external_pool_adapter_sandbox_conformance_reports WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredExternalPoolAdapterSandboxConformance {
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
    stored: StoredExternalPoolAdapterSandboxConformance,
) -> Result<StoredExternalPoolAdapterSandboxConformance> {
    validate_sandbox_conformance_receipt(&stored.receipt)?;
    let (json, digest) = canonical_sandbox_conformance_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.conformance;
    let binding = &item.binding;
    let admission = admission_by_id_on(conn, &binding.admission_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox conformance lost its V222 admission root"))?;
    let vulnerability = historical_vulnerability_report_authority_on(
        conn,
        &binding.admission_id,
        &binding.vulnerability_report_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("sandbox conformance lost its V236 report root"))?;
    let verifier = sandbox_verifier_key_record_authority_on(
        conn,
        &binding.sandbox_verifier_key_record_id,
        &binding.sandbox_verifier_key_record_digest,
        &binding.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("sandbox conformance lost its V237 verifier root"))?;
    let vulnerability_receipt = vulnerability.receipt();
    let vulnerability_binding = &vulnerability_receipt.report.binding;
    let expected_plan = sandbox_capability_test_plan(
        &admission.admission_digest,
        &admission.supported_capabilities,
    )?;
    let challenge = sandbox_conformance_challenge(binding.clone())?;
    let message = STANDARD.decode(challenge.signature_message_base64)?;
    let signature = STANDARD.decode(&item.signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(verifier.public_key_pem())
        .context("decode historical sandbox verifier key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("historical sandbox conformance signature failed verification")?;
    if json != stored.receipt_json
        || digest != stored.receipt.sandbox_conformance_receipt_digest
        || admission.admission_digest != binding.admission_digest
        || admission.adapter_id != binding.adapter_id
        || admission.release_version != binding.release_version
        || admission.declared_implementation_sha256 != binding.declared_implementation_sha256
        || admission.supported_capabilities != binding.supported_capabilities
        || admission.capability_set_digest != binding.capability_set_digest
        || admission.expected_credential_verifier != binding.expected_credential_verifier
        || vulnerability_receipt.vulnerability_report_receipt_id
            != binding.vulnerability_report_receipt_id
        || vulnerability_binding.security_receipt_digest != binding.security_receipt_digest
        || vulnerability_binding.package_receipt_digest != binding.package_receipt_digest
        || vulnerability_binding.archive_sha256 != binding.archive_sha256
        || vulnerability_binding.sbom_digest != binding.sbom_digest
        || vulnerability_binding.intelligence.expires_at
            != binding.vulnerability_intelligence_expires_at
        || vulnerability_receipt.report.verified_at != binding.vulnerability_report_verified_at
        || verifier.key_record_id() != binding.sandbox_verifier_key_record_id
        || verifier.key_record_digest() != binding.sandbox_verifier_key_record_digest
        || verifier.key_id() != binding.sandbox_verifier_key_id
        || verifier.verifier_operator() != binding.sandbox_verifier_operator
        || verifier.verifier_product() != binding.sandbox_verifier_product
        || expected_plan != binding.test_plan
        || sandbox_test_plan_digest(&expected_plan)? != binding.test_plan_digest
        || sandbox_observation_inventory_digest(&binding.observations)?
            != binding.observation_inventory_digest
        || challenge.signature_message_digest != item.signature_message_digest
        || !exact_projection(conn, &stored)?
    {
        bail!("sandbox conformance failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterSandboxConformance,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.conformance;
    let binding = &item.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_sandbox_conformance_reports
              WHERE sandbox_conformance_receipt_id=?1 AND sandbox_conformance_receipt_digest=?2
                AND receipt_json=?3 AND conformance_material_digest=?4 AND admission_id=?5
                AND admission_digest=?6 AND vulnerability_report_receipt_digest=?7
                AND sandbox_verifier_key_record_digest=?8 AND verifier_report_id=?9
                AND runtime_image_digest=?10 AND test_plan_digest=?11
                AND observation_inventory_digest=?12 AND capability_count=?13
                AND passed_capability_count=?14 AND policy_violation_count=?15
                AND signature_message_digest=?16 AND signature_base64=?17
                AND signature_digest=?18 AND verified_by_admin_user_id=?19
                AND confirmation=?20 AND idempotency_scope=?21 AND idempotency_key=?22
                AND verified_at=?23 AND recorded_at=?24 AND evidence_scope=?25
                AND conformance_effect=?26 AND credential_effect=?27
                AND adapter_effect=?28 AND route_effect=?29",
            params![
                receipt.sandbox_conformance_receipt_id,
                receipt.sandbox_conformance_receipt_digest,
                stored.receipt_json,
                receipt.conformance_material_digest,
                binding.admission_id,
                binding.admission_digest,
                binding.vulnerability_report_receipt_digest,
                binding.sandbox_verifier_key_record_digest,
                binding.verifier_report_id,
                binding.runtime_image_digest,
                binding.test_plan_digest,
                binding.observation_inventory_digest,
                binding.supported_capabilities.len() as i64,
                binding.passed_capability_count as i64,
                binding.policy_violation_count as i64,
                item.signature_message_digest,
                item.signature_base64,
                item.signature_digest,
                item.verified_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.verified_at,
                item.recorded_at,
                item.evidence_scope,
                item.conformance_effect,
                item.credential_effect,
                item.adapter_effect,
                item.route_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn currentness_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<ExternalPoolAdapterSandboxConformanceCurrentness>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    let statuses: (String, String, String, String) = conn.query_row(
        "SELECT current_status,vulnerability_report_status,sandbox_verifier_key_status,report_validity_status
           FROM compute_external_pool_adapter_sandbox_conformance_current
          WHERE admission_id=?1 AND sandbox_conformance_receipt_digest=?2",
        params![admission_id, stored.receipt.sandbox_conformance_receipt_digest],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    Ok(Some(ExternalPoolAdapterSandboxConformanceCurrentness {
        schema: SANDBOX_CONFORMANCE_CURRENTNESS_SCHEMA,
        sandbox_conformance: stored.summary(),
        current_status: statuses.0,
        vulnerability_report_status: statuses.1,
        sandbox_verifier_key_status: statuses.2,
        report_validity_status: statuses.3,
    }))
}

impl Store {
    pub(crate) fn external_pool_adapter_sandbox_conformance_currentness(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterSandboxConformanceCurrentness>> {
        let connection = self.conn()?;
        currentness_on(&connection, admission_id)
    }
}
