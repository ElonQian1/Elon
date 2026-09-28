use super::super::broker::{LiveUiBroker, LiveUiSession};
use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn validate(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
    source_root: &Path,
) -> Result<bool> {
    let install_id = broker
        .node_install_id()
        .context("Missing node install identity")?;
    let deployed = broker.debug_integration.status_for(
        source_root.to_string_lossy().as_ref(),
        &session.debug_project_id,
        &session.device_identity,
    )?;
    let known_package = deployed
        .as_ref()
        .filter(|status| {
            status.status == "DEPLOYED"
                && status.desired_generation > 0
                && status.installed_generation == Some(status.desired_generation)
                && status.device_identity == session.device_identity
                && status.project_id == session.debug_project_id
        })
        .map(|status| status.package_name.as_str());
    super::super::debug_package::verify_existing_debug_package(
        &session.package_name,
        install_id,
        &session.device_id,
        known_package,
    )
}
