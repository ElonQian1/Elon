use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_authority_and_chain_guards(conn)?;
    install_source_guards(conn)?;
    Ok(())
}

fn install_authority_and_chain_guards(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_event_wire_projection
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN (SELECT COUNT(*) FROM json_each(NEW.message_json)) IS NOT 4
          OR (SELECT COUNT(*) FROM json_each(
               json_extract(NEW.message_json,'$.binding'))) IS NOT 10
          OR (SELECT COUNT(*) FROM json_each(
               json_extract(NEW.message_json,'$.binding.session_binding'))) IS NOT 16
          OR json_extract(NEW.message_json,'$.schema') IS NOT NEW.message_schema
          OR json_extract(NEW.message_json,'$.binding.bootstrap_id') IS NOT NEW.bootstrap_id
          OR json_extract(NEW.message_json,'$.binding.message_sequence') IS NOT NEW.message_sequence
          OR json_extract(NEW.message_json,'$.binding.delivery_id') IS NOT NEW.delivery_id
          OR json_extract(NEW.message_json,'$.binding.previous_message_digest')
               IS NOT NEW.previous_message_digest
          OR json_extract(NEW.message_json,'$.binding.message_digest') IS NOT NEW.message_digest
          OR json_extract(NEW.message_json,'$.binding.protocol_version') IS NOT 14
          OR json_extract(NEW.message_json,'$.binding.capability')
               IS NOT 'node_endpoint_planning_snapshot_bootstrap_v1'
          OR json_extract(NEW.message_json,'$.binding.canonicalization') IS NOT 'rfc8785_jcs'
          OR json_extract(NEW.message_json,'$.binding.digest_algorithm') IS NOT 'sha256'
          OR json_extract(NEW.message_json,'$.binding.session_binding.agent_id') IS NOT NEW.agent_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.owner_user_id')
               IS NOT NEW.owner_user_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.install_id')
               IS NOT NEW.install_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.installation_binding_digest')
               IS NOT NEW.installation_binding_digest
          OR json_extract(NEW.message_json,'$.binding.session_binding.credential_id')
               IS NOT NEW.credential_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.credential_revision')
               IS NOT NEW.credential_revision
          OR json_extract(NEW.message_json,'$.binding.session_binding.credential_digest')
               IS NOT NEW.credential_digest
          OR json_extract(NEW.message_json,'$.binding.session_binding.session_id')
               IS NOT NEW.session_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.session_generation')
               IS NOT NEW.session_generation
          OR json_extract(NEW.message_json,'$.binding.session_binding.authentication_receipt_id')
               IS NOT NEW.authentication_receipt_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.authentication_digest')
               IS NOT NEW.authentication_digest
          OR json_extract(NEW.message_json,'$.binding.session_binding.server_instance_id')
               IS NOT NEW.server_instance_id
          OR json_extract(NEW.message_json,'$.binding.session_binding.agent_version')
               IS NOT NEW.agent_version
          OR json_extract(NEW.message_json,'$.binding.session_binding.capability_set_digest')
               IS NOT NEW.capability_set_digest
          OR json_extract(NEW.message_json,'$.binding.session_binding.authenticated_at')
               IS NOT NEW.authenticated_at
          OR json_extract(NEW.message_json,'$.binding.session_binding.expires_at')
               IS NOT NEW.expires_at
        BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning message projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_event_current_session
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NOT EXISTS (
          SELECT 1
            FROM node_endpoint_session_authentication_receipts receipt
            JOIN node_endpoint_session_heads head
              ON head.agent_id=receipt.agent_id
             AND head.credential_id=receipt.credential_id
             AND head.credential_revision=receipt.credential_revision
             AND head.credential_digest=receipt.credential_digest
             AND head.authentication_receipt_id=receipt.authentication_receipt_id
             AND head.authentication_digest=receipt.authentication_digest
             AND head.session_id=receipt.session_id
             AND head.session_generation=receipt.session_generation
             AND head.server_instance_id=receipt.server_instance_id
            JOIN node_endpoint_credentials credential
              ON credential.credential_id=receipt.credential_id
             AND credential.agent_id=receipt.agent_id
             AND credential.owner_user_id=receipt.owner_user_id
             AND credential.install_id=receipt.install_id
             AND credential.installation_binding_digest=receipt.installation_binding_digest
             AND credential.current_credential_revision=receipt.credential_revision
             AND credential.current_credential_digest=receipt.credential_digest
           WHERE receipt.authentication_receipt_id=NEW.authentication_receipt_id
             AND receipt.authentication_digest=NEW.authentication_digest
             AND receipt.agent_id=NEW.agent_id
             AND receipt.owner_user_id=NEW.owner_user_id
             AND receipt.install_id=NEW.install_id
             AND receipt.installation_binding_digest=NEW.installation_binding_digest
             AND receipt.credential_id=NEW.credential_id
             AND receipt.credential_revision=NEW.credential_revision
             AND receipt.credential_digest=NEW.credential_digest
             AND receipt.session_id=NEW.session_id
             AND receipt.session_generation=NEW.session_generation
             AND receipt.server_instance_id=NEW.server_instance_id
             AND receipt.agent_version=NEW.agent_version
             AND receipt.authenticated_at=NEW.authenticated_at
             AND receipt.expires_at=NEW.expires_at
             AND receipt.protocol_version=NEW.protocol_version AND receipt.protocol_version=14
             AND receipt.capability_count=NEW.capability_count AND receipt.capability_count=1
             AND receipt.capability_set_json=NEW.capability_set_json
             AND receipt.capability_set_json='["node_endpoint_planning_snapshot_bootstrap_v1"]'
             AND receipt.capability_set_digest=NEW.capability_set_digest
             AND receipt.recorded_at<=NEW.recorded_at
             AND head.state='active'
             AND head.authenticated_at=receipt.authenticated_at
             AND head.expires_at=receipt.expires_at
             AND head.created_at=receipt.recorded_at
             AND head.updated_at=receipt.recorded_at
             AND credential.status='active'
        )
        BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning exact current session required');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_event_current_policy
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NOT EXISTS (
          SELECT 1
            FROM node_compute_sharing_policies policy
            JOIN node_compute_plugin_sharing_consents consent
              ON consent.receipt_id=policy.plugin_consent_receipt_id
             AND consent.node_id=policy.node_id
             AND consent.owner_user_id=policy.owner_user_id
             AND consent.installation_identity_digest=policy.plugin_installation_identity_digest
             AND consent.policy_revision=policy.plugin_policy_revision
             AND consent.policy_digest=policy.plugin_policy_digest
             AND consent.plugin_runtime_requested=policy.plugin_runtime_requested
             AND policy.enabled=policy.plugin_runtime_requested
             AND consent.allowed_model_ids_json=policy.allowed_model_ids_json
             AND consent.max_concurrent_runs=policy.max_concurrent_runs
             AND consent.daily_token_limit=policy.daily_token_limit
             AND consent.authorization_ref IS policy.plugin_authorization_ref
             AND consent.authorization_revision IS policy.plugin_authorization_revision
             AND consent.authorization_digest IS policy.plugin_authorization_digest
           WHERE policy.node_id=NEW.agent_id AND policy.owner_user_id=NEW.owner_user_id
             AND policy.plugin_runtime_requested=NEW.plugin_runtime_requested
             AND policy.plugin_consent_receipt_id=NEW.consent_receipt_id
             AND policy.plugin_installation_identity_digest=NEW.plugin_installation_identity_digest
             AND policy.plugin_policy_revision=NEW.policy_revision
             AND policy.plugin_policy_digest=NEW.policy_digest
             AND consent.consent_schema='elon.node_compute_plugin.sharing_consent.v1'
        )
        BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning exact current policy required');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_event_linear_chain
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence>1 AND NOT EXISTS (
          SELECT 1 FROM node_compute_plugin_endpoint_planning_chain_events_v1 prior
           WHERE prior.bootstrap_id=NEW.bootstrap_id
             AND prior.message_sequence=NEW.previous_message_sequence
             AND prior.event_id=NEW.previous_event_id
             AND prior.message_digest=NEW.previous_message_digest
             AND prior.recorded_at<NEW.recorded_at
             AND prior.agent_id=NEW.agent_id AND prior.owner_user_id=NEW.owner_user_id
             AND prior.install_id=NEW.install_id
             AND prior.installation_binding_digest=NEW.installation_binding_digest
             AND prior.plugin_installation_identity_digest=NEW.plugin_installation_identity_digest
             AND prior.credential_id=NEW.credential_id
             AND prior.credential_revision=NEW.credential_revision
             AND prior.credential_digest=NEW.credential_digest
             AND prior.authentication_receipt_id=NEW.authentication_receipt_id
             AND prior.authentication_digest=NEW.authentication_digest
             AND prior.session_id=NEW.session_id
             AND prior.session_generation=NEW.session_generation
             AND prior.server_instance_id=NEW.server_instance_id
             AND prior.agent_version=NEW.agent_version
             AND prior.authenticated_at=NEW.authenticated_at
             AND prior.expires_at=NEW.expires_at
             AND prior.protocol_version=NEW.protocol_version
             AND prior.capability_count=NEW.capability_count
             AND prior.capability_set_json=NEW.capability_set_json
             AND prior.capability_set_digest=NEW.capability_set_digest
             AND prior.consent_receipt_id=NEW.consent_receipt_id
             AND prior.policy_revision=NEW.policy_revision
             AND prior.policy_digest=NEW.policy_digest
             AND prior.policy_snapshot_digest=NEW.policy_snapshot_digest
             AND prior.plugin_runtime_requested=NEW.plugin_runtime_requested
             AND prior.sharing_delivery_id=NEW.sharing_delivery_id
             AND (NEW.message_sequence<=2 OR prior.sharing_observation_id=NEW.sharing_observation_id)
             AND (NEW.message_sequence<=2
                  OR prior.sharing_observation_digest=NEW.sharing_observation_digest)
             AND (NEW.message_sequence<=3 OR prior.preparation_id=NEW.preparation_id)
             AND (NEW.message_sequence<=3
                  OR prior.preparation_delivery_id=NEW.preparation_delivery_id)
             AND (NEW.message_sequence<=3
                  OR prior.preparation_request_digest=NEW.preparation_request_digest)
             AND (NEW.message_sequence<=4
                  OR prior.preparation_observation_id=NEW.preparation_observation_id)
             AND (NEW.message_sequence<=4
                  OR prior.preparation_observation_digest=NEW.preparation_observation_digest)
             AND (NEW.message_sequence<=5 OR prior.planning_delivery_id=NEW.planning_delivery_id)
             AND (NEW.message_sequence<=5 OR prior.planning_request_digest=NEW.planning_request_digest)
             AND (NEW.message_sequence NOT IN (3,5)
                  OR (prior.next_message_sequence=NEW.message_sequence
                      AND prior.next_event_id=NEW.event_id AND prior.accepted=1))
        )
        BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning chain predecessor mismatch');
        END;
        "#,
    )?;
    Ok(())
}

