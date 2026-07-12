use super::ai_cli_output::extract_json_agent_message;
use super::{
    run_via_pc_agent_once, AiCliRequestMode, NativeSessionScope, PcAgentRunOutcome,
};
use crate::types::{AppState, WsMessage};
use anyhow::{anyhow, Result};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

const REQUEST_PREFIX: &str = "ELON_REQUEST_UI_ROUTE:";

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_via_pc_agent(
    agent_id: &str,
    user_id: &str,
    cwd: Option<&str>,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    attempt_apk_sync: bool,
    cli_name: &str,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<PcAgentRunOutcome> {
    let first = run_via_pc_agent_once(
        agent_id,
        user_id,
        cwd,
        user_message,
        preflight_note,
        request_mode,
        native_session_scope.clone(),
        download_base,
        artifact_workspace,
        attempt_apk_sync,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await?;
    let PcAgentRunOutcome::UiRerouteRequested { reason } = first else {
        return Ok(first);
    };
    let promoted = crate::ui_design_tasks::promote_codex_ui_route(user_message)
        .map_err(|error| anyhow!("UI 路由救援契约生成失败：{error}"))?;
    let _ = tx.send(
        WsMessage::progress(format!("Codex 已识别为 UI 任务，正在按需启用 UI 工具链：{reason}"))
            .to_json(),
    );
    match run_via_pc_agent_once(
        agent_id,
        user_id,
        cwd,
        &promoted,
        preflight_note,
        request_mode,
        native_session_scope,
        download_base,
        artifact_workspace,
        attempt_apk_sync,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await?
    {
        PcAgentRunOutcome::UiRerouteRequested { .. } => {
            Err(anyhow!("Codex 在 UI 工具链内重复申请重路由，已停止以防循环"))
        }
        outcome => Ok(outcome),
    }
}

pub(super) fn requested_ui_route(codex_jsonl: &str) -> Option<String> {
    let message = extract_json_agent_message(codex_jsonl)?;
    let line = message
        .lines()
        .find(|line| line.trim().starts_with(REQUEST_PREFIX))?;
    let reason = line
        .trim()
        .strip_prefix(REQUEST_PREFIX)?
        .trim()
        .chars()
        .take(300)
        .collect::<String>();
    (!reason.is_empty()).then_some(reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_an_explicit_agent_route_request() {
        let output = concat!(
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"ELON_REQUEST_UI_ROUTE: 按钮视觉层级与间距调整"}}"#,
            "\n"
        );
        assert_eq!(
            requested_ui_route(output).as_deref(),
            Some("按钮视觉层级与间距调整")
        );
        assert!(requested_ui_route("ELON_REQUEST_UI_ROUTE: 用户原文").is_none());
    }
}
