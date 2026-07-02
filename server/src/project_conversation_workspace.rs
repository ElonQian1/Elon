use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    project_attachment_paths::safe_project_path_part, project_git::git_output,
    store::ProjectAccess, tools, types::AppState,
};

#[derive(Debug, Clone)]
pub(crate) struct ProjectConversationWorkspace {
    pub(crate) base_workspace: PathBuf,
    pub(crate) active_workspace: PathBuf,
    pub(crate) branch: Option<String>,
}

impl ProjectConversationWorkspace {
    pub(crate) fn shared(base_workspace: PathBuf) -> Self {
        Self {
            active_workspace: base_workspace.clone(),
            base_workspace,
            branch: None,
        }
    }

    pub(crate) fn isolated(
        base_workspace: PathBuf,
        active_workspace: PathBuf,
        branch: String,
    ) -> Self {
        Self {
            base_workspace,
            active_workspace,
            branch: Some(branch),
        }
    }

    pub(crate) fn is_isolated(&self) -> bool {
        self.branch.is_some()
    }

    pub(crate) fn active_path(&self) -> &Path {
        &self.active_workspace
    }
}

pub(crate) fn prepare_project_conversation_workspace(
    state: &AppState,
    project: &ProjectAccess,
    conversation_id: &str,
) -> anyhow::Result<ProjectConversationWorkspace> {
    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    if !is_git_work_tree(&base_workspace) {
        return Ok(ProjectConversationWorkspace::shared(base_workspace));
    }

    let project_part = safe_project_path_part(&project.id, 64);
    let conversation_part = safe_project_path_part(conversation_id, 80);
    let worktree_root = PathBuf::from(&state.workspace_root)
        .join("conversation-worktrees")
        .join(&project_part);
    let worktree_path = worktree_root.join(&conversation_part);
    std::fs::create_dir_all(&worktree_root)?;

    let branch = conversation_branch_name(&project_part, &conversation_part);
    let _ = git_fetch_origin(&base_workspace);
    if is_git_work_tree(&worktree_path) {
        return Ok(ProjectConversationWorkspace::isolated(
            base_workspace,
            worktree_path,
            branch,
        ));
    }
    if worktree_path.exists() {
        // 路径存在但不是有效的 git 工作树——上次 worktree remove 后可能留有空目录，直接清掉重建
        if std::fs::read_dir(&worktree_path)?.next().is_some() {
            anyhow::bail!(
                "会话 worktree 路径已存在但不是 Git 工作树: {}",
                worktree_path.display()
            );
        }
        let _ = std::fs::remove_dir(&worktree_path);
    }

    let start_ref = conversation_start_ref(&base_workspace);
    let mut args = vec!["worktree".to_string(), "add".to_string()];
    if !local_branch_exists(&base_workspace, &branch) {
        args.push("-b".into());
        args.push(branch.clone());
    }
    args.push(worktree_path.to_string_lossy().to_string());
    args.push(if local_branch_exists(&base_workspace, &branch) {
        branch.clone()
    } else {
        start_ref
    });
    git_output_owned(&base_workspace, &args)?;

    Ok(ProjectConversationWorkspace::isolated(
        base_workspace,
        worktree_path,
        branch,
    ))
}

