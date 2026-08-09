use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{MAX_LEDGER_JSON_BYTES, PLANNING_CANONICALIZATION, PLANNING_DIGEST_ALGORITHM};

const REQUEST_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_REQUEST_V2";
const OBSERVED_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_OBSERVED_V2";
const GENERATION_REQUEST_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_GENERATION_REQUEST_V1";
const GENERATION_OUTCOME_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_GENERATION_OUTCOME_V1";
const SOURCE_PREPARATION_OBSERVED_DOMAIN: &[u8] =
    b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1";
const SOURCE_PREPARATION_REQUEST_DOMAIN: &[u8] =
    b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1";

pub(super) fn planning_request_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    ledger_json_and_digest(REQUEST_DOMAIN, value)
}

pub(super) fn planning_observed_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    ledger_json_and_digest(OBSERVED_DOMAIN, value)
}

pub(super) fn generation_request_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    ledger_json_and_digest(GENERATION_REQUEST_DOMAIN, value)
}

pub(super) fn generation_outcome_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    ledger_json_and_digest(GENERATION_OUTCOME_DOMAIN, value)
}

pub(super) fn hashed_snapshot_json(
    hashed: &homecli_proto::HashedComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<String> {
    if hashed.schema
        != homecli_proto::HASHED_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA
        || hashed.canonicalization != PLANNING_CANONICALIZATION
        || hashed.snapshot_digest_algorithm != PLANNING_DIGEST_ALGORITHM
    {
        bail!("算力插件 Planning Snapshot V2 摘要元数据无效");
    }
    let (_, snapshot_digest) =
        canonical_compute_plugin_ijson_and_sha256(&hashed.snapshot, MAX_LEDGER_JSON_BYTES)?;
    if snapshot_digest != hashed.snapshot_digest {
        bail!("算力插件 Planning Snapshot V2 摘要不匹配");
    }
    Ok(canonical_compute_plugin_ijson_and_sha256(hashed, MAX_LEDGER_JSON_BYTES)?.0)
}

pub(super) fn source_preparation_observed_digest(json: &str) -> Result<String> {
    let value: Value = serde_json::from_str(json)?;
    let (canonical, _) = canonical_compute_plugin_ijson_and_sha256(&value, MAX_LEDGER_JSON_BYTES)?;
    if canonical != json {
        bail!("算力插件 Planning Snapshot V2 来源 observation 不是规范 JSON");
    }
    let mut digest = Sha256::new();
    digest.update(SOURCE_PREPARATION_OBSERVED_DOMAIN);
    digest.update([0]);
    digest.update(canonical.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

pub(super) fn source_preparation_request_digest(
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> String {
    let mut digest = Sha256::new();
    digest.update(SOURCE_PREPARATION_REQUEST_DOMAIN);
    digest.update([0]);
    digest_string(
        &mut digest,
        b"schema",
        homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
    );
    digest_string(&mut digest, b"preparation_id", &request.preparation_id);
    digest_string(&mut digest, b"node_id", &request.node_id);
    digest_string(&mut digest, b"owner_user_id", &request.owner_user_id);
    digest_string(
        &mut digest,
        b"installation_identity_digest",
        &request.installation_identity_digest,
    );
    digest_i64(
        &mut digest,
        b"policy_revision",
        request.policy_revision as i64,
    );
    digest_string(&mut digest, b"policy_digest", &request.policy_digest);
    digest_string(
        &mut digest,
        b"policy_snapshot_digest",
        &request.policy_snapshot_digest,
    );
    digest_string(
        &mut digest,
        b"authorization_ref",
        &request.authorization.authorization_ref,
    );
    digest_i64(
        &mut digest,
        b"authorization_revision",
        request.authorization.revision as i64,
    );
    digest_string(
        &mut digest,
        b"authorization_digest",
        &request.authorization.digest,
    );
    hex::encode(digest.finalize())
}

fn ledger_json_and_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<(String, String)> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_LEDGER_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok((json, hex::encode(digest.finalize())))
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
