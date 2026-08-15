use serde::{Deserialize, Serialize};

mod receipts;
mod roots;

pub(crate) use receipts::*;
pub(crate) use roots::*;

pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_ACTIVATION_ROOT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_active_successor_activation_root.v1";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_active_successor_receipt.v1";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_active_successor_revocation.v1";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES: usize = 1024 * 1024;
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_MAX_OBSERVATION_SECONDS: i64 = 15;

pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND: &str =
    "provider_active_successor_receipt";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND: &str =
    "provider_active_successor_revocation";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_ACTOR_KIND: &str = "platform_admin";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_provider_active_successor_revocation";
pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorProviderEvidence {
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_json: String,
    pub provider_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorEffects {
    pub credential_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub activation_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorReadiness {
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_ready: bool,
    pub runtime_launch_ready: bool,
    pub route_ready: bool,
    pub execution_ready: bool,
    pub activation_ready: bool,
}

/// Process-private custody commitments. Deliberately has no `Debug` implementation.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
    pub process_custody_epoch_digest: String,
    pub process_custody_nonce_digest: String,
    pub process_custody_seal_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorCurrentness {
    pub active_successor_receipt_id: String,
    pub active_successor_receipt_digest: String,
    pub provider_binding_id: String,
    pub activation_root_digest: String,
    pub successor_sequence: u64,
    pub evidence_provider_id: String,
    pub evidence_provider_policy_revision: i64,
    pub evidence_provider_digest: String,
    pub checked_at: String,
    pub observation_expires_at: String,
    pub current_status: String,
}
