use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const CHANNEL_AI_HEARTBEAT_ONLY_TIMEOUT_ENV: &str = "ELON_CHANNEL_AI_HEARTBEAT_ONLY_TIMEOUT_SECS";
const CHANNEL_AI_HEARTBEAT_ONLY_TIMEOUT_DEFAULT_SECS: u64 = 180;
const CHANNEL_AI_TOOL_RESULT_TIMEOUT_ENV: &str = "ELON_CHANNEL_AI_TOOL_RESULT_TIMEOUT_SECS";
const CHANNEL_AI_TOOL_RESULT_TIMEOUT_DEFAULT_SECS: u64 = 1800;

#[derive(Clone, Debug)]
pub(crate) struct ChannelAiPendingTool {
    tool: String,
    summary: String,
    started_at: Instant,
}

#[derive(Debug)]
pub(crate) struct ChannelAiPendingTools {
    queue: VecDeque<ChannelAiPendingTool>,
    timeout: Duration,
}

impl ChannelAiPendingTools {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            timeout: channel_ai_tool_result_timeout(),
        }
    }

    pub(crate) fn note_event(&mut self, event_type: &str, value: &serde_json::Value) {
        match event_type {
            "tool_call" => self.queue.push_back(pending_tool_from_event(value)),
            "tool_result" => {
                self.queue.pop_front();
            }
            "done" | "error" => self.queue.clear(),
            _ => {}
        }
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    pub(crate) fn idle_timed_out(
        &self,
        last_effective_progress_at: Instant,
        timeout: Duration,
    ) -> bool {
        !self.has_pending() && last_effective_progress_at.elapsed() >= timeout
    }

    pub(crate) fn timed_out(&self) -> Option<(&ChannelAiPendingTool, u64)> {
        let pending = self.queue.front()?;
        if pending.started_at.elapsed() < self.timeout {
            return None;
        }
        Some((pending, self.timeout.as_secs()))
    }
}

impl ChannelAiPendingTool {
    pub(crate) fn tool(&self) -> &str {
        &self.tool
    }

    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }

    pub(crate) fn label(&self) -> String {
        let tool_label = match self.tool.as_str() {
            "shell" => "shell 命令",
            "file_change" => "文件修改",
            "web_search" => "网络搜索",
            _ => "工具调用",
        };
        if self.summary.is_empty() {
            tool_label.to_string()
        } else {
            format!("{}：{}", tool_label, self.summary)
        }
    }
}

pub(crate) fn channel_ai_heartbeat_only_timeout() -> Duration {
    let secs = std::env::var(CHANNEL_AI_HEARTBEAT_ONLY_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(CHANNEL_AI_HEARTBEAT_ONLY_TIMEOUT_DEFAULT_SECS)
        .clamp(60, 1800);
    Duration::from_secs(secs)
}

fn channel_ai_tool_result_timeout() -> Duration {
    let secs = std::env::var(CHANNEL_AI_TOOL_RESULT_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(CHANNEL_AI_TOOL_RESULT_TIMEOUT_DEFAULT_SECS)
        .clamp(180, 7200);
    Duration::from_secs(secs)
}

fn pending_tool_from_event(value: &serde_json::Value) -> ChannelAiPendingTool {
    let tool = value
        .get("tool")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("tool")
        .to_string();
    ChannelAiPendingTool {
        summary: pending_tool_summary(value, &tool),
        tool,
        started_at: Instant::now(),
    }
}

fn pending_tool_summary(value: &serde_json::Value, tool: &str) -> String {
    let args = value.get("args").and_then(|v| v.as_object());
    let raw = match tool {
        "shell" => args
            .and_then(|a| a.get("command"))
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        "file_change" => args.and_then(|a| a.get("files")).map(|v| v.to_string()),
        _ => None,
    };
    truncate_tool_summary(raw.as_deref().unwrap_or(""))
}

fn truncate_tool_summary(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_LEN: usize = 180;
    if compact.chars().count() <= MAX_LEN {
        return compact;
    }
    let mut shortened = compact
        .chars()
        .take(MAX_LEN.saturating_sub(1))
        .collect::<String>();
    shortened.push('…');
    shortened
}

#[cfg(test)]
mod tests {
    use super::ChannelAiPendingTools;

    #[test]
    fn pending_tools_tracks_tool_call_and_result() {
        let mut pending_tools = ChannelAiPendingTools::new();
        assert!(!pending_tools.has_pending());

        pending_tools.note_event(
            "tool_call",
            &serde_json::json!({
                "tool": "shell",
                "args": { "command": "cargo test" }
            }),
        );
        assert!(pending_tools.has_pending());

        pending_tools.note_event("tool_result", &serde_json::json!({}));
        assert!(!pending_tools.has_pending());
    }

    #[test]
    fn pending_tools_clear_on_done_or_error() {
        for terminal_event in ["done", "error"] {
            let mut pending_tools = ChannelAiPendingTools::new();
            pending_tools.note_event(
                "tool_call",
                &serde_json::json!({
                    "tool": "shell",
                    "args": { "command": "cargo build" }
                }),
            );
            assert!(pending_tools.has_pending());

            pending_tools.note_event(terminal_event, &serde_json::json!({}));
            assert!(!pending_tools.has_pending());
        }
    }
}
