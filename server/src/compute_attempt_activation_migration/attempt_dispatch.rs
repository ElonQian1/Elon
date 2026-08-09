use anyhow::Result;
use rusqlite::Connection;

mod triggers;

pub(super) fn migration_v211(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_attempt_dispatch_commands (
            command_id TEXT PRIMARY KEY CHECK(length(trim(command_id)) BETWEEN 1 AND 160),
            command_schema TEXT NOT NULL CHECK(
                command_schema='compute_federation.attempt_dispatch_command.v1'
            ),
            command_type TEXT NOT NULL CHECK(command_type='start'),
            command_digest TEXT NOT NULL UNIQUE CHECK(length(command_digest)=64),
            command_json TEXT NOT NULL CHECK(
                json_valid(command_json) AND length(CAST(command_json AS BLOB)) <= 524288
            ),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            adapter_binding_json TEXT NOT NULL CHECK(
                json_valid(adapter_binding_json)
                AND length(CAST(adapter_binding_json AS BLOB)) <= 65536
            ),
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL CHECK(
                provider_kind IN ('user_node','managed_cluster','external_pool')
            ),
            route_kind TEXT NOT NULL CHECK(
                route_kind IN ('provider_endpoint','server_adapter')
            ),
            provider_policy_revision INTEGER NOT NULL CHECK(
                provider_policy_revision BETWEEN 1 AND 9007199254740991
            ),
            provider_digest TEXT NOT NULL CHECK(length(provider_digest)=64),
            endpoint_id TEXT,
            endpoint_transport TEXT,
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            adapter_version TEXT NOT NULL CHECK(length(trim(adapter_version)) BETWEEN 1 AND 80),
            adapter_config_revision INTEGER NOT NULL CHECK(
                adapter_config_revision BETWEEN 1 AND 9007199254740991
            ),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) > 0
                AND adapter_config_digest=trim(adapter_config_digest)
            ),
            job_id TEXT NOT NULL,
            job_revision INTEGER NOT NULL CHECK(job_revision BETWEEN 1 AND 9007199254740991),
            job_digest TEXT NOT NULL CHECK(length(job_digest)=64),
            reservation_id TEXT NOT NULL UNIQUE,
            reservation_revision INTEGER NOT NULL CHECK(
                reservation_revision BETWEEN 1 AND 9007199254740991
            ),
            reservation_digest TEXT NOT NULL CHECK(length(reservation_digest)=64),
            capacity_claim_id TEXT NOT NULL,
            claim_revision INTEGER NOT NULL CHECK(claim_revision BETWEEN 1 AND 9007199254740991),
            claim_digest TEXT NOT NULL CHECK(length(claim_digest)=64),
            budget_reservation_id TEXT NOT NULL,
            budget_reserved_fen INTEGER NOT NULL CHECK(
                budget_reserved_fen BETWEEN 0 AND 9007199254740991
            ),
            broker_request_digest TEXT NOT NULL CHECK(length(broker_request_digest)=64),
            offer_id TEXT NOT NULL,
            offer_version INTEGER NOT NULL CHECK(offer_version BETWEEN 1 AND 9007199254740991),
            offer_digest TEXT NOT NULL CHECK(length(offer_digest)=64),
            lease_id TEXT NOT NULL UNIQUE CHECK(length(trim(lease_id)) BETWEEN 1 AND 160),
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            attempt_no INTEGER NOT NULL CHECK(attempt_no=1),
            shard_id TEXT,
            fencing_generation INTEGER NOT NULL CHECK(fencing_generation=1),
            execution_plan_id TEXT NOT NULL CHECK(
                length(trim(execution_plan_id)) BETWEEN 1 AND 160
            ),
            execution_plan_schema TEXT NOT NULL CHECK(
                length(trim(execution_plan_schema)) BETWEEN 1 AND 160
            ),
            execution_plan_digest TEXT NOT NULL CHECK(length(execution_plan_digest)=64),
            lease_credential_ref TEXT NOT NULL CHECK(
                length(trim(lease_credential_ref)) BETWEEN 1 AND 512
            ),
            lease_credential_hint TEXT NOT NULL CHECK(
                length(trim(lease_credential_hint)) BETWEEN 1 AND 160
            ),
            activation_idempotency_key TEXT NOT NULL CHECK(
                length(trim(activation_idempotency_key)) BETWEEN 1 AND 160
            ),
            activated_by_user_id TEXT NOT NULL CHECK(
                length(trim(activated_by_user_id)) BETWEEN 1 AND 160
            ),
            lease_expires_at TEXT NOT NULL,
            hard_deadline_at TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            not_after TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK(shard_id IS NULL OR length(trim(shard_id)) BETWEEN 1 AND 160),
            CHECK(
                (route_kind='provider_endpoint'
                    AND endpoint_id IS NOT NULL AND endpoint_transport IS NOT NULL
                    AND length(trim(endpoint_id)) BETWEEN 1 AND 160
                    AND length(trim(endpoint_transport)) BETWEEN 1 AND 80)
                OR (route_kind='server_adapter'
                    AND endpoint_id IS NULL AND endpoint_transport IS NULL)
            ),
            CHECK(provider_kind!='external_pool' OR route_kind='server_adapter'),
            CHECK(length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,5,1)='-' AND substr(issued_at,8,1)='-'
                AND substr(issued_at,11,1)='T' AND substr(issued_at,14,1)=':'
                AND substr(issued_at,17,1)=':'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(length(not_after)=30 AND substr(not_after,20,1)='.'
                AND substr(not_after,5,1)='-' AND substr(not_after,8,1)='-'
                AND substr(not_after,11,1)='T' AND substr(not_after,14,1)=':'
                AND substr(not_after,17,1)=':'
                AND substr(not_after,30,1)='Z' AND julianday(not_after) IS NOT NULL),
            CHECK(length(lease_expires_at)=30 AND substr(lease_expires_at,20,1)='.'
                AND substr(lease_expires_at,5,1)='-'
                AND substr(lease_expires_at,8,1)='-'
                AND substr(lease_expires_at,11,1)='T'
                AND substr(lease_expires_at,14,1)=':'
                AND substr(lease_expires_at,17,1)=':'
                AND substr(lease_expires_at,30,1)='Z' AND julianday(lease_expires_at) IS NOT NULL),
            CHECK(length(hard_deadline_at)=30 AND substr(hard_deadline_at,20,1)='.'
                AND substr(hard_deadline_at,5,1)='-'
                AND substr(hard_deadline_at,8,1)='-'
                AND substr(hard_deadline_at,11,1)='T'
                AND substr(hard_deadline_at,14,1)=':'
                AND substr(hard_deadline_at,17,1)=':'
                AND substr(hard_deadline_at,30,1)='Z' AND julianday(hard_deadline_at) IS NOT NULL),
            CHECK(length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,5,1)='-' AND substr(created_at,8,1)='-'
                AND substr(created_at,11,1)='T' AND substr(created_at,14,1)=':'
                AND substr(created_at,17,1)=':'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(issued_at < not_after AND issued_at < lease_expires_at),
            CHECK(not_after < lease_expires_at),
            CHECK(
                unixepoch(substr(lease_expires_at,1,19)||'Z')
                    - unixepoch(substr(not_after,1,19)||'Z') > 60
                OR (
                    unixepoch(substr(lease_expires_at,1,19)||'Z')
                        - unixepoch(substr(not_after,1,19)||'Z') = 60
                    AND substr(lease_expires_at,21,9) >= substr(not_after,21,9)
                )
            ),
            CHECK(lease_expires_at < hard_deadline_at),
            CHECK(issued_at <= created_at AND created_at < not_after),
            UNIQUE(job_id, attempt_no),
            UNIQUE(provider_id, activation_idempotency_key),
            FOREIGN KEY(provider_id, provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id, policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY(job_id, job_revision)
                REFERENCES compute_job_versions(job_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id, reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(capacity_claim_id, claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(budget_reservation_id)
                REFERENCES billing_reservations(id)
                ON DELETE RESTRICT
        );
        CREATE TABLE IF NOT EXISTS compute_attempt_dispatch_acks (
            ack_id TEXT PRIMARY KEY CHECK(length(trim(ack_id)) BETWEEN 1 AND 160),
            command_id TEXT NOT NULL UNIQUE,
            provider_id TEXT NOT NULL,
            adapter_id TEXT NOT NULL,
            adapter_ack_id TEXT NOT NULL CHECK(length(trim(adapter_ack_id)) BETWEEN 1 AND 160),
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            outcome TEXT NOT NULL CHECK(outcome IN ('accepted','rejected')),
            disposition TEXT NOT NULL CHECK(
                disposition IN ('accepted_applied','rejected','quarantined')
            ),
            disposition_reason_code TEXT,
            activation_lease_id TEXT,
            application_id TEXT UNIQUE,
            remote_execution_ref TEXT,
            reason_code TEXT,
            ack_json TEXT NOT NULL CHECK(
                json_valid(ack_json) AND length(CAST(ack_json AS BLOB)) <= 131072
            ),
            ack_digest TEXT NOT NULL UNIQUE CHECK(length(ack_digest)=64),
            observed_at TEXT NOT NULL,
            received_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            CHECK(length(observed_at)=30 AND substr(observed_at,20,1)='.'
                AND substr(observed_at,5,1)='-' AND substr(observed_at,8,1)='-'
                AND substr(observed_at,11,1)='T' AND substr(observed_at,14,1)=':'
                AND substr(observed_at,17,1)=':'
                AND substr(observed_at,30,1)='Z' AND julianday(observed_at) IS NOT NULL),
            CHECK(length(received_at)=30 AND substr(received_at,20,1)='.'
                AND substr(received_at,5,1)='-' AND substr(received_at,8,1)='-'
                AND substr(received_at,11,1)='T' AND substr(received_at,14,1)=':'
                AND substr(received_at,17,1)=':'
                AND substr(received_at,30,1)='Z' AND julianday(received_at) IS NOT NULL),
            CHECK(length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,5,1)='-' AND substr(created_at,8,1)='-'
                AND substr(created_at,11,1)='T' AND substr(created_at,14,1)=':'
                AND substr(created_at,17,1)=':'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(observed_at <= received_at AND received_at <= created_at),
            CHECK(
                (outcome='accepted' AND remote_execution_ref IS NOT NULL
                    AND length(trim(remote_execution_ref)) BETWEEN 1 AND 512
                    AND reason_code IS NULL)
                OR (outcome='rejected' AND remote_execution_ref IS NULL
                    AND reason_code IS NOT NULL
                    AND length(trim(reason_code)) BETWEEN 1 AND 160)
            ),
            CHECK(
                (outcome='rejected' AND disposition='rejected'
                    AND disposition_reason_code IS NULL AND activation_lease_id IS NULL
                    AND application_id IS NULL)
                OR (outcome='accepted' AND disposition='accepted_applied'
                    AND disposition_reason_code IS NULL AND activation_lease_id IS NOT NULL
                    AND application_id IS NOT NULL
                    AND application_id='attempt_dispatch_application_'||ack_digest)
                OR (outcome='accepted' AND disposition='quarantined'
                    AND activation_lease_id IS NULL AND application_id IS NULL
                    AND disposition_reason_code IS NOT NULL
                    AND length(trim(disposition_reason_code)) BETWEEN 1 AND 160)
            ),
            UNIQUE(provider_id, adapter_id, adapter_ack_id),
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
            FOREIGN KEY(activation_lease_id) REFERENCES compute_attempt_activations(lease_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(application_id)
                REFERENCES compute_attempt_dispatch_applications(application_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );
        CREATE TABLE IF NOT EXISTS compute_attempt_dispatch_applications (
            application_id TEXT PRIMARY KEY CHECK(
                length(trim(application_id)) BETWEEN 1 AND 160
            ),
            command_id TEXT NOT NULL UNIQUE,
            ack_id TEXT NOT NULL UNIQUE,
            action TEXT NOT NULL CHECK(action='v185_activate'),
            lease_id TEXT NOT NULL UNIQUE,
            activation_request_digest TEXT NOT NULL CHECK(length(activation_request_digest)=64),
            lease_digest TEXT NOT NULL CHECK(length(lease_digest)=64),
            application_json TEXT NOT NULL CHECK(
                json_valid(application_json)
                AND length(CAST(application_json AS BLOB)) <= 131072
            ),
            application_digest TEXT NOT NULL UNIQUE CHECK(length(application_digest)=64),
            applied_at TEXT NOT NULL,
            created_at TEXT NOT NULL CHECK(
                length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,5,1)='-' AND substr(created_at,8,1)='-'
                AND substr(created_at,11,1)='T' AND substr(created_at,14,1)=':'
                AND substr(created_at,17,1)=':'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL
            ),
            CHECK(julianday(applied_at) IS NOT NULL
                AND julianday(applied_at)<=julianday(created_at)),
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(ack_id) REFERENCES compute_attempt_dispatch_acks(ack_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id)
                ON DELETE RESTRICT
        );
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_dispatch_commands_route
            ON compute_attempt_dispatch_commands(provider_id, adapter_id, created_at, command_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_dispatch_commands_deadline
            ON compute_attempt_dispatch_commands(not_after, command_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_dispatch_acks_received
            ON compute_attempt_dispatch_acks(received_at, ack_id);
        "#,
    )?;
    triggers::install(conn)?;
    Ok(())
}
