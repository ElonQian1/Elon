use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) const USER_NODE_PROVIDER_BINDING_SCHEMA_V1: &str =
    "compute_federation.user_node_provider_binding.v1";
pub(crate) const USER_NODE_PROVIDER_BINDING_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const USER_NODE_PROVIDER_BINDING_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const USER_NODE_PROVIDER_BINDING_CONFIRMATION: &str =
    "confirm_user_node_provider_binding";
pub(crate) const USER_NODE_PROVIDER_BINDING_ID_DOMAIN_V1: &str =
    "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-ID-V1";
pub(crate) const USER_NODE_PROVIDER_BINDING_REQUEST_DOMAIN_V1: &str =
    "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-REQUEST-V1";
pub(crate) const USER_NODE_PROVIDER_BINDING_MATERIAL_DOMAIN_V1: &str =
    "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-MATERIAL-V1";
pub(crate) const USER_NODE_PROVIDER_BINDING_RECEIPT_DOMAIN_V1: &str =
    "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-RECEIPT-V1";
pub(crate) const USER_NODE_PROVIDER_BINDING_IDENTITY_EFFECT: &str = "identity_binding_recorded";
pub(crate) const USER_NODE_PROVIDER_BINDING_NO_EFFECT: &str = "none";
pub(crate) const USER_NODE_PROVIDER_BINDING_GENESIS_POLICY_REVISION: i64 = 1;
pub(crate) const USER_NODE_PROVIDER_BINDING_MAX_JSON_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserNodeProviderBindingMaterial {
    pub(super) binding_id: String,
    pub(super) provider_id: String,
    pub(super) provider_genesis_policy_revision: i64,
    pub(super) provider_genesis_digest: String,
    pub(super) node_id: String,
    pub(super) owner_user_id: String,
    pub(super) installation_identity_digest: String,
    pub(super) endpoint_installation_binding_digest: String,
    pub(super) source_endpoint_credential_id: String,
    pub(super) source_endpoint_credential_revision: i64,
    pub(super) source_endpoint_credential_digest: String,
    pub(super) source_consent_receipt_id: String,
    pub(super) source_consent_policy_revision: i64,
    pub(super) source_consent_policy_digest: String,
    pub(super) source_authorization_ref: String,
    pub(super) source_authorization_revision: i64,
    pub(super) source_authorization_digest: String,
    pub(super) confirmation: String,
    pub(super) idempotency_scope: String,
    pub(super) idempotency_key: String,
    pub(super) request_digest: String,
    pub(super) bound_at: String,
    pub(super) recorded_at: String,
    pub(super) binding_effect: String,
    pub(super) provider_effect: String,
    pub(super) capacity_effect: String,
    pub(super) offer_effect: String,
    pub(super) readiness_effect: String,
    pub(super) route_effect: String,
    pub(super) execution_effect: String,
    pub(super) settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserNodeProviderBindingReceiptV1 {
    pub(super) schema: String,
    pub(super) binding_digest: String,
    pub(super) binding_material_digest: String,
    pub(super) canonicalization: String,
    pub(super) digest_algorithm: String,
    pub(super) binding: UserNodeProviderBindingMaterial,
}

macro_rules! string_getter {
    ($name:ident) => {
        pub(crate) fn $name(&self) -> &str {
            &self.$name
        }
    };
}

impl UserNodeProviderBindingMaterial {
    string_getter!(binding_id);
    string_getter!(provider_id);
    string_getter!(provider_genesis_digest);
    string_getter!(node_id);
    string_getter!(owner_user_id);
    string_getter!(installation_identity_digest);
    string_getter!(endpoint_installation_binding_digest);
    string_getter!(source_endpoint_credential_id);
    string_getter!(source_endpoint_credential_digest);
    string_getter!(source_consent_receipt_id);
    string_getter!(source_consent_policy_digest);
    string_getter!(source_authorization_ref);
    string_getter!(source_authorization_digest);
    string_getter!(confirmation);
    string_getter!(idempotency_scope);
    string_getter!(idempotency_key);
    string_getter!(request_digest);
    string_getter!(bound_at);
    string_getter!(recorded_at);
    string_getter!(binding_effect);
    string_getter!(provider_effect);
    string_getter!(capacity_effect);
    string_getter!(offer_effect);
    string_getter!(readiness_effect);
    string_getter!(route_effect);
    string_getter!(execution_effect);
    string_getter!(settlement_effect);

    pub(crate) fn provider_genesis_policy_revision(&self) -> i64 {
        self.provider_genesis_policy_revision
    }

    pub(crate) fn source_endpoint_credential_revision(&self) -> i64 {
        self.source_endpoint_credential_revision
    }

    pub(crate) fn source_consent_policy_revision(&self) -> i64 {
        self.source_consent_policy_revision
    }

    pub(crate) fn source_authorization_revision(&self) -> i64 {
        self.source_authorization_revision
    }
}

impl UserNodeProviderBindingReceiptV1 {
    string_getter!(schema);
    string_getter!(binding_digest);
    string_getter!(binding_material_digest);
    string_getter!(canonicalization);
    string_getter!(digest_algorithm);

    pub(crate) fn binding_id(&self) -> &str {
        self.binding.binding_id()
    }

    pub(crate) fn binding(&self) -> &UserNodeProviderBindingMaterial {
        &self.binding
    }

    pub(crate) fn binding_json(&self) -> Result<String> {
        super::canonical::canonical_user_node_provider_binding_receipt_json_and_digest(self)
            .map(|(json, _)| json)
    }
}
