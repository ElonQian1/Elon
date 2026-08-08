use super::AgentToServer;

impl AgentToServer {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::TaskStdout { task_id, .. }
            | Self::TaskStderr { task_id, .. }
            | Self::TaskExit { task_id, .. }
            | Self::TaskError { task_id, .. } => Some(task_id.as_str()),
            Self::Register { .. }
            | Self::Pong { .. }
            | Self::HttpResponse { .. }
            | Self::HttpError { .. }
            | Self::CliPromptAccepted { .. }
            | Self::CliChunk { .. }
            | Self::CliDone { .. }
            | Self::CliCompletionReplay { .. }
            | Self::CliLocalTaskSync { .. }
            | Self::CliTaskJournalSnapshot { .. }
            | Self::ToolApprovalDecisionAck { .. }
            | Self::ComputePluginSharingPolicyObservedV1 { .. }
            | Self::ComputePluginInstallPlanPreparationObservedV1 { .. }
            | Self::RegisterCapabilities { .. }
            | Self::LlmStreamChunk { .. }
            | Self::LlmStreamEnd { .. }
            | Self::LlmStreamError { .. }
            | Self::ProjectWorkspaceProvisioned { .. }
            | Self::ProjectWorkspaceProvisionError { .. }
            | Self::ProjectStorageRepoReady { .. }
            | Self::ProjectStorageRepoError { .. }
            | Self::ProjectWorkspaceInspected { .. }
            | Self::ProjectWorkspaceInspectError { .. }
            | Self::ProjectGitWorktreesAudited { .. }
            | Self::ProjectGitWorktreeAuditError { .. }
            | Self::ProjectDocumentsRead { .. }
            | Self::ProjectDocumentsReadError { .. }
            | Self::ProjectDocumentFederationRead { .. }
            | Self::ProjectDocumentFederationReadError { .. }
            | Self::ProjectDocumentFileRead { .. }
            | Self::ProjectDocumentFileReadError { .. }
            | Self::ProjectDocumentFileWritten { .. }
            | Self::ProjectDocumentFileWriteError { .. }
            | Self::ProjectWorkspaceCleaned { .. }
            | Self::ProjectWorkspaceCleanupError { .. }
            | Self::TtsSynthesizeResponse { .. }
            | Self::TtsSynthesizeError { .. } => None,
        }
    }

    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::HttpResponse { req_id, .. }
            | Self::HttpError { req_id, .. }
            | Self::CliPromptAccepted { req_id, .. }
            | Self::CliChunk { req_id, .. }
            | Self::CliDone { req_id, .. }
            | Self::CliTaskJournalSnapshot { req_id, .. }
            | Self::ComputePluginSharingPolicyObservedV1 { req_id, .. }
            | Self::ComputePluginInstallPlanPreparationObservedV1 { req_id, .. }
            | Self::LlmStreamChunk { req_id, .. }
            | Self::LlmStreamEnd { req_id, .. }
            | Self::LlmStreamError { req_id, .. }
            | Self::ProjectWorkspaceProvisioned { req_id, .. }
            | Self::ProjectWorkspaceProvisionError { req_id, .. }
            | Self::ProjectStorageRepoReady { req_id, .. }
            | Self::ProjectStorageRepoError { req_id, .. }
            | Self::ProjectWorkspaceInspected { req_id, .. }
            | Self::ProjectWorkspaceInspectError { req_id, .. }
            | Self::ProjectGitWorktreesAudited { req_id, .. }
            | Self::ProjectGitWorktreeAuditError { req_id, .. }
            | Self::ProjectDocumentsRead { req_id, .. }
            | Self::ProjectDocumentsReadError { req_id, .. }
            | Self::ProjectDocumentFederationRead { req_id, .. }
            | Self::ProjectDocumentFederationReadError { req_id, .. }
            | Self::ProjectDocumentFileRead { req_id, .. }
            | Self::ProjectDocumentFileReadError { req_id, .. }
            | Self::ProjectDocumentFileWritten { req_id, .. }
            | Self::ProjectDocumentFileWriteError { req_id, .. }
            | Self::ProjectWorkspaceCleaned { req_id, .. }
            | Self::ProjectWorkspaceCleanupError { req_id, .. }
            | Self::TtsSynthesizeResponse { req_id, .. }
            | Self::TtsSynthesizeError { req_id, .. } => Some(req_id.as_str()),
            _ => None,
        }
    }

    /// 流式消息需要保留在 pending map 中（还有后续），其余 req_id 消息在发送后删除。
    pub fn is_final_req_msg(&self) -> bool {
        !matches!(
            self,
            Self::CliPromptAccepted { .. } | Self::CliChunk { .. } | Self::LlmStreamChunk { .. }
        )
    }
}
