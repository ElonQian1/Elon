use super::*;
use serde_json::json;

#[test]
fn forms_and_embedded_url_queries_exclude_credentials_and_preserve_business() {
    for input in [
        "csrfToken=SYNTHETIC_SECRET&gridCount=20&token=ESK",
        "authorization=Bearer SYNTHETIC_SECRET&symbol=ESK",
        "%61ccess_token=SYNTHETIC_SECRET&margin=12.50",
        "const endpoint='https://x.test/grid?signature=SYNTHETIC_SECRET&gridCount=20';",
        r#"{"link":"https://x.test/grid?access_token=SYNTHETIC_SECRET&gridCount=20","token":"ESK"}"#,
    ] {
        let (clean, changed) = clean_body(input).unwrap();
        assert!(changed, "{input}");
        assert!(!clean.contains("SYNTHETIC_SECRET"));
        assert!(clean.contains("credential_excluded"));
    }
    let (clean, _) = clean_body("csrfToken=SYNTHETIC_SECRET&gridCount=20&token=ESK").unwrap();
    assert!(clean.contains("gridCount=20"));
    assert!(clean.contains("token=ESK"));
}

#[test]
fn quoted_bootstrap_and_ambiguous_scalars_have_explicit_scope() {
    let input = r#"window.bootstrap={token:"SYNTHETIC_SECRET",sessionId:'SYNTHETIC_SESSION',gridCount:20}; const authorization=request.headers.get('Authorization');"#;
    let (clean, changed) = clean_body(input).unwrap();
    assert!(changed);
    assert!(!clean.contains("SYNTHETIC_SECRET"));
    assert!(!clean.contains("SYNTHETIC_SESSION"));
    assert!(clean.contains("gridCount:20"));
    assert!(clean.contains("request.headers.get('Authorization')"));
    let business = json!({"token":{"symbol":"ESK","network":"test"},"rows":[{"token":"ESK","session_id":42}],
        "sessionId":"short-id","unknownStrategyCollection":[{"margin":"12.500"}]});
    let raw = business.to_string();
    let (clean, changed) = clean_body(&raw).unwrap();
    assert!(!changed);
    assert_eq!(clean, raw);
}

#[test]
fn html_meta_and_hidden_inputs_cover_case_order_and_unquoted_attributes() {
    for input in [
        r#"<meta name="csrf-token" content="SYNTHETIC_SECRET">"#,
        r#"<META CONTENT='SYNTHETIC_SECRET' NAME='XSRF-TOKEN'>"#,
        "<input value=SYNTHETIC_SECRET name=csrf-token type=hidden>",
        r#"<input name="password" value="short-secret">"#,
        r#"<meta property="token" content="SYNTHETIC_SECRET">"#,
        r#"<input name="sessionId" value="SYNTHETIC_SESSION">"#,
    ] {
        let (clean, changed) = clean_body(input).unwrap();
        assert!(changed, "{input}");
        assert!(!clean.contains("SYNTHETIC_SECRET"));
        assert!(!clean.contains("SYNTHETIC_SESSION"));
        assert!(!clean.contains("short-secret"));
    }
    let plain = r#"<input name="symbol" value="ESK"><meta name="token" content="ESK">"#;
    assert_eq!(clean_body(plain).unwrap(), (plain.into(), false));
}

#[test]
fn cleaned_content_is_idempotent_and_initiator_url_credentials_are_removed() {
    let original = json!({"url":"https://x.test/a?signature=SYNTHETIC_SECRET&strategyId=42",
        "callFrames":[{"url":"https://user:secret@x.test/script.js"}],"lineNumber":100});
    let clean = clean_initiator(&original).unwrap();
    let raw = clean.to_string();
    assert!(!raw.contains("SYNTHETIC_SECRET"));
    assert!(!raw.contains("user:secret"));
    assert!(raw.contains("strategyId=42"));
    assert_eq!(clean_body(&raw).unwrap(), (raw, false));
    let url = safe_url("https://x.test/a?signature=SYNTHETIC_SECRET&token=ESK#private").unwrap();
    assert!(!url.contains("SYNTHETIC_SECRET"));
    assert!(url.contains("token=ESK"));
    assert!(!url.contains("#private"));
}

#[test]
fn privacy_work_is_bounded_and_explicit_credentials_are_blocked_at_any_json_depth() {
    assert_eq!(
        clean_body(&"x".repeat(BODY_LIMIT + 1)).unwrap_err(),
        "body_too_large"
    );
    let input = json!({"dynamicField":[{"api_secret":"SYNTHETIC_SECRET","businessAmount":123}]})
        .to_string();
    let (clean, changed) = clean_body(&input).unwrap();
    assert!(changed);
    assert!(!clean.contains("SYNTHETIC_SECRET"));
    assert!(clean.contains("businessAmount"));
    assert!(clean.contains("123"));
}

#[test]
fn duplicate_json_branches_are_rejected_instead_of_losing_or_disclosing_fields() {
    for input in [
        r#"{"data":{"authorization":{"value":"SYNTHETIC_SECRET"}},"data":{"symbol":"ESK"}}"#,
        r#"{"rows":[{"data":{"authorization":"SYNTHETIC_SECRET"},"data":null}]}"#,
        r#"{"margin":10,"margin":20}"#,
        r#"{"data":1,"\u0064ata":2}"#,
    ] {
        assert_eq!(
            clean_body(input).unwrap_err(),
            "duplicate_json_keys_not_captured"
        );
    }
    let input = r#"{"first":{"symbol":"ESK"},"second":{"symbol":"OTHER"},"margin":"123.4500"}"#;
    assert_eq!(clean_body(input).unwrap(), (input.into(), false));
}

#[test]
fn json_container_limit_is_enforced_before_building_a_value() {
    let nested = |count| format!("{}0{}", "[".repeat(count), "]".repeat(count));
    assert!(clean_body(&nested(64)).is_ok());
    assert_eq!(
        clean_body(&nested(65)).unwrap_err(),
        "json_depth_limit_not_captured"
    );
    assert_eq!(
        clean_body(&nested(129)).unwrap_err(),
        "json_depth_limit_not_captured"
    );
}
