use super::*;
use contract::{validate_result, SiteManifest, MAX_COMMAND_BYTES, MAX_RESULT_BYTES};
use serde_json::json;

fn workspace() -> std::path::PathBuf {
    std::env::current_dir().unwrap().canonicalize().unwrap()
}
fn command(value: Value) -> ResearchCommand {
    serde_json::from_value(value).unwrap()
}
fn success(token: &str, result: Value) -> ReceiptInput {
    ReceiptInput {
        claim_token: token.into(),
        status: "succeeded".into(),
        result: Some(result),
        error_code: None,
    }
}
fn manifest() -> SiteManifest {
    serde_json::from_value(json!({
        "schema":"yilong.browser-research.site.v1","id":"test-site","name":"通用测试",
        "entry_url":"https://example.org/research",
        "navigation_origins":["https://example.org"],"resource_origins":["https://static.example.org"],
        "api_origins":["https://example.org"],"identity_origins":[]
    })).unwrap()
}

#[test]
fn browser_research_commands_reject_execution_and_unknown_scope_overrides() {
    for kind in [
        "trade",
        "eval",
        "request",
        "navigate",
        "delete",
        "register_script",
    ] {
        assert_eq!(
            command(json!({"kind":kind})).validate(),
            Err("invalid_command")
        );
    }
    assert!(serde_json::from_value::<ResearchCommand>(
        json!({"kind":"sites","project_key":"other"})
    )
    .is_err());
    assert!(
        serde_json::from_value::<ResearchCommand>(json!({"kind":"sites","owner_key":"other"}))
            .is_err()
    );
    assert!(command(json!({"kind":"sites","query":"ignored"}))
        .validate()
        .is_err());
    assert!(command(json!({"kind":"read_resource","session_id":"s"}))
        .validate()
        .is_err());
    assert!(
        command(json!({"kind":"resources","session_id":"s","limit":0}))
            .validate()
            .is_err()
    );
    assert!(command(
        json!({"kind":"read_resource","session_id":"s","resource_id":"r","limit":8193})
    )
    .validate()
    .is_err());
    assert!(
        command(json!({"kind":"resources","session_id":"s","limit":51}))
            .validate()
            .is_err()
    );
    assert!(command(json!({"kind":"status"})).validate().is_err());
    assert!(
        command(json!({"kind":"search","session_id":"s","query":"x".repeat(200)}))
            .validate()
            .is_ok()
    );
    assert!(
        command(json!({"kind":"search","session_id":"s","query":"x".repeat(201)}))
            .validate()
            .is_err()
    );
    assert!(
        command(json!({"kind":"search","session_id":"s","query":"策略"}))
            .validate()
            .is_ok()
    );
    for kind in ["sites", "sessions"] {
        assert!(command(json!({"kind":kind,"offset":30,"limit":30}))
            .validate()
            .is_ok());
    }
}

#[test]
fn browser_research_site_manifest_has_exact_origins_and_no_script_escape() {
    let original = manifest();
    assert!(original.validate().is_ok());
    for origin in [
        "https://example.org/",
        "https://example.org/path",
        "https://*.example.org",
        "http://localhost:4000",
        "file://local",
    ] {
        let mut value = original.clone();
        value.resource_origins = vec![origin.into()];
        assert!(value.validate().is_err(), "{origin}");
    }
    let mut value = original.clone();
    value.entry_url = "https://other.example/".into();
    assert!(value.validate().is_err());
    value = original.clone();
    value.entry_url = "https://example.org/?token=secret".into();
    assert!(value.validate().is_err());
    value = original;
    value.entry_url = "http://127.0.0.1:4567/fixture".into();
    value.navigation_origins = vec!["http://127.0.0.1:4567".into()];
    assert!(value.validate().is_ok());
    value
        .navigation_origins
        .push("http://127.0.0.1:4567".into());
    assert!(value.validate().is_err());
}

#[test]
fn browser_research_result_preserves_business_values_and_blocks_credentials() {
    let value = json!({"data":{"unknownStrategyCollection":[{"amount":"123.4500","symbol":"TESTUSDT"}]},
        "session_id":"research-session","token":"ESK","text":"中文业务资料",
        "content":"const authorization = request.headers.get('Authorization'); const token = 'ESK';"});
    let before = value.clone();
    assert!(validate_result(&value).is_ok());
    assert_eq!(value, before);
    assert!(
        validate_result(&json!({"Authorization":"[credential_excluded]",
        "url":"https://example.org/?access_token=%5Bcredential_excluded%5D"}))
        .is_ok()
    );
    for value in [
        json!({"Authorization":"secret"}),
        json!({"data":[{"csrf_token":"secret"}]}),
        json!({"text":"Bearer private-secret"}),
        json!({"url":"https://example.org/?access_token=secret"}),
        json!({"secret":"unredacted"}),
        json!({"Authorization":"[credential_excluded]extra"}),
    ] {
        assert_eq!(validate_result(&value), Err("credentials_forbidden"));
    }
    assert_eq!(
        validate_result(&json!({"text":"x".repeat(MAX_RESULT_BYTES)})),
        Err("result_too_large")
    );
}

