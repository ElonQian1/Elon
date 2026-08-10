use anyhow::Result;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::node_compute_sharing::endpoint_authority::NodeEndpointCredentialBinding;

use super::Store;

mod bearer_currentness;
mod currentness;
mod mutations;
mod normalize;

use bearer_currentness::require_registration_bearer_current_on;
use currentness::{
    endpoint_authority_at_end_on, endpoint_authority_at_start_on,
    require_legacy_credential_current_on, require_legacy_endpoint_authority_absent_on,
    verify_legacy_secret_proof_on,
};
use mutations::{
    create_legacy_credential_on, renew_by_existing_secret_on, renew_by_install_id_on,
    renew_by_legacy_device_on,
};
use normalize::{required_trimmed, NormalizedRegistrationRequest};

pub(crate) struct LegacyNodeRegistrationRequest<'a> {
    owner_user_id: &'a str,
    proposed_agent_id: &'a str,
    new_secret_hash: &'a str,
    existing_agent_id: Option<&'a str>,
    existing_secret_hash: Option<&'a str>,
    install_id: Option<&'a str>,
    label: Option<&'a str>,
    device_name: Option<&'a str>,
    current_bearer_token: Option<&'a str>,
}

impl<'a> LegacyNodeRegistrationRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        owner_user_id: &'a str,
        proposed_agent_id: &'a str,
        new_secret_hash: &'a str,
        existing_agent_id: Option<&'a str>,
        existing_secret_hash: Option<&'a str>,
        install_id: Option<&'a str>,
        label: Option<&'a str>,
        device_name: Option<&'a str>,
    ) -> Self {
        Self {
            owner_user_id,
            proposed_agent_id,
            new_secret_hash,
            existing_agent_id,
            existing_secret_hash,
            install_id,
            label,
            device_name,
            current_bearer_token: None,
        }
    }

    pub(crate) fn with_current_bearer_token(mut self, bearer_token: &'a str) -> Self {
        self.current_bearer_token = Some(bearer_token);
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LegacyNodeEndpointAuthority {
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    status: String,
}

impl LegacyNodeEndpointAuthority {
    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl From<NodeEndpointCredentialBinding> for LegacyNodeEndpointAuthority {
    fn from(current: NodeEndpointCredentialBinding) -> Self {
        Self {
            agent_id: current.agent_id().to_string(),
            owner_user_id: current.owner_user_id().to_string(),
            install_id: current.install_id().to_string(),
            credential_id: current.credential_id().to_string(),
            credential_revision: current.credential_revision(),
            credential_digest: current.credential_digest().to_string(),
            status: current.status().to_string(),
        }
    }
}

pub(crate) enum LegacyNodeRegistrationOutcome {
    Renewed {
        agent_id: String,
    },
    Created {
        agent_id: String,
    },
    EndpointAuthorityRequired {
        endpoint_authority: LegacyNodeEndpointAuthority,
    },
}

pub(crate) struct LegacyNodeWebSocketAuthCandidate {
    database_secret_hash: Option<String>,
    owner_user_id: Option<String>,
    install_id: Option<String>,
}

impl LegacyNodeWebSocketAuthCandidate {
    pub(crate) fn database_secret_hash(&self) -> Option<&str> {
        self.database_secret_hash.as_deref()
    }

    pub(crate) fn is_database_bound(&self) -> bool {
        self.database_secret_hash.is_some()
    }

    pub(crate) fn owner_user_id(&self) -> Option<&str> {
        self.owner_user_id.as_deref()
    }

    pub(crate) fn install_id(&self) -> Option<&str> {
        self.install_id.as_deref()
    }
}

impl Store {
    /// Resolve every legacy registration branch inside one write transaction.
    ///
    /// Callers may invoke this synchronous method while holding the AgentManager write fence.
    pub(crate) fn register_or_renew_legacy_node_credential(
        &self,
        request: LegacyNodeRegistrationRequest<'_>,
    ) -> Result<LegacyNodeRegistrationOutcome> {
        let request = NormalizedRegistrationRequest::new(request)?;
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_registration_bearer_current_on(&transaction, &request)?;

        if let Some(endpoint_authority) = endpoint_authority_at_start_on(&transaction, &request)? {
            transaction.rollback()?;
            return Ok(LegacyNodeRegistrationOutcome::EndpointAuthorityRequired {
                endpoint_authority,
            });
        }

        let outcome = if let Some(agent_id) = renew_by_install_id_on(&transaction, &request)? {
            LegacyNodeRegistrationOutcome::Renewed { agent_id }
        } else if let Some(agent_id) = renew_by_existing_secret_on(&transaction, &request)? {
            LegacyNodeRegistrationOutcome::Renewed { agent_id }
        } else if let Some(agent_id) = renew_by_legacy_device_on(&transaction, &request)? {
            LegacyNodeRegistrationOutcome::Renewed { agent_id }
        } else {
            create_legacy_credential_on(&transaction, &request)?;
            LegacyNodeRegistrationOutcome::Created {
                agent_id: request.proposed_agent_id.to_string(),
            }
        };

        let final_agent_id = match &outcome {
            LegacyNodeRegistrationOutcome::Renewed { agent_id }
            | LegacyNodeRegistrationOutcome::Created { agent_id } => agent_id,
            LegacyNodeRegistrationOutcome::EndpointAuthorityRequired { .. } => unreachable!(),
        };
        if let Some(endpoint_authority) =
            endpoint_authority_at_end_on(&transaction, &request, final_agent_id)?
        {
            transaction.rollback()?;
            return Ok(LegacyNodeRegistrationOutcome::EndpointAuthorityRequired {
                endpoint_authority,
            });
        }
        require_registration_bearer_current_on(&transaction, &request)?;

        transaction.commit()?;
        Ok(outcome)
    }

    /// Read the DB credential candidate only after proving that no endpoint root exists.
    pub(crate) fn legacy_node_websocket_auth_candidate(
        &self,
        agent_id: &str,
        claimed_owner_user_id: Option<&str>,
        claimed_install_id: Option<&str>,
    ) -> Result<LegacyNodeWebSocketAuthCandidate> {
        let agent_id = required_trimmed(agent_id, "LEGACY_NODE_AGENT_ID_INVALID")?;
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let anchor = transaction
            .query_row(
                "SELECT owner_user_id, install_id
                   FROM node_credentials
                  WHERE agent_id=?1",
                params![agent_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let authoritative_owner_user_id = anchor
            .as_ref()
            .map(|(owner_user_id, _)| owner_user_id.as_str())
            .or(claimed_owner_user_id);
        let authoritative_install_id = anchor
            .as_ref()
            .and_then(|(_, install_id)| install_id.as_deref())
            .filter(|value| !value.trim().is_empty())
            .or(claimed_install_id);
        require_legacy_endpoint_authority_absent_on(
            &transaction,
            agent_id,
            authoritative_owner_user_id,
            authoritative_install_id,
        )?;
        let database_secret_hash = if anchor.is_some() {
            transaction
                .query_row(
                    "SELECT secret_hash FROM node_credentials WHERE agent_id=?1",
                    params![agent_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
        } else {
            None
        };
        transaction.commit()?;
        Ok(match anchor {
            Some((owner_user_id, install_id)) => LegacyNodeWebSocketAuthCandidate {
                database_secret_hash,
                owner_user_id: Some(owner_user_id),
                install_id,
            },
            None => LegacyNodeWebSocketAuthCandidate {
                database_secret_hash: None,
                owner_user_id: None,
                install_id: None,
            },
        })
    }

    /// Revalidate the authenticated namespace before legacy handshake side effects.
    /// A missing durable install binding may still be filled by the guarded preparation step.
    pub(crate) fn require_legacy_node_websocket_preparation_current(
        &self,
        agent_id: &str,
        owner_user_id: Option<&str>,
        install_id: Option<&str>,
        expected_database_secret_hash: Option<&str>,
    ) -> Result<()> {
        self.require_legacy_node_websocket_credential_current(
            agent_id,
            owner_user_id,
            install_id,
            expected_database_secret_hash,
            true,
        )
    }

    /// Final install gate used under the AgentManager write fence.
    ///
    /// Endpoint-root currentness is checked before the durable DB secret is read. A missing
    /// expected DB hash means the handshake used the environment-only legacy namespace; that
    /// namespace is valid only while no DB credential row exists for the agent.
    pub(crate) fn require_legacy_node_websocket_install_current(
        &self,
        agent_id: &str,
        owner_user_id: Option<&str>,
        install_id: Option<&str>,
        expected_database_secret_hash: Option<&str>,
    ) -> Result<()> {
        self.require_legacy_node_websocket_credential_current(
            agent_id,
            owner_user_id,
            install_id,
            expected_database_secret_hash,
            false,
        )
    }

    fn require_legacy_node_websocket_credential_current(
        &self,
        agent_id: &str,
        owner_user_id: Option<&str>,
        install_id: Option<&str>,
        expected_database_secret_hash: Option<&str>,
        allow_install_enrichment: bool,
    ) -> Result<()> {
        let agent_id = required_trimmed(agent_id, "LEGACY_NODE_AGENT_ID_INVALID")?;
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_legacy_credential_current_on(
            &transaction,
            agent_id,
            owner_user_id,
            install_id,
            expected_database_secret_hash,
            allow_install_enrichment,
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Verify a legacy DB hash proof only while no endpoint root owns this agent/install.
    pub(crate) fn verify_legacy_node_secret_proof(
        &self,
        agent_id: &str,
        owner_user_id: &str,
        presented_secret_hash: &str,
    ) -> Result<()> {
        let agent_id = required_trimmed(agent_id, "节点凭证缺失")?;
        let owner_user_id = required_trimmed(owner_user_id, "节点凭证缺失")?;
        let presented_secret_hash = required_trimmed(presented_secret_hash, "节点凭证缺失")?;
        let mut connection = self.conn.lock().unwrap();
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_legacy_secret_proof_on(
            &transaction,
            agent_id,
            owner_user_id,
            presented_secret_hash,
        )?;
        transaction.commit()?;
        Ok(())
    }
}
