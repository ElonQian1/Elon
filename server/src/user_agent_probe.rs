//! Probe user-provided OpenAI-compatible API settings without saving secrets.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

use crate::{agent_llm_call::friendly_ai_api_error, types::UserAgentConfig};

#[derive(Debug, Deserialize)]
pub struct UserAgentProbeRequest {
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserAgentProbeConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct UserAgentProbeResult {
    pub api_base: String,
    pub model: String,
    pub latency_ms: u128,
    pub sample: String,
    pub tool_call_ok: bool,
    pub tool_call_name: Option<String>,
    pub capability: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolProbeOutcome {
    pub ok: bool,
    pub tool_call_name: Option<String>,
    pub warning: Option<String>,
}

pub(crate) fn normalize_api_base(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return None;
    }
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return None;
    }
    Some(trimmed)
}

pub(crate) fn resolve_probe_config(
    req: UserAgentProbeRequest,
    existing: &UserAgentConfig,
) -> Result<UserAgentProbeConfig> {
    let api_base = req
        .api_base
        .as_deref()
        .and_then(normalize_api_base)
        .or_else(|| existing.api_base.as_deref().and_then(normalize_api_base))
        .ok_or_else(|| anyhow!("请填写有效的 API 地址，例如 https://api.deepseek.com/v1"))?;

    let api_key = req
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| existing.api_key.clone())
        .ok_or_else(|| {
            if existing.api_key_encrypted.is_some() {
                anyhow!("已保存的 API Key 当前无法解密，请重新填写后再测试")
            } else {
                anyhow!("请填写 API Key；留空只会使用已保存的密钥")
            }
        })?;

    let model = req
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| existing.model.clone())
        .ok_or_else(|| anyhow!("请填写模型名称，例如 deepseek-chat"))?;

    Ok(UserAgentProbeConfig {
        api_base,
        api_key,
        model,
    })
}

pub(crate) async fn probe_openai_compatible_api(
    client: &Client,
    cfg: &UserAgentProbeConfig,
) -> Result<UserAgentProbeResult> {
    let url = format!("{}/chat/completions", cfg.api_base);
    let body = json!({
        "model": cfg.model,
        "messages": [
            {
                "role": "user",
                "content": "Reply with only OK."
            }
        ],
        "stream": false,
        "temperature": 0,
        "max_tokens": 8
    });

    let started = Instant::now();
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .timeout(Duration::from_secs(probe_timeout_secs()))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow!("测试超时，请检查 API 地址、网络或模型是否可用")
            } else {
                anyhow!("测试请求失败: {}", e)
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    let value: Value = resp
        .json()
        .await
        .map_err(|e| anyhow!("模型返回的 JSON 无法解析: {}", e))?;
    let sample = extract_sample(&value);
    let tool_probe = probe_tool_call_support(client, cfg, &url).await;
    let capability = if tool_probe.ok {
        "tools_ok"
    } else {
        "chat_only"
    };

    Ok(UserAgentProbeResult {
        api_base: cfg.api_base.clone(),
        model: cfg.model.clone(),
        latency_ms: started.elapsed().as_millis(),
        sample,
        tool_call_ok: tool_probe.ok,
        tool_call_name: tool_probe.tool_call_name,
        capability: capability.to_string(),
        warning: tool_probe.warning,
    })
}

pub(crate) async fn probe_development_agent_capability(
    client: &Client,
    cfg: &UserAgentProbeConfig,
) -> Result<UserAgentProbeResult> {
    let result = probe_openai_compatible_api(client, cfg).await?;
    if user_api_requires_tool_calls() && !result.tool_call_ok {
        return Err(anyhow!("{}", development_tool_call_error(&result)));
    }
    Ok(result)
}

