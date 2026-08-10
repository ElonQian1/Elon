//! Strict endpoint-only wire for the inert sharing -> preparation -> planning bootstrap chain.
//!
//! These DTOs are deliberately independent from the legacy agent enums. They carry no task,
//! command, PlanApply, Runtime, Ready, route, outbox, acknowledgement, or lease authority.

pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_CANONICALIZATION: &str = "rfc8785_jcs";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_ALGORITHM: &str = "sha256";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_DIGEST_DOMAIN: &str =
    "ELON_NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_V1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_DIGEST_DOMAIN_SEPARATOR: u8 = 0;

pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_sharing_request_v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_sharing_observed_v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_preparation_request_v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_preparation_observed_v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_snapshot_request_v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_TYPE: &str =
    "node_endpoint_planning_bootstrap_snapshot_observed_v1";

pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.sharing_request.v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.sharing_observed.v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.preparation_request.v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.preparation_observed.v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.snapshot_request.v1";
pub const NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_SCHEMA: &str =
    "elon.node_endpoint.planning_bootstrap.snapshot_observed.v1";

macro_rules! define_planning_bootstrap_message {
    (
        $name:ident, $fields:ident, $payload_field:ident : $payload_ty:ty,
        $message_type:expr, $schema:expr, $sequence:expr, $validator:path
    ) => {
        #[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            #[serde(rename = "type")]
            message_type: String,
            schema: String,
            binding: super::NodeEndpointPlanningBootstrapMessageBindingV1,
            $payload_field: $payload_ty,
        }

        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $fields {
            pub binding: super::NodeEndpointPlanningBootstrapMessageBindingV1Fields,
            pub $payload_field: $payload_ty,
        }

        impl $name {
            pub fn new(fields: $fields) -> Result<Self, &'static str> {
                let binding =
                    super::NodeEndpointPlanningBootstrapMessageBindingV1::new(fields.binding)?;
                binding.validate_for_sequence($sequence)?;
                $validator(&fields.$payload_field, binding.session_binding())?;
                Ok(Self {
                    message_type: $message_type.to_string(),
                    schema: $schema.to_string(),
                    binding,
                    $payload_field: fields.$payload_field,
                })
            }

            pub fn validate(&self) -> Result<(), &'static str> {
                if self.message_type != $message_type || self.schema != $schema {
                    return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_CONTRACT_INVALID");
                }
                self.binding.validate_for_sequence($sequence)?;
                $validator(&self.$payload_field, self.binding.session_binding())
            }

            /// Returns the exact RFC 8785 input object for this wire message. The self-referential
            /// digest slot is omitted; every other envelope, binding, and payload field remains.
            pub fn digest_material(&self) -> Result<serde_json::Value, &'static str> {
                self.validate()?;
                super::digest_material(self)
            }

            pub fn into_fields(self) -> Result<$fields, &'static str> {
                self.validate()?;
                Ok($fields {
                    binding: self.binding.into_fields()?,
                    $payload_field: self.$payload_field,
                })
            }
        }
    };
}

fn digest_material<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, &'static str> {
    let mut material = serde_json::to_value(value)
        .map_err(|_| "NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_MATERIAL_SERIALIZATION_INVALID")?;
    let binding = material
        .as_object_mut()
        .and_then(|message| message.get_mut("binding"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_MATERIAL_BINDING_INVALID")?;
    if binding.remove("message_digest").is_none() {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_DIGEST_MATERIAL_DIGEST_MISSING");
    }
    Ok(material)
}

mod message_binding;
mod preparation;
mod session_binding;
mod sharing;
mod snapshot;
mod validation;

pub use message_binding::{
    NodeEndpointPlanningBootstrapMessageBindingV1,
    NodeEndpointPlanningBootstrapMessageBindingV1Fields,
};
pub use preparation::{
    NodeEndpointPlanningBootstrapPreparationObservedV1,
    NodeEndpointPlanningBootstrapPreparationObservedV1Fields,
    NodeEndpointPlanningBootstrapPreparationRequestV1,
    NodeEndpointPlanningBootstrapPreparationRequestV1Fields,
};
pub use session_binding::{
    NodeEndpointPlanningBootstrapSessionBindingV1,
    NodeEndpointPlanningBootstrapSessionBindingV1Fields,
};
pub use sharing::{
    NodeEndpointPlanningBootstrapSharingObservedV1,
    NodeEndpointPlanningBootstrapSharingObservedV1Fields,
    NodeEndpointPlanningBootstrapSharingRequestV1,
    NodeEndpointPlanningBootstrapSharingRequestV1Fields,
};
pub use snapshot::{
    NodeEndpointPlanningBootstrapSnapshotObservedV1,
    NodeEndpointPlanningBootstrapSnapshotObservedV1Fields,
    NodeEndpointPlanningBootstrapSnapshotRequestV1,
    NodeEndpointPlanningBootstrapSnapshotRequestV1Fields,
};
