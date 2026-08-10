use serde::{Deserialize, Serialize};

use crate::{
    node_endpoint_wire::{
        bounded_identifier, bounded_timestamp, positive_safe_integer, sha256_digest,
    },
    NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST,
};

/// Complete wire projection of the exact durable endpoint session used by every bootstrap frame.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointPlanningBootstrapSessionBindingV1 {
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    session_id: String,
    session_generation: u64,
    authentication_receipt_id: String,
    authentication_digest: String,
    server_instance_id: String,
    agent_version: String,
    capability_set_digest: String,
    authenticated_at: String,
    expires_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointPlanningBootstrapSessionBindingV1Fields {
    pub agent_id: String,
    pub owner_user_id: String,
    pub install_id: String,
    pub installation_binding_digest: String,
    pub credential_id: String,
    pub credential_revision: u64,
    pub credential_digest: String,
    pub session_id: String,
    pub session_generation: u64,
    pub authentication_receipt_id: String,
    pub authentication_digest: String,
    pub server_instance_id: String,
    pub agent_version: String,
    pub capability_set_digest: String,
    pub authenticated_at: String,
    pub expires_at: String,
}

impl NodeEndpointPlanningBootstrapSessionBindingV1 {
    pub fn new(
        fields: NodeEndpointPlanningBootstrapSessionBindingV1Fields,
    ) -> Result<Self, &'static str> {
        validate_fields(&fields)?;
        Ok(Self {
            agent_id: fields.agent_id,
            owner_user_id: fields.owner_user_id,
            install_id: fields.install_id,
            installation_binding_digest: fields.installation_binding_digest,
            credential_id: fields.credential_id,
            credential_revision: fields.credential_revision,
            credential_digest: fields.credential_digest,
            session_id: fields.session_id,
            session_generation: fields.session_generation,
            authentication_receipt_id: fields.authentication_receipt_id,
            authentication_digest: fields.authentication_digest,
            server_instance_id: fields.server_instance_id,
            agent_version: fields.agent_version,
            capability_set_digest: fields.capability_set_digest,
            authenticated_at: fields.authenticated_at,
            expires_at: fields.expires_at,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        validate_fields(&self.fields())
    }

    pub fn into_fields(
        self,
    ) -> Result<NodeEndpointPlanningBootstrapSessionBindingV1Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointPlanningBootstrapSessionBindingV1Fields {
            agent_id: self.agent_id,
            owner_user_id: self.owner_user_id,
            install_id: self.install_id,
            installation_binding_digest: self.installation_binding_digest,
            credential_id: self.credential_id,
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest,
            session_id: self.session_id,
            session_generation: self.session_generation,
            authentication_receipt_id: self.authentication_receipt_id,
            authentication_digest: self.authentication_digest,
            server_instance_id: self.server_instance_id,
            agent_version: self.agent_version,
            capability_set_digest: self.capability_set_digest,
            authenticated_at: self.authenticated_at,
            expires_at: self.expires_at,
        })
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
    pub fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
    pub fn install_id(&self) -> &str {
        &self.install_id
    }
    pub fn installation_binding_digest(&self) -> &str {
        &self.installation_binding_digest
    }
    pub fn credential_id(&self) -> &str {
        &self.credential_id
    }
    pub fn credential_revision(&self) -> u64 {
        self.credential_revision
    }
    pub fn credential_digest(&self) -> &str {
        &self.credential_digest
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }
    pub fn authentication_receipt_id(&self) -> &str {
        &self.authentication_receipt_id
    }
    pub fn authentication_digest(&self) -> &str {
        &self.authentication_digest
    }
    pub fn server_instance_id(&self) -> &str {
        &self.server_instance_id
    }
    pub fn agent_version(&self) -> &str {
        &self.agent_version
    }
    pub fn capability_set_digest(&self) -> &str {
        &self.capability_set_digest
    }
    pub fn authenticated_at(&self) -> &str {
        &self.authenticated_at
    }
    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }

    fn fields(&self) -> NodeEndpointPlanningBootstrapSessionBindingV1Fields {
        NodeEndpointPlanningBootstrapSessionBindingV1Fields {
            agent_id: self.agent_id.clone(),
            owner_user_id: self.owner_user_id.clone(),
            install_id: self.install_id.clone(),
            installation_binding_digest: self.installation_binding_digest.clone(),
            credential_id: self.credential_id.clone(),
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest.clone(),
            session_id: self.session_id.clone(),
            session_generation: self.session_generation,
            authentication_receipt_id: self.authentication_receipt_id.clone(),
            authentication_digest: self.authentication_digest.clone(),
            server_instance_id: self.server_instance_id.clone(),
            agent_version: self.agent_version.clone(),
            capability_set_digest: self.capability_set_digest.clone(),
            authenticated_at: self.authenticated_at.clone(),
            expires_at: self.expires_at.clone(),
        }
    }
}

fn validate_fields(
    fields: &NodeEndpointPlanningBootstrapSessionBindingV1Fields,
) -> Result<(), &'static str> {
    if !bounded_identifier(&fields.agent_id, 160)
        || !bounded_identifier(&fields.owner_user_id, 160)
        || !bounded_identifier(&fields.install_id, 512)
        || !sha256_digest(&fields.installation_binding_digest)
        || !bounded_identifier(&fields.credential_id, 160)
        || !positive_safe_integer(fields.credential_revision)
        || !sha256_digest(&fields.credential_digest)
        || !bounded_identifier(&fields.session_id, 160)
        || !positive_safe_integer(fields.session_generation)
        || !bounded_identifier(&fields.authentication_receipt_id, 160)
        || !sha256_digest(&fields.authentication_digest)
        || !bounded_identifier(&fields.server_instance_id, 160)
        || !bounded_identifier(&fields.agent_version, 160)
        || fields.capability_set_digest != NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST
        || !bounded_timestamp(&fields.authenticated_at)
        || !bounded_timestamp(&fields.expires_at)
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SESSION_BINDING_INVALID");
    }
    Ok(())
}
