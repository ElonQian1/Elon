use super::{
    extract_lightweight_pc_chat_reply,
    pc_passthrough_reply::{
        codex_reply_is_complete, extract_codex_reply, pc_lightweight_no_readable_diagnostic,
    },
    PC_PROJECT_NO_CHANGES_ERROR,
};

pub(super) struct PcCliReadableOutput {
    pub(super) codex_final_reply: String,
    pub(super) has_success_output: bool,
    pub(super) incomplete_after_tools: bool,
}

impl PcCliReadableOutput {
    pub(super) fn completion_status(
        &self,
        exit_ok: bool,
        no_project_changes: bool,
        is_codex: bool,
        lightweight_pc_chat: bool,
        error: Option<&str>,
    ) -> (bool, Option<String>) {
        let missing_reply_error =
            self.missing_final_reply_error(exit_ok, is_codex, lightweight_pc_chat);
        let effective_error = if no_project_changes {
            Some(PC_PROJECT_NO_CHANGES_ERROR.to_string())
        } else {
            missing_reply_error
                .clone()
                .or_else(|| error.map(str::to_string))
        };
        (
            exit_ok && !no_project_changes && missing_reply_error.is_none(),
            effective_error,
        )
    }

    pub(super) fn missing_final_reply_error(
        &self,
        exit_ok: bool,
        is_codex: bool,
        lightweight_pc_chat: bool,
    ) -> Option<String> {
        if !exit_ok || !is_codex || lightweight_pc_chat || self.has_success_output {
            return None;
        }
        let detail = if self.incomplete_after_tools {
            "Codex 在最后一条公开说明之后仍执行了命令或工具，但没有返回收尾回复"
        } else {
            "Codex 已结束，但没有返回可展示的最终回复"
        };
        Some(format!(
            "PC CLI 执行未完成：{detail}；本轮结果无法确认完成。请点击“重试处理”继续。"
        ))
    }
}

pub(super) fn pc_cli_readable_output(
    is_codex: bool,
    lightweight_pc_chat: bool,
    stream_started: bool,
    full_text: &str,
) -> PcCliReadableOutput {
    let codex_final_reply = is_codex
        .then(|| extract_codex_reply(full_text))
        .unwrap_or_default();
    let codex_reply_complete = !is_codex || codex_reply_is_complete(full_text);
    let has_success_output = if lightweight_pc_chat {
        !extract_lightweight_pc_chat_reply(full_text, is_codex)
            .trim()
            .is_empty()
    } else if is_codex {
        !codex_final_reply.trim().is_empty() && codex_reply_complete
    } else {
        stream_started || !full_text.trim().is_empty()
    };
    let incomplete_after_tools =
        is_codex && !codex_final_reply.trim().is_empty() && !codex_reply_complete;
    PcCliReadableOutput {
        codex_final_reply,
        has_success_output,
        incomplete_after_tools,
    }
}

pub(super) fn pc_codex_error_output_can_complete(
    is_codex: bool,
    has_success_output: bool,
    no_project_changes: bool,
    error: Option<&str>,
    output: &str,
) -> bool {
    is_codex
        && has_success_output
        && !no_project_changes
        && !pc_codex_failure_requires_error(error, output)
        && error
            .map(|e| {
                !pc_cli_terminal_may_have_unreported_usage(Some(e))
                    && !e.contains("断线")
                    && !e.contains("超时")
                    && !e.contains("worktree")
                    && !e.contains("合并")
            })
            .unwrap_or(true)
}

/// These terminal errors are only emitted after the node has started a real
/// child process. Missing usage therefore means "unknown after execution", not
/// an authoritative zero. A shared/managed reservation must stay with durable
/// replay instead of being released immediately.
pub(super) fn pc_cli_terminal_may_have_unreported_usage(error: Option<&str>) -> bool {
    let error = error.unwrap_or_default().trim();
    if error.is_empty() {
        return false;
    }
    let lower = error.to_ascii_lowercase();
    error.contains("用户已停止 PC CLI 任务")
        || error.contains("执行超时")
        || lower.contains("cli task canceled after start")
        || lower.contains("cli task timed out after start")
}

fn pc_codex_failure_requires_error(error: Option<&str>, output: &str) -> bool {
    let combined = format!("{} {}", error.unwrap_or_default(), output);
    let lower = combined.to_ascii_lowercase();
    let terminal_auth_failure = [
        "refresh_token_invalidated",
        "refresh token was revoked",
        "session has ended. please log in again",
        "access token could not be refreshed",
        "authentication token has been invalidated",
        "token_invalidated",
    ]
    .iter()
    .any(|signature| lower.contains(signature));
    let classified = crate::errors::classify_ai_error(&combined);
    terminal_auth_failure
        || matches!(
            classified.category,
            crate::errors::AiErrorCategory::Quota | crate::errors::AiErrorCategory::AuthConfig
        )
}

pub(super) fn pc_cli_terminal_error_message(
    cli_name: &str,
    no_project_changes: bool,
    error: Option<&str>,
    output: &str,
) -> String {
    if no_project_changes {
        return PC_PROJECT_NO_CHANGES_ERROR.to_string();
    }

    let detail = error.map(str::trim).filter(|value| !value.is_empty());
    let diagnostic = pc_lightweight_no_readable_diagnostic(output, cli_name);
    let message = match (detail, diagnostic.as_deref()) {
        (Some(detail), Some(diagnostic)) if !detail.contains(diagnostic) => {
            format!("{detail}；{diagnostic}")
        }
        (Some(detail), _) => detail.to_string(),
        (None, Some(diagnostic)) => diagnostic.to_string(),
        (None, None) => "未返回具体错误".to_string(),
    };
    format!("PC CLI 执行失败: {message}")
}

