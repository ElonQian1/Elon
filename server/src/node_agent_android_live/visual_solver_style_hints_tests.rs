use std::collections::BTreeMap;

use image::{Rgba, RgbaImage};

use super::protocol::{
    LiveGeometry, LivePropertySnapshot, LivePropertyValue, LiveRect, LiveUiNode,
};
use super::visual_diff::PixelRect;
use super::visual_solver_style_hints::target_color_operations;

#[test]
fn extracts_background_and_content_colors_from_target_crop() {
    let root = std::env::temp_dir().join(format!("elon-style-hint-{}.png", uuid::Uuid::new_v4()));
    let mut image = RgbaImage::from_pixel(120, 60, Rgba([32, 48, 64, 255]));
    for y in 22..38 {
        for x in 42..78 {
            image.put_pixel(x, y, Rgba([238, 238, 238, 255]));
        }
    }
    image.save(&root).unwrap();
    let operations = target_color_operations(
        root.to_str().unwrap(),
        PixelRect {
            left: 0,
            top: 0,
            right: 120,
            bottom: 60,
        },
        &node(),
        &["backgroundColor".into(), "contentColor".into()],
    )
    .unwrap();
    let _ = std::fs::remove_file(root);

    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].property, "backgroundColor");
    assert_eq!(operations[0].value.value, "#FF223344");
    assert_eq!(operations[1].property, "contentColor");
    assert_eq!(operations[1].value.value, "#FFEEEEEE");
}

fn node() -> LiveUiNode {
    let property = || LivePropertySnapshot {
        effective: Some(LivePropertyValue {
            value_type: "argb".into(),
            value: "#FF000000".into(),
        }),
        measured: None,
        change_level: "LIVE".into(),
        commit_mode: "DETERMINISTIC".into(),
        binding: None,
        constraints: None,
    };
    LiveUiNode {
        runtime_node_id: "node".into(),
        definition_id: "test.node".into(),
        instance_key: None,
        parent_runtime_node_id: None,
        screen_id: "test".into(),
        kind: "button".into(),
        text: None,
        resource_id: None,
        class_name: "Button".into(),
        source: None,
        geometry: LiveGeometry {
            bounds_in_display_px: LiveRect::default(),
            density: 1.0,
            font_scale: 1.0,
            rotation: 0,
            visible: true,
        },
        properties: BTreeMap::from([
            ("backgroundColor".into(), property()),
            ("contentColor".into(), property()),
        ]),
        capabilities: BTreeMap::new(),
    }
}
