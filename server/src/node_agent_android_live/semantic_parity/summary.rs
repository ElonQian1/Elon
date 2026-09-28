use super::{contract, evidence};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

static WRITER: OnceLock<Mutex<()>> = OnceLock::new();
pub(super) fn begin(
    root: &Path,
    task: &str,
    revision: &str,
    contract_path: &str,
    hash: &str,
) -> Result<String> {
    let _guard = WRITER
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("SEMANTIC_WRITER_LOCK"))?;
    let path = root.join("cross-platform-verification.json");
    if path.exists() {
        let value = evidence::read_json(root, "cross-platform-verification.json")?;
        if value["schemaVersion"] == 3
            && value["verificationMode"] == "SEMANTIC_PARITY"
            && value["taskId"] == task
            && value["sourceRevision"] == revision
            && value["contractSha256"] == hash
            && value["contractPath"] == contract_path
        {
            if let Some(id) = value["runId"].as_str().filter(|id| contract::safe_id(id)) {
                return Ok(id.into());
            }
        }
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    let value = json!({"schemaVersion":3,"verificationMode":"SEMANTIC_PARITY","taskId":task,"sourceRevision":revision,
        "contractPath":contract_path,"contractSha256":hash,"runId":id,"stateReceipts":{},"scope":"OBSERVED_CAPABILITY_STATE"});
    crate::node_agent_atomic_file::write(&path, &serde_json::to_vec_pretty(&value)?)?;
    Ok(id)
}
pub(super) fn record(root: &Path, run_id: &str, state: &str, receipt: &str) -> Result<Value> {
    let _guard = WRITER
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("SEMANTIC_WRITER_LOCK"))?;
    let mut summary = evidence::read_json(root, "cross-platform-verification.json")?;
    if summary["runId"] != run_id || !summary["stateReceipts"].is_object() {
        bail!("SEMANTIC_RUN_SUPERSEDED");
    }
    summary["stateReceipts"][state] = json!(receipt);
    crate::node_agent_atomic_file::write(
        &root.join("cross-platform-verification.json"),
        &serde_json::to_vec_pretty(&summary)?,
    )?;
    Ok(summary)
}
