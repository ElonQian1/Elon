use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_sealed_plan_v212
        BEFORE INSERT ON compute_attempt_dispatch_commands
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_execution_plans p
              JOIN compute_attempt_execution_plan_seals s
                ON s.plan_id=p.plan_id AND s.plan_digest=p.plan_digest
              JOIN compute_execution_capability_receipts cap
                ON cap.capability_id=p.capability_id
               AND cap.capability_digest=p.capability_digest
             WHERE p.plan_id=NEW.execution_plan_id
               AND p.plan_schema=NEW.execution_plan_schema
               AND p.plan_digest=NEW.execution_plan_digest
               AND s.capability_digest=p.capability_digest
               AND s.artifact_access_count=p.artifact_access_count
               AND s.artifact_access_set_digest=p.artifact_access_set_digest
               AND s.resource_grant_digest=p.resource_grant_digest
               AND p.provider_id=NEW.provider_id
               AND p.provider_kind=NEW.provider_kind
               AND p.provider_owner_account_id=NEW.activated_by_user_id
               AND p.provider_policy_revision=NEW.provider_policy_revision
               AND p.provider_digest=NEW.provider_digest
               AND p.offer_id=NEW.offer_id
               AND p.offer_version=NEW.offer_version
               AND p.offer_digest=NEW.offer_digest
               AND p.job_id=NEW.job_id
               AND p.job_revision=NEW.job_revision
               AND p.job_digest=NEW.job_digest
               AND p.reservation_id=NEW.reservation_id
               AND p.reservation_revision=NEW.reservation_revision
               AND p.reservation_digest=NEW.reservation_digest
               AND p.capacity_claim_id=NEW.capacity_claim_id
               AND p.claim_revision=NEW.claim_revision
               AND p.claim_digest=NEW.claim_digest
               AND p.budget_reservation_id=NEW.budget_reservation_id
               AND p.budget_reserved_fen=NEW.budget_reserved_fen
               AND p.broker_request_digest=NEW.broker_request_digest
               AND p.attempt_lease_id=NEW.lease_id
               AND p.attempt_no=NEW.attempt_no
               AND p.shard_id IS NEW.shard_id
               AND p.fencing_generation=NEW.fencing_generation
               AND p.executor_id=NEW.executor_id
               AND p.route_binding_digest=NEW.adapter_binding_digest
               AND cap.provider_id=NEW.provider_id
               AND cap.provider_kind=NEW.provider_kind
               AND cap.executor_id=NEW.executor_id
               AND cap.route_kind=NEW.route_kind
               AND cap.route_binding_digest=NEW.adapter_binding_digest
               AND cap.endpoint_id IS NEW.endpoint_id
               AND cap.endpoint_transport IS NEW.endpoint_transport
               AND cap.adapter_id=NEW.adapter_id
               AND cap.adapter_version=NEW.adapter_version
               AND cap.adapter_config_revision=NEW.adapter_config_revision
               AND cap.adapter_config_digest=NEW.adapter_config_digest
               AND cap.expires_at=p.capability_expires_at
               AND p.lease_expires_at=NEW.lease_expires_at
               AND p.hard_deadline_at=NEW.hard_deadline_at
               AND p.not_after=NEW.not_after
               AND p.planned_at<=NEW.issued_at
               AND s.sealed_at<=NEW.issued_at
               AND s.recorded_at<=NEW.created_at
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch requires exact sealed execution plan');
        END;
        "#,
    )?;
    Ok(())
}
