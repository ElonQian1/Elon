use anyhow::{anyhow, bail, Context, Result};
use elon_pc_dev_runtime::{
    configured_node_data_root, legacy_default_workspace_root, legacy_workspace_root_override,
    NodeDataPaths, NODE_DATA_ROOT_ENV,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub(crate) mod admin;

const ROOT_MARKER_FILE: &str = ".elon-node-data-root.json";

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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CleanupEntry {
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) existed: bool,
    pub(crate) estimated_bytes: u64,
    pub(crate) removed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CleanupResult {
    pub(crate) apply: bool,
    pub(crate) estimated_bytes: u64,
    pub(crate) entries: Vec<CleanupEntry>,
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
    let (configured, source) = if let Some(root) = env_root {
        (Some(root), NodeDataRootSource::Environment)
    } else if let Some(root) = persisted_root {
        (Some(root), NodeDataRootSource::Persisted)
    } else {
        (None, NodeDataRootSource::Unconfigured)
    };

    let (paths, invalid_reason) = match configured {
        Some(root) if root.is_absolute() => (Some(NodeDataPaths::new(root)), None),
        Some(root) => (
            None,
            Some(format!(
                "{} 必须是绝对路径，当前值为 {}",
                NODE_DATA_ROOT_ENV,
                root.display()
            )),
        ),
        None => (None, None),
    };

    NodeDataRootState {
        paths,
        source,
        invalid_reason,
        legacy_workspace_root,
        legacy_storage_root,
    }
}

impl NodeDataRootState {
    pub(crate) fn configured_root(&self) -> Option<&Path> {
        self.paths.as_ref().map(NodeDataPaths::root)
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
            "保留旧目录只读；确认无脏 worktree 后从 Git 基线在新根重建",
        );
        push_migration_item(
            &mut items,
            "storage",
            self.legacy_storage_root.as_deref(),
            storage_target.as_deref(),
            "保留旧目录只读；裸仓库需复制并执行 Git 完整性校验后再切换",
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
            "configuration_required": !configured,
            "source": self.source,
            "root_path": self.configured_root().map(path_text),
            "workspace_root": self.paths.as_ref().map(|paths| path_text(&paths.workspaces())),
            "storage_root": self.paths.as_ref().map(|paths| path_text(&paths.storage())),
            "cache_root": self.paths.as_ref().map(|paths| path_text(&paths.cache())),
            "temp_root": self.paths.as_ref().map(|paths| path_text(&paths.temp())),
            "invalid_reason": self.invalid_reason,
            "migration_required": plan.iter().any(|item| item.has_data),
            "migration_plan": plan,
            "legacy_policy": "preserve_read_only_no_automatic_git_move",
        })
    }
}

pub(crate) fn validate_and_prepare(root: &str, install_id: &str) -> Result<NodeDataPaths> {
    let root = clean_root(root)?;
    if root.parent().is_none() {
        bail!("节点数据根不能直接使用磁盘根目录: {}", root.display());
    }
    if root.exists() && !root.is_dir() {
        bail!("节点数据根已存在但不是目录: {}", root.display());
    }
    std::fs::create_dir_all(&root)
        .with_context(|| format!("无法创建节点数据根 {}", root.display()))?;
    reject_reparse_point(&root)?;

    let paths = NodeDataPaths::new(root);
    for managed in paths.managed_roots() {
        std::fs::create_dir_all(&managed)
            .with_context(|| format!("无法创建节点数据目录 {}", managed.display()))?;
        reject_reparse_point(&managed)?;
    }
    write_root_marker(&paths, install_id)?;
    Ok(paths)
}

pub(crate) fn validate_no_root_overlap(root: &str, current: &NodeDataRootState) -> Result<()> {
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
        if paths_equal(&candidate, existing)
            || candidate.starts_with(existing)
            || existing.starts_with(&candidate)
        {
            bail!(
                "新数据根不能与现有节点目录互相嵌套: {} <-> {}",
                candidate.display(),
                existing.display()
            );
        }
    }
    Ok(())
}

