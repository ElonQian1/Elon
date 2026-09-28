//! Explicit state-by-state cross-platform observation, with node-authenticated evidence.
use super::broker::{LiveUiBroker, LiveUiSession};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

mod contract;
mod evidence;
mod signing;
mod summary;
#[cfg(test)]
mod tests;
use super::native_runtime_proof::validate as validate_native;

pub(super) async fn write(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
    task_root: &Path,
    task_id: &str,
    revision: &str,
    arguments: &Value,
) -> Result<Value> {
    let root = Path::new(
        session
            .project_root
            .as_deref()
            .context("SEMANTIC_PROJECT_MISSING")?,
    )
    .canonicalize()?;
    let task_root = task_root.canonicalize()?;
    if evidence::project_root(&task_root)? != root {
        bail!("SEMANTIC_TASK_PROJECT_MISMATCH");
    }
    for forbidden in [
        "androidArtifact",
        "webArtifact",
        "visualLoss",
        "maxVisualLoss",
        "sourceWritebackVerified",
        "patchFreeBuildVerified",
    ] {
        if arguments.get(forbidden).is_some() {
            bail!("SEMANTIC_CALLER_PROOF_REJECTED: {forbidden}");
        }
    }
    let contract_path = arguments["semanticContractPath"]
        .as_str()
        .context("SEMANTIC_CONTRACT_MISSING")?;
    let (contract, contract_hash) = contract::load(&root, contract_path, task_id)?;
    let state_id = arguments["stateId"]
        .as_str()
        .context("SEMANTIC_STATE_MISSING")?;
    let state = contract
        .states
        .iter()
        .find(|state| state.id == state_id)
        .context("SEMANTIC_STATE_UNKNOWN")?;
    let key = receipt_key()?;
    let native_proof = super::native_runtime_proof::read(broker, session, revision).await?;
    let run_id = summary::begin(&task_root, task_id, revision, contract_path, &contract_hash)?;
    let (android_nodes, android_frame) = native_snapshot(broker, session).await?;
    let mut web_arguments = state.web_capture.clone();
    web_arguments["evidence"] = json!({"sourceRevision":revision,"routeRevision":format!("semantic:{contract_hash}:{state_id}")});
    let web = crate::node_agent_pwa_runtime::capture_tool(root.to_str(), web_arguments).await;
    if web["ok"] != true {
        bail!(
            "SEMANTIC_WEB_CAPTURE_FAILED: {}",
            web["diagnostic"]["code"].as_str().unwrap_or("UNKNOWN")
        );
    }
    let capture_root = root
        .join(".elon/ui-tuner/pwa-runtime/captures")
        .canonicalize()?;
    let web_png = captured_file(&capture_root, &web["artifact"]["path"])?;
    let web_tree_bytes = captured_file(&capture_root, &web["semanticTree"]["path"])?;
    let web_manifest = captured_file(&capture_root, &web["artifact"]["manifestPath"])?;
    let web_tree: Value = serde_json::from_slice(&web_tree_bytes)?;
    let result = contract::evaluate(state, &android_nodes, &web_tree)?;
    let current = super::native_runtime_proof::read(broker, session, revision).await?;
    if current["runtime"]["sourceProof"] != native_proof["runtime"]["sourceProof"]
        || super::fit_run::workspace_fingerprint(root.to_str().context("SEMANTIC_PROJECT_UTF8")?)?
            .as_deref()
            != Some(revision)
    {
        bail!("SEMANTIC_SOURCE_CHANGED_DURING_CAPTURE");
    }
    let directory = format!(
        "evidence/semantic/{run_id}/{state_id}-{}",
        uuid::Uuid::new_v4().simple()
    );
    create_artifact_directory(&task_root, &directory)?;
    let mut artifacts = BTreeMap::new();
    for (name, filename, bytes) in [
        ("androidImage", "android.webp", android_frame),
        (
            "androidTree",
            "android.json",
            serde_json::to_vec(&android_nodes)?,
        ),
        ("webImage", "web.png", web_png),
        ("webTree", "web.json", web_tree_bytes),
        ("webManifest", "web-manifest.json", web_manifest),
    ] {
        let relative = format!("{directory}/{filename}");
        crate::node_agent_atomic_file::write(&task_root.join(&relative), &bytes)?;
        artifacts.insert(
            name,
            evidence::Artifact {
                path: relative,
                sha256: contract::digest(&bytes),
            },
        );
    }
    let payload = json!({"schemaVersion":1,"taskId":task_id,"stateId":state_id,"runId":run_id,
        "projectRoot":root,"sourceRevision":revision,"contractSha256":contract_hash,
        "capturedAt":chrono::Utc::now().to_rfc3339(),"nativeProof":native_proof,
        "webAuthentication":web["authentication"],"artifacts":artifacts,"result":result});
    let receipt = json!({"signature":signing::sign(&payload, &key)?,"payload":payload});
    evidence::verify_state(
        &task_root,
        &receipt,
        &contract,
        &contract_hash,
        state,
        revision,
        &run_id,
        &key,
    )?;
    let receipt_path = format!("{directory}/receipt.json");
    crate::node_agent_atomic_file::write(
        &task_root.join(&receipt_path),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    let summary = summary::record(&task_root, &run_id, state_id, &receipt_path)?;
    let missing = contract
        .states
        .iter()
        .filter(|state| summary["stateReceipts"].get(&state.id).is_none())
        .map(|state| state.id.clone())
        .collect::<Vec<_>>();
    let verified = if missing.is_empty() {
        evidence::verify_summary(
            &task_root.join("cross-platform-verification.json"),
            task_id,
            revision,
            &key,
        )?
    } else {
        summary
    };
    Ok(
        json!({"capability":"CROSS_PLATFORM_STYLE_WRITEBACK","verificationMode":"SEMANTIC_PARITY",
        "status":if missing.is_empty() {"PASSED"} else {"PARTIAL"},"stateResult":result,"missingStates":missing,
        "evidencePath":task_root.join("cross-platform-verification.json"),"evidence":verified,
        "next":"ui_check_workflow_completion"}),
    )
}

pub(super) fn verify_file(path: &Path, task_id: &str, revision: Option<&str>) -> Result<Value> {
    evidence::verify_summary(
        path,
        task_id,
        revision.context("SEMANTIC_SOURCE_REVISION_REQUIRED")?,
        &receipt_key()?,
    )
}

fn receipt_key() -> Result<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let state = crate::node_agent_config::load_persisted()
        .context("SEMANTIC_RECEIPT_NODE_IDENTITY_MISSING")?;
    let agent = state
        .agent_id
        .filter(|v| !v.is_empty())
        .context("SEMANTIC_RECEIPT_NODE_IDENTITY_MISSING")?;
    let owner = state
        .owner_user_id
        .filter(|v| !v.is_empty())
        .context("SEMANTIC_RECEIPT_NODE_IDENTITY_MISSING")?;
    let secret = state
        .agent_secret
        .filter(|v| v.len() >= 16)
        .context("SEMANTIC_RECEIPT_NODE_IDENTITY_MISSING")?;
    Ok(Sha256::digest(serde_json::to_vec(&(
        "semantic-parity-v1",
        agent,
        owner,
        secret,
    ))?)
    .to_vec())
}

