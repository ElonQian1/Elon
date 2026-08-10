use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::node_compute_sharing::endpoint_authority::NodeEndpointCredentialBinding;

use super::credentials;

pub(in crate::store::node_credentials) fn current_node_endpoint_root_by_agent_on(
    connection: &Connection,
    agent_id: &str,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    credentials::current_binding_by_agent_on(connection, agent_id)
}

pub(in crate::store::node_credentials) fn current_node_endpoint_root_by_owner_install_on(
    connection: &Connection,
    owner_user_id: &str,
    install_id: &str,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    let agent_id = connection
        .query_row(
            "SELECT agent_id
               FROM node_endpoint_credentials
              WHERE owner_user_id=?1 AND install_id=?2",
            params![owner_user_id, install_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    agent_id
        .map(|agent_id| credentials::current_binding_by_agent_on(connection, &agent_id))
        .transpose()
        .map(Option::flatten)
}
