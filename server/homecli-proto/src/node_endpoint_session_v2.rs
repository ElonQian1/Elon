use serde::{Deserialize, Serialize};

use crate::node_endpoint_wire::{
    bounded_identifier, bounded_timestamp, positive_safe_integer, sha256_digest,
};

pub const NODE_ENDPOINT_SESSION_V2_PROTO_VERSION: u32 = 14;
/// Current endpoint-session protocol advertised by this crate.
pub const NODE_ENDPOINT_SESSION_PROTO_VERSION: u32 = NODE_ENDPOINT_SESSION_V2_PROTO_VERSION;
pub const CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1: &str =
    "node_endpoint_planning_snapshot_bootstrap_v1";
pub const NODE_ENDPOINT_SESSION_V2_CAPABILITIES: [&str; 1] =
    [CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1];
pub const NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST: &str =
    "20145b73356bd38d50dfeca0ba23e39429395130e5200bc14c565f8a816cf9a8";
pub const NODE_ENDPOINT_SESSION_REGISTER_V2_TYPE: &str = "node_endpoint_session_register_v2";
pub const NODE_ENDPOINT_SESSION_ACCEPTED_V2_TYPE: &str = "node_endpoint_session_accepted_v2";
pub const NODE_ENDPOINT_SESSION_REGISTER_V2_SCHEMA: &str = "elon.node_endpoint_session.register.v2";
pub const NODE_ENDPOINT_SESSION_ACCEPTED_V2_SCHEMA: &str = "elon.node_endpoint_session.accepted.v2";
pub const NODE_ENDPOINT_SESSION_MODE_PLANNING_SNAPSHOT_BOOTSTRAP_ONLY: &str =
    "planning_snapshot_bootstrap_only";

/// Strict v14 registration. The only admissible capability set is the fixed planning-bootstrap
/// singleton; it does not grant execution, Ready, routing, dispatch, or lease authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointSessionRegisterV2 {
    #[serde(rename = "type")]
    message_type: String,
    schema: String,
    session_mode: String,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    agent_version: String,
    protocol_version: u32,
    capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointSessionRegisterV2Fields {
    pub agent_id: String,
    pub owner_user_id: String,
    pub install_id: String,
    pub credential_id: String,
    pub credential_revision: u64,
    pub credential_digest: String,
    pub agent_version: String,
    pub capabilities: Vec<String>,
}

impl NodeEndpointSessionRegisterV2 {
    pub fn new(fields: NodeEndpointSessionRegisterV2Fields) -> Result<Self, &'static str> {
        validate_register_fields(&fields)?;
        Ok(Self {
            message_type: NODE_ENDPOINT_SESSION_REGISTER_V2_TYPE.to_string(),
            schema: NODE_ENDPOINT_SESSION_REGISTER_V2_SCHEMA.to_string(),
            session_mode: NODE_ENDPOINT_SESSION_MODE_PLANNING_SNAPSHOT_BOOTSTRAP_ONLY.to_string(),
            agent_id: fields.agent_id,
            owner_user_id: fields.owner_user_id,
            install_id: fields.install_id,
            credential_id: fields.credential_id,
            credential_revision: fields.credential_revision,
            credential_digest: fields.credential_digest,
            agent_version: fields.agent_version,
            protocol_version: NODE_ENDPOINT_SESSION_V2_PROTO_VERSION,
            capabilities: fields.capabilities,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.message_type != NODE_ENDPOINT_SESSION_REGISTER_V2_TYPE
            || self.schema != NODE_ENDPOINT_SESSION_REGISTER_V2_SCHEMA
            || self.session_mode != NODE_ENDPOINT_SESSION_MODE_PLANNING_SNAPSHOT_BOOTSTRAP_ONLY
            || self.protocol_version != NODE_ENDPOINT_SESSION_V2_PROTO_VERSION
        {
            return Err("NODE_ENDPOINT_SESSION_REGISTER_V2_CONTRACT_INVALID");
        }
        validate_register_fields(&self.fields())
    }

    pub fn into_fields(self) -> Result<NodeEndpointSessionRegisterV2Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointSessionRegisterV2Fields {
            agent_id: self.agent_id,
            owner_user_id: self.owner_user_id,
            install_id: self.install_id,
            credential_id: self.credential_id,
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest,
            agent_version: self.agent_version,
            capabilities: self.capabilities,
        })
    }

    fn fields(&self) -> NodeEndpointSessionRegisterV2Fields {
        NodeEndpointSessionRegisterV2Fields {
            agent_id: self.agent_id.clone(),
            owner_user_id: self.owner_user_id.clone(),
            install_id: self.install_id.clone(),
            credential_id: self.credential_id.clone(),
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest.clone(),
            agent_version: self.agent_version.clone(),
            capabilities: self.capabilities.clone(),
        }
    }
}

