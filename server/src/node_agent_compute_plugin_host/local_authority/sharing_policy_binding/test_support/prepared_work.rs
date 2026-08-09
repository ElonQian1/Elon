use rusqlite::{params, Connection};

use super::super::types::PolicyBindingAuthorityState;
use crate::node_agent_compute_plugin_host::local_authority_schema::ensure_schema;

const PLAN_ID: &str = "plan_prepared_work";
const PLAN_DIGEST: &str = "6666666666666666666666666666666666666666666666666666666666666666";
const BUNDLE_REVISION: i64 = 1;
const PUBLISHER_DIGEST: &str = "7777777777777777777777777777777777777777777777777777777777777777";
const CONTROL_DIGEST: &str = "8888888888888888888888888888888888888888888888888888888888888888";

pub(in crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding) fn seed_prepared_work(
    connection: &mut Connection,
    before: &PolicyBindingAuthorityState,
) {
    let triggers = remove_triggers(connection);
    seed_keyring(connection, before.updated_at_ms);
    seed_plan(connection, before);
    seed_candidates_and_downloads(connection, before.inventory_revision, before.updated_at_ms);
    seed_work(connection, before);
    restore_triggers(connection, triggers);
    ensure_schema(connection).unwrap();
}

fn remove_triggers(connection: &Connection) -> Vec<(String, String)> {
    let mut statement = connection
        .prepare(
            "SELECT name, sql FROM sqlite_schema \
             WHERE type = 'trigger' AND sql IS NOT NULL ORDER BY name",
        )
        .unwrap();
    let triggers = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<(String, String)>>>()
        .unwrap();
    drop(statement);
    for (name, _) in &triggers {
        let quoted = name.replace('"', "\"\"");
        connection
            .execute_batch(&format!("DROP TRIGGER \"{quoted}\""))
            .unwrap();
    }
    triggers
}

fn restore_triggers(connection: &Connection, triggers: Vec<(String, String)>) {
    for (_, sql) in triggers {
        connection.execute_batch(&sql).unwrap();
    }
}

fn seed_keyring(connection: &Connection, installed_at_ms: i64) {
    connection
        .execute(
            r#"INSERT INTO keyring_bundles (
                bundle_revision, bundle_digest, signed_envelope_digest, signed_bundle_json,
                root_signing_key_id, root_key_fingerprint,
                publisher_revision, publisher_digest, control_revision, control_digest,
                publisher_key_count, control_key_count,
                generated_at_ms, expires_at_ms, installed_at_ms
            ) VALUES (?1, ?2, ?3, '{}', 'root_key_1', ?4, 1, ?5, 1, ?6, 0, 0, ?7, ?8, ?9)"#,
            params![
                BUNDLE_REVISION,
                "9".repeat(64),
                "a".repeat(64),
                "b".repeat(64),
                PUBLISHER_DIGEST,
                CONTROL_DIGEST,
                installed_at_ms - 60_000,
                installed_at_ms + 3_600_000,
                installed_at_ms,
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO keyring_seals (bundle_revision, sealed_at_ms) VALUES (?1, ?2)",
            params![BUNDLE_REVISION, installed_at_ms],
        )
        .unwrap();
}

fn seed_plan(connection: &Connection, before: &PolicyBindingAuthorityState) {
    connection
        .execute(
            r#"INSERT INTO plan_applications (
                plan_id, plan_digest, application_request_digest,
                signed_plan_envelope_digest, signed_manifest_set_digest,
                signed_plan_json, signed_manifests_json,
                admission_bindings_json, admission_bindings_digest,
                expected_inventory_revision, expected_inventory_digest,
                application_inventory_revision, inventory_after_digest, inventory_after_json,
                application_state_revision, authority_epoch_at_apply,
                keyring_bundle_revision, publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                control_signing_key_fingerprint,
                new_candidate_count, closed_candidate_count, download_count, download_bytes,
                applied_at_ms, expires_at_ms, receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, '{}', '[]', '[]', ?6,
                ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, 1, ?15, 1, ?16, ?17,
                2, 0, 2, 128, ?18, ?19, '{}', ?20
            )"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                "c".repeat(64),
                "d".repeat(64),
                "e".repeat(64),
                "f".repeat(64),
                before.inventory_revision - 1,
                &before.inventory_digest,
                before.inventory_revision,
                &before.inventory_digest,
                &before.inventory_json,
                before.state_revision - 1,
                before.authority_epoch - 1,
                BUNDLE_REVISION,
                PUBLISHER_DIGEST,
                CONTROL_DIGEST,
                "1".repeat(64),
                before.updated_at_ms - 30_000,
                before.updated_at_ms + 3_600_000,
                "2".repeat(64),
            ],
        )
        .unwrap();
    connection
        .execute(
            r#"INSERT INTO plan_application_seals (
                plan_id, plan_digest, application_request_digest, receipt_digest, sealed_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                "c".repeat(64),
                "2".repeat(64),
                before.updated_at_ms - 30_000,
            ],
        )
        .unwrap();
}

