//! 项目级 agent 后端选择（本地 CLI vs API LLM）+ 闲聊兜底回复（从 agent.rs 抽出）。
//!
//! `choose_backend` 决定本次任务用本地 Codex CLI 还是远程 API LLM；周边几个
//! `is_*` / `*_agent_name` / `*_option_id` 辅助负责解析用户传入的 agent 别名。
//! `casual_chat_prompt` / `quick_casual_reply` 给非开发的闲聊路由提供 prompt 和短答。

use std::sync::Arc;

use crate::{
    intent_router::CapabilityRoute,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    types::{AiBackend, AppState, UserAgentConfig},
    user_agent_secrets::user_byok_api_enabled,
};

use chrono::{Duration, Utc};

pub(crate) async fn has_api_agents(state: &Arc<AppState>) -> bool {
    !state.agents_config.read().await.agents.is_empty()
}

pub(crate) fn choose_backend(
    state: &Arc<AppState>,
    user_config: Option<&UserAgentConfig>,
    agent_name: Option<&str>,
    _route: CapabilityRoute,
) -> AiBackend {
    // 锁定 Codex CLI 时，只有用户显式保存的 BYOK API 配置可以作为例外。
    if state.ai_cli.codex_cli_only {
        if user_byok_api_enabled()
            && user_config
                .map(|cfg| cfg.has_direct_custom_api())
                .unwrap_or(false)
        {
            return AiBackend::Api;
        }
        return AiBackend::LocalCli;
    }

    // ── 调用方或用户显式指定了 agent → 优先遵从，无视路由 ──
    if let Some(name) = agent_name {
        if is_local_cli_option(state, name) {
            return AiBackend::LocalCli;
        }
        // 其他名字（API agent name 或 "api"/"remote" 别名）→ Api
        return AiBackend::Api;
    }

    // ── 用户有保存的偏好 → 遵从 ──
    if let Some(cfg) = user_config {
        if cfg.has_config() {
            if cfg
                .use_agent
                .as_deref()
                .map(|name| is_local_cli_option(state, name))
                .unwrap_or(false)
            {
                return AiBackend::LocalCli;
            }
            return AiBackend::Api;
        }
    }

    // ── 无显式偏好：CLI 可用时项目会话全部走 CLI ──
    // 本机 PC agent 连线优先（run_with_workspace 内部处理），其次服务器 Copilot CLI。
    // CLI 不可用时才退回 API。
    if state.ai_cli.enabled {
        return AiBackend::LocalCli;
    }

    AiBackend::Api
}

pub(crate) fn api_agent_name<'a>(
    state: &Arc<AppState>,
    agent_name: Option<&'a str>,
) -> Option<&'a str> {
    agent_name.filter(|name| !is_local_cli_option(state, name) && !is_api_backend_alias(name))
}

pub(crate) fn resolve_cli_option_id(
    state: &Arc<AppState>,
    agent_name: Option<&str>,
) -> Option<String> {
    let name = agent_name.map(str::trim).filter(|name| !name.is_empty())?;

    if is_local_default_alias(name) {
        return None;
    }

    if let Some(opt) = state
        .ai_cli
        .options
        .iter()
        .find(|opt| opt.id.eq_ignore_ascii_case(name))
    {
        return Some(opt.id.clone());
    }

    if let Some(cli) = named_cli_alias(name) {
        return state
            .ai_cli
            .options
            .iter()
            .find(|opt| {
                opt.provider.eq_ignore_ascii_case(cli)
                    || opt.id.to_ascii_lowercase().contains(cli)
                    || opt.bin.to_ascii_lowercase().contains(cli)
            })
            .map(|opt| opt.id.clone());
    }

    None
}

pub(crate) fn is_local_cli_option(state: &Arc<AppState>, name: &str) -> bool {
    is_cli_alias(name) || state.ai_cli.has_option(name)
}

pub(crate) fn requested_agent_for_runtime_route<'a>(
    agent_name: Option<&'a str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<&'a str> {
    if agent_name
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
    {
        return agent_name;
    }
    match pc_runtime_route {
        Some(
            PcRuntimeRoutePreference::RouteB
            | PcRuntimeRoutePreference::RouteC
            | PcRuntimeRoutePreference::RouteC2,
        ) => Some("api"),
        _ => agent_name,
    }
}

fn is_cli_alias(name: &str) -> bool {
    is_local_default_alias(name) || named_cli_alias(name).is_some()
}

fn is_local_default_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "cli" | "local" | "local_cli"
    )
}

