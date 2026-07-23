use serde::{Deserialize, Serialize};

/// PC 节点上报的单个模型能力描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model_id: String,
    pub display_name: String,
    pub context_len: u32,
    pub provider: String,
    pub price_per_1k_credits: f64,
}

/// PC 节点硬件画像。所有字段都是可选的，便于旧节点/受限环境渐进上报。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeHardwareProfile {
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub cpu_brand: Option<String>,
    #[serde(default)]
    pub cpu_cores: Option<u32>,
    #[serde(default)]
    pub memory_total_bytes: Option<u64>,
    #[serde(default)]
    pub gpu_names: Vec<String>,
    #[serde(default)]
    pub gpu_memory_total_bytes: Option<u64>,
    #[serde(default)]
    pub disk_free_bytes: Option<u64>,
}
