use serde::{Deserialize, Serialize};

/// Historical authentication-only endpoint protocol. Its wire meaning is frozen.
pub const NODE_ENDPOINT_SESSION_V1_PROTO_VERSION: u32 = 13;
pub const NODE_ENDPOINT_SESSION_REGISTER_V1_TYPE: &str = "node_endpoint_session_register_v1";
pub const NODE_ENDPOINT_SESSION_ACCEPTED_V1_TYPE: &str = "node_endpoint_session_accepted_v1";
pub const NODE_ENDPOINT_SESSION_REGISTER_V1_SCHEMA: &str = "elon.node_endpoint_session.register.v1";
pub const NODE_ENDPOINT_SESSION_ACCEPTED_V1_SCHEMA: &str = "elon.node_endpoint_session.accepted.v1";
pub const NODE_ENDPOINT_SESSION_MODE_COMPUTE_INERT: &str = "compute_inert";
pub const NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS: u64 = 15 * 60 * 1_000;
pub const NODE_ENDPOINT_SESSION_RENEWAL_MARGIN_MS: u64 = 60 * 1_000;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Validated, compute-inert endpoint registration metadata.
///
/// Wire fields stay private so consumers must use `new` or `into_fields`, both
/// of which enforce the complete standalone v13 contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointSessionRegisterV1 {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointSessionRegisterV1Fields {
    pub agent_id: String,
    pub owner_user_id: String,
    pub install_id: String,
    pub credential_id: String,
    pub credential_revision: u64,
    pub credential_digest: String,
    pub agent_version: String,
}

impl NodeEndpointSessionRegisterV1 {
    pub fn new(fields: NodeEndpointSessionRegisterV1Fields) -> Result<Self, &'static str> {
        validate_register_fields(&fields)?;
        Ok(Self {
            message_type: NODE_ENDPOINT_SESSION_REGISTER_V1_TYPE.to_string(),
            schema: NODE_ENDPOINT_SESSION_REGISTER_V1_SCHEMA.to_string(),
            session_mode: NODE_ENDPOINT_SESSION_MODE_COMPUTE_INERT.to_string(),
            agent_id: fields.agent_id,
            owner_user_id: fields.owner_user_id,
            install_id: fields.install_id,
            credential_id: fields.credential_id,
            credential_revision: fields.credential_revision,
            credential_digest: fields.credential_digest,
            agent_version: fields.agent_version,
            protocol_version: NODE_ENDPOINT_SESSION_V1_PROTO_VERSION,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.message_type != NODE_ENDPOINT_SESSION_REGISTER_V1_TYPE
            || self.schema != NODE_ENDPOINT_SESSION_REGISTER_V1_SCHEMA
            || self.session_mode != NODE_ENDPOINT_SESSION_MODE_COMPUTE_INERT
            || self.protocol_version != NODE_ENDPOINT_SESSION_V1_PROTO_VERSION
        {
            return Err("NODE_ENDPOINT_SESSION_REGISTER_CONTRACT_INVALID");
        }
        validate_register_fields(&NodeEndpointSessionRegisterV1Fields {
            agent_id: self.agent_id.clone(),
            owner_user_id: self.owner_user_id.clone(),
            install_id: self.install_id.clone(),
            credential_id: self.credential_id.clone(),
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest.clone(),
            agent_version: self.agent_version.clone(),
        })
    }

    pub fn into_fields(self) -> Result<NodeEndpointSessionRegisterV1Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointSessionRegisterV1Fields {
            agent_id: self.agent_id,
            owner_user_id: self.owner_user_id,
            install_id: self.install_id,
            credential_id: self.credential_id,
            credential_revision: self.credential_revision,
            credential_digest: self.credential_digest,
            agent_version: self.agent_version,
        })
    }
}

/// Server proof that the standalone endpoint session was authenticated while
/// all compute authority remained disabled.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointSessionAcceptedV1 {
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
    capability_set_digest: String,
    authenticated_at: String,
    expires_at: String,
    expires_in_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointSessionAcceptedV1Fields {
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
    pub capability_set_digest: String,
    pub authenticated_at: String,
    pub expires_at: String,
    pub expires_in_ms: u64,
}

