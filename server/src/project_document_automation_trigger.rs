//! Durable commit-triggered handoff for project document organization.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const TRIGGER_VERSION: u8 = 1;
const MAX_PATHS: usize = 100;
const MAX_REASONS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationTrigger {
    pub version: u8,
    pub trigger_id: String,
    pub operation_id: String,
    pub commit_sha: String,
    pub severity: String,
    pub paths: Vec<String>,
    pub reasons: Vec<String>,
    pub created_at: u64,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<u64>,
}

pub(crate) fn enqueue(
    workspace: &Path,
    commit_sha: &str,
    severity: &str,
    paths: &[String],
    reasons: &[String],
) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    let commit_sha = normalize_commit_sha(commit_sha)?;
    let severity = normalize_severity(severity)?;
    let paths = normalize_paths(paths)?;
    let reasons = normalize_reasons(reasons);
    with_trigger_lock(|| {
        if let Some(existing) = read_trigger_unlocked(&workspace)? {
            if existing.commit_sha == commit_sha {
                return serde_json::to_value(existing).context("序列化自动文档整理触发器失败");
            }
        }
        let digest = trigger_digest(&workspace, &commit_sha);
        let trigger = DocumentOrganizationTrigger {
            version: TRIGGER_VERSION,
            trigger_id: format!("doc-trigger-{}", &digest[..32]),
            operation_id: format!("docs_auto_{}", &digest[..32]),
            commit_sha,
            severity,
            paths,
            reasons,
            created_at: unix_seconds(),
            status: "pending".to_string(),
            claimed_at: None,
        };
        write_trigger_unlocked(&workspace, &trigger)?;
        serde_json::to_value(trigger).context("序列化自动文档整理触发器失败")
    })
}

pub(crate) fn get_pending(workspace: &Path) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    with_trigger_lock(|| {
        let trigger =
            read_trigger_unlocked(&workspace)?.filter(|trigger| trigger.status == "pending");
        Ok(serde_json::json!({ "trigger": trigger }))
    })
}

pub(crate) fn claim(workspace: &Path, trigger_id: &str, operation_id: &str) -> Result<Value> {
    let workspace = validate_workspace(workspace)?;
    with_trigger_lock(|| {
        let mut trigger = read_trigger_unlocked(&workspace)?
            .ok_or_else(|| anyhow::anyhow!("当前项目没有自动文档整理触发器"))?;
        if trigger.trigger_id != trigger_id.trim() || trigger.operation_id != operation_id.trim() {
            bail!("自动文档整理触发器已被更新的提交替代");
        }
        if trigger.status == "pending" {
            trigger.status = "claimed".to_string();
            trigger.claimed_at = Some(unix_seconds());
            write_trigger_unlocked(&workspace, &trigger)?;
        }
        serde_json::to_value(trigger).context("序列化自动文档整理触发器失败")
    })
}

fn validate_workspace(workspace: &Path) -> Result<PathBuf> {
    let root = workspace
        .canonicalize()
        .context("project_root 不存在或不可访问")?;
    if !root.is_dir() || !root.join(".git").exists() {
        bail!("project_root 必须是现存 Git 工作区");
    }
    Ok(root)
}

fn normalize_commit_sha(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !(40..=64).contains(&value.len()) || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("commit_sha 格式无效");
    }
    Ok(value)
}

fn normalize_severity(value: &str) -> Result<String> {
    match value.trim() {
        "warning" => Ok("warning".to_string()),
        "blocking" => Ok("blocking".to_string()),
        _ => bail!("severity 必须是 warning 或 blocking"),
    }
}

fn normalize_paths(values: &[String]) -> Result<Vec<String>> {
    if values.is_empty() || values.len() > MAX_PATHS {
        bail!("paths 必须包含 1 至 {MAX_PATHS} 个 Markdown 路径");
    }
    let mut paths = Vec::new();
    for value in values {
        let normalized = value.trim().replace('\\', "/");
        let path = Path::new(&normalized);
        let valid_extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(extension.to_ascii_lowercase().as_str(), "md" | "mdx")
            });
        if normalized.is_empty()
            || path.is_absolute()
            || !valid_extension
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            bail!("paths 只能包含工作区内的 Markdown 相对路径");
        }
        if !paths.contains(&normalized) {
            paths.push(normalized);
        }
    }
    Ok(paths)
}

fn normalize_reasons(values: &[String]) -> Vec<String> {
    values
        .iter()
        .take(MAX_REASONS)
        .map(|value| value.trim().chars().take(500).collect::<String>())
        .filter(|value| !value.is_empty())
        .collect()
}

fn trigger_digest(workspace: &Path, commit_sha: &str) -> String {
    let mut key = workspace.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key = key.to_ascii_lowercase();
    }
    format!(
        "{:x}",
        Sha256::digest(format!("{key}\n{commit_sha}").as_bytes())
    )
}

fn trigger_path(workspace: &Path) -> PathBuf {
    let mut key = workspace.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key = key.to_ascii_lowercase();
    }
    let hash = format!("{:x}", Sha256::digest(key.as_bytes()));
    std::env::temp_dir()
        .join("elon-project-docs-automation")
        .join(format!("{hash}.json"))
}

fn read_trigger_unlocked(workspace: &Path) -> Result<Option<DocumentOrganizationTrigger>> {
    let path = trigger_path(workspace);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path)
        .with_context(|| format!("读取自动文档整理触发器失败：{}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .with_context(|| format!("自动文档整理触发器损坏：{}", path.display()))
}

fn write_trigger_unlocked(workspace: &Path, trigger: &DocumentOrganizationTrigger) -> Result<()> {
    let path = trigger_path(workspace);
    let bytes = serde_json::to_vec_pretty(trigger)?;
    crate::node_agent_atomic_file::write(&path, &bytes)
        .with_context(|| format!("写入自动文档整理触发器失败：{}", path.display()))
}

fn with_trigger_lock<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "project_document_automation_trigger_tests.rs"]
mod tests;
