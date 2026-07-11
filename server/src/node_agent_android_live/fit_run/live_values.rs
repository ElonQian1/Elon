use std::fs::File;
use std::io::Read;

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

use super::model::FitRunDocument;
use crate::node_agent_android_live::protocol::{LivePatchOperation, LivePropertyValue, LiveUiNode};

pub(super) fn resolve_runtime_node<'a>(
    run: &FitRunDocument,
    nodes: &'a [LiveUiNode],
) -> Result<&'a LiveUiNode> {
    if let Some(node) = nodes
        .iter()
        .find(|node| node.runtime_node_id == run.pair.runtime_node_id)
    {
        if node.definition_id == run.pair.definition_id
            && node.instance_key == run.pair.instance_key
        {
            return Ok(node);
        }
        bail!("runtimeNodeId 已指向不同稳定节点，必须重新绑定");
    }
    let matches = nodes
        .iter()
        .filter(|node| {
            node.definition_id == run.pair.definition_id
                && run
                    .pair
                    .instance_key
                    .as_ref()
                    .is_none_or(|key| node.instance_key.as_ref() == Some(key))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [node] => Ok(*node),
        [] => bail!("FitRun 目标节点在当前 Runtime 树中不存在"),
        _ => bail!("稳定 Node ID 对应多个运行实例；必须提供 instanceKey 后重新绑定"),
    }
}

pub(super) fn inverse_operations(values: &[serde_json::Value]) -> Result<Vec<LivePatchOperation>> {
    values
        .iter()
        .map(|operation| {
            let property = operation
                .get("property")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("FitRun candidate operation 缺少 property"))?;
            let before = operation
                .get("beforeValue")
                .cloned()
                .ok_or_else(|| anyhow!("取消 FitRun 需要 operation.beforeValue: {property}"))?;
            Ok(LivePatchOperation {
                property: property.to_string(),
                value: serde_json::from_value(before)?,
            })
        })
        .collect()
}

pub(super) fn candidate_operation_value(
    operation: &LivePatchOperation,
    node: &LiveUiNode,
    previous_best: Option<&[serde_json::Value]>,
) -> serde_json::Result<serde_json::Value> {
    let mut value = serde_json::to_value(operation)?;
    let inherited = previous_best.and_then(|operations| {
        operations.iter().find_map(|candidate| {
            (candidate
                .get("property")
                .and_then(serde_json::Value::as_str)
                == Some(operation.property.as_str()))
            .then(|| candidate.get("beforeValue").cloned())
            .flatten()
        })
    });
    let before = match inherited {
        Some(value) => Some(value),
        None => baseline_property_value(node, &operation.property)
            .map(serde_json::to_value)
            .transpose()?,
    };
    if let Some(before) = before {
        if let Some(object) = value.as_object_mut() {
            object.insert("beforeValue".to_string(), before);
        }
    }
    Ok(value)
}

fn baseline_property_value(node: &LiveUiNode, property: &str) -> Option<LivePropertyValue> {
    if let Some(value) = node.properties.get(property).and_then(|snapshot| {
        snapshot
            .effective
            .clone()
            .or_else(|| snapshot.measured.clone())
    }) {
        return Some(value);
    }
    let density = node.geometry.density.max(0.01) as f64;
    let (value_type, value) = match property {
        "width" => (
            "dp",
            node.geometry.bounds_in_display_px.width as f64 / density,
        ),
        "height" => (
            "dp",
            node.geometry.bounds_in_display_px.height as f64 / density,
        ),
        "translationX" | "translationY" => ("dp", 0.0),
        "opacity" => ("float", 1.0),
        _ => return None,
    };
    Some(LivePropertyValue {
        value_type: value_type.to_string(),
        value: serde_json::json!((value * 1000.0).round() / 1000.0),
    })
}

pub(super) fn sha256_file(path: &str) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("目标设计图不存在: {path}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect};

    #[test]
    fn solver_operation_carries_real_geometry_baseline() {
        let operation = LivePatchOperation {
            property: "height".to_string(),
            value: LivePropertyValue {
                value_type: "dp".to_string(),
                value: serde_json::json!(28.0),
            },
        };
        let value = candidate_operation_value(&operation, &node(40), None).unwrap();
        assert_eq!(
            value
                .pointer("/beforeValue/value")
                .and_then(serde_json::Value::as_f64),
            Some(20.0)
        );
    }

    #[test]
    fn later_trial_keeps_the_original_fit_run_baseline() {
        let operation = LivePatchOperation {
            property: "height".to_string(),
            value: LivePropertyValue {
                value_type: "dp".to_string(),
                value: serde_json::json!(30.0),
            },
        };
        let previous = vec![serde_json::json!({
            "property": "height",
            "value": {"type": "dp", "value": 28.0},
            "beforeValue": {"type": "dp", "value": 18.0}
        })];
        let value = candidate_operation_value(&operation, &node(56), Some(&previous)).unwrap();
        assert_eq!(
            value
                .pointer("/beforeValue/value")
                .and_then(serde_json::Value::as_f64),
            Some(18.0)
        );
        let inverse = inverse_operations(&[value]).unwrap();
        assert_eq!(inverse[0].value.value, serde_json::json!(18.0));
    }

    fn node(height: i32) -> LiveUiNode {
        LiveUiNode {
            runtime_node_id: "runtime-button".to_string(),
            definition_id: "checkout.button".to_string(),
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
                    left: 0,
                    top: 0,
                    right: 100,
                    bottom: height,
                    width: 100,
                    height,
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
}
