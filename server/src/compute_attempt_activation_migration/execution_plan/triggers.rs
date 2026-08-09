use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_execution_capability_receipts_projection
        BEFORE INSERT ON compute_execution_capability_receipts
        WHEN json_extract(NEW.capability_json,'$.schema') IS NOT NEW.capability_schema
          OR json_extract(NEW.capability_json,'$.capability_id') IS NOT NEW.capability_id
          OR json_extract(NEW.capability_json,'$.capability_digest') IS NOT NEW.capability_digest
          OR json_extract(NEW.capability_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.capability_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.capability_json,'$.capability.capability_kind')
                IS NOT NEW.capability_kind
          OR json_extract(NEW.capability_json,'$.capability.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.capability_json,'$.capability.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.capability_json,'$.capability.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.capability_json,'$.capability.route.route_kind')
                IS NOT NEW.route_kind
          OR json_extract(NEW.capability_json,'$.capability.route.route_binding_digest')
                IS NOT NEW.route_binding_digest
          OR json_type(NEW.capability_json,'$.capability.route.endpoint_id') IS NULL
          OR json_extract(NEW.capability_json,'$.capability.route.endpoint_id')
                IS NOT NEW.endpoint_id
          OR json_type(NEW.capability_json,'$.capability.route.endpoint_transport') IS NULL
          OR json_extract(NEW.capability_json,'$.capability.route.endpoint_transport')
                IS NOT NEW.endpoint_transport
          OR json_extract(NEW.capability_json,'$.capability.route.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.capability_json,'$.capability.route.adapter_version')
                IS NOT NEW.adapter_version
          OR json_extract(NEW.capability_json,'$.capability.route.adapter_config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.capability_json,'$.capability.route.adapter_config_digest')
                IS NOT NEW.adapter_config_digest
          OR json_extract(NEW.capability_json,'$.capability.provenance.source_schema')
                IS NOT NEW.source_schema
          OR json_extract(NEW.capability_json,'$.capability.provenance.source_id')
                IS NOT NEW.source_id
          OR json_extract(NEW.capability_json,'$.capability.provenance.source_digest')
                IS NOT NEW.source_digest
          OR json_extract(NEW.capability_json,'$.capability.provenance.verification_kind')
                IS NOT NEW.verification_kind
          OR json_extract(NEW.capability_json,'$.capability.provenance.verifier_id')
                IS NOT NEW.verifier_id
          OR json_extract(NEW.capability_json,'$.capability.provenance.verification_digest')
                IS NOT NEW.verification_digest
          OR json_extract(NEW.capability_json,'$.capability.provenance.authenticated_at')
                IS NOT NEW.authenticated_at
          OR (NEW.capability_kind='node_ready' AND (
                json_type(NEW.capability_json,'$.capability.node_ready.installation_identity_digest') IS NOT 'text'
                OR length(json_extract(NEW.capability_json,
                    '$.capability.node_ready.installation_identity_digest')) IS NOT 64
                OR json_extract(NEW.capability_json,'$.capability.node_ready.installation_identity_digest') GLOB '*[^0-9a-f]*'
                OR json_type(NEW.capability_json,'$.capability.node_ready.inventory_revision') IS NOT 'integer'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.inventory_revision') NOT BETWEEN 1 AND 9007199254740991
                OR json_type(NEW.capability_json,'$.capability.node_ready.install_generation') IS NOT 'integer'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.install_generation') NOT BETWEEN 1 AND 9007199254740991
                OR json_type(NEW.capability_json,'$.capability.node_ready.activation_generation') IS NOT 'integer'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.activation_generation') NOT BETWEEN 1 AND 9007199254740991
                OR json_type(NEW.capability_json,'$.capability.node_ready.runtime_generation') IS NOT 'integer'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.runtime_generation') NOT BETWEEN 1 AND 9007199254740991
                OR json_type(NEW.capability_json,'$.capability.node_ready.slot_ref') IS NOT 'text'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.slot_ref') IS NOT trim(json_extract(NEW.capability_json,'$.capability.node_ready.slot_ref'))
                OR length(trim(json_extract(NEW.capability_json,'$.capability.node_ready.slot_ref'))) NOT BETWEEN 1 AND 160
                OR json_type(NEW.capability_json,'$.capability.node_ready.evidence_ref') IS NOT 'text'
                OR json_extract(NEW.capability_json,'$.capability.node_ready.evidence_ref') IS NOT trim(json_extract(NEW.capability_json,'$.capability.node_ready.evidence_ref'))
                OR length(trim(json_extract(NEW.capability_json,'$.capability.node_ready.evidence_ref'))) NOT BETWEEN 1 AND 512))
          OR json_extract(NEW.capability_json,'$.capability.observed_at') IS NOT NEW.observed_at
          OR json_extract(NEW.capability_json,'$.capability.expires_at') IS NOT NEW.expires_at
        BEGIN
            SELECT RAISE(ABORT, 'compute execution capability projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_artifact_access_receipts_projection
        BEFORE INSERT ON compute_artifact_access_receipts
        WHEN json_extract(NEW.access_json,'$.schema') IS NOT NEW.access_schema
          OR json_extract(NEW.access_json,'$.access_id') IS NOT NEW.access_id
          OR json_extract(NEW.access_json,'$.access_digest') IS NOT NEW.access_digest
          OR json_extract(NEW.access_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.access_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.access_json,'$.access.non_bearer_access_ref')
                IS NOT NEW.non_bearer_access_ref
          OR json_extract(NEW.access_json,'$.access.authorization_digest')
                IS NOT NEW.authorization_digest
          OR json_extract(NEW.access_json,'$.access.audience.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.access_json,'$.access.audience.reservation_id')
                IS NOT NEW.reservation_id
          OR json_extract(NEW.access_json,'$.access.audience.attempt_lease_id')
                IS NOT NEW.attempt_lease_id
          OR json_extract(NEW.access_json,'$.access.audience.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.access_json,'$.access.audience.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.access_json,'$.access.audience.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.access_json,'$.access.audience.route_binding_digest')
                IS NOT NEW.route_binding_digest
          OR json_extract(NEW.access_json,'$.access.target.access_kind') IS NOT NEW.access_kind
          OR (NEW.access_kind='read' AND (
                json_extract(NEW.access_json,'$.access.target.target.artifact_id')
                    IS NOT NEW.target_id
                OR json_extract(NEW.access_json,'$.access.target.target.artifact_digest')
                    IS NOT NEW.target_digest
                OR json_extract(NEW.access_json,'$.access.target.target.media_type')
                    IS NOT NEW.media_type
                OR json_extract(NEW.access_json,'$.access.target.target.size_bytes')
                    IS NOT NEW.size_limit_bytes))
          OR (NEW.access_kind='write' AND (
                json_extract(NEW.access_json,'$.access.target.target.namespace_id')
                    IS NOT NEW.target_id
                OR json_extract(NEW.access_json,'$.access.target.target.namespace_digest')
                    IS NOT NEW.target_digest
                OR json_extract(NEW.access_json,'$.access.target.target.media_type')
                    IS NOT NEW.media_type
                OR json_extract(NEW.access_json,'$.access.target.target.max_bytes')
                    IS NOT NEW.size_limit_bytes))
          OR json_extract(NEW.access_json,'$.access.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.access_json,'$.access.expires_at') IS NOT NEW.expires_at
        BEGIN
            SELECT RAISE(ABORT, 'compute artifact access projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plans_projection
        BEFORE INSERT ON compute_attempt_execution_plans
        WHEN json_extract(NEW.plan_json,'$.schema') IS NOT NEW.plan_schema
          OR json_extract(NEW.plan_json,'$.plan_id') IS NOT NEW.plan_id
          OR json_extract(NEW.plan_json,'$.plan_digest') IS NOT NEW.plan_digest
          OR json_extract(NEW.plan_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.plan_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.plan_json,'$.plan.sources.consumer_account_id')
                IS NOT NEW.consumer_account_id
          OR json_extract(NEW.plan_json,'$.plan.sources.provider.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.plan_json,'$.plan.sources.provider.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.plan_json,'$.plan.sources.provider.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.plan_json,'$.plan.sources.provider.policy_revision')
                IS NOT NEW.provider_policy_revision
          OR json_extract(NEW.plan_json,'$.plan.sources.provider.provider_digest')
                IS NOT NEW.provider_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.offer.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.plan_json,'$.plan.sources.offer.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.plan_json,'$.plan.sources.offer.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.plan_json,'$.plan.sources.offer.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.job.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.plan_json,'$.plan.sources.job.job_revision') IS NOT NEW.job_revision
          OR json_extract(NEW.plan_json,'$.plan.sources.job.job_digest') IS NOT NEW.job_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.reservation.reservation_id')
                IS NOT NEW.reservation_id
          OR json_extract(NEW.plan_json,'$.plan.sources.reservation.reservation_revision')
                IS NOT NEW.reservation_revision
          OR json_extract(NEW.plan_json,'$.plan.sources.reservation.reservation_digest')
                IS NOT NEW.reservation_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.capacity_claim.claim_id')
                IS NOT NEW.capacity_claim_id
          OR json_extract(NEW.plan_json,'$.plan.sources.capacity_claim.claim_revision')
                IS NOT NEW.claim_revision
          OR json_extract(NEW.plan_json,'$.plan.sources.capacity_claim.claim_digest')
                IS NOT NEW.claim_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.price_snapshot.price_snapshot_id')
                IS NOT NEW.price_snapshot_id
          OR json_extract(NEW.plan_json,'$.plan.sources.price_snapshot.price_snapshot_digest')
                IS NOT NEW.price_snapshot_digest
          OR json_extract(NEW.plan_json,'$.plan.sources.budget.budget_reservation_id')
                IS NOT NEW.budget_reservation_id
          OR json_extract(NEW.plan_json,'$.plan.sources.budget.budget_reserved_fen')
                IS NOT NEW.budget_reserved_fen
          OR json_extract(NEW.plan_json,'$.plan.sources.broker_request_digest')
                IS NOT NEW.broker_request_digest
          OR json_extract(NEW.plan_json,'$.plan.attempt.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.plan_json,'$.plan.attempt.reservation_id') IS NOT NEW.reservation_id
          OR json_extract(NEW.plan_json,'$.plan.attempt.attempt_lease_id')
                IS NOT NEW.attempt_lease_id
          OR json_extract(NEW.plan_json,'$.plan.attempt.attempt_no') IS NOT NEW.attempt_no
          OR json_type(NEW.plan_json,'$.plan.attempt.shard_id') IS NULL
          OR json_extract(NEW.plan_json,'$.plan.attempt.shard_id') IS NOT NEW.shard_id
          OR json_extract(NEW.plan_json,'$.plan.attempt.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.plan_json,'$.plan.route_binding_digest')
                IS NOT NEW.route_binding_digest
          OR json_extract(NEW.plan_json,'$.plan.capability.capability_id')
                IS NOT NEW.capability_id
          OR json_extract(NEW.plan_json,'$.plan.capability.capability_digest')
                IS NOT NEW.capability_digest
          OR json_extract(NEW.plan_json,'$.plan.capability.capability_kind')
                IS NOT NEW.capability_kind
          OR json_extract(NEW.plan_json,'$.plan.capability.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.plan_json,'$.plan.capability.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.plan_json,'$.plan.capability.expires_at')
                IS NOT NEW.capability_expires_at
          OR json_extract(NEW.plan_json,'$.plan.start.identity.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.plan_json,'$.plan.start.identity.reservation_id')
                IS NOT NEW.reservation_id
          OR json_extract(NEW.plan_json,'$.plan.start.identity.attempt_lease_id')
                IS NOT NEW.attempt_lease_id
          OR json_extract(NEW.plan_json,'$.plan.start.identity.attempt_no') IS NOT NEW.attempt_no
          OR json_type(NEW.plan_json,'$.plan.start.identity.shard_id') IS NULL
          OR json_extract(NEW.plan_json,'$.plan.start.identity.shard_id') IS NOT NEW.shard_id
          OR json_extract(NEW.plan_json,'$.plan.start.identity.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.plan_json,'$.plan.start.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.plan_json,'$.plan.start.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.plan_json,'$.plan.start.offer.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.plan_json,'$.plan.start.offer.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.plan_json,'$.plan.start.offer.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.plan_json,'$.plan.start.lease_expires_at')
                IS NOT NEW.lease_expires_at
          OR json_extract(NEW.plan_json,'$.plan.start.hard_deadline_at')
                IS NOT NEW.hard_deadline_at
          OR json_extract(NEW.plan_json,'$.plan.resource_grant.schema')
                IS NOT NEW.resource_grant_schema
          OR json_extract(NEW.plan_json,'$.plan.resource_grant.grant_id')
                IS NOT NEW.resource_grant_id
          OR json_extract(NEW.plan_json,'$.plan.resource_grant.enforcement_kind')
                IS NOT NEW.resource_grant_enforcement_kind
          OR json_extract(NEW.plan_json,'$.plan.resource_grant.grant_digest')
                IS NOT NEW.resource_grant_digest
          OR json_extract(NEW.plan_json,'$.plan.resource_grant') IS NOT NEW.resource_grant_json
          OR json_array_length(json_extract(NEW.plan_json,'$.plan.artifact_accesses'))
                IS NOT NEW.artifact_access_count
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.attempt_lease_id')
                IS NOT NEW.attempt_lease_id
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.authority_kind')
                IS NOT NEW.lease_authority_kind
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.delivery_mode')
                IS NOT NEW.lease_delivery_mode
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.audience')
                IS NOT NEW.lease_audience
          OR json_extract(NEW.plan_json,'$.plan.lease_authority.valid_until')
                IS NOT NEW.lease_authority_valid_until
          OR json_extract(NEW.plan_json,'$.plan.planned_at') IS NOT NEW.planned_at
          OR json_extract(NEW.plan_json,'$.plan.not_after') IS NOT NEW.not_after
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt execution plan projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_accesses_exact
        BEFORE INSERT ON compute_attempt_execution_plan_accesses
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_execution_plans p
              JOIN compute_artifact_access_receipts a
                ON a.access_id=NEW.access_id AND a.access_digest=NEW.access_digest
             WHERE p.plan_id=NEW.plan_id AND p.plan_digest=NEW.plan_digest
               AND NEW.ordinal<p.artifact_access_count
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].ordinal') IS NEW.ordinal
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].access_id') IS NEW.access_id
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].access_digest') IS NEW.access_digest
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].access_kind') IS NEW.access_kind
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].target_id') IS NEW.target_id
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].target_digest') IS NEW.target_digest
               AND json_extract(p.plan_json,
                    '$.plan.artifact_accesses['||NEW.ordinal||'].expires_at') IS NEW.expires_at
               AND a.job_id=p.job_id AND a.reservation_id=p.reservation_id
               AND a.attempt_lease_id=p.attempt_lease_id AND a.provider_id=p.provider_id
               AND a.executor_id=p.executor_id AND a.fencing_generation=p.fencing_generation
               AND a.route_binding_digest=p.route_binding_digest
               AND a.access_kind=NEW.access_kind AND a.target_id=NEW.target_id
               AND a.target_digest=NEW.target_digest AND a.expires_at=NEW.expires_at
               AND a.recorded_at<=p.planned_at AND p.hard_deadline_at<=a.expires_at
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt execution plan access mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_seals_exact
        BEFORE INSERT ON compute_attempt_execution_plan_seals
        WHEN json_extract(NEW.seal_json,'$.schema') IS NOT NEW.seal_schema
          OR json_extract(NEW.seal_json,'$.seal_id') IS NOT NEW.seal_id
          OR json_extract(NEW.seal_json,'$.seal_digest') IS NOT NEW.seal_digest
          OR json_extract(NEW.seal_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.seal_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.seal_json,'$.plan_id') IS NOT NEW.plan_id
          OR json_extract(NEW.seal_json,'$.plan_digest') IS NOT NEW.plan_digest
          OR json_extract(NEW.seal_json,'$.capability_digest') IS NOT NEW.capability_digest
          OR json_extract(NEW.seal_json,'$.artifact_access_count')
                IS NOT NEW.artifact_access_count
          OR json_extract(NEW.seal_json,'$.artifact_access_set_digest')
                IS NOT NEW.artifact_access_set_digest
          OR json_extract(NEW.seal_json,'$.resource_grant_digest')
                IS NOT NEW.resource_grant_digest
          OR json_extract(NEW.seal_json,'$.sealed_at') IS NOT NEW.sealed_at
          OR NOT EXISTS (
                SELECT 1 FROM compute_attempt_execution_plans p
                 WHERE p.plan_id=NEW.plan_id AND p.plan_digest=NEW.plan_digest
                   AND p.capability_digest=NEW.capability_digest
                   AND p.artifact_access_count=NEW.artifact_access_count
                   AND p.artifact_access_set_digest=NEW.artifact_access_set_digest
                   AND p.resource_grant_digest=NEW.resource_grant_digest
                   AND p.recorded_at<=NEW.sealed_at AND NEW.sealed_at<p.not_after
                   AND (SELECT COUNT(*) FROM compute_attempt_execution_plan_accesses x
                         WHERE x.plan_id=p.plan_id)=NEW.artifact_access_count
                   AND (NEW.artifact_access_count=0 OR (
                        (SELECT MIN(x.ordinal) FROM compute_attempt_execution_plan_accesses x
                          WHERE x.plan_id=p.plan_id)=0
                        AND (SELECT MAX(x.ordinal) FROM compute_attempt_execution_plan_accesses x
                          WHERE x.plan_id=p.plan_id)=NEW.artifact_access_count-1))
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt execution plan seal mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_execution_capability_receipts_no_replace
        BEFORE INSERT ON compute_execution_capability_receipts
        WHEN EXISTS (SELECT 1 FROM compute_execution_capability_receipts x
              WHERE x.capability_id=NEW.capability_id
                 OR x.capability_digest=NEW.capability_digest)
        BEGIN SELECT RAISE(ABORT, 'compute execution capability replacement is forbidden'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_artifact_access_receipts_no_replace
        BEFORE INSERT ON compute_artifact_access_receipts
        WHEN EXISTS (SELECT 1 FROM compute_artifact_access_receipts x
              WHERE x.access_id=NEW.access_id OR x.access_digest=NEW.access_digest)
        BEGIN SELECT RAISE(ABORT, 'compute artifact access replacement is forbidden'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plans_no_replace
        BEFORE INSERT ON compute_attempt_execution_plans
        WHEN EXISTS (SELECT 1 FROM compute_attempt_execution_plans x
              WHERE x.plan_id=NEW.plan_id OR x.plan_digest=NEW.plan_digest
                 OR x.attempt_lease_id=NEW.attempt_lease_id
                 OR (x.job_id=NEW.job_id AND x.attempt_no=NEW.attempt_no))
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan replacement is forbidden'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_accesses_no_replace
        BEFORE INSERT ON compute_attempt_execution_plan_accesses
        WHEN EXISTS (SELECT 1 FROM compute_attempt_execution_plan_accesses x
              WHERE (x.plan_id=NEW.plan_id AND x.ordinal=NEW.ordinal)
                 OR x.access_id=NEW.access_id)
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan access replacement is forbidden'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_seals_no_replace
        BEFORE INSERT ON compute_attempt_execution_plan_seals
        WHEN EXISTS (SELECT 1 FROM compute_attempt_execution_plan_seals x
              WHERE x.seal_id=NEW.seal_id OR x.seal_digest=NEW.seal_digest
                 OR x.plan_id=NEW.plan_id)
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan seal replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_execution_capability_receipts_no_update
        BEFORE UPDATE ON compute_execution_capability_receipts
        BEGIN SELECT RAISE(ABORT, 'compute execution capability receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_execution_capability_receipts_no_delete
        BEFORE DELETE ON compute_execution_capability_receipts
        BEGIN SELECT RAISE(ABORT, 'compute execution capability receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_artifact_access_receipts_no_update
        BEFORE UPDATE ON compute_artifact_access_receipts
        BEGIN SELECT RAISE(ABORT, 'compute artifact access receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_artifact_access_receipts_no_delete
        BEFORE DELETE ON compute_artifact_access_receipts
        BEGIN SELECT RAISE(ABORT, 'compute artifact access receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plans_no_update
        BEFORE UPDATE ON compute_attempt_execution_plans
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plans are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plans_no_delete
        BEFORE DELETE ON compute_attempt_execution_plans
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plans are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_accesses_no_update
        BEFORE UPDATE ON compute_attempt_execution_plan_accesses
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan accesses are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_accesses_no_delete
        BEFORE DELETE ON compute_attempt_execution_plan_accesses
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan accesses are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_seals_no_update
        BEFORE UPDATE ON compute_attempt_execution_plan_seals
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan seals are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_plan_seals_no_delete
        BEFORE DELETE ON compute_attempt_execution_plan_seals
        BEGIN SELECT RAISE(ABORT, 'compute attempt execution plan seals are append-only'); END;
        "#,
    )?;
    Ok(())
}
