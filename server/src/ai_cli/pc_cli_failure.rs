use super::{
    extract_lightweight_pc_chat_reply,
    pc_passthrough_reply::{extract_codex_reply, pc_lightweight_no_readable_diagnostic},
    PC_PROJECT_NO_CHANGES_ERROR,
};

pub(super) struct PcCliReadableOutput {
    pub(super) codex_final_reply: String,
    pub(super) has_success_output: bool,
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
    let has_success_output = if lightweight_pc_chat {
        !extract_lightweight_pc_chat_reply(full_text, is_codex)
            .trim()
            .is_empty()
    } else if is_codex {
        !codex_final_reply.trim().is_empty()
    } else {
        stream_started || !full_text.trim().is_empty()
    };
    PcCliReadableOutput {
        codex_final_reply,
        has_success_output,
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
                !e.contains("断线")
                    && !e.contains("超时")
                    && !e.contains("worktree")
                    && !e.contains("合并")
            })
            .unwrap_or(true)
}

fn pc_codex_failure_requires_error(error: Option<&str>, output: &str) -> bool {
    let combined = format!("{} {}", error.unwrap_or_default(), output);
    let classified = crate::errors::classify_ai_error(&combined);
    matches!(
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
    use super::{pc_cli_terminal_error_message, pc_codex_failure_requires_error};
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
}
