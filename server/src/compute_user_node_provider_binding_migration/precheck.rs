use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::{
    compute_federation::user_node_provider_binding::user_node_provider_binding_json_is_canonical,
    compute_plugin_sharing_directive::derive_compute_plugin_installation_identity_digest,
    node_compute_sharing::endpoint_authority::derive_node_endpoint_installation_binding_digest,
};

const RECEIPT_EXACT: &str = "elon_v279_user_node_provider_binding_is_exact";
const PLUGIN_INSTALLATION_EXACT: &str = "elon_v279_compute_plugin_installation_identity_is_exact";
const ENDPOINT_INSTALLATION_EXACT: &str = "elon_v279_node_endpoint_installation_binding_is_exact";

pub(super) fn register(connection: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function(RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(
            context
                .get_raw(0)
                .as_str()
                .ok()
                .is_some_and(user_node_provider_binding_json_is_canonical),
        ))
    })?;
    connection.create_scalar_function(PLUGIN_INSTALLATION_EXACT, 2, flags, |context| {
        let install_id = text(context, 0);
        let expected = text(context, 1);
        Ok(i64::from(install_id.zip(expected).is_some_and(
            |(install_id, expected)| {
                derive_compute_plugin_installation_identity_digest(install_id)
                    .is_ok_and(|actual| actual == expected)
            },
        )))
    })?;
    connection.create_scalar_function(ENDPOINT_INSTALLATION_EXACT, 4, flags, |context| {
        let agent_id = text(context, 0);
        let owner_user_id = text(context, 1);
        let install_id = text(context, 2);
        let expected = text(context, 3);
        Ok(i64::from(
            agent_id
                .zip(owner_user_id)
                .zip(install_id)
                .zip(expected)
                .is_some_and(|(((agent_id, owner_user_id), install_id), expected)| {
                    derive_node_endpoint_installation_binding_digest(
                        agent_id,
                        owner_user_id,
                        install_id,
                    )
                    .is_ok_and(|actual| actual == expected)
                }),
        ))
    })?;
    Ok(())
}

fn text<'a>(context: &'a rusqlite::functions::Context<'a>, index: usize) -> Option<&'a str> {
    context.get_raw(index).as_str().ok()
}

pub(super) fn install(connection: &Connection) -> Result<()> {
    connection.execute_batch(&format!(
        r#"
        DROP TRIGGER IF EXISTS v279_user_node_provider_binding_receipt_integrity;
        CREATE TRIGGER v279_user_node_provider_binding_receipt_integrity
        BEFORE INSERT ON compute_user_node_provider_bindings
        WHEN {RECEIPT_EXACT}(NEW.binding_json) IS NOT 1
          OR json_extract(NEW.binding_json,'$.schema') IS NOT NEW.binding_schema
          OR json_extract(NEW.binding_json,'$.binding_digest') IS NOT NEW.binding_digest
          OR json_extract(NEW.binding_json,'$.binding_material_digest')
               IS NOT NEW.binding_material_digest
          OR json_extract(NEW.binding_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.binding_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.binding_json,'$.binding.binding_id') IS NOT NEW.binding_id
          OR json_extract(NEW.binding_json,'$.binding.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.binding_json,'$.binding.provider_genesis_policy_revision')
               IS NOT NEW.provider_genesis_policy_revision
          OR json_extract(NEW.binding_json,'$.binding.provider_genesis_digest')
               IS NOT NEW.provider_genesis_digest
          OR json_extract(NEW.binding_json,'$.binding.node_id') IS NOT NEW.node_id
          OR json_extract(NEW.binding_json,'$.binding.owner_user_id') IS NOT NEW.owner_user_id
          OR json_extract(NEW.binding_json,'$.binding.installation_identity_digest')
               IS NOT NEW.installation_identity_digest
          OR json_extract(NEW.binding_json,'$.binding.endpoint_installation_binding_digest')
               IS NOT NEW.endpoint_installation_binding_digest
          OR json_extract(NEW.binding_json,'$.binding.source_endpoint_credential_id')
               IS NOT NEW.source_endpoint_credential_id
          OR json_extract(NEW.binding_json,'$.binding.source_endpoint_credential_revision')
               IS NOT NEW.source_endpoint_credential_revision
          OR json_extract(NEW.binding_json,'$.binding.source_endpoint_credential_digest')
               IS NOT NEW.source_endpoint_credential_digest
          OR json_extract(NEW.binding_json,'$.binding.source_consent_receipt_id')
               IS NOT NEW.source_consent_receipt_id
          OR json_extract(NEW.binding_json,'$.binding.source_consent_policy_revision')
               IS NOT NEW.source_consent_policy_revision
          OR json_extract(NEW.binding_json,'$.binding.source_consent_policy_digest')
               IS NOT NEW.source_consent_policy_digest
          OR json_extract(NEW.binding_json,'$.binding.source_authorization_ref')
               IS NOT NEW.source_authorization_ref
          OR json_extract(NEW.binding_json,'$.binding.source_authorization_revision')
               IS NOT NEW.source_authorization_revision
          OR json_extract(NEW.binding_json,'$.binding.source_authorization_digest')
               IS NOT NEW.source_authorization_digest
          OR json_extract(NEW.binding_json,'$.binding.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.binding_json,'$.binding.idempotency_scope')
               IS NOT NEW.idempotency_scope
          OR json_extract(NEW.binding_json,'$.binding.idempotency_key')
               IS NOT NEW.idempotency_key
          OR json_extract(NEW.binding_json,'$.binding.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.binding_json,'$.binding.bound_at') IS NOT NEW.bound_at
          OR json_extract(NEW.binding_json,'$.binding.recorded_at') IS NOT NEW.recorded_at
          OR json_extract(NEW.binding_json,'$.binding.binding_effect') IS NOT NEW.binding_effect
          OR json_extract(NEW.binding_json,'$.binding.provider_effect') IS NOT NEW.provider_effect
          OR json_extract(NEW.binding_json,'$.binding.capacity_effect') IS NOT NEW.capacity_effect
          OR json_extract(NEW.binding_json,'$.binding.offer_effect') IS NOT NEW.offer_effect
          OR json_extract(NEW.binding_json,'$.binding.readiness_effect') IS NOT NEW.readiness_effect
          OR json_extract(NEW.binding_json,'$.binding.route_effect') IS NOT NEW.route_effect
          OR json_extract(NEW.binding_json,'$.binding.execution_effect') IS NOT NEW.execution_effect
          OR json_extract(NEW.binding_json,'$.binding.settlement_effect')
               IS NOT NEW.settlement_effect
        BEGIN
            SELECT RAISE(ABORT,'V279 user-node Provider binding receipt integrity mismatch');
        END;
        "#,
    ))?;
    Ok(())
}

pub(super) const PLUGIN_INSTALLATION_EXACT_FUNCTION: &str = PLUGIN_INSTALLATION_EXACT;
pub(super) const ENDPOINT_INSTALLATION_EXACT_FUNCTION: &str = ENDPOINT_INSTALLATION_EXACT;
