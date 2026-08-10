use anyhow::{bail, Result};
use chrono::SecondsFormat;
use serde::{de::DeserializeOwned, Serialize};

use super::super::node_credentials::NodeEndpointSessionPermit;

const PLACEHOLDER_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_MESSAGE_BYTES: usize = 1_048_576;

pub(super) struct BuiltMessage<T> {
    pub(super) value: T,
    pub(super) json: String,
    pub(super) digest: String,
    pub(super) binding: homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields,
}

pub(super) struct ValidatedObserved<T> {
    pub(super) payload: T,
    pub(super) json: String,
    pub(super) digest: String,
    pub(super) binding: homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields,
}

pub(super) fn build_sharing_request(
    permit: &NodeEndpointSessionPermit,
    bootstrap_id: &str,
    delivery_id: &str,
    snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1,
) -> Result<BuiltMessage<homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1>> {
    let placeholder = homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1::new(
        homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1Fields {
            binding: binding_fields(
                permit,
                bootstrap_id,
                1,
                delivery_id,
                permit.binding().authentication_digest(),
                PLACEHOLDER_DIGEST,
            )?,
            snapshot: snapshot.clone(),
        },
    )
    .map_err(anyhow::Error::msg)?;
    let digest = digest_for(&placeholder)?;
    finish_request(
        homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1::new(
            homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1Fields {
                binding: binding_fields(
                    permit,
                    bootstrap_id,
                    1,
                    delivery_id,
                    permit.binding().authentication_digest(),
                    &digest,
                )?,
                snapshot,
            },
        )
        .map_err(anyhow::Error::msg)?,
        digest,
    )
}

pub(super) fn build_preparation_request(
    permit: &NodeEndpointSessionPermit,
    bootstrap_id: &str,
    delivery_id: &str,
    previous_digest: &str,
    request: homecli_proto::ComputePluginInstallPlanPreparationRequestV1,
) -> Result<BuiltMessage<homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1>> {
    let placeholder = homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1::new(
        homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1Fields {
            binding: binding_fields(
                permit,
                bootstrap_id,
                3,
                delivery_id,
                previous_digest,
                PLACEHOLDER_DIGEST,
            )?,
            request: request.clone(),
        },
    )
    .map_err(anyhow::Error::msg)?;
    let digest = digest_for(&placeholder)?;
    finish_request(
        homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1::new(
            homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1Fields {
                binding: binding_fields(
                    permit,
                    bootstrap_id,
                    3,
                    delivery_id,
                    previous_digest,
                    &digest,
                )?,
                request,
            },
        )
        .map_err(anyhow::Error::msg)?,
        digest,
    )
}

pub(super) fn build_snapshot_request(
    permit: &NodeEndpointSessionPermit,
    bootstrap_id: &str,
    delivery_id: &str,
    previous_digest: &str,
    request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> Result<BuiltMessage<homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1>> {
    let placeholder = homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1::new(
        homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1Fields {
            binding: binding_fields(
                permit,
                bootstrap_id,
                5,
                delivery_id,
                previous_digest,
                PLACEHOLDER_DIGEST,
            )?,
            request: request.clone(),
        },
    )
    .map_err(anyhow::Error::msg)?;
    let digest = digest_for(&placeholder)?;
    finish_request(
        homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1::new(
            homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1Fields {
                binding: binding_fields(
                    permit,
                    bootstrap_id,
                    5,
                    delivery_id,
                    previous_digest,
                    &digest,
                )?,
                request,
            },
        )
        .map_err(anyhow::Error::msg)?,
        digest,
    )
}

pub(super) fn validate_sharing_observed(
    permit: &NodeEndpointSessionPermit,
    expected: &BuiltMessageRef<'_>,
    value: &homecli_proto::NodeEndpointPlanningBootstrapSharingObservedV1,
) -> Result<ValidatedObserved<homecli_proto::ComputePluginSharingPolicyObservedV1>> {
    let fields = value.clone().into_fields().map_err(anyhow::Error::msg)?;
    validate_observed_binding(permit, expected, &fields.binding, 2)?;
    let (json, digest) = validate_message(value, &fields.binding)?;
    Ok(ValidatedObserved {
        payload: fields.observed,
        json,
        digest,
        binding: fields.binding,
    })
}

pub(super) fn validate_preparation_observed(
    permit: &NodeEndpointSessionPermit,
    expected: &BuiltMessageRef<'_>,
    value: &homecli_proto::NodeEndpointPlanningBootstrapPreparationObservedV1,
) -> Result<ValidatedObserved<homecli_proto::ComputePluginInstallPlanPreparationObservedV1>> {
    let fields = value.clone().into_fields().map_err(anyhow::Error::msg)?;
    validate_observed_binding(permit, expected, &fields.binding, 4)?;
    let (json, digest) = validate_message(value, &fields.binding)?;
    Ok(ValidatedObserved {
        payload: fields.observed,
        json,
        digest,
        binding: fields.binding,
    })
}

pub(super) fn validate_snapshot_observed(
    permit: &NodeEndpointSessionPermit,
    expected: &BuiltMessageRef<'_>,
    value: &homecli_proto::NodeEndpointPlanningBootstrapSnapshotObservedV1,
) -> Result<ValidatedObserved<homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2>> {
    let fields = value.clone().into_fields().map_err(anyhow::Error::msg)?;
    validate_observed_binding(permit, expected, &fields.binding, 6)?;
    let (json, digest) = validate_message(value, &fields.binding)?;
    Ok(ValidatedObserved {
        payload: fields.observed,
        json,
        digest,
        binding: fields.binding,
    })
}

pub(super) struct BuiltMessageRef<'a> {
    pub(super) bootstrap_id: &'a str,
    pub(super) delivery_id: &'a str,
    pub(super) message_digest: &'a str,
}

