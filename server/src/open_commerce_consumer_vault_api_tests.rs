use axum::http::{Method, StatusCode};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};

#[path = "open_commerce_consumer_vault_api_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn routes_enforce_auth_project_and_per_user_isolation() {
    let fixture = fixture();
    let id = "shared_record_123";
    let owner_ciphertext = BASE64.encode([21_u8; 17]);
    let member_ciphertext = BASE64.encode([22_u8; 17]);

    let (status, _) = create_item(
        &fixture,
        &fixture.project_id,
        &fixture.owner_token,
        id,
        "owner-label",
        21,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let item_path = item_path(&fixture.project_id, id);
    assert_eq!(
        send_json(&fixture.router, Method::GET, &item_path, None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::GET,
            &item_path,
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &item_path,
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("不存在"));

    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &list_path(&fixture.project_id),
        Some(&fixture.member_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);

    let (status, _) = create_item(
        &fixture,
        &fixture.project_id,
        &fixture.member_token,
        id,
        "member-label",
        22,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = create_item(
        &fixture,
        &fixture.second_project_id,
        &fixture.owner_token,
        id,
        "second-project-label",
        23,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let owner = get_item(&fixture, &fixture.project_id, &fixture.owner_token, id).await;
    let member = get_item(&fixture, &fixture.project_id, &fixture.member_token, id).await;
    assert_eq!(owner["envelope"]["ciphertext_base64"], owner_ciphertext);
    assert_eq!(member["envelope"]["ciphertext_base64"], member_ciphertext);

    let (_, list) = send_json(
        &fixture.router,
        Method::GET,
        &list_path(&fixture.project_id),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert!(list["items"][0].get("envelope").is_none());
}

#[tokio::test]
async fn routes_enforce_revision_delete_confirmation_and_audit_redaction() {
    let fixture = fixture();
    let id = "revision_record_123";
    let label_marker = "secret-label-marker";
    let ciphertext_marker = BASE64.encode([31_u8; 17]);
    assert_eq!(
        create_item(
            &fixture,
            &fixture.project_id,
            &fixture.owner_token,
            id,
            label_marker,
            31,
        )
        .await
        .0,
        StatusCode::OK
    );

    let update_path = format!("{}/update", item_path(&fixture.project_id, id));
    let update = json!({
        "expected_revision": 1,
        "label": "updated-label-marker",
        "item_kind": "finance",
        "envelope": envelope(id, 2, 32),
    });
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &update_path,
            Some(&fixture.owner_token),
            update.clone(),
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, stale) = send_json(
        &fixture.router,
        Method::POST,
        &update_path,
        Some(&fixture.owner_token),
        update,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(stale["error"].as_str().unwrap().contains("已变化"));

    let delete_path = format!("{}/delete", item_path(&fixture.project_id, id));
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &delete_path,
            Some(&fixture.owner_token),
            json!({"expected_revision":2,"confirmed_by_user":false}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &delete_path,
            Some(&fixture.owner_token),
            json!({"expected_revision":1,"confirmed_by_user":true}),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::POST,
            &delete_path,
            Some(&fixture.owner_token),
            json!({"expected_revision":2,"confirmed_by_user":true}),
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send_json(
            &fixture.router,
            Method::GET,
            &item_path(&fixture.project_id, id),
            Some(&fixture.owner_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let audit_payloads = fixture
        .state
        .store
        .conn()
        .unwrap()
        .prepare(
            "SELECT metadata_json FROM open_commerce_audit_events
              WHERE project_id=?1 AND subject_type='consumer_data_vault_item'
              ORDER BY created_at, id",
        )
        .unwrap()
        .query_map([&fixture.project_id], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(audit_payloads.len(), 3);
    let combined = audit_payloads.join("\n");
    assert!(!combined.contains(label_marker));
    assert!(!combined.contains("updated-label-marker"));
    assert!(!combined.contains(&ciphertext_marker));
    assert!(!combined.contains("passphrase"));
    assert!(!combined.contains("plaintext"));
}

#[tokio::test]
async fn route_rejects_the_one_hundred_and_first_item_per_owner() {
    let fixture = fixture();
    let conn = fixture.state.store.conn().unwrap();
    for index in 0..100 {
        conn.execute(
            "INSERT INTO open_commerce_consumer_data_vault_items (
               id, consumer_project_id, consumer_user_id, label, item_kind,
               envelope_json, ciphertext_sha256, ciphertext_bytes, revision,
               created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'label', 'private_note', '{}', 'digest', 17, 1, 'now', 'now')",
            rusqlite::params![
                format!("capacity_item_{index:03}"),
                fixture.project_id,
                fixture.owner_id,
            ],
        )
        .unwrap();
    }
    drop(conn);

    let (status, body) = create_item(
        &fixture,
        &fixture.project_id,
        &fixture.owner_token,
        "capacity_overflow",
        "overflow",
        41,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("最多保存 100"));

    assert_eq!(
        create_item(
            &fixture,
            &fixture.project_id,
            &fixture.member_token,
            "capacity_overflow",
            "member-capacity",
            42,
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_updates_commit_exactly_one_revision() {
    let fixture = fixture();
    let id = "concurrent_record_123";
    assert_eq!(
        create_item(
            &fixture,
            &fixture.project_id,
            &fixture.owner_token,
            id,
            "before-concurrency",
            51,
        )
        .await
        .0,
        StatusCode::OK
    );

    let update_path = format!("{}/update", item_path(&fixture.project_id, id));
    let left = {
        let router = fixture.router.clone();
        let path = update_path.clone();
        let token = fixture.owner_token.clone();
        tokio::spawn(async move {
            send_json(
                &router,
                Method::POST,
                &path,
                Some(&token),
                json!({
                    "expected_revision": 1,
                    "label": "concurrent-left",
                    "item_kind": "private_note",
                    "envelope": envelope(id, 2, 52),
                }),
            )
            .await
            .0
        })
    };
    let right = {
        let router = fixture.router.clone();
        let path = update_path;
        let token = fixture.owner_token.clone();
        tokio::spawn(async move {
            send_json(
                &router,
                Method::POST,
                &path,
                Some(&token),
                json!({
                    "expected_revision": 1,
                    "label": "concurrent-right",
                    "item_kind": "private_note",
                    "envelope": envelope(id, 2, 53),
                }),
            )
            .await
            .0
        })
    };
    let statuses = [left.await.unwrap(), right.await.unwrap()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::BAD_REQUEST)
            .count(),
        1
    );

    let item = get_item(&fixture, &fixture.project_id, &fixture.owner_token, id).await;
    assert_eq!(item["revision"], 2);
    assert!(matches!(
        item["label"].as_str(),
        Some("concurrent-left" | "concurrent-right")
    ));
}
