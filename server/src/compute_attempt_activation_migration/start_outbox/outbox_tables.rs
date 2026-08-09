use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_attempt_start_outbox (
            outbox_id TEXT PRIMARY KEY CHECK(length(trim(outbox_id)) BETWEEN 1 AND 160),
            outbox_schema TEXT NOT NULL CHECK(
                outbox_schema='compute_federation.attempt_start_outbox.v1'
            ),
            outbox_digest TEXT NOT NULL UNIQUE CHECK(
                length(outbox_digest)=64 AND outbox_digest NOT GLOB '*[^0-9a-f]*'
            ),
            outbox_json TEXT NOT NULL CHECK(
                json_valid(outbox_json) AND length(CAST(outbox_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            operation_kind TEXT NOT NULL CHECK(
                operation_kind IN ('prepare','commit','cancel','reconcile')
            ),
            operation_generation INTEGER NOT NULL CHECK(
                operation_generation BETWEEN 1 AND 9007199254740991
            ),
            subject_outbox_id TEXT,
            command_id TEXT NOT NULL,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            provider_id TEXT NOT NULL,
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            route_authorization_id TEXT NOT NULL,
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            actor_receipt_id TEXT NOT NULL,
            actor_receipt_digest TEXT NOT NULL CHECK(length(actor_receipt_digest)=64),
            plan_id TEXT NOT NULL,
            plan_digest TEXT NOT NULL CHECK(length(plan_digest)=64),
            lease_id TEXT NOT NULL,
            fencing_generation INTEGER NOT NULL CHECK(
                fencing_generation BETWEEN 1 AND 9007199254740991
            ),
            ack_id TEXT,
            ack_digest TEXT,
            application_id TEXT,
            application_digest TEXT,
            lease_authority_id TEXT,
            lease_authority_revision INTEGER,
            lease_authority_digest TEXT,
            issued_at TEXT NOT NULL,
            not_before TEXT NOT NULL,
            not_after TEXT NOT NULL,
            state TEXT NOT NULL CHECK(state IN (
                'blocked','pending','claimed','in_flight_unknown',
                'delivery_observed','abandoned_no_send','quarantined'
            )),
            state_revision INTEGER NOT NULL CHECK(
                state_revision BETWEEN 1 AND 9007199254740991
            ),
            attempt_count INTEGER NOT NULL CHECK(
                attempt_count BETWEEN 0 AND 9007199254740991
            ),
            next_attempt_at TEXT NOT NULL,
            claim_owner_id TEXT,
            claim_token_digest TEXT,
            claim_generation INTEGER NOT NULL CHECK(
                claim_generation BETWEEN 0 AND 9007199254740991
            ),
            claim_expires_at TEXT,
            last_failure_code TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(command_id, operation_kind, operation_generation),
            CHECK(command_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND actor_receipt_digest NOT GLOB '*[^0-9a-f]*'
                AND plan_digest NOT GLOB '*[^0-9a-f]*'
                AND (ack_digest IS NULL OR ack_digest NOT GLOB '*[^0-9a-f]*')
                AND (application_digest IS NULL
                    OR application_digest NOT GLOB '*[^0-9a-f]*')
                AND (lease_authority_digest IS NULL
                    OR lease_authority_digest NOT GLOB '*[^0-9a-f]*')
                AND (claim_token_digest IS NULL
                    OR claim_token_digest NOT GLOB '*[^0-9a-f]*')),
            CHECK(
                (operation_kind='prepare' AND operation_generation=1
                    AND subject_outbox_id IS NULL AND ack_id IS NULL AND ack_digest IS NULL
                    AND application_id IS NULL AND application_digest IS NULL
                    AND lease_authority_id IS NULL AND lease_authority_revision IS NULL
                    AND lease_authority_digest IS NULL)
                OR (operation_kind='commit' AND subject_outbox_id IS NOT NULL
                    AND ack_id IS NOT NULL AND ack_digest IS NOT NULL
                    AND application_id IS NOT NULL AND application_digest IS NOT NULL
                    AND lease_authority_id IS NOT NULL
                    AND lease_authority_revision IS NOT NULL
                    AND lease_authority_digest IS NOT NULL
                    AND length(trim(ack_id)) BETWEEN 1 AND 160 AND length(ack_digest)=64
                    AND length(trim(application_id)) BETWEEN 1 AND 160
                    AND length(application_digest)=64
                    AND length(trim(lease_authority_id)) BETWEEN 1 AND 160
                    AND lease_authority_revision BETWEEN 1 AND 9007199254740991
                    AND length(lease_authority_digest)=64)
                OR (operation_kind IN ('cancel','reconcile')
                    AND subject_outbox_id IS NOT NULL
                    AND ((ack_id IS NULL AND ack_digest IS NULL)
                        OR (ack_id IS NOT NULL AND ack_digest IS NOT NULL
                            AND length(trim(ack_id)) BETWEEN 1 AND 160
                            AND length(ack_digest)=64))
                    AND application_id IS NULL AND application_digest IS NULL
                    AND lease_authority_id IS NULL AND lease_authority_revision IS NULL
                    AND lease_authority_digest IS NULL)
            ),
            CHECK(state!='blocked' OR operation_kind='reconcile'),
            CHECK(state!='abandoned_no_send' OR operation_kind='prepare'),
            CHECK(
                (state IN ('claimed','in_flight_unknown')
                    AND claim_owner_id IS NOT NULL AND claim_token_digest IS NOT NULL
                    AND length(trim(claim_owner_id)) BETWEEN 1 AND 160
                    AND length(claim_token_digest)=64
                    AND claim_generation>0 AND claim_expires_at IS NOT NULL)
                OR (state NOT IN ('claimed','in_flight_unknown')
                    AND claim_owner_id IS NULL AND claim_token_digest IS NULL
                    AND claim_expires_at IS NULL)
            ),
            CHECK(length(next_attempt_at)=30 AND substr(next_attempt_at,20,1)='.'
                AND substr(next_attempt_at,30,1)='Z'
                AND julianday(next_attempt_at) IS NOT NULL),
            CHECK(claim_expires_at IS NULL OR (length(claim_expires_at)=30
                AND substr(claim_expires_at,20,1)='.' AND substr(claim_expires_at,30,1)='Z'
                AND julianday(claim_expires_at) IS NOT NULL)),
            CHECK(last_failure_code IS NULL OR
                length(trim(last_failure_code)) BETWEEN 1 AND 160),
            CHECK(length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(length(not_before)=30 AND substr(not_before,20,1)='.'
                AND substr(not_before,30,1)='Z' AND julianday(not_before) IS NOT NULL),
            CHECK(length(not_after)=30 AND substr(not_after,20,1)='.'
                AND substr(not_after,30,1)='Z' AND julianday(not_after) IS NOT NULL),
            CHECK(length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(length(updated_at)=30 AND substr(updated_at,20,1)='.'
                AND substr(updated_at,30,1)='Z' AND julianday(updated_at) IS NOT NULL),
            CHECK(issued_at<=created_at AND created_at=updated_at
                AND not_before<=not_after),
            FOREIGN KEY(subject_outbox_id) REFERENCES compute_attempt_start_outbox(outbox_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(actor_receipt_id)
                REFERENCES compute_attempt_dispatch_actor_receipts(actor_receipt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(plan_id) REFERENCES compute_attempt_execution_plans(plan_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(ack_id) REFERENCES compute_attempt_dispatch_acks(ack_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(application_id)
                REFERENCES compute_attempt_dispatch_applications(application_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(lease_authority_id, lease_authority_revision)
                REFERENCES compute_attempt_lease_authority_bindings(
                    lease_authority_id, authority_revision
                ) ON DELETE RESTRICT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_attempt_start_outbox_prepare
            ON compute_attempt_start_outbox(command_id) WHERE operation_kind='prepare';
        CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_attempt_start_outbox_commit
            ON compute_attempt_start_outbox(application_id) WHERE operation_kind='commit';
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_start_outbox_claimable
            ON compute_attempt_start_outbox(state, next_attempt_at, not_before, outbox_id);

        CREATE TABLE IF NOT EXISTS compute_attempt_start_send_attempts (
            send_attempt_id TEXT PRIMARY KEY CHECK(
                length(trim(send_attempt_id)) BETWEEN 1 AND 160
            ),
            send_attempt_schema TEXT NOT NULL CHECK(
                send_attempt_schema='compute_federation.attempt_start_send_attempt.v1'
            ),
            send_attempt_digest TEXT NOT NULL UNIQUE CHECK(
                length(send_attempt_digest)=64
                AND send_attempt_digest NOT GLOB '*[^0-9a-f]*'
            ),
            send_attempt_json TEXT NOT NULL CHECK(
                json_valid(send_attempt_json)
                AND length(CAST(send_attempt_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            outbox_id TEXT NOT NULL,
            outbox_digest TEXT NOT NULL CHECK(length(outbox_digest)=64),
            attempt_no INTEGER NOT NULL CHECK(
                attempt_no BETWEEN 1 AND 9007199254740991
            ),
            operation_kind TEXT NOT NULL CHECK(
                operation_kind IN ('prepare','commit','cancel','reconcile')
            ),
            command_id TEXT NOT NULL,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            route_authorization_id TEXT NOT NULL,
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            claim_generation INTEGER NOT NULL CHECK(
                claim_generation BETWEEN 1 AND 9007199254740991
            ),
            claim_token_digest TEXT NOT NULL CHECK(length(claim_token_digest)=64),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64 AND request_digest NOT GLOB '*[^0-9a-f]*'
            ),
            started_at TEXT NOT NULL,
            UNIQUE(outbox_id, attempt_no),
            CHECK(outbox_digest NOT GLOB '*[^0-9a-f]*'
                AND command_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND claim_token_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(length(started_at)=30 AND substr(started_at,20,1)='.'
                AND substr(started_at,30,1)='Z' AND julianday(started_at) IS NOT NULL),
            FOREIGN KEY(outbox_id) REFERENCES compute_attempt_start_outbox(outbox_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_start_remote_observations (
            observation_id TEXT PRIMARY KEY CHECK(
                length(trim(observation_id)) BETWEEN 1 AND 160
            ),
            observation_schema TEXT NOT NULL CHECK(
                observation_schema='compute_federation.attempt_start_remote_observation.v1'
            ),
            observation_digest TEXT NOT NULL UNIQUE CHECK(
                length(observation_digest)=64
                AND observation_digest NOT GLOB '*[^0-9a-f]*'
            ),
            observation_json TEXT NOT NULL CHECK(
                json_valid(observation_json)
                AND length(CAST(observation_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            send_attempt_id TEXT NOT NULL,
            outbox_id TEXT NOT NULL,
            outbox_digest TEXT NOT NULL CHECK(length(outbox_digest)=64),
            operation_kind TEXT NOT NULL CHECK(
                operation_kind IN ('prepare','commit','cancel','reconcile')
            ),
            observation_kind TEXT NOT NULL CHECK(observation_kind IN (
                'prepare_response','commit_response','cancel_response','reconcile_attestation'
            )),
            command_id TEXT NOT NULL,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            provider_id TEXT NOT NULL,
            adapter_id TEXT NOT NULL,
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            adapter_observation_id TEXT NOT NULL CHECK(
                length(trim(adapter_observation_id)) BETWEEN 1 AND 160
            ),
            response_outcome TEXT NOT NULL CHECK(
                response_outcome IN ('accepted','rejected','observed','unknown')
            ),
            remote_execution_state TEXT NOT NULL CHECK(remote_execution_state IN (
                'absent','prepared','committed','running','terminal_no_start',
                'terminal_after_run','unknown','rejected'
            )),
            terminality TEXT NOT NULL CHECK(terminality IN ('non_terminal','final')),
            remote_execution_ref TEXT,
            remote_sequence INTEGER NOT NULL CHECK(
                remote_sequence BETWEEN 0 AND 9007199254740991
            ),
            no_commit_tombstone_id TEXT,
            no_commit_tombstone_digest TEXT,
            reason_code TEXT,
            verification_kind TEXT NOT NULL CHECK(
                length(trim(verification_kind)) BETWEEN 1 AND 80
            ),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verification_digest TEXT NOT NULL CHECK(length(verification_digest)=64),
            authenticated_at TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            received_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(provider_id, adapter_id, adapter_observation_id),
            UNIQUE(send_attempt_id, observation_id),
            CHECK(outbox_digest NOT GLOB '*[^0-9a-f]*'
                AND command_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND verification_digest NOT GLOB '*[^0-9a-f]*'
                AND (no_commit_tombstone_digest IS NULL
                    OR no_commit_tombstone_digest NOT GLOB '*[^0-9a-f]*')),
            CHECK(
                (remote_execution_state='terminal_no_start' AND terminality='final'
                    AND observation_kind='reconcile_attestation'
                    AND no_commit_tombstone_id IS NOT NULL
                    AND no_commit_tombstone_digest IS NOT NULL
                    AND length(trim(no_commit_tombstone_id)) BETWEEN 1 AND 160
                    AND length(no_commit_tombstone_digest)=64)
                OR (remote_execution_state!='terminal_no_start'
                    AND no_commit_tombstone_id IS NULL
                    AND no_commit_tombstone_digest IS NULL)
            ),
            CHECK(observation_kind!='cancel_response'
                OR (terminality='non_terminal' AND remote_execution_state!='terminal_no_start')),
            CHECK(remote_execution_ref IS NULL OR
                length(trim(remote_execution_ref)) BETWEEN 1 AND 512),
            CHECK(reason_code IS NULL OR length(trim(reason_code)) BETWEEN 1 AND 160),
            CHECK(length(authenticated_at)=30 AND substr(authenticated_at,20,1)='.'
                AND substr(authenticated_at,30,1)='Z' AND julianday(authenticated_at) IS NOT NULL),
            CHECK(length(observed_at)=30 AND substr(observed_at,20,1)='.'
                AND substr(observed_at,30,1)='Z' AND julianday(observed_at) IS NOT NULL),
            CHECK(length(received_at)=30 AND substr(received_at,20,1)='.'
                AND substr(received_at,30,1)='Z' AND julianday(received_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(authenticated_at<=observed_at AND observed_at<=received_at
                AND received_at<=recorded_at),
            FOREIGN KEY(send_attempt_id)
                REFERENCES compute_attempt_start_send_attempts(send_attempt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(outbox_id) REFERENCES compute_attempt_start_outbox(outbox_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_no_start_proofs (
            proof_id TEXT PRIMARY KEY CHECK(length(trim(proof_id)) BETWEEN 1 AND 160),
            proof_schema TEXT NOT NULL CHECK(
                proof_schema='compute_federation.attempt_no_start_proof.v1'
            ),
            proof_digest TEXT NOT NULL UNIQUE CHECK(
                length(proof_digest)=64 AND proof_digest NOT GLOB '*[^0-9a-f]*'
            ),
            proof_json TEXT NOT NULL CHECK(
                json_valid(proof_json) AND length(CAST(proof_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            proof_kind TEXT NOT NULL CHECK(proof_kind IN (
                'local_never_sent','prepare_rejected','remote_never_committed'
            )),
            outbox_id TEXT NOT NULL,
            outbox_digest TEXT NOT NULL CHECK(length(outbox_digest)=64),
            command_id TEXT NOT NULL UNIQUE,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            plan_id TEXT NOT NULL,
            plan_digest TEXT NOT NULL CHECK(length(plan_digest)=64),
            provider_id TEXT NOT NULL,
            reservation_id TEXT NOT NULL UNIQUE,
            reservation_revision INTEGER NOT NULL CHECK(reservation_revision>0),
            reservation_digest TEXT NOT NULL CHECK(length(reservation_digest)=64),
            job_id TEXT NOT NULL,
            job_revision INTEGER NOT NULL CHECK(job_revision>0),
            job_digest TEXT NOT NULL CHECK(length(job_digest)=64),
            capacity_claim_id TEXT NOT NULL,
            capacity_claim_revision INTEGER NOT NULL CHECK(capacity_claim_revision>0),
            capacity_claim_digest TEXT NOT NULL CHECK(length(capacity_claim_digest)=64),
            budget_reservation_id TEXT NOT NULL,
            budget_reserved_fen INTEGER NOT NULL CHECK(
                budget_reserved_fen BETWEEN 0 AND 9007199254740991
            ),
            broker_request_digest TEXT NOT NULL CHECK(length(broker_request_digest)=64),
            lease_id TEXT NOT NULL UNIQUE,
            lease_digest TEXT CHECK(lease_digest IS NULL),
            fencing_generation INTEGER NOT NULL CHECK(
                fencing_generation BETWEEN 1 AND 9007199254740991
            ),
            adapter_id TEXT NOT NULL,
            adapter_revision INTEGER NOT NULL CHECK(adapter_revision>0),
            adapter_registry_digest TEXT NOT NULL CHECK(length(adapter_registry_digest)=64),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            route_authorization_id TEXT NOT NULL,
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            observation_id TEXT,
            observation_digest TEXT,
            no_commit_tombstone_id TEXT,
            no_commit_tombstone_digest TEXT,
            proven_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK(outbox_digest NOT GLOB '*[^0-9a-f]*'
                AND command_digest NOT GLOB '*[^0-9a-f]*'
                AND plan_digest NOT GLOB '*[^0-9a-f]*'
                AND reservation_digest NOT GLOB '*[^0-9a-f]*'
                AND job_digest NOT GLOB '*[^0-9a-f]*'
                AND capacity_claim_digest NOT GLOB '*[^0-9a-f]*'
                AND broker_request_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_registry_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND (observation_digest IS NULL
                    OR observation_digest NOT GLOB '*[^0-9a-f]*')
                AND (no_commit_tombstone_digest IS NULL
                    OR no_commit_tombstone_digest NOT GLOB '*[^0-9a-f]*')),
            CHECK(
                (proof_kind='local_never_sent' AND observation_id IS NULL
                    AND observation_digest IS NULL AND no_commit_tombstone_id IS NULL
                    AND no_commit_tombstone_digest IS NULL)
                OR (proof_kind='prepare_rejected'
                    AND observation_id IS NOT NULL AND observation_digest IS NOT NULL
                    AND length(trim(observation_id)) BETWEEN 1 AND 160
                    AND length(observation_digest)=64 AND no_commit_tombstone_id IS NULL
                    AND no_commit_tombstone_digest IS NULL)
                OR (proof_kind='remote_never_committed'
                    AND observation_id IS NOT NULL AND observation_digest IS NOT NULL
                    AND no_commit_tombstone_id IS NOT NULL
                    AND no_commit_tombstone_digest IS NOT NULL
                    AND length(trim(observation_id)) BETWEEN 1 AND 160
                    AND length(observation_digest)=64
                    AND length(trim(no_commit_tombstone_id)) BETWEEN 1 AND 160
                    AND length(no_commit_tombstone_digest)=64)
            ),
            CHECK(length(proven_at)=30 AND substr(proven_at,20,1)='.'
                AND substr(proven_at,30,1)='Z' AND julianday(proven_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(proven_at<=recorded_at),
            FOREIGN KEY(outbox_id) REFERENCES compute_attempt_start_outbox(outbox_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(plan_id) REFERENCES compute_attempt_execution_plans(plan_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
            FOREIGN KEY(capacity_claim_id) REFERENCES compute_capacity_claims(claim_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(budget_reservation_id) REFERENCES billing_reservations(id)
                ON DELETE RESTRICT,
            FOREIGN KEY(observation_id)
                REFERENCES compute_attempt_start_remote_observations(observation_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(adapter_id, adapter_revision)
                REFERENCES compute_route_adapter_versions(adapter_id, adapter_revision)
                ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
