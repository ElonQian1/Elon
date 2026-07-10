use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{
    clean_optional, new_id, now,
    project_module_types::{
        UI_TUNER_DEFAULT_SUMMARY, UI_TUNER_MAIN_CONVERSATION_PREFIX, UI_TUNER_MAIN_TITLE,
    },
    safe_external_id, CreateUiTunerContextArtifact, ProjectModuleContextArtifact,
    ProjectModuleConversation, ProjectModuleMemory, Store, UiTunerWorkspaceBundle,
    UI_TUNER_MODULE_KEY,
};

impl Store {
    pub(crate) fn ensure_ui_tuner_workspace(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<UiTunerWorkspaceBundle> {
        let ts = now();
        // messages 表按 project + conversation 查询，因此主会话 ID 必须包含用户桶，避免跨用户串线。
        let canonical_conversation_id = format!(
            "{}-{}",
            UI_TUNER_MAIN_CONVERSATION_PREFIX,
            safe_external_id(user_id, "user")
        );
        let conn = self.conn()?;
        let migrated_active_conversation_id: Option<String> = conn
            .query_row(
                "SELECT id FROM conversations
                 WHERE project_id = ?1 AND user_id = ?2 AND id LIKE 'ui-tuner-%'
                 ORDER BY updated_at DESC LIMIT 1",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        let initial_active_conversation_id = migrated_active_conversation_id
            .as_deref()
            .unwrap_or(&canonical_conversation_id);
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO conversations (project_id, user_id, id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET updated_at = excluded.updated_at",
            params![project_id, user_id, canonical_conversation_id, UI_TUNER_MAIN_TITLE, ts],
        )?;
        tx.execute(
            "INSERT INTO project_module_workspaces
             (project_id, user_id, module_key, canonical_conversation_id, active_conversation_id,
              stable_summary, memory_revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)
             ON CONFLICT(project_id, user_id, module_key) DO NOTHING",
            params![
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY,
                canonical_conversation_id,
                initial_active_conversation_id,
                UI_TUNER_DEFAULT_SUMMARY,
                ts
            ],
        )?;
        tx.execute(
            "INSERT INTO project_module_conversations
             (project_id, user_id, module_key, conversation_id, title, is_canonical, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 'active', ?6, ?6)
             ON CONFLICT(project_id, user_id, module_key, conversation_id) DO NOTHING",
            params![project_id, user_id, UI_TUNER_MODULE_KEY, canonical_conversation_id, UI_TUNER_MAIN_TITLE, ts],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO project_module_conversations
             (project_id, user_id, module_key, conversation_id, title, is_canonical,
              status, last_task_id, created_at, updated_at)
             SELECT c.project_id, c.user_id, ?3, c.id,
                    COALESCE(NULLIF(c.title, ''), '微调画布 · 历史会话'), 0, c.status,
                    (SELECT t.id FROM tasks t
                     WHERE t.project_id = c.project_id AND t.user_id = c.user_id AND t.conversation_id = c.id
                     ORDER BY t.updated_at DESC LIMIT 1),
                    c.created_at, c.updated_at
             FROM conversations c
             WHERE c.project_id = ?1 AND c.user_id = ?2 AND c.id LIKE 'ui-tuner-%'",
            params![project_id, user_id, UI_TUNER_MODULE_KEY],
        )?;
        for (category, content, importance) in default_ui_tuner_memories() {
            tx.execute(
                "INSERT OR IGNORE INTO project_module_memories
                 (id, project_id, owner_user_id, module_key, scope_type, category, content,
                  status, importance, created_at, updated_at)
                 VALUES (?1, ?2, NULL, ?3, 'project', ?4, ?5, 'accepted', ?6, ?7, ?7)",
                params![
                    new_id("pmm"),
                    project_id,
                    UI_TUNER_MODULE_KEY,
                    category,
                    content,
                    importance,
                    ts
                ],
            )?;
        }
        tx.commit()?;
        drop(conn);
        self.ui_tuner_workspace_bundle(project_id, user_id)
    }

    pub(crate) fn create_ui_tuner_context_artifact(
        &self,
        project_id: &str,
        user_id: &str,
        input: CreateUiTunerContextArtifact<'_>,
    ) -> Result<ProjectModuleContextArtifact> {
        self.require_ui_tuner_session(project_id, user_id, input.conversation_id)?;
        let id = new_id("ctx");
        let ts = now();
        self.conn()?.execute(
            "INSERT INTO project_module_context_artifacts
             (id, project_id, user_id, module_key, conversation_id, schema_version, payload_json,
              payload_sha256, selected_element_name, resource_id, source_file, user_intent, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
            params![id, project_id, user_id, UI_TUNER_MODULE_KEY, input.conversation_id,
                input.schema_version, input.payload_json, input.payload_sha256,
                clean_optional(input.selected_element_name), clean_optional(input.resource_id),
                clean_optional(input.source_file), input.user_intent, ts],
        )?;
        self.get_ui_tuner_context_artifact(project_id, user_id, &id)
    }

    pub(crate) fn bind_ui_tuner_task(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        artifact_id: &str,
        task_id: &str,
    ) -> Result<()> {
        let ts = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE project_module_context_artifacts SET task_id = ?1, updated_at = ?2
             WHERE id = ?3 AND project_id = ?4 AND user_id = ?5 AND module_key = ?6
               AND conversation_id = ?7 AND task_id IS NULL",
            params![
                task_id,
                ts,
                artifact_id,
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY,
                conversation_id
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!(
                "ui-tuner Context Artifact 不存在、会话不匹配或已被使用"
            ));
        }
        tx.execute(
            "UPDATE project_module_conversations SET last_task_id = ?1, status = 'running', updated_at = ?2
             WHERE project_id = ?3 AND user_id = ?4 AND module_key = ?5 AND conversation_id = ?6",
            params![task_id, ts, project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id],
        )?;
        tx.execute(
            "UPDATE project_module_workspaces SET active_conversation_id = ?1, updated_at = ?2
             WHERE project_id = ?3 AND user_id = ?4 AND module_key = ?5",
            params![
                conversation_id,
                ts,
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn register_ui_tuner_fork(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        title: &str,
        parent_conversation_id: &str,
        source_message_id: Option<&str>,
        source_checkpoint_id: Option<&str>,
        selected_element_name: Option<&str>,
    ) -> Result<ProjectModuleConversation> {
        let ts = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO conversations (project_id, user_id, id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET title = excluded.title, updated_at = excluded.updated_at",
            params![project_id, user_id, conversation_id, title, ts],
        )?;
        tx.execute(
            "INSERT INTO project_module_conversations
             (project_id, user_id, module_key, conversation_id, title, is_canonical,
              parent_conversation_id, source_message_id, source_checkpoint_id, selected_element_name,
              status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, 'active', ?10, ?10)",
            params![project_id, user_id, UI_TUNER_MODULE_KEY, conversation_id, title,
                parent_conversation_id, clean_optional(source_message_id), clean_optional(source_checkpoint_id),
                clean_optional(selected_element_name), ts],
        )?;
        tx.execute(
            "UPDATE project_module_workspaces SET active_conversation_id = ?1, updated_at = ?2
             WHERE project_id = ?3 AND user_id = ?4 AND module_key = ?5",
            params![
                conversation_id,
                ts,
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY
            ],
        )?;
        tx.commit()?;
        drop(conn);
        self.get_ui_tuner_session(project_id, user_id, conversation_id)
    }

    pub(crate) fn new_ui_tuner_conversation_id(&self) -> String {
        new_id("ui-tuner")
    }

    pub(crate) fn review_ui_tuner_memory(
        &self,
        project_id: &str,
        user_id: &str,
        memory_id: &str,
        decision: &str,
        scope_type: &str,
    ) -> Result<ProjectModuleMemory> {
        let status = match decision {
            "accepted" | "rejected" => decision,
            _ => return Err(anyhow!("decision 必须是 accepted 或 rejected")),
        };
        let scope_type = if scope_type == "project" {
            "project"
        } else {
            "user"
        };
        let owner_user_id = (scope_type == "user").then_some(user_id);
        let ts = now();
        let changed = self.conn()?.execute(
            "UPDATE project_module_memories
             SET status = ?1, scope_type = ?2, owner_user_id = ?3, reviewed_by = ?4,
                 reviewed_at = ?5, updated_at = ?5
             WHERE id = ?6 AND project_id = ?7 AND module_key = ?8
               AND (owner_user_id IS NULL OR owner_user_id = ?4)",
            params![
                status,
                scope_type,
                owner_user_id,
                user_id,
                ts,
                memory_id,
                project_id,
                UI_TUNER_MODULE_KEY
            ],
        )?;
        if changed != 1 {
            return Err(anyhow!("记忆不存在或无权审核"));
        }
        self.get_ui_tuner_memory(project_id, user_id, memory_id)
    }

    pub(crate) fn import_legacy_ui_tuner_memory(
        &self,
        project_id: &str,
        user_id: &str,
        stable_summary: &str,
        accepted_decisions: &[String],
        preferred_standards: &[String],
        open_questions: &[String],
    ) -> Result<UiTunerWorkspaceBundle> {
        self.ensure_ui_tuner_workspace(project_id, user_id)?;
        let ts = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        if !stable_summary.trim().is_empty() {
            tx.execute(
                "UPDATE project_module_workspaces SET stable_summary = ?1, updated_at = ?2
                 WHERE project_id = ?3 AND user_id = ?4 AND module_key = ?5
                   AND memory_revision = 1 AND last_checkpoint_id IS NULL",
                params![
                    stable_summary.trim(),
                    ts,
                    project_id,
                    user_id,
                    UI_TUNER_MODULE_KEY
                ],
            )?;
        }
        for (category, status, content) in
            legacy_memory_rows(accepted_decisions, preferred_standards, open_questions)
        {
            tx.execute(
                "INSERT OR IGNORE INTO project_module_memories
                 (id, project_id, owner_user_id, module_key, scope_type, category, content,
                  status, importance, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'user', ?5, ?6, ?7, 7, ?8, ?8)",
                params![
                    new_id("pmm"),
                    project_id,
                    user_id,
                    UI_TUNER_MODULE_KEY,
                    category,
                    content,
                    status,
                    ts
                ],
            )?;
        }
        tx.execute(
            "UPDATE project_module_workspaces SET memory_revision = 2, updated_at = ?1
             WHERE project_id = ?2 AND user_id = ?3 AND module_key = ?4
               AND memory_revision = 1 AND last_checkpoint_id IS NULL",
            params![ts, project_id, user_id, UI_TUNER_MODULE_KEY],
        )?;
        tx.commit()?;
        drop(conn);
        self.ui_tuner_workspace_bundle(project_id, user_id)
    }
}

fn default_ui_tuner_memories() -> [(&'static str, &'static str, i64); 4] {
    [
        (
            "architecture",
            "Codex 会话必须归属于自项目项目会话，并复用同一 conversation/native session。",
            10,
        ),
        (
            "context",
            "每个任务都携带当前选中 APK 元素的服务端 Context Artifact。",
            10,
        ),
        (
            "standard",
            "可复用 UI 标准必须保存为 tokens、components、screens 三层 JSON 配置。",
            9,
        ),
        (
            "filter",
            "产品模式默认隐藏结构容器、重复边界和非目标包节点。",
            8,
        ),
    ]
}

fn legacy_memory_rows<'a>(
    accepted_decisions: &'a [String],
    preferred_standards: &'a [String],
    open_questions: &'a [String],
) -> impl Iterator<Item = (&'static str, &'static str, &'a str)> {
    accepted_decisions
        .iter()
        .map(|value| ("legacy_decision", "accepted", value.as_str()))
        .chain(
            preferred_standards
                .iter()
                .map(|value| ("legacy_preference", "accepted", value.as_str())),
        )
        .chain(
            open_questions
                .iter()
                .map(|value| ("legacy_question", "candidate", value.as_str())),
        )
        .filter(|(_, _, value)| !value.trim().is_empty())
}
