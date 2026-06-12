use anyhow::{anyhow, Context, Result};
use homecli_proto::NodeStorageProfile;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct StorageSettings {
    pub enabled: bool,
    pub root_path: Option<String>,
    pub git_base_url: Option<String>,
}

pub struct StorageRepoRequest {
    pub project_id: String,
    pub user_id: String,
    pub name: String,
    pub branch: Option<String>,
}

pub struct StorageRepoResult {
    pub storage_repo_path: String,
    pub storage_repo_url: Option<String>,
    pub branch: Option<String>,
    pub created: bool,
}

pub fn default_storage_root() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config"))
            })
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("elon-node-agent")
        .join("storage")
}

pub fn storage_profile(settings: &StorageSettings) -> NodeStorageProfile {
    if !settings.enabled {
        return NodeStorageProfile::default();
    }
    let root = storage_root(settings);
    NodeStorageProfile {
        enabled: true,
        root_path: Some(root.to_string_lossy().to_string()),
        git_base_url: settings.git_base_url.clone(),
        disk_free_bytes: disk_free_bytes(&root),
    }
}

pub fn prepare_project_storage_repo(
    settings: &StorageSettings,
    req: StorageRepoRequest,
) -> Result<StorageRepoResult> {
    if !settings.enabled {
        anyhow::bail!("该 PC 节点未启用硬盘服务");
    }
    if !git_available() {
        anyhow::bail!("该 PC 节点未安装 Git，不能作为项目硬盘节点");
    }

    let root = storage_root(settings);
    let project_part = safe_path_part(&req.project_id, "project", 96);
    let user_part = safe_path_part(&req.user_id, "user", 80);
    let repo_dir = root.join("git").join("projects").join(user_part);
    std::fs::create_dir_all(&repo_dir).with_context(|| {
        format!(
            "failed to create storage repo parent {}",
            repo_dir.display()
        )
    })?;
    let repo = repo_dir.join(format!("{project_part}.git"));
    let created = !repo.exists();
    if created {
        run_git(
            &repo_dir,
            &["init", "--bare", repo.to_string_lossy().as_ref()],
        )
        .with_context(|| format!("failed to initialize bare repo {}", repo.display()))?;
        let branch = clean_branch(req.branch.as_deref()).unwrap_or_else(|| "main".to_string());
        set_bare_head(&repo, &branch)?;
        write_description(&repo, &req.name);
    } else if !repo.join("HEAD").exists() {
        anyhow::bail!("存储仓库路径已存在但不是 Git 裸仓库: {}", repo.display());
    }

    let branch = clean_branch(req.branch.as_deref()).or_else(|| bare_head_branch(&repo));
    Ok(StorageRepoResult {
        storage_repo_path: repo.to_string_lossy().to_string(),
        storage_repo_url: storage_repo_url(settings.git_base_url.as_deref(), &req.project_id),
        branch,
        created,
    })
}

fn storage_root(settings: &StorageSettings) -> PathBuf {
    settings
        .root_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_storage_root)
}

fn storage_repo_url(base_url: Option<&str>, project_id: &str) -> Option<String> {
    let base = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .trim_end_matches('/');
    let project = safe_path_part(project_id, "project", 96);
    Some(format!("{base}/projects/{project}.git"))
}

fn set_bare_head(repo: &Path, branch: &str) -> Result<()> {
    run_git(
        repo,
        &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")],
    )
}

fn bare_head_branch(repo: &Path) -> Option<String> {
    let head = std::fs::read_to_string(repo.join("HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .map(ToOwned::to_owned)
}

fn write_description(repo: &Path, name: &str) {
    let description = name.trim();
    if description.is_empty() {
        return;
    }
    let _ = std::fs::write(repo.join("description"), description);
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn clean_branch(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn safe_path_part(value: &str, fallback: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('-');
        }
        if out.len() >= max_len {
            break;
        }
    }
    let trimmed = out.trim_matches(['-', '.', '_']);
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(windows)]
fn disk_free_bytes(path: &Path) -> Option<u64> {
    use std::path::{Component, Prefix};

    let drive = path.components().find_map(|component| match component {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some((letter as char).to_ascii_uppercase())
            }
            _ => None,
        },
        _ => None,
    })?;
    let script = format!("(Get-PSDrive -Name '{}').Free", drive);
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[cfg(not(windows))]
fn disk_free_bytes(path: &Path) -> Option<u64> {
    let target = if path.exists() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("/"))
    };
    let output = Command::new("df").args(["-Pk"]).arg(target).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let available_kb = line.split_whitespace().nth(3)?.parse::<u64>().ok()?;
    available_kb.checked_mul(1024)
}
