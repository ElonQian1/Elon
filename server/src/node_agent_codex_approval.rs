// server/src/node_agent_codex_approval.rs

use serde_json::{json, Value};

const MAX_RECENT_OUTPUT: usize = 8192;
const MAX_PROMPT_EXCERPT: usize = 600;
const APPROVAL_TIMEOUT_MS: u128 = 10 * 60 * 1_000;

#[derive(Debug, Clone, Default)]
pub(crate) struct CodexApprovalTracker {
    recent_output: String,
    pending: Option<PendingCodexApproval>,
    next_index: usize,
}

#[derive(Debug, Clone)]
struct PendingCodexApproval {
    approval_id: String,
    tool: String,
}

impl CodexApprovalTracker {
    pub(crate) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn observe_output(
        &mut self,
        task_id: &str,
        sidecar_session_id: &str,
        text: &str,
        at_ms: u128,
    ) -> Option<Value> {
        if self.pending.is_some() {
            return None;
        }
        self.recent_output
            .push_str(&strip_cli_control_sequences(text));
        trim_recent_output(&mut self.recent_output);
        if !looks_like_codex_approval_prompt(&self.recent_output) {
            return None;
        }
        self.next_index += 1;
        let approval_id = format!("sidecar_tap_{}", self.next_index);
        let tool = infer_tool_name(&self.recent_output).to_string();
        self.pending = Some(PendingCodexApproval {
            approval_id: approval_id.clone(),
            tool: tool.clone(),
        });
        Some(approval_required_event(
            task_id,
            sidecar_session_id,
            &approval_id,
            &tool,
            &self.recent_output,
            at_ms,
        ))
    }

    pub(crate) fn observe_decision(
        &mut self,
        task_id: &str,
        approval_id: Option<&str>,
        decision: Option<&str>,
        at_ms: u128,
    ) -> Option<Value> {
        let pending = self.pending.as_ref()?;
        let approval_id = approval_id
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        if approval_id != pending.approval_id {
            return None;
        }
        let decision = normalize_decision(decision?)?;
        let event = approval_decision_event(task_id, approval_id, &pending.tool, decision, at_ms);
        self.pending = None;
        self.recent_output.clear();
        Some(event)
    }
}

fn approval_required_event(
    task_id: &str,
    sidecar_session_id: &str,
    approval_id: &str,
    tool: &str,
    output: &str,
    at_ms: u128,
) -> Value {
    json!({
        "type": "tool_approval_required",
        "schema": "elon.sidecar.tool_approval.v1",
        "req_id": task_id,
        "approval_id": approval_id,
        "tool": tool,
        "source": "managed_pty_conpty_sidecar",
        "prompt_excerpt": prompt_excerpt(output),
        "approval_checkpoint": {
            "schema": "elon.sidecar.tool_approval_checkpoint.v1",
            "task_id": task_id,
            "sidecar_session_id": sidecar_session_id,
            "registered_at_ms": at_ms,
            "expires_at_ms": at_ms.saturating_add(APPROVAL_TIMEOUT_MS),
            "restart_recovery": {
                "supported": true,
                "next_action": "approve_or_deny_sidecar_waiter",
                "reason": "审批由一龙 sidecar 持有；node-agent 重启后可通过 sidecar mailbox 继续写入审批决定。"
            }
        }
    })
}

fn approval_decision_event(
    task_id: &str,
    approval_id: &str,
    tool: &str,
    decision: &str,
    at_ms: u128,
) -> Value {
    json!({
        "type": "tool_approval_decision",
        "schema": "elon.sidecar.tool_approval.v1",
        "req_id": task_id,
        "approval_id": approval_id,
        "tool": tool,
        "decision": decision,
        "status": if decision == "approve" { "approved" } else { "denied" },
        "source": "managed_pty_conpty_sidecar",
        "at_ms": at_ms
    })
}

fn looks_like_codex_approval_prompt(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    let asks_decision = lower.contains("allow")
        || lower.contains("approve")
        || lower.contains("permission")
        || lower.contains("approval")
        || lower.contains("y/n")
        || lower.contains("yes/no");
    let mentions_action = lower.contains("command")
        || lower.contains("tool")
        || lower.contains("patch")
        || lower.contains("edit")
        || lower.contains("write")
        || lower.contains("run");
    asks_decision
        && mentions_action
        && (lower.contains("?")
            || lower.contains("[y")
            || lower.contains("(y")
            || lower.contains("allow command")
            || lower.contains("approve command"))
}

fn infer_tool_name(output: &str) -> &'static str {
    let lower = output.to_ascii_lowercase();
    if lower.contains("apply_patch") || lower.contains("patch") {
        "apply_patch"
    } else if lower.contains("write") || lower.contains("edit") {
        "write_file"
    } else if lower.contains("command") || lower.contains("run") || lower.contains("$ ") {
        "run_command"
    } else {
        "codex_cli"
    }
}

fn prompt_excerpt(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.chars().count() <= MAX_PROMPT_EXCERPT {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .rev()
        .take(MAX_PROMPT_EXCERPT)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("...{tail}")
}

fn normalize_decision(decision: &str) -> Option<&'static str> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" | "allow" | "allowed" | "yes" | "y" => Some("approve"),
        "deny" | "denied" | "reject" | "rejected" | "no" | "n" => Some("deny"),
        _ => None,
    }
}

fn trim_recent_output(output: &mut String) {
    if output.len() <= MAX_RECENT_OUTPUT {
        return;
    }
    let keep_from = output
        .char_indices()
        .rev()
        .take(MAX_RECENT_OUTPUT)
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    *output = output[keep_from..].to_string();
}

fn strip_cli_control_sequences(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\u{1b}' {
            i += 1;
            if i < chars.len() && chars[i] == '[' {
                i += 1;
                while i < chars.len() {
                    let next = chars[i];
                    i += 1;
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::CodexApprovalTracker;

    #[test]
    fn detects_approval_prompt_and_decision_once() {
        let mut tracker = CodexApprovalTracker::default();
        let required = tracker
            .observe_output(
                "task-1",
                "sidecar-1",
                "\u{1b}[36mAllow command?\u{1b}[0m\n$ echo hi\n[y/N]\n",
                100,
            )
            .expect("approval prompt should be detected");

        assert_eq!(required["type"], "tool_approval_required");
        assert_eq!(required["approval_id"], "sidecar_tap_1");
        assert_eq!(required["tool"], "run_command");
        assert_eq!(
            required["approval_checkpoint"]["restart_recovery"]["next_action"],
            "approve_or_deny_sidecar_waiter"
        );
        assert!(tracker.has_pending());
        assert!(tracker
            .observe_output("task-1", "sidecar-1", "Allow command?\n", 101)
            .is_none());

        let decision = tracker
            .observe_decision("task-1", Some("sidecar_tap_1"), Some("approve"), 102)
            .expect("matching decision should close pending approval");
        assert_eq!(decision["type"], "tool_approval_decision");
        assert_eq!(decision["req_id"], "task-1");
        assert_eq!(decision["decision"], "approve");
        assert!(!tracker.has_pending());
    }

    #[test]
    fn ignores_mismatched_decision() {
        let mut tracker = CodexApprovalTracker::default();
        tracker
            .observe_output("task-1", "sidecar-1", "Approve command?\n[y/N]\n", 100)
            .expect("approval should be detected");

        assert!(tracker
            .observe_decision("task-1", Some("other"), Some("approve"), 101)
            .is_none());
        assert!(tracker.has_pending());
    }
}
