use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(ROUTE_AUTHORITY)?;
    Ok(())
}

const ROUTE_AUTHORITY: &str = r#"
CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_route_current_authority
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_route_authorization_receipts route
    JOIN compute_attempt_start_outbox outbox
      ON outbox.outbox_id=NEW.outbox_id
     AND outbox.outbox_digest=NEW.outbox_digest
    JOIN compute_attempt_dispatch_commands command
      ON command.command_id=outbox.command_id
     AND command.command_digest=outbox.command_digest
    JOIN compute_attempt_dispatch_actor_receipts actor
      ON actor.actor_receipt_id=outbox.actor_receipt_id
     AND actor.actor_receipt_digest=outbox.actor_receipt_digest
    JOIN compute_service_actor_authorizations actor_authority
      ON actor_authority.actor_authorization_id=actor.actor_authorization_id
     AND actor_authority.actor_authorization_digest=actor.actor_authorization_digest
    JOIN compute_route_adapters current_adapter
      ON current_adapter.adapter_id=route.adapter_id
    JOIN compute_route_adapter_versions adapter
      ON adapter.adapter_id=route.adapter_id
     AND adapter.adapter_revision=route.adapter_revision
     AND adapter.adapter_digest=route.adapter_registry_digest
    JOIN compute_route_authorization_seals seal
      ON seal.route_authorization_id=route.route_authorization_id
     AND seal.route_authorization_digest=route.route_authorization_digest
   WHERE route.route_authorization_id=NEW.route_authorization_id
     AND route.route_authorization_digest=NEW.route_authorization_digest
     AND route.provider_id=NEW.provider_id
     AND route.provider_kind='external_pool'
     AND route.route_kind='server_adapter'
     AND route.adapter_id=NEW.adapter_id
     AND route.adapter_revision=NEW.adapter_revision
     AND route.adapter_registry_digest=NEW.adapter_registry_digest
     AND route.implementation_digest=NEW.adapter_implementation_digest
     AND route.executor_id=command.executor_id
     AND outbox.command_id=NEW.command_id
     AND outbox.command_digest=NEW.command_digest
     AND outbox.provider_id=NEW.provider_id
     AND outbox.adapter_id=NEW.adapter_id
     AND outbox.route_authorization_id=NEW.route_authorization_id
     AND outbox.route_authorization_digest=NEW.route_authorization_digest
     AND outbox.adapter_binding_digest=route.adapter_binding_digest
     AND (NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')
       OR (current_adapter.current_adapter_revision=NEW.adapter_revision
           AND current_adapter.current_adapter_digest=NEW.adapter_registry_digest
           AND current_adapter.status='active'
           AND adapter.status='active'))
     AND adapter.route_kind='server_adapter'
     AND adapter.implementation_digest=NEW.adapter_implementation_digest
     AND adapter.credential_verifier_id=NEW.credential_verifier_id
     AND adapter.credential_verifier_revision=NEW.credential_verifier_revision
     AND adapter.credential_verifier_digest=NEW.credential_verifier_digest
     AND EXISTS (
       SELECT 1 FROM json_each(adapter.supported_provider_kinds_json) supported_kind
        WHERE supported_kind.type='text' AND supported_kind.value='external_pool')
     AND actor.command_id=NEW.command_id
     AND actor.command_digest=NEW.command_digest
     AND actor.provider_id=NEW.provider_id
     AND actor.provider_owner_account_id=route.provider_owner_account_id
     AND actor.route_authorization_id=NEW.route_authorization_id
     AND actor.route_authorization_digest=NEW.route_authorization_digest
     AND actor.actor_phase=CASE outbox.operation_kind
          WHEN 'commit' THEN 'application' ELSE 'dispatch' END
     AND route.recorded_at<=actor.issued_at
     AND actor_authority.issued_at<=actor.issued_at
     AND actor.recorded_at<actor_authority.valid_until
     AND actor_authority.provider_id=NEW.provider_id
     AND actor_authority.provider_owner_account_id=route.provider_owner_account_id
     AND actor_authority.service_actor_id=actor.service_actor_id
     AND route.verified_by_service_actor_id=actor.service_actor_id
     AND route.actor_authorization_id=actor_authority.actor_authorization_id
     AND route.actor_authorization_digest=actor_authority.actor_authorization_digest
     AND actor_authority.issued_at<=route.authenticated_at
     AND route.recorded_at<actor_authority.valid_until
     AND (NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')
       OR (actor.issued_at<=NEW.started_at AND NEW.started_at<actor.valid_until
           AND actor_authority.issued_at<=NEW.started_at
           AND NEW.started_at<actor_authority.valid_until))
     AND EXISTS (
       SELECT 1 FROM json_each(actor_authority.allowed_route_kinds_json) allowed_route
        WHERE allowed_route.type='text' AND allowed_route.value='server_adapter')
     AND EXISTS (
       SELECT 1 FROM json_each(actor_authority.allowed_actor_phases_json) allowed_phase
        WHERE allowed_phase.type='text' AND allowed_phase.value=actor.actor_phase)
     AND seal.route_authorization_revision=route.route_authorization_revision
     AND seal.adapter_id=NEW.adapter_id
     AND seal.adapter_revision=NEW.adapter_revision
     AND seal.adapter_registry_digest=NEW.adapter_registry_digest
     AND seal.credential_id=NEW.route_credential_id
     AND seal.credential_revision=NEW.route_credential_revision
     AND seal.credential_digest=NEW.route_credential_digest
     AND seal.capability_count=6
     AND seal.capability_count=route.capability_count
     AND seal.capability_set_digest=route.capability_set_digest
     AND seal.recorded_at<=NEW.started_at
     AND route.capability_count=6
     AND (SELECT count(*) FROM compute_route_authorization_capabilities capability
           WHERE capability.route_authorization_id=route.route_authorization_id)=6
     AND json_array_length(json_extract(
           route.route_authorization_json,'$.authorization.capabilities'))=6
     AND NOT EXISTS (
       SELECT 1 FROM compute_route_authorization_capabilities capability
        WHERE capability.route_authorization_id=route.route_authorization_id
          AND (json_extract(route.route_authorization_json,
                 '$.authorization.capabilities['||capability.ordinal||'].ordinal') IS NOT capability.ordinal
            OR json_extract(route.route_authorization_json,
                 '$.authorization.capabilities['||capability.ordinal||'].capability_id') IS NOT capability.capability_id
            OR json_extract(route.route_authorization_json,
                 '$.authorization.capabilities['||capability.ordinal||'].capability_revision') IS NOT capability.capability_revision
            OR NOT EXISTS (
              SELECT 1 FROM json_each(adapter.supported_capabilities_json) supported
               WHERE json_extract(supported.value,'$.capability_id')=capability.capability_id
                 AND json_extract(supported.value,'$.capability_revision')=capability.capability_revision)))
)
BEGIN SELECT RAISE(ABORT,'V273 exchange attempt lacks exact current route authority'); END;
"#;
