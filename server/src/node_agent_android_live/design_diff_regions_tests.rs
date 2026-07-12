use std::collections::BTreeMap;

use image::{DynamicImage, Rgba, RgbaImage};

use super::design_diff_regions::{analyze_design_diff_images, DesignDiffRegionRequest};
use super::protocol::{LiveGeometry, LiveRect, LiveUiNode};

#[test]
fn detects_ps_changed_region_and_prefers_matching_runtime_node() {
    let baseline =
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 300, Rgba([10, 10, 10, 255])));
    let mut target = baseline.to_rgba8();
    for y in 210..250 {
        for x in 40..160 {
            target.put_pixel(x, y, Rgba([100, 80, 220, 255]));
        }
    }
    let target = DynamicImage::ImageRgba8(target);
    let nodes = vec![
        node("root", 0, 0, 200, 300, false),
        node("pay_button", 35, 205, 165, 255, true),
    ];
    let result = analyze_design_diff_images(
        &baseline,
        &target,
        &nodes,
        &DesignDiffRegionRequest::default(),
    )
    .unwrap();

    assert_eq!(result.regions.len(), 1);
    assert_eq!(
        result.regions[0].recommended_runtime_node_id.as_deref(),
        Some("pay_button")
    );
    assert!(result.regions[0].confidence > 0.5);
}

#[test]
fn scales_runtime_bounds_when_target_resolution_differs() {
    let baseline = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 255])));
    let mut target = RgbaImage::from_pixel(200, 200, Rgba([0, 0, 0, 255]));
    for y in 40..100 {
        for x in 20..80 {
            target.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    let result = analyze_design_diff_images(
        &baseline,
        &DynamicImage::ImageRgba8(target),
        &[node("scaled", 10, 20, 40, 50, true)],
        &DesignDiffRegionRequest::default(),
    )
    .unwrap();

    assert_eq!(result.scale_x, 2.0);
    assert_eq!(
        result.regions[0].recommended_runtime_node_id.as_deref(),
        Some("scaled")
    );
}

fn node(id: &str, left: i32, top: i32, right: i32, bottom: i32, editable: bool) -> LiveUiNode {
    let mut capabilities = BTreeMap::new();
    capabilities.insert("resizeWidth".to_string(), editable);
    LiveUiNode {
        runtime_node_id: id.to_string(),
        definition_id: format!("test.{id}"),
        instance_key: None,
        parent_runtime_node_id: None,
        screen_id: "test".to_string(),
        kind: "button".to_string(),
        text: None,
        resource_id: None,
        class_name: "Button".to_string(),
        source: None,
        geometry: LiveGeometry {
            bounds_in_display_px: LiveRect {
                left,
                top,
                right,
                bottom,
                width: right - left,
                height: bottom - top,
            },
            density: 1.0,
            font_scale: 1.0,
            rotation: 0,
            visible: true,
        },
        properties: BTreeMap::new(),
        capabilities,
    }
}
