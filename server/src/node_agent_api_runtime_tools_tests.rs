    use super::{add_tools_to_payload, agent_response_from_tool_calls, should_retry_without_tools};
    use serde_json::json;

    #[test]
    fn payload_adds_route_b_tool_definitions() {
        let mut payload = json!({
            "model": "gpt-test",
            "messages": []
        });

        add_tools_to_payload(&mut payload);

        assert_eq!(payload["tool_choice"], "auto");
        let tools = payload["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "search_files"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "file_info"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "read_file_range"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_status"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_diff"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_log"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_show"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "download_router_status"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "download_router_configure"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "apply_patch"));
    }

    #[test]
    fn tool_calls_are_converted_to_existing_action_schema() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file_range",
                            "arguments": "{\"path\":\"src/main.rs\",\"start_line\":10,\"line_count\":20}"
                        }
                    }]
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("tool calls should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["done"], false);
        assert_eq!(agent["actions"][0]["tool"], "read_file_range");
        assert_eq!(agent["actions"][0]["tool_call_id"], "call_1");
        assert_eq!(agent["actions"][0]["path"], "src/main.rs");
        assert_eq!(agent["actions"][0]["start_line"], 10);
    }

    #[test]
    fn legacy_function_call_is_converted_to_existing_action_schema() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "function_call": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"README.md\"}"
                    }
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("legacy function call should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["done"], false);
        assert_eq!(agent["actions"][0]["tool"], "read_file");
        assert_eq!(agent["actions"][0]["tool_call_id"], "legacy_function_call");
        assert_eq!(agent["actions"][0]["path"], "README.md");
    }

    #[test]
    fn modern_tool_calls_take_precedence_over_legacy_function_call() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "list_dir",
                            "arguments": {"path":"src"}
                        }
                    }],
                    "function_call": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"README.md\"}"
                    }
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("tool calls should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["actions"].as_array().unwrap().len(), 1);
        assert_eq!(agent["actions"][0]["tool"], "list_dir");
        assert_eq!(agent["actions"][0]["tool_call_id"], "call_1");
        assert_eq!(agent["actions"][0]["path"], "src");
    }

    #[test]
    fn invalid_tool_call_arguments_fail_fast() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "read_file",
                            "arguments": "not json"
                        }
                    }]
                }
            }]
        });

        let error = agent_response_from_tool_calls(&response).unwrap_err();
        assert!(format!("{error:#}").contains("function.arguments is not JSON"));
    }

    #[test]
    fn invalid_legacy_function_call_arguments_fail_fast() {
        let response = json!({
            "choices": [{
                "message": {
                    "function_call": {
                        "name": "read_file",
                        "arguments": "not json"
                    }
                }
            }]
        });

        let error = agent_response_from_tool_calls(&response).unwrap_err();
        assert!(format!("{error:#}").contains("function.arguments is not JSON"));
    }

    #[test]
    fn tool_retry_is_limited_to_compatibility_errors() {
        assert!(should_retry_without_tools(
            reqwest::StatusCode::BAD_REQUEST,
            "Unrecognized request argument supplied: tools"
        ));
        assert!(should_retry_without_tools(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "function calling is not supported by this model"
        ));
        assert!(!should_retry_without_tools(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key"
        ));
    }