/// v14 server acceptance. It proves only authenticated planning-bootstrap transport custody.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointSessionAcceptedV2 {
    #[serde(rename = "type")]
    message_type: String,
    schema: String,
    accepted: bool,
    session_mode: String,
    compute_authority: bool,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    credential_id: String,
    credential_revision: u64,
    credential_digest: String,
    installation_binding_digest: String,
    agent_version: String,
    protocol_version: u32,
    session_id: String,
    session_generation: u64,
    authentication_receipt_id: String,
    authentication_digest: String,
    server_instance_id: String,
    capability_count: u32,
    capabilities: Vec<String>,
    capability_set_digest: String,
    authenticated_at: String,
    expires_at: String,
    expires_in_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointSessionAcceptedV2Fields {
    pub agent_id: String,
    pub owner_user_id: String,
    pub install_id: String,
    pub credential_id: String,
    pub credential_revision: u64,
    pub credential_digest: String,
    pub installation_binding_digest: String,
    pub agent_version: String,
    pub session_id: String,
    pub session_generation: u64,
    pub authentication_receipt_id: String,
    pub authentication_digest: String,
    pub server_instance_id: String,
    pub capabilities: Vec<String>,
    pub capability_set_digest: String,
    pub authenticated_at: String,
    pub expires_at: String,
    pub expires_in_ms: u64,
}

impl NodeEndpointSessionAcceptedV2 {
    pub fn new(fields: NodeEndpointSessionAcceptedV2Fields) -> Result<Self, &'static str> {
        validate_accepted_fields(&fields)?;
        Ok(Self {
            message_type: NODE_ENDPOINT_SESSION_ACCEPTED_V2_TYPE.to_string(),
            schema: NODE_ENDPOINT_SESSION_ACCEPTED_V2_SCHEMA.to_string(),
            accepted: true,
            session_mode: NODE_ENDPOINT_SESSION_MODE_PLANNING_SNAPSHOT_BOOTSTRAP_ONLY.to_string(),
            compute_authority: false,
            agent_id: fields.agent_id,
            owner_user_id: fields.owner_user_id,
            install_id: fields.install_id,
            credential_id: fields.credential_id,
            credential_revision: fields.credential_revision,
            credential_digest: fields.credential_digest,
            installation_binding_digest: fields.installation_binding_digest,
            agent_version: fields.agent_version,
            protocol_version: NODE_ENDPOINT_SESSION_V2_PROTO_VERSION,
            session_id: fields.session_id,
            session_generation: fields.session_generation,
            authentication_receipt_id: fields.authentication_receipt_id,
            authentication_digest: fields.authentication_digest,
            server_instance_id: fields.server_instance_id,
            capability_count: u32::try_from(fields.capabilities.len())
                .map_err(|_| "NODE_ENDPOINT_SESSION_ACCEPTED_V2_CAPABILITY_COUNT_INVALID")?,
            capabilities: fields.capabilities,
            capability_set_digest: fields.capability_set_digest,
            authenticated_at: fields.authenticated_at,
            expires_at: fields.expires_at,
            expires_in_ms: fields.expires_in_ms,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.message_type != NODE_ENDPOINT_SESSION_ACCEPTED_V2_TYPE
            || self.schema != NODE_ENDPOINT_SESSION_ACCEPTED_V2_SCHEMA
            || !self.accepted
            || self.session_mode != NODE_ENDPOINT_SESSION_MODE_PLANNING_SNAPSHOT_BOOTSTRAP_ONLY
            || self.compute_authority
            || self.protocol_version != NODE_ENDPOINT_SESSION_V2_PROTO_VERSION
            || self.capability_count as usize != self.capabilities.len()
        {
            return Err("NODE_ENDPOINT_SESSION_ACCEPTED_V2_CONTRACT_INVALID");
        }
        validate_accepted_fields(&self.fields())
    }

    pub fn into_fields(self) -> Result<NodeEndpointSessionAcceptedV2Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointSessionAcceptedV2Fields {
            agent_id: self.agent_id,
            owner_user_id: self.owner_user_id,
            install_id: self.install_id,
            credential_id: self.credential_id,
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest,
            installation_binding_digest: self.installation_binding_digest,
            agent_version: self.agent_version,
            session_id: self.session_id,
            session_generation: self.session_generation,
            authentication_receipt_id: self.authentication_receipt_id,
            authentication_digest: self.authentication_digest,
            server_instance_id: self.server_instance_id,
            capabilities: self.capabilities,
            capability_set_digest: self.capability_set_digest,
            authenticated_at: self.authenticated_at,
            expires_at: self.expires_at,
            expires_in_ms: self.expires_in_ms,
        })
    }

    fn fields(&self) -> NodeEndpointSessionAcceptedV2Fields {
        NodeEndpointSessionAcceptedV2Fields {
            agent_id: self.agent_id.clone(),
            owner_user_id: self.owner_user_id.clone(),
            install_id: self.install_id.clone(),
            credential_id: self.credential_id.clone(),
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest.clone(),
            installation_binding_digest: self.installation_binding_digest.clone(),
            agent_version: self.agent_version.clone(),
            session_id: self.session_id.clone(),
            session_generation: self.session_generation,
            authentication_receipt_id: self.authentication_receipt_id.clone(),
            authentication_digest: self.authentication_digest.clone(),
            server_instance_id: self.server_instance_id.clone(),
            capabilities: self.capabilities.clone(),
            capability_set_digest: self.capability_set_digest.clone(),
            authenticated_at: self.authenticated_at.clone(),
            expires_at: self.expires_at.clone(),
            expires_in_ms: self.expires_in_ms,
        }
    }
}

