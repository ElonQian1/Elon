//! HTTP-over-WS 反向代理：将 APK 的 HTTP 请求通过已建立的 agent WS 隧道
//! 转发给 PC 本地 elon server 处理，再把响应透传回 APK。
//!
//! 路由：`/api/pc-relay/:agent_id/*path`
//! 认证：APK 携带的 Bearer token（透传给 PC 侧，PC 侧再做 owner token 检查）
//!
//! 工作流：
//!   1. APK → POST/GET /api/pc-relay/elon-pc-1/health
//!   2. 云端收到请求，从 agent_manager 找到对应 PC 的 WS 连接
//!   3. 发送 ServerToAgent::HttpRequest 给 PC
//!   4. PC 本地 elon server 收到后转发到 127.0.0.1:{port}
//!   5. PC 回送 AgentToServer::HttpResponse 或 HttpError
//!   6. 云端把响应透传给 APK

use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use homecli_proto::AgentToServer;
use std::{collections::HashMap, sync::Arc};

use crate::types::AppState;

/// GET|POST|PUT|DELETE /api/pc-relay/:agent_id/*path
pub async fn pc_relay_handler(
    State(state): State<Arc<AppState>>,
    Path((agent_id, sub_path)): Path<(String, String)>,
    req: Request,
) -> Response {
    let method = req.method().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let path = format!("/{}{}", sub_path, query);

    // 收集转发 header（过滤掉 hop-by-hop）
    let skip_headers: &[&str] = &[
        "host",
        "connection",
        "transfer-encoding",
        "upgrade",
        "keep-alive",
        "proxy-connection",
    ];
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter(|(k, _)| !skip_headers.contains(&k.as_str()))
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
        })
        .collect();

    // 读取 body（限制 32 MB）
    let body_bytes = match axum::body::to_bytes(req.into_body(), 32 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("body read error: {e}"),
            )
                .into_response();
        }
    };
    let body_b64 = if body_bytes.is_empty() {
        None
    } else {
        Some(B64.encode(&body_bytes))
    };

    // 通过 WS 隧道转发给 PC
    match state
        .agent_manager
        .dispatch_http(&agent_id, method, path, headers, body_b64)
        .await
    {
        Ok(AgentToServer::HttpResponse {
            status,
            headers: resp_headers,
            body_b64,
            ..
        }) => {
            let status_code =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body = body_b64
                .as_deref()
                .and_then(|b| B64.decode(b).ok())
                .unwrap_or_default();

            let mut response = Response::builder().status(status_code);
            for (k, v) in &resp_headers {
                if let (Ok(name), Ok(val)) = (
                    k.parse::<HeaderName>(),
                    v.parse::<HeaderValue>(),
                ) {
                    response = response.header(name, val);
                }
            }
            response.body(Body::from(body)).unwrap_or_else(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            })
        }
        Ok(AgentToServer::HttpError { message, .. }) => (
            StatusCode::BAD_GATEWAY,
            format!("PC relay error: {message}"),
        )
            .into_response(),
        Ok(_) => (StatusCode::INTERNAL_SERVER_ERROR, "unexpected relay response").into_response(),
        Err(e) => {
            let body = format!("{{\"error\":\"PC 未在线或连接超时: {e}\"}}");
            (
                StatusCode::BAD_GATEWAY,
                [(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/json"),
                )],
                body,
            )
                .into_response()
        }
    }
}
