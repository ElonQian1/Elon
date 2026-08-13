use super::*;

pub(super) async fn assert_adapter_applies_invocation(
    client: &reqwest::Client,
    base_url: &str,
    adapter_token: &str,
    invocation_id: &str,
) {
    let claims_url = format!("{base_url}/api/open-commerce/adapter/business-handoff-claims");
    let claimed = adapter_post(
        client,
        &claims_url,
        adapter_token,
        &json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(claimed["claimed"], true);
    assert_eq!(claimed["issue"]["claim"]["invocation_id"], invocation_id);
    let claim_id = claimed["issue"]["claim"]["id"].as_str().unwrap();
    let lease_token = claimed["issue"]["lease_token"].as_str().unwrap();
    let completed = adapter_post(
        client,
        &format!("{claims_url}/{claim_id}/complete"),
        adapter_token,
        &json!({
            "lease_token":lease_token,
            "receipt_key":"production-runtime-adapter-applied-1",
            "status":"applied",
            "target_domain":"erp",
            "target_reference":"erp-order-production-runtime-1",
            "completed_at":Utc::now().to_rfc3339()
        }),
    )
    .await;
    assert_eq!(completed["invocation_id"], invocation_id);
    assert_eq!(completed["status"], "applied");
    assert_eq!(completed["funds_moved"], false);
}