fn seed_candidates_and_downloads(
    connection: &Connection,
    inventory_revision: i64,
    created_at_ms: i64,
) {
    for (token, plugin_id, slot_ref, generation) in [
        ("candidate_fetch", "plugin_fetch", "slot_fetch", 1_i64),
        (
            "candidate_verification",
            "plugin_verification",
            "slot_verification",
            2_i64,
        ),
    ] {
        connection
            .execute(
                r#"INSERT INTO candidate_owners (
                    candidate_token, plugin_id, slot_ref, candidate_generation,
                    release_json, permission_grant_digest,
                    owner_plan_id, owner_plan_digest, application_inventory_revision,
                    state, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, '{}', ?5, ?6, ?7, ?8, 'owned', ?9)"#,
                params![
                    token,
                    plugin_id,
                    slot_ref,
                    generation,
                    "3".repeat(64),
                    PLAN_ID,
                    PLAN_DIGEST,
                    inventory_revision,
                    created_at_ms - 20_000,
                ],
            )
            .unwrap();
    }
    connection
        .execute(
            r#"INSERT INTO planned_downloads (
                plan_id, plan_digest, ordinal, item_index, candidate_token,
                artifact_kind, artifact_id, artifact_digest, source_ref, cache_class,
                part_relative_path, size_bytes, committed_offset, cursor_generation,
                state, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, 0, 0, 'candidate_fetch', 'package', 'fetch_artifact', ?3,
                'https://example.invalid/fetch', 'private', 'fetch.part',
                64, 32, 1, 'downloading', ?4, ?5)"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                "4".repeat(64),
                created_at_ms - 20_000,
                created_at_ms - 10_000,
            ],
        )
        .unwrap();
    connection
        .execute(
            r#"INSERT INTO planned_downloads (
                plan_id, plan_digest, ordinal, item_index, candidate_token,
                artifact_kind, artifact_id, artifact_digest, source_ref, cache_class,
                part_relative_path, size_bytes, committed_offset, cursor_generation,
                state, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, 1, 1, 'candidate_verification', 'package',
                'verification_artifact', ?3, 'https://example.invalid/verification',
                'private', 'verification.part', 64, 64, 1, 'complete', ?4, ?5)"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                "5".repeat(64),
                created_at_ms - 20_000,
                created_at_ms - 10_000,
            ],
        )
        .unwrap();
}

fn seed_work(connection: &Connection, before: &PolicyBindingAuthorityState) {
    let prepared_at_ms = before.trusted_time_high_water_ms - 1_000;
    connection
        .execute(
            r#"INSERT INTO fetch_claims (
                claim_id, plan_id, plan_digest, ordinal, candidate_token,
                authority_epoch, process_owner_epoch, cursor_generation,
                redirect_generation, offset_bytes, length_bytes, end_offset_bytes,
                state, prepared_at_ms
            ) VALUES ('claim_prepared', ?1, ?2, 0, 'candidate_fetch',
                ?3, ?4, 1, 0, 32, 16, 48, 'prepared', ?5)"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                before.authority_epoch,
                before.process_owner_epoch,
                prepared_at_ms,
            ],
        )
        .unwrap();
    connection
        .execute(
            r#"INSERT INTO candidate_verification_runs (
                verification_id, candidate_token, owner_plan_id, owner_plan_digest,
                verification_generation, candidate_generation,
                application_inventory_revision, authority_state_revision,
                authority_epoch, process_owner_epoch, artifact_count, artifact_bytes,
                expected_artifact_set_digest, file_set_binding_digest,
                state, prepared_at_ms
            ) VALUES ('verification_prepared', 'candidate_verification', ?1, ?2,
                1, 2, ?3, ?4, ?5, ?6, 1, 64, ?7, ?8, 'prepared', ?9)"#,
            params![
                PLAN_ID,
                PLAN_DIGEST,
                before.inventory_revision,
                before.state_revision,
                before.authority_epoch,
                before.process_owner_epoch,
                "6".repeat(64),
                "7".repeat(64),
                prepared_at_ms,
            ],
        )
        .unwrap();
}
