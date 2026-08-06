//! Safe persistence and evidence binding for the Git feature registry.

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    project_document_file_operation_model::normalize_document_path,
    project_document_files::{read_project_document_file, write_project_document_file},
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_native_context::{validate_evidence_current, ProjectContextEvidence},
    project_feature_registry::{
        normalize_registry, parse_registry, ProjectFeature, ProjectFeatureRegistry,
        ProjectFeatureStatus, FEATURE_REGISTRY_PATH,
    },
};

const MAX_EVIDENCE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub(crate) struct FeatureEvidenceInput {
    pub path: String,
    #[serde(default)]
    pub locator: String,
    #[serde(default)]
    pub evidence_kind: String,
}

pub(crate) struct LoadedFeatureRegistry {
    pub registry: ProjectFeatureRegistry,
    pub revision: Option<String>,
}

pub(crate) fn load_registry(workspace: &Path) -> Result<LoadedFeatureRegistry> {
    if !workspace.join(FEATURE_REGISTRY_PATH).is_file() {
        return Ok(LoadedFeatureRegistry {
            registry: ProjectFeatureRegistry::default(),
            revision: None,
        });
    }
    let file = read_project_document_file(workspace, FEATURE_REGISTRY_PATH)?;
    Ok(LoadedFeatureRegistry {
        registry: parse_registry(Some(&file.content))?,
        revision: Some(file.revision),
    })
}

pub(crate) fn save_registry(
    workspace: &Path,
    registry: ProjectFeatureRegistry,
    expected_revision: Option<&str>,
) -> Result<LoadedFeatureRegistry> {
    let _write_guard = acquire_registry_write_lock(workspace)?;
    let disk_revision = if workspace.join(FEATURE_REGISTRY_PATH).is_file() {
        Some(read_project_document_file(workspace, FEATURE_REGISTRY_PATH)?.revision)
    } else {
        None
    };
    match (disk_revision.as_deref(), expected_revision) {
        (None, None) => {}
        (Some(current), Some(expected)) if current == expected => {}
        _ => bail!("功能登记已被其他进程修改，请刷新后重试"),
    }
    let registry = normalize_registry(registry)?;
    let content = format!("{}\n", serde_json::to_string_pretty(&registry)?);
    let saved = write_project_document_file(
        workspace,
        FEATURE_REGISTRY_PATH,
        &content,
        expected_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    Ok(LoadedFeatureRegistry {
        registry,
        revision: Some(saved.revision),
    })
}

struct RegistryWriteGuard {
    _file: File,
    _path: PathBuf,
}

fn acquire_registry_write_lock(workspace: &Path) -> Result<RegistryWriteGuard> {
    let git_dir = resolve_git_dir(workspace)?;
    let path = git_dir.join("elon-project-features.lock");
    let file = open_exclusive(&path).with_context(|| {
        format!(
            "另一代理正在更新功能登记；当前操作失败关闭：{}",
            path.display()
        )
    })?;
    Ok(RegistryWriteGuard {
        _file: file,
        _path: path,
    })
}

fn resolve_git_dir(workspace: &Path) -> Result<PathBuf> {
    let marker = workspace.join(".git");
    if marker.is_dir() {
        return marker.canonicalize().context("无法解析 Git 元数据目录");
    }
    let pointer = fs::read_to_string(&marker).context("无法读取 Git worktree 指针")?;
    let value = pointer
        .trim()
        .strip_prefix("gitdir:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Git worktree 指针格式无效"))?;
    let path = PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    path.canonicalize()
        .context("无法解析 Git worktree 元数据目录")
}

#[cfg(windows)]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .open(path)
}

#[cfg(not(windows))]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    use std::os::fd::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(file)
}

#[cfg(not(windows))]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

pub(crate) fn verify_registry_revision(
    current: Option<&str>,
    expected: Option<&str>,
) -> Result<()> {
    match (current, expected.filter(|value| !value.trim().is_empty())) {
        (Some(current), Some(expected)) if current == expected => Ok(()),
        (None, None) => Ok(()),
        (Some(_), None) => {
            bail!("功能登记已存在，必须先 list/plan 并传 expected_registry_revision")
        }
        _ => bail!("功能登记已被其他会话修改，请刷新后重试"),
    }
}

