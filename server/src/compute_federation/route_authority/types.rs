use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_ROUTE_ADAPTER_VERSION_SCHEMA: &str = "compute_federation.route_adapter.v1";
pub(crate) const COMPUTE_ROUTE_CREDENTIAL_SCHEMA: &str = "compute_federation.route_credential.v1";
pub(crate) const COMPUTE_ROUTE_CREDENTIAL_REVOCATION_SCHEMA: &str =
    "compute_federation.route_credential_revocation.v1";
pub(crate) const COMPUTE_ROUTE_AUTHORIZATION_SCHEMA: &str =
    "compute_federation.route_authorization.v1";
pub(crate) const COMPUTE_ROUTE_AUTHORIZATION_SEAL_SCHEMA: &str =
    "compute_federation.route_authorization_seal.v1";
pub(crate) const COMPUTE_SERVICE_ACTOR_AUTHORIZATION_SCHEMA: &str =
    "compute_federation.service_actor_authorization.v1";
pub(crate) const COMPUTE_ROUTE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_ROUTE_DIGEST_ALGORITHM: &str = "sha256";

pub(crate) const COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT: &str = "provider_endpoint";
pub(crate) const COMPUTE_ROUTE_KIND_SERVER_ADAPTER: &str = "server_adapter";
pub(crate) const COMPUTE_PROVIDER_KIND_USER_NODE: &str = "user_node";
pub(crate) const COMPUTE_PROVIDER_KIND_MANAGED_CLUSTER: &str = "managed_cluster";
pub(crate) const COMPUTE_PROVIDER_KIND_EXTERNAL_POOL: &str = "external_pool";

pub(crate) const COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK: &str = "authenticated_ack";
pub(crate) const COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS: &str = "authenticated_events";
pub(crate) const COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START: &str = "cancel_no_start";
pub(crate) const COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT: &str = "idempotent_commit";
pub(crate) const COMPUTE_ROUTE_CAPABILITY_PREPARE: &str = "prepare";
pub(crate) const COMPUTE_ROUTE_CAPABILITY_RECONCILE: &str = "reconcile";
pub(crate) const COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT: i64 = 6;

pub(crate) const COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE: &str = "active";
pub(crate) const COMPUTE_ROUTE_ADAPTER_STATUS_DRAINING: &str = "draining";
pub(crate) const COMPUTE_ROUTE_ADAPTER_STATUS_REVOKED: &str = "revoked";
pub(crate) const COMPUTE_ROUTE_SOURCE_PROVIDER_ACTIVATION: &str = "provider_activation_application";
pub(crate) const COMPUTE_ROUTE_SOURCE_PROVIDER_RECOVERY: &str = "provider_recovery_application";
pub(crate) const COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING: &str = "external_pool_onboarding";
pub(crate) const COMPUTE_ACTOR_PHASE_DISPATCH: &str = "dispatch";
pub(crate) const COMPUTE_ACTOR_PHASE_APPLICATION: &str = "application";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAdapterVersionEnvelope {
    pub schema: String,
    pub adapter_id: String,
    pub adapter_revision: i64,
    pub adapter_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub adapter: ComputeRouteAdapterVersion,
}

