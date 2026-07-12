use std::collections::BTreeMap;

use anyhow::{bail, Result};
use serde_json::json;

use super::protocol::{LivePatchOperation, LivePropertyValue, LiveUiNode};
use super::visual_diff::PixelRect;

pub(super) fn solver_properties(node: &LiveUiNode, requested: &[String]) -> Result<Vec<String>> {
    let defaults = ["width", "height"];
    let source = if requested.is_empty() {
        defaults.iter().map(ToString::to_string).collect()
    } else {
        requested.to_vec()
    };
    let mut result = Vec::new();
    for property in source {
        if matches!(
            property.as_str(),
            "backgroundColor" | "contentColor" | "borderColor"
        ) {
            continue;
        }
        if !matches!(
            property.as_str(),
            "width"
                | "height"
                | "translationX"
                | "translationY"
                | "opacity"
                | "padding.start"
                | "padding.top"
                | "padding.end"
                | "padding.bottom"
                | "margin.start"
                | "margin.top"
                | "margin.end"
                | "margin.bottom"
                | "cornerRadius.all"
                | "textSize"
                | "fontWeight"
                | "lineHeight"
                | "letterSpacing"
                | "borderWidth"
        ) {
            bail!("视觉求解不支持属性: {property}");
        }
        let editable = node
            .properties
            .get(&property)
            .is_some_and(|value| value.change_level == "LIVE")
            || matches!(property.as_str(), "translationX" | "translationY");
        if editable && !result.contains(&property) {
            result.push(property);
        }
    }
    Ok(result)
}

pub(super) fn initial_values(node: &LiveUiNode, properties: &[String]) -> BTreeMap<String, f64> {
    let density = node.geometry.density.max(0.01) as f64;
    properties
        .iter()
        .map(|property| {
            let fallback = match property.as_str() {
                "width" => node.geometry.bounds_in_display_px.width as f64 / density,
                "height" => node.geometry.bounds_in_display_px.height as f64 / density,
                "opacity" => 1.0,
                _ => 0.0,
            };
            let value = node
                .properties
                .get(property)
                .and_then(|snapshot| snapshot.effective.as_ref())
                .and_then(|value| value.value.as_f64())
                .unwrap_or(fallback);
            (property.clone(), value)
        })
        .collect()
}

pub(super) fn seed_geometry_target(
    values: &mut BTreeMap<String, f64>,
    node: &LiveUiNode,
    target: PixelRect,
) {
    let density = node.geometry.density.max(0.01) as f64;
    let current = &node.geometry.bounds_in_display_px;
    if values.contains_key("width") {
        values.insert(
            "width".to_string(),
            (target.right - target.left).max(1) as f64 / density,
        );
    }
    if values.contains_key("height") {
        values.insert(
            "height".to_string(),
            (target.bottom - target.top).max(1) as f64 / density,
        );
    }
    for (property, delta_px) in [
        ("translationX", target.left - current.left),
        ("translationY", target.top - current.top),
    ] {
        if values.contains_key(property) {
            values.insert(
                property.to_string(),
                values.get(property).copied().unwrap_or_default() + delta_px as f64 / density,
            );
        }
    }
}

pub(super) fn seed_prior_deltas(
    values: &mut BTreeMap<String, f64>,
    node: &LiveUiNode,
    deltas: &BTreeMap<String, f64>,
) {
    for (property, delta) in deltas {
        let Some(current) = values.get(property).copied() else {
            continue;
        };
        if delta.is_finite() {
            values.insert(
                property.clone(),
                constrained_value(node, property, current + delta),
            );
        }
    }
}

pub(super) fn constrained_value(node: &LiveUiNode, property: &str, value: f64) -> f64 {
    let constraints = node
        .properties
        .get(property)
        .and_then(|item| item.constraints.as_ref());
    let minimum = constraints
        .and_then(|value| value.get("minimum"))
        .and_then(|value| value.as_f64())
        .unwrap_or(if property == "opacity" {
            0.0
        } else {
            -10_000.0
        });
    let maximum = constraints
        .and_then(|value| value.get("maximum"))
        .and_then(|value| value.as_f64())
        .unwrap_or(if property == "opacity" { 1.0 } else { 10_000.0 });
    value.clamp(minimum, maximum)
}

