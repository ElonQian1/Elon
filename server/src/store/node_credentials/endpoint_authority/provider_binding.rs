use std::marker::PhantomData;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::node_compute_sharing::endpoint_authority::NodeEndpointCredentialBinding;

use super::credentials;

/// A transaction-scoped current endpoint credential for the user-node Provider binding kernel.
/// It does not authorize a socket, Ready claim, route, dispatch, or compute execution.
pub(in crate::store) struct CurrentNodeEndpointCredentialForUserNodeProviderBinding<'conn> {
    binding: NodeEndpointCredentialBinding,
    _connection: PhantomData<&'conn Connection>,
}

impl CurrentNodeEndpointCredentialForUserNodeProviderBinding<'_> {
    pub(in crate::store) fn binding(&self) -> &NodeEndpointCredentialBinding {
        &self.binding
    }
}

pub(in crate::store) fn current_node_endpoint_credential_for_user_node_provider_binding_on<
    'conn,
>(
    connection: &'conn Connection,
    node_id: &str,
    expected_owner_user_id: &str,
    source_credential_id: &str,
    source_credential_revision: i64,
    source_credential_digest: &str,
    expected_installation_binding_digest: &str,
) -> Result<Option<CurrentNodeEndpointCredentialForUserNodeProviderBinding<'conn>>> {
    let Some(current) = current_node_endpoint_credential_source_for_user_node_provider_binding_on(
        connection,
        node_id,
        expected_owner_user_id,
    )?
    else {
        return Ok(None);
    };
    let binding = current.binding();
    if binding.credential_id() != source_credential_id
        || i64::try_from(binding.credential_revision())? < source_credential_revision
        || binding.installation_binding_digest() != expected_installation_binding_digest
    {
        return Ok(None);
    }
    let source_exists = connection
        .query_row(
            "SELECT 1 FROM node_endpoint_credential_versions
              WHERE credential_id=?1 AND credential_revision=?2 AND credential_digest=?3
                AND agent_id=?4 AND owner_user_id=?5 AND installation_binding_digest=?6",
            params![
                source_credential_id,
                source_credential_revision,
                source_credential_digest,
                node_id,
                expected_owner_user_id,
                expected_installation_binding_digest,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !source_exists {
        anyhow::bail!("NODE_ENDPOINT_CREDENTIAL_BINDING_SOURCE_MISSING");
    }
    Ok(Some(current))
}

pub(in crate::store) fn current_node_endpoint_credential_source_for_user_node_provider_binding_on<
    'conn,
>(
    connection: &'conn Connection,
    node_id: &str,
    expected_owner_user_id: &str,
) -> Result<Option<CurrentNodeEndpointCredentialForUserNodeProviderBinding<'conn>>> {
    let Some(binding) = credentials::current_binding_by_agent_on(connection, node_id)? else {
        return Ok(None);
    };
    if binding.owner_user_id() != expected_owner_user_id || binding.status() != "active" {
        return Ok(None);
    }
    credentials::require_current_binding_on(connection, &binding)?;
    Ok(Some(
        CurrentNodeEndpointCredentialForUserNodeProviderBinding {
            binding,
            _connection: PhantomData,
        },
    ))
}
