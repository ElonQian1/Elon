use std::sync::Arc;

use serde_json::json;

use crate::{agent_llm_call::call_chat_llm, types::AppState};

use super::config::ContextCompilerConfig;

pub(crate) async fn build_llm_brief(
    state: &Arc<AppState>,
    config: &ContextCompilerConfig,
    user_id: &str,
    user_message: &str,
    deterministic_pack: &str,
) -> Option<String> {
    if !config.llm_brief_enabled {
        return None;
    }
    let agent = {
        let agents = state.agents_config.read().await;
        agents.get_agent(Some(&config.agent_name)).cloned()
    }?;

    let messages = vec![
        json!({
            "role": "system",
            "content": "你是只读 Context Compiler。根据确定性工具产出的事实，生成给代码执行 Agent 使用的短中文 brief。不要编造文件、符号或测试；只基于输入内容。"
        }),
        json!({
            "role": "user",
            "content": format!(
                "用户请求：\n{}\n\n确定性上下文包：\n{}\n\n请输出：\n1. 任务理解\n2. 最可能相关的文件/模块\n3. 风险和不变量\n4. 建议验证命令\n控制在 900 字以内。",
                user_message.trim(),
                deterministic_pack
            )
        }),
    ];

    let response = call_chat_llm(state, &agent, &messages, user_id, "context_compiler")
        .await
        .ok()?;
    extract_message_content(&response)
}

fn extract_message_content(response: &serde_json::Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_chat_completion_content() {
        let value = json!({
            "choices": [
                {"message": {"content": " brief "}}
            ]
        });

        assert_eq!(extract_message_content(&value).as_deref(), Some("brief"));
    }
}
