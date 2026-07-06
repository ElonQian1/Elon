    use super::{
        ServerAgentRuntimeLimits, DUPLICATE_REQUEST_WINDOW_SECS_ENV, MAX_ACTIONS_ENV,
        MAX_ACTIONS_TOTAL_CHARS_ENV, MAX_ACTION_CHARS_ENV, MAX_CONCURRENT_GLOBAL_ENV,
        MAX_CONCURRENT_PER_USER_ENV, MAX_MESSAGES_ENV, MAX_MESSAGE_CHARS_ENV,
        MAX_OUTPUT_TOKENS_ENV, MAX_REQUESTS_PER_MINUTE_ENV, MAX_TOTAL_CHARS_ENV, TEMPERATURE_ENV,
    };
    use serde_json::json;

    #[test]
    fn accepts_normal_runtime_messages() {
        let messages = vec![
            json!({"role": "system", "content": "Return JSON"}),
            json!({"role": "user", "content": "Read README"}),
        ];

        let limits = ServerAgentRuntimeLimits::current();
        assert!(limits.max_concurrent_global >= limits.max_concurrent_per_user);
        limits.validate_messages(&messages).unwrap();
    }

    #[test]
    fn rejects_tool_role_messages() {
        let messages = vec![json!({"role": "tool", "content": "result"})];

        assert!(ServerAgentRuntimeLimits::current()
            .validate_messages(&messages)
            .is_err());
    }

    #[test]
    fn rejects_empty_messages() {
        assert!(ServerAgentRuntimeLimits::current()
            .validate_messages(&[])
            .is_err());
    }

    #[test]
    fn rejects_messages_over_operational_limits() {
        let limits = ServerAgentRuntimeLimits::current();
        let too_many = (0..=limits.max_messages)
            .map(|_| json!({"role": "user", "content": "x"}))
            .collect::<Vec<_>>();
        assert!(limits.validate_messages(&too_many).is_err());

        let too_long = vec![json!({
            "role": "user",
            "content": "x".repeat(limits.max_total_chars + 1)
        })];
        assert!(limits.validate_messages(&too_long).is_err());

        let too_long_single_message = vec![json!({
            "role": "user",
            "content": "x".repeat(limits.max_message_chars + 1)
        })];
        assert!(limits.validate_messages(&too_long_single_message).is_err());
    }

    #[test]
    fn runtime_limits_accept_operator_downscale_overrides() {
        let limits = ServerAgentRuntimeLimits::from_lookup(|name| {
            match name {
                MAX_MESSAGES_ENV => Some("8"),
                MAX_MESSAGE_CHARS_ENV => Some("6000"),
                MAX_TOTAL_CHARS_ENV => Some("12000"),
                MAX_OUTPUT_TOKENS_ENV => Some("1024"),
                MAX_ACTIONS_ENV => Some("6"),
                MAX_ACTION_CHARS_ENV => Some("8000"),
                MAX_ACTIONS_TOTAL_CHARS_ENV => Some("16000"),
                MAX_REQUESTS_PER_MINUTE_ENV => Some("3"),
                MAX_CONCURRENT_PER_USER_ENV => Some("1"),
                MAX_CONCURRENT_GLOBAL_ENV => Some("4"),
                DUPLICATE_REQUEST_WINDOW_SECS_ENV => Some("9"),
                TEMPERATURE_ENV => Some("0.1"),
                _ => None,
            }
            .map(str::to_string)
        });

        assert_eq!(limits.max_messages, 8);
        assert_eq!(limits.max_message_chars, 6_000);
        assert_eq!(limits.max_total_chars, 12_000);
        assert_eq!(limits.max_output_tokens, 1024);
        assert_eq!(limits.max_actions, 6);
        assert_eq!(limits.max_action_chars, 8_000);
        assert_eq!(limits.max_actions_total_chars, 16_000);
        assert_eq!(limits.max_requests_per_minute, 3);
        assert_eq!(limits.max_concurrent_per_user, 1);
        assert_eq!(limits.max_concurrent_global, 4);
        assert_eq!(limits.duplicate_request_window_secs, 9);
        assert_eq!(limits.temperature, 0.1);
    }

    #[test]
    fn runtime_limits_ignore_invalid_or_unsafe_operator_overrides() {
        let defaults = ServerAgentRuntimeLimits::from_lookup(|_| None);
        let limits = ServerAgentRuntimeLimits::from_lookup(|name| {
            match name {
                MAX_MESSAGES_ENV => Some("0"),
                MAX_MESSAGE_CHARS_ENV => Some("999999999"),
                MAX_TOTAL_CHARS_ENV => Some("999999999"),
                MAX_OUTPUT_TOKENS_ENV => Some("not-a-number"),
                MAX_ACTIONS_ENV => Some("999"),
                MAX_ACTION_CHARS_ENV => Some("999999999"),
                MAX_ACTIONS_TOTAL_CHARS_ENV => Some("999999999"),
                MAX_REQUESTS_PER_MINUTE_ENV => Some("0"),
                MAX_CONCURRENT_PER_USER_ENV => Some("999"),
                MAX_CONCURRENT_GLOBAL_ENV => Some("999"),
                DUPLICATE_REQUEST_WINDOW_SECS_ENV => Some("999"),
                TEMPERATURE_ENV => Some("nan"),
                _ => None,
            }
            .map(str::to_string)
        });

        assert_eq!(limits.max_messages, defaults.max_messages);
        assert_eq!(limits.max_message_chars, defaults.max_message_chars);
        assert_eq!(limits.max_total_chars, defaults.max_total_chars);
        assert_eq!(limits.max_output_tokens, defaults.max_output_tokens);
        assert_eq!(limits.max_actions, defaults.max_actions);
        assert_eq!(limits.max_action_chars, defaults.max_action_chars);
        assert_eq!(
            limits.max_actions_total_chars,
            defaults.max_actions_total_chars
        );
        assert_eq!(
            limits.max_requests_per_minute,
            defaults.max_requests_per_minute
        );
        assert_eq!(
            limits.max_concurrent_per_user,
            defaults.max_concurrent_per_user
        );
        assert_eq!(limits.max_concurrent_global, defaults.max_concurrent_global);
        assert_eq!(
            limits.duplicate_request_window_secs,
            defaults.duplicate_request_window_secs
        );
        assert_eq!(limits.temperature, defaults.temperature);
    }

    #[test]
    fn runtime_limits_allow_disabling_duplicate_request_debounce() {
        let limits = ServerAgentRuntimeLimits::from_lookup(|name| {
            (name == DUPLICATE_REQUEST_WINDOW_SECS_ENV).then(|| "0".to_string())
        });

        assert_eq!(limits.duplicate_request_window_secs, 0);
    }
