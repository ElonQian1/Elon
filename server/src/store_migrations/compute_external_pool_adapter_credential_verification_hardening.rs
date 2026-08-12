use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(crate) fn migration_v245(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    reject_existing_projection_drift(&transaction)?;
    super::compute_external_pool_adapter_adoption_hardening::reject_existing_projection_drift(
        &transaction,
    )?;
    install_projection_guard(&transaction)?;
    super::compute_external_pool_adapter_adoption_hardening::install_projection_guards(
        &transaction,
    )?;
    transaction.execute_batch(VIEW_SQL)?;
    transaction.commit()?;
    Ok(())
}

fn install_projection_guard(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS credential_verification_json_projection;
         CREATE TRIGGER credential_verification_json_projection
         AFTER INSERT ON compute_external_pool_adapter_credential_verification_receipts
         WHEN {}
         BEGIN SELECT RAISE(ABORT,'credential verification JSON projection mismatch'); END;",
        projection_mismatch_sql("NEW", "NEW")
    ))?;
    Ok(())
}

fn reject_existing_projection_drift(conn: &Connection) -> Result<()> {
    let drifted: bool = conn.query_row(
        &format!(
            "SELECT EXISTS(
               SELECT 1 FROM compute_external_pool_adapter_credential_verification_receipts receipt
                WHERE {}
             )",
            projection_mismatch_sql("receipt", "receipt")
        ),
        [],
        |row| row.get(0),
    )?;
    if drifted {
        anyhow::bail!("credential verification existing JSON projection mismatch");
    }
    Ok(())
}