async fn native_snapshot(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
) -> Result<(Vec<Value>, Vec<u8>)> {
    for _ in 0..3 {
        let (_, before) = broker.tree(&session.id).await?;
        let frame = super::frame::capture_runtime_frame_image(session).await?;
        let (_, after) = broker.tree(&session.id).await?;
        let before = serde_json::to_value(before)?;
        if before == serde_json::to_value(after)? {
            return Ok((
                before
                    .as_array()
                    .context("SEMANTIC_NATIVE_TREE_INVALID")?
                    .clone(),
                frame.bytes,
            ));
        }
    }
    bail!("SEMANTIC_NATIVE_STATE_UNSTABLE: tree changed during all three captures");
}
fn captured_file(capture_root: &Path, value: &Value) -> Result<Vec<u8>> {
    let path =
        PathBuf::from(value.as_str().context("SEMANTIC_WEB_ARTIFACT_MISSING")?).canonicalize()?;
    if !path.starts_with(capture_root)
        || !path.is_file()
        || path.metadata()?.len() > 16 * 1024 * 1024
    {
        bail!("SEMANTIC_WEB_ARTIFACT_PATH_INVALID");
    }
    Ok(std::fs::read(path)?)
}
fn create_artifact_directory(root: &Path, relative: &str) -> Result<()> {
    let destination = root.join(relative);
    let mut existing = destination.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .context("SEMANTIC_ARTIFACT_PATH_INVALID")?;
    }
    if !existing.canonicalize()?.starts_with(root) {
        bail!("SEMANTIC_ARTIFACT_PATH_ESCAPE");
    }
    std::fs::create_dir_all(&destination)?;
    if !destination.canonicalize()?.starts_with(root) {
        bail!("SEMANTIC_ARTIFACT_PATH_ESCAPE");
    }
    Ok(())
}
