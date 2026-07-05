use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, ServerToAgent};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::homecli_agent::AgentManager;

impl AgentManager {
    pub async fn dispatch_project_git_worktree_audit(
        &self,
        agent_id: &str,
        workspace_path: String,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(ServerToAgent::AuditProjectGitWorktrees {
                req_id: req_id.clone(),
                workspace_path,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let timeout = Duration::from_secs(12);
        let outcome = match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!(
                "agent disconnected before git worktree audit response"
            )),
            Err(_) => Err(anyhow!(
                "project git worktree audit timeout ({}s)",
                timeout.as_secs()
            )),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }
}
