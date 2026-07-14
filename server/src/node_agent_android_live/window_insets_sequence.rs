use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use crate::node_agent_android_inspector::adb_capture::{
    capture_snapshot, launch_app, resolve_online_device_id,
};
use crate::node_agent_android_inspector::adb_command::run_adb_text;
use crate::node_agent_android_inspector::types::{BoundsRect, CaptureRequest, RuntimeUiNode};
use crate::node_agent_android_inspector::xml_parser::parse_bounds;

const MAX_ADB_TEXT: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceRequest {
    steps: Vec<TraceStep>,
    selectors: Vec<NodeSelector>,
    #[serde(default = "default_settle_ms")]
    settle_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceStep {
    name: String,
    action: TraceAction,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
enum TraceAction {
    Launch,
    Tap {
        x: i32,
        y: i32,
    },
    TapNode {
        #[serde(flatten)]
        selector: ActionNodeSelector,
    },
    Back,
    Wait,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeSelector {
    label: String,
    #[serde(flatten)]
    matcher: NodeMatcher,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionNodeSelector {
    #[serde(flatten)]
    matcher: NodeMatcher,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeMatcher {
    resource_id_suffix: Option<String>,
    text: Option<String>,
    content_description: Option<String>,
    #[serde(default)]
    occurrence: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct InsetsEdges {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsetsSnapshot {
    display_width: i32,
    display_height: i32,
    current_focus: Option<String>,
    system_bars: InsetsEdges,
    ime: InsetsEdges,
    visible_sources: Vec<String>,
}

#[derive(Debug, Clone)]
struct InsetsSource {
    source_type: String,
    frame: BoundsRect,
    visible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeMeasurement {
    label: String,
    matched: bool,
    resource_id: Option<String>,
    text: Option<String>,
    bounds: Option<BoundsRect>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraceState {
    name: String,
    activity_name: Option<String>,
    captured_at: String,
    screenshot_path: Option<String>,
    insets: InsetsSnapshot,
    nodes: Vec<NodeMeasurement>,
}

pub(crate) async fn run(session: &LiveUiSession, arguments: Value) -> Result<Value> {
    let request: TraceRequest = serde_json::from_value(arguments).context("页面序列参数无效")?;
    validate_request(&request)?;
    let view = session.view().await;
    if !view.connected {
        bail!("WINDOW_INSETS_SEQUENCE_TRACE 需要已连接的真实 Android Renderer")
    }
    let device_id = resolve_online_device_id(&session.device_id).await?;
    let mut previous_nodes = Vec::new();
    let mut states = Vec::with_capacity(request.steps.len());
    for step in &request.steps {
        perform_action(
            &device_id,
            &session.package_name,
            &step.action,
            &previous_nodes,
        )
        .await
        .with_context(|| format!("执行页面序列步骤 {} 失败", step.name))?;
        tokio::time::sleep(Duration::from_millis(request.settle_ms)).await;
        let snapshot = capture_snapshot(CaptureRequest {
            device_id: device_id.clone(),
            package_name: Some(session.package_name.clone()),
            include_raw_xml: Some(false),
            include_screenshot_data_url: Some(false),
            launch_app: Some(false),
            project_root: session.project_root.clone(),
            lease: None,
        })
        .await
        .with_context(|| format!("采集页面序列步骤 {} 失败", step.name))?;
        let screenshot = snapshot
            .screenshot
            .as_ref()
            .ok_or_else(|| anyhow!("真机快照缺少屏幕尺寸"))?;
        let window_dump = window_dump(&device_id).await?;
        let insets = parse_window_insets(
            &window_dump,
            screenshot.width as i32,
            screenshot.height as i32,
        )?;
        let nodes = request
            .selectors
            .iter()
            .map(|selector| measure_node(selector, &snapshot.nodes))
            .collect();
        previous_nodes = snapshot.nodes;
        states.push(TraceState {
            name: step.name.clone(),
            activity_name: snapshot.activity_name,
            captured_at: snapshot.captured_at,
            screenshot_path: snapshot.artifact.map(|artifact| artifact.screenshot_path),
            insets,
            nodes,
        });
    }
    Ok(json!({
        "capability": "WINDOW_INSETS_SEQUENCE_TRACE",
        "deviceId": device_id,
        "packageName": session.package_name,
        "states": states,
        "comparisons": compact_comparisons(&states),
    }))
}

fn validate_request(request: &TraceRequest) -> Result<()> {
    if request.steps.is_empty() || request.steps.len() > 16 {
        bail!("steps 数量必须为 1..16")
    }
    if request.selectors.is_empty() || request.selectors.len() > 16 {
        bail!("selectors 数量必须为 1..16")
    }
    if !(100..=5_000).contains(&request.settle_ms) {
        bail!("settleMs 必须为 100..5000")
    }
    for step in &request.steps {
        if step.name.trim().is_empty() || step.name.chars().count() > 80 {
            bail!("步骤 name 必须为 1..80 字")
        }
    }
    for selector in &request.selectors {
        if selector.label.trim().is_empty() || selector.label.chars().count() > 80 {
            bail!("selector.label 必须为 1..80 字")
        }
        validate_matcher(&selector.matcher)?;
    }
    Ok(())
}

fn validate_matcher(matcher: &NodeMatcher) -> Result<()> {
    if matcher
        .resource_id_suffix
        .as_deref()
        .is_none_or(str::is_empty)
        && matcher.text.as_deref().is_none_or(str::is_empty)
        && matcher
            .content_description
            .as_deref()
            .is_none_or(str::is_empty)
    {
        bail!("节点选择器至少需要 resourceIdSuffix、text 或 contentDescription")
    }
    if matcher.occurrence > 50 {
        bail!("occurrence 不能大于 50")
    }
    Ok(())
}

async fn perform_action(
    device_id: &str,
    package_name: &str,
    action: &TraceAction,
    previous_nodes: &[RuntimeUiNode],
) -> Result<()> {
    match action {
        TraceAction::Launch => {
            launch_app(device_id, package_name).await?;
        }
        TraceAction::Tap { x, y } => {
            if *x < 0 || *y < 0 {
                bail!("TAP 坐标不能为负数")
            }
            adb_input(device_id, &["tap", &x.to_string(), &y.to_string()]).await?;
        }
        TraceAction::TapNode { selector } => {
            validate_matcher(&selector.matcher)?;
            let node = find_node(&selector.matcher, previous_nodes)
                .ok_or_else(|| anyhow!("TAP_NODE 未匹配到上一状态节点"))?;
            let x = (node.bounds.left + node.bounds.right) / 2;
            let y = (node.bounds.top + node.bounds.bottom) / 2;
            adb_input(device_id, &["tap", &x.to_string(), &y.to_string()]).await?;
        }
        TraceAction::Back => {
            adb_input(device_id, &["keyevent", "KEYCODE_BACK"]).await?;
        }
        TraceAction::Wait => {}
    }
    Ok(())
}

async fn adb_input(device_id: &str, input_args: &[&str]) -> Result<String> {
    let mut args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "shell".to_string(),
        "input".to_string(),
    ];
    args.extend(input_args.iter().map(|value| value.to_string()));
    run_adb_text(&args, Duration::from_secs(5), 64 * 1024).await
}

async fn window_dump(device_id: &str) -> Result<String> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "shell".to_string(),
        "dumpsys".to_string(),
        "window".to_string(),
    ];
    run_adb_text(&args, Duration::from_secs(8), MAX_ADB_TEXT).await
}

fn parse_window_insets(
    raw: &str,
    display_width: i32,
    display_height: i32,
) -> Result<InsetsSnapshot> {
    let sources = raw
        .lines()
        .filter_map(parse_insets_source)
        .collect::<Vec<_>>();
    if !sources.iter().any(|source| {
        source.visible && matches!(source.source_type.as_str(), "statusBars" | "navigationBars")
    }) {
        bail!("dumpsys window 未返回可见 system bar InsetsSource，无法生成真实 Insets 轨迹")
    }
    let system_bars = insets_for_types(
        &sources,
        display_width,
        display_height,
        &["statusBars", "navigationBars", "captionBar"],
    );
    let ime = insets_for_types(&sources, display_width, display_height, &["ime"]);
    let visible_sources = sources
        .iter()
        .filter(|source| source.visible)
        .map(|source| source.source_type.clone())
        .collect::<Vec<_>>();
    Ok(InsetsSnapshot {
        display_width,
        display_height,
        current_focus: raw.lines().find_map(|line| {
            line.split_once("mCurrentFocus=")
                .map(|(_, value)| value.trim().to_string())
        }),
        system_bars,
        ime,
        visible_sources,
    })
}

fn parse_insets_source(line: &str) -> Option<InsetsSource> {
    let (_, kind_tail) = line.split_once("type=")?;
    let kind_end = kind_tail
        .find(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '}'))
        .unwrap_or(kind_tail.len());
    let source_type = kind_tail[..kind_end].trim().to_string();
    if source_type.is_empty() {
        return None;
    }
    let (_, frame_tail) = line.split_once("frame=")?;
    let split = frame_tail.find("][")?;
    let end = frame_tail[split + 2..].find(']')? + split + 3;
    let frame = parse_bounds(&frame_tail[..end])?;
    let visible = line
        .split_once("visible=")
        .and_then(|(_, value)| value.split_whitespace().next())
        .is_some_and(|value| value.trim_end_matches(['}', ',']) == "true");
    Some(InsetsSource {
        source_type,
        frame,
        visible,
    })
}

fn insets_for_types(
    sources: &[InsetsSource],
    display_width: i32,
    display_height: i32,
    types: &[&str],
) -> InsetsEdges {
    let mut result = InsetsEdges {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    for source in sources
        .iter()
        .filter(|source| source.visible && types.contains(&source.source_type.as_str()))
    {
        let frame = &source.frame;
        if frame.left <= 0 && frame.right >= display_width {
            if frame.top <= 0 {
                result.top = result.top.max(frame.bottom.max(0));
            }
            if frame.bottom >= display_height {
                result.bottom = result.bottom.max((display_height - frame.top).max(0));
            }
        }
        if frame.top <= 0 && frame.bottom >= display_height {
            if frame.left <= 0 {
                result.left = result.left.max(frame.right.max(0));
            }
            if frame.right >= display_width {
                result.right = result.right.max((display_width - frame.left).max(0));
            }
        }
    }
    result
}

fn find_node<'a>(matcher: &NodeMatcher, nodes: &'a [RuntimeUiNode]) -> Option<&'a RuntimeUiNode> {
    nodes
        .iter()
        .filter(|node| {
            matcher
                .resource_id_suffix
                .as_deref()
                .is_none_or(|expected| {
                    node.resource_id
                        .as_deref()
                        .is_some_and(|actual| actual.ends_with(expected))
                })
                && matcher
                    .text
                    .as_deref()
                    .is_none_or(|expected| node.text == expected)
                && matcher
                    .content_description
                    .as_deref()
                    .is_none_or(|expected| node.content_desc == expected)
        })
        .nth(matcher.occurrence)
}

fn measure_node(selector: &NodeSelector, nodes: &[RuntimeUiNode]) -> NodeMeasurement {
    let node = find_node(&selector.matcher, nodes);
    NodeMeasurement {
        label: selector.label.clone(),
        matched: node.is_some(),
        resource_id: node.and_then(|node| node.resource_id.clone()),
        text: node
            .map(|node| node.text.clone())
            .filter(|text| !text.is_empty()),
        bounds: node.map(|node| node.bounds.clone()),
    }
}

fn compact_comparisons(states: &[TraceState]) -> Vec<Value> {
    let Some(baseline) = states.first() else {
        return Vec::new();
    };
    states
        .iter()
        .skip(1)
        .map(|state| {
            let baseline_nodes = baseline
                .nodes
                .iter()
                .map(|node| (node.label.as_str(), node))
                .collect::<BTreeMap<_, _>>();
            let nodes = state
                .nodes
                .iter()
                .filter_map(|node| {
                    let before = baseline_nodes.get(node.label.as_str())?;
                    Some(json!({
                        "label": node.label,
                        "matchedBoth": before.matched && node.matched,
                        "deltaPx": bounds_delta(before.bounds.as_ref(), node.bounds.as_ref()),
                    }))
                })
                .collect::<Vec<_>>();
            json!({
                "from": baseline.name,
                "to": state.name,
                "systemBarsDeltaPx": edges_delta(&baseline.insets.system_bars, &state.insets.system_bars),
                "imeDeltaPx": edges_delta(&baseline.insets.ime, &state.insets.ime),
                "nodes": nodes,
            })
        })
        .collect()
}

fn bounds_delta(before: Option<&BoundsRect>, after: Option<&BoundsRect>) -> Option<Value> {
    Some(json!({
        "left": after?.left - before?.left,
        "top": after?.top - before?.top,
        "right": after?.right - before?.right,
        "bottom": after?.bottom - before?.bottom,
        "width": after?.width - before?.width,
        "height": after?.height - before?.height,
    }))
}

fn edges_delta(before: &InsetsEdges, after: &InsetsEdges) -> InsetsEdges {
    InsetsEdges {
        left: after.left - before.left,
        top: after.top - before.top,
        right: after.right - before.right,
        bottom: after.bottom - before.bottom,
    }
}

fn default_settle_ms() -> u64 {
    700
}

#[cfg(test)]
#[path = "window_insets_sequence_tests.rs"]
mod tests;
