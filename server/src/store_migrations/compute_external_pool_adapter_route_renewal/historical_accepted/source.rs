//! Exact durable V273 source shared by every historical Accepted write guard.

const ACTOR_BINDINGS: &str = r#"
               NEW.actor_phase='application'
               AND NEW.command_id=command.command_id
               AND NEW.command_digest=command.command_digest
               AND NEW.provider_id=command.provider_id
               AND NEW.provider_owner_account_id=command.activated_by_user_id
               AND NEW.service_actor_id=route.verified_by_service_actor_id
               AND NEW.actor_authorization_id=route.actor_authorization_id
               AND NEW.actor_authorization_digest=route.actor_authorization_digest
               AND NEW.route_authorization_id=route.route_authorization_id
               AND NEW.route_authorization_digest=route.route_authorization_digest
               AND NEW.ack_id=ack.ack_id AND NEW.ack_digest=ack.ack_digest
               AND NEW.application_id=ack.application_id"#;

const AUTHORITY_JOINS: &str = r#"
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.actor_receipt_id=NEW.application_actor_receipt_id
               AND actor.actor_receipt_digest=NEW.application_actor_receipt_digest"#;

const AUTHORITY_BINDINGS: &str = r#"
               NEW.command_id=command.command_id
               AND NEW.command_digest=command.command_digest
               AND NEW.plan_id=command.execution_plan_id
               AND NEW.plan_digest=command.execution_plan_digest
               AND NEW.ack_id=ack.ack_id AND NEW.ack_digest=ack.ack_digest
               AND NEW.application_id=ack.application_id
               AND NEW.lease_id=activation.lease_id
               AND NEW.lease_digest=activation.lease_digest
               AND NEW.provider_id=command.provider_id
               AND NEW.executor_id=command.executor_id
               AND NEW.fencing_generation=command.fencing_generation
               AND NEW.route_authorization_id=route.route_authorization_id
               AND NEW.route_authorization_digest=route.route_authorization_digest
               AND actor.command_id=command.command_id
               AND actor.command_digest=command.command_digest
               AND actor.actor_phase='application'
               AND actor.ack_id=ack.ack_id AND actor.ack_digest=ack.ack_digest
               AND actor.application_id=NEW.application_id
               AND actor.application_digest=NEW.application_digest"#;

const COMMIT_JOINS: &str = r#"
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.actor_receipt_id=NEW.actor_receipt_id
               AND actor.actor_receipt_digest=NEW.actor_receipt_digest
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.lease_authority_id=NEW.lease_authority_id
               AND authority.authority_revision=NEW.lease_authority_revision
               AND authority.lease_authority_digest=NEW.lease_authority_digest"#;

const COMMIT_BINDINGS: &str = r#"
               NEW.operation_kind='commit' AND NEW.operation_generation=1
               AND NEW.subject_outbox_id=prepare.outbox_id
               AND NEW.command_id=command.command_id
               AND NEW.command_digest=command.command_digest
               AND NEW.provider_id=command.provider_id
               AND NEW.adapter_id=command.adapter_id
               AND NEW.adapter_binding_digest=command.adapter_binding_digest
               AND NEW.route_authorization_id=route.route_authorization_id
               AND NEW.route_authorization_digest=route.route_authorization_digest
               AND NEW.plan_id=command.execution_plan_id
               AND NEW.plan_digest=command.execution_plan_digest
               AND NEW.lease_id=activation.lease_id
               AND NEW.fencing_generation=command.fencing_generation
               AND NEW.ack_id=ack.ack_id AND NEW.ack_digest=ack.ack_digest
               AND NEW.application_id=ack.application_id
               AND actor.command_id=command.command_id
               AND actor.actor_phase='application'
               AND actor.ack_id=ack.ack_id AND actor.ack_digest=ack.ack_digest
               AND actor.application_id=NEW.application_id
               AND actor.application_digest=NEW.application_digest
               AND authority.command_id=command.command_id
               AND authority.command_digest=command.command_digest
               AND authority.ack_id=ack.ack_id AND authority.ack_digest=ack.ack_digest
               AND authority.application_id=NEW.application_id
               AND authority.application_digest=NEW.application_digest
               AND authority.application_actor_receipt_id=actor.actor_receipt_id
               AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
               AND authority.lease_id=NEW.lease_id
               AND authority.route_authorization_id=NEW.route_authorization_id
               AND authority.route_authorization_digest=NEW.route_authorization_digest"#;

