use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::types::{
    BoundsRect, RuntimeUiNode, SelectionArtifact, SelectionArtifactRequest, SnapshotArtifact,
};

const MAX_SELECTION_CROP_BYTES: usize = 3 * 1024 * 1024;
const MAX_SNAPSHOTS_PER_PROJECT: usize = 24;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotManifest<'a> {
    version: u32,
    id: &'a str,
    device_id: &'a str,
    package_name: Option<&'a str>,
    activity_name: Option<&'a str>,
    captured_at: &'a str,
    source_root: Option<&'a str>,
    source_fingerprint: Option<&'a str>,
    screenshot_width: u32,
    screenshot_height: u32,
    node_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectionContext<'a> {
    version: u32,
    snapshot_id: &'a str,
    selection_id: &'a str,
    bounds: &'a BoundsRect,
    resource_id: Option<&'a str>,
    component_key: Option<&'a str>,
    crop_path: String,
}

pub(crate) struct PersistSnapshotInput<'a> {
    pub device_id: &'a str,
    pub package_name: Option<&'a str>,
    pub activity_name: Option<&'a str>,
    pub captured_at: &'a str,
    pub source_root: Option<&'a str>,
    pub source_fingerprint: Option<&'a str>,
    pub screenshot_png: &'a [u8],
    pub screenshot_width: u32,
    pub screenshot_height: u32,
    pub raw_xml: &'a str,
    pub nodes: &'a [RuntimeUiNode],
}

pub(crate) fn persist_snapshot(input: PersistSnapshotInput<'_>) -> Result<SnapshotArtifact> {
    let project_key = input
        .source_fingerprint
        .map(str::to_string)
        .unwrap_or_else(|| short_hash(input.package_name.unwrap_or("unknown-package")));
    let id = snapshot_id(input.device_id, input.captured_at);
    let project_dir = artifacts_root().join(project_key);
    let snapshot_dir = project_dir.join(&id);
    fs::create_dir_all(&snapshot_dir)
        .with_context(|| format!("创建真机快照目录失败: {}", snapshot_dir.display()))?;

    let screenshot_path = snapshot_dir.join("screenshot.png");
    let hierarchy_path = snapshot_dir.join("hierarchy.json");
    let raw_xml_path = (!input.raw_xml.is_empty()).then(|| snapshot_dir.join("hierarchy.xml"));
    let manifest_path = snapshot_dir.join("manifest.json");
    fs::write(&screenshot_path, input.screenshot_png)
        .with_context(|| format!("保存真机截图失败: {}", screenshot_path.display()))?;
    write_json(&hierarchy_path, input.nodes)?;
    if let Some(path) = raw_xml_path.as_ref() {
        fs::write(path, input.raw_xml)
            .with_context(|| format!("保存真机 XML 失败: {}", path.display()))?;
    }
    write_json(
        &manifest_path,
        &SnapshotManifest {
            version: 1,
            id: &id,
            device_id: input.device_id,
            package_name: input.package_name,
            activity_name: input.activity_name,
            captured_at: input.captured_at,
            source_root: input.source_root,
            source_fingerprint: input.source_fingerprint,
            screenshot_width: input.screenshot_width,
            screenshot_height: input.screenshot_height,
            node_count: input.nodes.len(),
        },
    )?;
    prune_snapshots(&project_dir, &id);

    Ok(SnapshotArtifact {
        id,
        root_dir: path_string(&snapshot_dir),
        manifest_path: path_string(&manifest_path),
        screenshot_path: path_string(&screenshot_path),
        hierarchy_path: path_string(&hierarchy_path),
        raw_xml_path: raw_xml_path.as_deref().map(path_string),
    })
}

pub(crate) fn persist_selection(req: SelectionArtifactRequest) -> Result<SelectionArtifact> {
    persist_selection_in_root(req, &artifacts_root())
}

fn persist_selection_in_root(
    req: SelectionArtifactRequest,
    artifacts_root: &Path,
) -> Result<SelectionArtifact> {
    validate_identifier(&req.snapshot_id, "快照 ID")?;
    validate_identifier(&req.selection_id, "选中节点 ID")?;
    let snapshot_dir = find_snapshot_dir(artifacts_root, &req.snapshot_id)?;
    let selection_dir = snapshot_dir.join("selections");
    fs::create_dir_all(&selection_dir)
        .with_context(|| format!("创建选区目录失败: {}", selection_dir.display()))?;
    let crop = decode_png_data_url(&req.crop_data_url)?;
    let safe_selection = safe_file_part(&req.selection_id);
    let crop_path = selection_dir.join(format!("{safe_selection}.png"));
    let context_path = selection_dir.join(format!("{safe_selection}.json"));
    fs::write(&crop_path, crop)
        .with_context(|| format!("保存选区截图失败: {}", crop_path.display()))?;
    write_json(
        &context_path,
        &SelectionContext {
            version: 1,
            snapshot_id: &req.snapshot_id,
            selection_id: &req.selection_id,
            bounds: &req.bounds,
            resource_id: req.resource_id.as_deref(),
            component_key: req.component_key.as_deref(),
            crop_path: path_string(&crop_path),
        },
    )?;
    Ok(SelectionArtifact {
        snapshot_id: req.snapshot_id,
        selection_id: req.selection_id,
        crop_path: path_string(&crop_path),
        context_path: path_string(&context_path),
    })
}

