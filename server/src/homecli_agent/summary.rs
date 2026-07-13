use homecli_proto::{
    NodeDevRuntimeProfile, NodeHardwareProfile, NodeLifecycleReport, NodeStorageProfile,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub version: String,
    pub proto_version: u32,
    pub capabilities: Vec<String>,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub lifecycle: Option<NodeLifecycleReport>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
}
