use super::{detect_node_project_profile, detect_node_workspace_module};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_project(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "elon-node-project-profile-{label}-{}-{}",
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
fn detects_electron_desktop_commands_without_scripts() {
    let dir = temp_project("electron-root");
    std::fs::write(
        dir.join("package.json"),
        r#"{"devDependencies":{"electron":"^31.0.0","electron-builder":"^24.0.0"}}"#,
    )
    .unwrap();

    let profile = detect_node_project_profile(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(profile.project_type.as_deref(), Some("Electron 桌面应用"));
    assert_eq!(
        profile.run_command.as_deref(),
        Some("npm exec electron -- .")
    );
    assert_eq!(
        profile.build_command.as_deref(),
        Some("npm exec electron-builder")
    );
}

#[test]
fn detects_workspace_electron_commands_without_scripts() {
    let dir = temp_project("electron-workspace");
    let app = dir.join("app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("pnpm-lock.yaml"), "").unwrap();
    std::fs::write(
        app.join("package.json"),
        r#"{"devDependencies":{"electron":"^31.0.0","@electron-forge/cli":"^7.0.0"}}"#,
    )
    .unwrap();

    let module = detect_node_workspace_module("app", &app).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(module.project_type.as_deref(), Some("Electron 桌面应用"));
    assert_eq!(
        module.run_command.as_deref(),
        Some("pnpm --dir app exec electron .")
    );
    assert_eq!(
        module.build_command.as_deref(),
        Some("pnpm --dir app exec electron-forge make")
    );
}
