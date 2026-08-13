use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};

use crate::{
    compute_attempt_activation_migration,
    compute_federation_broker_service::{self, control_plane_tests::BrokerFixture},
    store::{
        ActivateComputeAttemptRequest, ComputeAttemptLeaseStateReceipt,
        ComputeAttemptTerminalCandidateReceipt, ComputeAttemptUsageDeclarationReceipt,
        ComputeDeclaredUsageInput, DeclareComputeAttemptTerminalCandidateRequest,
        DeclareComputeAttemptUsageRequest, Store,
    },
};

use super::super::compute_attempt_leases::RenewComputeAttemptLeaseRequest;

pub(super) const USAGE_SEAL_TRIGGER: &str = "trg_compute_attempt_usage_declarations_terminal_seal";
pub(super) const CANDIDATE_HEAD_TRIGGER: &str =
    "trg_compute_attempt_terminal_candidates_final_usage_head";

pub(super) struct LiveAttemptFixture {
    pub broker: BrokerFixture,
    pub path: PathBuf,
    pub lease: ComputeAttemptLeaseStateReceipt,
}

impl LiveAttemptFixture {
    pub(super) fn new(label: &str) -> Self {
        let broker = BrokerFixture::new();
        broker
            .supply
            .store
            .billing_recharge(
                &broker.consumer_id,
                100,
                "final_usage_fence_test",
                &broker.supply.admin_id,
                None,
            )
            .unwrap();
        let quoted = broker.create_quoted_job(label);
        let reserved = compute_federation_broker_service::reserve_for_user(
            &broker.supply.store,
            &broker.consumer_id,
            Some(&broker.project_id),
            broker.reserve_request(&quoted, label, 20, 1),
        )
        .unwrap();
        let reservation = broker
            .supply
            .store
            .compute_reservation(&reserved.reservation_id)
            .unwrap();
        let reservation_expiry = parse_utc(&reservation.reservation.expires_at);
        let hard_deadline = reservation_expiry - Duration::seconds(5);
        let initial_expiry = hard_deadline - Duration::seconds(60);
        let renewed_expiry = hard_deadline - Duration::seconds(30);
        // The production Gateway still has no constructible Adapter. Seed only the v185
        // prerequisite, then immediately restore the v211 acceptance trigger before V226 runs.
        broker
            .supply
            .store
            .conn()
            .unwrap()
            .execute(
                "DROP TRIGGER trg_compute_attempt_activation_requires_dispatch_acceptance",
                [],
            )
            .unwrap();
        let activation = broker
            .supply
            .store
            .activate_compute_attempt(&ActivateComputeAttemptRequest {
                lease_id: format!("lease-{label}-{}", broker.consumer_id),
                reservation_id: reserved.reservation_id.clone(),
                provider_id: broker.supply.provider_id.clone(),
                executor_id: format!("executor-{label}"),
                shard_id: None,
                attempt_no: 1,
                fencing_generation: 1,
                executor_acceptance_ref: format!("acceptance://{label}"),
                lease_credential_ref: format!("vault://{label}"),
                lease_credential_hint: "test-only".into(),
                expected_job_revision: reserved.reserved_job.job_revision,
                expected_job_digest: reserved.reserved_job.job_digest.clone(),
                expected_reservation_revision: reserved.reservation_revision,
                expected_reservation_digest: reserved.reservation_digest.clone(),
                expected_claim_revision: reserved.capacity_claim.claim_revision,
                expected_claim_digest: reserved.capacity_claim.claim_digest.clone(),
                expires_at: initial_expiry.to_rfc3339(),
                hard_deadline_at: hard_deadline.to_rfc3339(),
                idempotency_key: format!("activate-{label}"),
                activated_by_user_id: broker.supply.owner_id.clone(),
            })
            .unwrap();
        compute_attempt_activation_migration::migration_v211(&broker.supply.store.conn().unwrap())
            .unwrap();
        let lease = broker
            .supply
            .store
            .renew_compute_attempt_lease(&RenewComputeAttemptLeaseRequest {
                lease_id: activation.lease.lease_id,
                provider_id: broker.supply.provider_id.clone(),
                expected_lease_revision: 1,
                expected_lease_digest: activation.lease_digest,
                expected_fencing_generation: 1,
                executor_heartbeat_ref: format!("heartbeat://{label}"),
                expires_at: renewed_expiry.to_rfc3339(),
                idempotency_key: format!("renew-{label}"),
                renewed_by_user_id: broker.supply.owner_id.clone(),
            })
            .unwrap()
            .state;
        let path = broker.supply.root.join("state.sqlite");
        Self {
            broker,
            path,
            lease,
        }
    }

    pub(super) fn open_peer(&self) -> Store {
        Store::open(&self.path).unwrap()
    }

