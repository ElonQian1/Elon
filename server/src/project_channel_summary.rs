// server/src/project_channel_summary.rs
//! Background AI summaries for selected project channel messages.

use std::sync::Arc;

use crate::{
    agent, pc_agent_runtime_choice::PcRuntimeRoutePreference, store::ProjectAccess, types::AppState,
};

pub(crate) struct ChannelSummaryTask {
    pub state: Arc<AppState>,
    pub user_id: String,
    pub project: ProjectAccess,
    pub project_id: String,
    pub channel_id: String,
    pub prompt: String,
    pub agent: Option<String>,
    pub runtime_route: Option<PcRuntimeRoutePreference>,
    pub trace_id: String,
}

pub(crate) fn spawn_channel_summary(task: ChannelSummaryTask) {
    tokio::spawn(async move {
        let _ = task.state.store.insert_project_channel_message(
            &task.project_id,
            &task.channel_id,
            None,
            "ai_progress",
            "AI 正在总结这些聊天记录...",
            None,
            None,
        );
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let run_state = task.state.clone();
        let run_project = task.project.clone();
        let run_user_id = task.user_id.clone();
        let run_prompt = task.prompt.clone();
        let run_agent = task.agent.clone();
        let run_runtime_route = task.runtime_route;
        let run_trace_id = task.trace_id.clone();
        let summary_conversation_id = format!("channel-summary-{}", task.channel_id);
        let download_base = format!(
            "{}/api/projects/{}/download",
            task.state.public_url, task.project.id
        );
        let runner = tokio::spawn(async move {
            agent::run_for_project(
                &run_user_id,
                &run_project,
                &download_base,
                Some(&summary_conversation_id),
                &run_prompt,
                run_agent.as_deref(),
                run_runtime_route,
                Some(&run_trace_id),
                &run_state,
                tx,
            )
            .await;
        });

        let mut final_reply = String::new();
        let mut error = None;
        while let Some(raw) = rx.recv().await {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let message = value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                match event_type {
                    "done" => final_reply = message.if_blank("AI 已完成总结。").to_string(),
                    "error" => error = Some(message.if_blank("AI 总结失败。").to_string()),
                    _ => {}
                }
            }
        }
        let _ = runner.await;
        let content = if let Some(error) = error {
            format!("AI 总结失败：{}", error)
        } else {
            format!(
                "AI 总结\n{}",
                final_reply.if_blank("AI 已完成总结，但没有返回可展示内容。")
            )
        };
        let _ = task.state.store.insert_project_channel_message(
            &task.project_id,
            &task.channel_id,
            None,
            "ai_result",
            &content,
            None,
            None,
        );
        task.state.server_traces.record(
            &task.trace_id,
            "channel_summary_done",
            serde_json::json!({
                "project_id": &task.project.id,
                "channel_id": &task.channel_id,
                "ok": content.starts_with("AI 总结\n"),
            }),
        );
    });
}

trait BlankFallback {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl BlankFallback for str {
    fn if_blank<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}
