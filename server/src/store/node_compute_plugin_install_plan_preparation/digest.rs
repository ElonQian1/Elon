use anyhow::{bail, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::NodeComputePluginInstallPlanPreparationDispatchIntent;
use crate::store::{NodeComputePluginSharingAuthorization, NodeComputePluginSharingDispatchIntent};

const REQUEST_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1";
const CONTEXT_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_CONTEXT_V1";
const OBSERVED_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1";

pub(super) fn preparation_request_digest(
    preparation_id: &str,
    sharing: &NodeComputePluginSharingDispatchIntent,
    authorization: &NodeComputePluginSharingAuthorization,
    policy_snapshot_digest: &str,
) -> String {
    request_digest(
        preparation_id,
        &sharing.node_id,
        &sharing.owner_user_id,
        &sharing.installation_identity_digest,
        sharing.policy_revision,
        &sharing.policy_digest,
        policy_snapshot_digest,
        &authorization.authorization_ref,
        authorization.revision,
        &authorization.digest,
    )
}

pub(super) fn preparation_intent_request_digest(
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
) -> String {
    request_digest(
        &intent.preparation_id,
        &intent.node_id,
        &intent.owner_user_id,
        &intent.installation_identity_digest,
        intent.policy_revision,
        &intent.policy_digest,
        &intent.policy_snapshot_digest,
        &intent.authorization.authorization_ref,
        intent.authorization.revision,
        &intent.authorization.digest,
    )
}

#[allow(clippy::too_many_arguments)]
fn request_digest(
    preparation_id: &str,
    node_id: &str,
    owner_user_id: &str,
    installation_identity_digest: &str,
    policy_revision: i64,
    policy_digest: &str,
    policy_snapshot_digest: &str,
    authorization_ref: &str,
    authorization_revision: i64,
    authorization_digest: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(REQUEST_DOMAIN);
    digest.update([0]);
    digest_string(
        &mut digest,
        b"schema",
        homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
    );
    digest_string(&mut digest, b"preparation_id", preparation_id);
    digest_string(&mut digest, b"node_id", node_id);
    digest_string(&mut digest, b"owner_user_id", owner_user_id);
    digest_string(
        &mut digest,
        b"installation_identity_digest",
        installation_identity_digest,
    );
    digest_i64(&mut digest, b"policy_revision", policy_revision);
    digest_string(&mut digest, b"policy_digest", policy_digest);
    digest_string(
        &mut digest,
        b"policy_snapshot_digest",
        policy_snapshot_digest,
    );
    digest_string(&mut digest, b"authorization_ref", authorization_ref);
    digest_i64(
        &mut digest,
        b"authorization_revision",
        authorization_revision,
    );
    digest_string(&mut digest, b"authorization_digest", authorization_digest);
    hex::encode(digest.finalize())
}

pub(super) fn context_json_and_digest(value: &Value) -> Result<(String, String)> {
    canonical_json_and_digest(CONTEXT_DOMAIN, value)
}

pub(super) fn observed_json_and_digest(value: &Value) -> Result<(String, String)> {
    canonical_json_and_digest(OBSERVED_DOMAIN, value)
}

fn canonical_json_and_digest(domain: &[u8], value: &Value) -> Result<(String, String)> {
    let mut bytes = Vec::new();
    write_canonical_json(value, &mut bytes)?;
    let json = String::from_utf8(bytes.clone())?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(bytes);
    Ok((json, hex::encode(digest.finalize())))
}

fn write_canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) if value.is_i64() || value.is_u64() => {
            output.extend_from_slice(value.to_string().as_bytes());
        }
        Value::Number(_) => bail!("InstallPlan 准备账本不接受浮点 JSON 数字"),
        Value::String(value) => output.extend_from_slice(serde_json::to_string(value)?.as_bytes()),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(serde_json::to_string(key)?.as_bytes());
                output.push(b':');
                write_canonical_json(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn digest_string(digest: &mut Sha256, label: &[u8], value: &str) {
    digest_field(digest, label, value.as_bytes());
}

fn digest_i64(digest: &mut Sha256, label: &[u8], value: i64) {
    digest_field(digest, label, &value.to_be_bytes());
}

fn digest_field(digest: &mut Sha256, label: &[u8], value: &[u8]) {
    digest.update((label.len() as u64).to_be_bytes());
    digest.update(label);
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}
