use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_challenge_lineage
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_challenges
        WHEN (NEW.sequence=1 AND NEW.predecessor_receipt_id IS NOT NULL)
          OR (NEW.sequence<>1 AND NEW.predecessor_receipt_id IS NULL)
          OR (NEW.sequence=1 AND EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts existing
             WHERE existing.registry_release_id=NEW.registry_release_id))
          OR (NEW.predecessor_receipt_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts predecessor
             WHERE predecessor.reattestation_receipt_id=NEW.predecessor_receipt_id
               AND predecessor.reattestation_receipt_digest=NEW.predecessor_receipt_digest
               AND predecessor.registry_release_id=NEW.registry_release_id
               AND predecessor.registry_release_digest=NEW.registry_release_digest
               AND predecessor.sequence+1=NEW.sequence
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts successor
                                WHERE successor.predecessor_receipt_id=predecessor.reattestation_receipt_id)))
        BEGIN SELECT RAISE(ABORT,'V252 challenge requires exact current predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_receipt_lineage
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_receipts
        WHEN (NEW.sequence=1 AND NEW.predecessor_receipt_id IS NOT NULL)
          OR (NEW.sequence<>1 AND NEW.predecessor_receipt_id IS NULL)
          OR (NEW.sequence=1 AND EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts existing
             WHERE existing.registry_release_id=NEW.registry_release_id))
          OR (NEW.predecessor_receipt_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts predecessor
             WHERE predecessor.reattestation_receipt_id=NEW.predecessor_receipt_id
               AND predecessor.reattestation_receipt_digest=NEW.predecessor_receipt_digest
               AND predecessor.registry_release_id=NEW.registry_release_id
               AND predecessor.registry_release_digest=NEW.registry_release_digest
               AND predecessor.sequence+1=NEW.sequence
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts successor
                                WHERE successor.predecessor_receipt_id=predecessor.reattestation_receipt_id)))
        BEGIN SELECT RAISE(ABORT,'V252 receipt requires exact current predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_receipt_time_bounds
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_receipts
        WHEN NEW.run_started_at<NEW.vulnerability_reattestation_verified_at
          OR NEW.run_completed_at<NEW.run_started_at
          OR NEW.report_generated_at<NEW.run_completed_at
          OR NEW.report_expires_at<=NEW.report_generated_at
          OR NEW.verified_at<NEW.report_generated_at
          OR NEW.verified_at>=NEW.report_expires_at
          OR NEW.report_expires_at>NEW.vulnerability_intelligence_expires_at
          OR NEW.run_completed_at>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.run_started_at,'+30 minutes')||substr(NEW.run_started_at,20))
          OR NEW.report_expires_at>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.report_generated_at,'+24 hours')||substr(NEW.report_generated_at,20))
          OR NEW.verified_at>
               (strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z')
        BEGIN SELECT RAISE(ABORT,'V252 receipt is outside signed sandbox time bounds'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_challenge_time_bounds
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_challenges
        WHEN json_extract(NEW.challenge_json,'$.binding.run_started_at') NOT GLOB
               '????-??-??T??:??:??.?????????Z'
          OR length(json_extract(NEW.challenge_json,'$.binding.run_started_at'))<>30
          OR julianday(json_extract(NEW.challenge_json,'$.binding.run_started_at')) IS NULL
          OR json_extract(NEW.challenge_json,'$.binding.run_completed_at') NOT GLOB
               '????-??-??T??:??:??.?????????Z'
          OR length(json_extract(NEW.challenge_json,'$.binding.run_completed_at'))<>30
          OR julianday(json_extract(NEW.challenge_json,'$.binding.run_completed_at')) IS NULL
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at') NOT GLOB
               '????-??-??T??:??:??.?????????Z'
          OR length(json_extract(NEW.challenge_json,'$.binding.report_generated_at'))<>30
          OR julianday(json_extract(NEW.challenge_json,'$.binding.report_generated_at')) IS NULL
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at') NOT GLOB
               '????-??-??T??:??:??.?????????Z'
          OR length(json_extract(NEW.challenge_json,'$.binding.report_expires_at'))<>30
          OR julianday(json_extract(NEW.challenge_json,'$.binding.report_expires_at')) IS NULL
          OR json_extract(NEW.challenge_json,'$.binding.run_completed_at')<
               json_extract(NEW.challenge_json,'$.binding.run_started_at')
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at')<
               json_extract(NEW.challenge_json,'$.binding.run_completed_at')
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')<=
               json_extract(NEW.challenge_json,'$.binding.report_generated_at')
          OR json_extract(NEW.challenge_json,'$.binding.run_completed_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',json_extract(NEW.challenge_json,'$.binding.run_started_at'),'+30 minutes')||
                substr(json_extract(NEW.challenge_json,'$.binding.run_started_at'),20))
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',json_extract(NEW.challenge_json,'$.binding.report_generated_at'),'+24 hours')||
                substr(json_extract(NEW.challenge_json,'$.binding.report_generated_at'),20))
          OR json_extract(NEW.challenge_json,'$.binding.run_started_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.run_completed_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.report_generated_at')>
               (strftime('%Y-%m-%dT%H:%M:%S',NEW.issued_at,'+5 minutes')||substr(NEW.issued_at,20))
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')<=NEW.issued_at
        BEGIN SELECT RAISE(ABORT,'V252 challenge contains stale or future-dated report evidence'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_challenge_evidence_shape
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_challenges
        WHEN json_type(NEW.challenge_json,'$.binding.test_plan') IS NOT 'array'
          OR json_array_length(json_extract(NEW.challenge_json,'$.binding.test_plan')) IS NOT 6
          OR json_type(NEW.challenge_json,'$.binding.observations') IS NOT 'array'
          OR json_array_length(json_extract(NEW.challenge_json,'$.binding.observations')) IS NOT 6
          OR json_type(NEW.challenge_json,'$.binding.peak_memory_bytes') IS NOT 'integer'
          OR json_extract(NEW.challenge_json,'$.binding.peak_memory_bytes') NOT BETWEEN 1 AND 536870912
          OR json_extract(NEW.challenge_json,'$.binding.peak_memory_bytes') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.cpu_time_ms') IS NOT 'integer'
          OR json_extract(NEW.challenge_json,'$.binding.cpu_time_ms') NOT BETWEEN 1 AND 900000
          OR json_extract(NEW.challenge_json,'$.binding.cpu_time_ms') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.verifier_report_id') IS NOT 'text'
          OR length(trim(json_extract(NEW.challenge_json,'$.binding.verifier_report_id'))) NOT BETWEEN 1 AND 200
          OR json_extract(NEW.challenge_json,'$.binding.verifier_report_id') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.sandbox_runtime_id') IS NOT 'text'
          OR length(trim(json_extract(NEW.challenge_json,'$.binding.sandbox_runtime_id'))) NOT BETWEEN 1 AND 200
          OR json_extract(NEW.challenge_json,'$.binding.sandbox_runtime_id') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.runtime_image_digest') IS NOT 'text'
          OR length(json_extract(NEW.challenge_json,'$.binding.runtime_image_digest'))<>64
          OR json_extract(NEW.challenge_json,'$.binding.runtime_image_digest') GLOB '*[^0-9a-f]*'
          OR json_extract(NEW.challenge_json,'$.binding.runtime_image_digest') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.test_plan_digest') IS NOT 'text'
          OR length(json_extract(NEW.challenge_json,'$.binding.test_plan_digest'))<>64
          OR json_extract(NEW.challenge_json,'$.binding.test_plan_digest') GLOB '*[^0-9a-f]*'
          OR json_extract(NEW.challenge_json,'$.binding.test_plan_digest') IS NULL
          OR json_type(NEW.challenge_json,'$.binding.observation_inventory_digest') IS NOT 'text'
          OR length(json_extract(NEW.challenge_json,'$.binding.observation_inventory_digest'))<>64
          OR json_extract(NEW.challenge_json,'$.binding.observation_inventory_digest') GLOB '*[^0-9a-f]*'
          OR json_extract(NEW.challenge_json,'$.binding.observation_inventory_digest') IS NULL
          OR length(CAST(json_extract(NEW.challenge_json,'$.binding.test_plan') AS BLOB))>262144
          OR length(CAST(json_extract(NEW.challenge_json,'$.binding.observations') AS BLOB))>262144
        BEGIN SELECT RAISE(ABORT,'V252 challenge evidence shape is not fail-closed'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_revocation_time_order
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_revocations
        WHEN NEW.revoked_at>
               (strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z') OR NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts receipt
           WHERE receipt.reattestation_receipt_id=NEW.reattestation_receipt_id
             AND receipt.reattestation_receipt_digest=NEW.reattestation_receipt_digest
             AND receipt.registry_release_id=NEW.registry_release_id
             AND receipt.registry_release_digest=NEW.registry_release_digest
             AND receipt.verified_at<=NEW.revoked_at
             AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts successor
                              WHERE successor.predecessor_receipt_id=receipt.reattestation_receipt_id))
        BEGIN SELECT RAISE(ABORT,'V252 revocation requires exact current head'); END;
        "#,
    )?;
    Ok(())
}
