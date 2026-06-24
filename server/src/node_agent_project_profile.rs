// server/src/node_agent_project_profile.rs

use std::path::Path;

use crate::node_agent_project_profile_node::{
    detect_node_project_profile, detect_node_workspace_module, NodeWorkspaceModule,
};
use crate::node_agent_project_profile_python::{
    detect_python_project_profile, python_workspace_commands, PythonWorkspaceModule,
};

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

    for module in [
        "server", "backend", "api", "app", "cmd", "web", "frontend", "client", "android",
    ] {
        let module_path = path.join(module);
        if !module_path.is_dir() {
            continue;
        }

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
mod tests {
    use super::detect_project_profile;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-profile-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_rust_project_commands() {
        let dir = temp_project("rust");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Rust"));
        assert_eq!(profile.package_manager.as_deref(), Some("Cargo"));
        assert_eq!(profile.test_command.as_deref(), Some("cargo test"));
        assert!(profile.detected_files.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn detects_node_package_manager_and_scripts() {
        let dir = temp_project("node");
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"dev":"vite","test":"vitest","build":"vite build"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(profile.run_command.as_deref(), Some("pnpm dev"));
        assert_eq!(profile.test_command.as_deref(), Some("pnpm test"));
        assert_eq!(profile.build_command.as_deref(), Some("pnpm build"));
    }

    #[test]
    fn detects_bun_package_manager_and_scripts() {
        let dir = temp_project("node-bun");
        std::fs::write(dir.join("bun.lockb"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"dev":"vite --host","test":"bun test","build":"vite build"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("bun"));
        assert_eq!(profile.run_command.as_deref(), Some("bun run dev"));
        assert_eq!(profile.test_command.as_deref(), Some("bun run test"));
        assert_eq!(profile.build_command.as_deref(), Some("bun run build"));
        assert!(profile.detected_files.contains(&"bun.lockb".to_string()));
    }

    #[test]
    fn detects_node_alternate_script_names() {
        let dir = temp_project("node-alternate-scripts");
        std::fs::write(dir.join("yarn.lock"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"serve":"vite","check":"tsc --noEmit","compile":"vite build"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("yarn"));
        assert_eq!(profile.run_command.as_deref(), Some("yarn serve"));
        assert_eq!(profile.test_command.as_deref(), Some("yarn check"));
        assert_eq!(profile.build_command.as_deref(), Some("yarn compile"));
    }

    #[test]
    fn detects_node_vite_fallback_without_scripts() {
        let dir = temp_project("node-vite-fallback");
        std::fs::write(
            dir.join("package.json"),
            r#"{"devDependencies":{"vite":"^5.0.0"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(
            profile.run_command.as_deref(),
            Some("npm exec vite -- --host 127.0.0.1")
        );
        assert!(profile.test_command.is_none());
        assert_eq!(
            profile.build_command.as_deref(),
            Some("npm exec vite -- build")
        );
    }

    #[test]
    fn detects_tauri_desktop_project_without_scripts() {
        let dir = temp_project("node-tauri-fallback");
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"devDependencies":{"@tauri-apps/cli":"^2.0.0","vite":"^5.0.0"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Tauri 桌面应用"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(profile.run_command.as_deref(), Some("pnpm exec tauri dev"));
        assert_eq!(
            profile.build_command.as_deref(),
            Some("pnpm exec tauri build")
        );
    }

    #[test]
    fn detects_gradle_projects() {
        let dir = temp_project("gradle");
        std::fs::write(dir.join("build.gradle.kts"), "plugins {}\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Gradle"));
        assert!(profile
            .build_command
            .as_deref()
            .unwrap_or_default()
            .contains("build"));
    }

    #[test]
    fn detects_go_project_commands() {
        let dir = temp_project("go");
        std::fs::write(dir.join("go.mod"), "module github.com/example/pc-agent\n").unwrap();
        std::fs::write(dir.join("go.sum"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Go"));
        assert_eq!(profile.package_manager.as_deref(), Some("go"));
        assert_eq!(profile.run_command.as_deref(), Some("go run ."));
        assert_eq!(profile.test_command.as_deref(), Some("go test ./..."));
        assert_eq!(profile.build_command.as_deref(), Some("go build ./..."));
        assert!(profile.detected_files.contains(&"go.mod".to_string()));
        assert!(profile.detected_files.contains(&"go.sum".to_string()));
    }

    #[test]
    fn detects_wails_desktop_project_commands() {
        let dir = temp_project("wails");
        std::fs::write(dir.join("go.mod"), "module github.com/example/desktop\n").unwrap();
        std::fs::write(dir.join("wails.json"), r#"{"name":"Desktop"}"#).unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Wails 桌面应用"));
        assert_eq!(profile.package_manager.as_deref(), Some("wails"));
        assert_eq!(profile.run_command.as_deref(), Some("wails dev"));
        assert_eq!(profile.test_command.as_deref(), Some("go test ./..."));
        assert_eq!(profile.build_command.as_deref(), Some("wails build"));
        assert!(profile.detected_files.contains(&"wails.json".to_string()));
        assert!(profile.detected_files.contains(&"go.mod".to_string()));
    }

    #[test]
    fn detects_dotnet_solution_and_project_commands() {
        let dir = temp_project("dotnet");
        std::fs::write(dir.join("Demo.sln"), "").unwrap();
        std::fs::write(dir.join("Demo.Web.csproj"), "<Project />\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some(".NET"));
        assert_eq!(profile.package_manager.as_deref(), Some("dotnet"));
        assert_eq!(profile.run_command.as_deref(), Some("dotnet run"));
        assert_eq!(profile.test_command.as_deref(), Some("dotnet test"));
        assert_eq!(profile.build_command.as_deref(), Some("dotnet build"));
        assert!(profile.detected_files.contains(&"Demo.sln".to_string()));
        assert!(profile
            .detected_files
            .contains(&"Demo.Web.csproj".to_string()));
    }

    #[test]
    fn detects_dotnet_solution_without_run_command() {
        let dir = temp_project("dotnet-solution-only");
        std::fs::write(dir.join("Demo.sln"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some(".NET"));
        assert_eq!(profile.package_manager.as_deref(), Some("dotnet"));
        assert!(profile.run_command.is_none());
        assert_eq!(profile.test_command.as_deref(), Some("dotnet test"));
        assert_eq!(profile.build_command.as_deref(), Some("dotnet build"));
    }

    #[test]
    fn detects_shallow_rust_android_workspace_from_repo_root() {
        let dir = temp_project("workspace-rust-android");
        std::fs::create_dir_all(dir.join("server")).unwrap();
        std::fs::create_dir_all(dir.join("android")).unwrap();
        std::fs::write(
            dir.join("server").join("Cargo.toml"),
            "[package]\nname='demo'\n",
        )
        .unwrap();
        std::fs::write(dir.join("android").join("build.gradle.kts"), "plugins {}\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            profile.project_type.as_deref(),
            Some("多模块项目（Rust + Gradle）")
        );
        assert_eq!(profile.package_manager.as_deref(), Some("Cargo / Gradle"));
        assert_eq!(
            profile.test_command.as_deref(),
            Some("cargo test --manifest-path server/Cargo.toml")
        );
        assert_eq!(
            profile.build_command.as_deref(),
            Some("cargo build --manifest-path server/Cargo.toml")
        );
        assert!(profile
            .detected_files
            .contains(&"server/Cargo.toml".to_string()));
        assert!(profile
            .detected_files
            .contains(&"android/build.gradle.kts".to_string()));
    }

    #[test]
    fn detects_shallow_node_workspace_from_repo_root() {
        let dir = temp_project("workspace-node");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(
            dir.join("web").join("package.json"),
            r#"{"scripts":{"dev":"vite","test":"vitest","build":"vite build"}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("web").join("pnpm-lock.yaml"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(profile.run_command.as_deref(), Some("pnpm --dir web dev"));
        assert_eq!(
            profile.build_command.as_deref(),
            Some("pnpm --dir web build")
        );
        assert!(profile
            .detected_files
            .contains(&"web/package.json".to_string()));
    }

    #[test]
    fn detects_shallow_go_workspace_from_repo_root() {
        let dir = temp_project("workspace-go");
        std::fs::create_dir_all(dir.join("server")).unwrap();
        std::fs::write(
            dir.join("server").join("go.mod"),
            "module github.com/example/server\n",
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Go"));
        assert_eq!(profile.package_manager.as_deref(), Some("go"));
        assert_eq!(profile.run_command.as_deref(), Some("go -C server run ."));
        assert_eq!(
            profile.test_command.as_deref(),
            Some("go -C server test ./...")
        );
        assert_eq!(
            profile.build_command.as_deref(),
            Some("go -C server build ./...")
        );
        assert!(profile
            .detected_files
            .contains(&"server/go.mod".to_string()));
    }

    #[test]
    fn detects_shallow_wails_workspace_without_guessing_subdir_commands() {
        let dir = temp_project("workspace-wails");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::write(
            dir.join("app").join("go.mod"),
            "module github.com/example/desktop\n",
        )
        .unwrap();
        std::fs::write(dir.join("app").join("wails.json"), r#"{"name":"Desktop"}"#).unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Wails 桌面应用"));
        assert_eq!(profile.package_manager.as_deref(), Some("wails"));
        assert!(profile.run_command.is_none());
        assert!(profile.build_command.is_none());
        assert!(profile
            .detected_files
            .contains(&"app/wails.json".to_string()));
    }

    #[test]
    fn detects_shallow_python_workspace_from_repo_root() {
        let dir = temp_project("workspace-python");
        std::fs::create_dir_all(dir.join("backend")).unwrap();
        std::fs::write(
            dir.join("backend").join("pyproject.toml"),
            "[project]\nname='demo-api'\n",
        )
        .unwrap();
        std::fs::write(dir.join("backend").join("uv.lock"), "").unwrap();
        std::fs::write(dir.join("backend").join("manage.py"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Python"));
        assert_eq!(profile.package_manager.as_deref(), Some("uv"));
        assert_eq!(
            profile.run_command.as_deref(),
            Some("uv run --project backend python backend/manage.py runserver")
        );
        assert_eq!(
            profile.test_command.as_deref(),
            Some("uv run --project backend pytest")
        );
        assert_eq!(profile.build_command.as_deref(), Some("uv build backend"));
        assert!(profile
            .detected_files
            .contains(&"backend/pyproject.toml".to_string()));
        assert!(profile
            .detected_files
            .contains(&"backend/uv.lock".to_string()));
    }

    #[test]
    fn detects_shallow_python_requirements_workspace_from_repo_root() {
        let dir = temp_project("workspace-python-requirements");
        std::fs::create_dir_all(dir.join("api")).unwrap();
        std::fs::write(dir.join("api").join("requirements.txt"), "pytest\n").unwrap();
        std::fs::write(dir.join("api").join("pytest.ini"), "[pytest]\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Python"));
        assert_eq!(profile.package_manager.as_deref(), Some("pip"));
        assert!(profile.run_command.is_none());
        assert_eq!(
            profile.test_command.as_deref(),
            Some("python -m pytest api")
        );
        assert!(profile.build_command.is_none());
        assert!(profile
            .detected_files
            .contains(&"api/requirements.txt".to_string()));
    }

    #[test]
    fn shallow_node_workspace_keeps_missing_scripts_empty() {
        let dir = temp_project("workspace-node-no-scripts");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(dir.join("web").join("package.json"), r#"{"scripts":{}}"#).unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert!(profile.run_command.is_none());
        assert!(profile.test_command.is_none());
        assert!(profile.build_command.is_none());
    }

    #[test]
    fn detects_shallow_node_vite_workspace_fallback_from_repo_root() {
        let dir = temp_project("workspace-node-vite-fallback");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(
            dir.join("web").join("package.json"),
            r#"{"devDependencies":{"vite":"^5.0.0"}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("web").join("pnpm-lock.yaml"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(
            profile.run_command.as_deref(),
            Some("pnpm --dir web exec vite --host 127.0.0.1")
        );
        assert!(profile.test_command.is_none());
        assert_eq!(
            profile.build_command.as_deref(),
            Some("pnpm --dir web exec vite build")
        );
    }

    #[test]
    fn detects_shallow_tauri_workspace_from_repo_root() {
        let dir = temp_project("workspace-tauri");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::write(dir.join("app").join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            dir.join("app").join("package.json"),
            r#"{"devDependencies":{"@tauri-apps/cli":"^2.0.0","vite":"^5.0.0"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Tauri 桌面应用"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(
            profile.run_command.as_deref(),
            Some("pnpm --dir app exec tauri dev")
        );
        assert_eq!(
            profile.build_command.as_deref(),
            Some("pnpm --dir app exec tauri build")
        );
    }
}