pub(crate) fn merge_conversation_worktree(
    workspace: &ProjectConversationWorkspace,
) -> anyhow::Result<String> {
    let Some(branch) = workspace.branch.as_deref() else {
        return Ok("共享工作区任务无需额外合并。".into());
    };
    if !worktree_clean(&workspace.active_workspace)? {
        anyhow::bail!("会话 worktree 仍有未提交或未 add 的改动，请先提交后再合并");
    }
    if !tracked_worktree_clean(&workspace.base_workspace)? {
        anyhow::bail!("项目主工作区仍有未提交的已跟踪改动，暂不能自动合并");
    }

    let base_branch = current_branch(&workspace.base_workspace).unwrap_or_else(|| "main".into());
    if has_origin_remote(&workspace.base_workspace) {
        fast_forward_current_branch_from_origin(&workspace.base_workspace, &base_branch)?;
    }

    let before = git_output(&workspace.base_workspace, &["rev-parse", "HEAD"])?;
    let merge_output = Command::new("git")
        .args(["merge", "--no-ff", "--no-edit", branch])
        .current_dir(&workspace.base_workspace)
        .output()?;
    if !merge_output.status.success() {
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(&workspace.base_workspace)
            .output();
        anyhow::bail!("{}", String::from_utf8_lossy(&merge_output.stderr).trim());
    }
    copy_latest_apk_artifact(&workspace.active_workspace, &workspace.base_workspace)?;
    if has_origin_remote(&workspace.base_workspace) {
        git_output_owned(
            &workspace.base_workspace,
            &["push".into(), "origin".into(), base_branch.clone()],
        )?;
    }
    if tracked_worktree_clean(&workspace.active_workspace)? {
        let _ = git_output_owned(
            &workspace.active_workspace,
            &["reset".into(), "--hard".into(), base_branch],
        );
    }
    let after = git_output(&workspace.base_workspace, &["rev-parse", "HEAD"])?;
    // 合并完成后异步清理：
    // 1. git worktree remove —— 释放会话工作目录（含所有临时文件）
    // 2. git branch -d       —— 删除 ai/session/... 分支（已合并，可安全删除）
    // 3. 删除 gradle-home    —— 防止磁盘持续增长（wrapper/dists 是共享符号链接，不会丢失发行版缓存）
    let base_for_cleanup = workspace.base_workspace.clone();
    let active_for_cleanup = workspace.active_workspace.clone();
    let branch_for_cleanup = branch.to_string();
    std::thread::spawn(move || {
        // 先 remove worktree（git 会取消注册并删除目录）
        let _ = Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                &active_for_cleanup.to_string_lossy(),
            ])
            .current_dir(&base_for_cleanup)
            .output();
        // 删除已合并的会话分支
        let _ = Command::new("git")
            .args(["branch", "-d", &branch_for_cleanup])
            .current_dir(&base_for_cleanup)
            .output();
        // 删除 gradle-home
        let workspace_key = active_for_cleanup
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !workspace_key.is_empty() {
            let gradle_home =
                std::path::PathBuf::from("/opt/elon/gradle-homes").join(workspace_key);
            if gradle_home.exists() {
                let _ = std::fs::remove_dir_all(&gradle_home);
            }
        }
    });
    if before == after {
        Ok("会话分支没有新的提交需要合并。".into())
    } else {
        Ok(format!(
            "会话分支已合并回项目主工作区：{}",
            short_sha(&after)
        ))
    }
}

fn copy_latest_apk_artifact(
    source_workspace: &Path,
    target_workspace: &Path,
) -> anyhow::Result<()> {
    let Some(apk_path) = tools::find_latest_apk(source_workspace) else {
        return Ok(());
    };
    let Ok(relative_path) = apk_path.strip_prefix(source_workspace) else {
        return Ok(());
    };
    let target_path = target_workspace.join(relative_path);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&apk_path, &target_path)?;
    Ok(())
}

pub(crate) fn project_conversation_execution_key(
    project_id: &str,
    conversation_id: &str,
) -> String {
    format!("conversation:{}:{}", project_id, conversation_id)
}

pub(crate) fn project_shared_execution_key(project_id: &str) -> String {
    format!("shared:{}", project_id)
}

pub(crate) fn project_merge_execution_key(project_id: &str) -> String {
    format!("merge:{}", project_id)
}

fn conversation_branch_name(project_part: &str, conversation_part: &str) -> String {
    format!("ai/session/{}/{}", project_part, conversation_part)
}

fn conversation_start_ref(workspace: &Path) -> String {
    let branch = current_branch(workspace).unwrap_or_else(|| "main".into());
    let origin_ref = format!("origin/{}", branch);
    if git_output(workspace, &["rev-parse", "--verify", &origin_ref]).is_ok() {
        origin_ref
    } else {
        branch
    }
}

fn is_git_work_tree(workspace: &Path) -> bool {
    workspace.exists()
        && git_output(workspace, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
}

fn current_branch(workspace: &Path) -> Option<String> {
    git_output(workspace, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

fn local_branch_exists(workspace: &Path, branch: &str) -> bool {
    git_output(
        workspace,
        &["rev-parse", "--verify", &format!("refs/heads/{}", branch)],
    )
    .is_ok()
}

fn has_origin_remote(workspace: &Path) -> bool {
    git_output(workspace, &["remote", "get-url", "origin"]).is_ok()
}

fn git_fetch_origin(workspace: &Path) -> anyhow::Result<String> {
    if has_origin_remote(workspace) {
        git_output(workspace, &["fetch", "origin"])
    } else {
        Ok(String::new())
    }
}

fn fast_forward_current_branch_from_origin(workspace: &Path, branch: &str) -> anyhow::Result<()> {
    git_fetch_origin(workspace)?;
    let origin_ref = format!("origin/{branch}");
    if git_output(workspace, &["rev-parse", "--verify", &origin_ref]).is_err() {
        return Ok(());
    }
    git_output_owned(workspace, &["merge".into(), "--ff-only".into(), origin_ref]).map(|_| ())
}

fn tracked_worktree_clean(workspace: &Path) -> anyhow::Result<bool> {
    Ok(git_output(
        workspace,
        &["status", "--porcelain", "--untracked-files=no"],
    )?
    .trim()
    .is_empty())
}

fn worktree_clean(workspace: &Path) -> anyhow::Result<bool> {
    Ok(git_output(workspace, &["status", "--porcelain"])?
        .trim()
        .is_empty())
}

fn git_output_owned(workspace: &Path, args: &[String]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}
