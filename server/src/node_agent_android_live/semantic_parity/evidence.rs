//! Revalidates signed state receipts and the bytes of every real artifact.
use super::{
    contract::{self, Contract, State},
    signing,
};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Artifact {
    pub path: String,
    pub sha256: String,
}

pub(crate) fn read_bytes(root: &Path, relative: &str, max: usize) -> Result<Vec<u8>> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        bail!("SEMANTIC_ARTIFACT_PATH_INVALID");
    }
    let root = root.canonicalize()?;
    let target = root.join(path).canonicalize()?;
    if !target.starts_with(&root) || !target.is_file() || target.metadata()?.len() > max as u64 {
        bail!("SEMANTIC_ARTIFACT_PATH_OR_SIZE_INVALID");
    }
    Ok(std::fs::read(target)?)
}
pub(crate) fn read_json(root: &Path, relative: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&read_bytes(
        root,
        relative,
        2 * 1024 * 1024,
    )?)?)
}
pub(crate) fn read_artifact(task_root: &Path, artifact: &Artifact) -> Result<Vec<u8>> {
    let bytes = read_bytes(task_root, &artifact.path, 16 * 1024 * 1024)?;
    if contract::digest(&bytes) != artifact.sha256 {
        bail!("SEMANTIC_ARTIFACT_CHANGED: {}", artifact.path);
    }
    Ok(bytes)
}

pub(crate) fn verify_state(
    task_root: &Path,
    envelope: &Value,
    contract: &Contract,
    contract_hash: &str,
    state: &State,
    revision: &str,
    run_id: &str,
    key: &[u8],
) -> Result<Value> {
    let payload = &envelope["payload"];
    signing::verify(
        payload,
        envelope["signature"]
            .as_str()
            .context("SEMANTIC_RECEIPT_UNSIGNED")?,
        key,
    )?;
    if payload["schemaVersion"] != 1
        || payload["taskId"] != contract.task_id
        || payload["stateId"] != state.id
        || payload["contractSha256"] != contract_hash
        || payload["sourceRevision"] != revision
        || payload["runId"] != run_id
    {
        bail!("SEMANTIC_RECEIPT_IDENTITY_MISMATCH");
    }
    let expected_root = project_root(task_root)?;
    let observed_root = PathBuf::from(
        payload["projectRoot"]
            .as_str()
            .context("SEMANTIC_RECEIPT_IDENTITY_MISMATCH")?,
    )
    .canonicalize()?;
    if observed_root != expected_root {
        bail!("SEMANTIC_RECEIPT_PROJECT_MISMATCH");
    }
    let captured = DateTime::parse_from_rfc3339(
        payload["capturedAt"]
            .as_str()
            .context("SEMANTIC_RECEIPT_TIME_INVALID")?,
    )?;
    let age = Utc::now().signed_duration_since(captured).num_seconds();
    if !(-60..=6 * 3600).contains(&age) {
        bail!("SEMANTIC_RECEIPT_EXPIRED: recapture state");
    }
    super::validate_native(
        &payload["nativeProof"]["runtime"],
        &payload["nativeProof"]["integration"],
        revision,
    )?;
    let artifacts: BTreeMap<String, Artifact> =
        serde_json::from_value(payload["artifacts"].clone())?;
    if artifacts.len() != 5 {
        bail!("SEMANTIC_ARTIFACT_SET_INVALID");
    }
    let get = |name: &str| -> Result<Vec<u8>> {
        read_artifact(
            task_root,
            artifacts.get(name).context("SEMANTIC_ARTIFACT_MISSING")?,
        )
    };
    let android_image = get("androidImage")?;
    let web_image = get("webImage")?;
    if artifacts["androidImage"].path == artifacts["webImage"].path {
        bail!("SEMANTIC_ARTIFACT_NOT_INDEPENDENT");
    }
    for bytes in [&android_image, &web_image] {
        image::load_from_memory(bytes).context("SEMANTIC_IMAGE_INVALID")?;
    }
    let android_nodes: Vec<Value> = serde_json::from_slice(&get("androidTree")?)?;
    let web_tree: Value = serde_json::from_slice(&get("webTree")?)?;
    let manifest: Value = serde_json::from_slice(&get("webManifest")?)?;
    if manifest["schema"] != "elon.pwa.runtime-capture.v1"
        || manifest["revision"]["sourceRevision"] != revision
        || manifest["revision"]["routeRevision"] != format!("semantic:{contract_hash}:{}", state.id)
        || manifest["artifact"]["sha256"] != artifacts["webImage"].sha256
        || manifest["semanticTree"]["sha256"] != artifacts["webTree"].sha256
        || manifest["expectedPage"] != state.web_capture["expectedPage"]
        || manifest["fixtureProfile"] != state.web_capture["fixtureProfile"]
        || payload["webAuthentication"]["profile"] != state.web_capture["authProfile"]
    {
        bail!("SEMANTIC_WEB_PROVENANCE_MISMATCH");
    }
    let url = reqwest::Url::parse(
        state.web_capture["url"]
            .as_str()
            .context("SEMANTIC_WEB_URL_INVALID")?,
    )?;
    let route = url.join(
        web_tree["route"]
            .as_str()
            .context("SEMANTIC_WEB_ROUTE_MISSING")?,
    )?;
    if route != url
        || manifest["route"]["origin"] != url.origin().ascii_serialization()
        || manifest["route"]["path"] != url.path()
    {
        bail!("SEMANTIC_WEB_ROUTE_MISMATCH");
    }
    if state.web_capture["expectedPage"]["kind"] == "PUBLIC_LOGIN"
        && manifest["authenticationMode"] != "none"
    {
        bail!("SEMANTIC_AUTH_STATE_MISMATCH");
    }
    let result = contract::evaluate(state, &android_nodes, &web_tree)?;
    if result != payload["result"] {
        bail!("SEMANTIC_RECEIPT_RESULT_CHANGED");
    }
    Ok(result)
}

