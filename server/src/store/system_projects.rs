//! 用户级系统项目。
//!
//! 这些项目不是用户主动开发的新代码仓库，而是固定业务入口的归档容器：
//! 悬浮球手机控制、普通聊天记忆等。它们仍然使用真实 project_id，
//! 这样会话列表、成员权限和记忆作用域都能复用项目体系。

use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, Store};

pub const PHONE_CONTROL_PROJECT_NAME: &str = "手机控制";
pub const CHAT_MEMORY_PROJECT_NAME: &str = "聊天记忆";
const PHONE_CONTROL_SOURCE_TYPE: &str = "agent_balloon";
const CHAT_MEMORY_SOURCE_TYPE: &str = "chat_memory";

struct SystemProjectSpec {
    name: &'static str,
    description: &'static str,
    source_type: &'static str,
    template: &'static str,
}

const PHONE_CONTROL_PROJECT: SystemProjectSpec = SystemProjectSpec {
    name: PHONE_CONTROL_PROJECT_NAME,
    description: "悬浮球语音助手的会话和脚本历史",
    source_type: PHONE_CONTROL_SOURCE_TYPE,
    template: PHONE_CONTROL_SOURCE_TYPE,
};

const CHAT_MEMORY_PROJECT: SystemProjectSpec = SystemProjectSpec {
    name: CHAT_MEMORY_PROJECT_NAME,
    description: "普通聊天会话、偏好和长期记忆",
    source_type: CHAT_MEMORY_SOURCE_TYPE,
    template: CHAT_MEMORY_SOURCE_TYPE,
};

impl Store {
    /// 为用户确保“手机控制”系统项目存在（幂等）。
    pub fn ensure_balloon_project_for_user(&self, user_id: &str) -> Result<(String, bool)> {
        self.ensure_system_project_for_user(user_id, &PHONE_CONTROL_PROJECT)
    }

    /// 为用户确保“聊天记忆”系统项目存在（幂等）。
    pub fn ensure_chat_memory_project_for_user(&self, user_id: &str) -> Result<(String, bool)> {
        self.ensure_system_project_for_user(user_id, &CHAT_MEMORY_PROJECT)
    }

    fn ensure_system_project_for_user(
        &self,
        user_id: &str,
        spec: &SystemProjectSpec,
    ) -> Result<(String, bool)> {
        let conn = self.conn()?;

        let existing: Option<String> = conn
            .query_row(
                "SELECT p.id FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE pm.user_id = ?1 AND pm.role = 'owner'
                   AND p.name = ?2
                   AND p.status = 'active'
                 ORDER BY CASE WHEN p.source_type = ?3 THEN 0 ELSE 1 END, p.created_at ASC
                 LIMIT 1",
                params![user_id, spec.name, spec.source_type],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            return Ok((id, false));
        }

        let id = new_id("prj");
        let now_str = now();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, 'active', ?6, ?7, ?7)",
            params![
                id,
                spec.name,
                spec.description,
                spec.template,
                spec.source_type,
                user_id,
                now_str
            ],
        )?;
        tx.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![id, user_id, now_str],
        )?;
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'system_project_created', ?4, ?5)",
            params![
                new_id("evt"),
                id,
                user_id,
                serde_json::json!({
                    "name": spec.name,
                    "source_type": spec.source_type,
                })
                .to_string(),
                now_str
            ],
        )?;
        tx.commit()?;
        Ok((id, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_system_projects_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn system_projects_are_idempotent_and_distinct() {
        let store = temp_store();
        let user = store
            .create_user("system-projects@example.com", "secret1", None, None)
            .expect("user should be created");

        let (phone_id, phone_created) = store
            .ensure_balloon_project_for_user(&user.id)
            .expect("phone project should be created");
        let (phone_id_again, phone_created_again) = store
            .ensure_balloon_project_for_user(&user.id)
            .expect("phone project should be reused");
        let (chat_id, chat_created) = store
            .ensure_chat_memory_project_for_user(&user.id)
            .expect("chat memory project should be created");

        assert!(phone_created);
        assert!(!phone_created_again);
        assert!(chat_created);
        assert_eq!(phone_id, phone_id_again);
        assert_ne!(phone_id, chat_id);

        let names = store
            .list_projects_for_user(&user.id)
            .expect("projects should list")
            .into_iter()
            .map(|project| project.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&PHONE_CONTROL_PROJECT_NAME.to_string()));
        assert!(names.contains(&CHAT_MEMORY_PROJECT_NAME.to_string()));
    }
}
