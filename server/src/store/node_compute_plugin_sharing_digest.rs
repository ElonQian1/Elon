use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    node_compute_plugin_sharing::{
        NodeComputePluginSharingConsentRequest, NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
    },
    node_compute_sharing::UpdateNodeComputeSharingPolicy,
};

#[derive(Serialize)]
struct RequestFacts<'a> {
    schema: &'static str,
    node_id: &'a str,
    owner_user_id: &'a str,
    installation_identity_digest: &'a str,
    expected_policy_revision: i64,
    legacy_sharing_enabled: bool,
    allowed_model_ids: &'a [String],
    max_concurrent_runs: i64,
    daily_token_limit: i64,
    plugin_runtime_requested: bool,
}

pub(super) fn request_facts_digest(
    node: &str,
    owner: &str,
    installation: &str,
    update: &UpdateNodeComputeSharingPolicy,
    models: &[String],
    consent: &NodeComputePluginSharingConsentRequest,
) -> Result<String> {
    digest_payload(
        b"ELON_COMPUTE_PLUGIN_SHARING_REQUEST_FACTS_V1",
        &RequestFacts {
            schema: NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
            node_id: node,
            owner_user_id: owner,
            installation_identity_digest: installation,
            expected_policy_revision: consent.expected_policy_revision,
            legacy_sharing_enabled: update.enabled,
            allowed_model_ids: models,
            max_concurrent_runs: update.max_concurrent_runs,
            daily_token_limit: update.daily_token_limit,
            plugin_runtime_requested: consent.plugin_runtime_requested,
        },
    )
}

#[derive(Serialize)]
struct ResolvedPolicy<'a> {
    schema: &'static str,
    node_id: &'a str,
    owner_user_id: &'a str,
    installation_identity_digest: &'a str,
    policy_revision: i64,
    legacy_sharing_enabled: bool,
    allowed_model_ids: &'a [String],
    max_concurrent_runs: i64,
    daily_token_limit: i64,
    plugin_runtime_requested: bool,
    authorization_ref: Option<&'a str>,
}

pub(super) fn resolved_policy_digest(
    node: &str,
    owner: &str,
    installation: &str,
    revision: i64,
    update: &UpdateNodeComputeSharingPolicy,
    models: &[String],
    requested: bool,
    authorization_ref: Option<&str>,
) -> Result<String> {
    digest_payload(
        b"ELON_COMPUTE_PLUGIN_SHARING_RESOLVED_POLICY_V1",
        &ResolvedPolicy {
            schema: NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
            node_id: node,
            owner_user_id: owner,
            installation_identity_digest: installation,
            policy_revision: revision,
            legacy_sharing_enabled: update.enabled,
            allowed_model_ids: models,
            max_concurrent_runs: update.max_concurrent_runs,
            daily_token_limit: update.daily_token_limit,
            plugin_runtime_requested: requested,
            authorization_ref,
        },
    )
}

fn digest_payload(domain: &[u8], payload: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(payload).context("算力插件策略无法规范序列化")?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(bytes);
    Ok(hex::encode(digest.finalize()))
}
