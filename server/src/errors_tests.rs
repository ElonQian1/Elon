    use super::*;

    #[test]
    fn classifies_codex_websocket_failure_as_retryable_capacity() {
        let classified =
            classify_ai_error("Codex CLI network unhealthy: Responses WebSocket failed");
        assert_eq!(classified.code, "ai_service_busy");
        assert!(classified.retryable);
        assert!(classified.should_retry_local_cli());
        assert!(classified.message.contains("本轮没有完成"));
        assert!(classified.message.contains("不会继续在后台处理"));
        assert!(!classified.message.contains("自动重试"));
    }

    #[test]
    fn classifies_provider_reachability_as_retryable_capacity() {
        let classified = classify_ai_error(
            "reachability one or more required provider endpoints are unreachable over HTTP",
        );
        assert_eq!(classified.code, "ai_service_busy");
        assert_eq!(classified.category, AiErrorCategory::TemporaryCapacity);
        assert!(classified.message.contains("本轮没有完成"));
        assert!(!classified.message.contains("自动重试"));
    }

    #[test]
    fn classifies_auth_error_as_non_retryable() {
        let classified = classify_ai_error("invalid api key");
        assert_eq!(classified.code, "ai_auth_config_error");
        assert!(!classified.retryable);
    }

    #[test]
    fn classifies_codex_usage_limit_as_quota() {
        let classified = classify_ai_error("Codex failed: usage limit reached for this account");
        assert_eq!(classified.code, "codex_usage_limit_exhausted");
        assert_eq!(classified.category, AiErrorCategory::Quota);
        assert!(!classified.retryable);
    }

    #[test]
    fn classifies_codex_refresh_token_reuse_as_auth_json_failure() {
        let classified = classify_ai_error(
            "Failed to refresh token: 401 Unauthorized refresh_token_reused Your refresh token has already been used to generate a new access token.",
        );

        assert_eq!(classified.code, "codex_auth_json_invalid");
        assert_eq!(classified.category, AiErrorCategory::AuthConfig);
        assert!(classified.message.contains("auth.json"));
        assert!(classified.message.contains("重新登录 Codex"));
        assert!(!classified.retryable);
    }

    #[test]
    fn classifies_dirty_conversation_worktree_with_user_facing_message() {
        let classified = classify_ai_error(
            "PC CLI 执行失败: conversation worktree still has uncommitted changes: /tmp/worktree",
        );

        assert_eq!(classified.code, "project_workspace_error");
        assert!(classified.message.contains("未提交改动"));
        assert!(!classified.message.contains("/tmp/worktree"));
    }

    #[test]
    fn classifies_no_project_changes_as_failed_workspace_result() {
        let classified = classify_ai_error(
            "开发助手已经结束，但项目工作区没有产生新提交；本轮需求没有实际修改项目。",
        );

        assert_eq!(classified.code, "project_workspace_error");
        assert!(classified.message.contains("没有实际修改项目"));
        assert!(!classified.retryable);
    }
