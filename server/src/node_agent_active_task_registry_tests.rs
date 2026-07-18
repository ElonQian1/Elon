use super::{ActiveCliPromptRegistry, CliPromptRegistration};
use crate::node_agent_active_task::ActiveCliPromptHandle;
use tokio::sync::watch;

fn handle(req_id: &str, route: &str) -> ActiveCliPromptHandle {
    handle_with_rx(req_id, route).0
}

fn handle_with_rx(req_id: &str, route: &str) -> (ActiveCliPromptHandle, watch::Receiver<bool>) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    (
        ActiveCliPromptHandle::new(
            req_id.to_string(),
            "codex".to_string(),
            route.to_string(),
            Some("D:/demo".to_string()),
            Some("project_write".to_string()),
            cancel_tx,
        ),
        cancel_rx,
    )
}

#[tokio::test]
async fn rejects_duplicate_req_id_without_replacing_live_handle() {
    let registry = ActiveCliPromptRegistry::new();
    assert_eq!(
        registry
            .try_insert_with_status(handle("req-1", "route_a_external_cli"))
            .await,
        CliPromptRegistration::Inserted
    );
    assert_eq!(
        registry
            .try_insert_with_status(handle("req-1", "route_c_server_runtime"))
            .await,
        CliPromptRegistration::DuplicateReq
    );

    let view = registry.view("req-1", Vec::new()).await.unwrap();
    assert_eq!(view.route, "route_a_external_cli");
    assert_eq!(registry.len().await, 1);
}

#[tokio::test]
async fn cancel_sender_and_remove_are_idempotent() {
    let registry = ActiveCliPromptRegistry::new();
    let (handle, mut cancel_rx) = handle_with_rx("req-1", "route_a_external_cli");
    assert_eq!(
        registry.try_insert_with_status(handle).await,
        CliPromptRegistration::Inserted
    );

    let cancel_tx = registry.cancel_tx("req-1").await.unwrap();
    assert!(cancel_tx.send(true).is_ok());
    assert!(cancel_rx.changed().await.is_ok());
    assert!(*cancel_rx.borrow());
    assert!(registry.remove("req-1").await);
    assert!(!registry.remove("req-1").await);
    assert!(registry.cancel_tx("req-1").await.is_none());
}

#[tokio::test]
async fn views_without_approvals_exposes_live_handles() {
    let registry = ActiveCliPromptRegistry::new();
    assert_eq!(
        registry
            .try_insert_with_status(handle("req-1", "route_b_api_runtime"))
            .await,
        CliPromptRegistration::Inserted
    );
    assert_eq!(
        registry
            .try_insert_with_status(handle("req-2", "route_c_server_runtime"))
            .await,
        CliPromptRegistration::Inserted
    );

    let mut views = registry.views_without_approvals().await;
    views.sort_by(|left, right| left.req_id.cmp(&right.req_id));

    assert_eq!(views.len(), 2);
    assert_eq!(views[0].req_id, "req-1");
    assert_eq!(views[0].run_handle_id, "req-1");
    assert!(views[0].control_handle_live);
    assert!(views[0].pending_approvals.is_empty());
}

#[tokio::test]
async fn cloud_control_selection_only_returns_marked_tasks() {
    let registry = ActiveCliPromptRegistry::new();
    assert_eq!(
        registry
            .try_insert_with_status(handle("local", "route_a_external_cli"))
            .await,
        CliPromptRegistration::Inserted
    );
    assert_eq!(
        registry
            .try_insert_with_status(
                handle("shared", "route_a_external_cli").with_requires_cloud_control(true),
            )
            .await,
        CliPromptRegistration::Inserted
    );

    assert!(registry.set_requires_cloud_control("local", false).await);
    assert!(registry.set_requires_cloud_control("shared", true).await);

    let mut req_ids = registry.cloud_controlled_req_ids().await;
    req_ids.sort();
    assert_eq!(req_ids, vec!["shared"]);

    let local_view = registry.view("local", Vec::new()).await.unwrap();
    let shared_view = registry.view("shared", Vec::new()).await.unwrap();
    assert!(!local_view.requires_cloud_control);
    assert!(shared_view.requires_cloud_control);
}

#[tokio::test]
async fn midrun_adoption_cancels_when_disconnect_scan_won_before_the_write_lock() {
    let registry = ActiveCliPromptRegistry::new();
    let (handle, mut cancel_rx) = handle_with_rx("req-1", "route_a_external_cli");
    assert_eq!(
        registry.try_insert_with_status(handle).await,
        CliPromptRegistration::Inserted
    );

    // Disconnect cleanup can snapshot the registry immediately before the
    // credential switch acquires the write lock.
    assert!(registry.cloud_controlled_req_ids().await.is_empty());
    let cancel_tx = registry.adopt_cloud_control("req-1").await.unwrap();
    let deadline = crate::node_agent_cloud_control::freeze_cloud_control_deadline(
        true,
        Some("2999-01-01T00:00:00Z"),
        Some("2998-12-31T23:59:00Z"),
        Some(60_000),
        None,
    )
    .unwrap();
    let disconnected = crate::node_agent_cloud_control::validate_registered_cloud_control(
        true,
        false,
        deadline.as_ref(),
    );
    assert!(disconnected.is_err());
    assert!(cancel_tx.send(true).is_ok());
    assert!(cancel_rx.changed().await.is_ok());
    assert!(*cancel_rx.borrow());

    assert!(registry.set_requires_cloud_control("req-1", false).await);

    assert_eq!(
        registry.cloud_controlled_req_ids().await,
        vec!["req-1".to_string()]
    );
}

#[tokio::test]
async fn exclusive_resume_workspace_rejects_active_occupancy_without_affecting_normal_tasks() {
    let registry = ActiveCliPromptRegistry::new();
    assert_eq!(
        registry
            .try_insert_with_status(handle("normal-1", "route_a_external_cli"))
            .await,
        CliPromptRegistration::Inserted
    );
    assert_eq!(
        registry
            .try_insert_with_status(handle("normal-2", "route_a_external_cli"))
            .await,
        CliPromptRegistration::Inserted,
        "ordinary submissions retain their existing concurrency semantics"
    );
    assert_eq!(
        registry
            .try_insert_with_status(
                handle("resume", "route_a_external_cli").with_exclusive_workspace(true),
            )
            .await,
        CliPromptRegistration::WorkspaceBusy
    );
}
