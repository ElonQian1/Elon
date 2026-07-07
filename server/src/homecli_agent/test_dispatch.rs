use anyhow::Result;
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use homecli_proto::AgentToServer;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use crate::{admin, types::AppState};
use super::{AgentManager, agent_session::run_agent_session};

#[derive(Debug, Deserialize)]
pub struct TestDispatchReq {
    pub agent_id: String,
    pub cli: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Serialize)]
pub struct TestDispatchResp {
    pub task_id: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestCliPromptReq {
    pub agent_id: String,
    pub cli: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct TestCliPromptResp {
    pub req_id: String,
    pub exit_ok: Option<bool>,
    pub text: String,
    pub error: Option<String>,
}

/// Synchronously dispatch a command to the named agent and collect everything
/// until exit/error. Decodes base64 stdout into a single concatenated string.
pub async fn test_dispatch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TestDispatchReq>,
) -> impl IntoResponse {
    // Require admin token (already used by /api/admin/* endpoints).
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if presented.is_empty() || presented != state.admin_token {
        return (StatusCode::UNAUTHORIZED, "admin token required").into_response();
    }

    let (task_id, mut rx) = match state
        .agent_manager
        .dispatch(&req.agent_id, req.cli, req.args, req.cwd, vec![])
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };

    let mut pid = None;
    let mut exit_code = None;
    let mut stdout = Vec::<u8>::new();
    let mut error = None;
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentToServer::TaskStarted { pid: p, .. } => pid = Some(p),
            AgentToServer::TaskStdout { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    stdout.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskExit { code, .. } => {
                exit_code = code;
                break;
            }
            AgentToServer::TaskError { message, .. } => {
                error = Some(message);
                break;
            }
            _ => {}
        }
    }

    let resp = TestDispatchResp {
        task_id,
        pid,
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        error,
    };
    Json(resp).into_response()
}

/// Smoke test for CliPrompt flow (cloud -> PC relay -> local CLI -> stream back).
/// Requires ADMIN_TOKEN in Authorization header.
pub async fn test_cli_prompt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TestCliPromptReq>,
) -> impl IntoResponse {
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if presented.is_empty() || presented != state.admin_token {
        return (StatusCode::UNAUTHORIZED, "admin token required").into_response();
    }

    let (req_id, mut rx) = match state
        .agent_manager
        .dispatch_cli_prompt(&req.agent_id, req.cli, req.extra_args, req.prompt)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let mut text = String::new();
    let mut exit_ok = None;
    let mut error = None;

    loop {
        match tokio::time::timeout(Duration::from_secs(120), rx.recv()).await {
            Ok(Some(AgentToServer::CliChunk { text: chunk, .. })) => {
                text.push_str(&chunk);
            }
            Ok(Some(AgentToServer::CliDone {
                exit_ok: ok,
                error: e,
                ..
            })) => {
                exit_ok = Some(ok);
                error = e;
                break;
            }
            Ok(Some(_)) => {
                // Ignore unrelated message variants.
            }
            Ok(None) => {
                error = Some("cli prompt channel closed".to_string());
                break;
            }
            Err(_) => {
                error = Some("cli prompt timeout (120s)".to_string());
                break;
            }
        }
    }

    Json(TestCliPromptResp {
        req_id,
        exit_ok,
        text,
        error,
    })
    .into_response()
}