pub(super) fn operations_from_values(values: &BTreeMap<String, f64>) -> Vec<LivePatchOperation> {
    values
        .iter()
        .map(|(property, value)| LivePatchOperation {
            property: property.clone(),
            value: LivePropertyValue {
                value_type: match property.as_str() {
                    "textSize" | "lineHeight" => "sp",
                    "fontWeight" | "letterSpacing" => "float",
                    "opacity" => "float",
                    _ => "dp",
                }
                .to_string(),
                value: json!((value * 1000.0).round() / 1000.0),
            },
        })
        .collect()
}

pub(super) fn predicted_rect(node: &LiveUiNode, values: &BTreeMap<String, f64>) -> PixelRect {
    let base = &node.geometry.bounds_in_display_px;
    let density = node.geometry.density.max(0.01) as f64;
    let tx = values.get("translationX").copied().unwrap_or_default() * density;
    let ty = values.get("translationY").copied().unwrap_or_default() * density;
    let width = values
        .get("width")
        .map(|value| value * density)
        .unwrap_or(base.width as f64)
        .max(1.0);
    let height = values
        .get("height")
        .map(|value| value * density)
        .unwrap_or(base.height as f64)
        .max(1.0);
    let left = base.left as f64 + tx;
    let top = base.top as f64 + ty;
    PixelRect {
        left: left.round() as i32,
        top: top.round() as i32,
        right: (left + width).round() as i32,
        bottom: (top + height).round() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_android_live::protocol::{LiveGeometry, LivePropertySnapshot, LiveRect};

    fn editable_node() -> LiveUiNode {
        let property = || LivePropertySnapshot {
            effective: None,
            measured: None,
            change_level: "LIVE".to_string(),
            commit_mode: "DETERMINISTIC".to_string(),
            binding: None,
            constraints: None,
        };
        LiveUiNode {
            runtime_node_id: "runtime-button".to_string(),
            definition_id: "checkout.pay_button".to_string(),
            instance_key: None,
            parent_runtime_node_id: None,
            screen_id: "checkout".to_string(),
            kind: "button".to_string(),
            text: None,
            resource_id: None,
            class_name: "Button".to_string(),
            source: None,
            geometry: LiveGeometry {
                bounds_in_display_px: LiveRect {
                    left: 10,
                    top: 20,
                    right: 110,
                    bottom: 60,
                    width: 100,
                    height: 40,
                },
                density: 2.0,
                font_scale: 1.0,
                rotation: 0,
                visible: true,
            },
            properties: BTreeMap::from([
                ("width".to_string(), property()),
                ("height".to_string(), property()),
            ]),
            capabilities: BTreeMap::new(),
        }
    }

    #[test]
    fn default_properties_exclude_session_translation() {
        assert_eq!(
            solver_properties(&editable_node(), &[]).unwrap(),
            ["width", "height"]
        );
    }

    #[test]
    fn projected_device_rect_seeds_dimensions_in_dp() {
        let node = editable_node();
        let mut values = initial_values(&node, &["width".to_string(), "height".to_string()]);
        seed_geometry_target(
            &mut values,
            &node,
            PixelRect {
                left: 40,
                top: 80,
                right: 240,
                bottom: 180,
            },
        );
        assert_eq!(values["width"], 100.0);
        assert_eq!(values["height"], 50.0);
        assert!(!values.contains_key("translationX"));
    }

    #[test]
    fn accepted_prior_delta_respects_constraints() {
        let mut node = editable_node();
        node.properties.insert(
            "padding.start".to_string(),
            LivePropertySnapshot {
                effective: Some(LivePropertyValue {
                    value_type: "dp".to_string(),
                    value: json!(12.0),
                }),
                measured: None,
                change_level: "LIVE".to_string(),
                commit_mode: "DETERMINISTIC".to_string(),
                binding: None,
                constraints: Some(json!({ "minimum": 0.0, "maximum": 20.0 })),
            },
        );
        let mut values = initial_values(&node, &["padding.start".to_string()]);
        seed_prior_deltas(
            &mut values,
            &node,
            &BTreeMap::from([("padding.start".to_string(), 15.0)]),
        );
        assert_eq!(values["padding.start"], 20.0);
    }
}