/// Registry metadata only. No executable, resolver, path, or network target is represented.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAdapterVersion {
    pub release_version: String,
    pub implementation_digest: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    pub credential_verifier: ComputeRouteCredentialVerifierBinding,
    pub supported_capabilities: Vec<ComputeRouteCapabilityRevision>,
    pub status: String,
    pub registered_by_service_actor_id: String,
    pub actor_authorization_id: String,
    pub actor_authorization_digest: String,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCapabilityRevision {
    pub capability_id: String,
    pub capability_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCredentialVerifierBinding {
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteProviderBinding {
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAdapterBinding {
    pub adapter_id: String,
    pub adapter_revision: i64,
    pub adapter_registry_digest: String,
    pub adapter_release_version: String,
    pub implementation_digest: String,
    pub config_revision: i64,
    /// Opaque exact identifier (1..=512); it is not interpreted as a hash.
    pub config_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteShape {
    pub route_kind: String,
    /// Exact v212 execution-plan route binding. v212 deliberately reuses the canonical v211
    /// `ComputeAttemptAdapterBinding` digest, so this equals `adapter_binding_digest`.
    pub route_binding_digest: String,
    /// Exact v211 `ComputeAttemptAdapterBinding` digest. Registry identity remains separately
    /// anchored by `adapter.adapter_registry_digest`; these two digest domains are not
    /// interchangeable.
    pub adapter_binding_digest: String,
    pub endpoint_id: Option<String>,
    pub endpoint_transport: Option<String>,
    pub adapter: ComputeRouteAdapterBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCredentialEnvelope {
    pub schema: String,
    pub credential_id: String,
    pub credential_revision: i64,
    pub credential_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub credential: ComputeRouteCredential,
}

/// The ref and hint are lookup metadata only; secrets and bearer material are forbidden.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCredential {
    pub provider: ComputeRouteProviderBinding,
    pub route: ComputeRouteShape,
    pub non_bearer_credential_ref: String,
    pub credential_hint: String,
    pub verifier: ComputeRouteCredentialVerifierBinding,
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub verified_by_service_actor_id: String,
    pub actor_authorization_id: String,
    pub actor_authorization_digest: String,
    pub authenticated_at: String,
    pub expires_at: String,
    pub cleanup_expires_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCredentialRevocationEnvelope {
    pub schema: String,
    pub revocation_id: String,
    pub revocation_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub credential_id: String,
    pub credential_revision: i64,
    pub credential_digest: String,
    pub provider_id: String,
    pub reason_code: String,
    pub revoked_by_service_actor_id: String,
    pub actor_authorization_id: String,
    pub actor_authorization_digest: String,
    pub revoked_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCredentialBinding {
    pub credential_id: String,
    pub credential_revision: i64,
    pub credential_digest: String,
    pub expires_at: String,
    pub cleanup_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteCapabilityBinding {
    pub ordinal: i64,
    pub capability_id: String,
    pub capability_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAuthorizationSourceBinding {
    pub source_kind: String,
    pub source_id: String,
    pub source_digest: String,
    pub approved_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAuthorizationEnvelope {
    pub schema: String,
    pub route_authorization_id: String,
    pub route_authorization_revision: i64,
    pub route_authorization_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub authorization: ComputeRouteAuthorization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAuthorization {
    pub provider: ComputeRouteProviderBinding,
    pub executor_id: String,
    pub route: ComputeRouteShape,
    pub credential: ComputeRouteCredentialBinding,
    /// Store requires exactly the six fixed capabilities in canonical ordinal order.
    pub capabilities: Vec<ComputeRouteCapabilityBinding>,
    pub source: ComputeRouteAuthorizationSourceBinding,
    pub verifier: ComputeRouteCredentialVerifierBinding,
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub verified_by_service_actor_id: String,
    pub actor_authorization_id: String,
    pub actor_authorization_digest: String,
    pub authenticated_at: String,
    pub authorized_at: String,
    pub expires_at: String,
    /// Cleanup-only horizon for cancel/reconcile; prepare/commit must stop at `expires_at`.
    pub cleanup_expires_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRouteAuthorizationSealEnvelope {
    pub schema: String,
    pub seal_id: String,
    pub seal_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub route_authorization_id: String,
    pub route_authorization_revision: i64,
    pub route_authorization_digest: String,
    pub adapter_id: String,
    pub adapter_revision: i64,
    pub adapter_registry_digest: String,
    pub credential_id: String,
    pub credential_revision: i64,
    pub credential_digest: String,
    pub capability_count: i64,
    pub capability_set_digest: String,
    pub sealed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeServiceActorAuthorizationEnvelope {
    pub schema: String,
    pub actor_authorization_id: String,
    pub actor_authorization_revision: i64,
    pub actor_authorization_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub authorization: ComputeServiceActorAuthorization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeServiceActorAuthorization {
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub service_actor_id: String,
    pub service_actor_kind: String,
    pub allowed_route_kinds: Vec<String>,
    pub allowed_actor_phases: Vec<String>,
    pub issued_by_user_id: String,
    pub issued_at: String,
    pub valid_until: String,
    pub recorded_at: String,
}
