    use super::*;

    #[test]
    fn exposes_public_context_quality_guidance() {
        let guidance = public_context_quality_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_quality.v1");
        assert!(guidance["warning_catalog"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "missing_context_pack"));
        assert!(public_context_quality_guidance("unknown").is_none());
    }

    #[test]
    fn quality_reports_missing_contract_fields() {
        let context = json!({
            "matches": [],
            "tool_contract": null
        });
        let quality = context_quality(&context, true);

        let warnings = quality["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("missing_context_pack")));
        assert!(warnings.contains(&json!("missing_context_pack_version")));
        assert!(warnings.contains(&json!("missing_generated_at")));
        assert!(warnings.contains(&json!("missing_tool_contract")));
        assert_eq!(quality["tool_readiness"]["status"], "missing");
        assert_eq!(quality["schema"], "fb2.context_pack.v1");
    }

    #[test]
    fn match_quality_uses_today_matches_schema() {
        let quality = context_quality(
            &json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "matches": [{"id": "m1"}]
            }),
            false,
        );

        assert_eq!(quality["schema"], "fb2.today_matches.v1");
        assert_eq!(quality["tool_readiness"]["status"], "not_applicable");
        assert_eq!(quality["warnings"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn quality_promotes_budget_status_to_warning() {
        let quality = context_quality(
            &json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "context_pack_version": "fb2-chat-pack-v1",
                "context_pack": "<fb2_context_pack>large</fb2_context_pack>",
                "matches": [{"id": "m1"}],
                "tool_contract": {"tools": [{"name": "get_match_detail"}]},
                "metrics": {"budget_status": "too_large"}
            }),
            true,
        );

        let warnings = quality["warnings"].as_array().unwrap();
        assert!(warnings.contains(&json!("fb2_budget_too_large")));
    }

    #[test]
    fn quality_promotes_readiness_status_to_warning() {
        let quality = context_quality(
            &json!({
                "generated_at": "2026-06-20T16:00:00+08:00",
                "context_pack_version": "fb2-chat-pack-v1",
                "context_pack": "<fb2_context_pack>blocked</fb2_context_pack>",
                "matches": [{"id": "m1"}],
                "tool_contract": {"tools": [{"name": "get_match_detail"}]},
                "preflight_readiness": {"status": "blocked"}
            }),
            true,
        );

        assert!(quality["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("fb2_readiness_blocked")));
    }