    pub(super) fn usage_request(
        &self,
        sequence_no: i64,
        token_quantity: i64,
        key: &str,
    ) -> DeclareComputeAttemptUsageRequest {
        DeclareComputeAttemptUsageRequest {
            lease_id: self.lease.lease.lease_id.clone(),
            provider_id: self.broker.supply.provider_id.clone(),
            expected_lease_revision: self.lease.lease_revision,
            expected_lease_digest: self.lease.lease_digest.clone(),
            expected_fencing_generation: self.lease.lease.fencing_generation,
            sequence_no,
            executor_usage_ref: format!("usage://{key}"),
            cumulative_declared_usage: vec![
                ComputeDeclaredUsageInput {
                    meter: "tokens".into(),
                    cumulative_quantity: token_quantity,
                },
                ComputeDeclaredUsageInput {
                    meter: "concurrency".into(),
                    cumulative_quantity: 1,
                },
            ],
            idempotency_key: key.into(),
            declared_by_user_id: self.broker.supply.owner_id.clone(),
        }
    }

    pub(super) fn declare_usage(
        &self,
        sequence_no: i64,
        token_quantity: i64,
        key: &str,
    ) -> ComputeAttemptUsageDeclarationReceipt {
        self.broker
            .supply
            .store
            .declare_compute_attempt_usage(&self.usage_request(sequence_no, token_quantity, key))
            .unwrap()
    }

    pub(super) fn candidate_request(
        &self,
        usage: &ComputeAttemptUsageDeclarationReceipt,
        key: &str,
    ) -> DeclareComputeAttemptTerminalCandidateRequest {
        DeclareComputeAttemptTerminalCandidateRequest {
            lease_id: self.lease.lease.lease_id.clone(),
            provider_id: self.broker.supply.provider_id.clone(),
            expected_lease_revision: self.lease.lease_revision,
            expected_lease_digest: self.lease.lease_digest.clone(),
            expected_fencing_generation: self.lease.lease.fencing_generation,
            final_usage_snapshot_id: usage.snapshot_id.clone(),
            final_usage_sequence_no: usage.sequence_no,
            final_cumulative_usage_digest: usage.cumulative_usage_digest.clone(),
            executor_terminal_ref: format!("terminal://{key}"),
            outcome: "failed".into(),
            reason_code: "test.failure".into(),
            diagnostic_ref: Some("diagnostic://test".into()),
            output_digest: None,
            result_artifacts: Vec::new(),
            idempotency_key: key.into(),
            declared_by_user_id: self.broker.supply.owner_id.clone(),
        }
    }

    pub(super) fn declare_candidate(
        &self,
        usage: &ComputeAttemptUsageDeclarationReceipt,
        key: &str,
    ) -> ComputeAttemptTerminalCandidateReceipt {
        self.broker
            .supply
            .store
            .declare_compute_attempt_terminal_candidate(&self.candidate_request(usage, key))
            .unwrap()
    }
}

pub(super) fn trigger_count(path: &PathBuf, name: &str) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![name],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn drop_final_usage_triggers(connection: &Connection) {
    connection
        .execute_batch(&format!(
            "DROP TRIGGER IF EXISTS {USAGE_SEAL_TRIGGER};\
             DROP TRIGGER IF EXISTS {CANDIDATE_HEAD_TRIGGER};"
        ))
        .unwrap();
}

pub(super) fn insert_drifted_usage(connection: &Connection, lease_id: &str) {
    connection
        .execute(
            "INSERT INTO compute_attempt_usage_declarations (
                snapshot_id, lease_id, provider_id, consumer_account_id,
                sequence_no, source_lease_revision, source_lease_digest,
                source_lease_status, fencing_generation, job_id, job_revision,
                job_digest, reservation_id, reservation_revision,
                reservation_digest, capacity_claim_id, capacity_claim_revision,
                capacity_claim_digest, executor_usage_ref, cumulative_usage_json,
                cumulative_usage_digest, reserved_contract_json,
                reserved_contract_digest, overage_meters_json, request_digest,
                event_digest, idempotency_scope, idempotency_key,
                declared_by_user_id, declared_at, created_at
             )
             SELECT 'drifted-usage', lease_id, provider_id, consumer_account_id,
                    sequence_no + 1, source_lease_revision, source_lease_digest,
                    source_lease_status, fencing_generation, job_id, job_revision,
                    job_digest, reservation_id, reservation_revision,
                    reservation_digest, capacity_claim_id, capacity_claim_revision,
                    capacity_claim_digest, 'usage://drift', cumulative_usage_json,
                    cumulative_usage_digest, reserved_contract_json,
                    reserved_contract_digest, overage_meters_json, ?2, ?3,
                    idempotency_scope, 'usage-drift', declared_by_user_id,
                    declared_at, created_at
               FROM compute_attempt_usage_declarations
              WHERE lease_id=?1 AND sequence_no=1",
            params![lease_id, "d".repeat(64), "e".repeat(64)],
        )
        .unwrap();
}

fn parse_utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
