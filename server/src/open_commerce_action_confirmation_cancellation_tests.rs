use serde_json::json;
use std::sync::{Arc, Barrier};

use crate::{
    open_commerce_action_confirmation_model::{
        ACTION_CANCELLATION_PHRASE, ACTION_CONFIRMATION_PHRASE,
    },
    open_commerce_action_confirmation_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

use super::test_support::{action_request, fixture};

#[test]
fn pending_and_confirmed_cancellations_are_idempotent_and_audited_once() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };

    for (key, confirm_first) in [
        ("cancel-pending-action", false),
        ("cancel-confirmed-action", true),
    ] {
        let prepared = open_commerce_action_confirmation_service::prepare(
            &fixture.store,
            &actor,
            action_request(&fixture, key),
        )
        .unwrap();
        if confirm_first {
            open_commerce_action_confirmation_service::confirm(
                &fixture.store,
                &actor,
                &prepared.id,
                ACTION_CONFIRMATION_PHRASE,
            )
            .unwrap();
        }

        let canceled = open_commerce_action_confirmation_service::cancel(
            &fixture.store,
            &actor,
            &prepared.id,
            ACTION_CANCELLATION_PHRASE,
        )
        .unwrap();
        assert_eq!(canceled.status, "expired");
        assert!(canceled.canceled_at.is_some());
        assert!(canceled.invocation_id.is_none());

        let replayed = open_commerce_action_confirmation_service::cancel(
            &fixture.store,
            &actor,
            &prepared.id,
            ACTION_CANCELLATION_PHRASE,
        )
        .unwrap();
        assert_eq!(replayed.canceled_at, canceled.canceled_at);

        let confirm_error = open_commerce_action_confirmation_service::confirm(
            &fixture.store,
            &actor,
            &prepared.id,
            ACTION_CONFIRMATION_PHRASE,
        )
        .unwrap_err();
        assert!(confirm_error.to_string().contains("已过期"));

        let cancellation_audits = fixture
            .store
            .list_project_open_commerce_audit(&fixture.project_id, 200)
            .unwrap()
            .into_iter()
            .filter(|event| {
                event.action == "action_confirmation.canceled" && event.subject_id == prepared.id
            })
            .count();
        assert_eq!(cancellation_audits, 1);

        if !confirm_first {
            let replacement = open_commerce_action_confirmation_service::prepare(
                &fixture.store,
                &actor,
                action_request(&fixture, key),
            )
            .unwrap();
            assert_ne!(replacement.id, prepared.id);
            assert_eq!(replacement.status, "pending");
        }
    }
    assert!(fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn cancellation_and_invocation_creation_are_mutually_exclusive() {
    let fixture = fixture();
    let request = action_request(&fixture, "cancel-invocation-race");
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, request.clone())
            .unwrap();
    open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &prepared.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();

    let store = Arc::new(fixture.store);
    let barrier = Arc::new(Barrier::new(2));
    let cancel_store = Arc::clone(&store);
    let cancel_barrier = Arc::clone(&barrier);
    let cancel_user_id = fixture.owner_id.clone();
    let cancel_confirmation_id = prepared.id.clone();
    let cancel_task = tokio::task::spawn_blocking(move || {
        cancel_barrier.wait();
        let actor = OpenCommerceActor {
            user_id: &cancel_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        open_commerce_action_confirmation_service::cancel(
            &cancel_store,
            &actor,
            &cancel_confirmation_id,
            ACTION_CANCELLATION_PHRASE,
        )
    });

    let invoke_store = Arc::clone(&store);
    let invoke_barrier = Arc::clone(&barrier);
    let invoke_user_id = fixture.owner_id.clone();
    let invoke_confirmation_id = prepared.id.clone();
    let invoke_task = tokio::spawn(async move {
        invoke_barrier.wait();
        let actor = OpenCommerceActor {
            user_id: &invoke_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        open_commerce_service::invoke_with_action_confirmation(
            &invoke_store,
            &actor,
            request,
            Some(&invoke_confirmation_id),
        )
        .await
    });

    let cancel_result = cancel_task.await.unwrap();
    let invoke_result = invoke_task.await.unwrap();
    assert_ne!(cancel_result.is_ok(), invoke_result.is_ok());

    let row = store
        .open_commerce_action_confirmation(&prepared.id)
        .unwrap();
    assert_ne!(row.canceled_at.is_some(), row.invocation_id.is_some());
}

#[tokio::test]
async fn independent_connections_serialize_cancellation_and_invocation_creation() {
    let fixture = fixture();
    let request = action_request(&fixture, "cancel-invocation-cross-connection");
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, request.clone())
            .unwrap();
    open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &prepared.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();

    let cancel_store = Store::open(&fixture.path).unwrap();
    let invoke_store = Store::open(&fixture.path).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let cancel_barrier = Arc::clone(&barrier);
    let cancel_user_id = fixture.owner_id.clone();
    let cancel_confirmation_id = prepared.id.clone();
    let cancel_task = tokio::task::spawn_blocking(move || {
        cancel_barrier.wait();
        let actor = OpenCommerceActor {
            user_id: &cancel_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        open_commerce_action_confirmation_service::cancel(
            &cancel_store,
            &actor,
            &cancel_confirmation_id,
            ACTION_CANCELLATION_PHRASE,
        )
    });

    let invoke_barrier = Arc::clone(&barrier);
    let invoke_user_id = fixture.owner_id.clone();
    let invoke_confirmation_id = prepared.id.clone();
    let invoke_task = tokio::spawn(async move {
        invoke_barrier.wait();
        let actor = OpenCommerceActor {
            user_id: &invoke_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        open_commerce_service::invoke_with_action_confirmation(
            &invoke_store,
            &actor,
            request,
            Some(&invoke_confirmation_id),
        )
        .await
    });

    let cancel_result = cancel_task.await.unwrap();
    let invoke_result = invoke_task.await.unwrap();
    assert_ne!(cancel_result.is_ok(), invoke_result.is_ok());

    let row = fixture
        .store
        .open_commerce_action_confirmation(&prepared.id)
        .unwrap();
    assert_ne!(row.canceled_at.is_some(), row.invocation_id.is_some());
}

#[tokio::test]
async fn cancellation_rejects_wrong_actor_expiry_and_consumed_action() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "cancel-boundary-action"),
    )
    .unwrap();

    let wrong_phrase = open_commerce_action_confirmation_service::cancel(
        &fixture.store,
        &actor,
        &prepared.id,
        "cancel",
    )
    .unwrap_err();
    assert!(wrong_phrase.to_string().contains("短语无效"));

    let other = fixture
        .store
        .create_user("cancel-other@example.com", "secret1", None, None)
        .unwrap();
    for other_actor in [
        OpenCommerceActor {
            user_id: &other.id,
            app_id: "pc-web",
            project_role: None,
        },
        OpenCommerceActor {
            user_id: &fixture.owner_id,
            app_id: "mcp-client",
            project_role: Some("owner"),
        },
    ] {
        let error = open_commerce_action_confirmation_service::cancel(
            &fixture.store,
            &other_actor,
            &prepared.id,
            ACTION_CANCELLATION_PHRASE,
        )
        .unwrap_err();
        assert!(error.to_string().contains("不存在"));
    }

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_action_confirmations
             SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
            [&prepared.id],
        )
        .unwrap();
    let expired = open_commerce_action_confirmation_service::cancel(
        &fixture.store,
        &actor,
        &prepared.id,
        ACTION_CANCELLATION_PHRASE,
    )
    .unwrap_err();
    assert!(expired.to_string().contains("自然过期"));
    let expired_row = fixture
        .store
        .open_commerce_action_confirmation(&prepared.id)
        .unwrap();
    assert_eq!(expired_row.status, "expired");
    assert!(expired_row.canceled_at.is_none());

    let consumed_request = action_request(&fixture, "cancel-consumed-action");
    let consumed_confirmation = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        consumed_request.clone(),
    )
    .unwrap();
    open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &consumed_confirmation.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();
    open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        consumed_request,
        Some(&consumed_confirmation.id),
    )
    .await
    .unwrap();
    let consumed_error = open_commerce_action_confirmation_service::cancel(
        &fixture.store,
        &actor,
        &consumed_confirmation.id,
        ACTION_CANCELLATION_PHRASE,
    )
    .unwrap_err();
    assert!(consumed_error.to_string().contains("已创建调用"));
}

#[tokio::test]
async fn mcp_cancellation_reports_canceled_without_creating_an_invocation() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "mcp-cancel-action"),
    )
    .unwrap();

    let canceled = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_cancel_my_action_confirmation",
            "arguments":{
                "confirmation_id":prepared.id,
                "confirmation_phrase":"CANCEL_ACTION"
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(canceled["structuredContent"]["status"], "canceled");
    assert_eq!(canceled["structuredContent"]["invocation_created"], false);
    assert!(canceled["structuredContent"]["canceled_at"].is_string());

    let status = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_get_my_action_confirmation",
            "arguments":{"confirmation_id":prepared.id}
        }),
    )
    .await
    .unwrap();
    assert_eq!(status["structuredContent"]["status"], "canceled");
    assert_eq!(status["structuredContent"]["next_step"], "stop");
    assert!(fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());
}
