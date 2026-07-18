// server/src/pc_storage_repo.rs

use anyhow::{anyhow, Context, Result};
use homecli_proto::NodeStorageProfile;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
    pub access_token: Option<String>,
    pub prepare_worktree: bool,
}

pub struct StorageRepoResult {
    pub storage_repo_path: String,
    pub storage_repo_url: Option<String>,
    pub storage_worktree_path: Option<String>,
    pub branch: Option<String>,
    pub created: bool,
}

pub fn default_storage_root() -> PathBuf {
    if let Some(root) = elon_pc_dev_runtime::configured_node_data_root() {
        return elon_pc_dev_runtime::NodeDataPaths::new(root).storage();
    }
    legacy_default_storage_root()
}

pub fn legacy_default_storage_root() -> PathBuf {
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
        relay_git_url_enabled: git_available(),
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
    ensure_http_receive_pack(&repo)?;
    write_access_token(&repo, req.access_token.as_deref())?;

    let branch = clean_branch(req.branch.as_deref()).or_else(|| bare_head_branch(&repo));
    let effective_branch = branch.clone().unwrap_or_else(|| "main".to_string());
    let storage_worktree_path = if req.prepare_worktree {
        Some(
            prepare_owner_worktree(&root, &repo, &req, &effective_branch)?
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };
    Ok(StorageRepoResult {
        storage_repo_path: repo.to_string_lossy().to_string(),
        storage_repo_url: storage_repo_url(
            settings.git_base_url.as_deref(),
            &req.user_id,
            &req.project_id,
        ),
        storage_worktree_path,
        branch,
        created,
    })
}

pub fn storage_root(settings: &StorageSettings) -> PathBuf {
    settings
        .root_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_storage_root)
}

pub fn git_project_root(settings: &StorageSettings) -> PathBuf {
    storage_root(settings).join("git")
}

fn storage_repo_url(base_url: Option<&str>, user_id: &str, project_id: &str) -> Option<String> {
    let base = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .trim_end_matches('/');
    let user = safe_path_part(user_id, "user", 80);
    let project = safe_path_part(project_id, "project", 96);
    Some(format!("{base}/projects/{user}/{project}.git"))
}

fn set_bare_head(repo: &Path, branch: &str) -> Result<()> {
    run_git(
        repo,
        &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")],
    )
}

fn ensure_http_receive_pack(repo: &Path) -> Result<()> {
    run_git(repo, &["config", "http.receivepack", "true"])
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

fn write_access_token(repo: &Path, token: Option<&str>) -> Result<()> {
    let Some(token) = clean_access_token(token) else {
        return Ok(());
    };
    run_git(repo, &["config", "elon.storageToken", &token])
        .with_context(|| format!("failed to write storage access token {}", repo.display()))
}

fn prepare_owner_worktree(
    root: &Path,
    bare_repo: &Path,
    req: &StorageRepoRequest,
    branch: &str,
) -> Result<PathBuf> {
    let project_part = safe_path_part(&req.project_id, "project", 96);
    let user_part = safe_path_part(&req.user_id, "user", 80);
    let worktree = root
        .join("worktrees")
        .join("users")
        .join(user_part)
        .join(project_part)
        .join("repo");
    let parent = worktree
        .parent()
        .ok_or_else(|| anyhow!("invalid storage worktree path: {}", worktree.display()))?;
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create owner worktree parent {}",
            parent.display()
        )
    })?;

    if !worktree.exists() || dir_is_empty(&worktree)? {
        clone_storage_repo(bare_repo, &worktree)?;
    } else if !is_git_work_tree(&worktree) {
        anyhow::bail!(
            "项目硬盘工作目录已存在但不是 Git 仓库: {}",
            worktree.display()
        );
    }

    let remote = bare_repo.to_string_lossy().to_string();
    ensure_remote_origin(&worktree, &remote)?;
    ensure_worktree_branch(&worktree, branch)?;
    if !git_has_head(&worktree) {
        ensure_owner_seed_files(&worktree, req)?;
        commit_and_push_seed(&worktree, branch)?;
    }
    Ok(worktree)
}