pub(crate) fn persist_to_env_file(paths: &NodeDataPaths) -> Result<()> {
    let path = crate::node_agent_env::node_agent_env_file_path()
        .ok_or_else(|| anyhow!("无法定位 _internal/node-agent.env"))?;
    crate::node_agent_api_runtime_config::upsert_env_file(
        &path,
        NODE_DATA_ROOT_ENV,
        &path_text(paths.root()),
    )
}

pub(crate) fn apply_to_process(paths: &NodeDataPaths) {
    std::env::set_var(NODE_DATA_ROOT_ENV, paths.root());
}

pub(crate) fn cleanup(paths: &NodeDataPaths, apply: bool) -> Result<CleanupResult> {
    let targets = [("cache", paths.cache()), ("temp", paths.temp())];
    let mut entries = Vec::with_capacity(targets.len());
    for (kind, target) in targets {
        ensure_managed_child(paths.root(), &target)?;
        let existed = target.exists();
        if existed && std::fs::symlink_metadata(&target)?.file_type().is_symlink() {
            bail!("拒绝清理符号链接目录: {}", target.display());
        }
        let estimated_bytes = directory_size_without_following_links(&target)?;
        if apply && existed {
            std::fs::remove_dir_all(&target)
                .with_context(|| format!("无法清理节点 {kind} 目录 {}", target.display()))?;
            std::fs::create_dir_all(&target)
                .with_context(|| format!("无法重建节点 {kind} 目录 {}", target.display()))?;
        }
        entries.push(CleanupEntry {
            kind,
            path: path_text(&target),
            existed,
            estimated_bytes,
            removed: apply && existed,
        });
    }
    Ok(CleanupResult {
        apply,
        estimated_bytes: entries.iter().map(|entry| entry.estimated_bytes).sum(),
        entries,
    })
}

fn clean_root(value: &str) -> Result<PathBuf> {
    let value = value.trim();
    if value.is_empty() {
        bail!("节点数据根不能为空");
    }
    if value.chars().any(|ch| matches!(ch, '\r' | '\n' | '\0')) {
        bail!("节点数据根包含非法控制字符");
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        bail!("节点数据根必须是绝对路径: {}", path.display());
    }
    Ok(path)
}

fn reject_reparse_point(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("无法检查节点数据目录 {}", path.display()))?;
    let mut rejected = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        rejected |= metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    if rejected {
        bail!(
            "节点数据目录不能是符号链接、junction 或重解析点: {}",
            path.display()
        );
    }
    Ok(())
}

fn write_root_marker(paths: &NodeDataPaths, install_id: &str) -> Result<()> {
    let marker = paths.root().join(ROOT_MARKER_FILE);
    if marker.exists() {
        let existing = std::fs::read_to_string(&marker)
            .with_context(|| format!("无法读取节点数据根标记 {}", marker.display()))?;
        let existing_install_id = serde_json::from_str::<serde_json::Value>(&existing)
            .ok()
            .and_then(|value| value.get("install_id")?.as_str().map(ToOwned::to_owned));
        if existing_install_id
            .as_deref()
            .is_some_and(|value| value != install_id)
        {
            bail!("该目录已属于另一台一龙节点: {}", marker.display());
        }
    }
    let temporary = paths.root().join(format!("{ROOT_MARKER_FILE}.tmp"));
    let content = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "install_id": install_id,
    }))?;
    std::fs::write(&temporary, content)
        .with_context(|| format!("无法写入节点数据根标记 {}", temporary.display()))?;
    if marker.exists() {
        std::fs::remove_file(&marker)
            .with_context(|| format!("无法更新节点数据根标记 {}", marker.display()))?;
    }
    std::fs::rename(&temporary, &marker)
        .with_context(|| format!("无法提交节点数据根标记 {}", marker.display()))
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
        read_only_compatibility: true,
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
    if target == root || !target.starts_with(root) {
        bail!(
            "拒绝清理节点数据根之外的目录: {} (root: {})",
            target.display(),
            root.display()
        );
    }
    Ok(())
}

fn directory_size_without_following_links(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total = total.saturating_add(directory_size_without_following_links(&entry.path())?);
    }
    Ok(total)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = path_text(left);
    let right = path_text(right);
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests;
