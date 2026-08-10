use std::sync::Arc;

use anyhow::Result;
use tokio::sync::oneshot;

use crate::{store::node_credentials::LegacyNodeRegistrationOutcome, types::AppState};

use super::{
    agent_session::{fail_pending_approvals, fail_pending_pings},
    AgentEntry, AgentManager,
};

const ENDPOINT_ROOT_LEGACY_CLOSE_REASON: &str =
    "节点已切换到安全端点凭据，旧连接已关闭；请重新认证安全端点。";

struct DetachedLegacySessionCleanup {
    registry_done: oneshot::Receiver<()>,
    start_pending: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl DetachedLegacySessionCleanup {
    async fn wait_for_registry(&mut self) {
        let _ = (&mut self.registry_done).await;
    }

    async fn finish(mut self) {
        if let Some(start_pending) = self.start_pending.take() {
            let _ = start_pending.send(());
        }
        if let Err(error) = self.task.await {
            tracing::error!(%error, "detached legacy session cleanup task failed");
        }
    }
}

fn spawn_detached_session_cleanup(
    state: &Arc<AppState>,
    old_entry: AgentEntry,
) -> DetachedLegacySessionCleanup {
    let registry = Arc::clone(&state.node_registry);
    let manager = Arc::clone(&state.agent_manager);
    let process_session = old_entry.process_session.clone();
    let agent_id = process_session.agent_id().to_string();
    let (registry_done_tx, registry_done) = oneshot::channel();
    let (start_pending, start_pending_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        registry.unregister_exact(&process_session).await;
        let _ = registry_done_tx.send(());
        // A dropped sender means the request future was cancelled; continue cleanup anyway.
        let _ = start_pending_rx.await;
        manager
            .recover_session_pending(
                &agent_id,
                &old_entry.pending,
                &old_entry.cli_pending_ids,
                ENDPOINT_ROOT_LEGACY_CLOSE_REASON,
            )
            .await;
        fail_pending_approvals(&old_entry.approval_acks).await;
        fail_pending_pings(&old_entry.ping_acks).await;
    });
    DetachedLegacySessionCleanup {
        registry_done,
        start_pending: Some(start_pending),
        task,
    }
}

impl AgentManager {
    /// Hold the single process-local authority transition fence across one synchronous operation.
    /// Store operations used here must not await.
    pub(crate) async fn with_endpoint_authority_write_fence<R>(
        &self,
        operation: impl FnOnce() -> R,
    ) -> R {
        let _agents = self.agents.write().await;
        operation()
    }

    /// Serialize legacy registration against endpoint-root mutations and evict any process
    /// session whose secret was renewed or whose durable endpoint root already exists.
    pub(crate) async fn run_legacy_registration_and_close_process_session(
        &self,
        state: &Arc<AppState>,
        registration: impl FnOnce() -> Result<LegacyNodeRegistrationOutcome>,
    ) -> Result<LegacyNodeRegistrationOutcome> {
        let (result, mut cleanup) = {
            let mut agents = self.agents.write().await;
            let result = registration();
            let affected_agent_id = match result.as_ref().ok() {
                Some(LegacyNodeRegistrationOutcome::Renewed { agent_id })
                | Some(LegacyNodeRegistrationOutcome::Created { agent_id }) => {
                    Some(agent_id.as_str())
                }
                Some(LegacyNodeRegistrationOutcome::EndpointAuthorityRequired {
                    endpoint_authority,
                }) => Some(endpoint_authority.agent_id()),
                None => None,
            };
            let old_entry = affected_agent_id.and_then(|agent_id| agents.remove(agent_id));
            if let Some(entry) = old_entry.as_ref() {
                let _ = entry.session_shutdown.send(true);
            }
            let mut cleanup =
                old_entry.map(|old_entry| spawn_detached_session_cleanup(state, old_entry));
            if let Some(cleanup) = cleanup.as_mut() {
                cleanup.wait_for_registry().await;
            }
            (result, cleanup)
        };

        if let Some(cleanup) = cleanup.take() {
            cleanup.finish().await;
        }
        result
    }

    /// Preauthorize an endpoint-root mutation, then remove and stop its legacy process session.
    ///
    /// Both Store closures are synchronous and run while the AgentManager write guard is held.
    /// A failed preflight leaves the process session untouched; after successful preflight, a
    /// failed mutation deliberately leaves it closed. Registry removal follows the Store attempt
    /// under the same Manager -> Registry lock order; pending cleanup happens only after the write
    /// guard is released.
    pub(crate) async fn run_endpoint_root_mutation_and_close_process_session<M, T>(
        &self,
        state: &Arc<AppState>,
        mutation_request: M,
        preflight: impl FnOnce(&M) -> Result<String>,
        mutation: impl FnOnce(M) -> Result<T>,
    ) -> Result<T> {
        let (result, mut cleanup) = {
            let mut agents = self.agents.write().await;
            let agent_id = preflight(&mutation_request)?;
            let old_entry = agents.remove(&agent_id);
            if let Some(entry) = old_entry.as_ref() {
                let _ = entry.session_shutdown.send(true);
                tracing::warn!(
                    agent_id = agent_id,
                    session_id = %entry.process_session.session_id(),
                    "closing legacy agent session before endpoint-root mutation"
                );
            }
            let result = mutation(mutation_request);
            let mut cleanup =
                old_entry.map(|old_entry| spawn_detached_session_cleanup(state, old_entry));
            if let Some(cleanup) = cleanup.as_mut() {
                cleanup.wait_for_registry().await;
            }
            (result, cleanup)
        };

        if let Some(cleanup) = cleanup.take() {
            cleanup.finish().await;
        }
        result
    }
}
