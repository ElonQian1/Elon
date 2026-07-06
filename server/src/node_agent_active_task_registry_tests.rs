    use super::ActiveCliPromptRegistry;
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
        assert!(
            registry
                .try_insert(handle("req-1", "route_a_external_cli"))
                .await
        );
        assert!(
            !registry
                .try_insert(handle("req-1", "route_c_server_runtime"))
                .await
        );

        let view = registry.view("req-1", Vec::new()).await.unwrap();
        assert_eq!(view.route, "route_a_external_cli");
        assert_eq!(registry.len().await, 1);
    }

    #[tokio::test]
    async fn cancel_sender_and_remove_are_idempotent() {
        let registry = ActiveCliPromptRegistry::new();
        let (handle, mut cancel_rx) = handle_with_rx("req-1", "route_a_external_cli");
        assert!(registry.try_insert(handle).await);

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
        assert!(
            registry
                .try_insert(handle("req-1", "route_b_api_runtime"))
                .await
        );
        assert!(
            registry
                .try_insert(handle("req-2", "route_c_server_runtime"))
                .await
        );

        let mut views = registry.views_without_approvals().await;
        views.sort_by(|left, right| left.req_id.cmp(&right.req_id));

        assert_eq!(views.len(), 2);
        assert_eq!(views[0].req_id, "req-1");
        assert_eq!(views[0].run_handle_id, "req-1");
        assert!(views[0].control_handle_live);
        assert!(views[0].pending_approvals.is_empty());
    }
