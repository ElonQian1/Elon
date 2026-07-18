use homecli_proto::CliWorkspaceStatus;

use super::AiCliRequestMode;

pub(super) fn pc_project_execution_had_no_changes(
    request_mode: AiCliRequestMode,
    lightweight_pc_chat: bool,
    workspace_status: Option<&CliWorkspaceStatus>,
    allow_artifact_only_delivery: bool,
) -> bool {
    if allow_artifact_only_delivery
        || lightweight_pc_chat
        || request_mode.is_plan()
        || request_mode.is_passthrough()
    {
        return false;
    }

    workspace_status
        .and_then(|status| status.merge_message.as_deref())
        .map(|message| message.contains("conversation branch had no new commits"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use homecli_proto::CliWorkspaceStatus;

    use super::{pc_project_execution_had_no_changes, AiCliRequestMode};

    #[test]
    fn pc_project_execution_detects_no_new_commits_as_no_changes() {
        let status = CliWorkspaceStatus {
            base_workspace_path: Some("D:/project".into()),
            active_workspace_path: "D:/project-worktree".into(),
            isolated: true,
            branch: Some("ai/session/project/conversation".into()),
            git_head: None,
            prepare_status: "prepared".into(),
            merge_status: Some("merged".into()),
            merge_message: Some("conversation branch had no new commits".into()),
        };

        assert!(pc_project_execution_had_no_changes(
            AiCliRequestMode::Execute,
            false,
            Some(&status),
            false
        ));
        assert!(!pc_project_execution_had_no_changes(
            AiCliRequestMode::Plan,
            false,
            Some(&status),
            false
        ));
        assert!(!pc_project_execution_had_no_changes(
            AiCliRequestMode::Execute,
            true,
            Some(&status),
            false
        ));
        assert!(!pc_project_execution_had_no_changes(
            AiCliRequestMode::Passthrough,
            false,
            Some(&status),
            false
        ));
        assert!(!pc_project_execution_had_no_changes(
            AiCliRequestMode::Execute,
            false,
            Some(&status),
            true
        ));
    }
}
