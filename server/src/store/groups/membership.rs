//! Transactional, idempotent membership commands. No client can grant itself authority.
use super::{new_id, now, roster::access, Store};
use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MembershipCommand {
    pub request_id: String,
    pub action: String,
    #[serde(default)]
    pub user_ids: Vec<String>,
    pub role: Option<String>,
    pub invitation_policy: Option<String>,
    pub invitation_id: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct MembershipReceipt {
    pub ok: bool,
    pub changed: usize,
    pub message: String,
    pub exited: bool,
}

pub(super) fn notice(conn: &Connection, group: &str, content: &str, timestamp: &str) -> Result<()> {
    conn.execute("INSERT INTO users(id,password_hash,nickname,status,password_login_enabled,created_at,updated_at)
        VALUES('usr_group_notifications','','群通知','disabled',0,?1,?1) ON CONFLICT(id) DO NOTHING",[timestamp])?;
    conn.execute(
        "INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at)
        VALUES(?1,?2,'usr_group_notifications',?3,?4)",
        params![new_id("gnotice"), group, content, timestamp],
    )?;
    conn.execute(
        "UPDATE friend_groups SET updated_at=?2 WHERE id=?1",
        params![group, timestamp],
    )?;
    Ok(())
}

fn name(conn: &Connection, id: &str) -> Result<String> {
    let value: String = conn.query_row(
        "SELECT COALESCE(NULLIF(nickname,''),'群成员') FROM users WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    Ok(value.chars().filter(|c| !c.is_control()).take(80).collect())
}

fn member_role(conn: &Connection, group: &str, user: &str, owner: &str) -> Result<String> {
    ensure!(
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id=?1 AND user_id=?2)",
            params![group, user],
            |r| r.get::<_, bool>(0)
        )?,
        "所选用户已不在群聊中，请刷新"
    );
    if user == owner {
        return Ok("owner".into());
    }
    let admin: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_admins WHERE group_id=?1 AND user_id=?2)",
        params![group, user],
        |r| r.get(0),
    )?;
    Ok(if admin { "admin" } else { "member" }.into())
}

pub(super) fn invite(
    conn: &Connection,
    actor: &str,
    group: &str,
    ids: &[String],
    timestamp: &str,
) -> Result<usize> {
    // Validate the whole batch before writing; one invalid target must not partially invite people.
    for id in ids {
        let allowed: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM user_friends f JOIN users u ON u.id=f.friend_user_id
            WHERE f.user_id=?1 AND f.friend_user_id=?2 AND u.status!='disabled')",
            params![actor, id],
            |r| r.get(0),
        )?;
        ensure!(allowed, "只能邀请已添加且可用的好友");
    }
    let mut changed = 0;
    for id in ids {
        if conn.execute("INSERT INTO friend_group_members(group_id,user_id,created_at,last_read_at) VALUES(?1,?2,?3,?3)
            ON CONFLICT(group_id,user_id) DO NOTHING",params![group,id,timestamp])?>0 {
            notice(conn,group,&format!("{} 邀请 {} 加入了群聊",name(conn,actor)?,name(conn,id)?),timestamp)?;
            changed+=1;
        }
    }
    Ok(changed)
}