#[cfg(test)]
mod tests {
    use super::{
        pc_cli_readable_output, pc_cli_terminal_error_message,
        pc_cli_terminal_may_have_unreported_usage, pc_codex_error_output_can_complete,
        pc_codex_failure_requires_error,
    };
    use crate::ai_cli::pc_passthrough_reply::pc_lightweight_no_readable_diagnostic;

    #[test]
    fn pc_codex_quota_failure_cannot_be_treated_as_successful_output() {
        let output = r#"{"type":"turn.failed","error":{"message":"You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage"}}"#;

        assert!(pc_codex_failure_requires_error(
            Some("You've hit your usage limit."),
            output
        ));
        let message = pc_cli_terminal_error_message(
            "codex",
            false,
            Some("You've hit your usage limit."),
            output,
        );

        assert!(message.contains("PC CLI 执行失败"));
        assert!(message.contains("usage limit"));
        assert!(message.contains("Codex达到使用额度"));
    }

    #[test]
    fn pc_codex_auth_json_failure_cannot_be_treated_as_successful_output() {
        let output = "Failed to refresh token: 401 Unauthorized\nrefresh_token_reused\nYour refresh token has already been used to generate a new access token.";

        assert!(pc_codex_failure_requires_error(None, output));
        let diagnostic = pc_lightweight_no_readable_diagnostic(output, "codex").unwrap();
        let message = pc_cli_terminal_error_message("codex", false, None, output);

        assert!(diagnostic.contains("auth.json 无法刷新"));
        assert!(message.contains("PC CLI 执行失败"));
        assert!(message.contains("auth.json 无法刷新"));
    }

    #[test]
    fn pc_codex_revoked_refresh_token_cannot_be_treated_as_successful_output() {
        let output = concat!(
            "Failed to refresh token: 401 Unauthorized\n",
            "Your session has ended. Please log in again.\n",
            "refresh_token_invalidated\n",
            r#"{"type":"turn.failed","error":{"message":"Your refresh token was revoked."}}"#,
        );

        assert!(pc_codex_failure_requires_error(
            Some("codex CLI exited with status 1"),
            output,
        ));
    }

    #[test]
    fn pc_codex_retry_notice_is_not_a_final_reply() {
        let output = concat!(
            "codex\n",
            "已发现本机 Codex session 失效，正在清理旧 session 并自动重新开始本轮任务。\n",
        );

        let readable = pc_cli_readable_output(true, false, true, output);
        assert!(!readable.has_success_output);
        assert!(readable.codex_final_reply.is_empty());
    }

    #[test]
    fn pc_codex_progress_before_tools_is_not_a_final_reply() {
        let output = concat!(
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"我先读取规则，然后继续。"}}"#,
            "\n",
            r#"{"type":"item.started","item":{"type":"command_execution","command":"git status"}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"command_execution","command":"git status","exit_code":0}}"#,
        );

        let readable = pc_cli_readable_output(true, false, true, output);
        assert!(!readable.has_success_output);
        assert!(readable.incomplete_after_tools);
    }

    #[test]
    fn pc_codex_reply_after_tools_is_a_final_reply() {
        let output = concat!(
            r#"{"type":"item.started","item":{"type":"command_execution","command":"git status"}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"command_execution","command":"git status","exit_code":0}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"检查完成，工作区干净。"}}"#,
        );

        let readable = pc_cli_readable_output(true, false, true, output);
        assert!(readable.has_success_output);
        assert!(!readable.incomplete_after_tools);
    }

    #[test]
    fn pc_codex_missing_final_reply_becomes_a_retryable_error() {
        let readable = pc_cli_readable_output(
            true,
            false,
            true,
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"我先读取规则。"}}
{"type":"item.completed","item":{"type":"command_execution","exit_code":0}}"#,
        );

        let message = readable
            .missing_final_reply_error(true, true, false)
            .unwrap();
        assert!(message.contains("没有返回收尾回复"));
        assert!(message.contains("重试处理"));
    }

    #[test]
    fn pc_codex_missing_final_reply_does_not_replace_cli_failures() {
        let readable = pc_cli_readable_output(true, false, true, "");

        let (exit_ok, error) =
            readable.completion_status(false, false, true, false, Some("502 Bad Gateway"));
        assert!(!exit_ok);
        assert_eq!(error.as_deref(), Some("502 Bad Gateway"));
    }

    #[test]
    fn canceled_or_timed_out_child_with_unknown_usage_is_not_zero_usage() {
        for error in [
            "用户已停止 PC CLI 任务",
            "codex 执行超时（超过300秒），已强制终止",
            "codex pipe sidecar 执行超时（超过 300 秒）",
        ] {
            assert!(pc_cli_terminal_may_have_unreported_usage(Some(error)));
            assert!(!pc_codex_error_output_can_complete(
                true,
                true,
                false,
                Some(error),
                "codex\n已有部分公开输出\n",
            ));
        }
        assert!(!pc_cli_terminal_may_have_unreported_usage(Some(
            "无法启动 codex: executable not found",
        )));
    }
}
