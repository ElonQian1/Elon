use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_projection
        BEFORE INSERT ON compute_external_pool_adapter_release_admission_terminal_receipts
        WHEN json_type(NEW.terminal_receipt_json) IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json))<>7
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.terminal_receipt_json)
                 WHERE key NOT IN ('schema','terminal_receipt_id','terminal_receipt_digest',
                    'request_digest','canonicalization','digest_algorithm','terminal'))
          OR json_type(NEW.terminal_receipt_json,'$.terminal') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,'$.terminal'))<>17
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.terminal_receipt_json,'$.terminal')
                 WHERE key NOT IN ('admission','prior_status','terminal_status',
                    'successor_admission','actor_kind','actor_id','reason','confirmation',
                    'idempotency_scope','idempotency_key','occurred_at','recorded_at',
                    'currentness_effect','artifact_intake_effect',
                    'existing_artifact_source_effect','adapter_effect','route_effect'))
          OR json_type(NEW.terminal_receipt_json,'$.terminal.admission') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(
                NEW.terminal_receipt_json,'$.terminal.admission'))<>4
          OR EXISTS (
                SELECT 1 FROM json_each(
                    NEW.terminal_receipt_json,'$.terminal.admission')
                 WHERE key NOT IN (
                    'admission_id','admission_digest','adapter_id','release_version'))
          OR (NEW.successor_admission_id IS NULL
                AND json_type(NEW.terminal_receipt_json,
                    '$.terminal.successor_admission') IS NOT 'null')
          OR (NEW.successor_admission_id IS NOT NULL AND (
                json_type(NEW.terminal_receipt_json,
                    '$.terminal.successor_admission') IS NOT 'object'
                OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,
                    '$.terminal.successor_admission'))<>3
                OR EXISTS (
                    SELECT 1 FROM json_each(NEW.terminal_receipt_json,
                        '$.terminal.successor_admission')
                     WHERE key NOT IN (
                        'admission_id','admission_digest','release_version'))))
          OR json_extract(NEW.terminal_receipt_json,'$.schema')
                IS NOT NEW.terminal_receipt_schema
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_id')
                IS NOT NEW.terminal_receipt_id
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_digest')
                IS NOT NEW.terminal_receipt_digest
          OR json_extract(NEW.terminal_receipt_json,'$.request_digest')
                IS NOT NEW.request_digest
          OR json_extract(NEW.terminal_receipt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.terminal_receipt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.admission.admission_id')
                IS NOT NEW.admission_id
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.admission.admission_digest')
                IS NOT NEW.admission_digest
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.admission.adapter_id')
                IS NOT NEW.adapter_id
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.admission.release_version')
                IS NOT NEW.release_version
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.prior_status')
                IS NOT NEW.prior_status
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.terminal_status')
                IS NOT NEW.terminal_status
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.successor_admission.admission_id')
                IS NOT NEW.successor_admission_id
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.successor_admission.admission_digest')
                IS NOT NEW.successor_admission_digest
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.successor_admission.release_version')
                IS NOT NEW.successor_release_version
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.actor_kind')
                IS NOT NEW.actor_kind
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.actor_id')
                IS NOT NEW.actor_id
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.reason') IS NOT NEW.reason
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.confirmation')
                IS NOT NEW.confirmation
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.occurred_at')
                IS NOT NEW.occurred_at
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.recorded_at')
                IS NOT NEW.recorded_at
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.currentness_effect')
                IS NOT NEW.currentness_effect
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.artifact_intake_effect')
                IS NOT NEW.artifact_intake_effect
          OR json_extract(NEW.terminal_receipt_json,
                '$.terminal.existing_artifact_source_effect')
                IS NOT NEW.existing_artifact_source_effect
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.adapter_effect')
                IS NOT NEW.adapter_effect
          OR json_extract(NEW.terminal_receipt_json,'$.terminal.route_effect')
                IS NOT NEW.route_effect
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release admission terminal projection mismatch');
        END;
        "#,
    )?;
    Ok(())
}