impl Store {
    pub(crate) fn group_membership_command(
        &self,
        actor: &str,
        group: &str,
        command: &MembershipCommand,
    ) -> Result<MembershipReceipt> {
        ensure!(
            (8..=128).contains(&command.request_id.len())
                && command
                    .request_id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "无效的请求标识"
        );
        ensure!(command.user_ids.len() <= 100, "每次最多操作 100 人");
        let serialized = serde_json::to_string(command)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let prior:Option<(String,String,String)>=tx.query_row("SELECT actor_id,request_json,receipt_json FROM friend_group_membership_actions WHERE group_id=?1 AND request_id=?2",
            params![group,command.request_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((owner, request, receipt)) = prior {
            ensure!(
                owner == actor && request == serialized,
                "REQUEST_CONFLICT: 请求标识已用于其他操作"
            );
            return Ok(serde_json::from_str(&receipt)?);
        }
        let current = access(&tx, actor, group)?;
        let ids: Vec<String> = command
            .user_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let timestamp = now();
        let mut receipt = MembershipReceipt {
            ok: true,
            changed: 0,
            message: "操作已完成".into(),
            exited: false,
        };
        match command.action.as_str() {
            "invite" => {
                ensure!(!ids.is_empty(), "请至少选择一位好友");
                ensure!(
                    current.role != "member" || current.policy != "admins",
                    "GROUP_PERMISSION_DENIED: 仅群主或管理员可以邀请成员"
                );
                if current.role == "member" && current.policy == "approval" {
                    // Check friendship even for pending requests.
                    for id in &ids {
                        ensure!(tx.query_row("SELECT EXISTS(SELECT 1 FROM user_friends WHERE user_id=?1 AND friend_user_id=?2)",params![actor,id],|r|r.get::<_,bool>(0))?,"只能邀请已添加的好友");
                    }
                    tx.execute("INSERT INTO friend_group_invitation_requests(id,group_id,actor_id,user_ids,created_at) VALUES(?1,?2,?3,?4,?5)",
                        params![new_id("ginvite"),group,actor,serde_json::to_string(&ids)?,timestamp])?;
                    receipt.message = "邀请已提交，等待群主或管理员审核".into();
                } else {
                    receipt.changed = invite(&tx, actor, group, &ids, &timestamp)?;
                    receipt.message = format!("已邀请 {} 位新成员", receipt.changed);
                }
            }
            "remove" => {
                ensure!(
                    current.role != "member",
                    "GROUP_PERMISSION_DENIED: 仅群主或管理员可以移除成员"
                );
                ensure!(!ids.is_empty(), "请先选择成员");
                for id in &ids {
                    let role = member_role(&tx, group, id, &current.owner)?;
                    ensure!(
                        id != actor
                            && role != "owner"
                            && (current.role == "owner" || role == "member"),
                        "GROUP_PERMISSION_DENIED: 不能移除自己、群主或同级管理员"
                    );
                }
                for id in &ids {
                    tx.execute(
                        "DELETE FROM friend_group_members WHERE group_id=?1 AND user_id=?2",
                        params![group, id],
                    )?;
                    notice(
                        &tx,
                        group,
                        &format!("{} 将 {} 移出了群聊", name(&tx, actor)?, name(&tx, id)?),
                        &timestamp,
                    )?;
                }
                receipt.changed = ids.len();
                receipt.message = format!("已移出 {} 位成员", ids.len());
            }
            "role" => {
                ensure!(
                    current.role == "owner",
                    "GROUP_PERMISSION_DENIED: 只有群主可以设置管理员"
                );
                ensure!(ids.len() == 1, "请选择一位成员");
                ensure!(ids[0] != current.owner, "群主不能修改自己的角色");
                let previous = member_role(&tx, group, &ids[0], &current.owner)?;
                let role = command.role.as_deref().unwrap_or("");
                ensure!(["admin", "member"].contains(&role), "无效的成员角色");
                if previous != role {
                    if role == "admin" {
                        tx.execute(
                            "INSERT INTO friend_group_admins(group_id,user_id) VALUES(?1,?2)",
                            params![group, ids[0]],
                        )?;
                    } else {
                        tx.execute(
                            "DELETE FROM friend_group_admins WHERE group_id=?1 AND user_id=?2",
                            params![group, ids[0]],
                        )?;
                    }
                    notice(
                        &tx,
                        group,
                        &format!(
                            "{} {}",
                            name(&tx, &ids[0])?,
                            if role == "admin" {
                                "成为了管理员"
                            } else {
                                "不再担任管理员"
                            }
                        ),
                        &timestamp,
                    )?;
                    receipt.changed = 1;
                }
            }
            "transfer" => {
                ensure!(
                    current.role == "owner",
                    "GROUP_PERMISSION_DENIED: 只有群主可以转让群聊"
                );
                ensure!(ids.len() == 1 && ids[0] != actor, "请选择另一位群成员");
                member_role(&tx, group, &ids[0], &current.owner)?;
                tx.execute(
                    "UPDATE friend_groups SET owner_user_id=?2 WHERE id=?1",
                    params![group, ids[0]],
                )?;
                tx.execute(
                    "DELETE FROM friend_group_admins WHERE group_id=?1 AND user_id IN (?2,?3)",
                    params![group, actor, ids[0]],
                )?;
                notice(
                    &tx,
                    group,
                    &format!(
                        "{} 将群主转让给了 {}",
                        name(&tx, actor)?,
                        name(&tx, &ids[0])?
                    ),
                    &timestamp,
                )?;
                receipt.changed = 1;
                receipt.message = "群主已转让，你仍是群成员".into();
            }
            "policy" => {
                ensure!(
                    current.role == "owner",
                    "GROUP_PERMISSION_DENIED: 只有群主可以修改邀请规则"
                );
                let policy = command.invitation_policy.as_deref().unwrap_or("");
                ensure!(
                    ["members", "admins", "approval"].contains(&policy),
                    "无效的邀请规则"
                );
                receipt.changed=tx.execute("UPDATE friend_group_management SET invitation_policy=?2,revision=revision+1 WHERE group_id=?1 AND invitation_policy!=?2",params![group,policy])?;
                if receipt.changed > 0 {
                    notice(&tx, group, "群主更新了成员邀请规则", &timestamp)?;
                }
            }
            "approve" | "reject" => {
                ensure!(
                    current.role != "member",
                    "GROUP_PERMISSION_DENIED: 只有群主或管理员可以审核邀请"
                );
                let invitation = command
                    .invitation_id
                    .as_deref()
                    .ok_or_else(|| anyhow!("缺少邀请标识"))?;
                let pending:Option<(String,String,String)>=tx.query_row("SELECT actor_id,user_ids,status FROM friend_group_invitation_requests WHERE group_id=?1 AND id=?2",params![group,invitation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
                let (requester, targets, status) = pending.ok_or_else(|| anyhow!("邀请不存在"))?;
                ensure!(status == "pending", "这条邀请已处理，请刷新");
                if command.action == "approve" {
                    access(&tx, &requester, group)?;
                    receipt.changed = invite(
                        &tx,
                        &requester,
                        group,
                        &serde_json::from_str::<Vec<String>>(&targets)?,
                        &timestamp,
                    )?;
                }
                tx.execute(
                    "UPDATE friend_group_invitation_requests SET status=?2 WHERE id=?1",
                    params![
                        invitation,
                        if command.action == "approve" {
                            "approved"
                        } else {
                            "rejected"
                        }
                    ],
                )?;
                receipt.message = if command.action == "approve" {
                    "邀请已通过"
                } else {
                    "邀请已拒绝"
                }
                .into();
            }
            "leave" | "dissolve" => {
                let count: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM friend_group_members WHERE group_id=?1",
                    [group],
                    |r| r.get(0),
                )?;
                if command.action == "dissolve" {
                    ensure!(
                        current.role == "owner",
                        "GROUP_PERMISSION_DENIED: 只有群主可以解散群聊"
                    );
                } else {
                    ensure!(
                        current.role != "owner" || count == 1,
                        "请先转让群主，再退出群聊"
                    );
                }
                let dissolve = command.action == "dissolve" || count == 1;
                notice(
                    &tx,
                    group,
                    &format!(
                        "{} {}",
                        name(&tx, actor)?,
                        if dissolve {
                            "解散了群聊"
                        } else {
                            "退出了群聊"
                        }
                    ),
                    &timestamp,
                )?;
                if dissolve {
                    tx.execute("UPDATE friend_group_management SET dissolved=1,revision=revision+1 WHERE group_id=?1",[group])?;
                    receipt.changed = tx.execute(
                        "DELETE FROM friend_group_members WHERE group_id=?1",
                        [group],
                    )?;
                    tx.execute("UPDATE friend_group_invitation_requests SET status='rejected' WHERE group_id=?1 AND status='pending'",[group])?;
                } else {
                    receipt.changed = tx.execute(
                        "DELETE FROM friend_group_members WHERE group_id=?1 AND user_id=?2",
                        params![group, actor],
                    )?;
                }
                receipt.exited = true;
                receipt.message = if dissolve {
                    "群聊已解散"
                } else {
                    "已退出群聊"
                }
                .into();
            }
            _ => return Err(anyhow!("不支持的成员操作")),
        }
        tx.execute("INSERT INTO friend_group_membership_actions(group_id,request_id,actor_id,request_json,receipt_json,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![group,command.request_id,actor,serialized,serde_json::to_string(&receipt)?,timestamp])?;
        tx.commit()?;
        Ok(receipt)
    }
}
