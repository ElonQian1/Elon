//! Machine-readable Desktop supervision feature negotiation.

use serde_json::{json, Value};

pub(crate) const DELTA_WAIT_V1: &str = "delta_wait_v1";
pub(crate) const RESUME_CONTEXT_V1: &str = "resume_context_v1";
pub(crate) const CONTRACT_SUPERSEDE_V1: &str = "contract_supersede_v1";
pub(crate) const DESKTOP_REVIEW_TICKET_V2: &str = "desktop_review_ticket_v2";
pub(crate) const DESKTOP_REVIEW_TICKET_V3: &str = "desktop_review_ticket_v3";
pub(crate) const CODEX_TERMINAL_HINT_V1: &str = "codex_terminal_hint_v1";

const CAPABILITIES: &[&str] = &[
    DELTA_WAIT_V1,
    RESUME_CONTEXT_V1,
    CONTRACT_SUPERSEDE_V1,
    DESKTOP_REVIEW_TICKET_V2,
    DESKTOP_REVIEW_TICKET_V3,
    CODEX_TERMINAL_HINT_V1,
];

pub(crate) fn status_payload() -> Value {
    json!({
        "protocol": crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL,
        "schemaVersion": 1,
        "capabilities": CAPABILITIES,
        "contracts": {
            "taskDelta": "elon.supervision.task_delta.v1",
            "resumeContext": "elon.resume_context.v1",
            "contractRevision": "elon.supervision.contract_revision.v1",
            "desktopReviewTicket": ["elon.desktop_review_ticket.v3", "elon.desktop_review_ticket.v2"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_advertises_all_reliability_contracts() {
        let payload = status_payload();
        let capabilities = payload["capabilities"].as_array().unwrap();
        for required in [
            DELTA_WAIT_V1,
            RESUME_CONTEXT_V1,
            CONTRACT_SUPERSEDE_V1,
            DESKTOP_REVIEW_TICKET_V2,
            DESKTOP_REVIEW_TICKET_V3,
            CODEX_TERMINAL_HINT_V1,
        ] {
            assert!(capabilities.iter().any(|value| value == required));
        }
        assert!(!capabilities
            .iter()
            .any(|value| value == "desktop_review_ticket_v1"));
        assert_eq!(
            payload["contracts"]["contractRevision"],
            "elon.supervision.contract_revision.v1"
        );
        assert_eq!(
            payload["contracts"]["desktopReviewTicket"],
            json!([
                "elon.desktop_review_ticket.v3",
                "elon.desktop_review_ticket.v2"
            ])
        );
    }
}