impl NodeEndpointSessionAcceptedV1 {
    pub fn new(fields: NodeEndpointSessionAcceptedV1Fields) -> Result<Self, &'static str> {
        validate_accepted_fields(&fields)?;
        Ok(Self {
            message_type: NODE_ENDPOINT_SESSION_ACCEPTED_V1_TYPE.to_string(),
            schema: NODE_ENDPOINT_SESSION_ACCEPTED_V1_SCHEMA.to_string(),
            accepted: true,
            session_mode: NODE_ENDPOINT_SESSION_MODE_COMPUTE_INERT.to_string(),
            compute_authority: false,
            agent_id: fields.agent_id,
            owner_user_id: fields.owner_user_id,
            install_id: fields.install_id,
            credential_id: fields.credential_id,
            credential_revision: fields.credential_revision,
            credential_digest: fields.credential_digest,
            installation_binding_digest: fields.installation_binding_digest,
            agent_version: fields.agent_version,
            protocol_version: NODE_ENDPOINT_SESSION_V1_PROTO_VERSION,
            session_id: fields.session_id,
            session_generation: fields.session_generation,
            authentication_receipt_id: fields.authentication_receipt_id,
            authentication_digest: fields.authentication_digest,
            server_instance_id: fields.server_instance_id,
            capability_count: 0,
            capability_set_digest: fields.capability_set_digest,
            authenticated_at: fields.authenticated_at,
            expires_at: fields.expires_at,
            expires_in_ms: fields.expires_in_ms,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.message_type != NODE_ENDPOINT_SESSION_ACCEPTED_V1_TYPE
            || self.schema != NODE_ENDPOINT_SESSION_ACCEPTED_V1_SCHEMA
            || !self.accepted
            || self.session_mode != NODE_ENDPOINT_SESSION_MODE_COMPUTE_INERT
            || self.compute_authority
            || self.protocol_version != NODE_ENDPOINT_SESSION_V1_PROTO_VERSION
            || self.capability_count != 0
        {
            return Err("NODE_ENDPOINT_SESSION_ACCEPTED_CONTRACT_INVALID");
        }
        validate_accepted_fields(&NodeEndpointSessionAcceptedV1Fields {
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
            capability_set_digest: self.capability_set_digest.clone(),
            authenticated_at: self.authenticated_at.clone(),
            expires_at: self.expires_at.clone(),
            expires_in_ms: self.expires_in_ms,
        })
    }

    pub fn into_fields(self) -> Result<NodeEndpointSessionAcceptedV1Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointSessionAcceptedV1Fields {
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
            capability_set_digest: self.capability_set_digest,
            authenticated_at: self.authenticated_at,
            expires_at: self.expires_at,
            expires_in_ms: self.expires_in_ms,
        })
    }
}

fn validate_register_fields(
    fields: &NodeEndpointSessionRegisterV1Fields,
) -> Result<(), &'static str> {
    if !bounded_identifier(&fields.agent_id, 160)
        || !bounded_identifier(&fields.owner_user_id, 160)
        || !bounded_identifier(&fields.install_id, 512)
        || !bounded_identifier(&fields.credential_id, 160)
        || !positive_safe_integer(fields.credential_revision)
        || !sha256_digest(&fields.credential_digest)
        || !bounded_identifier(&fields.agent_version, 160)
    {
        return Err("NODE_ENDPOINT_SESSION_REGISTER_FIELDS_INVALID");
    }
    Ok(())
}

fn validate_accepted_fields(
    fields: &NodeEndpointSessionAcceptedV1Fields,
) -> Result<(), &'static str> {
    validate_register_fields(&NodeEndpointSessionRegisterV1Fields {
        agent_id: fields.agent_id.clone(),
        owner_user_id: fields.owner_user_id.clone(),
        install_id: fields.install_id.clone(),
        credential_id: fields.credential_id.clone(),
        credential_revision: fields.credential_revision,
        credential_digest: fields.credential_digest.clone(),
        agent_version: fields.agent_version.clone(),
    })?;
    if !bounded_identifier(&fields.session_id, 160)
        || !positive_safe_integer(fields.session_generation)
        || !bounded_identifier(&fields.authentication_receipt_id, 160)
        || !sha256_digest(&fields.authentication_digest)
        || !sha256_digest(&fields.installation_binding_digest)
        || !bounded_identifier(&fields.server_instance_id, 160)
        || !sha256_digest(&fields.capability_set_digest)
        || !bounded_timestamp(&fields.authenticated_at)
        || !bounded_timestamp(&fields.expires_at)
        || !(1..=NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS).contains(&fields.expires_in_ms)
    {
        return Err("NODE_ENDPOINT_SESSION_ACCEPTED_FIELDS_INVALID");
    }
    Ok(())
}

fn bounded_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= max_bytes
        && !value.chars().any(char::is_control)
}

fn bounded_timestamp(value: &str) -> bool {
    (20..=64).contains(&value.len())
        && value == value.trim()
        && !value.chars().any(char::is_control)
}

fn positive_safe_integer(value: u64) -> bool {
    (1..=MAX_SAFE_INTEGER).contains(&value)
}

fn sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
