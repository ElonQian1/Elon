// server/src/node_agent_project_profile.rs

use std::path::Path;

use crate::node_agent_project_profile_node::{
    detect_node_project_profile, detect_node_workspace_module, NodeWorkspaceModule,
};
use crate::node_agent_project_profile_python::{
    detect_python_project_profile, python_workspace_commands, PythonWorkspaceModule,
};
use crate::node_agent_workspace_modules::workspace_module_candidates;

#[derive(Debug, Default)]
pub(crate) struct ProjectProfile {
    pub(crate) project_type: Option<String>,
    pub(crate) package_manager: Option<String>,
    pub(crate) run_command: Option<String>,
    pub(crate) test_command: Option<String>,
    pub(crate) build_command: Option<String>,
    pub(crate) detected_files: Vec<String>,
}

pub(crate) fn detect_project_profile(path: &Path) -> ProjectProfile {
    let mut profile = ProjectProfile::default();
    let cargo = path.join("Cargo.toml");
    let gradle = path.join("build.gradle");
    let gradle_kts = path.join("build.gradle.kts");
    let go_mod = path.join("go.mod");

    if cargo.exists() {
        profile.project_type = Some("Rust".to_string());
        profile.package_manager = Some("Cargo".to_string());
        profile.run_command = Some("cargo run".to_string());
        profile.test_command = Some("cargo test".to_string());
        profile.build_command = Some("cargo build".to_string());
        profile.detected_files.push("Cargo.toml".to_string());
        return profile;
    }

    if let Some(node) = detect_node_project_profile(path) {
        return node;
    }

    if gradle.exists() || gradle_kts.exists() {
        profile.project_type = Some("Gradle".to_string());
        profile.package_manager = Some("Gradle".to_string());
        profile.build_command = Some(gradle_command(path, "build"));
        profile.test_command = Some(gradle_command(path, "test"));
        profile.detected_files.push(
            if gradle.exists() {
                "build.gradle"
            } else {
                "build.gradle.kts"
            }
            .to_string(),
        );
        return profile;
    }

    if go_mod.exists() && path.join("wails.json").exists() {
        return wails_project_profile(path);
    }

    if go_mod.exists() {
        return go_project_profile(path);
    }

    if let Some(workspace) = shallow_workspace_project_profile(path) {
        return workspace;
    }

    if let Some(dotnet) = dotnet_project_profile(path) {
        return dotnet;
    }

    if let Some(python) = detect_python_project_profile(path) {
        return python;
    }

    profile
}

fn shallow_workspace_project_profile(path: &Path) -> Option<ProjectProfile> {
    const MAX_DETECTED_FILES: usize = 8;
    let mut detected_files = Vec::new();
    let mut project_types = Vec::new();
    let mut package_managers = Vec::new();
    let mut cargo_manifest = None;
    let mut node_module = None;
    let mut gradle_dir = None;
    let mut go_module = None;
    let mut python_module = None;

    for candidate in workspace_module_candidates(path) {
        let module = candidate.module.as_str();
        let module_path = candidate.path;

        let cargo = module_path.join("Cargo.toml");
        if cargo.is_file() {
            push_unique(&mut detected_files, format!("{module}/Cargo.toml"));
            push_unique(&mut project_types, "Rust".to_string());
            push_unique(&mut package_managers, "Cargo".to_string());
            if cargo_manifest.is_none() {
                cargo_manifest = Some(format!("{module}/Cargo.toml"));
            }
        }

        if let Some(detected_node) = detect_node_workspace_module(module, &module_path) {
            push_unique(&mut detected_files, format!("{module}/package.json"));
            push_unique(
                &mut project_types,
                detected_node
                    .project_type
                    .clone()
                    .unwrap_or_else(|| "Node.js".to_string()),
            );
            push_unique(&mut package_managers, detected_node.manager.clone());
            if node_module.is_none() {
                node_module = Some(detected_node);
            }
        }

        let gradle = module_path.join("build.gradle");
        let gradle_kts = module_path.join("build.gradle.kts");
        if gradle.is_file() || gradle_kts.is_file() {
            push_unique(
                &mut detected_files,
                if gradle.is_file() {
                    format!("{module}/build.gradle")
                } else {
                    format!("{module}/build.gradle.kts")
                },
            );
            push_unique(&mut project_types, "Gradle".to_string());
            push_unique(&mut package_managers, "Gradle".to_string());
            if gradle_dir.is_none() {
                gradle_dir = Some(module.to_string());
            }
        }

        let go_mod = module_path.join("go.mod");
        if go_mod.is_file() {
            push_unique(&mut detected_files, format!("{module}/go.mod"));
            if module_path.join("wails.json").is_file() {
                push_unique(&mut detected_files, format!("{module}/wails.json"));
                push_unique(&mut project_types, "Wails 桌面应用".to_string());
                push_unique(&mut package_managers, "wails".to_string());
            } else {
                push_unique(&mut project_types, "Go".to_string());
                push_unique(&mut package_managers, "go".to_string());
                if go_module.is_none() {
                    go_module = Some(module.to_string());
                }
            }
        }

        if let Some(python) = detect_python_project_profile(&module_path) {
            for file in python.detected_files {
                push_unique(&mut detected_files, format!("{module}/{file}"));
            }
            push_unique(&mut project_types, "Python".to_string());
            let manager = python.package_manager.unwrap_or_else(|| "pip".to_string());
            push_unique(&mut package_managers, manager.clone());
            if python_module.is_none() {
                python_module = Some(PythonWorkspaceModule {
                    module: module.to_string(),
                    manager,
                    has_manage_py: module_path.join("manage.py").is_file(),
                    has_pyproject: module_path.join("pyproject.toml").is_file(),
                });
            }
        }

        if detected_files.len() >= MAX_DETECTED_FILES {
            break;
        }
    }

    if detected_files.is_empty() {
        return None;
    }

    detected_files.truncate(MAX_DETECTED_FILES);
    let project_type = if project_types.len() == 1 {
        project_types.pop()
    } else {
        Some(format!("多模块项目（{}）", project_types.join(" + ")))
    };
    let package_manager = if package_managers.len() == 1 {
        package_managers.pop()
    } else {
        Some(package_managers.join(" / "))
    };

    let (run_command, test_command, build_command) = workspace_commands(
        cargo_manifest.as_deref(),
        node_module.as_ref(),
        go_module.as_deref(),
        python_module.as_ref(),
        gradle_dir.as_deref(),
    );

    Some(ProjectProfile {
        project_type,
        package_manager,
        run_command,
        test_command,
        build_command,
        detected_files,
    })
}