fn projection_matches(row: &str, json_owner: &str) -> String {
    SIGNED_PROJECTIONS
        .iter()
        .map(|(path, column)| {
            let expected = column
                .map(|column| format!("{row}.{column}"))
                .unwrap_or_else(|| match *path {
                    "$.schema" => "'compute_federation.external_pool_adapter_credential_verification_receipt.v1'".to_string(),
                    "$.canonicalization" => "'rfc8785_jcs'".to_string(),
                    "$.digest_algorithm" => "'sha256'".to_string(),
                    _ => unreachable!("unmapped signed projection"),
                });
            format!("json_extract({json_owner}.receipt_json,'{path}') IS {expected}")
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn projection_mismatch_sql(row: &str, json_owner: &str) -> String {
    format!("NOT ({})", projection_matches(row, json_owner))
}

const SIGNED_PROJECTIONS: &[(&str, Option<&str>)] = &[
    ("$.schema", None),
    (
        "$.credential_verification_receipt_id",
        Some("credential_verification_receipt_id"),
    ),
    (
        "$.credential_verification_receipt_digest",
        Some("credential_verification_receipt_digest"),
    ),
    (
        "$.verification_material_digest",
        Some("verification_material_digest"),
    ),
    ("$.canonicalization", None),
    ("$.digest_algorithm", None),
    (
        "$.verification.binding.application_id",
        Some("application_id"),
    ),
    (
        "$.verification.binding.application_digest",
        Some("application_digest"),
    ),
    ("$.verification.binding.provider_id", Some("provider_id")),
    (
        "$.verification.binding.provider_policy_revision",
        Some("provider_policy_revision"),
    ),
    (
        "$.verification.binding.provider_digest",
        Some("provider_digest"),
    ),
    ("$.verification.binding.adapter_id", Some("adapter_id")),
    (
        "$.verification.binding.adapter_release_version",
        Some("adapter_release_version"),
    ),
    (
        "$.verification.binding.adapter_config_revision",
        Some("adapter_config_revision"),
    ),
    (
        "$.verification.binding.adapter_config_digest",
        Some("adapter_config_digest"),
    ),
    (
        "$.verification.binding.credential_ref_scheme",
        Some("credential_ref_scheme"),
    ),
    (
        "$.verification.binding.credential_locator_commitment",
        Some("credential_locator_commitment"),
    ),
    ("$.verification.binding.admission_id", Some("admission_id")),
    (
        "$.verification.binding.admission_digest",
        Some("admission_digest"),
    ),
    (
        "$.verification.binding.credential_verifier_key_record_id",
        Some("credential_verifier_key_record_id"),
    ),
    (
        "$.verification.binding.credential_verifier_key_record_digest",
        Some("credential_verifier_key_record_digest"),
    ),
    (
        "$.verification.binding.credential_verifier_key_id",
        Some("credential_verifier_key_id"),
    ),
    (
        "$.verification.binding.credential_verifier_record_id",
        Some("credential_verifier_record_id"),
    ),
    (
        "$.verification.binding.credential_verifier_record_digest",
        Some("credential_verifier_record_digest"),
    ),
    (
        "$.verification.binding.verifier_report_id",
        Some("verifier_report_id"),
    ),
    (
        "$.verification.binding.report_expires_at",
        Some("report_expires_at"),
    ),
    (
        "$.verification.binding.provider_response_evidence_digest",
        Some("provider_response_evidence_digest"),
    ),
    (
        "$.verification.signature_message_digest",
        Some("signature_message_digest"),
    ),
    ("$.verification.signature_base64", Some("signature_base64")),
    ("$.verification.signature_digest", Some("signature_digest")),
    (
        "$.verification.recorded_by_admin_user_id",
        Some("recorded_by_admin_user_id"),
    ),
    ("$.verification.confirmation", Some("confirmation")),
    (
        "$.verification.idempotency_scope",
        Some("idempotency_scope"),
    ),
    ("$.verification.idempotency_key", Some("idempotency_key")),
    ("$.verification.verified_at", Some("verified_at")),
    ("$.verification.recorded_at", Some("recorded_at")),
    ("$.verification.evidence_scope", Some("evidence_scope")),
    (
        "$.verification.credential_effect",
        Some("credential_effect"),
    ),
    ("$.verification.adapter_effect", Some("adapter_effect")),
    ("$.verification.route_effect", Some("route_effect")),
    ("$.verification.execution_effect", Some("execution_effect")),
    (
        "$.verification.settlement_effect",
        Some("settlement_effect"),
    ),
];

const VIEW_SQL: &str = r#"
DROP VIEW IF EXISTS compute_external_pool_adapter_credential_verification_current;
CREATE VIEW compute_external_pool_adapter_credential_verification_current AS
SELECT receipt.credential_verification_receipt_id,
       receipt.credential_verification_receipt_digest,
       CASE WHEN app.application_id IS NOT NULL
                  AND provider.provider_id IS NOT NULL
                  AND admission.admission_id IS NOT NULL
                  AND admission.current_status='staged'
                  AND verifier.key_record_id IS NOT NULL
                  AND verifier.current_status='active'
                  AND json_extract(receipt.receipt_json,'$.verification.binding.report_expires_at')=receipt.report_expires_at
                  AND julianday(json_extract(receipt.receipt_json,'$.verification.binding.report_expires_at'))>julianday('now')
            THEN 'verified_current' ELSE 'historical_only' END AS current_status,
       CASE WHEN app.application_id IS NOT NULL THEN 'exact' ELSE 'not_exact' END AS onboarding_status,
       CASE WHEN provider.provider_id IS NOT NULL THEN 'exact_registering' ELSE 'not_current' END AS provider_status,
       COALESCE(admission.current_status,'not_current') AS admission_status,
       COALESCE(verifier.current_status,'not_current') AS verifier_key_status,
       CASE WHEN json_extract(receipt.receipt_json,'$.verification.binding.report_expires_at')=receipt.report_expires_at
                  AND julianday(json_extract(receipt.receipt_json,'$.verification.binding.report_expires_at'))>julianday('now')
            THEN 'current' ELSE 'expired' END AS report_validity_status
  FROM compute_external_pool_adapter_credential_verification_receipts receipt
  LEFT JOIN compute_external_pool_onboarding_applications app
    ON app.application_id=receipt.application_id
   AND app.application_digest=receipt.application_digest
   AND app.provider_id=receipt.provider_id
   AND app.target_provider_digest=receipt.provider_digest
  LEFT JOIN compute_providers provider
    ON provider.provider_id=receipt.provider_id
   AND provider.current_policy_revision=receipt.provider_policy_revision
   AND provider.current_provider_digest=receipt.provider_digest
   AND provider.status='registering'
  LEFT JOIN compute_external_pool_adapter_release_admission_current admission
    ON admission.admission_id=receipt.admission_id
   AND admission.admission_digest=receipt.admission_digest
  LEFT JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
    ON verifier.key_record_id=receipt.credential_verifier_key_record_id
   AND verifier.key_record_digest=receipt.credential_verifier_key_record_digest
   AND verifier.verifier_record_id=receipt.credential_verifier_record_id
   AND verifier.verifier_record_digest=receipt.credential_verifier_record_digest
   AND verifier.key_id=receipt.credential_verifier_key_id;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_repeatable_and_guards_complete_projection() {
        let connection = fixture();
        migration_v245(&connection).unwrap();
        migration_v245(&connection).unwrap();

        let trigger_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                ["credential_verification_json_projection"],
                |row| row.get(0),
            )
            .unwrap();
        for (path, column) in SIGNED_PROJECTIONS {
            assert!(trigger_sql.contains(path), "missing JSON path {path}");
            if let Some(column) = column {
                assert!(trigger_sql.contains(column), "missing scalar {column}");
            }
        }

        let view_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='view' AND name=?1",
                ["compute_external_pool_adapter_credential_verification_current"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(view_sql.contains("json_extract(receipt.receipt_json"));
        assert!(view_sql.contains("=receipt.report_expires_at"));

        let adoption_view_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='view' AND name=?1",
                ["compute_external_pool_adapter_adoption_current"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(adoption_view_sql
            .contains("compute_external_pool_adapter_credential_verification_current"));
        let adoption_rows: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM compute_external_pool_adapter_adoption_current",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(adoption_rows, 0);
    }

    #[test]
    fn migration_rejects_existing_forged_expiry() {
        let connection = fixture();
        connection
            .execute_batch(
                "PRAGMA foreign_keys=OFF;
                 DROP TRIGGER credential_verification_json_projection;
                 DROP TRIGGER credential_verification_exact_onboarding;
                 DROP TRIGGER credential_verification_exact_admission;
                 DROP TRIGGER credential_verification_requires_current_inputs;
                 INSERT INTO compute_external_pool_adapter_credential_verification_receipts
                   (credential_verification_receipt_id,credential_verification_receipt_digest,
                    receipt_json,verification_material_digest,application_id,application_digest,
                    provider_id,provider_policy_revision,provider_digest,adapter_id,
                    adapter_release_version,adapter_config_revision,adapter_config_digest,
                    credential_ref_scheme,credential_locator_commitment,admission_id,
                    admission_digest,credential_verifier_key_record_id,
                    credential_verifier_key_record_digest,credential_verifier_key_id,
                    credential_verifier_record_id,credential_verifier_record_digest,
                    verifier_report_id,report_expires_at,provider_response_evidence_digest,
                    signature_message_digest,signature_base64,signature_digest,
                    recorded_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,
                    verified_at,recorded_at,evidence_scope,credential_effect,adapter_effect,
                    route_effect,execution_effect,settlement_effect)
                 SELECT
                   'receipt-1',lower(hex(randomblob(32))),json_object(
                     'schema','compute_federation.external_pool_adapter_credential_verification_receipt.v1',
                     'credential_verification_receipt_id','receipt-1',
                     'credential_verification_receipt_digest','forged',
                     'verification_material_digest','forged',
                     'canonicalization','rfc8785_jcs','digest_algorithm','sha256',
                     'verification',json_object('binding',json_object(
                       'report_expires_at','2099-01-01T00:05:00.000000000Z'))),
                   lower(hex(randomblob(32))),'application-1',lower(hex(randomblob(32))),
                   'provider-1',1,lower(hex(randomblob(32))),'adapter-1','1.0.0',1,
                   'config-digest','vault_ref',lower(hex(randomblob(32))),'admission-1',
                   lower(hex(randomblob(32))),'key-record-1',lower(hex(randomblob(32))),
                   'key-id-1','verifier-record-1',lower(hex(randomblob(32))),'report-1',
                   '2099-01-01T00:10:00.000000000Z',lower(hex(randomblob(32))),
                   lower(hex(randomblob(32))),'AA==',lower(hex(randomblob(32))),'admin-1',
                   'confirm_external_pool_adapter_credential_verification','scope-1','key-1',
                   '2099-01-01T00:00:00.000000000Z','2099-01-01T00:00:00.000000000Z',
                   'verifier_signature_over_exact_v221_non_bearer_locator_commitment_v222_admission_and_asserted_authentication',
                   'signed_credential_verification_current','none','none','none','none';
                 PRAGMA foreign_keys=ON;",
            )
            .unwrap();

        let error = migration_v245(&connection).unwrap_err();
        assert!(error
            .to_string()
            .contains("existing JSON projection mismatch"));
    }

    #[test]
    fn complete_projection_marks_each_signed_scalar_drift() {
        let mismatch = projection_mismatch_sql("receipt", "receipt");
        for (path, column) in SIGNED_PROJECTIONS {
            assert!(mismatch.contains(path), "missing JSON path {path}");
            if let Some(column) = column {
                assert!(mismatch.contains(column), "missing scalar {column}");
            }
        }
        assert!(projection_mismatch_sql("NEW", "NEW")
            .contains("$.verification.binding.report_expires_at"));
    }

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        crate::store_schema::apply_migrations(&connection).unwrap();
        connection
    }
}
