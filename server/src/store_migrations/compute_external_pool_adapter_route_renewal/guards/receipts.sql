DROP TRIGGER IF EXISTS v278_route_renewal_receipt_lineage;
CREATE TRIGGER v278_route_renewal_receipt_lineage
BEFORE INSERT ON compute_external_pool_adapter_route_renewal_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_atomic_activation_receipts activation
    JOIN compute_external_pool_adapter_provider_active_successor_receipts genesis
      ON genesis.active_successor_receipt_id=NEW.activation_genesis_successor_receipt_id
     AND genesis.receipt_digest=NEW.activation_genesis_successor_receipt_digest
     AND genesis.successor_sequence=1
     AND genesis.activation_witness_id=activation.activation_receipt_id
     AND genesis.activation_witness_digest=activation.activation_receipt_digest
     AND genesis.activation_root_digest=activation.activation_root_digest
    JOIN compute_provider_versions provider
      ON provider.provider_id=NEW.active_provider_id
     AND provider.policy_revision=NEW.active_provider_policy_revision
     AND provider.provider_digest=NEW.active_provider_digest
    JOIN compute_external_pool_adapter_credential_reattestation_receipts credential_evidence
      ON credential_evidence.reattestation_receipt_id=NEW.credential_reattestation_receipt_id
     AND credential_evidence.reattestation_receipt_digest=NEW.credential_reattestation_receipt_digest
     AND credential_evidence.provider_binding_id=NEW.provider_binding_id
     AND credential_evidence.provider_binding_digest=NEW.provider_binding_digest
     AND credential_evidence.provider_id=NEW.active_provider_id
     AND credential_evidence.observed_provider_policy_revision=NEW.active_provider_policy_revision
     AND credential_evidence.observed_provider_digest=NEW.active_provider_digest
    JOIN compute_external_pool_provider_activation_delegations delegation
      ON delegation.delegation_id=NEW.delegation_id
     AND delegation.delegation_digest=NEW.delegation_digest
     AND delegation.provider_binding_id=NEW.provider_binding_id
     AND delegation.provider_binding_digest=NEW.provider_binding_digest
     AND delegation.service_actor_id=NEW.service_actor_id
    JOIN compute_service_actor_authorizations actor
      ON actor.actor_authorization_id=NEW.service_actor_authorization_id
     AND actor.actor_authorization_revision=NEW.service_actor_authorization_revision
     AND actor.actor_authorization_digest=NEW.service_actor_authorization_digest
     AND actor.provider_id=NEW.active_provider_id
     AND actor.service_actor_id=NEW.service_actor_id
    JOIN compute_route_credential_versions credential
      ON credential.credential_id=NEW.route_credential_id
     AND credential.credential_revision=NEW.route_credential_revision
     AND credential.credential_digest=NEW.route_credential_digest
     AND credential.provider_id=NEW.active_provider_id
     AND credential.adapter_id=NEW.route_adapter_projection_id
     AND credential.adapter_revision=NEW.route_adapter_revision
     AND credential.adapter_registry_digest=NEW.route_adapter_digest
     AND credential.adapter_binding_digest=NEW.projected_v211_adapter_binding_digest
     AND credential.verification_receipt_id=NEW.credential_reattestation_receipt_id
     AND credential.verification_receipt_digest=NEW.credential_reattestation_receipt_digest
     AND credential.actor_authorization_id=NEW.service_actor_authorization_id
     AND credential.actor_authorization_digest=NEW.service_actor_authorization_digest
    JOIN compute_route_authorization_receipts route
      ON route.route_authorization_id=NEW.route_authorization_id
     AND route.route_authorization_revision=NEW.route_authorization_revision
     AND route.route_authorization_digest=NEW.route_authorization_digest
     AND route.provider_id=NEW.active_provider_id
     AND route.executor_id=NEW.executor_id
     AND route.adapter_id=NEW.route_adapter_projection_id
     AND route.adapter_revision=NEW.route_adapter_revision
     AND route.adapter_registry_digest=NEW.route_adapter_digest
     AND route.adapter_binding_digest=NEW.projected_v211_adapter_binding_digest
     AND route.credential_id=NEW.route_credential_id
     AND route.credential_revision=NEW.route_credential_revision
     AND route.credential_digest=NEW.route_credential_digest
     AND route.capability_count=NEW.route_capability_count
     AND route.capability_set_digest=NEW.route_capability_set_digest
     AND route.actor_authorization_id=NEW.service_actor_authorization_id
     AND route.actor_authorization_digest=NEW.service_actor_authorization_digest
    JOIN compute_route_authorization_seals seal
      ON seal.seal_id=NEW.route_seal_id AND seal.seal_digest=NEW.route_seal_digest
     AND seal.route_authorization_id=NEW.route_authorization_id
     AND seal.route_authorization_digest=NEW.route_authorization_digest
     AND seal.credential_id=NEW.route_credential_id
     AND seal.credential_revision=NEW.route_credential_revision
     AND seal.credential_digest=NEW.route_credential_digest
   WHERE activation.activation_receipt_id=NEW.activation_receipt_id
     AND activation.activation_receipt_digest=NEW.activation_receipt_digest
     AND activation.provider_binding_id=NEW.provider_binding_id
     AND activation.provider_binding_digest=NEW.provider_binding_digest
     AND activation.activation_root_digest=NEW.activation_root_digest
     AND activation.executor_id=NEW.executor_id
     AND activation.stable_executor_binding_digest=NEW.stable_executor_binding_digest
     AND activation.projected_v211_adapter_binding_digest=NEW.projected_v211_adapter_binding_digest
     AND activation.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND activation.route_adapter_revision=NEW.route_adapter_revision
     AND activation.route_adapter_digest=NEW.route_adapter_digest
     AND genesis.provider_binding_id=NEW.provider_binding_id
     AND genesis.provider_binding_digest=NEW.provider_binding_digest
     AND genesis.activation_root_digest=NEW.activation_root_digest
     AND genesis.service_actor_id=NEW.service_actor_id
     AND delegation.issued_by_owner_user_id=actor.issued_by_user_id
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                      WHERE revoked.delegation_id=NEW.delegation_id
                        AND revoked.delegation_digest=NEW.delegation_digest)
     AND (
       (NEW.renewal_sequence=1
        AND NEW.predecessor_service_actor_authorization_id=activation.service_actor_authorization_id
        AND NEW.predecessor_service_actor_authorization_digest=activation.service_actor_authorization_digest
        AND NEW.predecessor_route_credential_id=activation.route_credential_id
        AND NEW.predecessor_route_credential_revision=activation.route_credential_revision
        AND NEW.predecessor_route_credential_digest=activation.route_credential_digest
        AND NEW.predecessor_route_authorization_id=activation.route_authorization_id
        AND NEW.predecessor_route_authorization_revision=activation.route_authorization_revision
        AND NEW.predecessor_route_authorization_digest=activation.route_authorization_digest
        AND NEW.predecessor_route_seal_id=activation.route_seal_id
        AND NEW.predecessor_route_seal_digest=activation.route_seal_digest)
       OR
       (NEW.renewal_sequence>1 AND EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_route_renewal_receipts predecessor
           WHERE predecessor.route_renewal_receipt_id=NEW.predecessor_route_renewal_receipt_id
             AND predecessor.route_renewal_receipt_digest=NEW.predecessor_route_renewal_receipt_digest
             AND predecessor.renewal_sequence+1=NEW.renewal_sequence
             AND predecessor.provider_binding_id=NEW.provider_binding_id
             AND predecessor.activation_root_digest=NEW.activation_root_digest
             AND predecessor.service_actor_authorization_id=NEW.predecessor_service_actor_authorization_id
             AND predecessor.service_actor_authorization_digest=NEW.predecessor_service_actor_authorization_digest
             AND predecessor.route_credential_id=NEW.predecessor_route_credential_id
             AND predecessor.route_credential_revision=NEW.predecessor_route_credential_revision
             AND predecessor.route_credential_digest=NEW.predecessor_route_credential_digest
             AND predecessor.route_authorization_id=NEW.predecessor_route_authorization_id
             AND predecessor.route_authorization_revision=NEW.predecessor_route_authorization_revision
             AND predecessor.route_authorization_digest=NEW.predecessor_route_authorization_digest
             AND predecessor.route_seal_id=NEW.predecessor_route_seal_id
             AND predecessor.route_seal_digest=NEW.predecessor_route_seal_digest)))
)
BEGIN SELECT RAISE(ABORT,'V278 route-renewal lineage/root mismatch'); END;

