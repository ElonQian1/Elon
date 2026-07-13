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

    pub(crate) async fn views_without_approvals(&self) -> Vec<ActiveCliPromptView> {
        let mut prompts = self.prompts.write().await;
        prompts
            .values_mut()
            .map(|handle| {
                handle.touch();
                handle.view(Vec::new())
            })
            .collect()
    }

    pub(crate) async fn set_os_pid(&self, req_id: &str, pid: Option<u32>) {
        if let Some(handle) = self.prompts.write().await.get_mut(req_id) {
            handle.set_os_pid(pid);
        }
    }

    pub(crate) async fn set_requires_cloud_control(&self, req_id: &str, required: bool) -> bool {
        let mut prompts = self.prompts.write().await;
        let Some(handle) = prompts.get_mut(req_id) else {
            return false;
        };
        handle.set_requires_cloud_control(required);
        true
    }

    /// Atomically makes the task visible to disconnect cleanup and returns its
    /// cancellation sender for the adopter's post-lock connection/deadline check.
    pub(crate) async fn adopt_cloud_control(&self, req_id: &str) -> Option<watch::Sender<bool>> {
        let mut prompts = self.prompts.write().await;
        let handle = prompts.get_mut(req_id)?;
        handle.set_requires_cloud_control(true);
        Some(handle.cancel_tx())
    }

    pub(crate) async fn cloud_controlled_req_ids(&self) -> Vec<String> {
        self.prompts
            .read()
            .await
            .values()
            .filter(|handle| handle.requires_cloud_control())
            .map(|handle| handle.req_id().to_string())
            .collect()
    }

    pub(crate) async fn remove(&self, req_id: &str) -> bool {
        self.prompts.write().await.remove(req_id).is_some()
    }

    #[cfg(test)]
    pub(crate) async fn len(&self) -> usize {
        self.prompts.read().await.len()
    }
}

#[cfg(test)]
#[path = "node_agent_active_task_registry_tests.rs"]
mod tests;
