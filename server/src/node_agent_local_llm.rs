//! 本地 LLM 服务发现（Ollama / LM Studio / 自定义 OpenAI-compatible）及推理。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::time::Duration;

use homecli_proto::{AgentToServer, ModelCapability};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use super::{ws_text, NodeConfig};

// ── 本地 LLM 扫描 ─────────────────────────────────────────────────────────────

/// Ollama /api/tags 响应
#[derive(Deserialize)]
struct OllamaTagsResp {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

/// OpenAI-compatible /v1/models 响应
#[derive(Deserialize)]
struct OpenAiModelsResp {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

async fn scan_ollama(base_url: &str, price: f64) -> Vec<ModelCapability> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    let url = format!("{}/api/tags", base_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<OllamaTagsResp>().await {
                return body
                    .models
                    .into_iter()
                    .map(|m| ModelCapability {
                        model_id: m.name.clone(),
                        display_name: m.name,
                        context_len: 4096,
                        provider: "ollama".into(),
                        price_per_1k_credits: price,
                    })
                    .collect();
            }
        }
        _ => {}
    }
    vec![]
}

async fn scan_openai_compat(base_url: &str, provider: &str, price: f64) -> Vec<ModelCapability> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    let url = format!("{}/v1/models", base_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<OpenAiModelsResp>().await {
                return body
                    .data
                    .into_iter()
                    .map(|m| ModelCapability {
                        model_id: m.id.clone(),
                        display_name: m.id,
                        context_len: 4096,
                        provider: provider.to_string(),
                        price_per_1k_credits: price,
                    })
                    .collect();
            }
        }
        _ => {}
    }
    vec![]
}

pub(super) async fn discover_models(cfg: &NodeConfig) -> Vec<ModelCapability> {
    let mut models = Vec::new();

    let ollama = scan_ollama(&cfg.ollama_url, cfg.price_per_1k).await;
    if !ollama.is_empty() {
        info!("✅ Ollama: {} 个模型", ollama.len());
        models.extend(ollama);
    }

    if let Some(ref url) = cfg.lm_studio_url {
        let lm = scan_openai_compat(url, "lm_studio", cfg.price_per_1k).await;
        if !lm.is_empty() {
            info!("✅ LM Studio: {} 个模型", lm.len());
            models.extend(lm);
        }
    }

    if let Some(ref url) = cfg.custom_url {
        let custom = scan_openai_compat(url, "custom", cfg.price_per_1k).await;
        if !custom.is_empty() {
            info!("✅ 自定义 LLM: {} 个模型", custom.len());
            models.extend(custom);
        }
    }

    models
}

// ── LLM 推理（OpenAI-compatible 流式）────────────────────────────────────────

// ── LLM 推理（OpenAI-compatible 流式）────────────────────────────────────────

/// 调用本地 LLM（OpenAI-compatible stream 接口），把 chunk 通过 out_tx 发回云端
pub async fn run_llm_inference(
    cfg: &NodeConfig,
    req_id: String,
    model: &str,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
    out_tx: mpsc::UnboundedSender<Message>,
) {
    // 选择端点
    let base_url = if model.contains('/') || cfg.lm_studio_url.is_some() {
        cfg.lm_studio_url.as_deref().unwrap_or(&cfg.ollama_url)
    } else {
        &cfg.ollama_url
    };

    // Ollama 使用 /api/chat，其余使用 /v1/chat/completions
    let endpoint = if base_url.contains(":11434") {
        format!("{}/api/chat", base_url)
    } else {
        format!("{}/v1/chat/completions", base_url)
    };

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });
    if let Some(mt) = max_tokens {
        body["max_tokens"] = serde_json::json!(mt);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    let resp = match client.post(&endpoint).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamError {
                req_id,
                message: format!("LLM 请求失败: {e}"),
            }));
            return;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let msg = resp.text().await.unwrap_or_default();
        let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamError {
            req_id,
            message: format!("LLM 错误 {status}: {msg}"),
        }));
        return;
    }

    // 读取 SSE 流
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut finish_reason = "stop".to_string();
    let mut stream = resp.bytes_stream();

    let mut buf = String::new();
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                warn!("LLM 流读取错误: {e}");
                break;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // SSE 每行 "data: {...}\n\n" 或 Ollama JSON lines
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);

            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }

            let json_str = line.strip_prefix("data: ").unwrap_or(&line);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                // OpenAI-compatible delta
                if let Some(delta) = val
                    .pointer("/choices/0/delta/content")
                    .and_then(|v| v.as_str())
                {
                    if !delta.is_empty() {
                        completion_tokens += 1; // 近似计数
                        let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamChunk {
                            req_id: req_id.clone(),
                            delta: delta.to_string(),
                        }));
                    }
                }
                // Ollama message.content
                if let Some(content) = val.pointer("/message/content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        completion_tokens += 1;
                        let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamChunk {
                            req_id: req_id.clone(),
                            delta: content.to_string(),
                        }));
                    }
                }
                // 完成信号
                if let Some(r) = val
                    .pointer("/choices/0/finish_reason")
                    .and_then(|v| v.as_str())
                {
                    if !r.is_empty() && r != "null" {
                        finish_reason = r.to_string();
                    }
                }
                if val.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                    prompt_tokens = val
                        .pointer("/prompt_eval_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    completion_tokens =
                        val.pointer("/eval_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(completion_tokens as u64) as u32;
                }
                // token usage from OpenAI response
                if let Some(usage) = val.get("usage") {
                    prompt_tokens = usage
                        .get("prompt_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(prompt_tokens as u64) as u32;
                    completion_tokens = usage
                        .get("completion_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(completion_tokens as u64)
                        as u32;
                }
            }
        }
    }

    let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamEnd {
        req_id,
        prompt_tokens,
        completion_tokens,
        finish_reason,
    }));
}