fn clone_storage_repo(bare_repo: &Path, worktree: &Path) -> Result<()> {
    let mut command = crate::git_command_error::git_command();
    command.arg("clone").arg(bare_repo).arg(worktree);
    let output = command_output(&mut command).context("failed to run git clone")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn ensure_remote_origin(worktree: &Path, remote: &str) -> Result<()> {
    match git_output(worktree, &["remote", "get-url", "origin"]) {
        Ok(existing) if existing.trim() == remote => Ok(()),
        Ok(_) => run_git(worktree, &["remote", "set-url", "origin", remote]),
        Err(_) => run_git(worktree, &["remote", "add", "origin", remote]),
    }
}

fn ensure_worktree_branch(worktree: &Path, branch: &str) -> Result<()> {
    if git_has_head(worktree) {
        if current_branch(worktree).as_deref() != Some(branch) {
            if local_branch_exists(worktree, branch) {
                run_git(worktree, &["checkout", branch])?;
            } else if remote_branch_exists(worktree, branch) {
                let remote_ref = format!("origin/{branch}");
                run_git(
                    worktree,
                    &["checkout", "-b", branch, "--track", &remote_ref],
                )?;
            } else {
                run_git(worktree, &["checkout", "-B", branch])?;
            }
        }
    } else {
        run_git(worktree, &["checkout", "-B", branch])?;
    }

    if remote_branch_exists(worktree, branch) {
        let remote_ref = format!("origin/{branch}");
        let _ = run_git(
            worktree,
            &["branch", "--set-upstream-to", &remote_ref, branch],
        );
        run_git(worktree, &["pull", "--ff-only", "origin", branch])?;
    }
    Ok(())
}

fn ensure_owner_seed_files(worktree: &Path, req: &StorageRepoRequest) -> Result<()> {
    let readme = worktree.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            format!(
                "# {}\n\nThis is the owner's local checkout for an Elon project.\n\n- project_id: {}\n- owner_user_id: {}\n- storage role: canonical project checkout on this PC\n\nThe bare Git repository beside this checkout is used to rebuild the project on other PC compute nodes.\n",
                req.name.trim(),
                req.project_id,
                req.user_id
            ),
        )?;
    }

    let agents = worktree.join("AGENTS.md");
    if !agents.exists() {
        std::fs::write(
            &agents,
            format!(
                "# Project Workspace\n\nThis project is managed by an Elon storage PC node.\n\nRules:\n- Keep source code and build outputs inside this repository.\n- Use git for every meaningful code change.\n- Remote compute nodes clone from the platform storage Git remote.\n\nProject metadata:\n- project_id: {}\n",
                req.project_id
            ),
        )?;
    }

    let gitignore = worktree.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(
            &gitignore,
            ".gradle/\nbuild/\napp/build/\n*.apk\n*.aab\nlocal.properties\n.env\n.env.local\n",
        )?;
    }
    Ok(())
}

fn commit_and_push_seed(worktree: &Path, branch: &str) -> Result<()> {
    let _ = run_git(worktree, &["config", "user.name", "Elon Storage Node"]);
    let _ = run_git(worktree, &["config", "user.email", "storage@elon.local"]);
    run_git(worktree, &["add", "."])?;
    run_git(
        worktree,
        &["commit", "-m", "chore: initialize storage project"],
    )?;
    run_git(worktree, &["push", "-u", "origin", branch])
}

pub fn validate_repo_access_token(repo: &Path, token: &str) -> bool {
    let Some(token) = clean_access_token(Some(token)) else {
        return false;
    };
    git_output(repo, &["config", "--get", "elon.storageToken"])
        .map(|stored| stored.trim() == token)
        .unwrap_or(false)
}

fn dir_is_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn is_git_work_tree(repo: &Path) -> bool {
    repo.exists()
        && git_output(repo, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
}

fn git_has_head(repo: &Path) -> bool {
    git_output(repo, &["rev-parse", "--verify", "HEAD"]).is_ok()
}

fn current_branch(repo: &Path) -> Option<String> {
    git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

fn local_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/remotes/origin/{branch}"),
        ],
    )
    .is_ok()
}

fn git_available() -> bool {
    let mut command = crate::git_command_error::git_command();
    command.arg("--version");
    command_output(&mut command)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let mut command = crate::git_command_error::git_command();
    command.args(args).current_dir(cwd);
    let output = command_output(&mut command).context("failed to run git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String> {
    let mut command = crate::git_command_error::git_command();
    command.args(args).current_dir(cwd);
    let output = command_output(&mut command).context("failed to run git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn clean_branch(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn clean_access_token(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| value.len() >= 32)
        .filter(|value| {
            value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        })
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
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Prefix};

    let drive_root = path.components().find_map(|component| match component {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some(format!("{}:\\", (letter as char).to_ascii_uppercase()))
            }
            _ => None,
        },
        _ => None,
    })?;
    let wide_path = OsStr::new(&drive_root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let mut free_available = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut free_available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free_available)
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
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

fn command_output(command: &mut Command) -> std::io::Result<Output> {
    hide_command_window(command);
    command.output()
}

fn hide_command_window(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        _command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

#[cfg(test)]
#[path = "pc_storage_repo_tests.rs"]
mod tests;
