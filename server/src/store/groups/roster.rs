use anyhow::{anyhow, ensure, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::Store;

#[derive(Default, Deserialize)]
pub(crate) struct RosterQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub filter: String,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RosterMember {
    pub id: String,
    pub display_name: String,
    pub avatar_data_url: Option<String>,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Roster {
    pub group_id: String,
    pub name: String,
    pub total_count: i64,
    pub matched_count: i64,
    pub revision: i64,
    pub viewer_id: String,
    pub viewer_role: String,
    pub invitation_policy: String,
    pub permissions: Permissions,
    pub members: Vec<RosterMember>,
    pub next_cursor: Option<String>,
    pub pending_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct Permissions {
    pub invite: bool,
    pub manage: bool,
    pub owner: bool,
}

pub(super) struct Access {
    pub name: String,
    pub owner: String,
    pub role: String,
    pub policy: String,
    pub revision: i64,
}

pub(super) fn access(conn: &Connection, actor: &str, group: &str) -> Result<Access> {
    conn.query_row("SELECT g.name,g.owner_user_id,s.invitation_policy,s.revision,
        CASE WHEN g.owner_user_id=?2 THEN 'owner' WHEN a.user_id IS NOT NULL THEN 'admin' ELSE 'member' END
        FROM friend_groups g JOIN friend_group_management s ON s.group_id=g.id
        JOIN friend_group_members m ON m.group_id=g.id AND m.user_id=?2
        LEFT JOIN friend_group_admins a ON a.group_id=g.id AND a.user_id=?2
        WHERE g.id=?1 AND s.dissolved=0", params![group,actor], |r| Ok(Access {
            name:r.get(0)?,owner:r.get(1)?,policy:r.get(2)?,revision:r.get(3)?,role:r.get(4)?,
        })).optional()?.ok_or_else(|| anyhow!("GROUP_ACCESS_DENIED: 你已不在这个群聊中，或群聊已解散"))
}

#[derive(Serialize, Deserialize)]
struct Cursor {
    group: String,
    revision: i64,
    q: String,
    filter: String,
    offset: i64,
}

impl Store {
    pub(crate) fn group_roster(
        &self,
        actor: &str,
        group: &str,
        query: &RosterQuery,
    ) -> Result<Roster> {
        let conn = self.conn()?;
        let access = access(&conn, actor, group)?;
        let q = query.q.trim();
        ensure!(q.chars().count() <= 80, "搜索内容最多 80 个字符");
        let filter = query.filter.as_str();
        ensure!(
            ["", "all", "admins", "recent"].contains(&filter),
            "无效的成员筛选"
        );
        let offset = if let Some(value) = &query.cursor {
            ensure!(value.len() <= 2048, "无效的分页游标");
            let cursor: Cursor = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(value)?)?;
            ensure!(
                cursor.group == group
                    && cursor.q == q
                    && cursor.filter == filter
                    && cursor.offset >= 0,
                "无效的分页游标"
            );
            ensure!(
                cursor.revision == access.revision,
                "ROSTER_CHANGED: 成员名单已更新，请刷新后继续"
            );
            cursor.offset
        } else {
            0
        };
        let limit = query.limit.unwrap_or(50).clamp(1, 100) as i64;
        let total_count = conn.query_row(
            "SELECT COUNT(*) FROM friend_group_members WHERE group_id=?1",
            [group],
            |r| r.get(0),
        )?;
        let from = "FROM friend_group_members m JOIN users u ON u.id=m.user_id
            JOIN friend_groups g ON g.id=m.group_id
            LEFT JOIN friend_group_admins a ON a.group_id=m.group_id AND a.user_id=m.user_id
            WHERE m.group_id=?1 AND (?2='' OR instr(lower(COALESCE(NULLIF(u.nickname,''),u.id)),lower(?2))>0)
            AND (?3!='admins' OR u.id=g.owner_user_id OR a.user_id IS NOT NULL)";
        let matched_count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) {from}"),
            params![group, q, filter],
            |r| r.get(0),
        )?;
        let mut stmt=conn.prepare(&format!("SELECT u.id,COALESCE(NULLIF(u.nickname,''),u.id),u.avatar_data_url,
            CASE WHEN u.id=g.owner_user_id THEN 'owner' WHEN a.user_id IS NOT NULL THEN 'admin' ELSE 'member' END,m.created_at
            {from} ORDER BY CASE WHEN ?3='recent' THEN 0 WHEN u.id=g.owner_user_id THEN 0 WHEN a.user_id IS NOT NULL THEN 1 ELSE 2 END,
            CASE WHEN ?3='recent' THEN m.created_at ELSE '' END DESC,u.id LIMIT ?4 OFFSET ?5"))?;
        let members = stmt
            .query_map(params![group, q, filter, limit, offset], |r| {
                Ok(RosterMember {
                    id: r.get(0)?,
                    display_name: r.get(1)?,
                    avatar_data_url: r.get(2)?,
                    role: r.get(3)?,
                    joined_at: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let next = offset + members.len() as i64;
        let next_cursor = if next < matched_count {
            Some(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&Cursor {
                group: group.into(),
                revision: access.revision,
                q: q.into(),
                filter: filter.into(),
                offset: next,
            })?))
        } else {
            None
        };
        let manage = access.role != "member";
        let pending_count = if manage {
            conn.query_row("SELECT COUNT(*) FROM friend_group_invitation_requests WHERE group_id=?1 AND status='pending'",[group],|r|r.get(0))?
        } else {
            0
        };
        Ok(Roster {
            group_id: group.into(),
            name: access.name,
            total_count,
            matched_count,
            revision: access.revision,
            viewer_id: actor.into(),
            permissions: Permissions {
                invite: manage || access.policy != "admins",
                manage,
                owner: access.role == "owner",
            },
            viewer_role: access.role,
            invitation_policy: access.policy,
            members,
            next_cursor,
            pending_count,
        })
    }
}
