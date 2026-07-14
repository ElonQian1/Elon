use tokio::sync::mpsc::UnboundedSender;

use super::NativeSessionScope;
use crate::types::{AppState, WsMessage};

#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_ui_route_learning(
    is_codex: bool,
    scope: Option<&NativeSessionScope>,
    user_message: &str,
    full_text: &str,
    exit_ok: bool,
    state: &AppState,
    tx: &UnboundedSender<String>,
) {
    if !is_codex {
        return;
    }
    let Some(scope) = scope else {
        return;
    };
    match crate::ui_design_tasks::finalize_ui_route_learning(
        &state.store,
        &scope.project_id,
        &scope.user_id,
        user_message,
        full_text,
        exit_ok,
    ) {
        Ok(Some(entry)) => {
            let sample = entry.sample_text.chars().take(40).collect::<String>();
            let _ = tx.send(
                WsMessage::progress(format!("已记录 UI 路由经验：{sample}（{}）", entry.status))
                    .to_json(),
            );
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(error = %error, "记录 UI 路由经验失败"),
    }
}
