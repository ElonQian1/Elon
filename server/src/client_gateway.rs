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

use crate::{agent, client_protocol, types};
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
        if let Message::Text(text) = msg {
            let state_clone = state.clone();
            let request = client_protocol::parse_client_message(&text);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            tokio::spawn(async move {
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

            while let Some(progress) = rx.recv().await {
                if sender.send(Message::Text(progress)).await.is_err() {
                    break;
                }
            }
        }
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

    let candidates = [
        workspace
            .join("app/build/outputs/apk/debug")
            .join(&filename),
        workspace
            .join("app/build/outputs/apk/release")
            .join(&filename),
        workspace
            .join("android/app/build/outputs/apk/debug")
            .join(&filename),
        workspace
            .join("android/app/build/outputs/apk/release")
            .join(&filename),
        workspace.join("build/outputs/apk/debug").join(&filename),
        workspace.join("build/outputs/apk/release").join(&filename),
    ];

    let apk_path = candidates
        .iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("APK file {} does not exist", filename),
            )
        })?;

    let data = tokio::fs::read(apk_path).await.map_err(|e| {
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