fn install_source_guards(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_sharing_request_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=1 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.sharing_request.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_sharing_request_v1'
          OR json_extract(NEW.message_json,'$.snapshot.installation_identity_digest')
               IS NOT NEW.plugin_installation_identity_digest
          OR (SELECT COUNT(*) FROM json_each(
               json_extract(NEW.message_json,'$.snapshot'))) IS NOT 8
          OR json_extract(NEW.message_json,'$.snapshot.plugin_runtime_requested')
               IS NOT NEW.plugin_runtime_requested
          OR NOT EXISTS (
            SELECT 1
              FROM node_compute_plugin_sharing_delivery_events delivery
              JOIN node_compute_sharing_policies policy
                ON policy.node_id=delivery.node_id
               AND policy.owner_user_id=NEW.owner_user_id
               AND policy.plugin_consent_receipt_id=delivery.consent_receipt_id
               AND policy.plugin_policy_revision=delivery.policy_revision
               AND policy.plugin_policy_digest=delivery.policy_digest
             WHERE delivery.delivery_id=NEW.sharing_delivery_id
               AND delivery.node_id=NEW.agent_id
               AND delivery.consent_receipt_id=NEW.consent_receipt_id
               AND delivery.policy_revision=NEW.policy_revision
               AND delivery.policy_digest=NEW.policy_digest
               AND delivery.event_sequence=1 AND delivery.event_kind='intent_committed'
               AND policy.plugin_runtime_requested=NEW.plugin_runtime_requested
               AND ((NEW.plugin_runtime_requested=0
                     AND json_type(NEW.message_json,'$.snapshot.authorization')='null')
                 OR (NEW.plugin_runtime_requested=1
                     AND json_type(NEW.message_json,'$.snapshot.authorization')='object'
                     AND (SELECT COUNT(*) FROM json_each(json_extract(
                           NEW.message_json,'$.snapshot.authorization'))) = 3
                     AND json_extract(NEW.message_json,
                           '$.snapshot.authorization.authorization_ref')
                           =policy.plugin_authorization_ref
                     AND json_extract(NEW.message_json,'$.snapshot.authorization.revision')
                           =policy.plugin_authorization_revision
                     AND json_extract(NEW.message_json,'$.snapshot.authorization.digest')
                           =policy.plugin_authorization_digest))
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning sharing request source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_sharing_observed_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=2 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.sharing_observed.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_sharing_observed_v1'
          OR (json_extract(NEW.message_json,'$.observed.phase') IS NOT 'blocked'
              AND json_extract(NEW.message_json,'$.observed.phase') IS NOT 'disabled')
          OR (NEW.accepted=1 AND json_extract(NEW.message_json,'$.observed.phase')
               IS NOT CASE WHEN NEW.plugin_runtime_requested=1
                           THEN 'blocked' ELSE 'disabled' END)
          OR (NEW.accepted=1 AND json_extract(
                 NEW.message_json,'$.observed.installation_identity_digest'
               ) IS NOT NEW.plugin_installation_identity_digest)
          OR NOT EXISTS (
            SELECT 1
              FROM node_compute_plugin_sharing_delivery_events delivery
              JOIN node_compute_plugin_sharing_observations observation
                ON observation.delivery_id=delivery.delivery_id
               AND observation.node_id=delivery.node_id
               AND observation.consent_receipt_id=delivery.consent_receipt_id
               AND observation.policy_revision=delivery.policy_revision
               AND observation.policy_digest=delivery.policy_digest
             WHERE delivery.delivery_id=NEW.sharing_delivery_id
               AND delivery.event_sequence=2 AND delivery.event_kind='dispatched'
               AND delivery.detail_code IS NULL
               AND observation.id=NEW.sharing_observation_id
               AND observation.accepted=NEW.accepted
               AND (NEW.accepted=0 OR json_extract(
                      observation.observed_json,'$.installation_identity_digest'
                    )=NEW.plugin_installation_identity_digest)
               AND json_extract(observation.observed_json,'$.replayed')=NEW.replayed
               AND (SELECT COUNT(*) FROM node_compute_plugin_sharing_delivery_events e
                     WHERE e.delivery_id=delivery.delivery_id)=2
               AND (SELECT COUNT(*) FROM node_compute_plugin_sharing_observations o
                     WHERE o.delivery_id=delivery.delivery_id)=1
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning sharing observation source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_preparation_request_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=3 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.preparation_request.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_preparation_request_v1'
          OR json_extract(NEW.message_json,'$.request.installation_identity_digest')
               IS NOT NEW.plugin_installation_identity_digest
          OR NOT EXISTS (
            SELECT 1
              FROM node_compute_plugin_install_plan_preparation_requests request
              JOIN node_compute_plugin_install_plan_preparation_delivery_events delivery
                ON delivery.preparation_id=request.preparation_id
               AND delivery.node_id=request.node_id
               AND delivery.consent_receipt_id=request.consent_receipt_id
               AND delivery.policy_revision=request.policy_revision
               AND delivery.policy_digest=request.policy_digest
             WHERE request.preparation_id=NEW.preparation_id
               AND request.request_digest=NEW.preparation_request_digest
               AND request.node_id=NEW.agent_id AND request.owner_user_id=NEW.owner_user_id
               AND request.consent_receipt_id=NEW.consent_receipt_id
               AND request.installation_identity_digest=NEW.plugin_installation_identity_digest
               AND request.policy_revision=NEW.policy_revision
               AND request.policy_digest=NEW.policy_digest
               AND request.policy_snapshot_digest=NEW.policy_snapshot_digest
               AND delivery.delivery_id=NEW.preparation_delivery_id
               AND delivery.sharing_delivery_id=NEW.sharing_delivery_id
               AND delivery.event_sequence=1 AND delivery.event_kind='intent_committed'
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning preparation request source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_preparation_observed_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=4 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.preparation_observed.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_preparation_observed_v1'
          OR (NEW.accepted=1 AND json_extract(
                 NEW.message_json,'$.observed.installation_identity_digest'
               ) IS NOT NEW.plugin_installation_identity_digest)
          OR NOT EXISTS (
            SELECT 1
              FROM node_compute_plugin_install_plan_preparation_delivery_events delivery
              JOIN node_compute_plugin_install_plan_preparation_observations observation
                ON observation.delivery_id=delivery.delivery_id
               AND observation.preparation_id=delivery.preparation_id
               AND observation.node_id=delivery.node_id
               AND observation.consent_receipt_id=delivery.consent_receipt_id
               AND observation.policy_revision=delivery.policy_revision
               AND observation.policy_digest=delivery.policy_digest
             WHERE delivery.delivery_id=NEW.preparation_delivery_id
               AND delivery.sharing_delivery_id=NEW.sharing_delivery_id
               AND delivery.preparation_id=NEW.preparation_id
               AND delivery.event_sequence=2 AND delivery.event_kind='dispatched'
               AND delivery.detail_code IS NULL
               AND observation.id=NEW.preparation_observation_id
               AND observation.observed_digest=NEW.preparation_observation_digest
               AND observation.policy_snapshot_digest=NEW.policy_snapshot_digest
               AND observation.accepted=NEW.accepted AND observation.replayed=NEW.replayed
               AND (NEW.accepted=0 OR json_extract(
                      observation.observed_json,'$.installation_identity_digest'
                    )=NEW.plugin_installation_identity_digest)
               AND observation.context_ready=0 AND observation.context_json IS NULL
               AND observation.context_digest IS NULL
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning preparation observation source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_snapshot_request_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=5 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.snapshot_request.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_snapshot_request_v1'
          OR json_extract(NEW.message_json,'$.request.installation_identity_digest')
               IS NOT NEW.plugin_installation_identity_digest
          OR NOT EXISTS (
            SELECT 1 FROM node_compute_plugin_install_plan_planning_delivery_events_v2 delivery
             WHERE delivery.planning_delivery_id=NEW.planning_delivery_id
               AND delivery.cloud_session_id=NEW.session_id
               AND delivery.source_sharing_delivery_id=NEW.sharing_delivery_id
               AND delivery.source_preparation_id=NEW.preparation_id
               AND delivery.source_preparation_delivery_id=NEW.preparation_delivery_id
               AND delivery.source_preparation_observation_id=NEW.preparation_observation_id
               AND delivery.source_preparation_observation_digest=NEW.preparation_observation_digest
               AND delivery.source_preparation_request_digest=NEW.preparation_request_digest
               AND delivery.request_digest=NEW.planning_request_digest
               AND delivery.node_id=NEW.agent_id AND delivery.owner_user_id=NEW.owner_user_id
               AND delivery.installation_identity_digest=NEW.plugin_installation_identity_digest
               AND delivery.event_sequence=1 AND delivery.event_kind='intent_committed'
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning snapshot request source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_snapshot_observed_source
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN NEW.message_sequence=6 AND (
          NEW.message_schema!='elon.node_endpoint.planning_bootstrap.snapshot_observed.v1'
          OR json_extract(NEW.message_json,'$.type')
               IS NOT 'node_endpoint_planning_bootstrap_snapshot_observed_v1'
          OR json_extract(NEW.message_json,'$.observed.phase') IS NOT 'blocked'
          OR json_extract(NEW.message_json,'$.observed.snapshot_ready') IS NOT 0
          OR json_type(NEW.message_json,'$.observed.snapshot') IS NOT 'null'
          OR (NEW.accepted=1 AND json_extract(
                 NEW.message_json,'$.observed.installation_identity_digest'
               ) IS NOT NEW.plugin_installation_identity_digest)
          OR NOT EXISTS (
            SELECT 1 FROM node_compute_plugin_install_plan_planning_delivery_events_v2 delivery
             WHERE delivery.id=NEW.planning_observation_event_id
               AND delivery.planning_delivery_id=NEW.planning_delivery_id
               AND delivery.cloud_session_id=NEW.session_id
               AND delivery.request_digest=NEW.planning_request_digest
               AND delivery.observed_digest=NEW.planning_observation_digest
               AND delivery.event_sequence=2 AND delivery.event_kind='observed'
               AND delivery.observed_snapshot_ready=0
               AND delivery.observed_snapshot_json IS NULL
               AND delivery.observed_snapshot_digest IS NULL
               AND json_extract(delivery.observed_json,'$.accepted')=NEW.accepted
               AND json_extract(delivery.observed_json,'$.replayed')=NEW.replayed
               AND (NEW.accepted=0 OR json_extract(
                      delivery.observed_json,'$.installation_identity_digest'
                    )=NEW.plugin_installation_identity_digest)
               AND json_extract(delivery.observed_json,'$.snapshot_ready')=0
               AND json_type(delivery.observed_json,'$.snapshot')='null'
               AND NOT EXISTS (
                 SELECT 1 FROM node_compute_plugin_install_plan_planning_snapshots_v2 snapshot
                  WHERE snapshot.planning_delivery_id=delivery.planning_delivery_id
               )
          )
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning terminal observation source mismatch');
        END;
        "#,
    )?;
    Ok(())
}