async fn probe_tool_call_support(
    client: &Client,
    cfg: &UserAgentProbeConfig,
    url: &str,
) -> ToolProbeOutcome {
    let body = json!({
        "model": cfg.model,
        "messages": [
            {
                "role": "system",
                "content": "You are checking OpenAI-compatible tool call support. Call the provided function exactly."
            },
            {
                "role": "user",
                "content": "Call the elon_probe tool with ok=true. Do not answer in text."
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": tool_probe_name(),
                    "description": "Capability probe only. It does not execute any real action.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "ok": {
                                "type": "boolean",
                                "description": "Set to true when the tool call protocol is supported."
                            }
                        },
                        "required": ["ok"]
                    }
                }
            }
        ],
        "tool_choice": {
            "type": "function",
            "function": {
                "name": tool_probe_name()
            }
        },
        "stream": false,
        "temperature": 0,
        "max_tokens": 32
    });

    let resp = match client
        .post(url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .timeout(Duration::from_secs(tool_probe_timeout_secs()))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let warning = if e.is_timeout() {
                "普通聊天可用，但工具调用测试超时；作为 API Agent 时可能无法稳定读写文件或构建项目"
                    .to_string()
            } else {
                format!(
                    "普通聊天可用，但工具调用测试请求失败；作为 API Agent 时可能受限: {}",
                    e
                )
            };
            return ToolProbeOutcome::unsupported(None, warning);
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return ToolProbeOutcome::unsupported(
            None,
            format!(
                "普通聊天可用，但工具调用未通过；作为 API Agent 时可能无法读写文件或构建项目: {}",
                friendly_ai_api_error(status, &text)
            ),
        );
    }

    let value: Value = match resp.json().await {
        Ok(value) => value,
        Err(e) => {
            return ToolProbeOutcome::unsupported(
                None,
                format!(
                    "普通聊天可用，但工具调用返回的 JSON 无法解析；作为 API Agent 时可能受限: {}",
                    e
                ),
            )
        }
    };

    tool_probe_outcome_from_response(&value)
}

pub(crate) fn tool_probe_outcome_from_response(value: &Value) -> ToolProbeOutcome {
    let tool_call_name = extract_tool_call_name(value);
    match tool_call_name.as_deref() {
        Some(name) if name == tool_probe_name() => ToolProbeOutcome {
            ok: true,
            tool_call_name,
            warning: None,
        },
        Some(name) => ToolProbeOutcome::unsupported(
            tool_call_name.clone(),
            format!(
                "普通聊天可用，但工具调用名称为 {}，不是预期的 {}；作为 API Agent 时可能不兼容",
                name,
                tool_probe_name()
            ),
        ),
        None => ToolProbeOutcome::unsupported(
            None,
            "普通聊天可用，但模型没有返回工具调用；作为 API Agent 时可能无法读写文件或构建项目"
                .into(),
        ),
    }
}

fn extract_sample(value: &Value) -> String {
    let text = value["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| value["choices"][0]["text"].as_str())
        .unwrap_or("");
    text.trim().chars().take(80).collect()
}

fn extract_tool_call_name(value: &Value) -> Option<String> {
    value["choices"][0]["message"]["tool_calls"]
        .as_array()
        .and_then(|calls| {
            calls
                .iter()
                .find_map(|call| call["function"]["name"].as_str())
        })
        .or_else(|| value["choices"][0]["message"]["function_call"]["name"].as_str())
        .map(ToOwned::to_owned)
}

fn probe_timeout_secs() -> u64 {
    std::env::var("AI_USER_API_PROBE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(20)
}

fn tool_probe_timeout_secs() -> u64 {
    std::env::var("AI_USER_API_TOOL_PROBE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| probe_timeout_secs().clamp(3, 12))
}

fn tool_probe_name() -> &'static str {
    "elon_probe"
}

fn user_api_requires_tool_calls() -> bool {
    std::env::var("AI_USER_API_REQUIRE_TOOL_CALLS")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(true)
}

fn development_tool_call_error(result: &UserAgentProbeResult) -> String {
    format!(
        "自定义模型可以普通聊天，但未通过工具调用能力测试，不能作为项目开发代理保存。请换用支持 OpenAI tools/function calling 的模型。{}",
        result
            .warning
            .as_deref()
            .map(|warning| format!("详情：{warning}"))
            .unwrap_or_else(|| "详情：模型没有返回可识别的工具调用。".to_string())
    )
}

impl ToolProbeOutcome {
    fn unsupported(tool_call_name: Option<String>, warning: String) -> Self {
        Self {
            ok: false,
            tool_call_name,
            warning: Some(warning),
        }
    }
}

#[cfg(test)]
#[path = "user_agent_probe_tests.rs"]
mod tests;
