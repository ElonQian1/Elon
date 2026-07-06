    use super::{
        admission_availability, admission_snapshot, audit_summary, operational_error_summary,
        protection_status, try_acquire_runtime_admission_for_request, ServerRuntimeAdmissionError,
        ServerRuntimeAdmissionSnapshot,
    };
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn audit_summary_uses_shape_not_prompt_text() {
        let limits = ServerAgentRuntimeLimits::current();
        let left = vec![json!({"role": "user", "content": "secret prompt A"})];
        let right = vec![json!({"role": "user", "content": "secret prompt B"})];

        let left_summary = audit_summary(&left, limits);
        let right_summary = audit_summary(&right, limits);

        assert_eq!(left_summary.message_count, 1);
        assert_eq!(left_summary.total_chars, "secret prompt A".chars().count());
        assert_eq!(
            left_summary.limit_max_message_chars,
            limits.max_message_chars
        );
        assert_eq!(left_summary.roles, vec!["user"]);
        assert_ne!(
            left_summary.request_fingerprint,
            right_summary.request_fingerprint
        );
        let serialized = serde_json::to_string(&left_summary).unwrap();
        assert!(!serialized.contains("secret prompt"));
    }

    #[test]
    fn fingerprint_changes_when_shape_changes() {
        let limits = ServerAgentRuntimeLimits::current();
        let one = audit_summary(&[json!({"role": "user", "content": "abc"})], limits);
        let two = audit_summary(&[json!({"role": "assistant", "content": "abc"})], limits);
        let three = audit_summary(&[json!({"role": "user", "content": "abcd"})], limits);

        assert_ne!(one.request_fingerprint, two.request_fingerprint);
        assert_ne!(one.request_fingerprint, three.request_fingerprint);
    }

    #[test]
    fn status_describes_operational_protections() {
        let status = protection_status();
        assert!(status.input_validation.contains("total_chars"));
        assert!(status.output_validation.contains("actions"));
        assert!(status
            .agent_selection
            .contains("ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"));
        assert!(status.admission_control.contains("global"));
        assert!(status.admission_control.contains("concurrency"));
        assert!(status
            .duplicate_request_debounce
            .contains("ELON_SERVER_AGENT_RUNTIME_DUPLICATE_WINDOW_SECS"));
        assert!(status
            .budget_gate
            .contains("ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT"));
        assert!(status
            .budget_gate
            .contains("ELON_SERVER_AGENT_RUNTIME_PER_USER_DAILY_CALL_LIMIT"));
        assert!(status
            .operational_switch
            .contains("ELON_SERVER_AGENT_RUNTIME_ENABLED"));
        assert!(status.billing_gate.contains("call_chat_llm"));
        assert!(status.audit.contains("fingerprint"));
    }

    #[test]
    fn operational_error_summary_omits_error_body() {
        let body = "provider returned secret-token and user prompt text";
        let summary = operational_error_summary(body);

        assert!(summary.contains("provider_error"));
        assert!(summary.contains("chars="));
        assert!(summary.contains("fingerprint="));
        assert!(!summary.contains("secret-token"));
        assert!(!summary.contains("user prompt text"));
    }

    #[test]
    fn admission_gate_limits_per_user_concurrency() {
        let user_id = unique_user("concurrent");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 1,
            max_concurrent_global: 10,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };

        let first = try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap();
        let second = try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap_err();
        assert_eq!(
            second,
            ServerRuntimeAdmissionError::TooManyConcurrent {
                max_concurrent_per_user: 1
            }
        );

        drop(first);
        let after_release =
            try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap();
        drop(after_release);
    }

    #[test]
    fn admission_gate_limits_per_user_rate() {
        let user_id = unique_user("rate");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 1,
            max_concurrent_per_user: 10,
            max_concurrent_global: 10,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };

        let first = try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap();
        drop(first);
        let second = try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap_err();
        assert!(matches!(
            second,
            ServerRuntimeAdmissionError::RateLimited {
                max_requests_per_minute: 1,
                retry_after_secs: 1..=60
            }
        ));
        assert!(second.public_message().contains("每分钟最多 1 次"));
    }

    #[test]
    fn admission_gate_limits_global_concurrency_across_users() {
        let open_limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: usize::MAX,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };
        let capped_limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: 1,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };

        let first_user = unique_user("global-a");
        let second_user = unique_user("global-b");
        let first =
            try_acquire_runtime_admission_for_request(&first_user, open_limits, None).unwrap();
        let second = try_acquire_runtime_admission_for_request(&second_user, capped_limits, None)
            .unwrap_err();
        assert_eq!(
            second,
            ServerRuntimeAdmissionError::TooManyGlobalConcurrent {
                max_concurrent_global: 1
            }
        );
        assert!(second.public_message().contains("全局任务过多"));

        drop(first);
        let after_release =
            try_acquire_runtime_admission_for_request(&second_user, open_limits, None).unwrap();
        drop(after_release);
    }

    #[test]
    fn admission_snapshot_reports_current_user_capacity() {
        let user_id = unique_user("snapshot");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 1,
            max_concurrent_per_user: 1,
            max_concurrent_global: 10,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };

        let guard = try_acquire_runtime_admission_for_request(&user_id, limits, None).unwrap();
        let snapshot = admission_snapshot(&user_id, limits);
        assert_eq!(snapshot.in_flight_for_user, 1);
        assert_eq!(snapshot.remaining_concurrent_for_user, 0);
        assert_eq!(snapshot.recent_requests_per_minute, 1);
        assert_eq!(snapshot.remaining_requests_per_minute, 0);
        assert!(matches!(snapshot.rate_limit_retry_after_secs, Some(1..=60)));
        assert_eq!(snapshot.duplicate_request_window_secs, 5);
        assert_eq!(snapshot.recent_duplicate_fingerprints, 0);
        assert!(!serde_json::to_string(&snapshot).unwrap().contains(&user_id));

        drop(guard);
        let released = admission_snapshot(&user_id, limits);
        assert_eq!(released.in_flight_for_user, 0);
    }

    #[test]
    fn admission_availability_reports_capacity_reason() {
        let mut snapshot = ServerRuntimeAdmissionSnapshot {
            in_flight_global: 0,
            max_concurrent_global: 24,
            remaining_concurrent_global: 24,
            in_flight_for_user: 0,
            max_concurrent_per_user: 2,
            remaining_concurrent_for_user: 2,
            recent_requests_per_minute: 0,
            max_requests_per_minute: 12,
            remaining_requests_per_minute: 12,
            rate_limit_retry_after_secs: None,
            duplicate_request_window_secs: 5,
            recent_duplicate_fingerprints: 0,
        };

        assert!(admission_availability(&snapshot).ready);

        snapshot.remaining_concurrent_global = 0;
        let global = admission_availability(&snapshot);
        assert!(!global.ready);
        assert_eq!(global.reason, Some("global_concurrency_limited"));

        snapshot.remaining_concurrent_global = 1;
        snapshot.remaining_concurrent_for_user = 0;
        let user = admission_availability(&snapshot);
        assert!(!user.ready);
        assert_eq!(user.reason, Some("user_concurrency_limited"));

        snapshot.remaining_concurrent_for_user = 1;
        snapshot.remaining_requests_per_minute = 0;
        snapshot.rate_limit_retry_after_secs = Some(17);
        let rate = admission_availability(&snapshot);
        assert!(!rate.ready);
        assert_eq!(rate.reason, Some("rate_limited"));
        assert_eq!(rate.retry_after_secs, Some(17));
    }

    #[test]
    fn admission_error_exposes_retry_after_for_clients() {
        let rate_limited = ServerRuntimeAdmissionError::RateLimited {
            max_requests_per_minute: 1,
            retry_after_secs: 17,
        };
        assert_eq!(rate_limited.retry_after_secs(), 17);

        let concurrent = ServerRuntimeAdmissionError::TooManyConcurrent {
            max_concurrent_per_user: 1,
        };
        assert_eq!(concurrent.retry_after_secs(), 1);

        let duplicate = ServerRuntimeAdmissionError::DuplicateRecent {
            retry_after_secs: 5,
        };
        assert_eq!(duplicate.retry_after_secs(), 5);
        assert!(duplicate.public_message().contains("相同请求"));
    }

    #[test]
    fn admission_gate_debounces_duplicate_request_fingerprints() {
        let user_id = unique_user("duplicate");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: 10,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        };

        let first =
            try_acquire_runtime_admission_for_request(&user_id, limits, Some("fingerprint-a"))
                .unwrap();
        let duplicate =
            try_acquire_runtime_admission_for_request(&user_id, limits, Some("fingerprint-a"))
                .unwrap_err();
        assert!(matches!(
            duplicate,
            ServerRuntimeAdmissionError::DuplicateRecent {
                retry_after_secs: 1..=5
            }
        ));

        let snapshot = admission_snapshot(&user_id, limits);
        assert_eq!(snapshot.recent_duplicate_fingerprints, 1);

        let distinct =
            try_acquire_runtime_admission_for_request(&user_id, limits, Some("fingerprint-b"))
                .unwrap();
        drop(distinct);
        drop(first);
    }

    #[test]
    fn admission_gate_can_disable_duplicate_request_debounce() {
        let user_id = unique_user("duplicate-disabled");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: 10,
            duplicate_request_window_secs: 0,
            temperature: 0.2,
        };

        let first =
            try_acquire_runtime_admission_for_request(&user_id, limits, Some("fingerprint-a"))
                .unwrap();
        let second =
            try_acquire_runtime_admission_for_request(&user_id, limits, Some("fingerprint-a"))
                .unwrap();
        drop(second);
        drop(first);
    }

    fn unique_user(label: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("user-{label}-{nanos}")
    }
