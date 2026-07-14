use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;

use super::*;
use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect, LiveUiNode};
use image::{DynamicImage, ImageFormat};

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
