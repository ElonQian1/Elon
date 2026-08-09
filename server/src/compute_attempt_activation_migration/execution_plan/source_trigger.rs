use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plans_exact_source
        BEFORE INSERT ON compute_attempt_execution_plans
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers p
              JOIN compute_provider_versions pv
                ON pv.provider_id=p.provider_id
               AND pv.policy_revision=NEW.provider_policy_revision
              JOIN compute_provider_versions cpv
                ON cpv.provider_id=p.provider_id
               AND cpv.policy_revision=p.current_policy_revision
              JOIN compute_offers o ON o.offer_id=NEW.offer_id
              JOIN compute_offer_versions ov
                ON ov.offer_id=NEW.offer_id AND ov.offer_version=NEW.offer_version
              JOIN compute_jobs j ON j.job_id=NEW.job_id
              JOIN compute_job_versions jv
                ON jv.job_id=NEW.job_id AND jv.revision=NEW.job_revision
              JOIN compute_reservations r ON r.reservation_id=NEW.reservation_id
              JOIN compute_reservation_versions rv
                ON rv.reservation_id=NEW.reservation_id
               AND rv.revision=NEW.reservation_revision
              JOIN compute_capacity_claims c ON c.claim_id=NEW.capacity_claim_id
              JOIN compute_capacity_claim_versions cv
                ON cv.claim_id=NEW.capacity_claim_id AND cv.revision=NEW.claim_revision
              JOIN compute_price_snapshots ps ON ps.snapshot_id=NEW.price_snapshot_id
              JOIN compute_execution_capability_receipts cap
                ON cap.capability_id=NEW.capability_id
               AND cap.capability_digest=NEW.capability_digest
             WHERE p.provider_id=NEW.provider_id
               AND p.provider_kind=NEW.provider_kind
               AND p.owner_account_id=NEW.provider_owner_account_id
               AND p.status IN ('active','draining')
               AND pv.provider_digest=NEW.provider_digest
               AND json_extract(pv.provider_json,'$.provider_kind') IS NEW.provider_kind
               AND json_extract(pv.provider_json,'$.owner_account_id')
                    IS NEW.provider_owner_account_id
               AND o.provider_id=NEW.provider_id AND o.status IN ('active','draining')
               AND ov.offer_digest=NEW.offer_digest AND ov.provider_id=NEW.provider_id
               AND ov.provider_policy_revision=NEW.provider_policy_revision
               AND ov.provider_digest=NEW.provider_digest
               AND julianday(NEW.planned_at)>=julianday(ov.valid_from)
               AND julianday(NEW.hard_deadline_at)<=julianday(ov.valid_until)
               AND j.consumer_account_id=NEW.consumer_account_id
               AND j.current_revision=NEW.job_revision AND j.current_job_digest=NEW.job_digest
               AND j.status='reserved' AND j.selected_provider_id=NEW.provider_id
               AND j.selected_offer_id=NEW.offer_id AND j.selected_offer_version=NEW.offer_version
               AND j.selected_offer_digest=NEW.offer_digest
               AND j.price_snapshot_id=NEW.price_snapshot_id
               AND jv.job_digest=NEW.job_digest AND jv.status='reserved'
               AND jv.selected_provider_id=NEW.provider_id AND jv.selected_offer_id=NEW.offer_id
               AND jv.selected_offer_version=NEW.offer_version
               AND jv.selected_offer_digest=NEW.offer_digest
               AND jv.price_snapshot_id=NEW.price_snapshot_id
               AND julianday(NEW.hard_deadline_at)<=julianday(
                    json_extract(jv.job_json,'$.workload.deadline_at'))
               AND r.consumer_account_id=NEW.consumer_account_id AND r.job_id=NEW.job_id
               AND r.job_revision=NEW.job_revision AND r.job_digest=NEW.job_digest
               AND r.provider_id=NEW.provider_id AND r.offer_id=NEW.offer_id
               AND r.offer_version=NEW.offer_version AND r.offer_digest=NEW.offer_digest
               AND r.price_snapshot_id=NEW.price_snapshot_id
               AND r.capacity_claim_id=NEW.capacity_claim_id
               AND r.capacity_claim_revision=NEW.claim_revision
               AND r.capacity_claim_digest=NEW.claim_digest
               AND r.current_revision=NEW.reservation_revision
               AND r.current_reservation_digest=NEW.reservation_digest AND r.status='active'
               AND julianday(NEW.hard_deadline_at)<=julianday(r.expires_at)
               AND rv.reservation_digest=NEW.reservation_digest AND rv.status='active'
               AND rv.job_id=NEW.job_id AND rv.job_revision=NEW.job_revision
               AND rv.job_digest=NEW.job_digest AND rv.provider_id=NEW.provider_id
               AND rv.offer_id=NEW.offer_id AND rv.offer_version=NEW.offer_version
               AND rv.offer_digest=NEW.offer_digest AND rv.price_snapshot_id=NEW.price_snapshot_id
               AND rv.capacity_claim_id=NEW.capacity_claim_id
               AND rv.capacity_claim_revision=NEW.claim_revision
               AND rv.capacity_claim_digest=NEW.claim_digest
               AND c.revision=NEW.claim_revision AND c.claim_digest=NEW.claim_digest
               AND c.status='held' AND c.claim_kind='reservation'
               AND c.subject_kind='compute_reservation'
               AND c.subject_id=NEW.reservation_id
               AND (c.expires_at IS NULL OR julianday(NEW.hard_deadline_at)<=julianday(c.expires_at))
               AND cv.claim_digest=NEW.claim_digest AND cv.status='held'
               AND ps.snapshot_digest=NEW.price_snapshot_digest
               AND ps.provider_id=NEW.provider_id AND ps.offer_id=NEW.offer_id
               AND ps.offer_version=NEW.offer_version AND ps.offer_digest=NEW.offer_digest
               AND cap.provider_id=NEW.provider_id AND cap.provider_kind=NEW.provider_kind
               AND cap.executor_id=NEW.executor_id
               AND cap.route_binding_digest=NEW.route_binding_digest
               AND cap.capability_kind=NEW.capability_kind
               AND cap.expires_at=NEW.capability_expires_at
               AND cap.recorded_at<=NEW.planned_at AND NEW.hard_deadline_at<=cap.expires_at
               AND (
                    (cap.route_kind='provider_endpoint'
                        AND json_extract(cpv.provider_json,'$.endpoint.endpoint_id')
                            IS cap.endpoint_id
                        AND json_extract(cpv.provider_json,'$.endpoint.transport')
                            IS cap.endpoint_transport)
                    OR (cap.route_kind='server_adapter'
                        AND json_extract(cpv.provider_json,'$.adapter.adapter_id')
                            IS cap.adapter_id
                        AND json_extract(cpv.provider_json,'$.adapter.adapter_version')
                            IS cap.adapter_version
                        AND json_extract(cpv.provider_json,'$.adapter.config_revision')
                            IS cap.adapter_config_revision
                        AND json_extract(cpv.provider_json,'$.adapter.config_digest')
                            IS cap.adapter_config_digest)
               )
               AND EXISTS (SELECT 1 FROM compute_broker_reserve_receipts b
                    WHERE b.reservation_id=NEW.reservation_id
                      AND b.consumer_account_id=NEW.consumer_account_id
                      AND b.request_digest=NEW.broker_request_digest
                      AND b.budget_reservation_id=NEW.budget_reservation_id
                      AND b.budget_reserved_fen=NEW.budget_reserved_fen
                      AND b.capacity_claim_id=NEW.capacity_claim_id
                      AND b.capacity_claim_revision=NEW.claim_revision
                      AND b.capacity_claim_digest=NEW.claim_digest
                      AND b.job_id=NEW.job_id AND b.reserved_job_revision=NEW.job_revision
                      AND b.reserved_job_digest=NEW.job_digest
                      AND b.reservation_revision=NEW.reservation_revision
                      AND b.reservation_digest=NEW.reservation_digest)
               AND EXISTS (SELECT 1 FROM billing_reservations br
                    WHERE br.id=NEW.budget_reservation_id
                      AND br.user_id=NEW.consumer_account_id
                      AND br.reserved_fen=NEW.budget_reserved_fen AND br.status='reserved'
                      AND (br.expires_at IS NULL OR julianday(br.expires_at)
                            >=julianday(NEW.hard_deadline_at)))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt execution plan source mismatch');
        END;
        "#,
    )?;
    Ok(())
}