DROP TRIGGER IF EXISTS v278_route_renewal_receipt_no_replace;
CREATE TRIGGER v278_route_renewal_receipt_no_replace
BEFORE INSERT ON compute_external_pool_adapter_route_renewal_receipts
WHEN EXISTS (SELECT 1 FROM compute_external_pool_adapter_route_renewal_receipts old
              WHERE old.route_renewal_receipt_id=NEW.route_renewal_receipt_id
                 OR old.idempotency_digest=NEW.idempotency_digest)
BEGIN SELECT RAISE(ABORT,'V278 route-renewal receipt cannot be replaced'); END;

DROP TRIGGER IF EXISTS v278_route_renewal_receipt_no_update;
CREATE TRIGGER v278_route_renewal_receipt_no_update
BEFORE UPDATE ON compute_external_pool_adapter_route_renewal_receipts
BEGIN SELECT RAISE(ABORT,'V278 route-renewal receipts are immutable'); END;

DROP TRIGGER IF EXISTS v278_route_renewal_receipt_no_delete;
CREATE TRIGGER v278_route_renewal_receipt_no_delete
BEFORE DELETE ON compute_external_pool_adapter_route_renewal_receipts
BEGIN SELECT RAISE(ABORT,'V278 route-renewal receipts cannot be deleted'); END;

DROP TRIGGER IF EXISTS v278_route_credential_root_cas;
CREATE TRIGGER v278_route_credential_root_cas
BEFORE UPDATE ON compute_route_credentials
WHEN EXISTS (SELECT 1 FROM compute_route_credential_versions version
              WHERE version.credential_id=OLD.credential_id
                AND version.credential_revision=OLD.current_credential_revision
                AND version.provider_kind='external_pool')
 AND elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
       'route_credential_root_cas',OLD.credential_id,OLD.current_credential_revision,
       OLD.current_credential_digest,OLD.status,OLD.updated_at,NEW.credential_id,
       NEW.current_credential_revision,NEW.current_credential_digest,NEW.status,NEW.updated_at
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V278 external_pool credential-root CAS lacks exact pending plan'); END;
