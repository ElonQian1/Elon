    use super::{detect_manifest_project_identity, detect_shallow_manifest_project_identity};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-manifest-identity-{label}-{}-{}",
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
    fn detects_deno_project_identity() {
        let dir = temp_project("deno");
        std::fs::write(
            dir.join("deno.json"),
            r#"{"name":"edge-script-kit","description":"Deno 自动化项目"}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("fallback", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "edge-script-kit");
        assert_eq!(identity.description.as_deref(), Some("Deno 自动化项目"));
        assert_eq!(identity.source, "deno.json");
    }

    #[test]
    fn detects_tauri_product_name_from_root_config() {
        let dir = temp_project("tauri-root");
        std::fs::write(
            dir.join("tauri.conf.json"),
            r#"{"productName":"一龙桌面工作台","package":{"description":"本机开发入口"}}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("fallback", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "一龙桌面工作台");
        assert_eq!(identity.description.as_deref(), Some("本机开发入口"));
        assert_eq!(identity.source, "tauri.conf.json");
    }

    #[test]
    fn detects_tauri_product_name_from_src_tauri_config() {
        let dir = temp_project("tauri-nested");
        std::fs::create_dir_all(dir.join("src-tauri")).unwrap();
        std::fs::write(
            dir.join("src-tauri").join("tauri.conf.json"),
            r#"{"package":{"productName":"Desktop Agent"}}"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "Desktop Agent");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: Desktop Agent")
        );
    }

    #[test]
    fn detects_gradle_root_project_name() {
        let dir = temp_project("gradle");
        std::fs::write(
            dir.join("settings.gradle"),
            "pluginManagement {}\nrootProject.name = 'AndroidWorkbench'\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "AndroidWorkbench");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: AndroidWorkbench")
        );
        assert_eq!(identity.source, "settings.gradle");
    }

    #[test]
    fn detects_gradle_kts_root_project_name() {
        let dir = temp_project("gradle-kts");
        std::fs::write(
            dir.join("settings.gradle.kts"),
            "dependencyResolutionManagement {}\nrootProject.name = \"ComposeDesk\"\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "ComposeDesk");
        assert_eq!(identity.source, "settings.gradle.kts");
    }

    #[test]
    fn ignores_commented_gradle_root_project_name() {
        let dir = temp_project("gradle-commented");
        std::fs::write(
            dir.join("settings.gradle"),
            "// rootProject.name = 'IgnoredName'\nrootProject.name = 'RealName'\n",
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "RealName");
    }

    #[test]
    fn detects_dotnet_solution_name() {
        let dir = temp_project("dotnet-sln");
        std::fs::write(dir.join("OpsDesk.sln"), "").unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "OpsDesk");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: OpsDesk")
        );
        assert_eq!(identity.source, "OpsDesk.sln");
    }

    #[test]
    fn detects_dotnet_project_identity_when_no_solution_exists() {
        let dir = temp_project("dotnet-csproj");
        std::fs::write(
            dir.join("Worker.Host.csproj"),
            r#"<Project>
  <PropertyGroup>
    <AssemblyName>WorkerHost</AssemblyName>
    <Description>后台任务服务</Description>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let identity = detect_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "WorkerHost");
        assert_eq!(identity.description.as_deref(), Some("后台任务服务"));
        assert_eq!(identity.source, "Worker.Host.csproj");
    }

    #[test]
    fn detects_shallow_package_json_identity() {
        let dir = temp_project("shallow-node");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(
            dir.join("web").join("package.json"),
            r#"{"displayName":"PC 工作台","description":"Discord 风格本机开发入口"}"#,
        )
        .unwrap();

        let identity = detect_shallow_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "PC 工作台");
        assert_eq!(
            identity.description.as_deref(),
            Some("Discord 风格本机开发入口")
        );
        assert_eq!(identity.source, "web/package.json");
    }

    #[test]
    fn detects_packages_package_json_identity() {
        let dir = temp_project("packages-node");
        std::fs::create_dir_all(dir.join("packages").join("admin")).unwrap();
        std::fs::write(
            dir.join("packages").join("admin").join("package.json"),
            r#"{"displayName":"后台管理台","description":"本地 monorepo 子应用"}"#,
        )
        .unwrap();

        let identity = detect_shallow_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "后台管理台");
        assert_eq!(
            identity.description.as_deref(),
            Some("本地 monorepo 子应用")
        );
        assert_eq!(identity.source, "packages/admin/package.json");
    }

    #[test]
    fn detects_shallow_cargo_identity() {
        let dir = temp_project("shallow-rust");
        std::fs::create_dir_all(dir.join("server")).unwrap();
        std::fs::write(
            dir.join("server").join("Cargo.toml"),
            "[package]\nname = \"pc-node-agent\"\ndescription = 'Win 节点运行时'\n",
        )
        .unwrap();

        let identity = detect_shallow_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "pc-node-agent");
        assert_eq!(identity.description.as_deref(), Some("Win 节点运行时"));
        assert_eq!(identity.source, "server/Cargo.toml");
    }

    #[test]
    fn detects_shallow_go_identity() {
        let dir = temp_project("shallow-go");
        std::fs::create_dir_all(dir.join("api")).unwrap();
        std::fs::write(
            dir.join("api").join("go.mod"),
            "module github.com/example/local-node-api/v2\n\ngo 1.22\n",
        )
        .unwrap();

        let identity = detect_shallow_manifest_project_identity("folder-name", &dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "local-node-api");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: local-node-api")
        );
        assert_eq!(identity.source, "api/go.mod");
    }
