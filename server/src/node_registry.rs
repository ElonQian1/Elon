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

use homecli_proto::{
    ModelCapability, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile,
};
use serde::Serialize;
use tokio::sync::RwLock;

const NODE_TIMEOUT: Duration = Duration::from_secs(90);

// ── 数据结构 ──────────────────────────────────────────────────────────────────

pub struct NodeEntry {
    pub node_id: String,
    /// 节点属主的用户 ID（积分归属）
    pub owner_user_id: String,
    /// PC 设备名，仅用于展示。
    pub device_name: Option<String>,
    /// PC 硬件画像，用于算力市场展示。
    pub hardware: Option<NodeHardwareProfile>,
    /// PC 项目代码硬盘服务能力。
    pub storage: Option<NodeStorageProfile>,
    /// PC 开发运行时能力。
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    /// 该节点支持的 LLM 模型列表
    pub models: Vec<ModelCapability>,
    /// 本机 TTS Worker URL（如 http://127.0.0.1:5011）——空表示无 TTS 能力
    pub tts_worker_url: Option<String>,
    /// 首次连接时间戳（Unix 秒）
    pub connected_at: u64,
    /// 最后一次收到心跳的时刻（用于 TTL 判断）
    pub last_seen: Instant,
}

/// 供 HTTP API 返回的节点摘要（不含内部状态）
#[derive(Debug, Clone, Serialize)]
pub struct NodeSummary {
    pub node_id: String,
    pub owner_user_id: String,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub models: Vec<ModelCapability>,
    pub tts_worker_url: Option<String>,
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
        device_name: Option<String>,
        hardware: Option<NodeHardwareProfile>,
        storage: Option<NodeStorageProfile>,
        dev_runtime: Option<NodeDevRuntimeProfile>,
        models: Vec<ModelCapability>,
        connected_at: u64,
    ) {
        let entry = NodeEntry {
            node_id: node_id.clone(),
            owner_user_id,
            device_name,
            hardware,
            storage,
            dev_runtime,
            models,
            tts_worker_url: None,
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
    pub async fn update_capabilities(
        &self,
        node_id: &str,
        models: Vec<ModelCapability>,
        tts_worker_url: Option<String>,
        hardware: Option<NodeHardwareProfile>,
        storage: Option<NodeStorageProfile>,
        dev_runtime: Option<NodeDevRuntimeProfile>,
    ) {
        if let Some(entry) = self.nodes.write().await.get_mut(node_id) {
            entry.models = models;
            if tts_worker_url.is_some() {
                entry.tts_worker_url = tts_worker_url;
            }
            if hardware.is_some() {
                entry.hardware = hardware;
            }
            if storage.is_some() {
                entry.storage = storage;
            }
            if dev_runtime.is_some() {
                entry.dev_runtime = dev_runtime;
            }
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
    /// 兼容旧调用；自动路由的新质量调度在 node_router 中完成。
    pub async fn find_node_for_model(&self, model_id: &str) -> Option<String> {
        self.list_candidates_for_model(model_id)
            .await
            .into_iter()
            .next()
            .map(|entry| entry.node_id)
    }

    /// 指定节点时严格校验该节点在线且支持模型；未指定时走自动匹配。
    pub async fn find_node_for_model_target(
        &self,
        model_id: &str,
        target_node_id: Option<&str>,
    ) -> Option<String> {
        let target = target_node_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if target.is_none() {
            return self.find_node_for_model(model_id).await;
        }
        let target = target?;
        let nodes = self.nodes.read().await;
        nodes
            .get(target)
            .filter(|entry| entry.last_seen.elapsed() < NODE_TIMEOUT)
            .filter(|entry| entry.models.iter().any(|m| m.model_id == model_id))
            .map(|entry| entry.node_id.clone())
    }

    /// 列出所有已知节点（含 online 状态标记）。
    pub async fn list_online(&self) -> Vec<NodeSummary> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .map(|e| NodeSummary {
                node_id: e.node_id.clone(),
                owner_user_id: e.owner_user_id.clone(),
                device_name: e.device_name.clone(),
                hardware: e.hardware.clone(),
                storage: e.storage.clone(),
                dev_runtime: e.dev_runtime.clone(),
                models: e.models.clone(),
                tts_worker_url: e.tts_worker_url.clone(),
                connected_at: e.connected_at,
                online: e.last_seen.elapsed() < NODE_TIMEOUT,
            })
            .collect()
    }

    /// 列出支持指定模型且在线的节点候选，供质量调度器排序。
    pub async fn list_candidates_for_model(&self, model_id: &str) -> Vec<NodeSummary> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| e.last_seen.elapsed() < NODE_TIMEOUT)
            .filter(|e| e.models.iter().any(|m| m.model_id == model_id))
            .map(|e| NodeSummary {
                node_id: e.node_id.clone(),
                owner_user_id: e.owner_user_id.clone(),
                device_name: e.device_name.clone(),
                hardware: e.hardware.clone(),
                storage: e.storage.clone(),
                dev_runtime: e.dev_runtime.clone(),
                models: e.models.clone(),
                tts_worker_url: e.tts_worker_url.clone(),
                connected_at: e.connected_at,
                online: true,
            })
            .collect()
    }

    /// 找到某用户的在线节点中，第一个有 TTS Worker 的节点。
    /// 返回 (node_id, tts_worker_url)。
    pub async fn find_tts_node_for_user(&self, owner_user_id: &str) -> Option<(String, String)> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| {
                e.last_seen.elapsed() < NODE_TIMEOUT
                    && e.owner_user_id == owner_user_id
                    && e.tts_worker_url.is_some()
            })
            .find_map(|e| e.tts_worker_url.clone().map(|url| (e.node_id.clone(), url)))
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

    /// 返回节点属主的 user_id（积分结算时使用）。
    pub async fn get_node_owner(&self, node_id: &str) -> Option<String> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).map(|e| e.owner_user_id.clone())
    }

    /// 返回节点上某个模型的报价（积分 / 1k tokens）。
    pub async fn get_node_model_price(&self, node_id: &str, model_id: &str) -> Option<f64> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).and_then(|e| {
            e.models
                .iter()
                .find(|m| m.model_id == model_id)
                .map(|m| m.price_per_1k_credits)
        })
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
                device_name: e.device_name.clone(),
                hardware: e.hardware.clone(),
                storage: e.storage.clone(),
                dev_runtime: e.dev_runtime.clone(),
                models: e.models.clone(),
                tts_worker_url: e.tts_worker_url.clone(),
                connected_at: e.connected_at,
                online: e.last_seen.elapsed() < NODE_TIMEOUT,
            })
            .collect()
    }
}

