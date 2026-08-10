use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauth_consumption_projection
        BEFORE INSERT ON node_endpoint_owner_reauthentication_consumptions
        WHEN json_type(NEW.consumption_json)!='object'
          OR (SELECT COUNT(*) FROM json_each(NEW.consumption_json))!=12
          OR json_type(NEW.consumption_json,'$.credential_result')!='object'
          OR (
                SELECT COUNT(*)
                  FROM json_each(NEW.consumption_json,'$.credential_result')
             )!=9
          OR json_extract(NEW.consumption_json,'$.schema')
                IS NOT NEW.consumption_schema
          OR json_extract(NEW.consumption_json,'$.consumption_id')
                IS NOT NEW.consumption_id
          OR json_extract(NEW.consumption_json,'$.reauthentication_receipt_id')
                IS NOT NEW.reauthentication_receipt_id
          OR json_extract(NEW.consumption_json,'$.reauthentication_digest')
                IS NOT NEW.reauthentication_digest
          OR json_extract(NEW.consumption_json,'$.owner_user_id')
                IS NOT NEW.owner_user_id
          OR json_extract(NEW.consumption_json,'$.authorization_action')
                IS NOT NEW.authorization_action
          OR json_extract(NEW.consumption_json,'$.credential_mutation_request_id')
                IS NOT NEW.credential_mutation_request_id
          OR json_extract(NEW.consumption_json,'$.credential_mutation_request_digest')
                IS NOT NEW.credential_mutation_request_digest
          OR json_extract(NEW.consumption_json,'$.authorization_target_digest')
                IS NOT NEW.authorization_target_digest
          OR json_extract(
                NEW.consumption_json,
                '$.credential_result.current_credential_id'
             ) IS NOT NEW.current_credential_id
          OR json_extract(
                NEW.consumption_json,
                '$.credential_result.current_credential_revision'
             ) IS NOT NEW.current_credential_revision
          OR json_extract(
                NEW.consumption_json,
                '$.credential_result.current_credential_digest'
             ) IS NOT NEW.current_credential_digest
          OR json_extract(
                NEW.consumption_json,
                '$.credential_result.current_credential_status'
             ) IS NOT NEW.current_credential_status
          OR (
                NEW.issued_credential_id IS NULL
                AND json_type(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_id'
                ) IS NOT 'null'
             )
          OR (
                NEW.issued_credential_id IS NOT NULL
                AND json_extract(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_id'
                ) IS NOT NEW.issued_credential_id
             )
          OR (
                NEW.issued_credential_revision IS NULL
                AND json_type(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_revision'
                ) IS NOT 'null'
             )
          OR (
                NEW.issued_credential_revision IS NOT NULL
                AND json_extract(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_revision'
                ) IS NOT NEW.issued_credential_revision
             )
          OR (
                NEW.issued_credential_digest IS NULL
                AND json_type(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_digest'
                ) IS NOT 'null'
             )
          OR (
                NEW.issued_credential_digest IS NOT NULL
                AND json_extract(
                    NEW.consumption_json,
                    '$.credential_result.issued_credential_digest'
                ) IS NOT NEW.issued_credential_digest
             )
          OR (
                NEW.revocation_id IS NULL
                AND json_type(
                    NEW.consumption_json,
                    '$.credential_result.revocation_id'
                ) IS NOT 'null'
             )
          OR (
                NEW.revocation_id IS NOT NULL
                AND json_extract(
                    NEW.consumption_json,
                    '$.credential_result.revocation_id'
                ) IS NOT NEW.revocation_id
             )
          OR (
                NEW.revocation_digest IS NULL
                AND json_type(
                    NEW.consumption_json,
                    '$.credential_result.revocation_digest'
                ) IS NOT 'null'
             )
          OR (
                NEW.revocation_digest IS NOT NULL
                AND json_extract(
                    NEW.consumption_json,
                    '$.credential_result.revocation_digest'
                ) IS NOT NEW.revocation_digest
             )
          OR json_extract(NEW.consumption_json,'$.consumed_at') IS NOT NEW.consumed_at
          OR json_extract(NEW.consumption_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR json_type(NEW.consumption_json,'$.consumption_digest') IS NOT NULL
          OR json_type(NEW.consumption_json,'$.canonicalization') IS NOT NULL
          OR json_type(NEW.consumption_json,'$.digest_algorithm') IS NOT NULL
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication consumption projection mismatch');
        END;
        "#,
    )?;
    Ok(())
}