pub(crate) fn bind_requirement(workspace: &Path, path: &str) -> Result<ProjectContextEvidence> {
    let file = read_project_document_file(workspace, path)?;
    Ok(ProjectContextEvidence {
        path: file.path.clone(),
        content_hash: file.revision,
        locator: String::new(),
        evidence_kind: "document".to_string(),
        git_identity: crate::project_document_native_context_git::capture(workspace, &file.path),
    })
}

pub(crate) fn bind_evidence(
    workspace: &Path,
    input: FeatureEvidenceInput,
) -> Result<ProjectContextEvidence> {
    let path = normalize_document_path(&input.path)?;
    let canonical_root = workspace.canonicalize().context("无法解析项目工作区")?;
    let canonical_path = workspace
        .join(&path)
        .canonicalize()
        .with_context(|| format!("实现证据不存在：{path}"))?;
    if !canonical_path.starts_with(&canonical_root) {
        bail!("实现证据越过项目工作区：{path}");
    }
    let metadata = fs::metadata(&canonical_path)?;
    if !metadata.is_file() || metadata.len() > MAX_EVIDENCE_BYTES {
        bail!("实现证据不是普通文件或超过 8 MiB：{path}");
    }
    let mut file = File::open(&canonical_path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(ProjectContextEvidence {
        path: path.clone(),
        content_hash: format!("{:x}", digest.finalize()),
        locator: input.locator,
        evidence_kind: if input.evidence_kind.trim().is_empty() {
            "source".to_string()
        } else {
            input.evidence_kind
        },
        git_identity: crate::project_document_native_context_git::capture(workspace, &path),
    })
}

pub(crate) fn ensure_requirement_is_current(workspace: &Path, path: &str) -> Result<()> {
    let lower = path.to_ascii_lowercase();
    if lower.split('/').any(|segment| {
        matches!(
            segment,
            "draft"
                | "drafts"
                | "inbox"
                | "history"
                | "archive"
                | "archives"
                | "discussion"
                | "discussions"
        )
    }) {
        bail!("accepted/ready 功能不能引用草稿、收件箱、历史、归档或讨论目录：{path}");
    }
    let content = read_project_document_file(workspace, path)?
        .content
        .to_ascii_lowercase();
    for status in ["draft", "deprecated", "superseded", "archived"] {
        if content
            .lines()
            .take(40)
            .any(|line| line.trim() == format!("version_status: {status}"))
        {
            bail!("accepted/ready 功能 requirement 不是 current：{path}");
        }
    }
    Ok(())
}

pub(crate) fn ensure_requirement_current(workspace: &Path, feature: &ProjectFeature) -> Result<()> {
    validate_evidence_current(workspace, &feature.requirement).context("需求文档已经漂移")?;
    ensure_requirement_is_current(workspace, &feature.requirement.path)
}

pub(crate) fn ensure_implementation_evidence_current(
    workspace: &Path,
    feature: &ProjectFeature,
    target: ProjectFeatureStatus,
) -> Result<()> {
    if feature.implementation_evidence.is_empty() {
        bail!("进入 {} 前必须记录实现证据", target.as_str());
    }
    for evidence in &feature.implementation_evidence {
        validate_evidence_current(workspace, evidence)?;
    }
    if matches!(
        target,
        ProjectFeatureStatus::Verified | ProjectFeatureStatus::Released
    ) && !feature
        .implementation_evidence
        .iter()
        .any(|evidence| evidence.evidence_kind == "test")
    {
        bail!("进入 verified/released 前至少需要一条 test 证据");
    }
    Ok(())
}

pub(crate) fn validate_knowledge_node(workspace: &Path, node_id: &str) -> Result<()> {
    if node_id.trim().is_empty() {
        return Ok(());
    }
    let path = workspace.join(SECTION_CONFIG_PATH);
    let content =
        fs::read_to_string(path).context("指定 knowledge_node_id 时必须存在文档知识图谱")?;
    let manifest = parse_manifest(Some(&content))?;
    if !manifest
        .knowledge_graph
        .nodes
        .iter()
        .any(|node| node.id.eq_ignore_ascii_case(node_id.trim()))
    {
        bail!("knowledge_node_id 不存在：{}", node_id.trim());
    }
    Ok(())
}
