use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};

use crate::types::AppState;

pub(crate) async fn rg_win(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    crate::node_api::download_node_agent_binary(state, "ripgrep-windows.zip", "ripgrep-windows.zip")
        .await
}
