//! 用户长期记忆异步提取。
//!
//! 每轮 AI 对话结束后，由 `agent_api_loop` 通过 `tokio::spawn` 异步调用。
//! 用轻量 LLM 提取本轮对话中值得记录的用户信息，写入 `user_memories` 表。
//! 所有错误只记录日志，不向调用方传播（spawn 中的任务不影响主链路）。

use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};

use crate::{
    agent_api_loop::resolve_agent, agent_llm_call::call_chat_llm, store::MEMORY_SCOPE_GLOBAL,
    types::AppState,
};

/// 从一轮对话中提取记忆并写入数据库。
///
/// - 用轻量 LLM（复用全局默认 agent）发一次 LLM 调用
/// - 要求返回 JSON 数组；无值得记忆的内容则返回 `[]`
/// - 解析结果后逐条 insert；静默忽略所有错误
pub async fn extract_and_save_memories(
    state: Arc<AppState>,
    user_id: String,
    user_message: String,
    assistant_reply: String,
) {
    extract_and_save_memories_scoped(
        state,
        user_id,
        user_message,
        assistant_reply,
        MEMORY_SCOPE_GLOBAL.to_string(),
        None,
        None,
    )
    .await;
}

/// 从一轮对话中提取记忆并写入指定作用域。
pub async fn extract_and_save_memories_scoped(
    state: Arc<AppState>,
    user_id: String,
    user_message: String,
    assistant_reply: String,
    scope_type: String,
    scope_id: Option<String>,
    source_conv_id: Option<String>,
) {
    // user_message 或 assistant_reply 过短时跳过（无有效信息）
    if user_message.trim().len() < 5 || assistant_reply.trim().len() < 5 {
        return;
    }

    let agent = match resolve_agent(&state, std::path::Path::new(""), None).await {
        Ok(a) => a,
        Err(e) => {
            debug!("记忆提取：无可用 agent，跳过 ({e})");
            return;
        }
    };

    let prompt = format!(
        r#"从以下对话中提取值得长期记住的用户信息。
只提取用户明确表达的偏好、背景、目标或事实，不要臆测或推断。
以 JSON 数组返回，每条格式：
{{"content": "简短描述（20字内）", "category": "preference|profile|goal|fact", "importance": 1-10}}
如果没有值得记忆的内容，返回空数组 []。
不要返回 JSON 以外的任何内容。

对话：
用户：{user_message}
助手：{assistant_reply}"#
    );

    let messages = vec![
        json!({
            "role": "system",
            "content": "你是一个记忆提取工具，只返回 JSON 数组，不解释。"
        }),
        json!({
            "role": "user",
            "content": prompt
        }),
    ];

    let response = match call_chat_llm(&state, &agent, &messages, &user_id, "memory_extract").await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("记忆提取 LLM 调用失败 user={user_id}: {e}");
            return;
        }
    };

    let raw = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if raw.is_empty() || raw == "[]" {
        return;
    }

    // 尝试解析 JSON 数组（可能被包裹在 ```json ... ``` 代码块里）
    let json_str = extract_json_array(&raw);
    let items: Vec<MemoryItem> = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            debug!("记忆提取：JSON 解析失败 user={user_id}: {e} | raw={raw}");
            return;
        }
    };

    for item in items {
        let content = item.content.trim().to_string();
        if content.is_empty() {
            continue;
        }
        let category = normalize_category(&item.category);
        let importance = item.importance.clamp(1, 10);
        if let Err(e) = state.store.insert_user_memory_scoped(
            &user_id,
            &content,
            &category,
            importance,
            source_conv_id.as_deref(),
            &scope_type,
            scope_id.as_deref(),
        ) {
            warn!("记忆写入失败 user={user_id}: {e}");
        }
    }
}

// ── 内部工具 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct MemoryItem {
    content: String,
    #[serde(default)]
    category: String,
    #[serde(default = "default_importance")]
    importance: i64,
}

fn default_importance() -> i64 {
    5
}

fn normalize_category(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "preference" | "偏好" => "preference".into(),
        "profile" | "背景" | "身份" => "profile".into(),
        "goal" | "目标" => "goal".into(),
        _ => "fact".into(),
    }
}

/// 从 LLM 响应中提取 JSON 数组字符串（兼容 ```json ... ``` 包裹形式）
fn extract_json_array(raw: &str) -> String {
    // 尝试去掉 ```json ``` 包裹
    if let Some(start) = raw.find('[') {
        if let Some(end) = raw.rfind(']') {
            if end >= start {
                return raw[start..=end].to_string();
            }
        }
    }
    raw.to_string()
}