#[test]
fn browser_research_project_isolation_and_claim_is_single_use() {
    let hub = BrowserResearchHub::default();
    let root = workspace();
    let action = hub
        .enqueue(&root, command(json!({"kind":"sites"})))
        .unwrap();
    assert!(hub
        .action(root.parent().unwrap(), &action.action_id)
        .is_err());
    assert!(hub.action(&root, &action.action_id).is_ok());
    assert_eq!(hub.pending(8).unwrap().len(), 1);
    let claim = hub.claim(&action.action_id).unwrap();
    assert_eq!(claim.action.status, "executing");
    assert_eq!(
        hub.claim(&action.action_id).unwrap_err(),
        "action_not_claimable"
    );
    assert!(hub.pending(8).unwrap().is_empty());
    let serialized = serde_json::to_string(&hub.action(&root, &action.action_id).unwrap()).unwrap();
    assert!(!serialized.contains(&claim.claim_token));
    assert!(!serialized.contains(&root.to_string_lossy().to_string()));
}

#[test]
fn browser_research_receipts_require_token_and_exact_duplicate_is_idempotent() {
    let hub = BrowserResearchHub::default();
    let action = hub
        .enqueue(&workspace(), command(json!({"kind":"sites"})))
        .unwrap();
    assert_eq!(
        hub.record_receipt(&action.action_id, success("wrong_claim", json!({})))
            .unwrap_err(),
        "invalid_claim"
    );
    let claim = hub.claim(&action.action_id).unwrap();
    let done = hub
        .record_receipt(
            &action.action_id,
            success(&claim.claim_token, json!({"sites":[]})),
        )
        .unwrap();
    assert_eq!(done.status, "succeeded");
    assert!(hub
        .record_receipt(
            &action.action_id,
            success(&claim.claim_token, json!({"sites":[]}))
        )
        .is_ok());
    assert_eq!(
        hub.record_receipt(
            &action.action_id,
            success(&claim.claim_token, json!({"different":true}))
        )
        .unwrap_err(),
        "receipt_conflict"
    );
}

#[test]
fn browser_research_expiry_and_cancel_discard_late_results() {
    let hub = BrowserResearchHub::default();
    let root = workspace();
    let action = hub
        .enqueue(&root, command(json!({"kind":"sites"})))
        .unwrap();
    let claim = hub.claim(&action.action_id).unwrap();
    hub.inner.lock().unwrap()[0].action.expires_at_ms = 0;
    assert_eq!(
        hub.action(&root, &action.action_id).unwrap().status,
        "expired"
    );
    assert!(hub
        .record_receipt(&action.action_id, success(&claim.claim_token, json!({})))
        .is_err());
    let action = hub
        .enqueue(&root, command(json!({"kind":"sites"})))
        .unwrap();
    let claim = hub.claim(&action.action_id).unwrap();
    assert_eq!(
        hub.cancel(&root, &action.action_id).unwrap().status,
        "cancelled"
    );
    assert!(hub
        .record_receipt(&action.action_id, success(&claim.claim_token, json!({})))
        .is_err());
}

#[test]
fn browser_research_queue_rejects_overflow_without_evicting_live_actions() {
    let hub = BrowserResearchHub::default();
    let root = workspace();
    for _ in 0..16 {
        hub.enqueue(&root, command(json!({"kind":"sites"})))
            .unwrap();
    }
    assert_eq!(
        hub.enqueue(&root, command(json!({"kind":"sites"})))
            .unwrap_err(),
        "queue_full"
    );
    assert_eq!(hub.pending(16).unwrap().len(), 16);
    assert_eq!(hub.pending(0).unwrap_err(), "invalid_limit");
}

#[test]
fn browser_research_failure_receipts_never_accept_raw_errors_or_results() {
    let hub = BrowserResearchHub::default();
    let action = hub
        .enqueue(&workspace(), command(json!({"kind":"sites"})))
        .unwrap();
    let claim = hub.claim(&action.action_id).unwrap();
    let bad = ReceiptInput {
        claim_token: claim.claim_token.clone(),
        status: "failed".into(),
        result: None,
        error_code: Some("unexpected private server error".into()),
    };
    assert_eq!(
        hub.record_receipt(&action.action_id, bad).unwrap_err(),
        "invalid_receipt"
    );
    let good = ReceiptInput {
        claim_token: claim.claim_token,
        status: "host_unavailable".into(),
        result: None,
        error_code: Some("host_unavailable".into()),
    };
    assert_eq!(
        hub.record_receipt(&action.action_id, good).unwrap().status,
        "host_unavailable"
    );
}

#[test]
fn browser_research_command_and_pending_batches_are_bounded() {
    let oversized =
        command(json!({"kind":"search","session_id":"s","query":"x".repeat(MAX_COMMAND_BYTES)}));
    assert_eq!(oversized.validate(), Err("command_too_large"));
    let hub = BrowserResearchHub::default();
    let root = workspace();
    let mut value = manifest();
    // Exact, distinct long origins remain valid; the batch must not multiply their size unboundedly.
    value.resource_origins = (0..16)
        .map(|n| format!("https://{}.{}.example.org", "a".repeat(60), n))
        .collect();
    value.api_origins = value.resource_origins.clone();
    value.identity_origins = value.resource_origins.clone();
    for _ in 0..16 {
        hub.enqueue(
            &root,
            command(json!({"kind":"register_site","manifest":value})),
        )
        .unwrap();
    }
    let pending = hub.pending(16).unwrap();
    assert!(!pending.is_empty());
    assert!(
        serde_json::to_vec(&json!({"ok":true,"actions":pending}))
            .unwrap()
            .len()
            <= MAX_RESULT_BYTES
    );
}
