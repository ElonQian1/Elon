// server/src/node_agent_project_profile_node.rs

use serde_json::Value;
use std::path::Path;

use crate::node_agent_project_profile::ProjectProfile;

const RUN_SCRIPT_NAMES: &[&str] = &["dev", "start", "serve", "watch"];
const TEST_SCRIPT_NAMES: &[&str] = &["test", "check", "test:unit", "typecheck"];
const BUILD_SCRIPT_NAMES: &[&str] = &["build", "compile", "dist"];

#[derive(Debug)]
pub(crate) struct NodeWorkspaceModule {
    pub(crate) project_type: Option<String>,
    pub(crate) manager: String,
    pub(crate) run_command: Option<String>,
    pub(crate) test_command: Option<String>,
    pub(crate) build_command: Option<String>,
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
    let package = read_package_json_info(&package_json);
    let scripts = package
        .as_ref()
        .and_then(|package| package.scripts.as_ref());
    let mut run_command = scripts
        .and_then(|scripts| first_script_name(scripts, RUN_SCRIPT_NAMES))
        .map(|script| node_workspace_script_command(&manager, module, &script));
    let test_command = scripts
        .and_then(|scripts| first_script_name(scripts, TEST_SCRIPT_NAMES))
        .map(|script| node_workspace_script_command(&manager, module, &script));
    let mut build_command = scripts
        .and_then(|scripts| first_script_name(scripts, BUILD_SCRIPT_NAMES))
        .map(|script| node_workspace_script_command(&manager, module, &script));
    if let Some(package) = package.as_ref() {
        apply_workspace_framework_fallback(
            &manager,
            module,
            package,
            &mut run_command,
            &mut build_command,
        );
    }
    Some(NodeWorkspaceModule {
        project_type: package
            .as_ref()
            .and_then(detect_node_desktop_project_type)
            .map(ToOwned::to_owned),
        manager,
        run_command,
        test_command,
        build_command,
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
    let Some(package) = read_package_json_info(package_json) else {
        return;
    };
    if let Some(label) = detect_node_desktop_project_type(&package) {
        profile.project_type = Some(label.to_string());
    }
    let manager = profile.package_manager.as_deref().unwrap_or("npm");
    profile.run_command = package
        .scripts
        .as_ref()
        .and_then(|scripts| first_script_command(manager, scripts, RUN_SCRIPT_NAMES));
    profile.test_command = package
        .scripts
        .as_ref()
        .and_then(|scripts| first_script_command(manager, scripts, TEST_SCRIPT_NAMES));
    profile.build_command = package
        .scripts
        .as_ref()
        .and_then(|scripts| first_script_command(manager, scripts, BUILD_SCRIPT_NAMES));
    apply_framework_fallback(
        manager,
        &package,
        &mut profile.run_command,
        &mut profile.build_command,
    );
}

#[derive(Debug)]
struct PackageJsonInfo {
    scripts: Option<serde_json::Map<String, Value>>,
    dependencies: Vec<String>,
}

fn read_package_json_info(package_json: &Path) -> Option<PackageJsonInfo> {
    let value = std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())?;
    let scripts = value
        .get("scripts")
        .and_then(|scripts| scripts.as_object().cloned());
    let mut dependencies = Vec::new();
    for key in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        if let Some(object) = value.get(key).and_then(Value::as_object) {
            for dependency in object.keys() {
                if !dependencies.iter().any(|existing| existing == dependency) {
                    dependencies.push(dependency.to_string());
                }
            }
        }
    }
    Some(PackageJsonInfo {
        scripts,
        dependencies,
    })
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

fn apply_framework_fallback(
    manager: &str,
    package: &PackageJsonInfo,
    run_command: &mut Option<String>,
    build_command: &mut Option<String>,
) {
    if run_command.is_some() && build_command.is_some() {
        return;
    }
    if let Some(framework) = detect_framework(package) {
        if run_command.is_none() {
            *run_command = Some(package_exec_command(
                manager,
                framework.binary,
                framework.dev_args,
            ));
        }
        if build_command.is_none() {
            *build_command = Some(package_exec_command(
                manager,
                framework.binary,
                framework.build_args,
            ));
        }
    }
}

fn apply_workspace_framework_fallback(
    manager: &str,
    module: &str,
    package: &PackageJsonInfo,
    run_command: &mut Option<String>,
    build_command: &mut Option<String>,
) {
    if run_command.is_some() && build_command.is_some() {
        return;
    }
    if let Some(framework) = detect_framework(package) {
        if run_command.is_none() {
            *run_command = Some(workspace_package_exec_command(
                manager,
                module,
                framework.binary,
                framework.dev_args,
            ));
        }
        if build_command.is_none() {
            *build_command = Some(workspace_package_exec_command(
                manager,
                module,
                framework.binary,
                framework.build_args,
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NodeFramework {
    binary: &'static str,
    dev_args: &'static [&'static str],
    build_args: &'static [&'static str],
}

fn detect_framework(package: &PackageJsonInfo) -> Option<NodeFramework> {
    if has_dependency(package, "@tauri-apps/cli") || has_dependency(package, "@tauri-apps/api") {
        return Some(NodeFramework {
            binary: "tauri",
            dev_args: &["dev"],
            build_args: &["build"],
        });
    }
    if has_dependency(package, "vite") {
        return Some(NodeFramework {
            binary: "vite",
            dev_args: &["--host", "127.0.0.1"],
            build_args: &["build"],
        });
    }
    if has_dependency(package, "next") {
        return Some(NodeFramework {
            binary: "next",
            dev_args: &["dev"],
            build_args: &["build"],
        });
    }
    if has_dependency(package, "astro") {
        return Some(NodeFramework {
            binary: "astro",
            dev_args: &["dev", "--host", "127.0.0.1"],
            build_args: &["build"],
        });
    }
    None
}

fn detect_node_desktop_project_type(package: &PackageJsonInfo) -> Option<&'static str> {
    if has_dependency(package, "@tauri-apps/cli") || has_dependency(package, "@tauri-apps/api") {
        return Some("Tauri 桌面应用");
    }
    if has_dependency(package, "electron") {
        return Some("Electron 桌面应用");
    }
    None
}

fn has_dependency(package: &PackageJsonInfo, name: &str) -> bool {
    package
        .dependencies
        .iter()
        .any(|dependency| dependency == name)
}

fn package_exec_command(manager: &str, binary: &str, args: &[&str]) -> String {
    let arg_text = args.join(" ");
    match manager {
        "pnpm" if arg_text.is_empty() => format!("pnpm exec {binary}"),
        "pnpm" => format!("pnpm exec {binary} {arg_text}"),
        "yarn" if arg_text.is_empty() => format!("yarn {binary}"),
        "yarn" => format!("yarn {binary} {arg_text}"),
        "bun" if arg_text.is_empty() => format!("bunx {binary}"),
        "bun" => format!("bunx {binary} {arg_text}"),
        _ if arg_text.is_empty() => format!("npm exec {binary}"),
        _ => format!("npm exec {binary} -- {arg_text}"),
    }
}

fn workspace_package_exec_command(
    manager: &str,
    module: &str,
    binary: &str,
    args: &[&str],
) -> String {
    let arg_text = args.join(" ");
    match manager {
        "pnpm" if arg_text.is_empty() => format!("pnpm --dir {module} exec {binary}"),
        "pnpm" => format!("pnpm --dir {module} exec {binary} {arg_text}"),
        "yarn" if arg_text.is_empty() => format!("yarn --cwd {module} {binary}"),
        "yarn" => format!("yarn --cwd {module} {binary} {arg_text}"),
        "bun" if arg_text.is_empty() => format!("bun --cwd {module} x {binary}"),
        "bun" => format!("bun --cwd {module} x {binary} {arg_text}"),
        _ if arg_text.is_empty() => format!("npm --prefix {module} exec {binary}"),
        _ => format!("npm --prefix {module} exec {binary} -- {arg_text}"),
    }
}
