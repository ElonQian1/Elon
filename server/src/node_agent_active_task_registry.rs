// server/src/node_agent_active_task_registry.rs

use std::collections::HashMap;

use tokio::sync::{watch, RwLock};

use crate::{
    node_agent_active_task::{ActiveCliPromptHandle, ActiveCliPromptView},
    node_agent_tool_approval::PendingToolApprovalView,
};

#[derive(Default)]
pub(crate) struct ActiveCliPromptRegistry {
    prompts: RwLock<HashMap<String, ActiveCliPromptHandle>>,
}

impl ActiveCliPromptRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn contains(&self, req_id: &str) -> bool {
        self.prompts.read().await.contains_key(req_id)
    }

    pub(crate) async fn try_insert(&self, handle: ActiveCliPromptHandle) -> bool {
        let mut prompts = self.prompts.write().await;
        let req_id = handle.req_id().to_string();
        if prompts.contains_key(&req_id) {
            return false;
        }
        prompts.insert(req_id, handle);
        true
    }

    pub(crate) async fn cancel_tx(&self, req_id: &str) -> Option<watch::Sender<bool>> {
        self.prompts
            .read()
            .await
            .get(req_id)
            .map(ActiveCliPromptHandle::cancel_tx)
    }

    pub(crate) async fn view(
        &self,
        req_id: &str,
        pending_approvals: Vec<PendingToolApprovalView>,
    ) -> Option<ActiveCliPromptView> {
        let mut prompts = self.prompts.write().await;
        let handle = prompts.get_mut(req_id)?;
        handle.touch();
        Some(handle.view(pending_approvals))
    }

    pub(crate) async fn set_os_pid(&self, req_id: &str, pid: Option<u32>) {
        if let Some(handle) = self.prompts.write().await.get_mut(req_id) {
            handle.set_os_pid(pid);
        }
    }

    pub(crate) async fn remove(&self, req_id: &str) -> bool {
        self.prompts.write().await.remove(req_id).is_some()
    }

    pub(crate) async fn len(&self) -> usize {
        self.prompts.read().await.len()
    }
}

#[cfg(test)]
mod tests {
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
}
