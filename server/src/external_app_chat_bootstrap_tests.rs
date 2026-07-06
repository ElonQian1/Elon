    use super::*;

    #[test]
    fn voice_contract_requires_full_composer_and_server_fallback() {
        let voice = voice_contract();

        assert_eq!(
            voice["composer"]["component"],
            "com.elon.chatvoice.VoiceComposerView"
        );
        assert!(voice["androidSdk"]["publicComponents"]
            .as_array()
            .unwrap()
            .contains(&json!("VoiceComposerBootstrap")));
        assert_eq!(
            voice["composer"]["recommendedConfigApi"],
            "VoiceComposerBootstrap.applyFb2GroupChatConfig(...)"
        );
        assert_eq!(
            voice["composer"]["defaultConfig"]["asr"]["serverFallbackEnabled"],
            true
        );
        assert_eq!(
            voice["composer"]["defaultConfig"]["recordingOverlayEnabled"],
            true
        );
        assert_eq!(voice["asr"]["billing"], "free_auth_and_limits_only");
        assert!(voice["composer"]["callbacks"]
            .as_array()
            .unwrap()
            .contains(&json!("onVoiceServerFallbackStarted")));
    }

    #[test]
    fn ai_reply_contract_declares_context_and_billing_boundary() {
        let ai_reply = ai_reply_contract("fb2");

        assert_eq!(ai_reply["schema"], "external_app.ai_reply.v1");
        assert_eq!(ai_reply["billableUnit"], "ai_reply_generation");
        assert_eq!(
            ai_reply["externalContext"]["primarySource"],
            "fb2:/api/main-project/context/pack"
        );
        assert!(ai_reply["externalContext"]["queryFields"]
            .as_array()
            .unwrap()
            .contains(&json!("topic_hint")));
        assert!(ai_reply["freePreparationSteps"]
            .as_array()
            .unwrap()
            .contains(&json!("external_context_fetch")));
        assert!(ai_reply["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|trigger| trigger["name"] == "selected_message_ai_reply"));
    }

    #[test]
    fn bb64a_ai_reply_contract_uses_local_mcp_doctor() {
        let ai_reply = ai_reply_contract("bb64a");

        assert_eq!(ai_reply["app_id"], "bb64a");
        assert_eq!(
            ai_reply["externalContext"]["primarySource"],
            "bb64a:local-mcp/bb64a_doctor"
        );
        assert!(ai_reply["answerRules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or("").contains("Dangerous")));
    }

    #[test]
    fn experience_contract_keeps_voice_free_and_ai_billable() {
        let experience = experience_contract();

        assert_eq!(experience["usagePolicy"]["asr"], "free");
        assert_eq!(experience["usagePolicy"]["tts"], "free");
        assert_eq!(experience["usagePolicy"]["aiReplyGeneration"], "billable");
        assert_eq!(experience["controls"]["fullWidthHoldToTalkButton"], true);
    }

    #[test]
    fn billing_contract_separates_voice_from_ai_reply_quota() {
        let billing = billing_contract("fb2");

        assert_eq!(billing["balanceEndpoint"], "/api/me/balance");
        assert_eq!(billing["gates"]["beforeAsr"], "never_check_ai_balance");
        assert_eq!(
            billing["gates"]["beforeAiReplyGeneration"],
            "check_balance_or_trial_credit"
        );
        assert!(billing["trialCredit"]["doesNotApplyTo"]
            .as_array()
            .unwrap()
            .contains(&json!("cloud_asr_fallback")));
    }
