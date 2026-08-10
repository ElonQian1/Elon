use std::sync::Arc;

use anyhow::{anyhow, Result};
use homecli_proto::{
    ModelCapability, NodeDevRuntimeProfile, NodeHardwareProfile, NodeLifecycleReport,
    NodeStorageProfile,
};

use crate::{node_registry::AgentProcessSessionKey, types::AppState};

use super::{
    agent_session::{fail_pending_approvals, fail_pending_pings},
    AgentEntry, AgentManager,
};

impl AgentManager {
    /// Run one synchronous operation while the process-local connection remains current.
    ///
    /// Callers must not await inside `operation`. Holding this read guard prevents a
    /// replacement writer from crossing a synchronous Store commit, but it is not
    /// durable endpoint-session currentness.
    pub(super) async fn with_current_process_session<R>(
        &self,
        process_session: &AgentProcessSessionKey,
        operation: impl FnOnce(&AgentEntry) -> R,
    ) -> Option<R> {
        let agents = self.agents.read().await;
        let entry = agents.get(process_session.agent_id())?;
        if &entry.process_session != process_session {
            return None;
        }
        Some(operation(entry))
    }

    /// Close the currently registered legacy session for an agent.
    ///
    /// This agent-id facade remains for legacy callers. Endpoint credential/session
    /// mutation must later use a durable exact binding and an exact process key.
    pub async fn close_agent_session(&self, agent_id: &str, reason: &str) -> bool {
        let process_session = {
            let agents = self.agents.read().await;
            agents
                .get(agent_id)
                .map(|entry| entry.process_session.clone())
        };
        let Some(process_session) = process_session else {
            return false;
        };
        self.close_process_session(&process_session, reason).await
    }

    pub(crate) async fn close_process_session(
        &self,
        process_session: &AgentProcessSessionKey,
        reason: &str,
    ) -> bool {
        let shutdown = {
            let agents = self.agents.read().await;
            agents
                .get(process_session.agent_id())
                .filter(|entry| &entry.process_session == process_session)
                .map(|entry| {
                    (
                        entry.session_shutdown.clone(),
                        entry.pending.clone(),
                        entry.cli_pending_ids.clone(),
                        entry.approval_acks.clone(),
                        entry.ping_acks.clone(),
                    )
                })
        };
        let Some((shutdown, pending, cli_pending_ids, approval_acks, ping_acks)) = shutdown else {
            return false;
        };
        let _ = shutdown.send(true);
        tracing::warn!(
            agent_id = %process_session.agent_id(),
            session_id = %process_session.session_id(),
            %reason,
            "closing PC agent session"
        );
        self.recover_session_pending(
            process_session.agent_id(),
            &pending,
            &cli_pending_ids,
            reason,
        )
        .await;
        fail_pending_approvals(&approval_acks).await;
        fail_pending_pings(&ping_acks).await;
        true
    }
}

