use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{
    project_module_types::{
        map_checkpoint, map_context_artifact, map_conversation, map_memory, map_workspace,
    },
    ProjectModuleCheckpoint, ProjectModuleContextArtifact, ProjectModuleConversation,
    ProjectModuleMemory, ProjectModuleWorkspace, Store, UiTunerWorkspaceBundle,
    UI_TUNER_MODULE_KEY,
};

impl Store {
    pub(crate) fn ui_tuner_workspace_bundle(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<UiTunerWorkspaceBundle> {
        Ok(UiTunerWorkspaceBundle {
            workspace: self.get_ui_tuner_workspace(project_id, user_id)?,
            sessions: self.list_ui_tuner_sessions(project_id, user_id)?,
            memories: self.list_ui_tuner_memories(project_id, user_id)?,
            latest_checkpoint: self.latest_ui_tuner_checkpoint(project_id, user_id, None)?,
        })
    }

    pub(crate) fn get_ui_tuner_context_artifact(
        &self,
        project_id: &str,
        user_id: &str,
        artifact_id: &str,
    ) -> Result<ProjectModuleContextArtifact> {
        self.conn()?.query_row(
            "SELECT id, conversation_id, schema_version, payload_json, payload_sha256,
                    selected_element_name, resource_id, source_file, user_intent, task_id, created_at, updated_at
             FROM project_module_context_artifacts
             WHERE id = ?1 AND project_id = ?2 AND user_id = ?3 AND module_key = ?4",
            params![artifact_id, project_id, user_id, UI_TUNER_MODULE_KEY], map_context_artifact,
        ).map_err(Into::into)
    }

    pub(crate) fn latest_ui_tuner_fork_point(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<(Option<String>, String)>> {
        let conn = self.conn()?;
        let checkpoint = conn
            .query_row(
                "SELECT id, source_message_id FROM project_module_checkpoints
             WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3 AND conversation_id = ?4
               AND status = 'done' ORDER BY created_at DESC LIMIT 1",
                params![project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id],
                |row| Ok((Some(row.get(0)?), row.get(1)?)),
            )
            .optional()?;
        if checkpoint.is_some() {
            return Ok(checkpoint);
        }
        conn.query_row(
            "SELECT id FROM messages WHERE project_id = ?1 AND conversation_id = ?2
             ORDER BY created_at DESC, id DESC LIMIT 1",
            params![project_id, conversation_id],
            |row| Ok((None, row.get(0)?)),
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn get_ui_tuner_workspace(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<ProjectModuleWorkspace> {
        self.conn()?.query_row(
            "SELECT project_id, user_id, module_key, canonical_conversation_id, active_conversation_id,
                    stable_summary, memory_revision, last_checkpoint_id, created_at, updated_at
             FROM project_module_workspaces WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3",
            params![project_id, user_id, UI_TUNER_MODULE_KEY], map_workspace,
        ).map_err(Into::into)
    }

    pub(crate) fn list_ui_tuner_memories(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<Vec<ProjectModuleMemory>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, owner_user_id, scope_type, category, content, status, importance,
                    source_conversation_id, source_message_id, source_task_id, reviewed_by,
                    reviewed_at, created_at, updated_at
             FROM project_module_memories
             WHERE project_id = ?1 AND module_key = ?2 AND (scope_type = 'project' OR owner_user_id = ?3)
             ORDER BY CASE status WHEN 'accepted' THEN 0 WHEN 'candidate' THEN 1 ELSE 2 END,
                      importance DESC, updated_at DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map(
                params![project_id, UI_TUNER_MODULE_KEY, user_id],
                map_memory,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(super) fn list_ui_tuner_sessions(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<Vec<ProjectModuleConversation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT conversation_id, title, is_canonical, parent_conversation_id, source_message_id,
                    source_checkpoint_id, selected_element_name, status, last_task_id, created_at, updated_at
             FROM project_module_conversations
             WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3
             ORDER BY is_canonical DESC, updated_at DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map(
                params![project_id, user_id, UI_TUNER_MODULE_KEY],
                map_conversation,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(super) fn get_ui_tuner_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<ProjectModuleConversation> {
        self.conn()?.query_row(
            "SELECT conversation_id, title, is_canonical, parent_conversation_id, source_message_id,
                    source_checkpoint_id, selected_element_name, status, last_task_id, created_at, updated_at
             FROM project_module_conversations
             WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3 AND conversation_id = ?4",
            params![project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id], map_conversation,
        ).map_err(Into::into)
    }

    pub(super) fn require_ui_tuner_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<()> {
        self.get_ui_tuner_session(project_id, user_id, conversation_id)
            .map(|_| ())
    }

    pub(super) fn latest_ui_tuner_checkpoint(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
    ) -> Result<Option<ProjectModuleCheckpoint>> {
        self.conn()?
            .query_row(
                "SELECT id, conversation_id, source_message_id, task_id, context_artifact_id,
                    memory_revision, status, summary, created_at
             FROM project_module_checkpoints
             WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3
               AND (?4 IS NULL OR conversation_id = ?4) ORDER BY created_at DESC LIMIT 1",
                params![project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id],
                map_checkpoint,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn ui_tuner_context_checkpoint(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<ProjectModuleCheckpoint>> {
        self.conn()?
            .query_row(
                "SELECT id, conversation_id, source_message_id, task_id, context_artifact_id,
                        memory_revision, status, summary, created_at
                 FROM project_module_checkpoints
                 WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3
                   AND (conversation_id = ?4 OR id = (
                     SELECT source_checkpoint_id FROM project_module_conversations
                     WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3 AND conversation_id = ?4
                   ))
                 ORDER BY CASE WHEN conversation_id = ?4 THEN 0 ELSE 1 END, created_at DESC LIMIT 1",
                params![project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id],
                map_checkpoint,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(super) fn get_ui_tuner_memory(
        &self,
        project_id: &str,
        user_id: &str,
        memory_id: &str,
    ) -> Result<ProjectModuleMemory> {
        self.conn()?
            .query_row(
                "SELECT id, owner_user_id, scope_type, category, content, status, importance,
                    source_conversation_id, source_message_id, source_task_id, reviewed_by,
                    reviewed_at, created_at, updated_at
             FROM project_module_memories WHERE id = ?1 AND project_id = ?2 AND module_key = ?3
               AND (scope_type = 'project' OR owner_user_id = ?4)",
                params![memory_id, project_id, UI_TUNER_MODULE_KEY, user_id],
                map_memory,
            )
            .map_err(Into::into)
    }
}