pub(crate) fn project_root(task_root: &Path) -> Result<PathBuf> {
    let root = task_root
        .ancestors()
        .nth(4)
        .context("SEMANTIC_TASK_PATH_INVALID")?
        .canonicalize()?;
    let expected = root.join(".elon/ui-design/tasks").canonicalize()?;
    if task_root
        .parent()
        .context("SEMANTIC_TASK_PATH_INVALID")?
        .canonicalize()?
        != expected
    {
        bail!("SEMANTIC_TASK_PATH_INVALID");
    }
    Ok(root)
}

pub(crate) fn verify_summary(
    path: &Path,
    task_id: &str,
    revision: &str,
    key: &[u8],
) -> Result<Value> {
    let task_root = path
        .parent()
        .context("SEMANTIC_TASK_PATH_INVALID")?
        .canonicalize()?;
    let summary = read_json(&task_root, "cross-platform-verification.json")?;
    if summary["schemaVersion"] != 3
        || summary["verificationMode"] != "SEMANTIC_PARITY"
        || summary["taskId"] != task_id
        || summary["sourceRevision"] != revision
        || summary.get("visualLoss").is_some()
        || summary.get("maxVisualLoss").is_some()
    {
        bail!("SEMANTIC_SUMMARY_INVALID");
    }
    let root = project_root(&task_root)?;
    let (contract, hash) = contract::load(
        &root,
        summary["contractPath"]
            .as_str()
            .context("SEMANTIC_CONTRACT_MISSING")?,
        task_id,
    )?;
    if summary["contractSha256"] != hash {
        bail!("SEMANTIC_CONTRACT_CHANGED");
    }
    let run_id = summary["runId"]
        .as_str()
        .filter(|id| contract::safe_id(id))
        .context("SEMANTIC_RUN_INVALID")?;
    let states = summary["stateReceipts"]
        .as_object()
        .context("SEMANTIC_STATES_MISSING")?;
    if states.len() != contract.states.len() {
        bail!("SEMANTIC_STATES_INCOMPLETE: all declared states required");
    }
    let mut results = Vec::new();
    for state in &contract.states {
        let receipt_path = states
            .get(&state.id)
            .and_then(Value::as_str)
            .context("SEMANTIC_STATES_INCOMPLETE")?;
        let receipt = read_json(&task_root, receipt_path)?;
        results.push(verify_state(
            &task_root, &receipt, &contract, &hash, state, revision, run_id, key,
        )?);
    }
    let mut verified = summary;
    verified["verifiedStates"] = json!(results);
    verified["status"] = json!("PASSED");
    Ok(verified)
}
