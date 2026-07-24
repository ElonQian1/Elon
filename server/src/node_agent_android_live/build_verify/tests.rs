use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::node_agent_android_live::broker::LiveUiBroker;
use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect, LiveUiNode};
use image::{DynamicImage, ImageFormat};
use serde_json::json;

#[test]
fn accepts_safe_debug_application_id_suffix() {
    assert_eq!(
        validate_debug_application_id_suffix(".uitest_2").unwrap(),
        ".uitest_2"
    );
}

#[test]
fn rejects_gradle_argument_injection() {
    assert!(validate_debug_application_id_suffix(".uitest -Pbad=true").is_err());
    assert!(validate_debug_application_id_suffix("uitest").is_err());
    assert!(validate_debug_application_id_suffix(".").is_err());
}

#[test]
fn validates_android_base_package_name() {
    assert_eq!(
        validate_package_name("com.elon.app").unwrap(),
        "com.elon.app"
    );
    assert!(validate_package_name("com.elon.app;rm").is_err());
    assert!(validate_package_name("com..app").is_err());
    assert!(validate_package_name("1com.elon.app").is_err());
}

#[test]
fn locates_android_gradle_root_without_leaving_project() {
    let root = std::env::temp_dir().join(format!("elon-build-verify-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("android")).unwrap();
    fs::write(root.join("android/gradlew.bat"), "@echo off").unwrap();
    assert_eq!(
        find_gradle_root(&root).unwrap(),
        root.join("android").canonicalize().unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unicode_node_data_root_uses_native_in_process_kotlin_arguments() {
    let gradle_root = std::env::temp_dir()
        .join(format!("一龙-node-data-{}", uuid::Uuid::new_v4()))
        .join("ElonNodeData")
        .join("android");
    fs::create_dir_all(&gradle_root).unwrap();
    let arguments = super::gradle::debug_build_arguments(
        &gradle_root,
        Some(".uituner_test"),
        Some("一龙调试"),
        false,
    );
    let arguments = arguments
        .iter()
        .map(|value| value.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(arguments
        .iter()
        .any(|value| value == "-Pkotlin.compiler.execution.strategy=in-process"));
    assert!(arguments
        .iter()
        .all(|value| { !value.contains("\\u4E00") && !value.contains("u4E00u9F99") }));
    let encoded = serde_json::to_string(&gradle_root).unwrap();
    let decoded: PathBuf = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, gradle_root);
    assert!(decoded.to_string_lossy().contains("一龙"));
    fs::remove_dir_all(
        gradle_root
            .ancestors()
            .nth(2)
            .expect("unicode fixture root"),
    )
    .unwrap();
}

#[tokio::test]
async fn failed_debug_runtime_releases_deployment_and_keeps_status_tools_responsive() {
    let project_root =
        std::env::temp_dir().join(format!("一龙-runtime-failure-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&project_root).unwrap();
    let broker = LiveUiBroker::default();
    let session = broker
        .create_session(
            "device-a".into(),
            "com.elon.app.uituner_test".into(),
            Some(project_root.to_string_lossy().to_string()),
            17_321,
        )
        .await;
    let deployment = broker
        .debug_deployments
        .acquire("device-a", "com.elon.app.uituner_test")
        .await
        .expect("fixture deployment lease");
    let failed =
        finish_debug_deployment::<()>(deployment, Err(anyhow::anyhow!("simulated Gradle failure")));
    assert!(failed.is_err());

    tokio::time::timeout(
        Duration::from_millis(100),
        broker
            .debug_deployments
            .acquire("device-a", "com.elon.app.uituner_test"),
    )
    .await
    .expect("failed debug build must release the deployment lease")
    .expect("fixture reacquires the released deployment lease");
    let view = tokio::time::timeout(Duration::from_millis(100), session.view())
        .await
        .expect("ui_get_runtime_status must remain responsive");
    assert!(!view.connected);
    let gap = tokio::time::timeout(
        Duration::from_millis(100),
        crate::node_agent_android_live::capability_gap::report_gap(
            &session,
            &json!({
                "taskId":"runtime_failure_regression",
                "executionMode":"BUSINESS_THREAD",
                "deliveryImpact":"DELIVERY_BLOCKING",
                "missingCapabilities":["PLATFORM_TOOL_DEFECT"],
                "evidence":["simulated failed debug-runtime build"],
                "proposedChanges":["keep status and gap tools responsive"],
                "resumeTarget":"retry runtime preparation"
            }),
        ),
    )
    .await
    .expect("ui_report_capability_gap must remain responsive")
    .expect("capability gap audit should be writable");
    assert_eq!(gap["gap"]["status"], "DEFERRED");
    fs::remove_dir_all(project_root).unwrap();
}

#[test]
fn matches_compose_and_view_preview_nodes() {
    let mut node = LiveUiNode {
        runtime_node_id: "runtime-1".to_string(),
        definition_id: "preview.compose.primary_card".to_string(),
        instance_key: None,
        parent_runtime_node_id: None,
        screen_id: "elon.compose.gallery".to_string(),
        kind: "compose".to_string(),
        text: None,
        resource_id: None,
        class_name: "PrimaryCard".to_string(),
        source: None,
        geometry: Default::default(),
        properties: Default::default(),
        capabilities: Default::default(),
    };
    assert!(nodes_match_preview(&[node.clone()], "elon.compose.gallery"));

    node.screen_id = "com.elon.uiruntime.view.UiRuntimePreviewHostActivity".to_string();
    node.definition_id = "preview.elon.view.gallery.root".to_string();
    assert!(nodes_match_preview(&[node], "elon.view.gallery"));
}

#[test]
fn verification_target_requires_unique_stable_instance() {
    let nodes = vec![
        test_node("runtime-1", None, 0),
        test_node("runtime-2", None, 100),
    ];
    assert!(verification_bounds(&nodes, Some("card.action"), None).is_err());
}

#[test]
fn verification_target_respects_instance_key() {
    let nodes = vec![
        test_node("runtime-1", Some("sku-1"), 0),
        test_node("runtime-2", Some("sku-2"), 100),
    ];
    let bounds = verification_bounds(&nodes, Some("card.action"), Some("sku-2"))
        .unwrap()
        .unwrap();
    assert_eq!(bounds.left, 100);
}

#[test]
fn exact_process_frames_override_stale_node_geometry() {
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::new_rgba8(8, 8)
        .write_to(&mut encoded, ImageFormat::WebP)
        .expect("encode frame");
    let frame = encoded.into_inner();
    let (diff, scope) = compare_source_parity(
        &frame,
        &frame,
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 4,
            bottom: 4,
        }),
        Some(PixelRect {
            left: 4,
            top: 4,
            right: 8,
            bottom: 8,
        }),
    )
    .expect("source parity");

    assert_eq!(scope, "PROCESS_FRAME_EXACT");
    assert_eq!(diff.visual_loss, 0.0);
    assert!(diff.score_report.target_gate.passed);
}

#[test]
fn origin_workspace_revision_must_remain_stable_during_generation_build() {
    let root = std::env::temp_dir().join(format!(
        "elon-origin-proof-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init"]);
    run_git(&root, &["config", "user.email", "test@example.com"]);
    run_git(&root, &["config", "user.name", "Test"]);
    fs::write(root.join("tracked.txt"), "origin").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "init"]);
    let revision = workspace_fingerprint(root.to_str().unwrap())
        .unwrap()
        .unwrap();

    verify_origin_workspace_revision(&root, &revision).unwrap();
    fs::write(root.join("tracked.txt"), "changed while building").unwrap();
    let error = verify_origin_workspace_revision(&root, &revision)
        .unwrap_err()
        .to_string();
    assert!(error.contains("FIT_SOURCE_PROOF_ORIGIN_CHANGED"));
    fs::remove_dir_all(root).unwrap();
}

fn test_node(runtime_id: &str, instance_key: Option<&str>, left: i32) -> LiveUiNode {
    LiveUiNode {
        runtime_node_id: runtime_id.to_string(),
        definition_id: "card.action".to_string(),
        instance_key: instance_key.map(str::to_string),
        parent_runtime_node_id: None,
        screen_id: "catalog".to_string(),
        kind: "button".to_string(),
        text: None,
        resource_id: None,
        class_name: "Button".to_string(),
        source: None,
        geometry: LiveGeometry {
            bounds_in_display_px: LiveRect {
                left,
                top: 0,
                right: left + 80,
                bottom: 40,
                width: 80,
                height: 40,
            },
            density: 2.0,
            font_scale: 1.0,
            rotation: 0,
            visible: true,
        },
        properties: BTreeMap::new(),
        capabilities: BTreeMap::new(),
    }
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    assert!(crate::git_command_error::git_command()
        .args(args)
        .current_dir(root)
        .status()
        .unwrap()
        .success());
}