pub(super) async fn install_process_session(
    state: &Arc<AppState>,
    process_session: &AgentProcessSessionKey,
    entry: AgentEntry,
    owner_user_id: String,
) -> Result<()> {
    if &entry.process_session != process_session || entry.agent_id != process_session.agent_id() {
        tracing::error!(
            agent_id = %process_session.agent_id(),
            "refusing mismatched process-local agent session installation"
        );
        let _ = entry.session_shutdown.send(true);
        return Err(anyhow!("process-local agent session identity mismatch"));
    }
    let registry_device_name = entry.device_name.clone();
    let registry_hardware = entry.hardware.clone();
    let registry_storage = entry.storage.clone();
    let registry_dev_runtime = entry.dev_runtime.clone();
    let registry_lifecycle = entry.lifecycle.clone();
    let connected_at = entry.connected_at;

    // Lock order is always AgentManager -> NodeRegistry. The old session's
    // pending queues are drained only after both current projections are installed.
    let old_entry = {
        let mut agents = state.agent_manager.agents.write().await;
        let old_entry = agents.insert(process_session.agent_id().to_string(), entry);
        state
            .node_registry
            .register_exact(
                process_session.clone(),
                owner_user_id,
                registry_device_name,
                registry_hardware,
                registry_storage,
                registry_dev_runtime,
                registry_lifecycle,
                vec![],
                connected_at,
            )
            .await;
        old_entry
    };

    if let Some(old_entry) = old_entry {
        let old_session_id = old_entry.process_session.session_id().to_string();
        let _ = old_entry.session_shutdown.send(true);
        tracing::info!(
            agent_id = %process_session.agent_id(),
            %old_session_id,
            new_session_id = %process_session.session_id(),
            "evicting previous agent session (same agent_id re-registered)"
        );
        state
            .agent_manager
            .recover_session_pending(
                process_session.agent_id(),
                &old_entry.pending,
                &old_entry.cli_pending_ids,
                "PC 节点通信临时中断：Win 端正在更新升级/重启或节点重新注册，旧连接已关闭。",
            )
            .await;
        fail_pending_approvals(&old_entry.approval_acks).await;
        fail_pending_pings(&old_entry.ping_acks).await;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_current_session_capabilities(
    state: &Arc<AppState>,
    process_session: &AgentProcessSessionKey,
    owner_user_id: &str,
    device_name: Option<&str>,
    agent_version: &str,
    models: &[ModelCapability],
    allowed_clis: &[String],
    tts_worker_url: Option<&str>,
    hardware: Option<&NodeHardwareProfile>,
    storage: Option<&NodeStorageProfile>,
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    lifecycle: Option<&NodeLifecycleReport>,
) -> bool {
    // Keep the manager write guard through the Registry CAS. No reverse
    // Registry -> Manager acquisition exists.
    let mut agents = state.agent_manager.agents.write().await;
    let Some(entry) = agents.get_mut(process_session.agent_id()) else {
        return false;
    };
    if &entry.process_session != process_session {
        return false;
    }

    let handshake_clis = if allowed_clis.is_empty() {
        entry.allowed_clis.clone()
    } else {
        allowed_clis.to_vec()
    };
    if !owner_user_id.is_empty() {
        if let Some(hardware) = hardware {
            if let Err(error) = state.store.upsert_node_hardware_snapshot(
                process_session.agent_id(),
                owner_user_id,
                device_name,
                hardware,
            ) {
                tracing::warn!(
                    agent_id = %process_session.agent_id(),
                    error = %error,
                    "failed to update node hardware snapshot"
                );
            }
        }
        if let Err(error) = state.store.record_node_handshake(
            process_session.agent_id(),
            owner_user_id,
            agent_version,
            &handshake_clis,
            dev_runtime,
        ) {
            tracing::warn!(
                agent_id = %process_session.agent_id(),
                error = %error,
                "failed to record node capability handshake"
            );
        }
    }

    if !allowed_clis.is_empty() {
        entry.allowed_clis = allowed_clis.to_vec();
    }
    if let Some(hardware) = hardware {
        entry.hardware = Some(hardware.clone());
    }
    if let Some(storage) = storage {
        entry.storage = Some(storage.clone());
    }
    if let Some(dev_runtime) = dev_runtime {
        entry.dev_runtime = Some(dev_runtime.clone());
    }
    if let Some(lifecycle) = lifecycle {
        entry.lifecycle = Some(lifecycle.clone());
    }

    state
        .node_registry
        .update_capabilities_exact(
            process_session,
            models.to_vec(),
            tts_worker_url.map(str::to_string),
            hardware.cloned(),
            storage.cloned(),
            dev_runtime.cloned(),
            lifecycle.cloned(),
        )
        .await
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use tokio::sync::{mpsc, watch, Mutex};

    use super::*;

    fn entry(process_session: AgentProcessSessionKey) -> AgentEntry {
        let (cmd_tx, _) = mpsc::unbounded_channel();
        let (session_shutdown, _) = watch::channel(false);
        AgentEntry {
            agent_id: process_session.agent_id().to_string(),
            process_session,
            version: "test".to_string(),
            proto_version: homecli_proto::PROTO_VERSION,
            capabilities: Vec::new(),
            device_name: None,
            hardware: None,
            storage: None,
            dev_runtime: None,
            lifecycle: None,
            allowed_clis: Vec::new(),
            allowed_cwds: Vec::new(),
            connected_at: 0,
            cmd_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            cli_pending_ids: Arc::new(Mutex::new(HashSet::new())),
            approval_acks: Arc::new(Mutex::new(HashMap::new())),
            ping_acks: Arc::new(Mutex::new(HashMap::new())),
            session_shutdown,
        }
    }

    #[tokio::test]
    async fn stale_process_session_does_not_enter_sync_fence() {
        let manager = AgentManager::new();
        let current = AgentProcessSessionKey::new("agent", "current");
        let stale = AgentProcessSessionKey::new("agent", "stale");
        manager
            .agents
            .write()
            .await
            .insert("agent".to_string(), entry(current.clone()));
        let calls = AtomicUsize::new(0);

        assert!(manager
            .with_current_process_session(&current, |_| calls.fetch_add(1, Ordering::SeqCst))
            .await
            .is_some());
        assert!(manager
            .with_current_process_session(&stale, |_| calls.fetch_add(1, Ordering::SeqCst))
            .await
            .is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
