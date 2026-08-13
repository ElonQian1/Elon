use std::{sync::Arc, time::Duration};

use axum::http::StatusCode;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use super::tests::{fixture, rotate_claim_token};
use crate::router;

#[tokio::test]
async fn adapter_claim_reaches_handoff_receipt_over_real_tcp() {
    let fixture = fixture();
    let claim_token = rotate_claim_token(&fixture);
    let project_id = fixture.project_id.clone();
    let state = Arc::clone(&fixture.state);
    let app = router::build_app(Arc::clone(&state));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let claims_url = format!("http://{address}/api/open-commerce/adapter/business-handoff-claims");

    let claimed = client
        .post(&claims_url)
        .bearer_auth(&claim_token)
        .json(&json!({"lease_seconds": 300}))
        .send()
        .await
        .unwrap();
    assert_eq!(claimed.status(), StatusCode::OK);
    let claimed: Value = claimed.json().await.unwrap();
    assert_eq!(claimed["claimed"], true);
    assert_eq!(
        claimed["issue"]["task"]["result"]["order"]["id"],
        "merchant-order-http-1"
    );
    assert_eq!(claimed["issue"]["lease_token_visible_once"], true);
    let claim_id = claimed["issue"]["claim"]["id"].as_str().unwrap();
    let lease_token = claimed["issue"]["lease_token"].as_str().unwrap();
    assert!(lease_token.starts_with("oc_claim_"));

    let complete_url = format!("{claims_url}/{claim_id}/complete");
    let completion = json!({
        "lease_token": lease_token,
        "receipt_key": "tcp-claim-applied",
        "status": "applied",
        "target_domain": "erp",
        "target_reference": "erp-order-tcp-1",
        "completed_at": "2026-08-13T00:00:00Z"
    });
    let completed = complete(&client, &complete_url, &claim_token, &completion).await;
    assert_eq!(completed["status"], "applied");
    assert_eq!(completed["target_domain"], "erp");
    assert_eq!(
        completed["target_reference_sha256"].as_str().unwrap().len(),
        64
    );
    assert!(!completed.to_string().contains("erp-order-tcp-1"));
    assert_eq!(completed["funds_moved"], false);
    let receipt_id = completed["id"].clone();
    let replayed = complete(&client, &complete_url, &claim_token, &completion).await;
    assert_eq!(replayed["id"], receipt_id);

    let list_url =
        format!("http://{address}/api/projects/{project_id}/open-commerce/adapter-handoff-claims");
    let listed = client
        .get(list_url)
        .bearer_auth(&fixture.owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed: Value = listed.json().await.unwrap();
    assert_eq!(listed["claims"].as_array().unwrap().len(), 1);
    assert_eq!(listed["claims"][0]["status"], "completed");
    assert_eq!(listed["claims"][0]["completed_receipt_id"], receipt_id);
    assert!(!listed.to_string().contains(lease_token));
    assert!(!listed.to_string().contains("token_hash"));

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("TCP test server should stop")
        .expect("TCP test server task should join");
}

async fn complete(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}
