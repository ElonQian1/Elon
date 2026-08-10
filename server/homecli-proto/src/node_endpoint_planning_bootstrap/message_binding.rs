use serde::{Deserialize, Serialize};

use crate::{
    node_endpoint_wire::{bounded_identifier, sha256_digest},
    CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1, NODE_ENDPOINT_SESSION_V2_PROTO_VERSION,
};

use super::{
    NodeEndpointPlanningBootstrapSessionBindingV1,
    NodeEndpointPlanningBootstrapSessionBindingV1Fields,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_CANONICALIZATION,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_ALGORITHM,
};

/// Common, strict chain metadata embedded by each of the six concrete messages.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpointPlanningBootstrapMessageBindingV1 {
    protocol_version: u32,
    capability: String,
    bootstrap_id: String,
    message_sequence: u32,
    session_binding: NodeEndpointPlanningBootstrapSessionBindingV1,
    delivery_id: String,
    previous_message_digest: String,
    canonicalization: String,
    digest_algorithm: String,
    message_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEndpointPlanningBootstrapMessageBindingV1Fields {
    pub bootstrap_id: String,
    pub message_sequence: u32,
    pub session_binding: NodeEndpointPlanningBootstrapSessionBindingV1Fields,
    pub delivery_id: String,
    pub previous_message_digest: String,
    pub message_digest: String,
}

impl NodeEndpointPlanningBootstrapMessageBindingV1 {
    pub fn new(
        fields: NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    ) -> Result<Self, &'static str> {
        let session_binding =
            NodeEndpointPlanningBootstrapSessionBindingV1::new(fields.session_binding)?;
        let value = Self {
            protocol_version: NODE_ENDPOINT_SESSION_V2_PROTO_VERSION,
            capability: CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1.to_string(),
            bootstrap_id: fields.bootstrap_id,
            message_sequence: fields.message_sequence,
            session_binding,
            delivery_id: fields.delivery_id,
            previous_message_digest: fields.previous_message_digest,
            canonicalization: NODE_ENDPOINT_PLANNING_BOOTSTRAP_CANONICALIZATION.to_string(),
            digest_algorithm: NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_ALGORITHM.to_string(),
            message_digest: fields.message_digest,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.session_binding.validate()?;
        if self.protocol_version != NODE_ENDPOINT_SESSION_V2_PROTO_VERSION
            || self.capability != CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1
            || !bounded_identifier(&self.bootstrap_id, 160)
            || !(1..=6).contains(&self.message_sequence)
            || !bounded_identifier(&self.delivery_id, 160)
            || !sha256_digest(&self.previous_message_digest)
            || (self.message_sequence == 1
                && self.previous_message_digest != self.session_binding.authentication_digest())
            || self.canonicalization != NODE_ENDPOINT_PLANNING_BOOTSTRAP_CANONICALIZATION
            || self.digest_algorithm != NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_ALGORITHM
            || !sha256_digest(&self.message_digest)
        {
            return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_BINDING_INVALID");
        }
        Ok(())
    }

    pub fn into_fields(
        self,
    ) -> Result<NodeEndpointPlanningBootstrapMessageBindingV1Fields, &'static str> {
        self.validate()?;
        Ok(NodeEndpointPlanningBootstrapMessageBindingV1Fields {
            bootstrap_id: self.bootstrap_id,
            message_sequence: self.message_sequence,
            session_binding: self.session_binding.into_fields()?,
            delivery_id: self.delivery_id,
            previous_message_digest: self.previous_message_digest,
            message_digest: self.message_digest,
        })
    }

    pub fn bootstrap_id(&self) -> &str {
        &self.bootstrap_id
    }
    pub fn message_sequence(&self) -> u32 {
        self.message_sequence
    }
    pub fn session_binding(&self) -> &NodeEndpointPlanningBootstrapSessionBindingV1 {
        &self.session_binding
    }
    pub fn delivery_id(&self) -> &str {
        &self.delivery_id
    }
    pub fn previous_message_digest(&self) -> &str {
        &self.previous_message_digest
    }
    pub fn message_digest(&self) -> &str {
        &self.message_digest
    }

    pub(super) fn validate_for_sequence(&self, expected: u32) -> Result<(), &'static str> {
        self.validate()?;
        if self.message_sequence != expected {
            return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_SEQUENCE_INVALID");
        }
        Ok(())
    }
}
