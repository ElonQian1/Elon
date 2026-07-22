//! CLI 提示执行辅助函数（从 node_agent_main.rs 拆分）。
//! 旧/外部项目继承已跑通的工作区与缓存，新托管项目使用推荐数据根。

use std::path::PathBuf;

use homecli_proto::{AgentToServer, CliWorkspaceStatus};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use super::{cli_prompt_read_only, ws_text};
use super::{node_agent_cli_security, node_agent_task_journal, pc_workspace_provisioner};

pub fn send_cli_chunk(
    out_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    task_journal: &node_agent_task_journal::TaskJournal,
    req_id: &str,
    stream: &str,
    text: &str,
) {
    let text = super::node_agent_cli_redaction::redact_text(text);
    let _ = task_journal.record_cli_chunk(req_id, stream, &text);
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text,
    }));
}

pub fn send_cli_chunk_message(
    out_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    req_id: &str,
    text: &str,
) {
    let text = super::node_agent_cli_redaction::redact_text(text);
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text,
    }));
}

pub fn codex_last_message_chunk(path: Option<&PathBuf>) -> Option<String> {
    let path = path?;
    let text = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    let reply = text.trim();
    if reply.is_empty() {
        None
    } else {
        Some(format!("codex\n{reply}\n"))
    }
}

pub fn contains_codex_reply_marker(output: &str) -> bool {
    strip_cli_control_sequences(output)
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("codex"))
}

