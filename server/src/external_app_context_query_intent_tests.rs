    use super::public_context_query_intent_guidance;
    use serde_json::{json, Value};

    fn values(value: &Value, field: &str) -> Vec<String> {
        value[field]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect()
    }

    #[test]
    fn exposes_fb2_context_query_intent_contract() {
        let contract = public_context_query_intent_guidance("fb2").unwrap();

        assert_eq!(contract["schema"], "fb2.context_query_intent.v1");
        assert_eq!(contract["complete"], true);
        assert_eq!(contract["stores_fb2_business_data_in_main_project"], false);
        assert_eq!(contract["scenario_count"], 7);

        let required_fields = values(&contract["request_shape"], "required_fields");
        for field in [
            "query_intent_id",
            "entrypoint",
            "scenario_id",
            "group_id",
            "topic_hint",
            "intent_lanes",
            "requested_indexes",
            "permission_scope",
            "source_request",
            "output_limits",
        ] {
            assert!(required_fields.contains(&field.to_string()));
        }

        let scenarios = contract["scenario_intents"].as_array().unwrap();
        for scenario_id in [
            "today_matches_analysis",
            "my_ticket_analysis",
            "platform_order_risk",
            "group_opinion_summary",
            "selected_message_review",
            "group_discussion_summary_post",
            "source_reference_audit",
        ] {
            assert!(scenarios
                .iter()
                .any(|scenario| scenario["scenario_id"] == json!(scenario_id)));
        }

        let ticket = scenarios
            .iter()
            .find(|scenario| scenario["scenario_id"] == "my_ticket_analysis")
            .unwrap();
        assert_eq!(ticket["permission_scope"], "current_user_only");
        assert!(ticket["requested_indexes"]
            .as_array()
            .unwrap()
            .contains(&json!("current_user_ticket_index")));
        assert!(ticket["required_headers"]
            .as_array()
            .unwrap()
            .contains(&json!("X-FB2-AI-CONTEXT-USER-ID")));

        let privacy_rules = values(&contract, "privacy_rules").join("\n");
        assert!(privacy_rules.contains("raw group message bodies"));
        assert!(public_context_query_intent_guidance("unknown").is_none());
    }