fn validate_observed_binding(
    permit: &NodeEndpointSessionPermit,
    expected: &BuiltMessageRef<'_>,
    actual: &homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    sequence: u32,
) -> Result<()> {
    if actual.bootstrap_id != expected.bootstrap_id
        || actual.message_sequence != sequence
        || actual.delivery_id != expected.delivery_id
        || actual.previous_message_digest != expected.message_digest
        || actual.session_binding != session_binding_fields(permit)?
    {
        bail!("NODE_ENDPOINT_PLANNING_OBSERVATION_BINDING_MISMATCH");
    }
    Ok(())
}

fn finish_request<T>(value: T, digest: String) -> Result<BuiltMessage<T>>
where
    T: Clone + DeserializeOwned + PartialEq + Serialize,
    T: EndpointMessageFields,
{
    let binding = value.binding_fields()?;
    let (json, calculated) = validate_message(&value, &binding)?;
    if calculated != digest {
        bail!("NODE_ENDPOINT_PLANNING_REQUEST_DIGEST_MISMATCH");
    }
    Ok(BuiltMessage {
        value,
        json,
        digest,
        binding,
    })
}

trait EndpointMessageFields {
    fn binding_fields(
        &self,
    ) -> Result<homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields>;
    fn digest_material(&self) -> Result<serde_json::Value>;
}

macro_rules! endpoint_message_fields {
    ($type:ty) => {
        impl EndpointMessageFields for $type {
            fn binding_fields(
                &self,
            ) -> Result<homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields> {
                Ok(self
                    .clone()
                    .into_fields()
                    .map_err(anyhow::Error::msg)?
                    .binding)
            }

            fn digest_material(&self) -> Result<serde_json::Value> {
                <$type>::digest_material(self).map_err(anyhow::Error::msg)
            }
        }
    };
}

endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1);
endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1);
endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1);
endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapSharingObservedV1);
endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapPreparationObservedV1);
endpoint_message_fields!(homecli_proto::NodeEndpointPlanningBootstrapSnapshotObservedV1);

fn validate_message<T>(
    value: &T,
    binding: &homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields,
) -> Result<(String, String)>
where
    T: DeserializeOwned + PartialEq + Serialize + EndpointMessageFields,
{
    let (_, digest) = crate::compute_plugin_sharing_directive::
        compute_plugin_endpoint_planning_message_json_and_digest(&value.digest_material()?)?;
    if digest != binding.message_digest {
        bail!("NODE_ENDPOINT_PLANNING_MESSAGE_DIGEST_MISMATCH");
    }
    let (json, _) =
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            MAX_MESSAGE_BYTES,
        )?;
    let readback: T = serde_json::from_str(&json)?;
    if &readback != value {
        bail!("NODE_ENDPOINT_PLANNING_MESSAGE_CANONICAL_READBACK_MISMATCH");
    }
    Ok((json, digest))
}

fn digest_for<T: EndpointMessageFields>(value: &T) -> Result<String> {
    Ok(crate::compute_plugin_sharing_directive::
        compute_plugin_endpoint_planning_message_json_and_digest(&value.digest_material()?)?
        .1)
}

fn binding_fields(
    permit: &NodeEndpointSessionPermit,
    bootstrap_id: &str,
    message_sequence: u32,
    delivery_id: &str,
    previous_message_digest: &str,
    message_digest: &str,
) -> Result<homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields> {
    Ok(
        homecli_proto::NodeEndpointPlanningBootstrapMessageBindingV1Fields {
            bootstrap_id: bootstrap_id.to_string(),
            message_sequence,
            session_binding: session_binding_fields(permit)?,
            delivery_id: delivery_id.to_string(),
            previous_message_digest: previous_message_digest.to_string(),
            message_digest: message_digest.to_string(),
        },
    )
}

pub(super) fn session_binding_fields(
    permit: &NodeEndpointSessionPermit,
) -> Result<homecli_proto::NodeEndpointPlanningBootstrapSessionBindingV1Fields> {
    permit.require_planning_bootstrap_v14()?;
    let binding = permit.binding();
    Ok(
        homecli_proto::NodeEndpointPlanningBootstrapSessionBindingV1Fields {
            agent_id: binding.agent_id().to_string(),
            owner_user_id: permit.owner_user_id().to_string(),
            install_id: permit.install_id().to_string(),
            installation_binding_digest: permit.installation_binding_digest().to_string(),
            credential_id: binding.credential_id().to_string(),
            credential_revision: binding.credential_revision(),
            credential_digest: binding.credential_digest().to_string(),
            session_id: binding.session_id().to_string(),
            session_generation: binding.session_generation(),
            authentication_receipt_id: binding.authentication_receipt_id().to_string(),
            authentication_digest: binding.authentication_digest().to_string(),
            server_instance_id: binding.server_instance_id().to_string(),
            agent_version: permit.agent_version().to_string(),
            capability_set_digest: permit.capability_set_digest().to_string(),
            authenticated_at: permit
                .authenticated_at()
                .to_rfc3339_opts(SecondsFormat::Nanos, true),
            expires_at: permit
                .expires_at()
                .to_rfc3339_opts(SecondsFormat::Nanos, true),
        },
    )
}
