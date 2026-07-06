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
    fn detects_packages_node_workspace_from_repo_root() {
        let dir = temp_project("workspace-packages-node");
        let package = dir.join("packages").join("web");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("package.json"),
            r#"{"scripts":{"dev":"vite","test":"vitest","build":"vite build"}}"#,
        )
        .unwrap();
        std::fs::write(package.join("pnpm-lock.yaml"), "").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(
            profile.run_command.as_deref(),
            Some("pnpm --dir packages/web dev")
        );
        assert_eq!(
            profile.test_command.as_deref(),
            Some("pnpm --dir packages/web test")
        );
        assert!(profile
            .detected_files
            .contains(&"packages/web/package.json".to_string()));
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
