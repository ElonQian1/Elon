use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, plugin_manifest::SignedComputePluginManifest,
};

pub(super) struct StoredVerificationApplication {
    pub plan_digest: String,
    pub application_request_digest: String,
    pub signed_manifests: Vec<SignedComputePluginManifest>,
}

pub(super) struct CandidateRow {
    pub token: String,
    pub token_digest: String,
    pub plugin_id: String,
    pub slot_ref: String,
    pub generation: i64,
    pub release: ComputePluginReleaseRef,
    pub permission_grant_digest: String,
    pub owner_plan_id: String,
    pub owner_plan_digest: String,
    pub application_inventory_revision: i64,
    pub state: String,
    pub created_at_ms: i64,
}
