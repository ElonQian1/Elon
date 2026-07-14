use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use super::broker::LiveUiSession;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeometryAssertion {
    name: String,
    left: GeometryOperand,
    right: GeometryOperand,
    #[serde(default)]
    expected_delta_px: i64,
    tolerance_px: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeometryOperand {
    step: String,
    source: GeometrySource,
    selector: Option<String>,
    anchor: GeometryAnchor,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GeometrySource {
    Node,
    Display,
    SafeContent,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GeometryAnchor {
    Left,
    Top,
    Right,
    Bottom,
    CenterX,
    CenterY,
    Width,
    Height,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl Bounds {
    fn width(self) -> i64 {
        self.right - self.left
    }

    fn height(self) -> i64 {
        self.bottom - self.top
    }
}

struct ResolvedOperand {
    value: i64,
    evidence: Value,
}

pub(crate) async fn run(session: &LiveUiSession, mut arguments: Value) -> Result<Value> {
    let assertions: Vec<GeometryAssertion> = serde_json::from_value(
        arguments
            .get("assertions")
            .cloned()
            .ok_or_else(|| anyhow!("缺少 assertions"))?,
    )
    .context("关系几何断言参数无效")?;
    validate_assertions(&assertions)?;
    arguments
        .as_object_mut()
        .ok_or_else(|| anyhow!("关系几何参数必须是对象"))?
        .remove("assertions");
    let trace = super::window_insets_sequence::run(session, arguments).await?;
    evaluate_trace(&trace, &assertions)
}

fn validate_assertions(assertions: &[GeometryAssertion]) -> Result<()> {
    if assertions.is_empty() || assertions.len() > 32 {
        bail!("assertions 数量必须为 1..32")
    }
    for assertion in assertions {
        if assertion.name.trim().is_empty() || assertion.name.chars().count() > 120 {
            bail!("assertion.name 必须为 1..120 字")
        }
        if assertion.tolerance_px > 10_000 {
            bail!("tolerancePx 不能大于 10000")
        }
        validate_operand(&assertion.left)?;
        validate_operand(&assertion.right)?;
    }
    Ok(())
}

fn validate_operand(operand: &GeometryOperand) -> Result<()> {
    if operand.step.trim().is_empty() || operand.step.chars().count() > 80 {
        bail!("operand.step 必须为 1..80 字")
    }
    match operand.source {
        GeometrySource::Node => {
            if operand.selector.as_deref().is_none_or(str::is_empty) {
                bail!("NODE operand 必须提供 selector")
            }
        }
        GeometrySource::Display | GeometrySource::SafeContent => {
            if operand.selector.is_some() {
                bail!("DISPLAY/SAFE_CONTENT operand 不应提供 selector")
            }
        }
    }
    Ok(())
}

fn evaluate_trace(trace: &Value, assertions: &[GeometryAssertion]) -> Result<Value> {
    let states = trace
        .get("states")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("真实 Android 序列轨迹缺少 states"))?;
    let results = assertions
        .iter()
        .map(|assertion| evaluate_assertion(states, assertion))
        .collect::<Vec<_>>();
    let passed = results
        .iter()
        .filter(|result| result.get("passed") == Some(&Value::Bool(true)))
        .count();
    let failed = results.len() - passed;
    Ok(json!({
        "capability": "RELATIONAL_LAYOUT_GEOMETRY_TRACE",
        "status": if failed == 0 { "PASSED" } else { "FAILED" },
        "deviceId": trace.get("deviceId"),
        "packageName": trace.get("packageName"),
        "summary": {
            "assertionCount": results.len(),
            "passed": passed,
            "failed": failed,
            "allPassed": failed == 0,
        },
        "assertions": results,
        "selectorStability": selector_stability(states, assertions),
        "states": states,
    }))
}

fn evaluate_assertion(states: &[Value], assertion: &GeometryAssertion) -> Value {
    let left = resolve_operand(states, &assertion.left);
    let right = resolve_operand(states, &assertion.right);
    match (left, right) {
        (Ok(left), Ok(right)) => {
            let delta = left.value - right.value;
            let error = (delta - assertion.expected_delta_px).unsigned_abs();
            json!({
                "name": assertion.name,
                "left": left.evidence,
                "right": right.evidence,
                "deltaPx": delta,
                "expectedDeltaPx": assertion.expected_delta_px,
                "errorPx": error,
                "tolerancePx": assertion.tolerance_px,
                "passed": error <= assertion.tolerance_px,
            })
        }
        (left, right) => json!({
            "name": assertion.name,
            "expectedDeltaPx": assertion.expected_delta_px,
            "tolerancePx": assertion.tolerance_px,
            "passed": false,
            "error": ([left.err(), right.err()]
                .into_iter()
                .flatten()
                .map(|error| format!("{error:#}"))
                .collect::<Vec<_>>()
                .join("; ")),
        }),
    }
}

fn resolve_operand(states: &[Value], operand: &GeometryOperand) -> Result<ResolvedOperand> {
    let state = states
        .iter()
        .find(|state| state.get("name").and_then(Value::as_str) == Some(operand.step.as_str()))
        .ok_or_else(|| anyhow!("未找到状态 {}", operand.step))?;
    let (bounds, identity) = match operand.source {
        GeometrySource::Node => {
            let selector = operand.selector.as_deref().unwrap_or_default();
            let node = state
                .get("nodes")
                .and_then(Value::as_array)
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .find(|node| node.get("label").and_then(Value::as_str) == Some(selector))
                })
                .ok_or_else(|| anyhow!("状态 {} 未包含 selector {}", operand.step, selector))?;
            if node.get("matched") != Some(&Value::Bool(true)) {
                bail!("状态 {} 的 selector {} 未匹配节点", operand.step, selector)
            }
            let bounds = bounds_from_value(
                node.get("bounds")
                    .ok_or_else(|| anyhow!("selector {selector} 缺少 bounds"))?,
            )?;
            let identity = node
                .get("resourceId")
                .and_then(Value::as_str)
                .or_else(|| node.get("text").and_then(Value::as_str))
                .map(str::to_string);
            (bounds, identity)
        }
        GeometrySource::Display => (display_bounds(state)?, None),
        GeometrySource::SafeContent => (safe_content_bounds(state)?, None),
    };
    let value = anchor_value(bounds, operand.anchor);
    Ok(ResolvedOperand {
        value,
        evidence: json!({
            "step": operand.step,
            "source": source_name(operand.source),
            "selector": operand.selector,
            "anchor": anchor_name(operand.anchor),
            "valuePx": value,
            "identity": identity,
            "bounds": bounds_json(bounds),
        }),
    })
}

fn display_bounds(state: &Value) -> Result<Bounds> {
    let insets = state
        .get("insets")
        .ok_or_else(|| anyhow!("状态缺少 insets"))?;
    Ok(Bounds {
        left: 0,
        top: 0,
        right: int_field(insets, "displayWidth")?,
        bottom: int_field(insets, "displayHeight")?,
    })
}

fn safe_content_bounds(state: &Value) -> Result<Bounds> {
    let display = display_bounds(state)?;
    let bars = state
        .pointer("/insets/systemBars")
        .ok_or_else(|| anyhow!("状态缺少 systemBars"))?;
    Ok(Bounds {
        left: int_field(bars, "left")?,
        top: int_field(bars, "top")?,
        right: display.right - int_field(bars, "right")?,
        bottom: display.bottom - int_field(bars, "bottom")?,
    })
}

fn bounds_from_value(value: &Value) -> Result<Bounds> {
    Ok(Bounds {
        left: int_field(value, "left")?,
        top: int_field(value, "top")?,
        right: int_field(value, "right")?,
        bottom: int_field(value, "bottom")?,
    })
}

fn int_field(value: &Value, field: &str) -> Result<i64> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("缺少整数几何字段 {field}"))
}

