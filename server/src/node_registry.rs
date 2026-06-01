//! 分布式计算节点注册中心。
//!
//! 每台用户 PC 运行 elon-node-agent 后，通过反向 WebSocket 隧道连接到中心服务器。
//! 注册时上报自身的 LLM 能力列表；服务器保存在此内存注册表中。
//! TTL 90s 超时自动标记为离线（实际断开时 homecli_agent 会立即 unregister）。

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use homecli_proto::ModelCapability;
use serde::Serialize;
use tokio::sync::RwLock;

const NODE_TIMEOUT: Duration = Duration::from_secs(90);

// ── 数据结构 ──────────────────────────────────────────────────────────────────

pub struct NodeEntry {
    pub node_id: String,
    /// 节点属主的用户 ID（积分归属）
    pub owner_user_id: String,
    /// 该节点支持的 LLM 模型列表
    pub models: Vec<ModelCapability>,
    /// 首次连接时间戳（Unix 秒）
    pub connected_at: u64,
    /// 最后一次收到心跳的时刻（用于 TTL 判断）
    pub last_seen: Instant,
}

/// 供 HTTP API 返回的节点摘要（不含内部状态）
#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub node_id: String,
    pub owner_user_id: String,
    pub models: Vec<ModelCapability>,
    pub connected_at: u64,
    /// 在 90s TTL 内 = online
    pub online: bool,
}

// ── 注册中心 ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct NodeRegistry {
    nodes: RwLock<HashMap<String, NodeEntry>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 节点上线：创建或覆盖注册条目。
    pub async fn register(
        &self,
        node_id: String,
        owner_user_id: String,
        models: Vec<ModelCapability>,
        connected_at: u64,
    ) {
        let entry = NodeEntry {
            node_id: node_id.clone(),
            owner_user_id,
            models,
            connected_at,
            last_seen: Instant::now(),
        };
        self.nodes.write().await.insert(node_id, entry);
    }

    /// 节点断开：立即删除注册条目。
    pub async fn unregister(&self, node_id: &str) {
        self.nodes.write().await.remove(node_id);
    }

    /// 节点更新能力列表（RegisterCapabilities 消息触发）。
    pub async fn update_capabilities(&self, node_id: &str, models: Vec<ModelCapability>) {
        if let Some(entry) = self.nodes.write().await.get_mut(node_id) {
            entry.models = models;
            entry.last_seen = Instant::now();
        }
    }

    /// 刷新节点最后活跃时间（收到心跳 Pong 时调用）。
    pub async fn touch(&self, node_id: &str) {
        if let Some(entry) = self.nodes.write().await.get_mut(node_id) {
            entry.last_seen = Instant::now();
        }
    }

    /// 为指定模型寻找一个在线节点，返回 node_id。
    /// 当前策略：随机选第一个在线且支持该模型的节点（后续可换成最低负载）。
    pub async fn find_node_for_model(&self, model_id: &str) -> Option<String> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| e.last_seen.elapsed() < NODE_TIMEOUT)
            .find(|e| e.models.iter().any(|m| m.model_id == model_id))
            .map(|e| e.node_id.clone())
    }

    /// 列出所有已知节点（含 online 状态标记）。
    pub async fn list_online(&self) -> Vec<NodeSummary> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .map(|e| NodeSummary {
                node_id: e.node_id.clone(),
                owner_user_id: e.owner_user_id.clone(),
                models: e.models.clone(),
                connected_at: e.connected_at,
                online: e.last_seen.elapsed() < NODE_TIMEOUT,
            })
            .collect()
    }

    /// 仅列出在线节点中所有可用模型的去重列表。
    pub async fn available_models(&self) -> Vec<ModelCapability> {
        let nodes = self.nodes.read().await;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for entry in nodes.values() {
            if entry.last_seen.elapsed() >= NODE_TIMEOUT {
                continue;
            }
            for m in &entry.models {
                if seen.insert(m.model_id.clone()) {
                    result.push(m.clone());
                }
            }
        }
        result
    }

    /// 返回属于指定用户的节点列表。
    pub async fn list_by_owner(&self, owner_user_id: &str) -> Vec<NodeSummary> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| e.owner_user_id == owner_user_id)
            .map(|e| NodeSummary {
                node_id: e.node_id.clone(),
                owner_user_id: e.owner_user_id.clone(),
                models: e.models.clone(),
                connected_at: e.connected_at,
                online: e.last_seen.elapsed() < NODE_TIMEOUT,
            })
            .collect()
    }
}

/// 全局单例，通过 Arc<NodeRegistry> 放入 AppState。
pub type SharedNodeRegistry = Arc<NodeRegistry>;
