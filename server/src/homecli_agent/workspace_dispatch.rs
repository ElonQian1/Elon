use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use homecli_proto::{AgentToServer, ServerToAgent};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use crate::types::AppState;
use super::{AgentManager, AgentEntry};
use super::agent_session::{
    project_storage_prepare_timeout, project_workspace_inspect_timeout,
    project_workspace_provision_timeout,
};

impl AgentManager {
    pub async fn dispatch_project_workspace_provision(
        &self,
        agent_id: &str,
        project_id: String,
        user_id: String,
        name: String,
        template: String,
        repo_url: Option<String>,
        branch: Option<String>,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        let send_result = agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::ProvisionProjectWorkspace {
                req_id: req_id.clone(),
                project_id,
                user_id,
                name,
                template,
                repo_url,
                branch,
            })
            .map_err(|_| anyhow!("agent writer closed"));
        if let Err(error) = send_result {
            pending.lock().await.remove(&req_id);
            return Err(error);
        }
        drop(agents);
        let timeout = project_workspace_provision_timeout();
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before provisioning response")),
            Err(_) => {
                pending.lock().await.remove(&req_id);
                Err(anyhow!(
                    "PC 节点创建项目工作区超时（{} 秒），请确认本机助手仍在运行后重试",
                    timeout.as_secs()
                ))
            }
        }
    }
    /// Ask a storage-capable PC node to create or reuse a bare Git repo for a project.
    pub async fn dispatch_project_storage_repo_prepare(
        &self,
        agent_id: &str,
        project_id: String,
        user_id: String,
        name: String,
        branch: Option<String>,
        access_token: Option<String>,
        prepare_worktree: bool,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        let send_result = agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::PrepareProjectStorageRepo {
                req_id: req_id.clone(),
                project_id,
                user_id,
                name,
                branch,
                access_token,
                prepare_worktree,
            })
            .map_err(|_| anyhow!("agent writer closed"));
        if let Err(error) = send_result {
            pending.lock().await.remove(&req_id);
            return Err(error);
        }
        drop(agents);

        let timeout = project_storage_prepare_timeout();
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before storage repo response")),
            Err(_) => {
                pending.lock().await.remove(&req_id);
                Err(anyhow!(
                    "PC 节点准备代码存储超时（{} 秒），请稍后重试或先不启用代码存储",
                    timeout.as_secs()
                ))
            }
        }
    }

    /// Ask a PC node to inspect a project workspace and return a single status frame.
    pub async fn dispatch_project_workspace_inspect(
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
            .send(homecli_proto::ServerToAgent::InspectProjectWorkspace {
                req_id: req_id.clone(),
                workspace_path,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let timeout = project_workspace_inspect_timeout();
        let outcome = match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!(
                "agent disconnected before workspace inspect response"
            )),
            Err(_) => Err(anyhow!(
                "project workspace inspect timeout ({}s)",
                timeout.as_secs()
            )),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// Ask a PC node to read fixed project documentation from a workspace.
    pub async fn dispatch_project_documents_read(
        &self,
        agent_id: &str,
        workspace_path: String,
        seed_defaults: bool,
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
            .send(homecli_proto::ServerToAgent::ReadProjectDocuments {
                req_id: req_id.clone(),
                workspace_path,
                seed_defaults,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let outcome = match tokio::time::timeout(Duration::from_secs(8), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before project docs response")),
            Err(_) => Err(anyhow!("project docs read timeout (8s)")),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// Ask a PC node to cleanup a managed project workspace and return a single status frame.
    pub async fn dispatch_project_workspace_cleanup(
        &self,
        agent_id: &str,
        project_id: String,
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
            .send(homecli_proto::ServerToAgent::CleanupProjectWorkspace {
                req_id: req_id.clone(),
                project_id,
                workspace_path,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let outcome = match tokio::time::timeout(Duration::from_secs(45), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!(
                "agent disconnected before workspace cleanup response"
            )),
            Err(_) => Err(anyhow!("project workspace cleanup timeout (45s)")),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// 向 PC 节点发起 TTS 合成请求，返回 TtsSynthesizeResponse 或 TtsSynthesizeError。
    /// timeout 设 180s（模型首次加载可能需要较长时间）。
    pub async fn dispatch_tts(
        &self,
        agent_id: &str,
        text: String,
        voice_id: Option<String>,
        emotion_id: Option<String>,
        intensity: Option<String>,
        provider: Option<String>,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("TTS agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::TtsSynthesizeRequest {
                req_id: req_id.clone(),
                text,
                voice_id,
                emotion_id,
                intensity,
                provider,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);
        match tokio::time::timeout(Duration::from_secs(180), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("TTS agent disconnected before response")),
            Err(_) => Err(anyhow!("TTS synthesis timeout (180s)")),
        }
    }
}
