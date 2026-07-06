    use super::*;
    use serde_json::Value;

    #[test]
    fn stream_event_emits_tool_call_and_progress_for_command_started() {
        let line = r#"{"type":"item.started","item":{"id":"call_1","type":"command_execution","command":"cargo check"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 2);
        let tool: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "shell");
        assert_eq!(tool["args"]["command"], "cargo check");
        assert_eq!(tool["args"]["id"], "call_1");
        let progress: Value = serde_json::from_str(&msgs[1]).unwrap();
        assert_eq!(progress["type"], "progress");
        assert!(progress["message"]
            .as_str()
            .unwrap()
            .contains("cargo check"));
    }

    #[test]
    fn stream_event_emits_tool_result_for_command_completed() {
        let line = r#"{"type":"item.completed","item":{"id":"call_1","type":"command_execution","command":"cargo check","exit_code":0,"aggregated_output":"Compiling foo\nFinished","status":"completed"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 2);
        let tool: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(tool["type"], "tool_result");
        assert_eq!(tool["tool"], "shell");
        assert!(tool["result"].as_str().unwrap().contains("exit=0"));
        assert!(tool["result"].as_str().unwrap().contains("Compiling foo"));
    }

    #[test]
    fn stream_event_emits_tool_call_and_result_for_file_change() {
        let started = r#"{"type":"item.started","item":{"id":"fc_1","type":"file_change","changes":[{"path":"src/main.rs","kind":"modify"}]}}"#;
        let completed = r#"{"type":"item.completed","item":{"id":"fc_1","type":"file_change","status":"applied","summary":"1 file changed"}}"#;
        let s = stream_event_to_ws_messages(started, None);
        assert_eq!(s.len(), 2);
        let tool: Value = serde_json::from_str(&s[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "file_change");
        assert!(tool["args"]["changes"].is_array());

        let c = stream_event_to_ws_messages(completed, None);
        let result: Value = serde_json::from_str(&c[0]).unwrap();
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool"], "file_change");
        assert!(result["result"].as_str().unwrap().contains("applied"));
    }

    #[test]
    fn stream_event_emits_usage_event_for_token_count() {
        let line = r#"{"type":"token_count","model":"gpt-5-codex","usage":{"input_tokens":1200,"output_tokens":350,"total_tokens":1550,"cached_input_tokens":800,"total_cost_usd":0.0123}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let usage: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(usage["type"], "usage");
        assert_eq!(usage["input_tokens"], 1200);
        assert_eq!(usage["output_tokens"], 350);
        assert_eq!(usage["total_tokens"], 1550);
        assert_eq!(usage["cached_input_tokens"], 800);
        assert_eq!(usage["total_cost_usd"], 0.0123);
        assert_eq!(usage["model"], "gpt-5-codex");
    }

    #[test]
    fn stream_event_emits_usage_event_for_turn_completed_with_usage() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":20}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let usage: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(usage["type"], "usage");
        assert_eq!(usage["input_tokens"], 10);
        assert_eq!(usage["output_tokens"], 20);
    }

    #[test]
    fn stream_event_ignores_blank_and_unknown_events() {
        assert!(stream_event_to_ws_messages("", None).is_empty());
        assert!(stream_event_to_ws_messages("not json", None).is_empty());
        assert!(stream_event_to_ws_messages(r#"{"type":"unknown_event"}"#, None).is_empty());
    }

    #[test]
    fn stream_event_emits_assistant_message_for_agent_message_completed() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"  我已经读完了 main.rs，准备开始改造。  "}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let value: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(value["type"], "assistant_message");
        assert_eq!(value["text"], "我已经读完了 main.rs，准备开始改造。");
    }

    #[test]
    fn stream_event_strips_yonghu_kejian_prefix() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：正在读取 main.rs，马上修改。"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let value: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(value["type"], "assistant_message");
        assert_eq!(value["text"], "正在读取 main.rs，马上修改。");
    }

    #[test]
    fn stream_event_skips_blank_after_prefix_strip() {
        let line =
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：   "}}"#;
        assert!(stream_event_to_ws_messages(line, None).is_empty());
    }

    #[test]
    fn stream_event_skips_blank_agent_message() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"   "}}"#;
        assert!(stream_event_to_ws_messages(line, None).is_empty());
    }
