use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    agent_api_loop::resolve_agent,
    agent_llm_call::call_chat_llm,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

#[derive(Deserialize)]
pub struct SpeechTranslateRequest {
    pub text: String,
    pub agent_name: Option<String>,
}

#[derive(Serialize)]
struct SpeechTranslateResponse {
    source_text: String,
    text: String,
    translated: bool,
}

pub async fn translate_user_speech(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(req): Json<SpeechTranslateRequest>,
) -> Response {
    let caller = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if caller.id != user_id {
        return json_error(StatusCode::FORBIDDEN, "无权为此用户执行语音翻译");
    }

    let source_text = req.text.trim().to_string();
    if source_text.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "语音文本不能为空");
    }
    if source_text.chars().count() > 2000 {
        return json_error(StatusCode::BAD_REQUEST, "语音文本过长，请分段输入");
    }

    let agent_name = req
        .agent_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let workspace = state.get_user_workspace(&user_id);
    let agent = match resolve_agent(&state, &workspace, agent_name).await {
        Ok(agent) => agent,
        Err(err) => {
            tracing::warn!("语音翻译没有可用 API 代理: {}", err);
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "当前没有可用翻译模型");
        }
    };

    let messages = vec![
        json!({
            "role": "system",
            "content": "你是语音识别结果后处理器。把用户给出的文本翻译或改写成自然、简洁的简体中文，只输出最终文本。若原文已经是简体中文，只做必要的错别字和标点整理。保留人名、品牌、代码、命令、URL、数字和单位；不要解释，不要加引号。"
        }),
        json!({
            "role": "user",
            "content": source_text
        }),
    ];

    let response =
        match call_chat_llm(&state, &agent, &messages, &user_id, "speech_translate").await {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!("语音翻译调用失败: {}", err);
                return json_error(StatusCode::BAD_GATEWAY, "语音翻译失败");
            }
        };

    let text = response["choices"][0]["message"]["content"]
        .as_str()
        .map(clean_translation)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| source_text.clone());

    Json(SpeechTranslateResponse {
        translated: text != source_text,
        source_text,
        text,
    })
    .into_response()
}

fn clean_translation(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('“')
        .trim_matches('”')
        .trim()
        .to_string()
}
