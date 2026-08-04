use anyhow::{bail, Context, Result};
use elon_pc_dev_runtime::{
    configured_node_data_root, legacy_default_workspace_root, legacy_workspace_root_override,
    NodeDataPaths, NODE_DATA_ROOT_ENV,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub(crate) mod admin;
mod automatic;
mod cleanup;
mod marker;

pub(crate) use automatic::{automatic_fallback_parent, prepare_automatic_root};
pub(crate) use cleanup::{cleanup, CleanupEntry, CleanupResult};
#[cfg(test)]
use marker::read_existing_root_marker;
use marker::{claim_or_verify_root_marker, root_marker_belongs_to};
pub(crate) use marker::{verify_root_marker, verify_root_marker_payload, ROOT_MARKER_FILE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeDataRootSource {
    Environment,
    Persisted,
    Unconfigured,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeDataRootState {
    pub(crate) paths: Option<NodeDataPaths>,
    configured_root_path: Option<PathBuf>,
    pub(crate) source: NodeDataRootSource,
    pub(crate) invalid_reason: Option<String>,
    pub(crate) legacy_workspace_root: Option<PathBuf>,
    pub(crate) legacy_storage_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MigrationPlanItem {
    pub(crate) kind: &'static str,
    pub(crate) source_path: String,
    pub(crate) target_path: Option<String>,
    pub(crate) exists: bool,
    pub(crate) has_data: bool,
    pub(crate) read_only_compatibility: bool,
    pub(crate) strategy: &'static str,
}

pub(crate) fn resolve(
    persisted_root: Option<&str>,
    legacy_workspace_root: Option<PathBuf>,
    legacy_storage_root: Option<PathBuf>,
) -> NodeDataRootState {
    resolve_from_values(
        configured_node_data_root(),
        persisted_root.map(PathBuf::from),
        legacy_workspace_root_override()
            .or(legacy_workspace_root)
            .or_else(|| Some(legacy_default_workspace_root())),
        legacy_storage_root,
    )
}

fn resolve_from_values(
    env_root: Option<PathBuf>,
    persisted_root: Option<PathBuf>,
    legacy_workspace_root: Option<PathBuf>,
    legacy_storage_root: Option<PathBuf>,
) -> NodeDataRootState {
    // node.json is the canonical value after the local API saves a root. The
    // environment is only a bootstrap for nodes that have never persisted one;
    // otherwise an obsolete launcher env file could silently undo a UI change.
    let (configured, source) = if let Some(root) = persisted_root {
        (Some(root), NodeDataRootSource::Persisted)
    } else if let Some(root) = env_root {
        (Some(root), NodeDataRootSource::Environment)
    } else {
        (None, NodeDataRootSource::Unconfigured)
    };

    let configured_root_path = configured.clone();
    let (paths, invalid_reason) = match configured {
        Some(root) => match clean_root(root.to_string_lossy().as_ref()) {
            Ok(root) => (Some(NodeDataPaths::new(root)), None),
            Err(reason) => (None, Some(reason.to_string())),
        },
        None => (None, None),
    };

    NodeDataRootState {
        paths,
        configured_root_path,
        source,
        invalid_reason,
        legacy_workspace_root,
        legacy_storage_root,
    }
}

impl NodeDataRootState {
    pub(crate) fn from_prepared_paths(
        paths: NodeDataPaths,
        source: NodeDataRootSource,
        legacy_workspace_root: Option<PathBuf>,
        legacy_storage_root: Option<PathBuf>,
    ) -> Self {
        Self {
            configured_root_path: Some(paths.root().to_path_buf()),
            paths: Some(paths),
            source,
            invalid_reason: None,
            legacy_workspace_root,
            legacy_storage_root,
        }
    }

    pub(crate) fn block_invalid_root(mut self, reason: impl ToString) -> Self {
        if let Some(paths) = self.paths.as_ref() {
            self.configured_root_path = Some(paths.root().to_path_buf());
        }
        self.paths = None;
        self.invalid_reason = Some(reason.to_string());
        self
    }

    pub(crate) fn configured_root(&self) -> Option<&Path> {
        self.paths
            .as_ref()
            .map(NodeDataPaths::root)
            .or(self.configured_root_path.as_deref())
    }

    pub(crate) fn migration_plan(&self) -> Vec<MigrationPlanItem> {
        let workspace_target = self.paths.as_ref().map(NodeDataPaths::workspaces);
        let storage_target = self.paths.as_ref().map(NodeDataPaths::storage);
        let mut items = Vec::new();
        push_migration_item(
            &mut items,
            "workspace",
            self.legacy_workspace_root.as_deref(),
            workspace_target.as_deref(),
            "旧项目继续原地运行；如用户选择迁移，再确认 Git 状态并从已验证基线在新根重建",
        );
        push_migration_item(
            &mut items,
            "storage",
            self.legacy_storage_root.as_deref(),
            storage_target.as_deref(),
            "旧仓库继续原地运行；如用户选择迁移，复制裸仓库并完成 Git 完整性校验后再切换",
        );
        items
    }

    pub(crate) fn migration_required(&self) -> bool {
        self.migration_plan().iter().any(|item| item.has_data)
    }

    pub(crate) fn status_payload(&self) -> serde_json::Value {
        let plan = self.migration_plan();
        let configured = self.paths.is_some();
        serde_json::json!({
            "configured": configured,
            "configuration_required": false,
            "configuration_recommended": !configured,
            "governance_mode": "advisory",
            "source": self.source,
            "root_path": self.configured_root().map(path_text),
            "workspace_root": self.paths.as_ref().map(|paths| path_text(&paths.workspaces())),
            "storage_root": self.paths.as_ref().map(|paths| path_text(&paths.storage())),
            "compute_plugin_root": self.paths.as_ref().map(|paths| path_text(&paths.compute_plugins())),
            "cache_root": self.paths.as_ref().map(|paths| path_text(&paths.cache())),
            "temp_root": self.paths.as_ref().map(|paths| path_text(&paths.temp())),
            "invalid_reason": self.invalid_reason,
            "migration_required": plan.iter().any(|item| item.has_data),
            "migration_plan": plan,
            "legacy_policy": "inherit_existing_projects_and_caches; recommend_managed_data_root_for_new_projects",
        })
    }
}

pub(crate) fn validate_and_prepare(root: &str, install_id: &str) -> Result<NodeDataPaths> {
    let root = clean_root(root)?;
    ensure_not_filesystem_root(&root)?;
    reject_existing_reparse_ancestors(&root)?;
    if root.exists() && !root.is_dir() {
        bail!("节点数据根已存在但不是目录: {}", root.display());
    }
    std::fs::create_dir_all(&root)
        .with_context(|| format!("无法创建节点数据根 {}", root.display()))?;
    let canonical_root = validate_created_root(&root)?;

    let paths = NodeDataPaths::new(root);
    // Claim ownership before creating any managed child. An unmarked existing
    // directory is accepted only when it is empty, so a broad path such as
    // `%LOCALAPPDATA%` can never turn its existing Temp directory into managed
    // node data.
    claim_or_verify_root_marker(&paths, install_id)?;
    for managed in paths.managed_roots() {
        reject_existing_reparse_ancestors(&managed)?;
        std::fs::create_dir_all(&managed)
            .with_context(|| format!("无法创建节点数据目录 {}", managed.display()))?;
        validate_canonical_managed_path(paths.root(), &canonical_root, &managed)?;
    }
    Ok(paths)
}

pub(crate) fn validate_no_root_overlap(
    root: &str,
    current: &NodeDataRootState,
    install_id: &str,
) -> Result<()> {
    let candidate = clean_root(root)?;
    if current
        .configured_root()
        .is_some_and(|existing| paths_equal(&candidate, existing))
    {
        return Ok(());
    }
    for existing in [
        current.configured_root(),
        current.legacy_workspace_root.as_deref(),
        current.legacy_storage_root.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if is_owned_previous_managed_child(&candidate, existing, install_id) {
            continue;
        }
        if paths_overlap(&candidate, existing) {
            bail!(
                "新数据根不能与现有节点目录互相嵌套: {} <-> {}",
                candidate.display(),
                existing.display()
            );
        }
    }
    Ok(())
}

/// Re-check overlap after the candidate exists so aliases such as a legacy
/// junction or an 8.3 path are compared by their canonical targets as well as
/// by their user-facing spelling.
pub(crate) fn validate_no_canonical_root_overlap(
    candidate: &Path,
    current: &NodeDataRootState,
    install_id: &str,
) -> Result<()> {
    let candidate = std::fs::canonicalize(candidate)
        .with_context(|| format!("无法规范化候选节点数据根 {}", candidate.display()))?;
    let owned_previous_root = root_marker_belongs_to(&candidate, install_id);

    if let Some(existing) = current.configured_root() {
        if let Some(existing) = canonicalize_existing(existing)? {
            if paths_equal(&candidate, &existing) {
                return Ok(());
            }
            reject_canonical_overlap(&candidate, &existing)?;
        }
    }
    for existing in [
        current.legacy_workspace_root.as_deref(),
        current.legacy_storage_root.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(existing) = canonicalize_existing(existing)? {
            if owned_previous_root && is_exact_managed_child(&candidate, &existing) {
                continue;
            }
            reject_canonical_overlap(&candidate, &existing)?;
        }
    }
    Ok(())
}

pub(crate) fn apply_to_process(paths: &NodeDataPaths) {
    std::env::set_var(NODE_DATA_ROOT_ENV, paths.root());
}

fn clean_root(value: &str) -> Result<PathBuf> {
    let value = value.trim();
    if value.is_empty() {
        bail!("节点数据根不能为空");
    }
    if value.chars().any(|ch| matches!(ch, '\r' | '\n' | '\0')) {
        bail!("节点数据根包含非法控制字符");
    }
    let path = normalize_absolute(Path::new(value))?;
    if !path.is_absolute() {
        bail!("节点数据根必须是绝对路径: {}", path.display());
    }
    Ok(path)
}

fn normalize_absolute(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("节点数据根必须是绝对路径: {}", path.display());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    bail!("节点数据根不能越过文件系统根: {}", path.display());
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn ensure_not_filesystem_root(path: &Path) -> Result<()> {
    let has_normal_component = path
        .components()
        .any(|component| matches!(component, std::path::Component::Normal(_)));
    if path.parent().is_none() || !has_normal_component {
        bail!("节点数据根不能直接使用磁盘根目录: {}", path.display());
    }
    Ok(())
}

fn reject_existing_reparse_ancestors(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if !current.is_absolute() {
            continue;
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata_is_reparse_point(&metadata) {
                    bail!(
                        "节点数据路径包含符号链接、junction 或重解析点: {}",
                        current.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法检查节点数据路径 {}", current.display()));
            }
        }
    }
    Ok(())
}

fn validate_created_root(root: &Path) -> Result<PathBuf> {
    ensure_not_filesystem_root(root)?;
    reject_existing_reparse_ancestors(root)?;
    let metadata = std::fs::symlink_metadata(root)
        .with_context(|| format!("无法检查节点数据根 {}", root.display()))?;
    if !metadata.is_dir() {
        bail!("节点数据根不是目录: {}", root.display());
    }
    reject_reparse_point(root)?;
    let canonical = std::fs::canonicalize(root)
        .with_context(|| format!("无法规范化节点数据根 {}", root.display()))?;
    ensure_not_filesystem_root(&canonical)?;
    Ok(canonical)
}

fn validate_canonical_managed_path(
    root: &Path,
    expected_canonical_root: &Path,
    target: &Path,
) -> Result<()> {
    ensure_managed_child(root, target)?;
    reject_existing_reparse_ancestors(target)?;

    let canonical_root = validate_created_root(root)?;
    if !paths_equal(&canonical_root, expected_canonical_root) {
        bail!("节点数据根在操作期间发生变化，拒绝继续: {}", root.display());
    }

    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查节点数据目录 {}", target.display()));
        }
    };
    if !metadata.is_dir() {
        bail!("节点数据目录不是目录: {}", target.display());
    }
    reject_reparse_point(target)?;
    let canonical_target = std::fs::canonicalize(target)
        .with_context(|| format!("无法规范化节点数据目录 {}", target.display()))?;
    if paths_equal(&canonical_target, &canonical_root)
        || !path_is_within(&canonical_target, &canonical_root)
    {
        bail!(
            "节点数据目录规范化后越出数据根: {} (root: {})",
            canonical_target.display(),
            canonical_root.display()
        );
    }
    Ok(())
}

fn reject_reparse_point(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("无法检查节点数据目录 {}", path.display()))?;
    if metadata_is_reparse_point(&metadata) {
        bail!(
            "节点数据目录不能是符号链接、junction 或重解析点: {}",
            path.display()
        );
    }
    Ok(())
}

fn metadata_is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    let mut rejected = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        rejected |= metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    rejected
}

fn is_owned_previous_managed_child(candidate: &Path, existing: &Path, install_id: &str) -> bool {
    is_exact_managed_child(candidate, existing) && root_marker_belongs_to(candidate, install_id)
}

fn is_exact_managed_child(root: &Path, candidate: &Path) -> bool {
    paths_equal(candidate, &root.join("workspaces"))
        || paths_equal(candidate, &root.join("storage"))
}

fn push_migration_item(
    items: &mut Vec<MigrationPlanItem>,
    kind: &'static str,
    source: Option<&Path>,
    target: Option<&Path>,
    strategy: &'static str,
) {
    let Some(source) = source else {
        return;
    };
    if target.is_some_and(|target| paths_equal(source, target)) {
        return;
    }
    let exists = source.exists();
    items.push(MigrationPlanItem {
        kind,
        source_path: path_text(source),
        target_path: target.map(path_text),
        exists,
        has_data: exists && directory_has_entries(source),
        read_only_compatibility: false,
        strategy,
    });
}

fn directory_has_entries(path: &Path) -> bool {
    std::fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

fn ensure_managed_child(root: &Path, target: &Path) -> Result<()> {
    if paths_equal(target, root) || !path_is_within(target, root) {
        bail!(
            "拒绝清理节点数据根之外的目录: {} (root: {})",
            target.display(),
            root.display()
        );
    }
    Ok(())
}

fn canonicalize_existing(path: &Path) -> Result<Option<PathBuf>> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => std::fs::canonicalize(path)
            .map(Some)
            .with_context(|| format!("无法规范化现有节点目录 {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("无法检查现有节点目录 {}", path.display()))
        }
    }
}

fn reject_canonical_overlap(candidate: &Path, existing: &Path) -> Result<()> {
    if paths_overlap(candidate, existing) {
        bail!(
            "新数据根规范化后不能与现有节点目录互相嵌套: {} <-> {}",
            candidate.display(),
            existing.display()
        );
    }
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = normalized_path_key(left);
    let right = normalized_path_key(right);
    left == right
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    path_is_within(left, right) || path_is_within(right, left)
}

fn path_is_within(candidate: &Path, root: &Path) -> bool {
    let candidate = normalized_path_key(candidate);
    let root = normalized_path_key(root);
    candidate == root
        || candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with(std::path::MAIN_SEPARATOR))
}

fn normalized_path_key(path: &Path) -> String {
    let normalized = normalize_absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let text = path_text(&normalized);
    if cfg!(windows) {
        text.to_ascii_lowercase()
    } else {
        text
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests;
