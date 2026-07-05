use super::AgentManager;
use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, ServerToAgent};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

impl AgentManager {
    /// Ask a connected PC node for a local CLI task journal / recovery snapshot.
    ///
    /// This is intentionally a WS protocol request rather than a relay call to the node's
    /// local admin HTTP API, because the cloud server must not know or bypass the local
    /// browser admin token.
    pub async fn dispatch_cli_task_journal_inspect(
        &self,
        agent_id: &str,
        task_id: &str,
        since: usize,
        limit: usize,
        timeout: Duration,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        let send_result = agent.cmd_tx.send(ServerToAgent::InspectCliTaskJournal {
            req_id: req_id.clone(),
            task_id: task_id.to_string(),
            since,
            limit,
        });
        if send_result.is_err() {
            pending.lock().await.remove(&req_id);
            return Err(anyhow!("agent writer closed"));
        }
        drop(agents);

        let outcome = match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before task journal response")),
            Err(_) => Err(anyhow!(
                "PC 节点任务 journal 查询超时（{} 秒）",
                timeout.as_secs()
            )),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }
}
