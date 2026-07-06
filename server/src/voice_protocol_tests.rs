    use super::{
        resolve_authenticated_voice_user, ClientControl, VOICE_TARGET_EXTERNAL_GROUP,
        VOICE_TARGET_SOCIAL_AI_DIRECT, VOICE_TARGET_TRANSCRIBE_ONLY,
    };

    #[test]
    fn hello_target_is_backward_compatible() {
        let old_client = r#"{
            "type":"hello",
            "user_id":"usr_1",
            "sample_rate":24000,
            "channels":1
        }"#;
        let parsed: ClientControl = serde_json::from_str(old_client).expect("old hello parses");
        let ClientControl::Hello { target, .. } = parsed else {
            panic!("expected hello");
        };
        assert_eq!(target, None);
    }

    #[test]
    fn hello_accepts_social_ai_direct_target() {
        let social_ai_client = format!(
            r#"{{
                "type":"hello",
                "user_id":"usr_1",
                "target":"{}",
                "sample_rate":24000,
                "channels":1
            }}"#,
            VOICE_TARGET_SOCIAL_AI_DIRECT
        );
        let parsed: ClientControl =
            serde_json::from_str(&social_ai_client).expect("social ai hello parses");
        let ClientControl::Hello { target, .. } = parsed else {
            panic!("expected hello");
        };
        assert_eq!(target.as_deref(), Some(VOICE_TARGET_SOCIAL_AI_DIRECT));
    }

    #[test]
    fn hello_accepts_transcribe_only_target() {
        let client = format!(
            r#"{{
                "type":"hello",
                "user_id":"usr_1",
                "target":"{}",
                "sample_rate":24000,
                "channels":1
            }}"#,
            VOICE_TARGET_TRANSCRIBE_ONLY
        );
        let parsed: ClientControl = serde_json::from_str(&client).expect("hello parses");
        let ClientControl::Hello {
            target, group_id, ..
        } = parsed
        else {
            panic!("expected hello");
        };
        assert_eq!(target.as_deref(), Some(VOICE_TARGET_TRANSCRIBE_ONLY));
        assert_eq!(group_id, None);
    }

    #[test]
    fn hello_accepts_external_group_target() {
        let client = format!(
            r#"{{
                "type":"hello",
                "user_id":"usr_1",
                "target":"{}",
                "group_id":"ext_fb2_official",
                "sample_rate":24000,
                "channels":1
            }}"#,
            VOICE_TARGET_EXTERNAL_GROUP
        );
        let parsed: ClientControl = serde_json::from_str(&client).expect("hello parses");
        let ClientControl::Hello {
            target, group_id, ..
        } = parsed
        else {
            panic!("expected hello");
        };
        assert_eq!(target.as_deref(), Some(VOICE_TARGET_EXTERNAL_GROUP));
        assert_eq!(group_id.as_deref(), Some("ext_fb2_official"));
    }

    #[test]
    fn voice_user_must_match_authenticated_user() {
        assert_eq!(
            resolve_authenticated_voice_user("u1", "u1".to_string()).unwrap(),
            "u1"
        );
        assert!(resolve_authenticated_voice_user("u1", "u2".to_string()).is_err());
    }

    #[test]
    fn local_owner_can_claim_debug_voice_user() {
        assert_eq!(
            resolve_authenticated_voice_user("local-owner", "u2".to_string()).unwrap(),
            "u2"
        );
    }
