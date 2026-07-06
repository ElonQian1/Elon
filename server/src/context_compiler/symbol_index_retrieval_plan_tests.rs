    use super::*;

    #[test]
    fn debug_error_plan_prefers_full_text_graph_and_error_context() {
        let plan = build_retrieval_plan("登录失败为什么返回 500？", false);

        assert_eq!(plan.intent, QueryIntent::DebugError);
        assert!(plan.features.status_codes.contains(&500));
        assert!(plan.graph_policy.include_error_mappers);
        assert!(plan.pack_policy.include_tests);
        assert!(plan.weights.full_text > plan.weights.symbol);
        assert!(!plan.retrievers.vector);
    }

    #[test]
    fn refactor_plan_uses_deeper_references_policy() {
        let plan = build_retrieval_plan("重构 AuthService::login callers", false);

        assert_eq!(plan.intent, QueryIntent::Refactor);
        assert!(plan
            .features
            .symbol_like_terms
            .contains(&"AuthService::login".to_string()));
        assert!(plan.graph_policy.include_references);
        assert!(plan.graph_policy.include_implementations);
        assert_eq!(plan.graph_policy.max_depth, 2);
    }

    #[test]
    fn plan_defaults_drive_depth_and_limits_but_respect_explicit_values() {
        let plan = build_retrieval_plan("重构 AuthService::login callers", false);

        assert_eq!(plan.planned_graph_depth(0), 2);
        assert_eq!(plan.planned_graph_depth(1), 1);
        assert_eq!(plan.planned_limit(0, 8, 20, "graph"), 12);
        assert_eq!(plan.planned_limit(5, 8, 20, "graph"), 5);
    }

    #[test]
    fn render_plan_exposes_selected_strategy() {
        let plan = build_retrieval_plan("新增 refresh token", true);
        let rendered = render_retrieval_plan(&plan);

        assert_eq!(plan.intent, QueryIntent::AddFeature);
        assert!(plan.weights.vector > 0.0);
        assert!(rendered.contains("<retrieval_plan intent=\"add_feature\">"));
        assert!(rendered.contains("vector=on"));
        assert!(rendered.contains("pack_policy"));
    }
