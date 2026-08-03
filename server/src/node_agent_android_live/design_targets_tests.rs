use std::fs;

use serde_json::json;

use super::{
    broker::LiveUiBroker,
    design_target_discovery::discover_targets,
    design_targets::{self, DesignPlatform},
};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon_headless_design_{name}_{}",
        uuid::Uuid::new_v4().simple()
    ))
}

#[test]
fn discovers_web_pwa_tauri_and_android_as_independent_targets() {
    let root = fixture_root("targets");
    fs::create_dir_all(root.join("app/src-tauri")).unwrap();
    fs::create_dir_all(root.join("app/public")).unwrap();
    fs::create_dir_all(root.join("android/app/src/main")).unwrap();
    fs::write(
        root.join("app/package.json"),
        r#"{"scripts":{"dev":"vite"},"devDependencies":{"vite-plugin-pwa":"latest"}}"#,
    )
    .unwrap();
    fs::write(root.join("app/public/app.webmanifest"), "{}").unwrap();
    fs::write(root.join("app/src-tauri/tauri.conf.json"), "{}").unwrap();
    fs::write(
        root.join("android/app/src/main/AndroidManifest.xml"),
        "<manifest />",
    )
    .unwrap();

    let (targets, _, truncated) = discover_targets(&root).unwrap();

    assert!(!truncated);
    for platform in [
        DesignPlatform::Web,
        DesignPlatform::Pwa,
        DesignPlatform::Tauri,
        DesignPlatform::Android,
    ] {
        assert!(targets.iter().any(|target| target.platform == platform));
    }
    let tauri = targets
        .iter()
        .find(|target| target.platform == DesignPlatform::Tauri)
        .unwrap();
    assert_eq!(tauri.evidence_level, "TAURI_FRONTEND_ONLY");
    assert!(!tauri.native_host_verified);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn opens_headless_session_without_android_runtime_or_pc_canvas() {
    let root = fixture_root("session");
    fs::create_dir_all(root.join("web")).unwrap();
    fs::write(
        root.join("web/package.json"),
        r#"{"scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    let broker = LiveUiBroker::new();
    let session = broker
        .create_session(
            "ui-design-bootstrap".into(),
            "ui.design.bootstrap".into(),
            Some(root.display().to_string()),
            38917,
        )
        .await;

    let opened = design_targets::call(
        &session,
        "ui_open_design_target",
        json!({"platform":"web","route":"/settings"}),
    )
    .await
    .unwrap();
    let id = opened["session"]["designSessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let surface = design_targets::call(
        &session,
        "ui_get_design_surface",
        json!({"designSessionId":id}),
    )
    .await
    .unwrap();

    assert_eq!(opened["session"]["state"], "READY_FOR_CAPTURE");
    assert_eq!(surface["status"], "AWAITING_CAPTURE");
    assert_eq!(surface["nodes"], json!([]));
    assert_eq!(design_targets::tool_definitions().len(), 5);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn project_session_can_be_resumed_by_another_mcp_session() {
    let root = fixture_root("resume");
    fs::create_dir_all(root.join("web")).unwrap();
    fs::write(
        root.join("web/package.json"),
        r#"{"scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    let broker = LiveUiBroker::new();
    let creator = broker
        .create_session(
            "ui-design-bootstrap-a".into(),
            "ui.design.bootstrap".into(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    let reader = broker
        .create_session(
            "ui-design-bootstrap-b".into(),
            "ui.design.bootstrap".into(),
            Some(root.display().to_string()),
            38918,
        )
        .await;

    let opened = design_targets::call(
        &creator,
        "ui_open_design_target",
        json!({"platform":"web","route":"/dashboard","url":"http://127.0.0.1:4173/dashboard"}),
    )
    .await
    .unwrap();
    let id = opened["session"]["designSessionId"].as_str().unwrap();
    let surface = design_targets::call(
        &reader,
        "ui_get_design_surface",
        json!({"designSessionId":id}),
    )
    .await
    .unwrap();
    let listed = design_targets::call(&reader, "ui_list_design_sessions", json!({}))
        .await
        .unwrap();

    assert_eq!(surface["status"], "AWAITING_CAPTURE");
    assert_eq!(listed["sessions"][0]["designSessionId"], id);
    assert_eq!(listed["sessions"][0]["route"], "/dashboard");
    fs::remove_dir_all(root).unwrap();
}