pub fn strip_cli_control_sequences(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\u{1b}' {
            i += 1;
            if i < chars.len() && chars[i] == '[' {
                i += 1;
                while i < chars.len() {
                    let next = chars[i];
                    i += 1;
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            if i < chars.len() && chars[i] == ']' {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\u{7}' {
                        i += 1;
                        break;
                    }
                    if chars[i] == '\u{1b}' && i + 1 < chars.len() && chars[i + 1] == '\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            continue;
        }
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

pub fn push_tracked_arg(
    cmd: &mut tokio::process::Command,
    sidecar_args: &mut Vec<String>,
    arg: impl AsRef<str>,
) {
    let arg = arg.as_ref().to_string();
    cmd.arg(&arg);
    sidecar_args.push(arg);
}

pub fn record_cli_done_outcome(
    task_journal: &node_agent_task_journal::TaskJournal,
    req_id: &str,
    exit_ok: bool,
    error: Option<&str>,
) {
    crate::node_agent_build_runtime::mark_cli_run_outcome(req_id, exit_ok);
    let status = if exit_ok {
        "done"
    } else if cli_done_error_is_canceled(error) {
        "canceled"
    } else {
        "failed"
    };
    if let Err(journal_error) = task_journal.record_finished_with_outcome(req_id, status, error) {
        warn!("PC 任务 journal 写入终态失败: {journal_error}");
    }
}

fn cli_done_error_is_canceled(error: Option<&str>) -> bool {
    let Some(error) = error.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let lower = error.to_ascii_lowercase();
    lower.contains("cancel")
        || lower.contains("cancelled")
        || lower.contains("canceled")
        || lower.contains("stopped")
        || error.contains("取消")
        || error.contains("停止")
        || error.contains("终止")
}

pub struct PreparedCliPromptCwd {
    pub cwd: Option<String>,
    pub conversation_workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
    pub project_context: Option<homecli_proto::CliProjectContext>,
    pub data_policy: crate::node_agent_project_data_policy::ProjectDataPolicy,
}

pub fn prepare_cli_prompt_cwd(
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    let data_paths = elon_pc_dev_runtime::configured_node_data_root()
        .map(elon_pc_dev_runtime::NodeDataPaths::new);
    prepare_cli_prompt_cwd_in(data_paths.as_ref(), cwd, project_context)
}

/// Existing/external projects keep the pre-upgrade conversation-worktree root
/// and inherit the environment that already worked. Only projects created
/// below the validated node roots opt into managed worktrees and caches.
pub fn prepare_cli_prompt_cwd_in(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    prepare_cli_prompt_cwd_in_with_supervision(data_paths, cwd, project_context, None)
}

pub fn prepare_cli_prompt_cwd_in_with_supervision(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
    supervision_root_task_id: Option<&str>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    let managed_project = project_context.is_some();
    let (base_cwd, context) = node_agent_cli_security::prepare_cli_base_cwd(cwd, project_context)?;
    let data_policy = crate::node_agent_project_data_policy::classify(data_paths, &base_cwd);
    if cli_prompt_read_only(context.runtime_permission.as_deref()) {
        return Ok(PreparedCliPromptCwd {
            cwd: Some(base_cwd.to_string_lossy().to_string()),
            conversation_workspace: None,
            project_context: managed_project.then_some(context),
            data_policy,
        });
    }
    let workspace_root = if data_policy.uses_managed_workspace() {
        data_paths
            .map(elon_pc_dev_runtime::NodeDataPaths::workspaces)
            .ok_or_else(|| anyhow::anyhow!("一龙推荐工作区暂不可用，请继续使用原项目目录"))?
    } else {
        elon_pc_dev_runtime::legacy_workspace_root_override()
            .unwrap_or_else(elon_pc_dev_runtime::legacy_default_workspace_root)
    };
    let workspace = pc_workspace_provisioner::prepare_conversation_workspace_in_with_supervision(
        &workspace_root,
        base_cwd.to_string_lossy().as_ref(),
        &context.project_id,
        &context.conversation_id,
        supervision_root_task_id,
    )?;
    if workspace.isolated {
        info!(
            "🧩 项目会话使用隔离 worktree: project={} conversation={} path={}",
            context.project_id, context.conversation_id, workspace.workspace_path
        );
    }
    Ok(PreparedCliPromptCwd {
        cwd: Some(workspace.workspace_path.clone()),
        conversation_workspace: Some(workspace),
        project_context: managed_project.then_some(context),
        data_policy,
    })
}

/// Reuse the exact isolated worktree recorded for a terminated supervised
/// parent task. Admission validates the parent/worktree identity before this
/// function is called; this layer keeps workspace finalization attached to the
/// inherited worktree instead of provisioning a new conversation directory.
pub fn prepare_inherited_cli_prompt_cwd_in(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    workspace: pc_workspace_provisioner::ConversationWorkspaceResult,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    if !workspace.isolated || workspace.base_workspace_path.is_none() {
        anyhow::bail!("续跑工作区必须是带基础仓库记录的隔离 worktree");
    }
    let (active_cwd, context) = node_agent_cli_security::prepare_cli_base_cwd(
        Some(workspace.workspace_path.clone()),
        project_context,
    )?;
    if cli_prompt_read_only(context.runtime_permission.as_deref()) {
        anyhow::bail!("只读任务不能继承可写的监督 worktree");
    }
    let base = workspace
        .base_workspace_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("续跑工作区缺少基础仓库"))?;
    let base = std::fs::canonicalize(&base)
        .map_err(|error| anyhow::anyhow!("续跑基础仓库不可用: {} ({error})", base.display()))?;
    let data_policy = crate::node_agent_project_data_policy::classify(data_paths, &base);
    Ok(PreparedCliPromptCwd {
        cwd: Some(active_cwd.to_string_lossy().to_string()),
        conversation_workspace: Some(workspace),
        project_context: Some(context),
        data_policy,
    })
}

pub fn finalize_cli_prompt_workspace(
    exit_ok: bool,
    error: Option<String>,
    workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
) -> (bool, Option<String>, Option<CliWorkspaceStatus>) {
    let Some(workspace) = workspace else {
        return (exit_ok, error, None);
    };
    if !exit_ok {
        return (
            exit_ok,
            error.clone(),
            Some(cli_workspace_status(
                &workspace,
                "skipped",
                error.as_deref(),
            )),
        );
    }
    match pc_workspace_provisioner::merge_conversation_workspace(&workspace) {
        Ok(message)
            if message.starts_with("conversation worktree still")
                || message.starts_with("conversation worktree missing git metadata")
                || message.starts_with("base workspace") =>
        {
            warn!("会话 worktree 暂未合并: {message}");
            if conversation_workspace_head_landed(&workspace) {
                let cleanup_status = cleanup_failure_status(&message);
                return (
                    true,
                    None,
                    Some(cli_workspace_status(
                        &workspace,
                        cleanup_status,
                        Some(&message),
                    )),
                );
            }
            (
                false,
                Some(message.clone()),
                Some(cli_workspace_status(&workspace, "blocked", Some(&message))),
            )
        }
        Ok(message) => {
            info!("会话 worktree 合并结果: {message}");
            let merge_status = if workspace.isolated {
                "merged"
            } else {
                "shared"
            };
            (
                true,
                None,
                Some(cli_workspace_status(
                    &workspace,
                    merge_status,
                    Some(&message),
                )),
            )
        }
        Err(e) => {
            warn!("会话 worktree 合并失败: {e:#}");
            let message = format!("会话 worktree 合并失败: {e}");
            if conversation_workspace_head_landed(&workspace) {
                let cleanup_status = cleanup_failure_status(&message);
                return (
                    true,
                    None,
                    Some(cli_workspace_status(
                        &workspace,
                        cleanup_status,
                        Some(&message),
                    )),
                );
            }
            (
                false,
                Some(message.clone()),
                Some(cli_workspace_status(&workspace, "failed", Some(&message))),
            )
        }
    }
}

fn conversation_workspace_head_landed(
    workspace: &pc_workspace_provisioner::ConversationWorkspaceResult,
) -> bool {
    pc_workspace_provisioner::conversation_workspace_head_landed(workspace).unwrap_or_else(
        |probe_error| {
            warn!("检查会话 worktree 是否已进入远端主线失败: {probe_error:#}");
            false
        },
    )
}

pub fn cleanup_failure_status(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("non-fast-forward")
        || lower.contains("ff-only")
        || lower.contains("fast-forward")
        || lower.contains("diverg")
        || lower.contains("fetch first")
    {
        "local_main_diverged"
    } else {
        "cleanup_failed"
    }
}

pub fn cli_workspace_status(
    workspace: &pc_workspace_provisioner::ConversationWorkspaceResult,
    merge_status: &str,
    merge_message: Option<&str>,
) -> CliWorkspaceStatus {
    CliWorkspaceStatus {
        base_workspace_path: workspace.base_workspace_path.clone(),
        active_workspace_path: workspace.workspace_path.clone(),
        isolated: workspace.isolated,
        branch: workspace.branch.clone(),
        git_head: pc_workspace_provisioner::conversation_workspace_git_head(workspace),
        prepare_status: "prepared".into(),
        merge_status: Some(merge_status.into()),
        merge_message: merge_message.map(ToOwned::to_owned),
    }
}

pub fn cli_model_from_args(cli_name: &str, args: &[String]) -> Option<String> {
    if cli_name.eq_ignore_ascii_case("codex") {
        return args.iter().find_map(|arg| {
            arg.strip_prefix("--codex-model=")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    }

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--model=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
        if arg == "--model" {
            if let Some(value) = iter.next().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                return Some(value.to_string());
            }
        }
    }
    None
}
