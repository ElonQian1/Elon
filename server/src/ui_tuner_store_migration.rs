use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v96(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_module_workspaces (
          project_id                TEXT NOT NULL,
          user_id                   TEXT NOT NULL,
          module_key                TEXT NOT NULL,
          canonical_conversation_id TEXT NOT NULL,
          active_conversation_id    TEXT NOT NULL,
          stable_summary            TEXT NOT NULL,
          memory_revision           INTEGER NOT NULL DEFAULT 1,
          last_checkpoint_id        TEXT,
          created_at                TEXT NOT NULL,
          updated_at                TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id, module_key),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_module_conversations (
          project_id             TEXT NOT NULL,
          user_id                TEXT NOT NULL,
          module_key             TEXT NOT NULL,
          conversation_id        TEXT NOT NULL,
          title                  TEXT NOT NULL,
          is_canonical           INTEGER NOT NULL DEFAULT 0,
          parent_conversation_id TEXT,
          source_message_id      TEXT,
          source_checkpoint_id   TEXT,
          selected_element_name  TEXT,
          status                 TEXT NOT NULL DEFAULT 'active',
          last_task_id           TEXT,
          created_at             TEXT NOT NULL,
          updated_at             TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id, module_key, conversation_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_module_conversations_recent
          ON project_module_conversations(project_id, user_id, module_key, updated_at DESC);

        CREATE TABLE IF NOT EXISTS project_module_memories (
          id                     TEXT PRIMARY KEY,
          project_id             TEXT NOT NULL,
          owner_user_id          TEXT,
          module_key             TEXT NOT NULL,
          scope_type             TEXT NOT NULL DEFAULT 'user',
          category               TEXT NOT NULL DEFAULT 'requirement',
          content                TEXT NOT NULL,
          status                 TEXT NOT NULL DEFAULT 'candidate',
          importance             INTEGER NOT NULL DEFAULT 5,
          source_conversation_id TEXT,
          source_message_id      TEXT,
          source_task_id         TEXT,
          reviewed_by            TEXT,
          reviewed_at            TEXT,
          created_at             TEXT NOT NULL,
          updated_at             TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (owner_user_id) REFERENCES users(id),
          FOREIGN KEY (reviewed_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_module_memories_scope
          ON project_module_memories(project_id, module_key, status, owner_user_id, importance DESC, updated_at DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_project_module_memories_dedup
          ON project_module_memories(project_id, module_key, IFNULL(owner_user_id, ''), scope_type, category, content);

        CREATE TABLE IF NOT EXISTS project_module_context_artifacts (
          id                    TEXT PRIMARY KEY,
          project_id            TEXT NOT NULL,
          user_id               TEXT NOT NULL,
          module_key            TEXT NOT NULL,
          conversation_id       TEXT NOT NULL,
          schema_version        TEXT NOT NULL,
          payload_json          TEXT NOT NULL,
          payload_sha256        TEXT NOT NULL,
          selected_element_name TEXT,
          resource_id           TEXT,
          source_file           TEXT,
          user_intent           TEXT NOT NULL,
          task_id               TEXT,
          created_at            TEXT NOT NULL,
          updated_at            TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_module_context_recent
          ON project_module_context_artifacts(project_id, user_id, module_key, conversation_id, created_at DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_project_module_context_task
          ON project_module_context_artifacts(task_id) WHERE task_id IS NOT NULL;

        CREATE TABLE IF NOT EXISTS project_module_checkpoints (
          id                  TEXT PRIMARY KEY,
          project_id          TEXT NOT NULL,
          user_id             TEXT NOT NULL,
          module_key          TEXT NOT NULL,
          conversation_id     TEXT NOT NULL,
          source_message_id   TEXT NOT NULL,
          task_id             TEXT NOT NULL,
          context_artifact_id TEXT,
          memory_revision     INTEGER NOT NULL,
          status              TEXT NOT NULL,
          summary             TEXT NOT NULL,
          created_at          TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id),
          FOREIGN KEY (context_artifact_id) REFERENCES project_module_context_artifacts(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_module_checkpoints_recent
          ON project_module_checkpoints(project_id, user_id, module_key, conversation_id, created_at DESC);
        "#,
    )?;
    Ok(())
}
