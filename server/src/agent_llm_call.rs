//! LLM API 调用与工具执行（从 agent.rs 抽出）。
//!
//! 这里只关注：
//! - OpenAI 兼容 /chat/completions 接口的两种调用形态（带 tools / 普通对话）
//! - LLM 错误信息的中文化
//! - 工具名 → tools::* 的派发
//!
//! 让 agent.rs 只保留路由、Agent 选择和高层编排。

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    agent_prompts::tool_definitions,
    tools,
    types::{AgentConfig, AppState},
};
/// 调用 LLM API（OpenAI 兼容接口）
pub(crate) async fn call_llm(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "tools": tool_definitions(),
        "tool_choice": "auto",
    });

    // GitHub Copilot 直连 API 需要额外的 editor 标识 header
    let is_copilot_direct = agent.api_base.contains("githubcopilot.com");
    let integration_id =
        std::env::var("COPILOT_INTEGRATION_ID").unwrap_or_else(|_| "vscode-chat".into());

    let mut req = state
        .http_client
        .post(&url)
        .bearer_auth(&agent.api_key)
        .json(&body);
    if is_copilot_direct {
        req = req
            .header("editor-version", "vscode/1.99.0")
            .header("editor-plugin-version", "copilot-chat/0.26.0")
            .header("Copilot-Integration-Id", integration_id);
    }
    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
        } else {
            anyhow::anyhow!("AI 请求失败: {}", e)
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    Ok(resp.json::<Value>().await?)
}

pub(crate) async fn call_chat_llm(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.8,
        "max_tokens": 700,
    });

    let is_copilot_direct = agent.api_base.contains("githubcopilot.com");
    let integration_id =
        std::env::var("COPILOT_INTEGRATION_ID").unwrap_or_else(|_| "vscode-chat".into());

    let mut req = state
        .http_client
        .post(&url)
        .bearer_auth(&agent.api_key)
        .json(&body);
    if is_copilot_direct {
        req = req
            .header("editor-version", "vscode/1.99.0")
            .header("editor-plugin-version", "copilot-chat/0.26.0")
            .header("Copilot-Integration-Id", integration_id);
    }
    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
        } else {
            anyhow::anyhow!("AI 请求失败: {}", e)
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    Ok(resp.json::<Value>().await?)
}

pub(crate) fn friendly_ai_api_error(status: reqwest::StatusCode, body: &str) -> String {
    let lower = body.to_lowercase();
    if status.as_u16() == 402
        || lower.contains("free_quota_exhausted")
        || lower.contains("payment required")
        || lower.contains("endpoint is inactive")
    {
        return "当前 AI 模型额度已用尽或接口不可用，请切换可用模型，或联系管理员补充额度后重试"
            .into();
    }
    if status.as_u16() == 401 || lower.contains("unauthorized") || lower.contains("invalid api key")
    {
        return "当前 AI 模型密钥无效或权限不足，请检查 AI 设置或切换可用模型".into();
    }
    if status.as_u16() == 429 || lower.contains("rate limit") || lower.contains("too many requests")
    {
        return "当前 AI 模型请求过于频繁，请稍后重试或切换可用模型".into();
    }
    if status.as_u16() >= 500 {
        return "AI 服务暂时不可用，请稍后重试".into();
    }

    let compact = body.lines().collect::<Vec<_>>().join(" ");
    let visible = compact.chars().take(120).collect::<String>();
    if visible.trim().is_empty() {
        format!("AI 服务返回错误 {}", status)
    } else {
        format!("AI 服务返回错误 {}：{}", status, visible)
    }
}

/// 根据工具名和参数，调用对应的工具函数
pub(crate) fn execute_tool(
    _state: &Arc<AppState>,
    workspace: &std::path::Path,
    tool_name: &str,
    args: &Value,
) -> Result<String> {
    match tool_name {
        "init_project" => {
            let project_type = args["project_type"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 project_type 参数"))?;
            tools::init_project(workspace, project_type)
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            tools::read_file(workspace, path)
        }
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;
            tools::write_file(workspace, path, content)
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            tools::list_dir(workspace, path)
        }
        "run_shell" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 command 参数"))?;
            tools::run_shell(workspace, command)
        }
        "git_commit" => {
            let message = args["message"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 message 参数"))?;
            tools::git_commit(workspace, message)
        }
        "build_project" => {
            let target = args["target"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 target 参数"))?;
            tools::build_project(workspace, target)
        }
        _ => Err(anyhow::anyhow!("未知工具: {}", tool_name)),
    }
}
