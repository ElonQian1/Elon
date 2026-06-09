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
pub(crate) const PHONE_CONTROL_SOURCE_TYPE: &str = "agent_balloon";
pub(crate) const CHAT_MEMORY_SOURCE_TYPE: &str = "chat_memory";

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

        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT p.id, p.source_type FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE pm.user_id = ?1 AND pm.role = 'owner'
                   AND p.name = ?2
                   AND p.status = 'active'
                 ORDER BY CASE WHEN p.source_type = ?3 THEN 0 ELSE 1 END, p.created_at ASC
                 LIMIT 1",
                params![user_id, spec.name, spec.source_type],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        if let Some((id, source_type)) = existing {
            if source_type != spec.source_type {
                conn.execute(
                    "UPDATE projects
                     SET source_type = ?2,
                         template = ?3,
                         description = CASE
                             WHEN description IS NULL OR TRIM(description) = '' THEN ?4
                             ELSE description
                         END,
                         is_public = 0,
                         join_mode = 'invite',
                         updated_at = ?5
                     WHERE id = ?1",
                    params![id, spec.source_type, spec.template, spec.description, now()],
                )?;
            }
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

pub(crate) fn is_system_project_name(name: &str) -> bool {
    matches!(
        name.trim(),
        PHONE_CONTROL_PROJECT_NAME | CHAT_MEMORY_PROJECT_NAME
    )
}

pub(crate) fn system_project_key_for_source_type(source_type: &str) -> Option<&'static str> {
    match source_type.trim() {
        PHONE_CONTROL_SOURCE_TYPE => Some("phone_control"),
        CHAT_MEMORY_SOURCE_TYPE => Some("chat_memory"),
        _ => None,
    }
}

pub(crate) fn is_system_project_source_type(source_type: &str) -> bool {
    system_project_key_for_source_type(source_type).is_some()
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

    #[test]
    fn ensure_system_project_normalizes_legacy_metadata() {
        let store = temp_store();
        let user = store
            .create_user("legacy-system-projects@example.com", "secret1", None, None)
            .expect("user should be created");
        let legacy_id = new_id("prj");
        let now_str = now();
        let conn = store.conn().expect("conn should open");
        conn.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, is_public, join_mode, created_at, updated_at
             )
             VALUES (?1, ?2, '旧数据', ?1, 'android', 'template',
                     'active', ?3, 1, 'open', ?4, ?4)",
            params![legacy_id, PHONE_CONTROL_PROJECT_NAME, user.id, now_str],
        )
        .expect("legacy project should insert");
        conn.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![legacy_id, user.id, now_str],
        )
        .expect("legacy membership should insert");
        drop(conn);

        let (project_id, created) = store
            .ensure_balloon_project_for_user(&user.id)
            .expect("system project should be normalized");

        assert_eq!(project_id, legacy_id);
        assert!(!created);

        let access = store
            .get_project_access(&user.id, &project_id)
            .expect("project should still exist");
        assert_eq!(access.source_type, PHONE_CONTROL_SOURCE_TYPE);
    }
}
