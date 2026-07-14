use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadPreviewRequest {
    pub project_root: String,
    pub layout_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitPreviewRequest {
    pub project_root: String,
    pub layout_file: String,
    pub source_revision: String,
    pub node_key: String,
    pub start_tag_start: usize,
    pub start_tag_end: usize,
    pub changes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourcePreviewDocument {
    pub ok: bool,
    pub ir_kind: String,
    pub ir_version: u32,
    pub project_root: String,
    pub layout_files: Vec<String>,
    pub selected_layout: String,
    pub source_revision: String,
    pub rendering: PreviewRendering,
    pub canvas: PreviewCanvas,
    pub root: PreviewNode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewRendering {
    pub backend: String,
    pub authoritative: bool,
    pub source_of_truth: String,
    pub calibration_required: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewCanvas {
    pub width: f32,
    pub height: f32,
    pub background: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewNode {
    pub key: String,
    pub resource_id: Option<String>,
    pub tag: String,
    pub name: String,
    pub kind: String,
    pub source: PreviewNodeSource,
    pub layout: PreviewLayout,
    pub style: PreviewStyle,
    pub editable: Vec<String>,
    pub children: Vec<PreviewNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewNodeSource {
    pub layout_file: String,
    pub start_tag_start: usize,
    pub start_tag_end: usize,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewLayout {
    pub mode: String,
    pub orientation: String,
    pub width: String,
    pub height: String,
    pub weight: f32,
    pub gravity: String,
    pub margin: EdgeValues,
    pub padding: EdgeValues,
    pub gap: f32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EdgeValues {
    pub start: f32,
    pub top: f32,
    pub end: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewStyle {
    pub text: String,
    pub text_color: String,
    pub background: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub border_radius: f32,
    pub opacity: f32,
    pub visible: bool,
    pub content_description: String,
}