fn workspace_commands(
    cargo_manifest: Option<&str>,
    node_module: Option<&NodeWorkspaceModule>,
    go_module: Option<&str>,
    python_module: Option<&PythonWorkspaceModule>,
    gradle_module: Option<&str>,
) -> (Option<String>, Option<String>, Option<String>) {
    if let Some(manifest) = cargo_manifest {
        return (
            Some(format!("cargo run --manifest-path {manifest}")),
            Some(format!("cargo test --manifest-path {manifest}")),
            Some(format!("cargo build --manifest-path {manifest}")),
        );
    }
    if let Some(module) = node_module {
        return (
            module.run_command.clone(),
            module.test_command.clone(),
            module.build_command.clone(),
        );
    }
    if let Some(module) = go_module {
        return (
            Some(format!("go -C {module} run .")),
            Some(format!("go -C {module} test ./...")),
            Some(format!("go -C {module} build ./...")),
        );
    }
    if let Some(module) = python_module {
        return python_workspace_commands(module);
    }
    if let Some(module) = gradle_module {
        return (
            None,
            Some(format!("gradle -p {module} test")),
            Some(format!("gradle -p {module} build")),
        );
    }
    (None, None, None)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn gradle_command(path: &Path, task: &str) -> String {
    if cfg!(windows) && path.join("gradlew.bat").exists() {
        format!("gradlew.bat {task}")
    } else if path.join("gradlew").exists() {
        format!("./gradlew {task}")
    } else {
        format!("gradle {task}")
    }
}

fn go_project_profile(path: &Path) -> ProjectProfile {
    let mut detected_files = vec!["go.mod".to_string()];
    if path.join("go.sum").exists() {
        detected_files.push("go.sum".to_string());
    }
    ProjectProfile {
        project_type: Some("Go".to_string()),
        package_manager: Some("go".to_string()),
        run_command: Some("go run .".to_string()),
        test_command: Some("go test ./...".to_string()),
        build_command: Some("go build ./...".to_string()),
        detected_files,
    }
}

fn wails_project_profile(path: &Path) -> ProjectProfile {
    let mut detected_files = vec!["wails.json".to_string(), "go.mod".to_string()];
    if path.join("go.sum").exists() {
        detected_files.push("go.sum".to_string());
    }
    ProjectProfile {
        project_type: Some("Wails 桌面应用".to_string()),
        package_manager: Some("wails".to_string()),
        run_command: Some("wails dev".to_string()),
        test_command: Some("go test ./...".to_string()),
        build_command: Some("wails build".to_string()),
        detected_files,
    }
}

fn dotnet_project_profile(path: &Path) -> Option<ProjectProfile> {
    let mut detected_files = directory_files_with_extensions(path, &["sln", "csproj"]);
    if detected_files.is_empty() {
        return None;
    }
    detected_files.sort();
    let has_project = detected_files.iter().any(|file| file.ends_with(".csproj"));
    Some(ProjectProfile {
        project_type: Some(".NET".to_string()),
        package_manager: Some("dotnet".to_string()),
        run_command: has_project.then(|| "dotnet run".to_string()),
        test_command: Some("dotnet test".to_string()),
        build_command: Some("dotnet build".to_string()),
        detected_files: detected_files.into_iter().take(8).collect(),
    })
}

fn directory_files_with_extensions(path: &Path, extensions: &[&str]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let extension = Path::new(&file_name)
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)?;
            extensions
                .iter()
                .any(|candidate| extension == *candidate)
                .then_some(file_name)
        })
        .collect()
}


#[cfg(test)]
#[path = "node_agent_project_profile_tests.rs"]
mod tests;