fn validate_register_fields(
    fields: &NodeEndpointSessionRegisterV2Fields,
) -> Result<(), &'static str> {
    if !bounded_identifier(&fields.agent_id, 160)
        || !bounded_identifier(&fields.owner_user_id, 160)
        || !bounded_identifier(&fields.install_id, 512)
        || !bounded_identifier(&fields.credential_id, 160)
        || !positive_safe_integer(fields.credential_revision)
        || !sha256_digest(&fields.credential_digest)
        || !bounded_identifier(&fields.agent_version, 160)
        || !exact_capabilities(&fields.capabilities)
    {
        return Err("NODE_ENDPOINT_SESSION_REGISTER_V2_FIELDS_INVALID");
    }
    Ok(())
}

fn validate_accepted_fields(
    fields: &NodeEndpointSessionAcceptedV2Fields,
) -> Result<(), &'static str> {
    validate_register_fields(&NodeEndpointSessionRegisterV2Fields {
        agent_id: fields.agent_id.clone(),
        owner_user_id: fields.owner_user_id.clone(),
        install_id: fields.install_id.clone(),
        credential_id: fields.credential_id.clone(),
        credential_revision: fields.credential_revision,
        credential_digest: fields.credential_digest.clone(),
        agent_version: fields.agent_version.clone(),
        capabilities: fields.capabilities.clone(),
    })?;
    if !sha256_digest(&fields.installation_binding_digest)
        || !bounded_identifier(&fields.session_id, 160)
        || !positive_safe_integer(fields.session_generation)
        || !bounded_identifier(&fields.authentication_receipt_id, 160)
        || !sha256_digest(&fields.authentication_digest)
        || !bounded_identifier(&fields.server_instance_id, 160)
        || fields.capability_set_digest != NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST
        || !bounded_timestamp(&fields.authenticated_at)
        || !bounded_timestamp(&fields.expires_at)
        || !(1..=crate::NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS).contains(&fields.expires_in_ms)
    {
        return Err("NODE_ENDPOINT_SESSION_ACCEPTED_V2_FIELDS_INVALID");
    }
    Ok(())
}

fn exact_capabilities(values: &[String]) -> bool {
    values.len() == 1 && values[0] == CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1
}
