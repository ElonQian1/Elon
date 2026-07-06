    use super::{
        runtime_status_chunk, runtime_summary_chunk, tool_approval_checkpoint,
        tool_approval_required_chunk, tool_approval_required_chunk_with_diff,
        tool_approval_required_chunk_with_diff_and_checkpoint, tool_call_chunk, tool_result_chunk,
    };
    use serde_json::{json, Value};

    #[test]
    fn tool_call_event_hides_write_content_and_secrets() {
        let line = tool_call_chunk(
            "req",
            2,
            3,
            &json!({
                "tool": "write_file",
                "path": "src/main.rs",
                "content": "secret body",
                "api_key": "should-not-render",
                "tool_call_id": "call_write_1"
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_call");
        assert_eq!(event["tool"], "write_file");
        assert_eq!(event["call_id"], "call_write_1");
        assert_eq!(event["args"]["path"], "src/main.rs");
        assert_eq!(event["args"]["content_chars"], 11);
        assert!(event["args"].get("content").is_none());
        assert!(event["args"].get("api_key").is_none());
        assert!(event["args"].get("tool_call_id").is_none());
    }

    #[test]
    fn run_command_preview_keeps_structured_command() {
        let line = tool_call_chunk(
            "req",
            1,
            1,
            &json!({
                "tool": "run_command",
                "program": "git",
                "args": ["status", "--short"],
                "reason": "inspect state"
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["args"]["program"], "git");
        assert_eq!(event["args"]["args"][0], "status");
        assert_eq!(event["args"]["reason"], "inspect state");
    }

    #[test]
    fn tool_result_event_marks_guard_errors() {
        let line = tool_result_chunk(
            "req",
            1,
            2,
            "run_command",
            "error: denied",
            Some(&json!({"tool_call_id": "call_run_1"})),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_result");
        assert_eq!(event["tool"], "run_command");
        assert_eq!(event["call_id"], "call_run_1");
        assert_eq!(event["status"], "error");
    }

    #[test]
    fn runtime_summary_event_reports_tool_counts() {
        let line = runtime_summary_chunk("req", "api-runtime", 3, "ok", 5, 1, "done");
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "runtime_summary");
        assert_eq!(event["schema"], "elon.routebc.runtime_summary.v1");
        assert_eq!(event["runtime"], "api-runtime");
        assert_eq!(event["turn"], 3);
        assert_eq!(event["status"], "ok");
        assert_eq!(event["total_tools"], 5);
        assert_eq!(event["failed_tools"], 1);
    }

    #[test]
    fn runtime_status_event_reports_phase_without_tool() {
        let line = runtime_status_chunk("req", 2, "api-runtime", "thinking", "calling model");
        let event: Value = serde_json::from_str(line.trim()).unwrap();

        assert_eq!(event["type"], "runtime_status");
        assert_eq!(event["schema"], "elon.routebc.runtime_status.v1");
        assert_eq!(event["runtime"], "api-runtime");
        assert_eq!(event["phase"], "thinking");
        assert_eq!(event["status"], "running");
        assert_eq!(event["turn"], 2);
        assert!(event.get("tool").is_none());
    }

    #[test]
    fn apply_patch_preview_uses_summary_and_diff_preview() {
        let line = tool_approval_required_chunk(
            "req",
            1,
            1,
            "tap_1_1",
            &json!({
                "tool": "apply_patch",
                "patch": "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n",
                "check_only": false
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_approval_required");
        assert_eq!(event["approval_id"], "tap_1_1");
        assert_eq!(event["args"]["files"][0], "src/main.rs");
        assert_eq!(event["diff"]["files"][0], "src/main.rs");
        assert!(event["args"]["patch_sha256"].as_str().unwrap().len() >= 64);
        assert!(event["args"].get("patch").is_none());
    }

    #[test]
    fn approval_event_keeps_provider_tool_call_id_out_of_args() {
        let line = tool_approval_required_chunk(
            "req",
            1,
            1,
            "tap_1_1",
            &json!({
                "tool": "run_command",
                "tool_call_id": "call_provider_42",
                "program": "git",
                "args": ["status", "--short"],
                "reason": "inspect state"
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();

        assert_eq!(event["call_id"], "call_provider_42");
        assert!(event["args"].get("tool_call_id").is_none());
        assert_eq!(event["args"]["program"], "git");
    }

    #[test]
    fn write_file_approval_can_include_diff_without_content() {
        let line = tool_approval_required_chunk_with_diff(
            "req",
            1,
            2,
            "tap_1_2",
            &json!({
                "tool": "write_file",
                "path": "src/main.rs",
                "content": "new secret body"
            }),
            json!({
                "format": "unified",
                "source": "write_file",
                "preview": "--- a/src/main.rs\n+++ b/src/main.rs\n-old\n+new\n",
                "truncated": false,
                "files": ["src/main.rs"]
            }),
        );

        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_approval_required");
        assert_eq!(event["tool"], "write_file");
        assert_eq!(event["args"]["path"], "src/main.rs");
        assert_eq!(event["args"]["content_chars"], 15);
        assert!(event["args"].get("content").is_none());
        assert_eq!(event["diff"]["source"], "write_file");
        assert_eq!(event["diff"]["files"][0], "src/main.rs");
        assert!(event["diff"]["preview"].as_str().unwrap().contains("+new"));
    }

    #[test]
    fn approval_checkpoint_hashes_action_without_exposing_write_content() {
        let action = json!({
            "tool": "write_file",
            "path": "src/main.rs",
            "content": "new secret body"
        });
        let diff = json!({
            "format": "unified",
            "source": "write_file",
            "preview": "--- a/src/main.rs\n+++ b/src/main.rs\n-old\n+new secret body\n",
            "truncated": false,
            "files": ["src/main.rs"],
            "new_sha256": "b".repeat(64)
        });
        let checkpoint = tool_approval_checkpoint(&action, &diff, 100, 200);
        let checkpoint_text = serde_json::to_string(&checkpoint).unwrap();

        assert_eq!(
            checkpoint["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert_eq!(checkpoint["registered_at_ms"], 100);
        assert_eq!(checkpoint["expires_at_ms"], 200);
        assert_eq!(
            checkpoint["restart_recovery"]["next_action"],
            "continue_from_snapshot"
        );
        assert_eq!(
            checkpoint["restart_recovery"]["supported"].as_bool(),
            Some(false)
        );
        assert!(checkpoint["action_sha256"].as_str().unwrap().len() >= 64);
        assert!(checkpoint["diff_sha256"].as_str().unwrap().len() >= 64);
        assert_eq!(
            checkpoint["diff_fingerprint"]["preview_removed"].as_bool(),
            Some(true)
        );
        assert!(!checkpoint_text.contains("new secret body"));

        let line = tool_approval_required_chunk_with_diff_and_checkpoint(
            "req", 1, 2, "tap_1_2", &action, diff, checkpoint,
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(
            event["approval_checkpoint"]["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert!(!serde_json::to_string(&event["approval_checkpoint"])
            .unwrap()
            .contains("new secret body"));
    }
