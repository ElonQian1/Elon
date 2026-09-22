//! Per-member, provider-account-scoped project binding. No provider credentials are stored.
use super::Store;
use anyhow::{ensure, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct ProjectAction {
    pub action: String,
    pub account_scope: String,
    pub operation_id: String,
    pub lease_id: Option<String>,
    pub generation: Option<i64>,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub confirmed_missing: bool,
}

#[derive(Serialize)]
pub(crate) struct ProjectBinding {
    pub binding_id: String,
    pub generation: i64,
    pub state: String,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub lease_id: String,
    pub lease_until: i64,
    pub group_name: String,
}

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE group_chatgpt_projects (
        user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
        account_scope TEXT NOT NULL CHECK(length(account_scope)=64),
        binding_id TEXT NOT NULL UNIQUE, generation INTEGER NOT NULL DEFAULT 1,
        state TEXT NOT NULL DEFAULT 'empty' CHECK(state IN ('empty','creating','ready')),
        project_id TEXT, conversation_id TEXT, operation_id TEXT,
        lease_id TEXT NOT NULL DEFAULT '', lease_until INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY(user_id,group_id,account_scope),
        CHECK((state='ready' AND project_id IS NOT NULL) OR (state!='ready' AND project_id IS NULL))
    );",
    )?;
    Ok(())
}

fn valid_project(value: &str) -> bool {
    value
        .strip_prefix("g-p-")
        .is_some_and(|v| v.len() == 32 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn load(conn: &Connection, user: &str, group: &str, scope: &str) -> Result<ProjectBinding> {
    Ok(conn.query_row("SELECT p.binding_id,p.generation,p.state,p.project_id,p.conversation_id,
        p.lease_id,p.lease_until,g.name FROM group_chatgpt_projects p JOIN friend_groups g ON g.id=p.group_id
        WHERE p.user_id=?1 AND p.group_id=?2 AND p.account_scope=?3", params![user,group,scope], |r| {
        Ok(ProjectBinding { binding_id:r.get(0)?, generation:r.get(1)?, state:r.get(2)?,
            project_id:r.get(3)?, conversation_id:r.get(4)?, lease_id:r.get(5)?, lease_until:r.get(6)?,
            group_name:r.get::<_,Option<String>>(7)?.unwrap_or_else(|| "群聊".into()) })
    })?)
}

impl Store {
    pub(crate) fn group_chatgpt_project_action(
        &self,
        user: &str,
        group: &str,
        req: &ProjectAction,
    ) -> Result<ProjectBinding> {
        let mut conn = self.conn()?;
        apply(&mut conn, user, group, req, chrono::Utc::now().timestamp())
    }
}

fn apply(
    conn: &mut Connection,
    user: &str,
    group: &str,
    req: &ProjectAction,
    now: i64,
) -> Result<ProjectBinding> {
    ensure!(
        req.account_scope.len() == 64
            && req
                .account_scope
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "无效账号范围"
    );
    ensure!(
        uuid::Uuid::parse_str(&req.operation_id).is_ok(),
        "无效操作标识"
    );
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let member: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE user_id=?1 AND group_id=?2)",
        params![user, group],
        |r| r.get(0),
    )?;
    ensure!(member, "无法访问当前群聊");
    if req.action == "acquire" {
        tx.execute("INSERT OR IGNORE INTO group_chatgpt_projects(user_id,group_id,account_scope,binding_id) VALUES(?1,?2,?3,?4)",
            params![user,group,req.account_scope,uuid::Uuid::new_v4().to_string()])?;
        let current = load(&tx, user, group, &req.account_scope)?;
        // Even the same operation cannot acquire twice concurrently. A lost receipt
        // expires without permitting a second create, because 'creating' survives.
        ensure!(
            current.lease_until <= now,
            "另一个设备正在处理本群 AI，请稍后再试"
        );
        tx.execute(
            "UPDATE group_chatgpt_projects SET operation_id=?4,lease_id=?5,lease_until=?6
            WHERE user_id=?1 AND group_id=?2 AND account_scope=?3",
            params![
                user,
                group,
                req.account_scope,
                req.operation_id,
                uuid::Uuid::new_v4().to_string(),
                now + 300
            ],
        )?;
    } else {
        let current = load(&tx, user, group, &req.account_scope)?;
        let owns: bool = tx.query_row(
            "SELECT operation_id=?4 FROM group_chatgpt_projects
            WHERE user_id=?1 AND group_id=?2 AND account_scope=?3",
            params![user, group, req.account_scope, req.operation_id],
            |r| r.get(0),
        )?;
        ensure!(
            owns && current.lease_until > now
                && req.lease_id.as_deref() == Some(current.lease_id.as_str())
                && req.generation == Some(current.generation),
            "操作已过期，请重新核对群项目"
        );
        match req.action.as_str() {
            "create_begin" => {
                ensure!(current.state == "empty", "项目创建不能重复派发");
                tx.execute(
                    "UPDATE group_chatgpt_projects SET state='creating' WHERE binding_id=?1",
                    [&current.binding_id],
                )?;
            }
            "bind" => {
                let project = req.project_id.as_deref().unwrap_or("");
                ensure!(
                    valid_project(project)
                        && (current.state == "creating"
                            || current.project_id.as_deref() == Some(project)),
                    "项目回执不匹配"
                );
                tx.execute("UPDATE group_chatgpt_projects SET state='ready',project_id=?2 WHERE binding_id=?1",params![current.binding_id,project])?;
            }
            "conversation" => {
                let id = req.conversation_id.as_deref().unwrap_or("");
                ensure!(
                    current.state == "ready"
                        && req.project_id == current.project_id
                        && uuid::Uuid::parse_str(id).is_ok(),
                    "会话归属不匹配"
                );
                ensure!(
                    current.conversation_id.as_deref().is_none_or(|v| v == id),
                    "群会话已由其他操作绑定"
                );
                tx.execute(
                    "UPDATE group_chatgpt_projects SET conversation_id=?2 WHERE binding_id=?1",
                    params![current.binding_id, id],
                )?;
            }
            "rebuild" => {
                ensure!(
                    req.confirmed_missing
                        && current.state == "ready"
                        && req.project_id == current.project_id,
                    "需要确认原项目已删除"
                );
                tx.execute("UPDATE group_chatgpt_projects SET state='empty',project_id=NULL,conversation_id=NULL,generation=generation+1
                    WHERE binding_id=?1", [&current.binding_id])?;
            }
            "release" => {
                tx.execute("UPDATE group_chatgpt_projects SET lease_until=0,lease_id='' WHERE binding_id=?1", [&current.binding_id])?;
            }
            _ => anyhow::bail!("不支持的项目操作"),
        }
    }
    let result = load(&tx, user, group, &req.account_scope)?;
    tx.commit()?;
    Ok(result)
}

#[cfg(test)]
#[path = "group_chatgpt_project_tests.rs"]
mod tests;
