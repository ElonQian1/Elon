    use super::{validate_server_runtime_output, ServerRuntimeOutputError};
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use serde_json::json;

    #[test]
    fn accepts_message_only_or_small_actions() {
        let limits = test_limits();

        validate_server_runtime_output(&json!({"message": "ok"}), limits).unwrap();
        validate_server_runtime_output(
            &json!({
                "message": "ok",
                "actions": [
                    {"tool": "read_file", "path": "README.md"},
                    {"tool": "run_command", "program": "git", "args": ["status", "--short"]}
                ]
            }),
            limits,
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_object_or_malformed_actions() {
        let limits = test_limits();

        assert_eq!(
            validate_server_runtime_output(&json!("oops"), limits).unwrap_err(),
            ServerRuntimeOutputError::ResponseNotObject
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": "oops"}), limits).unwrap_err(),
            ServerRuntimeOutputError::ActionsNotArray
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": ["oops"]}), limits).unwrap_err(),
            ServerRuntimeOutputError::ActionNotObject { index: 0 }
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": [{"path": "README.md"}]}), limits)
                .unwrap_err(),
            ServerRuntimeOutputError::MissingTool { index: 0 }
        );
    }

    #[test]
    fn rejects_action_budget_overflow_without_logging_payload() {
        let limits = test_limits();
        let too_many = json!({
            "actions": [
                {"tool": "read_file", "path": "a"},
                {"tool": "read_file", "path": "b"},
                {"tool": "read_file", "path": "c"}
            ]
        });

        assert_eq!(
            validate_server_runtime_output(&too_many, limits).unwrap_err(),
            ServerRuntimeOutputError::TooManyActions { count: 3, max: 2 }
        );

        let too_large = json!({"actions": [{"tool": "write_file", "content": "x".repeat(80)}]});
        let error = validate_server_runtime_output(&too_large, limits).unwrap_err();
        assert!(matches!(
            error,
            ServerRuntimeOutputError::ActionTooLarge { index: 0, .. }
        ));
        assert!(!error.public_message().contains('x'));
    }

    fn test_limits() -> ServerAgentRuntimeLimits {
        ServerAgentRuntimeLimits {
            max_messages: 8,
            max_message_chars: 1_000,
            max_total_chars: 2_000,
            max_output_tokens: 512,
            max_actions: 2,
            max_action_chars: 72,
            max_actions_total_chars: 120,
            max_requests_per_minute: 4,
            max_concurrent_per_user: 1,
            max_concurrent_global: 8,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        }
    }
