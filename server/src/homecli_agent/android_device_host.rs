use std::time::Duration;

use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, AndroidDeviceHostRequest, ServerToAgent};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::AgentManager;

impl AgentManager {
    /// Dispatch a project-authorized request to the node's restricted Android
    /// device-host relay. Generic PC relay traffic cannot produce this frame.
    pub async fn dispatch_android_device_host_http(
        &self,
        agent_id: &str,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body_b64: Option<String>,
        timeout: Duration,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        if !agent
            .capabilities
            .iter()
            .any(|value| value == homecli_proto::CAP_ANDROID_DEVICE_HOST_V1)
        {
            return Err(anyhow!(
                "agent does not support shared Android device hosting"
            ));
        }
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        let request = AndroidDeviceHostRequest {
            req_id: req_id.clone(),
            method,
            path,
            headers,
            body_b64,
        };
        if agent
            .cmd_tx
            .send(ServerToAgent::AndroidDeviceHostRequest { request })
            .is_err()
        {
            pending.lock().await.remove(&req_id);
            return Err(anyhow!("agent writer closed"));
        }
        drop(agents);
        let result = match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(message)) => Ok(message),
            Ok(None) => Err(anyhow!("agent disconnected before Android host response")),
            Err(_) => Err(anyhow!(
                "Android device host timeout ({}s)",
                timeout.as_secs()
            )),
        };
        pending.lock().await.remove(&req_id);
        result
    }
}
