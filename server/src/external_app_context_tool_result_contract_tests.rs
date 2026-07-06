    use super::*;

    fn contains_str(array: &Value, expected: &str) -> bool {
        array.as_array().unwrap().contains(&json!(expected))
    }

    #[test]
    fn exposes_fb2_tool_result_envelope_contract() {
        let contract = public_tool_result_envelope_guidance("fb2").unwrap();
        assert_eq!(contract["schema"], "fb2.tool_result_envelope.v1");
        assert_eq!(
            contract["normalized_result_schema"],
            "external_app.normalized_tool_result.v1"
        );

        let fields = &contract["normalized_envelope"]["required_fields"];
        for field in ["schema", "source_ids", "visibility", "grounding"] {
            assert!(contains_str(fields, field));
        }

        let business_kinds = &contract["source_registry"]["business_source_kinds"];
        assert!(contains_str(business_kinds, "match"));
        assert!(contains_str(business_kinds, "user_order"));
        assert!(contains_str(business_kinds, "platform_order_summary"));
        assert!(!contains_str(business_kinds, "feedback"));

        let quality_kinds = contract["source_registry"]["quality_history_kinds"]
            .as_array()
            .unwrap();
        assert!(quality_kinds.iter().any(|kind| kind["kind"] == "feedback"
            && kind["scope"] == "quality_history"
            && kind["default_chat_fact"] == false));
        assert_eq!(
            contract["answer_source_validation"]["schema"],
            "external_app.answer_source_validation.v1"
        );
        assert!(contract["answer_source_validation"]["rule"]
            .as_str()
            .unwrap()
            .contains("matched_tool_source_ids"));
        assert!(contract["answer_source_validation"]["rule"]
            .as_str()
            .unwrap()
            .contains("has_missing_explicit_sources"));
    }

    #[test]
    fn exposes_grounding_status_and_visibility_rules() {
        let contract = public_tool_result_envelope_guidance("fb2").unwrap();
        let statuses = contract["grounding"]["statuses"].as_array().unwrap();
        for status in ["grounded", "weak", "unsafe", "unavailable"] {
            assert!(statuses.iter().any(|item| item["status"] == status));
        }
        assert!(statuses
            .iter()
            .any(|item| item["status"] == "unsafe" && item["facts_allowed"] == false));

        let visibility = contract["visibility_contract"].as_array().unwrap();
        assert!(visibility.iter().any(|item| {
            item["expected_visibility"] == "current_user_only"
                && item["source_ids_required"] == true
                && item["tools"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("search_user_orders"))
        }));
        assert!(visibility.iter().any(|item| {
            item["expected_visibility"] == "privileged_summary"
                && item["tools"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("platform_orders"))
        }));
    }
