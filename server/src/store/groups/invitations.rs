use super::{roster::access, Store};
use anyhow::{ensure, Result};
use rusqlite::params;
use serde_json::{json, Value};

impl Store {
    pub(crate) fn group_pending_invitations(
        &self,
        actor: &str,
        group: &str,
        offset: usize,
    ) -> Result<Value> {
        let conn = self.conn()?;
        ensure!(
            access(&conn, actor, group)?.role != "member",
            "GROUP_PERMISSION_DENIED: 只有群主或管理员可以查看待审邀请"
        );
        ensure!(offset <= i64::MAX as usize, "无效的分页位置");
        let total:i64=conn.query_row("SELECT COUNT(*) FROM friend_group_invitation_requests WHERE group_id=?1 AND status='pending'",[group],|r|r.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT i.id,i.actor_id,COALESCE(NULLIF(u.nickname,''),u.id),i.user_ids,i.created_at
            FROM friend_group_invitation_requests i JOIN users u ON u.id=i.actor_id
            WHERE group_id=?1 AND i.status='pending' ORDER BY i.created_at,i.id LIMIT 50 OFFSET ?2",
        )?;
        let raw = stmt
            .query_map(params![group, offset as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut requests = Vec::new();
        for (id, actor_id, actor_name, ids, created_at) in raw {
            let mut people = Vec::new();
            for user in serde_json::from_str::<Vec<String>>(&ids)? {
                let name: String = conn.query_row(
                    "SELECT COALESCE(NULLIF(nickname,''),id) FROM users WHERE id=?1",
                    [&user],
                    |r| r.get(0),
                )?;
                people.push(json!({"id":user,"display_name":name}));
            }
            requests.push(json!({"id":id,"actor_id":actor_id,"actor_name":actor_name,"members":people,"created_at":created_at}));
        }
        let next = offset + requests.len();
        Ok(
            json!({"requests":requests,"total_count":total,"next_offset":if (next as i64)<total {Some(next)} else {None}}),
        )
    }
}
