use anyhow::{bail, Context, Result};
use chrono::Utc;

use super::broker::LiveUiSession;
use super::debug_integration::DebugIntegrationStatus;
use super::protocol::LiveSourceProofView;

pub(super) fn verified_proof(
    integration: &DebugIntegrationStatus,
    session: &LiveUiSession,
    generation_revision: String,
    origin_workspace_revision: String,
    runtime_build_id: Option<String>,
    source_parity_loss: f64,
) -> Result<LiveSourceProofView> {
    if integration.project_id != session.debug_project_id
        || integration.device_identity != session.device_identity
        || integration.package_name != session.package_name
    {
        bail!("FIT_SOURCE_PROOF_IDENTITY_DRIFT: 集成槽的项目、设备或包身份与 Runtime 会话不一致");
    }
    if integration.status != "DEPLOYED"
        || integration.installed_generation != Some(integration.desired_generation)
    {
        bail!(
            "FIT_SOURCE_PROOF_GENERATION_NOT_DEPLOYED: generation={} installed={:?} status={}",
            integration.desired_generation,
            integration.installed_generation,
            integration.status
        );
    }
    let integration_revision = integration
        .integration_revision
        .clone()
        .filter(|value| !value.trim().is_empty())
        .context("FIT_SOURCE_PROOF_INTEGRATION_REVISION_MISSING")?;
    let source_revision = integration
        .source_revision
        .clone()
        .filter(|value| !value.trim().is_empty())
        .context("FIT_SOURCE_PROOF_SOURCE_REVISION_MISSING")?;
    Ok(LiveSourceProofView {
        generation: integration.desired_generation,
        integration_revision,
        source_revision,
        generation_revision,
        origin_workspace_revision,
        runtime_build_id,
        source_parity_loss,
        verified_at: Utc::now().to_rfc3339(),
    })
}
