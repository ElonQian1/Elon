use anyhow::Result;
use rusqlite::Connection;

use super::precheck::{ENDPOINT_INSTALLATION_EXACT_FUNCTION, PLUGIN_INSTALLATION_EXACT_FUNCTION};

pub(super) fn install(connection: &Connection) -> Result<()> {
    install_binding_guards(connection)?;
    install_activation_guard(connection)?;
    Ok(())
}

fn install_binding_guards(connection: &Connection) -> Result<()> {
    connection.execute_batch(&format!(
        r#"
        DROP TRIGGER IF EXISTS v279_user_node_provider_binding_exact_sources;
        CREATE TRIGGER v279_user_node_provider_binding_exact_sources
        BEFORE INSERT ON compute_user_node_provider_bindings
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers provider
              JOIN compute_provider_versions genesis
                ON genesis.provider_id=provider.provider_id
               AND genesis.policy_revision=NEW.provider_genesis_policy_revision
               AND genesis.provider_digest=NEW.provider_genesis_digest
              JOIN compute_provider_versions current_provider
                ON current_provider.provider_id=provider.provider_id
               AND current_provider.policy_revision=provider.current_policy_revision
               AND current_provider.provider_digest=provider.current_provider_digest
              JOIN node_credentials node ON node.agent_id=NEW.node_id
              JOIN node_endpoint_credentials endpoint
                ON endpoint.agent_id=node.agent_id
               AND endpoint.owner_user_id=node.owner_user_id
               AND endpoint.install_id=node.install_id
              JOIN node_endpoint_credential_versions source_endpoint
                ON source_endpoint.credential_id=NEW.source_endpoint_credential_id
               AND source_endpoint.credential_revision=NEW.source_endpoint_credential_revision
               AND source_endpoint.credential_digest=NEW.source_endpoint_credential_digest
               AND source_endpoint.agent_id=node.agent_id
               AND source_endpoint.owner_user_id=node.owner_user_id
               AND source_endpoint.install_id=node.install_id
               AND source_endpoint.installation_binding_digest=
                    NEW.endpoint_installation_binding_digest
              JOIN node_endpoint_credential_versions current_endpoint
                ON current_endpoint.credential_id=endpoint.credential_id
               AND current_endpoint.credential_revision=endpoint.current_credential_revision
               AND current_endpoint.credential_digest=endpoint.current_credential_digest
               AND current_endpoint.agent_id=endpoint.agent_id
               AND current_endpoint.owner_user_id=endpoint.owner_user_id
               AND current_endpoint.install_id=endpoint.install_id
               AND current_endpoint.installation_binding_digest=
                    endpoint.installation_binding_digest
              JOIN node_compute_sharing_policies policy
                ON policy.node_id=node.agent_id AND policy.owner_user_id=node.owner_user_id
              JOIN node_compute_plugin_sharing_consents source_consent
                ON source_consent.receipt_id=NEW.source_consent_receipt_id
               AND source_consent.node_id=node.agent_id
               AND source_consent.owner_user_id=node.owner_user_id
               AND source_consent.installation_identity_digest=
                    NEW.installation_identity_digest
               AND source_consent.policy_revision=NEW.source_consent_policy_revision
               AND source_consent.policy_digest=NEW.source_consent_policy_digest
               AND source_consent.authorization_ref=NEW.source_authorization_ref
               AND source_consent.authorization_revision=NEW.source_authorization_revision
               AND source_consent.authorization_digest=NEW.source_authorization_digest
              JOIN node_compute_plugin_sharing_consents current_consent
                ON current_consent.receipt_id=policy.plugin_consent_receipt_id
               AND current_consent.node_id=policy.node_id
               AND current_consent.owner_user_id=policy.owner_user_id
               AND current_consent.installation_identity_digest=
                    policy.plugin_installation_identity_digest
               AND current_consent.policy_revision=policy.plugin_policy_revision
               AND current_consent.policy_digest=policy.plugin_policy_digest
               AND current_consent.plugin_runtime_requested=policy.plugin_runtime_requested
               AND current_consent.allowed_model_ids_json=policy.allowed_model_ids_json
               AND current_consent.max_concurrent_runs=policy.max_concurrent_runs
               AND current_consent.daily_token_limit=policy.daily_token_limit
               AND current_consent.authorization_ref IS policy.plugin_authorization_ref
               AND current_consent.authorization_revision IS policy.plugin_authorization_revision
               AND current_consent.authorization_digest IS policy.plugin_authorization_digest
             WHERE provider.provider_id=NEW.provider_id
               AND provider.provider_kind='user_node'
               AND provider.owner_account_id=NEW.owner_user_id
               AND provider.settlement_account_id=NEW.owner_user_id
               AND provider.status='registering'
               AND provider.trust_tier='self_declared'
               AND provider.current_policy_revision>=NEW.provider_genesis_policy_revision
               AND provider.current_policy_revision>=1
               AND json_extract(genesis.provider_json,'$.schema')=
                    'compute_federation.provider.v1'
               AND json_extract(genesis.provider_json,'$.provider_id')=provider.provider_id
               AND json_extract(genesis.provider_json,'$.provider_kind')='user_node'
               AND json_extract(genesis.provider_json,'$.owner_account_id')=
                    provider.owner_account_id
               AND json_extract(genesis.provider_json,'$.settlement_account_id')=
                    provider.owner_account_id
               AND json_extract(genesis.provider_json,'$.status')='registering'
               AND json_extract(genesis.provider_json,'$.trust_tier')='self_declared'
               AND json_extract(genesis.provider_json,'$.policy_revision')=1
               AND json_type(genesis.provider_json,'$.endpoint')='null'
               AND json_type(genesis.provider_json,'$.adapter')='null'
               AND json_extract(current_provider.provider_json,'$.schema')=
                    'compute_federation.provider.v1'
               AND json_extract(current_provider.provider_json,'$.provider_id')=
                    provider.provider_id
               AND json_extract(current_provider.provider_json,'$.provider_kind')=
                    provider.provider_kind
               AND json_extract(current_provider.provider_json,'$.owner_account_id')=
                    provider.owner_account_id
               AND json_extract(current_provider.provider_json,'$.settlement_account_id') IS
                    provider.settlement_account_id
               AND json_extract(current_provider.provider_json,'$.display_name')=
                    provider.display_name
               AND json_extract(current_provider.provider_json,'$.status')=provider.status
               AND json_extract(current_provider.provider_json,'$.trust_tier')=
                    provider.trust_tier
               AND (
                    (provider.home_region IS NULL
                     AND json_type(current_provider.provider_json,'$.home_region')='null')
                    OR (provider.home_region IS NOT NULL
                        AND json_type(current_provider.provider_json,'$.home_region')='text'
                        AND json_extract(current_provider.provider_json,'$.home_region')=
                            provider.home_region)
               )
               AND json_extract(current_provider.provider_json,'$.policy_revision')=
                    provider.current_policy_revision
               AND json_extract(current_provider.provider_json,'$.created_at')=
                    provider.created_at
               AND json_extract(current_provider.provider_json,'$.updated_at')=
                    provider.updated_at
               AND (
                    SELECT COUNT(*) FROM compute_provider_versions lineage
                     WHERE lineage.provider_id=provider.provider_id
                       AND lineage.policy_revision BETWEEN 1 AND provider.current_policy_revision
               )=provider.current_policy_revision
               AND NOT EXISTS (
                    SELECT 1 FROM compute_provider_versions lineage
                     WHERE lineage.provider_id=provider.provider_id
                       AND lineage.policy_revision BETWEEN 1 AND provider.current_policy_revision
                       AND (
                            json_extract(lineage.provider_json,'$.schema') IS NOT
                                'compute_federation.provider.v1'
                            OR json_extract(lineage.provider_json,'$.provider_id') IS NOT
                                provider.provider_id
                            OR json_extract(lineage.provider_json,'$.provider_kind') IS NOT
                                provider.provider_kind
                            OR json_extract(lineage.provider_json,'$.owner_account_id') IS NOT
                                provider.owner_account_id
                            OR json_extract(lineage.provider_json,'$.policy_revision') IS NOT
                                lineage.policy_revision
                       )
               )
               AND node.owner_user_id=NEW.owner_user_id
               AND node.install_id IS NOT NULL
               AND node.install_id=trim(node.install_id)
               AND length(CAST(node.install_id AS BLOB)) BETWEEN 1 AND 256
               AND endpoint.credential_id=NEW.source_endpoint_credential_id
               AND endpoint.status='active'
               AND endpoint.current_credential_revision=
                    NEW.source_endpoint_credential_revision
               AND endpoint.current_credential_digest=NEW.source_endpoint_credential_digest
               AND endpoint.installation_binding_digest=
                    NEW.endpoint_installation_binding_digest
               AND source_endpoint.credential_schema='elon.node_endpoint.credential.v1'
               AND current_endpoint.credential_schema='elon.node_endpoint.credential.v1'
               AND NOT EXISTS (
                    SELECT 1 FROM node_endpoint_credential_revocations revoked
                     WHERE revoked.credential_id=endpoint.credential_id
                       AND revoked.credential_revision=endpoint.current_credential_revision
                       AND revoked.credential_digest=endpoint.current_credential_digest
               )
               AND {ENDPOINT_INSTALLATION_EXACT_FUNCTION}(
                    node.agent_id,node.owner_user_id,node.install_id,
                    NEW.endpoint_installation_binding_digest
               ) IS 1
               AND source_consent.consent_schema=
                    'elon.node_compute_plugin.sharing_consent.v1'
               AND source_consent.plugin_runtime_requested=1
               AND source_consent.authorization_revision=source_consent.policy_revision
               AND source_consent.authorization_digest=source_consent.policy_digest
               AND policy.enabled=1
               AND policy.plugin_runtime_requested=1
               AND policy.plugin_consent_receipt_id=NEW.source_consent_receipt_id
               AND policy.plugin_installation_identity_digest=
                    NEW.installation_identity_digest
               AND policy.plugin_policy_revision=NEW.source_consent_policy_revision
               AND policy.plugin_policy_digest=NEW.source_consent_policy_digest
               AND policy.plugin_authorization_ref=NEW.source_authorization_ref
               AND policy.plugin_authorization_revision=NEW.source_authorization_revision
               AND policy.plugin_authorization_digest=NEW.source_authorization_digest
               AND current_consent.consent_schema=
                    'elon.node_compute_plugin.sharing_consent.v1'
               AND current_consent.plugin_runtime_requested=1
               AND current_consent.authorization_ref IS NOT NULL
               AND current_consent.authorization_revision=current_consent.policy_revision
               AND current_consent.authorization_digest=current_consent.policy_digest
               AND {PLUGIN_INSTALLATION_EXACT_FUNCTION}(
                    node.install_id,NEW.installation_identity_digest
               ) IS 1
        )
        BEGIN
            SELECT RAISE(ABORT,'V279 binding lacks exact Provider/node source continuity');
        END;

        DROP TRIGGER IF EXISTS v279_user_node_provider_binding_no_replace;
        CREATE TRIGGER v279_user_node_provider_binding_no_replace
        BEFORE INSERT ON compute_user_node_provider_bindings
        WHEN EXISTS (
            SELECT 1 FROM compute_user_node_provider_bindings old
             WHERE old.binding_id=NEW.binding_id
                OR old.binding_digest=NEW.binding_digest
                OR old.provider_id=NEW.provider_id
                OR old.node_id=NEW.node_id
                OR (old.idempotency_scope=NEW.idempotency_scope
                    AND old.idempotency_key=NEW.idempotency_key)
        )
        BEGIN
            SELECT RAISE(ABORT,'V279 user-node Provider binding cannot be replaced');
        END;

        DROP TRIGGER IF EXISTS v279_user_node_provider_binding_no_update;
        CREATE TRIGGER v279_user_node_provider_binding_no_update
        BEFORE UPDATE ON compute_user_node_provider_bindings
        BEGIN
            SELECT RAISE(ABORT,'V279 user-node Provider bindings are immutable');
        END;

        DROP TRIGGER IF EXISTS v279_user_node_provider_binding_no_delete;
        CREATE TRIGGER v279_user_node_provider_binding_no_delete
        BEFORE DELETE ON compute_user_node_provider_bindings
        BEGIN
            SELECT RAISE(ABORT,'V279 user-node Provider bindings cannot be deleted');
        END;
        "#,
    ))?;
    Ok(())
}

