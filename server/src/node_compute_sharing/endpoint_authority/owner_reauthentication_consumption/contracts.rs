use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::super::{
    credential::{NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope},
    types::{NodeEndpointCredentialBinding, CANONICALIZATION, DIGEST_ALGORITHM},
};

pub(super) const CONSUMPTION_SCHEMA: &str =
    "elon.node_endpoint.owner_reauthentication_consumption.v1";
pub(super) const CONSUMPTION_ID_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_ID_V1";
pub(super) const CONSUMPTION_DIGEST_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_V1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct CredentialMutationResultProjection {
    pub(super) current_credential_id: String,
    pub(super) current_credential_revision: u64,
    pub(super) current_credential_digest: String,
    pub(super) current_credential_status: String,
    pub(super) issued_credential_id: Option<String>,
    pub(super) issued_credential_revision: Option<u64>,
    pub(super) issued_credential_digest: Option<String>,
    pub(super) revocation_id: Option<String>,
    pub(super) revocation_digest: Option<String>,
}

pub(crate) struct NodeEndpointCredentialMutationResultBinding {
    pub(super) projection: CredentialMutationResultProjection,
    pub(super) current: NodeEndpointCredentialBinding,
    pub(super) issued: Option<NodeEndpointCredentialVersionEnvelope>,
    pub(super) issued_digest: Option<String>,
    pub(super) revocation: Option<NodeEndpointCredentialRevocationEnvelope>,
    pub(super) revocation_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeEndpointOwnerReauthenticationConsumptionEnvelope {
    pub(super) schema: String,
    pub(super) consumption_id: String,
    pub(super) reauthentication_receipt_id: String,
    pub(super) reauthentication_digest: String,
    pub(super) owner_user_id: String,
    pub(super) authorization_action: String,
    pub(super) credential_mutation_request_id: String,
    pub(super) credential_mutation_request_digest: String,
    pub(super) authorization_target_digest: String,
    pub(super) credential_result: CredentialMutationResultProjection,
    pub(super) consumed_at: String,
    pub(super) recorded_at: String,
}

pub(crate) struct PreparedNodeEndpointOwnerReauthenticationConsumption {
    pub(super) envelope: NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    pub(super) consumption_json: String,
    pub(super) consumption_digest: String,
}

impl PreparedNodeEndpointOwnerReauthenticationConsumption {
    pub(crate) fn envelope(&self) -> &NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        &self.envelope
    }

    pub(crate) fn consumption_json(&self) -> &str {
        &self.consumption_json
    }

    pub(crate) fn consumption_digest(&self) -> &str {
        &self.consumption_digest
    }

    pub(crate) fn canonicalization(&self) -> &'static str {
        CANONICALIZATION
    }

    pub(crate) fn digest_algorithm(&self) -> &'static str {
        DIGEST_ALGORITHM
    }
}

pub(super) struct ConsumptionTimes {
    pub(super) consumed_at: DateTime<Utc>,
    pub(super) recorded_at: DateTime<Utc>,
}
