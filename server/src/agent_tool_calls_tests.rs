    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_modern_tool_calls() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_1",
                    "function": {
                        "name": "repo_context_task_pack",
                        "arguments": "{\"q\":\"auth flow\",\"maxChars\":12000}"
                    }
                }
            ]
        });

        let calls = extract_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "repo_context_task_pack");
        assert!(!calls[0].legacy_function_call);
        assert_eq!(calls[0].args["q"], "auth flow");
        assert_eq!(calls[0].args["maxChars"], 12000);
    }

    #[test]
    fn extracts_legacy_function_call() {
        let message = json!({
            "function_call": {
                "name": "read_file",
                "arguments": "{\"path\":\"README.md\"}"
            }
        });

        let calls = extract_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "legacy_function_call");
        assert_eq!(calls[0].name, "read_file");
        assert!(calls[0].legacy_function_call);
        assert_eq!(calls[0].args["path"], "README.md");
    }

    #[test]
    fn invalid_arguments_become_empty_object() {
        let message = json!({
            "tool_calls": [
                {
                    "function": {
                        "name": "list_dir",
                        "arguments": "{not-json"
                    }
                }
            ]
        });

        let calls = extract_tool_calls(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args, json!({}));
    }
