use anyhow::{bail, Result};
use serde_json::Value;

pub(crate) fn required(
    effective: &[Value],
    android_project: bool,
    native_session: bool,
    launcher_only: bool,
) -> bool {
    !launcher_only
        && (android_project
            || native_session
            || effective.iter().any(|v| v == "REAL_ANDROID_RENDERER"))
        && effective
            .iter()
            .any(|v| v == "PATCH_FREE_BUILD_VERIFY" || v == "REAL_ANDROID_RENDERER")
}

pub(crate) fn validate(view: &Value, integration: &Value, revision: &str) -> Result<()> {
    let proof = &view["sourceProof"];
    let generation = proof["generation"].as_u64().filter(|v| *v > 0);
    let build = view["runtimeBuildId"].as_str().filter(|v| !v.is_empty());
    if !revision
        .strip_prefix("workspace-sha256:")
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        || view["connected"] != true
        || view["historyCount"] != 0
        || view["redoCount"] != 0
        || view["nodeCount"].as_u64().unwrap_or(0) == 0
        || view["deviceId"] == "ui-design-bootstrap"
        || view["packageName"] == "ui.design.bootstrap"
        || build.is_none()
        || proof["runtimeBuildId"].as_str() != build
        || proof["originWorkspaceRevision"].as_str() != Some(revision)
        || !proof["sourceParityLoss"]
            .as_f64()
            .is_some_and(|v| v.is_finite() && (0.0..=0.035).contains(&v))
        || generation.is_none()
        || integration["status"] != "DEPLOYED"
        || integration["desiredGeneration"].as_u64() != generation
        || integration["installedGeneration"].as_u64() != generation
        || integration["projectId"] != view["debugProjectId"]
        || integration["deviceIdentity"] != view["deviceIdentity"]
        || integration["packageName"] != view["packageName"]
        || integration["integrationRevision"] != proof["integrationRevision"]
        || integration["sourceRevision"] != proof["sourceRevision"]
        || !proof["generationRevision"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    {
        bail!("NATIVE_SOURCE_PROOF_MISSING_OR_STALE: connected current-source/current-build/deployed-generation patch-free proof required");
    }
    for key in [
        "projectId",
        "deviceIdentity",
        "packageName",
        "integrationRevision",
        "sourceRevision",
    ] {
        if !integration[key].as_str().is_some_and(|v| !v.is_empty()) {
            bail!("NATIVE_SOURCE_PROOF_MISSING_OR_STALE: missing integration identity");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_android_workflow_requires_source_even_without_clean_target() {
        let baseline = vec![Value::String("PATCH_FREE_BUILD_VERIFY".into())];
        assert!(required(&baseline, true, false, false));
        assert!(required(&baseline, false, true, false));
        assert!(!required(&baseline, false, false, false));
        assert!(!required(&baseline, true, true, true));
    }
}