fn install_activation_guard(connection: &Connection) -> Result<()> {
    connection.execute_batch(&format!(
        r#"
        DROP TRIGGER IF EXISTS v279_user_node_activation_request_current_binding;
        CREATE TRIGGER v279_user_node_activation_request_current_binding
        BEFORE INSERT ON compute_activation_evidence_requests
        WHEN EXISTS (
            SELECT 1 FROM compute_providers kind
             WHERE kind.provider_id=NEW.provider_id AND kind.provider_kind='user_node'
        )
        AND NOT EXISTS (
            SELECT 1
              FROM compute_user_node_provider_bindings binding
              JOIN compute_providers provider ON provider.provider_id=binding.provider_id
              JOIN compute_provider_versions genesis
                ON genesis.provider_id=binding.provider_id
               AND genesis.policy_revision=binding.provider_genesis_policy_revision
               AND genesis.provider_digest=binding.provider_genesis_digest
              JOIN compute_provider_versions current_provider
                ON current_provider.provider_id=provider.provider_id
               AND current_provider.policy_revision=provider.current_policy_revision
               AND current_provider.provider_digest=provider.current_provider_digest
              JOIN node_credentials node ON node.agent_id=binding.node_id
              JOIN node_endpoint_credentials endpoint
                ON endpoint.agent_id=node.agent_id
               AND endpoint.owner_user_id=node.owner_user_id
               AND endpoint.install_id=node.install_id
              JOIN node_endpoint_credential_versions source_endpoint
                ON source_endpoint.credential_id=binding.source_endpoint_credential_id
               AND source_endpoint.credential_revision=
                    binding.source_endpoint_credential_revision
               AND source_endpoint.credential_digest=binding.source_endpoint_credential_digest
               AND source_endpoint.agent_id=node.agent_id
               AND source_endpoint.owner_user_id=node.owner_user_id
               AND source_endpoint.install_id=node.install_id
               AND source_endpoint.installation_binding_digest=
                    binding.endpoint_installation_binding_digest
              JOIN node_endpoint_credential_versions current_endpoint
                ON current_endpoint.credential_id=endpoint.credential_id
               AND current_endpoint.credential_revision=endpoint.current_credential_revision
               AND current_endpoint.credential_digest=endpoint.current_credential_digest
               AND current_endpoint.agent_id=endpoint.agent_id
               AND current_endpoint.owner_user_id=endpoint.owner_user_id
               AND current_endpoint.install_id=endpoint.install_id
               AND current_endpoint.installation_binding_digest=
                    endpoint.installation_binding_digest
              JOIN node_compute_sharing_policies policy
                ON policy.node_id=binding.node_id
               AND policy.owner_user_id=binding.owner_user_id
              JOIN node_compute_plugin_sharing_consents source_consent
                ON source_consent.receipt_id=binding.source_consent_receipt_id
               AND source_consent.node_id=binding.node_id
               AND source_consent.owner_user_id=binding.owner_user_id
               AND source_consent.installation_identity_digest=
                    binding.installation_identity_digest
               AND source_consent.policy_revision=binding.source_consent_policy_revision
               AND source_consent.policy_digest=binding.source_consent_policy_digest
               AND source_consent.authorization_ref=binding.source_authorization_ref
               AND source_consent.authorization_revision=binding.source_authorization_revision
               AND source_consent.authorization_digest=binding.source_authorization_digest
              JOIN node_compute_plugin_sharing_consents current_consent
                ON current_consent.receipt_id=policy.plugin_consent_receipt_id
               AND current_consent.node_id=policy.node_id
               AND current_consent.owner_user_id=policy.owner_user_id
               AND current_consent.installation_identity_digest=
                    policy.plugin_installation_identity_digest
               AND current_consent.policy_revision=policy.plugin_policy_revision
               AND current_consent.policy_digest=policy.plugin_policy_digest
               AND current_consent.plugin_runtime_requested=policy.plugin_runtime_requested
               AND current_consent.allowed_model_ids_json=policy.allowed_model_ids_json
               AND current_consent.max_concurrent_runs=policy.max_concurrent_runs
               AND current_consent.daily_token_limit=policy.daily_token_limit
               AND current_consent.authorization_ref IS policy.plugin_authorization_ref
               AND current_consent.authorization_revision IS policy.plugin_authorization_revision
               AND current_consent.authorization_digest IS policy.plugin_authorization_digest
             WHERE binding.binding_id=NEW.node_binding_ref
               AND binding.provider_id=NEW.provider_id
               AND binding.owner_user_id=NEW.owner_user_id
               AND provider.provider_kind='user_node'
               AND provider.owner_account_id=NEW.owner_user_id
               AND provider.status='registering'
               AND provider.current_policy_revision=NEW.expected_provider_policy_revision
               AND provider.current_provider_digest=NEW.expected_provider_digest
               AND provider.current_policy_revision>=binding.provider_genesis_policy_revision
               AND json_extract(genesis.provider_json,'$.provider_id')=provider.provider_id
               AND json_extract(genesis.provider_json,'$.provider_kind')='user_node'
               AND json_extract(genesis.provider_json,'$.owner_account_id')=
                    provider.owner_account_id
               AND json_extract(genesis.provider_json,'$.policy_revision')=1
               AND json_extract(current_provider.provider_json,'$.provider_id')=
                    provider.provider_id
               AND json_extract(current_provider.provider_json,'$.provider_kind')=
                    provider.provider_kind
               AND json_extract(current_provider.provider_json,'$.owner_account_id')=
                    provider.owner_account_id
               AND json_extract(current_provider.provider_json,'$.status')=provider.status
               AND json_extract(current_provider.provider_json,'$.policy_revision')=
                    provider.current_policy_revision
               AND (
                    SELECT COUNT(*) FROM compute_provider_versions lineage
                     WHERE lineage.provider_id=provider.provider_id
                       AND lineage.policy_revision BETWEEN 1 AND provider.current_policy_revision
               )=provider.current_policy_revision
               AND NOT EXISTS (
                    SELECT 1 FROM compute_provider_versions lineage
                     WHERE lineage.provider_id=provider.provider_id
                       AND lineage.policy_revision BETWEEN 1 AND provider.current_policy_revision
                       AND (
                            json_extract(lineage.provider_json,'$.schema') IS NOT
                                'compute_federation.provider.v1'
                            OR json_extract(lineage.provider_json,'$.provider_id') IS NOT
                                provider.provider_id
                            OR json_extract(lineage.provider_json,'$.provider_kind') IS NOT
                                provider.provider_kind
                            OR json_extract(lineage.provider_json,'$.owner_account_id') IS NOT
                                provider.owner_account_id
                            OR json_extract(lineage.provider_json,'$.policy_revision') IS NOT
                                lineage.policy_revision
                       )
               )
               AND node.owner_user_id=binding.owner_user_id
               AND node.install_id IS NOT NULL
               AND endpoint.credential_id=binding.source_endpoint_credential_id
               AND endpoint.status='active'
               AND endpoint.current_credential_revision>=
                    binding.source_endpoint_credential_revision
               AND endpoint.installation_binding_digest=
                    binding.endpoint_installation_binding_digest
               AND source_endpoint.credential_schema='elon.node_endpoint.credential.v1'
               AND current_endpoint.credential_schema='elon.node_endpoint.credential.v1'
               AND NOT EXISTS (
                    SELECT 1 FROM node_endpoint_credential_revocations revoked
                     WHERE revoked.credential_id=endpoint.credential_id
                       AND revoked.credential_revision=endpoint.current_credential_revision
                       AND revoked.credential_digest=endpoint.current_credential_digest
               )
               AND {ENDPOINT_INSTALLATION_EXACT_FUNCTION}(
                    node.agent_id,node.owner_user_id,node.install_id,
                    binding.endpoint_installation_binding_digest
               ) IS 1
               AND source_consent.consent_schema=
                    'elon.node_compute_plugin.sharing_consent.v1'
               AND source_consent.plugin_runtime_requested=1
               AND source_consent.authorization_revision=source_consent.policy_revision
               AND source_consent.authorization_digest=source_consent.policy_digest
               AND policy.enabled=1
               AND policy.plugin_runtime_requested=1
               AND policy.plugin_installation_identity_digest=
                    binding.installation_identity_digest
               AND policy.plugin_policy_revision>=binding.source_consent_policy_revision
               AND current_consent.consent_schema=
                    'elon.node_compute_plugin.sharing_consent.v1'
               AND current_consent.plugin_runtime_requested=1
               AND current_consent.authorization_ref IS NOT NULL
               AND current_consent.authorization_revision=current_consent.policy_revision
               AND current_consent.authorization_digest=current_consent.policy_digest
               AND {PLUGIN_INSTALLATION_EXACT_FUNCTION}(
                    node.install_id,binding.installation_identity_digest
               ) IS 1
        )
        BEGIN
            SELECT RAISE(ABORT,'V279 user_node activation lacks current exact node binding');
        END;
        "#,
    ))?;
    Ok(())
}
