use serde::Serialize;

use crate::types::UserAgentConfig;

#[derive(Debug, Serialize)]
pub(crate) struct UserAgentRagReadiness {
    pub custom_configured: bool,
    pub api_base_set: bool,
    pub model_set: bool,
    pub api_key_set: bool,
    pub byok_api_enabled: bool,
    pub codex_cli_only: bool,
    pub development_ready: bool,
    pub tool_call_verified: bool,
    pub capability: Option<String>,
    pub capability_checked_at: Option<String>,
    pub capability_warning: Option<String>,
    pub status: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    pub required_capability: &'static str,
    pub tools: [&'static str; 3],
}

pub(crate) fn build_user_agent_rag_readiness(
    config: &UserAgentConfig,
    codex_cli_only: bool,
    byok_api_enabled: bool,
) -> UserAgentRagReadiness {
    let api_base_set = config
        .api_base
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty());
    let model_set = config
        .model
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty());
    let api_key_set = config.has_api_key_reference();
    let custom_configured = api_base_set && model_set && api_key_set;
    let blocked_by_policy = codex_cli_only && !byok_api_enabled;
    let tool_call_verified = matches!(config.tool_call_ok, Some(true));
    let development_ready = custom_configured && !blocked_by_policy && tool_call_verified;

    let (status, label, detail) = if blocked_by_policy {
        (
            "blocked_by_policy",
            "已锁定 Codex CLI",
            "当前服务器策略不允许用户自带 API Key；项目理解和开发仍走 Codex CLI。",
        )
    } else if development_ready {
        (
            "ready",
            "自定义模型可用于项目 RAG",
            "配置完整且最近一次能力探测已通过；AI 可调用 repo map、符号搜索和任务上下文工具。",
        )
    } else if custom_configured && matches!(config.tool_call_ok, Some(false)) {
        (
            "tool_call_failed",
            "模型未通过工具调用测试",
            "这个模型可以普通聊天，但不能作为项目开发/RAG 代理；请换用支持 OpenAI tools/function calling 的模型后重新保存。",
        )
    } else if custom_configured {
        (
            "needs_capability_check",
            "需要重新验证模型能力",
            "配置字段完整，但没有最近一次工具调用通过记录；请点击保存自定义模型，让服务器重新检测是否可用于项目 RAG。",
        )
    } else if !api_base_set && !model_set && !api_key_set {
        (
            "not_configured",
            "尚未配置自定义模型",
            "填写 API 地址、API Key 和模型名称后，测试并保存即可启用项目 RAG。",
        )
    } else if !api_key_set {
        (
            "missing_api_key",
            "缺少 API Key",
            "需要 API Key 才能连接用户自己的模型；留空只会保留已保存密钥。",
        )
    } else if !api_base_set {
        (
            "missing_api_base",
            "缺少 API 地址",
            "API 地址需要是 OpenAI 兼容的 /v1 base URL。",
        )
    } else {
        (
            "missing_model",
            "缺少模型名称",
            "需要填写支持 OpenAI tools/function calling 的模型名称。",
        )
    };

    UserAgentRagReadiness {
        custom_configured,
        api_base_set,
        model_set,
        api_key_set,
        byok_api_enabled,
        codex_cli_only,
        development_ready,
        tool_call_verified,
        capability: config.capability.clone(),
        capability_checked_at: config.capability_checked_at.clone(),
        capability_warning: config.capability_warning.clone(),
        status,
        label,
        detail,
        required_capability: "OpenAI tools/function calling",
        tools: [
            "repo_context_status",
            "repo_symbol_search",
            "repo_context_task_pack",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_when_custom_model_is_complete_and_byok_allowed() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            tool_call_ok: Some(true),
            capability: Some("tools_ok".into()),
            capability_checked_at: Some("2026-06-16 00:00:00 UTC".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, true, true);

        assert_eq!(readiness.status, "ready");
        assert!(readiness.development_ready);
        assert!(readiness.tool_call_verified);
        assert_eq!(
            readiness.required_capability,
            "OpenAI tools/function calling"
        );
    }

    #[test]
    fn blocked_when_codex_only_disallows_byok() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            tool_call_ok: Some(true),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, true, false);

        assert_eq!(readiness.status, "blocked_by_policy");
        assert!(!readiness.development_ready);
    }

    #[test]
    fn complete_config_without_probe_needs_capability_check() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "needs_capability_check");
        assert!(!readiness.development_ready);
        assert!(!readiness.tool_call_verified);
    }

    #[test]
    fn failed_probe_is_not_ready() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("chat-only".into()),
            tool_call_ok: Some(false),
            capability: Some("chat_only".into()),
            capability_warning: Some("no tool calls".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "tool_call_failed");
        assert!(!readiness.development_ready);
        assert_eq!(
            readiness.capability_warning.as_deref(),
            Some("no tool calls")
        );
    }

    #[test]
    fn explains_missing_api_key() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            model: Some("tool-model".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "missing_api_key");
        assert!(!readiness.development_ready);
    }
}
