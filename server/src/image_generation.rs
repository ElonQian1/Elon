use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

use crate::types::{AppState, ImageModelConfig};

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedImage {
    pub job_id: String,
    pub url: String,
    pub revised_prompt: Option<String>,
}

pub async fn generate_text_to_image(state: &AppState, prompt: &str) -> Result<GeneratedImage> {
    let cfg = state
        .image_model
        .as_ref()
        .ok_or_else(|| anyhow!("文生图模型未配置，请在 server/.env 设置 IMAGE_API_KEY"))?;

    let job_id = submit_text_to_image_job(state, cfg, prompt).await?;

    for attempt in 0..cfg.max_attempts {
        if attempt > 0 {
            sleep(Duration::from_secs(cfg.poll_interval_secs)).await;
        }

        let result = query_text_to_image_job(state, cfg, &job_id).await?;
        let status = result["status"].as_str().unwrap_or_default();

        if status == "completed" || status == "succeeded" || status == "success" {
            let url = extract_image_url(&result)
                .ok_or_else(|| anyhow!("图片任务已完成，但响应中没有找到图片 URL"))?;
            let revised_prompt = extract_revised_prompt(&result);
            return Ok(GeneratedImage {
                job_id,
                url,
                revised_prompt,
            });
        }

        if matches!(status, "failed" | "error" | "cancelled" | "canceled") {
            return Err(anyhow!("图片生成失败: {}", compact_json(&result)));
        }
    }

    Err(anyhow!(
        "图片生成超时，任务 ID: {}。可稍后用该 ID 查询结果",
        job_id
    ))
}

async fn submit_text_to_image_job(
    state: &AppState,
    cfg: &ImageModelConfig,
    prompt: &str,
) -> Result<String> {
    let url = format!("{}/submit", cfg.api_base.trim_end_matches('/'));
    let body = json!({
        "model": cfg.model,
        "prompt": prompt,
    });

    let response = post_json(state, cfg, &url, &body).await?;
    response["id"].as_str().map(str::to_string).ok_or_else(|| {
        anyhow!(
            "提交图片任务成功，但响应中没有任务 ID: {}",
            compact_json(&response)
        )
    })
}

async fn query_text_to_image_job(
    state: &AppState,
    cfg: &ImageModelConfig,
    job_id: &str,
) -> Result<Value> {
    let url = format!("{}/query", cfg.api_base.trim_end_matches('/'));
    let body = json!({
        "model": cfg.model,
        "id": job_id,
    });

    post_json(state, cfg, &url, &body).await
}

async fn post_json(
    state: &AppState,
    cfg: &ImageModelConfig,
    url: &str,
    body: &Value,
) -> Result<Value> {
    let response = state
        .http_client
        .post(url)
        .bearer_auth(&cfg.api_key)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow!("文生图请求超时，请稍后重试")
            } else {
                anyhow!("文生图请求失败: {}", e)
            }
        })?;

    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "文生图 API 返回错误 {}: {}",
            status,
            truncate(&text, 800)
        ));
    }

    serde_json::from_str(&text).map_err(|e| {
        anyhow!(
            "文生图 API 响应不是有效 JSON: {}; body={}",
            e,
            truncate(&text, 800)
        )
    })
}

fn extract_image_url(value: &Value) -> Option<String> {
    value["data"].as_array()?.iter().find_map(|item| {
        item["url"]
            .as_str()
            .or_else(|| item["image_url"].as_str())
            .or_else(|| item["result_url"].as_str())
            .map(str::to_string)
    })
}

fn extract_revised_prompt(value: &Value) -> Option<String> {
    value["data"].as_array()?.iter().find_map(|item| {
        item["revised_prompt"]
            .as_str()
            .or_else(|| item["revisedPrompt"].as_str())
            .map(str::to_string)
    })
}

fn compact_json(value: &Value) -> String {
    truncate(&value.to_string(), 800)
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}
