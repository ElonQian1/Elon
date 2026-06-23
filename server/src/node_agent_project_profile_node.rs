// server/src/node_agent_project_profile_node.rs

use serde_json::Value;
use std::path::Path;

use crate::node_agent_project_profile::ProjectProfile;

#[derive(Debug)]
pub(crate) struct NodeWorkspaceModule {
    pub(crate) module: String,
    pub(crate) manager: String,
    pub(crate) run_script: Option<String>,
    pub(crate) test_script: Option<String>,
    pub(crate) build_script: Option<String>,
}

pub(crate) fn detect_node_project_profile(path: &Path) -> Option<ProjectProfile> {
    let package_json = path.join("package.json");
    if !package_json.exists() {
        return None;
    }

    let mut profile = ProjectProfile {
        project_type: Some("Node.js".to_string()),
        package_manager: Some(node_package_manager(path)),
        detected_files: vec!["package.json".to_string()],
        ..ProjectProfile::default()
    };
    if path.join("bun.lockb").exists() {
        profile.detected_files.push("bun.lockb".to_string());
    } else if path.join("bun.lock").exists() {
        profile.detected_files.push("bun.lock".to_string());
    } else if path.join("pnpm-lock.yaml").exists() {
        profile.detected_files.push("pnpm-lock.yaml".to_string());
    } else if path.join("yarn.lock").exists() {
        profile.detected_files.push("yarn.lock".to_string());
    } else if path.join("package-lock.json").exists() {
        profile.detected_files.push("package-lock.json".to_string());
    }
    apply_package_json_scripts(&mut profile, &package_json);
    Some(profile)
}

pub(crate) fn detect_node_workspace_module(
    module: &str,
    module_path: &Path,
) -> Option<NodeWorkspaceModule> {
    let package_json = module_path.join("package.json");
    if !package_json.is_file() {
        return None;
    }

    let manager = node_package_manager(module_path);
    let scripts = read_package_json_scripts(&package_json);
    Some(NodeWorkspaceModule {
        module: module.to_string(),
        manager,
        run_script: scripts
            .as_ref()
            .and_then(|scripts| first_script_name(scripts, &["dev", "start"])),
        test_script: scripts
            .as_ref()
            .and_then(|scripts| first_script_name(scripts, &["test"])),
        build_script: scripts
            .as_ref()
            .and_then(|scripts| first_script_name(scripts, &["build"])),
    })
}

pub(crate) fn node_workspace_script_command(manager: &str, module: &str, script: &str) -> String {
    match manager {
        "pnpm" => format!("pnpm --dir {module} {script}"),
        "yarn" => format!("yarn --cwd {module} {script}"),
        "bun" => format!("bun --cwd {module} run {script}"),
        _ => format!("npm --prefix {module} run {script}"),
    }
}

fn node_package_manager(path: &Path) -> String {
    if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
        "bun".to_string()
    } else if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() {
        "yarn".to_string()
    } else {
        "npm".to_string()
    }
}

fn apply_package_json_scripts(profile: &mut ProjectProfile, package_json: &Path) {
    let Some(scripts) = read_package_json_scripts(package_json) else {
        return;
    };
    let manager = profile.package_manager.as_deref().unwrap_or("npm");
    profile.run_command = first_script_command(manager, &scripts, &["dev", "start"]);
    profile.test_command = first_script_command(manager, &scripts, &["test"]);
    profile.build_command = first_script_command(manager, &scripts, &["build"]);
}

fn read_package_json_scripts(package_json: &Path) -> Option<serde_json::Map<String, Value>> {
    std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.as_object().cloned())
}

fn first_script_command(
    manager: &str,
    scripts: &serde_json::Map<String, Value>,
    names: &[&str],
) -> Option<String> {
    first_script_name(scripts, names).map(|name| match manager {
        "yarn" => format!("yarn {name}"),
        "pnpm" => format!("pnpm {name}"),
        "bun" => format!("bun run {name}"),
        _ => format!("npm run {name}"),
    })
}

fn first_script_name(scripts: &serde_json::Map<String, Value>, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find(|name| scripts.get(**name).is_some())
        .map(|name| (*name).to_string())
}
