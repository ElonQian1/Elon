use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_execution_capability_receipts (
            capability_id TEXT PRIMARY KEY CHECK(length(trim(capability_id)) BETWEEN 1 AND 160),
            capability_schema TEXT NOT NULL CHECK(
                capability_schema='compute_federation.execution_capability.v1'
            ),
            capability_digest TEXT NOT NULL UNIQUE CHECK(
                length(capability_digest)=64
                AND capability_digest NOT GLOB '*[^0-9a-f]*'
            ),
            capability_json TEXT NOT NULL CHECK(
                json_valid(capability_json)
                AND length(CAST(capability_json AS BLOB))<=2097152
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            capability_kind TEXT NOT NULL CHECK(capability_kind IN (
                'node_ready','provider_endpoint','adapter_execution'
            )),
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL CHECK(provider_kind IN (
                'user_node','managed_cluster','external_pool'
            )),
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            route_kind TEXT NOT NULL CHECK(route_kind IN (
                'provider_endpoint','server_adapter'
            )),
            route_binding_digest TEXT NOT NULL CHECK(
                length(route_binding_digest)=64
                AND route_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            endpoint_id TEXT,
            endpoint_transport TEXT,
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            adapter_version TEXT NOT NULL CHECK(length(trim(adapter_version)) BETWEEN 1 AND 80),
            adapter_config_revision INTEGER NOT NULL CHECK(
                adapter_config_revision BETWEEN 1 AND 9007199254740991
            ),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512
                AND adapter_config_digest=trim(adapter_config_digest)
            ),
            source_schema TEXT NOT NULL CHECK(length(trim(source_schema)) BETWEEN 1 AND 160),
            source_id TEXT NOT NULL CHECK(length(trim(source_id)) BETWEEN 1 AND 160),
            source_digest TEXT NOT NULL CHECK(
                length(source_digest)=64 AND source_digest NOT GLOB '*[^0-9a-f]*'
            ),
            verification_kind TEXT NOT NULL CHECK(
                length(trim(verification_kind)) BETWEEN 1 AND 80
            ),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verification_digest TEXT NOT NULL CHECK(
                length(verification_digest)=64
                AND verification_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authenticated_at TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(capability_id, capability_digest),
            CHECK(
                (capability_kind='node_ready'
                    AND provider_kind='user_node'
                    AND route_kind='provider_endpoint'
                    AND endpoint_id IS NOT NULL AND endpoint_transport IS NOT NULL
                    AND length(trim(endpoint_id)) BETWEEN 1 AND 160
                    AND length(trim(endpoint_transport)) BETWEEN 1 AND 80
                    AND json_type(capability_json,'$.capability.node_ready') IS 'object'
                    AND json_type(capability_json,
                        '$.capability.runtime.plugin_release') IS 'object')
                OR (capability_kind='provider_endpoint'
                    AND provider_kind='managed_cluster'
                    AND route_kind='provider_endpoint'
                    AND endpoint_id IS NOT NULL AND endpoint_transport IS NOT NULL
                    AND length(trim(endpoint_id)) BETWEEN 1 AND 160
                    AND length(trim(endpoint_transport)) BETWEEN 1 AND 80
                    AND json_type(capability_json,'$.capability.node_ready') IS 'null')
                OR (capability_kind='adapter_execution'
                    AND provider_kind IN ('managed_cluster','external_pool')
                    AND route_kind='server_adapter'
                    AND endpoint_id IS NULL AND endpoint_transport IS NULL
                    AND json_type(capability_json,'$.capability.node_ready') IS 'null')
            ),
            CHECK(
                json_type(capability_json,'$.capability.runtime.plugin_release') IS 'null'
                OR json_type(capability_json,
                    '$.capability.runtime.plugin_release') IS 'object'
            ),
            CHECK(
                json_type(capability_json,'$.capability.model') IS 'null'
                OR (json_type(capability_json,'$.capability.model') IS 'object'
                    AND (json_type(capability_json,
                        '$.capability.model.tokenizer_digest') IS 'null'
                        OR json_type(capability_json,
                            '$.capability.model.tokenizer_digest') IS 'text'))
            ),
            CHECK(
                json_type(capability_json,'$.capability.node_ready') IS 'null'
                OR json_type(capability_json,'$.capability.node_ready') IS 'object'
            ),
            CHECK(length(authenticated_at)=30 AND substr(authenticated_at,5,1)='-'
                AND substr(authenticated_at,8,1)='-' AND substr(authenticated_at,11,1)='T'
                AND substr(authenticated_at,14,1)=':' AND substr(authenticated_at,17,1)=':'
                AND substr(authenticated_at,20,1)='.' AND substr(authenticated_at,30,1)='Z'
                AND julianday(authenticated_at) IS NOT NULL),
            CHECK(length(observed_at)=30 AND substr(observed_at,5,1)='-'
                AND substr(observed_at,8,1)='-' AND substr(observed_at,11,1)='T'
                AND substr(observed_at,14,1)=':' AND substr(observed_at,17,1)=':'
                AND substr(observed_at,20,1)='.' AND substr(observed_at,30,1)='Z'
                AND julianday(observed_at) IS NOT NULL),
            CHECK(length(expires_at)=30 AND substr(expires_at,5,1)='-'
                AND substr(expires_at,8,1)='-' AND substr(expires_at,11,1)='T'
                AND substr(expires_at,14,1)=':' AND substr(expires_at,17,1)=':'
                AND substr(expires_at,20,1)='.' AND substr(expires_at,30,1)='Z'
                AND julianday(expires_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,5,1)='-'
                AND substr(recorded_at,8,1)='-' AND substr(recorded_at,11,1)='T'
                AND substr(recorded_at,14,1)=':' AND substr(recorded_at,17,1)=':'
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            CHECK(authenticated_at<=observed_at AND observed_at<=recorded_at
                AND recorded_at<expires_at),
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_artifact_access_receipts (
            access_id TEXT PRIMARY KEY CHECK(length(trim(access_id)) BETWEEN 1 AND 160),
            access_schema TEXT NOT NULL CHECK(
                access_schema='compute_federation.artifact_access.v1'
            ),
            access_digest TEXT NOT NULL UNIQUE CHECK(
                length(access_digest)=64 AND access_digest NOT GLOB '*[^0-9a-f]*'
            ),
            access_json TEXT NOT NULL CHECK(
                json_valid(access_json)
                AND length(CAST(access_json AS BLOB))<=2097152
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            non_bearer_access_ref TEXT NOT NULL CHECK(
                length(trim(non_bearer_access_ref)) BETWEEN 1 AND 512
            ),
            authorization_digest TEXT NOT NULL CHECK(
                length(authorization_digest)=64
                AND authorization_digest NOT GLOB '*[^0-9a-f]*'
            ),
            job_id TEXT NOT NULL,
            reservation_id TEXT NOT NULL,
            attempt_lease_id TEXT NOT NULL CHECK(
                length(trim(attempt_lease_id)) BETWEEN 1 AND 160
            ),
            provider_id TEXT NOT NULL,
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            fencing_generation INTEGER NOT NULL CHECK(
                fencing_generation BETWEEN 1 AND 9007199254740991
            ),
            route_binding_digest TEXT NOT NULL CHECK(
                length(route_binding_digest)=64
                AND route_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            access_kind TEXT NOT NULL CHECK(access_kind IN ('read','write')),
            target_id TEXT NOT NULL CHECK(length(trim(target_id)) BETWEEN 1 AND 512),
            target_digest TEXT NOT NULL CHECK(
                length(target_digest)=64 AND target_digest NOT GLOB '*[^0-9a-f]*'
            ),
            media_type TEXT NOT NULL CHECK(length(trim(media_type)) BETWEEN 1 AND 255),
            size_limit_bytes INTEGER NOT NULL CHECK(
                size_limit_bytes<=9007199254740991
                AND ((access_kind='read' AND size_limit_bytes>=0)
                    OR (access_kind='write' AND size_limit_bytes>0))
                AND (access_kind='read' OR json_extract(access_json,
                    '$.access.target.target.purpose') IS 'result_write')
            ),
            issued_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(access_id, access_digest),
            CHECK(length(issued_at)=30 AND substr(issued_at,5,1)='-'
                AND substr(issued_at,8,1)='-' AND substr(issued_at,11,1)='T'
                AND substr(issued_at,14,1)=':' AND substr(issued_at,17,1)=':'
                AND substr(issued_at,20,1)='.' AND substr(issued_at,30,1)='Z'
                AND julianday(issued_at) IS NOT NULL),
            CHECK(length(expires_at)=30 AND substr(expires_at,5,1)='-'
                AND substr(expires_at,8,1)='-' AND substr(expires_at,11,1)='T'
                AND substr(expires_at,14,1)=':' AND substr(expires_at,17,1)=':'
                AND substr(expires_at,20,1)='.' AND substr(expires_at,30,1)='Z'
                AND julianday(expires_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,5,1)='-'
                AND substr(recorded_at,8,1)='-' AND substr(recorded_at,11,1)='T'
                AND substr(recorded_at,14,1)=':' AND substr(recorded_at,17,1)=':'
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            CHECK(issued_at<=recorded_at AND recorded_at<expires_at),
            FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_execution_plans (
            plan_id TEXT PRIMARY KEY CHECK(length(trim(plan_id)) BETWEEN 1 AND 160),
            plan_schema TEXT NOT NULL CHECK(
                plan_schema='compute_federation.attempt_execution_plan.v1'
            ),
            plan_digest TEXT NOT NULL UNIQUE CHECK(
                length(plan_digest)=64 AND plan_digest NOT GLOB '*[^0-9a-f]*'
            ),
            plan_json TEXT NOT NULL CHECK(
                json_valid(plan_json) AND length(CAST(plan_json AS BLOB))<=2097152
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            consumer_account_id TEXT NOT NULL CHECK(
                length(trim(consumer_account_id)) BETWEEN 1 AND 160
            ),
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL CHECK(provider_kind IN (
                'user_node','managed_cluster','external_pool'
            )),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160
            ),
            provider_policy_revision INTEGER NOT NULL CHECK(
                provider_policy_revision BETWEEN 1 AND 9007199254740991
            ),
            provider_digest TEXT NOT NULL CHECK(length(provider_digest)=64),
            offer_id TEXT NOT NULL,
            offer_version INTEGER NOT NULL CHECK(offer_version BETWEEN 1 AND 9007199254740991),
            offer_digest TEXT NOT NULL CHECK(length(offer_digest)=64),
            job_id TEXT NOT NULL,
            job_revision INTEGER NOT NULL CHECK(job_revision BETWEEN 1 AND 9007199254740991),
            job_digest TEXT NOT NULL CHECK(length(job_digest)=64),
            reservation_id TEXT NOT NULL,
            reservation_revision INTEGER NOT NULL CHECK(
                reservation_revision BETWEEN 1 AND 9007199254740991
            ),
            reservation_digest TEXT NOT NULL CHECK(length(reservation_digest)=64),
            capacity_claim_id TEXT NOT NULL,
            claim_revision INTEGER NOT NULL CHECK(claim_revision BETWEEN 1 AND 9007199254740991),
            claim_digest TEXT NOT NULL CHECK(length(claim_digest)=64),
            price_snapshot_id TEXT NOT NULL,
            price_snapshot_digest TEXT NOT NULL CHECK(length(price_snapshot_digest)=64),
            budget_reservation_id TEXT NOT NULL,
            budget_reserved_fen INTEGER NOT NULL CHECK(
                budget_reserved_fen BETWEEN 0 AND 9007199254740991
            ),
            broker_request_digest TEXT NOT NULL CHECK(length(broker_request_digest)=64),
            attempt_lease_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(attempt_lease_id)) BETWEEN 1 AND 160
            ),
            attempt_no INTEGER NOT NULL CHECK(attempt_no=1),
            shard_id TEXT,
            fencing_generation INTEGER NOT NULL CHECK(fencing_generation=1),
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            route_binding_digest TEXT NOT NULL CHECK(length(route_binding_digest)=64),
            capability_id TEXT NOT NULL,
            capability_digest TEXT NOT NULL CHECK(length(capability_digest)=64),
            capability_kind TEXT NOT NULL CHECK(capability_kind IN (
                'node_ready','provider_endpoint','adapter_execution'
            )),
            capability_expires_at TEXT NOT NULL,
            resource_grant_id TEXT NOT NULL CHECK(
                length(trim(resource_grant_id)) BETWEEN 1 AND 160
            ),
            resource_grant_schema TEXT NOT NULL CHECK(
                resource_grant_schema='compute_federation.execution_resource_grant.v1'
            ),
            resource_grant_json TEXT NOT NULL CHECK(
                json_valid(resource_grant_json)
                AND length(CAST(resource_grant_json AS BLOB))<=2097152
            ),
            resource_grant_digest TEXT NOT NULL CHECK(
                length(resource_grant_digest)=64
                AND resource_grant_digest NOT GLOB '*[^0-9a-f]*'
            ),
            resource_grant_enforcement_kind TEXT NOT NULL CHECK(
                resource_grant_enforcement_kind IN (
                    'node_host','provider_runtime','server_adapter'
                )
            ),
            artifact_access_count INTEGER NOT NULL CHECK(
                artifact_access_count BETWEEN 0 AND 9007199254740991
            ),
            artifact_access_set_digest TEXT NOT NULL CHECK(
                length(artifact_access_set_digest)=64
                AND artifact_access_set_digest NOT GLOB '*[^0-9a-f]*'
            ),
            lease_authority_kind TEXT NOT NULL CHECK(
                length(trim(lease_authority_kind)) BETWEEN 1 AND 80
            ),
            lease_delivery_mode TEXT NOT NULL CHECK(
                length(trim(lease_delivery_mode)) BETWEEN 1 AND 80
            ),
            lease_audience TEXT NOT NULL CHECK(length(trim(lease_audience)) BETWEEN 1 AND 255),
            lease_authority_valid_until TEXT NOT NULL,
            planned_at TEXT NOT NULL,
            not_after TEXT NOT NULL,
            lease_expires_at TEXT NOT NULL,
            hard_deadline_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(plan_id, plan_digest),
            UNIQUE(job_id, attempt_no),
            CHECK(shard_id IS NULL OR length(trim(shard_id)) BETWEEN 1 AND 160),
            CHECK(
                json_type(plan_json,'$.plan.start.selected_runtime.plugin_release') IS 'null'
                OR json_type(plan_json,
                    '$.plan.start.selected_runtime.plugin_release') IS 'object'
            ),
            CHECK(
                json_type(plan_json,'$.plan.start.selected_model') IS 'null'
                OR (json_type(plan_json,'$.plan.start.selected_model') IS 'object'
                    AND (json_type(plan_json,
                        '$.plan.start.selected_model.tokenizer_digest') IS 'null'
                        OR json_type(plan_json,
                            '$.plan.start.selected_model.tokenizer_digest') IS 'text'))
            ),
            CHECK(json_type(plan_json,'$.plan.start.latest_checkpoint') IS 'null'),
            CHECK(json_type(plan_json,'$.plan.start.workload.shard') IS 'null'
                OR json_type(plan_json,'$.plan.start.workload.shard') IS 'object'),
            CHECK(json_extract(plan_json,
                    '$.plan.start.workload.checkpoint_policy.mode') IS 'disabled'
                AND json_type(plan_json,
                    '$.plan.start.workload.checkpoint_policy.interval_seconds') IS 'null'
                AND json_extract(plan_json,
                    '$.plan.start.workload.checkpoint_policy.maximum_checkpoints') IS 0
                AND json_type(plan_json,
                    '$.plan.start.workload.checkpoint_policy.checkpoint_media_type') IS 'null'),
            CHECK(length(planned_at)=30 AND substr(planned_at,5,1)='-'
                AND substr(planned_at,8,1)='-' AND substr(planned_at,11,1)='T'
                AND substr(planned_at,14,1)=':' AND substr(planned_at,17,1)=':'
                AND substr(planned_at,20,1)='.' AND substr(planned_at,30,1)='Z'
                AND julianday(planned_at) IS NOT NULL),
            CHECK(length(not_after)=30 AND substr(not_after,5,1)='-'
                AND substr(not_after,8,1)='-' AND substr(not_after,11,1)='T'
                AND substr(not_after,14,1)=':' AND substr(not_after,17,1)=':'
                AND substr(not_after,20,1)='.' AND substr(not_after,30,1)='Z'
                AND julianday(not_after) IS NOT NULL),
            CHECK(length(capability_expires_at)=30 AND substr(capability_expires_at,5,1)='-'
                AND substr(capability_expires_at,8,1)='-'
                AND substr(capability_expires_at,11,1)='T'
                AND substr(capability_expires_at,14,1)=':'
                AND substr(capability_expires_at,17,1)=':'
                AND substr(capability_expires_at,20,1)='.'
                AND substr(capability_expires_at,30,1)='Z'
                AND julianday(capability_expires_at) IS NOT NULL),
            CHECK(length(lease_authority_valid_until)=30
                AND substr(lease_authority_valid_until,5,1)='-'
                AND substr(lease_authority_valid_until,8,1)='-'
                AND substr(lease_authority_valid_until,11,1)='T'
                AND substr(lease_authority_valid_until,14,1)=':'
                AND substr(lease_authority_valid_until,17,1)=':'
                AND substr(lease_authority_valid_until,20,1)='.'
                AND substr(lease_authority_valid_until,30,1)='Z'
                AND julianday(lease_authority_valid_until) IS NOT NULL),
            CHECK(length(lease_expires_at)=30 AND substr(lease_expires_at,5,1)='-'
                AND substr(lease_expires_at,8,1)='-' AND substr(lease_expires_at,11,1)='T'
                AND substr(lease_expires_at,14,1)=':' AND substr(lease_expires_at,17,1)=':'
                AND substr(lease_expires_at,20,1)='.' AND substr(lease_expires_at,30,1)='Z'
                AND julianday(lease_expires_at) IS NOT NULL),
            CHECK(length(hard_deadline_at)=30 AND substr(hard_deadline_at,5,1)='-'
                AND substr(hard_deadline_at,8,1)='-'
                AND substr(hard_deadline_at,11,1)='T'
                AND substr(hard_deadline_at,14,1)=':'
                AND substr(hard_deadline_at,17,1)=':'
                AND substr(hard_deadline_at,20,1)='.'
                AND substr(hard_deadline_at,30,1)='Z'
                AND julianday(hard_deadline_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,5,1)='-'
                AND substr(recorded_at,8,1)='-' AND substr(recorded_at,11,1)='T'
                AND substr(recorded_at,14,1)=':' AND substr(recorded_at,17,1)=':'
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            CHECK(planned_at<=recorded_at AND recorded_at<not_after
                AND hard_deadline_at<=capability_expires_at
                AND hard_deadline_at<=lease_authority_valid_until
                AND not_after<lease_expires_at AND lease_expires_at<hard_deadline_at),
            FOREIGN KEY(provider_id, provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id, policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
            FOREIGN KEY(job_id, job_revision)
                REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id, reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(capacity_claim_id, claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(price_snapshot_id) REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(budget_reservation_id) REFERENCES billing_reservations(id)
                ON DELETE RESTRICT,
            FOREIGN KEY(capability_id, capability_digest)
                REFERENCES compute_execution_capability_receipts(
                    capability_id, capability_digest
                ) ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_execution_plan_accesses (
            plan_id TEXT NOT NULL,
            plan_digest TEXT NOT NULL CHECK(length(plan_digest)=64),
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 9007199254740991),
            access_id TEXT NOT NULL,
            access_digest TEXT NOT NULL CHECK(length(access_digest)=64),
            access_kind TEXT NOT NULL CHECK(access_kind IN ('read','write')),
            target_id TEXT NOT NULL CHECK(length(trim(target_id)) BETWEEN 1 AND 512),
            target_digest TEXT NOT NULL CHECK(length(target_digest)=64),
            expires_at TEXT NOT NULL,
            CHECK(length(expires_at)=30 AND substr(expires_at,5,1)='-'
                AND substr(expires_at,8,1)='-' AND substr(expires_at,11,1)='T'
                AND substr(expires_at,14,1)=':' AND substr(expires_at,17,1)=':'
                AND substr(expires_at,20,1)='.' AND substr(expires_at,30,1)='Z'
                AND julianday(expires_at) IS NOT NULL),
            PRIMARY KEY(plan_id, ordinal),
            UNIQUE(access_id),
            FOREIGN KEY(plan_id, plan_digest)
                REFERENCES compute_attempt_execution_plans(plan_id, plan_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(access_id, access_digest)
                REFERENCES compute_artifact_access_receipts(access_id, access_digest)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_execution_plan_seals (
            seal_id TEXT PRIMARY KEY CHECK(length(trim(seal_id)) BETWEEN 1 AND 160),
            seal_schema TEXT NOT NULL CHECK(
                seal_schema='compute_federation.attempt_execution_plan_seal.v1'
            ),
            seal_digest TEXT NOT NULL UNIQUE CHECK(
                length(seal_digest)=64 AND seal_digest NOT GLOB '*[^0-9a-f]*'
            ),
            seal_json TEXT NOT NULL CHECK(
                json_valid(seal_json) AND length(CAST(seal_json AS BLOB))<=2097152
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            plan_id TEXT NOT NULL UNIQUE,
            plan_digest TEXT NOT NULL CHECK(length(plan_digest)=64),
            capability_digest TEXT NOT NULL CHECK(length(capability_digest)=64),
            artifact_access_count INTEGER NOT NULL CHECK(
                artifact_access_count BETWEEN 0 AND 9007199254740991
            ),
            artifact_access_set_digest TEXT NOT NULL CHECK(
                length(artifact_access_set_digest)=64
                AND artifact_access_set_digest NOT GLOB '*[^0-9a-f]*'
            ),
            resource_grant_digest TEXT NOT NULL CHECK(
                length(resource_grant_digest)=64
                AND resource_grant_digest NOT GLOB '*[^0-9a-f]*'
            ),
            sealed_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK(length(sealed_at)=30 AND substr(sealed_at,5,1)='-'
                AND substr(sealed_at,8,1)='-' AND substr(sealed_at,11,1)='T'
                AND substr(sealed_at,14,1)=':' AND substr(sealed_at,17,1)=':'
                AND substr(sealed_at,20,1)='.' AND substr(sealed_at,30,1)='Z'
                AND julianday(sealed_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,5,1)='-'
                AND substr(recorded_at,8,1)='-' AND substr(recorded_at,11,1)='T'
                AND substr(recorded_at,14,1)=':' AND substr(recorded_at,17,1)=':'
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            CHECK(sealed_at<=recorded_at),
            FOREIGN KEY(plan_id, plan_digest)
                REFERENCES compute_attempt_execution_plans(plan_id, plan_digest)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_execution_capability_expiry
            ON compute_execution_capability_receipts(provider_id, expires_at, capability_id);
        CREATE INDEX IF NOT EXISTS idx_compute_artifact_access_attempt
            ON compute_artifact_access_receipts(attempt_lease_id, expires_at, access_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_execution_plans_source
            ON compute_attempt_execution_plans(reservation_id, attempt_no, plan_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_execution_plans_deadline
            ON compute_attempt_execution_plans(not_after, plan_id);
        "#,
    )?;
    Ok(())
}