const APPLICATION_JOINS: &str = r#"
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.command_id=command.command_id
               AND actor.actor_phase='application'
               AND actor.ack_id=ack.ack_id
               AND actor.application_id=NEW.application_id
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.application_id=NEW.application_id
               AND authority.application_actor_receipt_id=actor.actor_receipt_id
               AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
              JOIN compute_attempt_start_outbox commit_intent
                ON commit_intent.application_id=NEW.application_id
               AND commit_intent.operation_kind='commit'"#;

const APPLICATION_BINDINGS: &str = r#"
               NEW.command_id=command.command_id AND NEW.ack_id=ack.ack_id
               AND NEW.lease_id=activation.lease_id
               AND NEW.lease_digest=activation.lease_digest
               AND actor.command_digest=command.command_digest
               AND actor.ack_digest=ack.ack_digest
               AND actor.application_digest=NEW.application_digest
               AND actor.route_authorization_id=route.route_authorization_id
               AND actor.route_authorization_digest=route.route_authorization_digest
               AND authority.command_id=command.command_id
               AND authority.command_digest=command.command_digest
               AND authority.ack_id=ack.ack_id AND authority.ack_digest=ack.ack_digest
               AND authority.application_digest=NEW.application_digest
               AND authority.lease_id=NEW.lease_id
               AND authority.lease_digest=NEW.lease_digest
               AND authority.route_authorization_id=route.route_authorization_id
               AND authority.route_authorization_digest=route.route_authorization_digest
               AND commit_intent.command_id=command.command_id
               AND commit_intent.command_digest=command.command_digest
               AND commit_intent.ack_id=ack.ack_id
               AND commit_intent.ack_digest=ack.ack_digest
               AND commit_intent.application_digest=NEW.application_digest
               AND commit_intent.actor_receipt_id=actor.actor_receipt_id
               AND commit_intent.actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.lease_authority_id=authority.lease_authority_id
               AND commit_intent.lease_authority_revision=authority.authority_revision
               AND commit_intent.lease_authority_digest=authority.lease_authority_digest"#;

pub(super) fn actor() -> String {
    exact_source("", ACTOR_BINDINGS, "NEW.recorded_at")
}

pub(super) fn authority() -> String {
    exact_source(AUTHORITY_JOINS, AUTHORITY_BINDINGS, "NEW.recorded_at")
}

pub(super) fn commit() -> String {
    exact_source(COMMIT_JOINS, COMMIT_BINDINGS, "NEW.created_at")
}

pub(super) fn application() -> String {
    exact_source(APPLICATION_JOINS, APPLICATION_BINDINGS, "NEW.created_at")
}

