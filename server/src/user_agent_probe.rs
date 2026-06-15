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

    Ok(UserAgentProbeResult {
        api_base: cfg.api_base.clone(),
        model: cfg.model.clone(),
        latency_ms: started.elapsed().as_millis(),
        sample,
    })
}

fn extract_sample(value: &Value) -> String {
    let text = value["choices"][0]["message"]["content"]
        .as_str()
        .or_else(|| value["choices"][0]["text"].as_str())
        .unwrap_or("");
    text.trim().chars().take(80).collect()
}

fn probe_timeout_secs() -> u64 {
    std::env::var("AI_USER_API_PROBE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(20)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_api_base_trims_trailing_slashes() {
        assert_eq!(
            normalize_api_base(" https://api.example.com/v1/// ").as_deref(),
            Some("https://api.example.com/v1")
        );
        assert!(normalize_api_base("api.example.com/v1").is_none());
    }

    #[test]
    fn resolve_probe_reuses_saved_key_when_request_key_is_empty() {
        let existing = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-saved".into()),
            model: Some("saved-model".into()),
            ..Default::default()
        };
        let cfg = resolve_probe_config(
            UserAgentProbeRequest {
                api_base: Some("https://api.other.com/v1/".into()),
                api_key: Some(" ".into()),
                model: Some("new-model".into()),
            },
            &existing,
        )
        .expect("probe config should resolve");

        assert_eq!(cfg.api_base, "https://api.other.com/v1");
        assert_eq!(cfg.api_key, "sk-saved");
        assert_eq!(cfg.model, "new-model");
    }
}
