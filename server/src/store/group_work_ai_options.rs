//! Immutable per-request work-AI choices, attached to the existing group reply owner.
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::{ensure_member_and_source, now, web, Store};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupWorkAiOptions {
    pub agent: Option<String>,
    #[serde(default = "default_fallback")]
    pub allow_fallback: bool,
}

fn default_fallback() -> bool {
    true
}
impl Default for GroupWorkAiOptions {
    fn default() -> Self {
        Self {
            agent: None,
            allow_fallback: true,
        }
    }
}

impl GroupWorkAiOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(agent) = &self.agent {
            ensure!(
                !agent.trim().is_empty()
                    && agent.len() <= 160
                    && agent.trim() == agent
                    && !agent.chars().any(char::is_control),
                "无效的群聊模型选项"
            );
        }
        Ok(())
    }
}

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS group_ai_work_options (
        request_id TEXT PRIMARY KEY REFERENCES group_ai_reply_requests(id) ON DELETE CASCADE,
        agent TEXT, allow_fallback INTEGER NOT NULL CHECK(allow_fallback IN (0,1))
    );",
    )?;
    Ok(())
}

fn read(conn: &Connection, id: &str) -> Result<Option<GroupWorkAiOptions>> {
    Ok(conn
        .query_row(
            "SELECT agent,allow_fallback FROM group_ai_work_options WHERE request_id=?1",
            [id],
            |r| {
                Ok(GroupWorkAiOptions {
                    agent: r.get(0)?,
                    allow_fallback: r.get(1)?,
                })
            },
        )
        .optional()?)
}

impl Store {
    pub(crate) fn activate_group_work_ai(
        &self,
        user: &str,
        group: &str,
        id: &str,
        operation: &str,
        options: &GroupWorkAiOptions,
    ) -> Result<web::WebGroupRequest> {
        options.validate()?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let before = web::read_owned(&tx, user, group, id, operation)?;
        if let Some(saved) = read(&tx, id)? {
            ensure!(
                &saved == options && before.engine == "server_api",
                "请求模型已固定，不能更改或重发"
            );
        } else {
            ensure!(
                before.state == "prepared" && before.engine == "chatgpt_web",
                "请求已经派发或取消，不能切换工作 AI"
            );
            tx.execute("INSERT INTO group_ai_work_options(request_id,agent,allow_fallback) VALUES (?1,?2,?3)",
                params![id, options.agent, options.allow_fallback])?;
            tx.execute("UPDATE group_ai_reply_requests SET engine='server_api',state='server_ready',updated_at=?1 WHERE id=?2",
                params![now(), id])?;
        }
        let result = web::read_owned(&tx, user, group, id, operation)?;
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn group_work_ai_options(
        &self,
        user: &str,
        request: &str,
    ) -> Result<Option<GroupWorkAiOptions>> {
        let conn = self.conn()?;
        let (group, source): (String,String) = conn.query_row(
            "SELECT group_id,trigger_message_id FROM group_ai_reply_requests WHERE id=?1 AND requester_id=?2 AND engine='server_api'",
            params![request,user], |r| Ok((r.get(0)?,r.get(1)?)))?;
        ensure_member_and_source(&conn, user, &group, &source)?;
        read(&conn, request)
    }
}
