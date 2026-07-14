use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use super::protocol::{LiveRect, LiveUiNode};

pub(crate) fn map_annotations(bundle: &Value, nodes: &[LiveUiNode]) -> Result<Value> {
    let envelope = bundle
        .get("task")
        .ok_or_else(|| anyhow!("设计任务缺少 task.json"))?;
    let intent = envelope
        .pointer("/task/attachment_intent")
        .or_else(|| envelope.pointer("/task/attachmentIntent"))
        .and_then(Value::as_str)
        .unwrap_or("AUTO");
    if intent != "ANNOTATED_CHANGE_REQUEST" {
        bail!("只有 ANNOTATED_CHANGE_REQUEST 使用标注到节点映射；当前是 {intent}");
    }
    if nodes.is_empty() {
        bail!("Runtime 尚未上报节点，无法映射标注区域");
    }
    let attachments = envelope
        .get("attachments")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("设计任务缺少附件元数据"))?;
    let attachment = attachments
        .iter()
        .find(|attachment| {
            attachment
                .get("annotations")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
        })
        .ok_or_else(|| anyhow!("标注修改任务没有结构化标注"))?;
    let annotations = attachment["annotations"]
        .as_array()
        .ok_or_else(|| anyhow!("annotations 格式无效"))?;
    let display_width = nodes
        .iter()
        .map(|node| node.geometry.bounds_in_display_px.right)
        .max()
        .filter(|value| *value > 0)
        .ok_or_else(|| anyhow!("Runtime 节点缺少有效显示宽度"))?;
    let display_height = nodes
        .iter()
        .map(|node| node.geometry.bounds_in_display_px.bottom)
        .max()
        .filter(|value| *value > 0)
        .ok_or_else(|| anyhow!("Runtime 节点缺少有效显示高度"))?;

    let mappings = annotations
        .iter()
        .enumerate()
        .map(|(index, annotation)| {
            let rect = normalized_annotation_rect(annotation, display_width, display_height);
            let mut candidates = nodes
                .iter()
                .filter(|node| node.geometry.visible)
                .filter_map(|node| {
                    let score = node_score(&rect, &node.geometry.bounds_in_display_px);
                    (score > 0.0).then(|| {
                        json!({
                            "runtimeNodeId": node.runtime_node_id,
                            "definitionId": node.definition_id,
                            "instanceKey": node.instance_key,
                            "screenId": node.screen_id,
                            "kind": node.kind,
                            "text": node.text,
                            "bounds": node.geometry.bounds_in_display_px,
                            "score": (score * 1000.0).round() / 1000.0,
                        })
                    })
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| {
                right["score"]
                    .as_f64()
                    .partial_cmp(&left["score"].as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            candidates.truncate(3);
            let needs_confirmation = candidates.len() != 1
                || candidates
                    .first()
                    .and_then(|item| item["score"].as_f64())
                    .unwrap_or(0.0)
                    < 0.72;
            json!({
                "annotationIndex": index,
                "note": annotation.get("note").and_then(Value::as_str).unwrap_or(""),
                "displayRect": rect,
                "candidates": candidates,
                "needsConfirmation": needs_confirmation,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "displayWidth": display_width,
        "displayHeight": display_height,
        "mappingCount": mappings.len(),
        "mappings": mappings,
    }))
}

fn normalized_annotation_rect(annotation: &Value, width: i32, height: i32) -> LiveRect {
    let x = number(annotation, "x").clamp(0.0, 1.0);
    let y = number(annotation, "y").clamp(0.0, 1.0);
    let w = number(annotation, "width").clamp(0.0, 1.0 - x);
    let h = number(annotation, "height").clamp(0.0, 1.0 - y);
    let left = (x * f64::from(width)).round() as i32;
    let top = (y * f64::from(height)).round() as i32;
    let right = ((x + w) * f64::from(width)).round() as i32;
    let bottom = ((y + h) * f64::from(height)).round() as i32;
    LiveRect {
        left,
        top,
        right,
        bottom,
        width: (right - left).max(0),
        height: (bottom - top).max(0),
    }
}

fn node_score(target: &LiveRect, node: &LiveRect) -> f64 {
    let intersection_width = (target.right.min(node.right) - target.left.max(node.left)).max(0);
    let intersection_height = (target.bottom.min(node.bottom) - target.top.max(node.top)).max(0);
    let intersection = f64::from(intersection_width * intersection_height);
    if intersection <= 0.0 {
        return 0.0;
    }
    let target_area = f64::from((target.width * target.height).max(1));
    let node_area = f64::from((node.width * node.height).max(1));
    let center_x = (target.left + target.right) / 2;
    let center_y = (target.top + target.bottom) / 2;
    let contains_center = node.left <= center_x
        && node.right >= center_x
        && node.top <= center_y
        && node.bottom >= center_y;
    let overlap = intersection / target_area;
    let area_similarity = target_area.min(node_area) / target_area.max(node_area);
    (if contains_center { 0.62 } else { 0.0 }) + overlap * 0.28 + area_similarity * 0.10
}

fn number(value: &Value, field: &str) -> f64 {
    value.get(field).and_then(Value::as_f64).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_containing_node_scores_above_overlap_only_node() {
        let target = LiveRect {
            left: 10,
            top: 10,
            right: 30,
            bottom: 30,
            width: 20,
            height: 20,
        };
        let centered = LiveRect {
            left: 8,
            top: 8,
            right: 32,
            bottom: 32,
            width: 24,
            height: 24,
        };
        let edge = LiveRect {
            left: 25,
            top: 10,
            right: 45,
            bottom: 30,
            width: 20,
            height: 20,
        };
        assert!(node_score(&target, &centered) > node_score(&target, &edge));
    }
}