fn exact_source(extra_joins: &str, bindings: &str, closure_at: &str) -> String {
    format!(
        r#"            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_providers provider ON provider.provider_id=command.provider_id
              JOIN compute_attempt_start_outbox prepare
                ON prepare.command_id=command.command_id
               AND prepare.command_digest=command.command_digest
               AND prepare.operation_kind='prepare' AND prepare.operation_generation=1
              JOIN compute_attempt_start_send_attempts send
                ON send.outbox_id=prepare.outbox_id
               AND send.outbox_digest=prepare.outbox_digest
               AND send.operation_kind='prepare'
               AND send.command_id=command.command_id
               AND send.command_digest=command.command_digest
              JOIN compute_attempt_dispatch_acks ack ON ack.command_id=command.command_id
              JOIN compute_attempt_activations activation
                ON activation.lease_id=ack.activation_lease_id
              JOIN compute_attempt_start_remote_observations observation
                ON observation.command_id=command.command_id
               AND observation.command_digest=command.command_digest
               AND observation.send_attempt_id=send.send_attempt_id
              JOIN compute_external_pool_adapter_task_exchange_receipts receipt
                ON receipt.exchange_receipt_id=observation.verifier_id
               AND receipt.semantic_observation_sha256=observation.verification_digest
              JOIN compute_external_pool_adapter_task_exchange_attempts receipt_attempt
                ON receipt_attempt.exchange_attempt_id=receipt.exchange_attempt_id
               AND receipt_attempt.exchange_attempt_digest=receipt.exchange_attempt_digest
              LEFT JOIN compute_external_pool_adapter_task_reconcile_polls poll
                ON receipt.source_kind='reconcile_poll'
               AND poll.reconcile_poll_id=receipt.source_id
               AND poll.reconcile_poll_digest=receipt.source_digest
              JOIN compute_external_pool_adapter_task_exchange_attempts source_attempt
                ON (receipt.source_kind='start_outbox_send_attempt'
                    AND source_attempt.exchange_attempt_id=receipt_attempt.exchange_attempt_id
                    AND source_attempt.exchange_attempt_digest=receipt_attempt.exchange_attempt_digest)
                OR (receipt.source_kind='reconcile_poll'
                    AND source_attempt.exchange_attempt_id=poll.uncertain_exchange_attempt_id
                    AND source_attempt.exchange_attempt_digest=poll.uncertain_exchange_attempt_digest)
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=source_attempt.route_authorization_id
               AND route.route_authorization_digest=source_attempt.route_authorization_digest
{extra_joins}
             WHERE {bindings}
               AND provider.provider_kind='external_pool'
               AND command.lease_id=activation.lease_id
               AND command.provider_id=activation.provider_id
               AND command.executor_id=activation.executor_id
               AND command.fencing_generation=activation.fencing_generation
               AND ack.provider_id=command.provider_id AND ack.adapter_id=command.adapter_id
               AND ack.command_digest=command.command_digest
               AND ack.adapter_binding_digest=command.adapter_binding_digest
               AND ack.outcome='accepted' AND ack.disposition='accepted_applied'
               AND ack.application_id IS NOT NULL
               AND prepare.provider_id=command.provider_id
               AND prepare.adapter_id=command.adapter_id
               AND prepare.adapter_binding_digest=command.adapter_binding_digest
               AND send.route_authorization_id=prepare.route_authorization_id
               AND send.route_authorization_digest=prepare.route_authorization_digest
               AND source_attempt.operation_kind='prepare'
               AND source_attempt.source_kind='start_outbox_send_attempt'
               AND source_attempt.source_id=send.send_attempt_id
               AND source_attempt.source_digest=send.send_attempt_digest
               AND source_attempt.command_id=command.command_id
               AND source_attempt.command_digest=command.command_digest
               AND source_attempt.outbox_id=prepare.outbox_id
               AND source_attempt.outbox_digest=prepare.outbox_digest
               AND source_attempt.send_attempt_id=send.send_attempt_id
               AND source_attempt.send_attempt_digest=send.send_attempt_digest
               AND source_attempt.provider_id=command.provider_id
               AND source_attempt.adapter_id=command.adapter_id
               AND source_attempt.route_authorization_id=prepare.route_authorization_id
               AND source_attempt.route_authorization_digest=prepare.route_authorization_digest
               AND source_attempt.fencing_generation=command.fencing_generation
               AND route.provider_id=command.provider_id
               AND route.adapter_id=command.adapter_id
               AND route.adapter_revision=source_attempt.adapter_revision
               AND route.adapter_registry_digest=source_attempt.adapter_registry_digest
               AND route.adapter_binding_digest=command.adapter_binding_digest
               AND route.credential_id=source_attempt.route_credential_id
               AND route.credential_revision=source_attempt.route_credential_revision
               AND route.credential_digest=source_attempt.route_credential_digest
               AND route.authenticated_at<=receipt.authenticated_at
               AND receipt_attempt.operation_kind=receipt.operation_kind
               AND receipt_attempt.source_kind=receipt.source_kind
               AND receipt_attempt.source_id=receipt.source_id
               AND receipt_attempt.source_digest=receipt.source_digest
               AND receipt_attempt.command_id=receipt.command_id
               AND receipt_attempt.command_digest=receipt.command_digest
               AND receipt_attempt.outbox_id=receipt.outbox_id
               AND receipt_attempt.outbox_digest=receipt.outbox_digest
               AND receipt_attempt.send_attempt_id=receipt.send_attempt_id
               AND receipt_attempt.send_attempt_digest=receipt.send_attempt_digest
               AND receipt_attempt.provider_id=receipt.provider_id
               AND receipt_attempt.adapter_id=receipt.adapter_id
               AND receipt_attempt.adapter_revision=receipt.adapter_revision
               AND receipt_attempt.adapter_registry_digest=receipt.adapter_registry_digest
               AND receipt_attempt.route_authorization_id=receipt.route_authorization_id
               AND receipt_attempt.route_authorization_digest=receipt.route_authorization_digest
               AND receipt_attempt.route_credential_id=receipt.route_credential_id
               AND receipt_attempt.route_credential_revision=receipt.route_credential_revision
               AND receipt_attempt.route_credential_digest=receipt.route_credential_digest
               AND receipt_attempt.executor_binding_digest=receipt.executor_binding_digest
               AND receipt_attempt.fencing_generation=receipt.fencing_generation
               AND receipt_attempt.fence_digest=receipt.fence_digest
               AND receipt.command_id=command.command_id
               AND receipt.command_digest=command.command_digest
               AND receipt.outbox_id=prepare.outbox_id
               AND receipt.outbox_digest=prepare.outbox_digest
               AND receipt.send_attempt_id=send.send_attempt_id
               AND receipt.send_attempt_digest=send.send_attempt_digest
               AND receipt.provider_id=source_attempt.provider_id
               AND receipt.adapter_id=source_attempt.adapter_id
               AND receipt.adapter_revision=source_attempt.adapter_revision
               AND receipt.adapter_registry_digest=source_attempt.adapter_registry_digest
               AND receipt.adapter_implementation_digest=source_attempt.adapter_implementation_digest
               AND receipt.route_authorization_id=source_attempt.route_authorization_id
               AND receipt.route_authorization_digest=source_attempt.route_authorization_digest
               AND receipt.route_credential_id=source_attempt.route_credential_id
               AND receipt.route_credential_revision=source_attempt.route_credential_revision
               AND receipt.route_credential_digest=source_attempt.route_credential_digest
               AND receipt.executor_binding_digest=source_attempt.executor_binding_digest
               AND receipt.fencing_generation=source_attempt.fencing_generation
               AND receipt.fence_digest=source_attempt.fence_digest
               AND observation.operation_kind='prepare'
               AND observation.provider_id=command.provider_id
               AND observation.adapter_id=command.adapter_id
               AND observation.adapter_binding_digest=command.adapter_binding_digest
               AND observation.adapter_observation_id=ack.adapter_ack_id
               AND observation.response_outcome='accepted'
               AND observation.remote_execution_state IN ('prepared','committed','running')
               AND observation.terminality='non_terminal'
               AND observation.remote_execution_ref=ack.remote_execution_ref
               AND observation.reason_code IS NULL
               AND observation.observed_at=ack.observed_at
               AND observation.received_at=ack.received_at
               AND observation.verification_kind='external_pool_adapter_task_receipt.v1'
               AND observation.authenticated_at=receipt.authenticated_at
               AND observation.received_at=receipt.received_at
               AND observation.recorded_at=receipt.recorded_at
               AND receipt.recorded_at<=ack.created_at AND ack.created_at<={closure_at}
               AND ((receipt.operation_kind='prepare'
                    AND receipt.source_kind='start_outbox_send_attempt'
                    AND receipt.source_id=send.send_attempt_id
                    AND receipt.source_digest=send.send_attempt_digest
                    AND observation.observation_kind='prepare_response')
                 OR (receipt.operation_kind='reconcile'
                    AND receipt.source_kind='reconcile_poll'
                    AND poll.uncertain_exchange_attempt_id=source_attempt.exchange_attempt_id
                    AND poll.uncertain_exchange_attempt_digest=source_attempt.exchange_attempt_digest
                    AND poll.command_id=command.command_id
                    AND poll.command_digest=command.command_digest
                    AND poll.outbox_id=prepare.outbox_id
                    AND poll.outbox_digest=prepare.outbox_digest
                    AND poll.send_attempt_id=send.send_attempt_id
                    AND poll.send_attempt_digest=send.send_attempt_digest
                    AND poll.route_authorization_id=route.route_authorization_id
                    AND poll.route_authorization_digest=route.route_authorization_digest
                    AND poll.executor_binding_digest=source_attempt.executor_binding_digest
                    AND poll.fencing_generation=source_attempt.fencing_generation
                    AND poll.fence_digest=source_attempt.fence_digest
                    AND poll.claim_status='delivery_observed'
                    AND observation.observation_kind='reconcile_attestation'))
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities capability
                     WHERE capability.route_authorization_id=route.route_authorization_id
                       AND capability.capability_id='authenticated_ack')
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities capability
                     WHERE capability.route_authorization_id=route.route_authorization_id
                       AND capability.capability_id='idempotent_commit')
               AND ((SELECT COUNT(*)
                       FROM compute_external_pool_adapter_route_renewal_receipts renewal
                      WHERE renewal.route_authorization_id=source_attempt.route_authorization_id
                        AND renewal.route_authorization_digest=source_attempt.route_authorization_digest
                        AND renewal.route_credential_id=source_attempt.route_credential_id
                        AND renewal.route_credential_revision=source_attempt.route_credential_revision
                        AND renewal.route_credential_digest=source_attempt.route_credential_digest
                        AND renewal.active_provider_id=source_attempt.provider_id
                        AND renewal.route_adapter_projection_id=source_attempt.adapter_id
                        AND renewal.route_adapter_revision=source_attempt.adapter_revision
                        AND renewal.route_adapter_digest=source_attempt.adapter_registry_digest
                        AND renewal.stable_executor_binding_digest=source_attempt.executor_binding_digest
                        AND receipt.recorded_at<renewal.cleanup_expires_at
                        AND {closure_at}<renewal.cleanup_expires_at)
                    + (SELECT COUNT(*)
                         FROM compute_external_pool_adapter_atomic_activation_receipts genesis
                         JOIN compute_external_pool_adapter_provider_active_successor_receipts successor
                           ON successor.successor_sequence=1
                          AND successor.activation_witness_id=genesis.activation_receipt_id
                          AND successor.activation_witness_digest=genesis.activation_receipt_digest
                         JOIN compute_route_authorization_receipts genesis_route
                           ON genesis_route.route_authorization_id=genesis.route_authorization_id
                          AND genesis_route.route_authorization_digest=genesis.route_authorization_digest
                          AND genesis_route.credential_id=genesis.route_credential_id
                          AND genesis_route.credential_revision=genesis.route_credential_revision
                          AND genesis_route.credential_digest=genesis.route_credential_digest
                        WHERE genesis.route_authorization_id=source_attempt.route_authorization_id
                          AND genesis.route_authorization_digest=source_attempt.route_authorization_digest
                          AND genesis.route_credential_id=source_attempt.route_credential_id
                          AND genesis.route_credential_revision=source_attempt.route_credential_revision
                          AND genesis.route_credential_digest=source_attempt.route_credential_digest
                          AND genesis.target_active_provider_id=source_attempt.provider_id
                          AND genesis.route_adapter_projection_id=source_attempt.adapter_id
                          AND genesis.route_adapter_revision=source_attempt.adapter_revision
                          AND genesis.route_adapter_digest=source_attempt.adapter_registry_digest
                          AND genesis.stable_executor_binding_digest=source_attempt.executor_binding_digest
                          AND receipt.recorded_at<genesis_route.cleanup_expires_at
                          AND {closure_at}<genesis_route.cleanup_expires_at))=1"#
    )
}
