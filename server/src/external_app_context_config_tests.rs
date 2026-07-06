    use super::*;

    #[test]
    fn infers_fb2_lottery_type_from_topic_hint() {
        assert_eq!(
            infer_lottery_type(Some("今天竞彩怎么看")),
            Some("JingCai".into())
        );
        assert_eq!(infer_lottery_type(Some("北单赛事")), Some("BeiDan".into()));
        assert_eq!(infer_lottery_type(Some("足球比赛")), None);
    }

    #[test]
    fn detects_platform_order_summary_intent() {
        assert!(platform_order_summary_requested(Some(
            "平台今天订单风险怎么样？只用匿名汇总"
        )));
        assert!(platform_order_summary_requested(Some(
            "全站投注集中在哪些方向"
        )));
        assert!(!platform_order_summary_requested(Some(
            "群里大家怎么看西班牙这场？只说群友观点和AI推断，不要平台订单汇总。"
        )));
        assert!(!platform_order_summary_requested(Some(
            "今天比赛怎么看？不需要平台订单汇总"
        )));
        assert!(!platform_order_summary_requested(Some(
            "帮我分析我的票有什么风险"
        )));
        assert!(!platform_order_summary_requested(Some(
            "群里大家怎么看西班牙这场"
        )));
        assert!(!platform_order_summary_requested(Some("这条消息说得对吗")));
    }

    #[test]
    fn env_flag_defaults_when_missing() {
        assert!(env_flag("__ELON_TEST_MISSING_FLAG__", true));
        assert!(!env_flag("__ELON_TEST_MISSING_FLAG__", false));
    }

    #[test]
    fn defaults_use_compact_chat_context_budget() {
        assert_eq!(DEFAULT_MATCH_LIMIT, 3);
        assert_eq!(DEFAULT_DISCUSSION_LIMIT, 6);
        assert_eq!(DEFAULT_ORDER_LIMIT, 2);
    }

    #[test]
    fn builds_fb2_permission_headers_for_user_and_platform_scope() {
        assert_eq!(
            fb2_request_context_headers(Some("  user-1  "), true),
            vec![
                (FB2_CONTEXT_USER_ID_HEADER, "user-1".to_string()),
                (
                    FB2_CONTEXT_SCOPE_HEADER,
                    FB2_PLATFORM_ORDER_SUMMARY_SCOPE.to_string()
                )
            ]
        );
        assert_eq!(fb2_request_context_headers(Some(""), false), Vec::new());
    }
