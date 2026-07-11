use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::broker::LiveUiBroker;
use super::protocol::LiveUiNode;

const UI_IR_VERSION: u32 = 1;
const MAX_TARGET_BYTES: usize = 16 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 16_384;
const MAX_IMAGE_PIXELS: u64 = 40 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiIrSnapshotRef {
    pub(crate) snapshot_id: String,
    pub(crate) screenshot_path: String,
    pub(crate) hierarchy_path: Option<String>,
    pub(crate) manifest_path: Option<String>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    #[serde(default)]
    pub(crate) screenshot_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetDesignRef {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) mime_type: String,
    pub(crate) figma_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetDesignUpload {
    pub(crate) name: String,
    pub(crate) data_url: String,
    pub(crate) figma_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BindUiIrRequest {
    pub(crate) snapshot: Option<UiIrSnapshotRef>,
    pub(crate) selected_runtime_node_id: Option<String>,
    #[serde(default)]
    pub(crate) source_candidates: Vec<Value>,
    pub(crate) target_design: Option<TargetDesignRef>,
    #[serde(default)]
    pub(crate) clear_target_design: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiIrScreenSummary {
    pub(crate) screen_id: Option<String>,
    pub(crate) node_count: usize,
    pub(crate) visible_node_count: usize,
    pub(crate) editable_node_count: usize,
    pub(crate) kind_counts: BTreeMap<String, usize>,
    pub(crate) selected_definition_id: Option<String>,
    pub(crate) has_target_design: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiIrDocument {
    pub(crate) version: u32,
    pub(crate) kind: String,
    pub(crate) revision: String,
    pub(crate) generated_at: String,
    pub(crate) session_id: String,
    pub(crate) device_id: String,
    pub(crate) package_name: String,
    pub(crate) project_root: Option<String>,
    pub(crate) tree_revision: u64,
    pub(crate) snapshot: Option<UiIrSnapshotRef>,
    pub(crate) target_design: Option<TargetDesignRef>,
    pub(crate) selected_runtime_node_id: Option<String>,
    pub(crate) source_candidates: Vec<Value>,
    pub(crate) summary: UiIrScreenSummary,
    pub(crate) nodes: Vec<LiveUiNode>,
}

pub(crate) async fn persist_target_design(
    broker: &LiveUiBroker,
    session_id: &str,
    upload: TargetDesignUpload,
) -> Result<TargetDesignRef> {
    let session = broker.session(session_id).await?;
    let (mime_type, bytes) = decode_data_url(&upload.data_url)?;
    let (width, height) = validated_image_dimensions(&bytes, "设计图")?;
    let image = image::load_from_memory(&bytes).context("设计图无法解码")?;
    if image.dimensions() != (width, height) {
        bail!("设计图解码尺寸与文件头不一致");
    }
    let sha256 = sha256_bytes(&bytes);
    let extension = match mime_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        _ => bail!("设计图只支持 PNG/JPEG"),
    };
    let root = live_artifact_dir(session.project_root.as_deref(), session_id)?;
    fs::create_dir_all(&root).with_context(|| format!("创建设计图目录失败: {}", root.display()))?;
    let path = root.join(format!("target-{sha256}.{extension}"));
    fs::write(&path, bytes).with_context(|| format!("保存设计图失败: {}", path.display()))?;
    cleanup_target_files(&root, Some(&path));
    let target = TargetDesignRef {
        id: format!("target_{}", &sha256[..16]),
        name: safe_name(&upload.name),
        path: path.display().to_string(),
        sha256,
        width,
        height,
        mime_type,
        figma_url: validate_figma_url(upload.figma_url)?,
    };
    write_json(&root.join("target-design.json"), &target)?;
    Ok(target)
}

pub(crate) async fn bind_ui_ir(
    broker: &LiveUiBroker,
    session_id: &str,
    mut request: BindUiIrRequest,
) -> Result<UiIrDocument> {
    let session = broker.session(session_id).await?;
    let root = live_artifact_dir(session.project_root.as_deref(), session_id)?;
    fs::create_dir_all(&root)
        .with_context(|| format!("创建 UI IR 目录失败: {}", root.display()))?;
    let existing = load_ui_ir_path(&root.join("ui-ir.json")).ok();
    if request.clear_target_design {
        request.target_design = None;
        let _ = fs::remove_file(root.join("target-design.json"));
        cleanup_target_files(&root, None);
    } else if request.target_design.is_none() {
        request.target_design = existing
            .as_ref()
            .and_then(|value| value.target_design.clone())
            .or_else(|| read_json(&root.join("target-design.json")).ok());
    }
    if request.snapshot.is_none() {
        request.snapshot = existing.as_ref().and_then(|value| value.snapshot.clone());
    }
    if request.source_candidates.is_empty() {
        request.source_candidates = existing
            .as_ref()
            .map(|value| value.source_candidates.clone())
            .unwrap_or_default();
    }
    let view = session.view().await;
    let (tree_revision, nodes) = broker.tree(session_id).await?;
    let summary = summarize(
        &nodes,
        request.selected_runtime_node_id.as_deref(),
        request.target_design.is_some(),
    );
    let revision = document_revision(tree_revision, &request, &nodes)?;
    let document = UiIrDocument {
        version: UI_IR_VERSION,
        kind: "elon.runtime_ui_ir".to_string(),
        revision,
        generated_at: Utc::now().to_rfc3339(),
        session_id: session_id.to_string(),
        device_id: view.device_id,
        package_name: view.package_name,
        project_root: view.project_root,
        tree_revision,
        snapshot: request.snapshot,
        target_design: request.target_design,
        selected_runtime_node_id: request.selected_runtime_node_id,
        source_candidates: request.source_candidates,
        summary,
        nodes,
    };
    write_json(&root.join("ui-ir.json"), &document)?;
    Ok(document)
}

pub(crate) async fn load_or_build_ui_ir(
    broker: &LiveUiBroker,
    session_id: &str,
) -> Result<UiIrDocument> {
    let session = broker.session(session_id).await?;
    let root = live_artifact_dir(session.project_root.as_deref(), session_id)?;
    match load_ui_ir_path(&root.join("ui-ir.json")) {
        Ok(document) => {
            let (tree_revision, _) = broker.tree(session_id).await?;
            if document.tree_revision == tree_revision {
                return Ok(document);
            }
            bind_ui_ir(
                broker,
                session_id,
                BindUiIrRequest {
                    snapshot: document.snapshot,
                    selected_runtime_node_id: document.selected_runtime_node_id,
                    source_candidates: document.source_candidates,
                    target_design: document.target_design,
                    clear_target_design: false,
                },
            )
            .await
        }
        Err(_) => bind_ui_ir(broker, session_id, BindUiIrRequest::default()).await,
    }
}

fn summarize(
    nodes: &[LiveUiNode],
    selected_runtime_node_id: Option<&str>,
    has_target_design: bool,
) -> UiIrScreenSummary {
    let mut kind_counts = BTreeMap::new();
    for node in nodes {
        *kind_counts.entry(node.kind.clone()).or_insert(0) += 1;
    }
    let selected = selected_runtime_node_id
        .and_then(|id| nodes.iter().find(|node| node.runtime_node_id == id));
    UiIrScreenSummary {
        screen_id: selected
            .map(|node| node.screen_id.clone())
            .or_else(|| nodes.first().map(|node| node.screen_id.clone())),
        node_count: nodes.len(),
        visible_node_count: nodes.iter().filter(|node| node.geometry.visible).count(),
        editable_node_count: nodes
            .iter()
            .filter(|node| {
                node.properties
                    .values()
                    .any(|value| value.change_level == "LIVE")
            })
            .count(),
        kind_counts,
        selected_definition_id: selected.map(|node| node.definition_id.clone()),
        has_target_design,
    }
}

pub(crate) fn ui_ir_path(project_root: Option<&str>, session_id: &str) -> Result<PathBuf> {
    Ok(live_artifact_dir(project_root, session_id)?.join("ui-ir.json"))
}

pub(crate) fn persisted_node_property_values(
    project_root: &str,
    session_id: &str,
    runtime_node_id: &str,
    definition_id: &str,
    instance_key: Option<&str>,
) -> Result<BTreeMap<String, Value>> {
    let document = load_ui_ir_path(&ui_ir_path(Some(project_root), session_id)?)?;
    let node = document
        .nodes
        .iter()
        .find(|node| node.runtime_node_id == runtime_node_id)
        .or_else(|| {
            document.nodes.iter().find(|node| {
                node.definition_id == definition_id
                    && instance_key.map_or(true, |key| node.instance_key.as_deref() == Some(key))
            })
        })
        .ok_or_else(|| anyhow!("持久化 UI IR 中找不到 FitRun 目标节点"))?;
    node.properties
        .iter()
        .filter_map(|(property, snapshot)| {
            snapshot.effective.as_ref().map(|value| {
                serde_json::to_value(value).map(|serialized| (property.clone(), serialized))
            })
        })
        .collect::<serde_json::Result<BTreeMap<_, _>>>()
        .map_err(Into::into)
}

fn live_artifact_dir(project_root: Option<&str>, session_id: &str) -> Result<PathBuf> {
    validate_session_id(session_id)?;
    let base = if let Some(value) = project_root.filter(|value| !value.trim().is_empty()) {
        PathBuf::from(value)
            .canonicalize()
            .context("Live UI 项目目录不存在")?
            .join(".elon")
            .join("ui-tuner")
            .join("live")
    } else {
        std::env::temp_dir().join("elon-ui-tuner-live")
    };
    Ok(base.join(session_id))
}

fn validate_session_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 96
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        bail!("Live UI sessionId 非法");
    }
    Ok(())
}

fn decode_data_url(value: &str) -> Result<(String, Vec<u8>)> {
    let (header, payload) = value
        .split_once(',')
        .ok_or_else(|| anyhow!("设计图 data URL 格式错误"))?;
    if !header.starts_with("data:image/") || !header.ends_with(";base64") {
        bail!("设计图必须是 Base64 image data URL");
    }
    let mime_type = header
        .trim_start_matches("data:")
        .trim_end_matches(";base64")
        .to_ascii_lowercase();
    if !matches!(mime_type.as_str(), "image/jpeg" | "image/png") {
        bail!("设计图只支持 PNG/JPEG");
    }
    let max_encoded_len = MAX_TARGET_BYTES.saturating_mul(4).div_ceil(3) + 8;
    if payload.is_empty() || payload.len() > max_encoded_len {
        bail!("设计图 Base64 体积超限");
    }
    let bytes = B64.decode(payload).context("设计图 Base64 解码失败")?;
    if bytes.is_empty() || bytes.len() > MAX_TARGET_BYTES {
        bail!("设计图大小必须在 1..16MiB");
    }
    Ok((mime_type, bytes))
}

fn validated_image_dimensions(bytes: &[u8], label: &str) -> Result<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .with_context(|| format!("{label}格式无法识别"))?;
    let (width, height) = reader
        .into_dimensions()
        .with_context(|| format!("{label}尺寸无法读取"))?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_IMAGE_SIDE
        || height > MAX_IMAGE_SIDE
        || pixels > MAX_IMAGE_PIXELS
    {
        bail!(
            "{label}尺寸超限：{width}x{height}，单边不超过 {MAX_IMAGE_SIDE}，总像素不超过 {MAX_IMAGE_PIXELS}"
        );
    }
    Ok((width, height))
}

fn document_revision(
    tree_revision: u64,
    request: &BindUiIrRequest,
    nodes: &[LiveUiNode],
) -> Result<String> {
    let bytes = serde_json::to_vec(&(tree_revision, request, nodes))?;
    Ok(format!("ir_{}", &sha256_bytes(&bytes)[..20]))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn safe_name(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "目标设计图".to_string()
    } else {
        value.chars().take(120).collect()
    }
}

fn validate_figma_url(value: Option<String>) -> Result<Option<String>> {
    let Some(value) = value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if value.len() > 2_048
        || !(value.starts_with("https://www.figma.com/") || value.starts_with("https://figma.com/"))
    {
        bail!("Figma 链接必须是 figma.com 的 HTTPS URL");
    }
    Ok(Some(value))
}

fn cleanup_target_files(root: &Path, keep: Option<&Path>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_target = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("target-"))
            .unwrap_or(false);
        if is_target && keep != Some(path.as_path()) {
            let _ = fs::remove_file(path);
        }
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_context(|| format!("写入 UI IR 失败: {}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("读取 UI IR 失败: {}", path.display()))?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn load_ui_ir_path(path: &Path) -> Result<UiIrDocument> {
    read_json(path)
}