fn named_cli_alias(name: &str) -> Option<&'static str> {
    match name.trim().to_ascii_lowercase().as_str() {
        "codex" | "codex_cli" => Some("codex"),
        "copilot" | "copilot_cli" => Some("copilot"),
        "claude" | "claude_cli" => Some("claude"),
        "gemini" | "gemini_cli" => Some("gemini"),
        _ => None,
    }
}

fn is_api_backend_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "api" | "llm" | "remote"
    )
}

pub(crate) fn casual_chat_prompt() -> &'static str {
    r#"你是「一龙开发助手」，也是用户身边一个有经验、有温度的产品与开发搭档。
用户可能只是闲聊、犹豫、没想好要做什么，或者想让你给灵感。

你的回复要自然、有生命力，不要像客服模板，也不要一直重复"这里只能开发 App"。
你可以正常聊天、共情、追问，也可以帮用户把模糊想法整理成 App 方向。

重要边界：
- 这一次是普通聊天模式，不能声称你已经修改代码、执行工具、打包 APK。
- 如果用户还没想好，主动给 2-4 个具体方向，让用户容易继续说下去。
- 如果用户明显想开始开发，引导他补充目标用户、核心功能、界面风格或优先级。
- 回复以中文为主，简洁但有内容。

注意：用户的部分消息来自手机语音识别，可能含有同音字替换或音近字错误。请优先推断最合理的语义，忽略明显的识别错误，直接给出正确理解下的回复，无需向用户解释纠错过程。"#
}

pub(crate) fn quick_casual_reply(user_message: &str) -> Option<String> {
    let normalized = user_message.trim().to_lowercase();
    if looks_like_current_time_question(&normalized) {
        return Some(current_beijing_time_reply());
    }

    match normalized.as_str() {
        "你好" | "你好？" | "你好?" | "你好呀" | "你好啊" | "你好在吗" | "你好，在吗"
        | "你好吗" | "你好吗？" | "你好吗?" | "在吗" | "你在吗" | "在不在" | "hi" | "hello" => {
            Some("你好，我在。你可以直接告诉我想改代码、查问题、构建 APK，或者先聊聊想法。".into())
        }
        "谢谢" | "谢谢你" | "辛苦了" => {
            Some("不客气，我在这边。你继续说下一步想怎么改就行。".into())
        }
        "真的能改吗" | "真的能改吗？" | "真的能改吗?" | "能改吗" | "能改代码吗"
        | "你能改代码吗" | "可以改代码吗" => Some(
            "能改。你把具体需求发过来，我会直接改代码、查问题、构建 APK 或部署；如果平台模型或节点通道不可用，我会明确告诉你卡在哪里。"
                .into(),
        ),
        _ => None,
    }
}

fn looks_like_current_time_question(message: &str) -> bool {
    let compact: String = message
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '!' | '！'
                        | '?'
                        | '？'
                        | '.'
                        | '。'
                        | ','
                        | '，'
                        | ';'
                        | '；'
                        | ':'
                        | '：'
                        | '~'
                        | '～'
                )
        })
        .collect();
    let chars = compact.chars().count();
    if chars == 0 || chars > 12 {
        return false;
    }

    matches!(
        compact.as_str(),
        "几点"
            | "几点了"
            | "几点几分"
            | "现在几点"
            | "现在几点了"
            | "现在几点几分"
            | "现在几分"
            | "现在时间"
            | "当前时间"
            | "当前几点"
            | "当前几点了"
            | "现在是什么时间"
            | "现在时间是多少"
    ) || ((compact.contains("现在") || compact.contains("当前"))
        && (compact.contains("几点") || compact.contains("几分") || compact.contains("时间")))
}

fn current_beijing_time_reply() -> String {
    let now = Utc::now() + Duration::hours(8);
    format!("现在是北京时间 {}。", now.format("%Y-%m-%d %H:%M"))
}

#[cfg(test)]
mod tests {
    use super::quick_casual_reply;

    #[test]
    fn quick_reply_handles_common_presence_greeting() {
        assert!(quick_casual_reply("你好在吗").is_some());
        assert!(quick_casual_reply("你好，在吗").is_some());
        assert!(quick_casual_reply("你好吗？").is_some());
        assert!(quick_casual_reply("你好？").is_some());
        assert!(quick_casual_reply("真的能改吗?").is_some());
    }

    #[test]
    fn quick_reply_handles_current_time_questions() {
        let reply = quick_casual_reply("现在几点几分").expect("time question should be quick");
        assert!(reply.starts_with("现在是北京时间 "));
        assert!(quick_casual_reply("现在几点").is_some());
        assert!(quick_casual_reply("当前时间？").is_some());
    }

    #[test]
    fn quick_reply_does_not_capture_time_feature_requests() {
        assert!(quick_casual_reply("帮我做一个显示当前时间的网页").is_none());
    }
}
