use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::info;

use crate::{agent, client_protocol, tools, types};
use types::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    use futures::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();

    info!("new WebSocket connection");

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(text) => text,
            Message::Ping(payload) => {
                if sender.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Binary(_) => continue,
        };

        info!("received WebSocket message: {} bytes", text.len());
        let state_clone = state.clone();
        let request = client_protocol::parse_client_message(&text);
        info!(
            "dispatching request: user_id={} workspace_user_id={} agent={:?} chars={}",
            request.user_id,
            request.workspace_user_id,
            request.agent,
            request.content.chars().count()
        );
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let agent_task = tokio::spawn(async move {
            agent::run(
                &request.user_id,
                &request.workspace_user_id,
                &request.content,
                request.agent.as_deref(),
                &state_clone,
                tx,
            )
            .await;
        });

        let mut client_disconnected = false;
        loop {
            tokio::select! {
                progress = rx.recv() => {
                    match progress {
                        Some(progress) => {
                            if sender.send(Message::Text(progress)).await.is_err() {
                                client_disconnected = true;
                                break;
                            }
                        }
                        None => break,
                    }
                }
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(Message::Ping(payload))) => {
                            if sender.send(Message::Pong(payload)).await.is_err() {
                                client_disconnected = true;
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Ok(Message::Close(_))) | None => {
                            client_disconnected = true;
                            break;
                        }
                        Some(Ok(Message::Text(text))) => {
                            info!(
                                "received WebSocket message while request was running; ignoring: {} bytes",
                                text.len()
                            );
                        }
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Err(error)) => {
                            info!("WebSocket receive error while request was running: {}", error);
                            client_disconnected = true;
                            break;
                        }
                    }
                }
            }
        }
        if client_disconnected {
            info!("client disconnected while request was running; aborting agent task");
            agent_task.abort();
            let _ = agent_task.await;
            break;
        }
        let _ = agent_task.await;
    }

    info!("WebSocket connection closed");
}

pub async fn download_apk(
    Path((user_id, filename)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((StatusCode::BAD_REQUEST, "invalid filename".into()));
    }
    if !filename.ends_with(".apk") {
        return Err((
            StatusCode::BAD_REQUEST,
            "only APK downloads are allowed".into(),
        ));
    }

    let workspace = types::get_user_workspace(&state.workspace_root, &user_id);
    let apk_path = tools::find_apk_by_filename(&workspace, &filename).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("APK file {} does not exist", filename),
        )
    })?;

    let data = tokio::fs::read(&apk_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read APK: {}", e),
        )
    })?;

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.android.package-archive",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