fn anchor_value(bounds: Bounds, anchor: GeometryAnchor) -> i64 {
    match anchor {
        GeometryAnchor::Left => bounds.left,
        GeometryAnchor::Top => bounds.top,
        GeometryAnchor::Right => bounds.right,
        GeometryAnchor::Bottom => bounds.bottom,
        GeometryAnchor::CenterX => bounds.left + bounds.width() / 2,
        GeometryAnchor::CenterY => bounds.top + bounds.height() / 2,
        GeometryAnchor::Width => bounds.width(),
        GeometryAnchor::Height => bounds.height(),
    }
}

fn selector_stability(states: &[Value], assertions: &[GeometryAssertion]) -> Vec<Value> {
    let mut references = BTreeMap::<&str, BTreeSet<&str>>::new();
    for operand in assertions
        .iter()
        .flat_map(|assertion| [&assertion.left, &assertion.right])
        .filter(|operand| matches!(operand.source, GeometrySource::Node))
    {
        references
            .entry(operand.selector.as_deref().unwrap_or_default())
            .or_default()
            .insert(operand.step.as_str());
    }
    references
        .into_iter()
        .map(|(selector, steps)| {
            let mut matched_steps = Vec::new();
            let mut identities = BTreeSet::new();
            for step in &steps {
                if let Some(node) = find_measured_node(states, step, selector) {
                    if node.get("matched") == Some(&Value::Bool(true)) {
                        matched_steps.push(*step);
                        if let Some(identity) = node
                            .get("resourceId")
                            .and_then(Value::as_str)
                            .or_else(|| node.get("text").and_then(Value::as_str))
                        {
                            identities.insert(identity);
                        }
                    }
                }
            }
            json!({
                "selector": selector,
                "referencedSteps": steps,
                "matchedSteps": matched_steps,
                "identities": identities,
                "stable": matched_steps.len() == steps.len() && identities.len() == 1,
            })
        })
        .collect()
}

fn find_measured_node<'a>(states: &'a [Value], step: &str, selector: &str) -> Option<&'a Value> {
    states
        .iter()
        .find(|state| state.get("name").and_then(Value::as_str) == Some(step))?
        .get("nodes")?
        .as_array()?
        .iter()
        .find(|node| node.get("label").and_then(Value::as_str) == Some(selector))
}

fn bounds_json(bounds: Bounds) -> Value {
    json!({
        "left": bounds.left,
        "top": bounds.top,
        "right": bounds.right,
        "bottom": bounds.bottom,
        "width": bounds.width(),
        "height": bounds.height(),
    })
}

fn source_name(source: GeometrySource) -> &'static str {
    match source {
        GeometrySource::Node => "NODE",
        GeometrySource::Display => "DISPLAY",
        GeometrySource::SafeContent => "SAFE_CONTENT",
    }
}

fn anchor_name(anchor: GeometryAnchor) -> &'static str {
    match anchor {
        GeometryAnchor::Left => "LEFT",
        GeometryAnchor::Top => "TOP",
        GeometryAnchor::Right => "RIGHT",
        GeometryAnchor::Bottom => "BOTTOM",
        GeometryAnchor::CenterX => "CENTER_X",
        GeometryAnchor::CenterY => "CENTER_Y",
        GeometryAnchor::Width => "WIDTH",
        GeometryAnchor::Height => "HEIGHT",
    }
}

#[cfg(test)]
#[path = "relational_layout_geometry_tests.rs"]
mod tests;
