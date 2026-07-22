//! HTTP request DTO for owner-only local task creation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateLocalTaskRequest {
    pub(super) project_id: String,
    #[serde(default)]
    pub(super) channel_id: Option<String>,
    #[serde(default)]
    pub(super) conversation_id: Option<String>,
    pub(super) workspace_path: String,
    pub(super) prompt: String,
    #[serde(default)]
    pub(super) runtime_permission: Option<String>,
    #[serde(default)]
    pub(super) supervision:
        Option<crate::node_agent_local_task_supervision::SupervisionContractInput>,
    #[serde(default)]
    pub(super) contract_revision:
        Option<crate::node_agent_local_task_contract_revision::ContractRevisionInput>,
}

impl CreateLocalTaskRequest {
    pub(super) fn validate_contract_revision(
        &self,
        supervision: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    ) -> Result<(), &'static str> {
        if self.contract_revision.is_some()
            && !supervision.is_some_and(|contract| contract.task_role == "resume_original")
        {
            return Err("contract_revision 只允许用于当前监督协议的 resume_original。");
        }
        Ok(())
    }
}