fn artifacts_root() -> PathBuf {
    crate::state_path().with_file_name("android-inspector-artifacts")
}

fn snapshot_id(device_id: &str, captured_at: &str) -> String {
    format!("snap_{}", short_hash(&format!("{device_id}:{captured_at}")))
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(&digest[..12])
}

fn write_json(path: &Path, value: &(impl Serialize + ?Sized)) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).context("序列化真机快照失败")?;
    fs::write(path, bytes).with_context(|| format!("保存真机快照失败: {}", path.display()))
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        bail!("{label} 格式不合法");
    }
    Ok(())
}

fn find_snapshot_dir(root: &Path, snapshot_id: &str) -> Result<PathBuf> {
    let entries =
        fs::read_dir(root).with_context(|| format!("读取真机快照目录失败: {}", root.display()))?;
    for entry in entries.flatten() {
        let candidate = entry.path().join(snapshot_id);
        if candidate.join("manifest.json").is_file() {
            return Ok(candidate);
        }
    }
    bail!("找不到对应真机快照，请重新点击调试真机")
}

fn decode_png_data_url(value: &str) -> Result<Vec<u8>> {
    let encoded = value
        .strip_prefix("data:image/png;base64,")
        .context("选区截图必须是 PNG data URL")?;
    let bytes = B64.decode(encoded).context("选区截图 Base64 无法解析")?;
    if bytes.is_empty() || bytes.len() > MAX_SELECTION_CROP_BYTES {
        bail!("选区截图大小不合法");
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        bail!("选区截图不是有效 PNG");
    }
    Ok(bytes)
}

fn safe_file_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

fn prune_snapshots(project_dir: &Path, keep_id: &str) {
    let Ok(entries) = fs::read_dir(project_dir) else {
        return;
    };
    let mut dirs = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let modified = entry.metadata().ok()?.modified().ok()?;
            path.is_dir().then_some((modified, path))
        })
        .collect::<Vec<_>>();
    dirs.sort_by_key(|(modified, _)| *modified);
    let remove_count = dirs.len().saturating_sub(MAX_SNAPSHOTS_PER_PROJECT);
    for (_, path) in dirs.into_iter().take(remove_count) {
        if path.file_name().and_then(|name| name.to_str()) != Some(keep_id) {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{persist_selection_in_root, safe_file_part, validate_identifier};
    use crate::node_agent_android_inspector::types::{BoundsRect, SelectionArtifactRequest};

    #[test]
    fn selection_ids_cannot_escape_artifact_root() {
        assert!(validate_identifier("runtime-node_12", "id").is_ok());
        assert!(validate_identifier("../escape", "id").is_err());
        assert_eq!(safe_file_part("runtime/node:12"), "runtime_node_12");
    }

    #[test]
    fn selection_crop_and_context_are_persisted_beside_snapshot() {
        let root =
            std::env::temp_dir().join(format!("elon-ui-tuner-selection-{}", uuid::Uuid::new_v4()));
        let snapshot = root.join("project").join("snap_e2e");
        fs::create_dir_all(&snapshot).unwrap();
        fs::write(snapshot.join("manifest.json"), b"{}").unwrap();
        let artifact = persist_selection_in_root(
            SelectionArtifactRequest {
                snapshot_id: "snap_e2e".to_string(),
                selection_id: "runtime-node_1".to_string(),
                crop_data_url: concat!(
                    "data:image/png;base64,",
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
                )
                .to_string(),
                bounds: BoundsRect {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                    width: 1,
                    height: 1,
                },
                resource_id: Some("com.elon.app:id/projectCard".to_string()),
                component_key: Some("layout:item_project_card".to_string()),
            },
            &root,
        )
        .unwrap();

        assert!(std::path::Path::new(&artifact.crop_path).is_file());
        let context = fs::read_to_string(&artifact.context_path).unwrap();
        assert!(context.contains("projectCard"));
        assert!(context.contains("layout:item_project_card"));
        let _ = fs::remove_dir_all(root);
    }
}
