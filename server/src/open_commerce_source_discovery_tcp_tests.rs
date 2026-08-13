use std::{sync::Arc, time::Duration};

use axum::http::StatusCode;
use serde_json::{json, Value};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    open_commerce_capability_source_service::test_support,
    open_commerce_developer_production_test_support::test_app_state, router,
};

#[tokio::test]
async fn source_binding_controls_consumer_discovery_over_real_tcp() {
    let fixture = test_support::fixture("tcp");
    test_support::publish_directory(&fixture);
    let consumer = fixture
        .store
        .create_user(
            &format!(
                "source-tcp-consumer-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Source TCP Consumer"),
            None,
        )
        .unwrap();
    let owner_token = session(&fixture.store, &fixture.owner_id);
    let consumer_token = session(&fixture.store, &consumer.id);
    let project_id = fixture.project_id.clone();
    let capability_id = fixture.capability_id.clone();
    let integration_id = fixture.integration_id.clone();
    let receipt_id = fixture.succeeded_receipt_id.clone();
    let merchant_id = fixture.merchant_id.clone();
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-source-tcp-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let state = Arc::new(test_app_state(fixture.store, &root));
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
    let source_url = format!(
        "http://{address}/api/projects/{project_id}/open-commerce/capabilities/{capability_id}/source-link"
    );
    let discover_url = format!("http://{address}/api/open-commerce/sandbox/discover");

    let linked = client
        .put(&source_url)
        .bearer_auth(&owner_token)
        .json(&json!({
            "integration_id": integration_id,
            "sync_receipt_id": receipt_id,
            "data_domain": "catalog"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(linked.status(), StatusCode::OK);
    let linked: Value = linked.json().await.unwrap();
    assert_eq!(linked["provider_key"], "merchant_erp");
    assert_eq!(linked["publishable"], true);

    let request = json!({
        "capability_key": "catalog.search",
        "requester_app_id": "pc-web",
        "require_internal_sync_receipt": true,
        "source_provider_key": "merchant_erp",
        "source_data_domain": "catalog",
        "limit": 5
    });
    let discovered = discover(&client, &discover_url, &consumer_token, &request).await;
    assert_eq!(discovered["matches"].as_array().unwrap().len(), 1);
    assert_eq!(discovered["matches"][0]["merchant"]["id"], merchant_id);
    assert_eq!(
        discovered["matches"][0]["capability"]["source"]["provider_key"],
        "merchant_erp"
    );
    assert_eq!(
        discovered["matches"][0]["capability"]["source"]["data_domain"],
        "catalog"
    );
    assert_eq!(
        discovered["matches"][0]["capability"]["source"]["externally_verified"],
        false
    );

    let removed = client
        .delete(&source_url)
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status(), StatusCode::OK);
    assert_eq!(removed.json::<Value>().await.unwrap()["removed"], true);
    let after_remove = discover(&client, &discover_url, &consumer_token, &request).await;
    assert_eq!(after_remove["matches"].as_array().unwrap().len(), 0);
    assert_eq!(source_audit_count(&state.store, &project_id), 2);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("TCP test server should stop")
        .expect("TCP test server task should join");
}

async fn discover(client: &reqwest::Client, url: &str, token: &str, request: &Value) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

fn session(store: &crate::store::Store, user_id: &str) -> String {
    store.create_session(user_id, Some("test"), None).unwrap().0
}

fn source_audit_count(store: &crate::store::Store, project_id: &str) -> usize {
    store
        .list_project_open_commerce_audit(project_id, 200)
        .unwrap()
        .into_iter()
        .filter(|event| event.action.starts_with("capability.source_"))
        .count()
}
