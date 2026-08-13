use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_credential_reattestation_challenge_lineage
        BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_challenges
        WHEN (NEW.sequence=1 AND NEW.predecessor_receipt_id IS NOT NULL)
          OR (NEW.sequence<>1 AND NEW.predecessor_receipt_id IS NULL)
          OR (NEW.sequence=1 AND EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts existing
             WHERE existing.provider_binding_id=NEW.provider_binding_id))
          OR (NEW.predecessor_receipt_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts predecessor
             WHERE predecessor.reattestation_receipt_id=NEW.predecessor_receipt_id
               AND predecessor.reattestation_receipt_digest=NEW.predecessor_receipt_digest
               AND predecessor.provider_binding_id=NEW.provider_binding_id
               AND predecessor.provider_binding_digest=NEW.provider_binding_digest
               AND predecessor.registry_release_id=NEW.registry_release_id
               AND predecessor.registry_release_digest=NEW.registry_release_digest
               AND predecessor.sequence+1=NEW.sequence
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                                WHERE successor.predecessor_receipt_id=predecessor.reattestation_receipt_id)))
        BEGIN SELECT RAISE(ABORT,'V253 challenge requires exact current predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_credential_reattestation_receipt_lineage
        BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_receipts
        WHEN (NEW.sequence=1 AND NEW.predecessor_receipt_id IS NOT NULL)
          OR (NEW.sequence<>1 AND NEW.predecessor_receipt_id IS NULL)
          OR (NEW.sequence=1 AND EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts existing
             WHERE existing.provider_binding_id=NEW.provider_binding_id))
          OR (NEW.predecessor_receipt_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts predecessor
             WHERE predecessor.reattestation_receipt_id=NEW.predecessor_receipt_id
               AND predecessor.reattestation_receipt_digest=NEW.predecessor_receipt_digest
               AND predecessor.provider_binding_id=NEW.provider_binding_id
               AND predecessor.provider_binding_digest=NEW.provider_binding_digest
               AND predecessor.registry_release_id=NEW.registry_release_id
               AND predecessor.registry_release_digest=NEW.registry_release_digest
               AND predecessor.sequence+1=NEW.sequence
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                                WHERE successor.predecessor_receipt_id=predecessor.reattestation_receipt_id)))
        BEGIN SELECT RAISE(ABORT,'V253 receipt requires exact current predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_credential_reattestation_challenge_time_bounds
        BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_challenges
        WHEN NEW.issued_at>(strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z')
          OR json_extract(NEW.challenge_json,'$.binding.verification_started_at') NOT GLOB '????-??-??T??:??:??.?????????Z'
          OR json_extract(NEW.challenge_json,'$.binding.verification_completed_at') NOT GLOB '????-??-??T??:??:??.?????????Z'
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at') NOT GLOB '????-??-??T??:??:??.?????????Z'
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at') NOT GLOB '????-??-??T??:??:??.?????????Z'
          OR length(json_extract(NEW.challenge_json,'$.binding.verification_started_at'))<>30
          OR length(json_extract(NEW.challenge_json,'$.binding.verification_completed_at'))<>30
          OR length(json_extract(NEW.challenge_json,'$.binding.report_generated_at'))<>30
          OR length(json_extract(NEW.challenge_json,'$.binding.report_expires_at'))<>30
          OR julianday(json_extract(NEW.challenge_json,'$.binding.verification_started_at')) IS NULL
          OR julianday(json_extract(NEW.challenge_json,'$.binding.verification_completed_at')) IS NULL
          OR julianday(json_extract(NEW.challenge_json,'$.binding.report_generated_at')) IS NULL
          OR julianday(json_extract(NEW.challenge_json,'$.binding.report_expires_at')) IS NULL
          OR json_extract(NEW.challenge_json,'$.binding.verification_completed_at')<
               json_extract(NEW.challenge_json,'$.binding.verification_started_at')
          OR json_extract(NEW.challenge_json,'$.binding.verification_completed_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',json_extract(NEW.challenge_json,'$.binding.verification_started_at'),'+10 minutes')||substr(json_extract(NEW.challenge_json,'$.binding.verification_started_at'),20))
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at')<
               json_extract(NEW.challenge_json,'$.binding.verification_completed_at')
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',json_extract(NEW.challenge_json,'$.binding.verification_completed_at'),'+5 minutes')||substr(json_extract(NEW.challenge_json,'$.binding.verification_completed_at'),20))
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')<=
               json_extract(NEW.challenge_json,'$.binding.report_generated_at')
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',json_extract(NEW.challenge_json,'$.binding.report_generated_at'),'+60 minutes')||substr(json_extract(NEW.challenge_json,'$.binding.report_generated_at'),20))
          OR json_extract(NEW.challenge_json,'$.binding.verification_started_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.verification_completed_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')<=NEW.issued_at
          OR json_extract(NEW.challenge_json,'$.binding.credential_resolution_outcome') IS NOT 'passed'
          OR json_extract(NEW.challenge_json,'$.binding.provider_authentication_outcome') IS NOT 'passed'
          OR json_type(NEW.challenge_json,'$.binding.verifier_report_id') IS NOT 'text'
          OR length(trim(json_extract(NEW.challenge_json,'$.binding.verifier_report_id'))) NOT BETWEEN 1 AND 200
          OR json_extract(NEW.challenge_json,'$.binding.verifier_report_id') IS NOT trim(json_extract(NEW.challenge_json,'$.binding.verifier_report_id'))
          OR json_type(NEW.challenge_json,'$.binding.provider_response_evidence_digest') IS NOT 'text'
          OR length(json_extract(NEW.challenge_json,'$.binding.provider_response_evidence_digest'))<>64
          OR json_extract(NEW.challenge_json,'$.binding.provider_response_evidence_digest') GLOB '*[^0-9a-f]*'
        BEGIN SELECT RAISE(ABORT,'V253 challenge contains stale or future-dated evidence'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_credential_reattestation_receipt_time_bounds
        BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_receipts
        WHEN NEW.verification_completed_at<NEW.verification_started_at
          OR NEW.verification_completed_at>(strftime('%Y-%m-%dT%H:%M:%S',NEW.verification_started_at,'+10 minutes')||substr(NEW.verification_started_at,20))
          OR NEW.report_generated_at<NEW.verification_completed_at
          OR NEW.report_generated_at>(strftime('%Y-%m-%dT%H:%M:%S',NEW.verification_completed_at,'+5 minutes')||substr(NEW.verification_completed_at,20))
          OR NEW.report_expires_at<=NEW.report_generated_at
          OR NEW.report_expires_at>(strftime('%Y-%m-%dT%H:%M:%S',NEW.report_generated_at,'+60 minutes')||substr(NEW.report_generated_at,20))
          OR NEW.verified_at<NEW.report_generated_at OR NEW.verified_at>=NEW.report_expires_at
          OR NEW.verified_at>(strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z')
        BEGIN SELECT RAISE(ABORT,'V253 receipt is outside signed credential time bounds'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_credential_reattestation_revocation_time_order
        BEFORE INSERT ON compute_external_pool_adapter_credential_reattestation_revocations
        WHEN NEW.revoked_at>(strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z')
          OR NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts target
             WHERE target.reattestation_receipt_id=NEW.reattestation_receipt_id
               AND target.reattestation_receipt_digest=NEW.reattestation_receipt_digest
               AND target.provider_binding_id=NEW.provider_binding_id
               AND target.provider_binding_digest=NEW.provider_binding_digest
               AND target.verified_at<=NEW.revoked_at
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                                WHERE successor.predecessor_receipt_id=target.reattestation_receipt_id))
        BEGIN SELECT RAISE(ABORT,'V253 revocation requires exact current head'); END;
        "#,
    )?;
    Ok(())
}
