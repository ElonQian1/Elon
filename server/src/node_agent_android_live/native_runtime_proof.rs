use super::broker::{LiveUiBroker, LiveUiSession};
use anyhow::{Context, Result};
use serde_json::{json, Value};

#[path = "native_runtime_proof_validation.rs"]
mod validation;
pub(super) use validation::{required, validate};

pub(super) async fn read(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
    revision: &str,
) -> Result<Value> {
    let root = session
        .project_root
        .as_deref()
        .context("NATIVE_SOURCE_PROOF_MISSING: no project")?;
    let mut view = serde_json::to_value(session.view().await)?;
    view["debugProjectId"] = json!(session.debug_project_id);
    view["deviceIdentity"] = json!(session.device_identity);
    let integration = broker
        .debug_integration
        .status_for(root, &session.debug_project_id, &session.device_identity)?
        .context("NATIVE_SOURCE_PROOF_MISSING: no deployed integration")?;
    let integration = serde_json::to_value(integration)?;
    validate(&view, &integration, revision)?;
    Ok(json!({"runtime":view,"integration":integration}))
}
