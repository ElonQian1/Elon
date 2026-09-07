use super::enforce_status_response_limit;
use serde_json::{json, Value};

fn oversized_recovery() -> Value {
    json!({
        "local_admin_token_header": "X-Elon-Local-Admin-Token",
        "active_cli_prompt_count": 1,
        "active_cli_prompt_task_ids": ["prompt-a"],
        "active_task_runtime": [{"task_id": "runtime-b", "runtime": {"detail": "x".repeat(80_000)}}],
        "update_recovery": {"install_gate": {"active_foreground_task_ids": ["x".repeat(80_000)]}},
    })
}

fn assert_bounded(value: &Value) {
    assert!(serde_json::to_vec(value).unwrap().len() <= super::ADMIN_STATUS_MAX_BYTES);
}

#[test]
fn recovery_compaction_preserves_complete_prompt_and_handle_identities() {
    let mut status = oversized_recovery();
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert_eq!(status["active_cli_prompt_count"], 1);
    assert_eq!(status["active_cli_prompt_task_ids"], json!(["prompt-a"]));
    assert_eq!(
        status["active_task_runtime"],
        json!([{"task_id": "runtime-b"}])
    );
    assert_eq!(
        status["local_admin_token_header"],
        "X-Elon-Local-Admin-Token"
    );
}

#[test]
fn genuinely_idle_snapshot_retains_explicit_empty_arrays() {
    let mut status = oversized_recovery();
    status["active_cli_prompt_count"] = json!(0);
    status["active_cli_prompt_task_ids"] = json!([]);
    status["active_task_runtime"] = json!([]);
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert_eq!(status["active_cli_prompt_count"], 0);
    assert_eq!(status["active_cli_prompt_task_ids"], json!([]));
    assert_eq!(status["active_task_runtime"], json!([]));
}

#[test]
fn absent_identity_evidence_is_not_synthesized_as_idle() {
    let mut status = oversized_recovery();
    status
        .as_object_mut()
        .unwrap()
        .remove("active_cli_prompt_task_ids");
    status
        .as_object_mut()
        .unwrap()
        .remove("active_task_runtime");
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert!(status["active_cli_prompt_task_ids"].as_array().is_none());
    assert!(status["active_task_runtime"].as_array().is_none());
    assert_eq!(status["active_cli_prompt_count"], 1);
}

#[test]
fn malformed_runtime_entry_is_not_dropped_or_given_an_identity() {
    let mut status = oversized_recovery();
    status["active_task_runtime"] = json!([{"runtime": "large"}, {"task_id": 17}]);
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    let entries = status["active_task_runtime"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0]["task_id"].as_str().is_none());
    assert_eq!(entries[1]["task_id"], 17);
}

#[test]
fn non_array_runtime_evidence_remains_invalid() {
    let mut status = oversized_recovery();
    status["active_task_runtime"] = json!("not-an-array");
    enforce_status_response_limit(&mut status);
    assert_eq!(status["active_task_runtime"], "not-an-array");
}

#[test]
fn inconsistent_count_is_not_repaired_by_compaction() {
    let mut status = oversized_recovery();
    status["active_cli_prompt_task_ids"] = json!([]);
    enforce_status_response_limit(&mut status);
    assert_eq!(status["active_cli_prompt_count"], 1);
    assert_eq!(status["active_cli_prompt_task_ids"], json!([]));
}

#[test]
fn oversized_identities_are_unavailable_not_a_truncated_allow_list() {
    let mut status = oversized_recovery();
    status["active_cli_prompt_task_ids"] = json!(["x".repeat(100_000)]);
    status["active_task_runtime"] = json!([{ "task_id": "y".repeat(100_000) }]);
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert!(status["active_cli_prompt_task_ids"].as_array().is_none());
    assert!(status["active_task_runtime"].as_array().is_none());
    assert_eq!(status["status"], "compacted");
}

#[test]
fn final_fallback_stays_bounded_even_when_bootstrap_is_oversized() {
    let mut status = oversized_recovery();
    status["compute_plugin_bootstrap"] = json!({"unexpected": "x".repeat(100_000)});
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert_eq!(status["status"], "compacted");
    assert!(status["active_cli_prompt_task_ids"].as_array().is_none());
}

#[test]
fn model_compaction_does_not_replace_small_runtime_details() {
    let runtime = json!([{"task_id": "runtime-b", "runtime": {"status": "running"}}]);
    let mut status = json!({
        "models": [{"description": "x".repeat(80_000)}],
        "active_cli_prompt_count": 1,
        "active_cli_prompt_task_ids": ["prompt-a"],
        "active_task_runtime": runtime.clone(),
    });
    enforce_status_response_limit(&mut status);
    assert_bounded(&status);
    assert_eq!(status["active_task_runtime"], runtime);
    assert_eq!(status["active_cli_prompt_task_ids"], json!(["prompt-a"]));
}