/// 全局单例，通过 Arc<NodeRegistry> 放入 AppState。
pub type SharedNodeRegistry = Arc<NodeRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str) -> ModelCapability {
        ModelCapability {
            model_id: id.to_string(),
            display_name: id.to_string(),
            context_len: 4096,
            provider: "test".to_string(),
            price_per_1k_credits: 1.0,
        }
    }

    #[tokio::test]
    async fn target_node_must_be_online_and_support_model() {
        let registry = NodeRegistry::new();
        registry
            .register(
                "node-a".to_string(),
                "user-a".to_string(),
                Some("PC-A".to_string()),
                None,
                None,
                None,
                vec![model("qwen")],
                1,
            )
            .await;
        registry
            .register(
                "node-b".to_string(),
                "user-b".to_string(),
                Some("PC-B".to_string()),
                None,
                None,
                None,
                vec![model("llama")],
                1,
            )
            .await;

        assert_eq!(
            registry
                .find_node_for_model_target("qwen", Some("node-a"))
                .await
                .as_deref(),
            Some("node-a")
        );
        assert_eq!(
            registry
                .find_node_for_model_target("qwen", Some("node-b"))
                .await,
            None
        );
        assert_eq!(
            registry
                .find_node_for_model_target("qwen", Some("missing"))
                .await,
            None
        );
    }
}
