// server/src/node_agent_project_manifest_identity.rs

use serde_json::Value;
use std::path::Path;

use crate::node_agent_workspace_modules::workspace_module_candidates;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ManifestProjectIdentity {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) source: String,
}

pub(crate) fn detect_manifest_project_identity(
    fallback_name: &str,
    project_root: &Path,
) -> Option<ManifestProjectIdentity> {
    identity_from_json_manifest(
        fallback_name,
        &project_root.join("deno.json"),
        &["name"],
        &["description"],
        "deno.json",
    )
    .or_else(|| identity_from_tauri_config(fallback_name, &project_root.join("tauri.conf.json")))
    .or_else(|| {
        identity_from_tauri_config(
            fallback_name,
            &project_root.join("src-tauri/tauri.conf.json"),
        )
    })
    .or_else(|| {
        identity_from_gradle_settings(
            fallback_name,
            &project_root.join("settings.gradle.kts"),
            "settings.gradle.kts",
        )
    })
    .or_else(|| {
        identity_from_gradle_settings(
            fallback_name,
            &project_root.join("settings.gradle"),
            "settings.gradle",
        )
    })
    .or_else(|| identity_from_dotnet_solution_or_project(fallback_name, project_root))
}

pub(crate) fn detect_shallow_manifest_project_identity(
    fallback_name: &str,
    project_root: &Path,
) -> Option<ManifestProjectIdentity> {
    for candidate in workspace_module_candidates(project_root) {
        let module = candidate.module;
        let module_root = candidate.path;
        if let Some(identity) = detect_module_manifest_project_identity(fallback_name, &module_root)
        {
            return Some(identity.with_source_prefix(&module));
        }
    }
    None
}


#[path = "node_agent_project_manifest_identity_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
