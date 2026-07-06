//! CLI 提示执行辅助函数（从 node_agent_main.rs 拆分）。
//! 保持行为不变。

use std::path::PathBuf;

use anyhow::Result;
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
    let _ = task_journal.record_cli_chunk(req_id, stream, text);
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text: text.to_string(),
    }));
}

pub fn send_cli_chunk_message(
    out_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    req_id: &str,
    text: &str,
) {
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text: text.to_string(),
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
}

pub fn prepare_cli_prompt_cwd(
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    let (base_cwd, context) = node_agent_cli_security::prepare_cli_base_cwd(cwd, project_context)?;
    if cli_prompt_read_only(context.runtime_permission.as_deref()) {
        return Ok(PreparedCliPromptCwd {
            cwd: Some(base_cwd.to_string_lossy().to_string()),
            conversation_workspace: None,
        });
    }
    let workspace = pc_workspace_provisioner::prepare_conversation_workspace(
        base_cwd.to_string_lossy().as_ref(),
        &context.project_id,
        &context.conversation_id,
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
            (
                false,
                Some(message.clone()),
                Some(cli_workspace_status(&workspace, "failed", Some(&message))),
            )
        }
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
