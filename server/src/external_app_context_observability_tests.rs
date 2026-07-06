    use super::*;

    #[test]
    fn exposes_public_context_observability_guidance() {
        let guidance = public_context_observability_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_observability.v1");
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "external_tool_grounding"));
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "non_synthetic_feedback_count"));
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "opinion_adoption_count"));
        assert!(guidance["recommended_log_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("opinion_memory_ref_count")));
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "topic_hint_present"));
        assert!(guidance["recommended_log_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("context_quality_warning_count")));
        assert_eq!(
            guidance["main_project_persistence"]["table"],
            "external_app_tool_executions"
        );
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "citation_coverage"));
        assert!(guidance["recommended_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["name"] == "topic_hint_present"));
        assert!(guidance["recommended_log_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("answer_policy_schema")));
        assert!(guidance["recommended_log_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("context_quality_warning_count")));
        assert!(guidance["privacy_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or_default().contains("shared secret")));
        assert!(public_context_observability_guidance("unknown").is_none());
    }
