//! Static integration and observation contracts for portable project memory.
//!
//! This module describes what can be wired without claiming that a vendor
//! runtime, hook, or token stream was actually observed.

use serde_json::{json, Value};

pub(crate) fn manifest() -> Value {
    json!({
        "schema": "elon.project_context_capabilities.v1",
        "source_priority": [
            "current_source_and_tests",
            "binding_project_rules_and_current_adrs",
            "git_shared_reviewed_navigation_memory",
            "local_unreviewed_candidates"
        ],
        "integration_modes": [
            {
                "id": "node_managed_codex",
                "status": "available",
                "context_profile": "automatic_for_broad_tasks",
                "receipt_profile": "automatic_for_broad_tasks",
                "hook_configuration": "session_scoped_non_managed",
                "trust_required": true
            },
            {
                "id": "direct_codex_install",
                "status": "plugin_bundle_available_not_installed",
                "context_profile": "plugin_bootstrap",
                "receipt_profile": "plugin_bootstrap",
                "hook_configuration": "not_installed_globally",
                "trust_required": true
            },
            {
                "id": "other_mcp_agents",
                "status": "manual_descriptor_available",
                "governance_profile": "vendor_neutral_streamable_http",
                "context_profile": "manual",
                "receipt_profile": "manual",
                "hook_configuration": "vendor_adapter_required"
            },
            {
                "id": "codex_plugin_bundle",
                "status": "repository_bundle_available_not_installed",
                "path": "plugins/yilong-project-memory",
                "profiles": ["context", "receipt"],
                "hook_configuration": "bundled_non_managed_review_required"
            }
        ],
        "hook_lifecycle": crate::node_agent_project_memory_hook_config::capability_manifest(),
        "runtime_observation": runtime_observation_contract(),
        "official_codex_memories": {
            "relationship": "separate_local_vendor_state",
            "read_by_project_docs": false,
            "written_by_project_docs": false,
            "copied_or_backed_up_by_project_docs": false,
            "generic_import_export_contract_available": false,
            "portable_team_memory": ".elon/document-sections.json.context_memories",
            "team_rules": "AGENTS.md_and_checked_in_project_documents",
            "boundary": "Never inspect, import, copy, delete, or treat Codex private memories as project authority."
        },
        "data_minimization": {
            "source_bodies": false,
            "prompts": false,
            "chat_or_transcripts": false,
            "commands_or_tool_outputs": false,
            "private_vendor_memories": false,
            "allowed": ["bounded_summary", "topics", "relative_evidence_identity", "lifecycle_metadata", "aggregate_runtime_counters"]
        }
    })
}

fn runtime_observation_contract() -> Value {
    json!({
        "schema": "elon.project_context_runtime_observation.v1",
        "status": "ingest_adapter_available",
        "source": "codex_app_server_jsonrpc",
        "adapter": "scripts/project-memory-app-server-observer.mjs",
        "accepted_notifications": [
            "hook/started",
            "hook/completed",
            "thread/tokenUsage/updated",
            "turn/started",
            "turn/completed",
            "item/started",
            "item/completed"
        ],
        "accepted_fields": [
            "irreversible_session_fingerprint",
            "measurement_window",
            "event_counts",
            "input_tokens",
            "cached_input_tokens",
            "output_tokens",
            "selected_memory_count",
            "returned_metadata_bytes",
            "elapsed_ms"
        ],
        "measurement_windows": ["baseline_without_project_memory", "with_project_memory"],
        "derived_metrics": [
            "input_token_delta",
            "elapsed_ms_delta",
            "native_file_read_delta"
        ],
        "excluded_payloads": [
            "prompt",
            "chat",
            "transcript",
            "assistant_message",
            "tool_input",
            "tool_output",
            "source_body",
            "command_text"
        ],
        "claim_rule": "Do not claim token or time savings until matched baseline and enabled windows for the same benchmark key were observed from app-server events.",
        "not_vendor_billing": true,
        "not_total_task_tokens": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_contract_never_claims_private_memory_access_or_runtime_proof() {
        let value = manifest();
        assert_eq!(
            value["official_codex_memories"]["read_by_project_docs"],
            false
        );
        assert_eq!(
            value["runtime_observation"]["status"],
            "ingest_adapter_available"
        );
        let encoded = serde_json::to_string(&value).unwrap();
        assert!(!encoded.contains("source_body\":"));
    }
}
