use super::TimelineRequest;
use crate::store::{friend_messages, groups, project_space, SOCIAL_AI_USER_ID};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

pub(super) fn authorize(conn: &Connection, owner: &str, r: &TimelineRequest) -> Result<()> {
    if r.id.is_empty() || r.id.len() > 160 || r.project.len() > 160 {
        bail!("invalid_timeline_scope");
    }
    let allowed = match r.kind.as_str() {
        "group" if r.project.is_empty() => conn
            .query_row(
                "SELECT 1 FROM friend_group_members WHERE group_id=?1 AND user_id=?2",
                params![r.id, owner],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false),
        "friend" if r.project.is_empty() => {
            r.id == SOCIAL_AI_USER_ID
                || conn
                    .query_row(
                        "SELECT 1 FROM user_friends WHERE user_id=?1 AND friend_user_id=?2",
                        params![owner, r.id],
                        |_| Ok(true),
                    )
                    .optional()?
                    .unwrap_or(false)
        }
        "ai" => conn
            .query_row(
                "SELECT 1 FROM conversations WHERE project_id=?1 AND id=?2 AND user_id=?3 AND NOT EXISTS(SELECT 1 FROM conversations other WHERE other.project_id=?1 AND other.id=?2 AND other.user_id!=?3)",
                params![r.project, r.id, owner],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false),
        "channel" => {
            let live = conn
                .query_row(
                    "SELECT 1 FROM projects WHERE id=?1 AND status!='deleted'",
                    [&r.project],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            live && project_space::project_member_channel_permissions_locked(
                conn, &r.project, &r.id, owner,
            )?
            .can_view
        }
        _ => false,
    };
    if !allowed {
        bail!("timeline_access_denied");
    }
    Ok(())
}

pub(super) fn event_scope(owner: &str, r: &TimelineRequest) -> String {
    if r.kind == "friend" {
        if owner < r.id.as_str() {
            json!([owner, r.id]).to_string()
        } else {
            json!([r.id, owner]).to_string()
        }
    } else {
        r.id.clone()
    }
}

pub(super) fn messages(
    conn: &Connection,
    owner: &str,
    r: &TimelineRequest,
    before: Option<&(String, String)>,
    ids: Option<&[String]>,
    limit: usize,
) -> Result<Vec<Value>> {
    let (select, source, filter) = match r.kind.as_str() {
        "group" => ("m.id,m.group_id,m.sender_user_id,COALESCE(u.nickname,u.email,u.phone,m.sender_user_id),m.content,m.attachments_json,m.created_at,m.recalled_at,m.recalled_by,m.revision,m.edited_at",
            "friend_group_messages m JOIN users u ON u.id=m.sender_user_id", "m.group_id=:id"),
        "friend" => ("m.id,m.sender_user_id,m.receiver_user_id,COALESCE(u.nickname,u.email,u.phone,m.sender_user_id),m.content,m.attachments_json,m.created_at,m.context_user_id,m.recalled_at,m.recalled_by",
            "friend_messages m LEFT JOIN users u ON u.id=m.sender_user_id",
            "(((m.sender_user_id=:owner AND m.receiver_user_id=:id) OR (m.sender_user_id=:id AND m.receiver_user_id=:owner)) AND (:id!=:ai OR m.context_user_id IS NULL)) OR (m.sender_user_id=:ai AND m.receiver_user_id=:owner AND m.context_user_id=:id AND :id!=:ai)"),
        "ai" => ("m.id,m.project_id,m.conversation_id,m.task_id,m.user_id,CASE WHEN lower(m.role) IN ('assistant','ai') THEN '一龙AI' ELSE COALESCE(u.nickname,u.phone,u.email,m.user_id,m.role) END,m.role,m.content,m.created_at,m.user_id=:owner OR lower(m.role)='user',m.recalled_at,m.recalled_by",
            "messages m LEFT JOIN users u ON u.id=m.user_id", "m.project_id=:project AND m.conversation_id=:id"),
        "channel" => ("m.id,m.project_id,m.channel_id,m.sender_user_id,COALESCE(u.nickname,u.phone,u.email,m.sender_user_id),u.avatar_data_url,m.reply_to_message_id,m.kind,m.content,m.task_id,t.status,t.error,t.apk_url,t.codex_thread_id,m.suggestion_status,m.suggestion_resolved_by,COALESCE(resolver.nickname,resolver.phone,resolver.email,m.suggestion_resolved_by),m.suggestion_resolved_at,m.created_at,m.recalled_at,m.recalled_by",
            "project_channel_messages m LEFT JOIN users u ON u.id=m.sender_user_id LEFT JOIN users resolver ON resolver.id=m.suggestion_resolved_by LEFT JOIN tasks t ON t.id=m.task_id", "m.project_id=:project AND m.channel_id=:id"),
        _ => bail!("invalid_timeline_kind"),
    };
    let extra = if ids.is_some() {
        "AND m.id IN (SELECT value FROM json_each(:ids))"
    } else if before.is_some() {
        "AND (m.created_at,m.id)<(:time,:message)"
    } else {
        ""
    };
    // Each direction of a private chat has its own indexed, limited seek. Applying
    // LIMIT after an OR over both senders can scan/sort the entire conversation.
    let bounded_friend;
    let source = if r.kind == "friend" && ids.is_none() {
        let legs = [
            "m.sender_user_id=:owner AND m.receiver_user_id=:id AND (:id!=:ai OR m.context_user_id IS NULL)",
            "m.sender_user_id=:id AND m.receiver_user_id=:owner AND (:id!=:ai OR m.context_user_id IS NULL)",
            "m.sender_user_id=:ai AND m.receiver_user_id=:owner AND m.context_user_id=:id AND :id!=:ai",
        ].map(|predicate| format!("SELECT * FROM (SELECT * FROM friend_messages m WHERE {predicate} {extra} ORDER BY m.created_at DESC,m.id DESC LIMIT :limit)"));
        bounded_friend = format!(
            "({}) m LEFT JOIN users u ON u.id=m.sender_user_id",
            legs.join(" UNION ALL ")
        );
        bounded_friend.as_str()
    } else {
        source
    };
    let sql = format!("SELECT {select} FROM {source} WHERE ({filter}) {extra} ORDER BY m.created_at DESC,m.id DESC LIMIT :limit");
    let mut stmt = conn.prepare(&sql)?;
    let ids = serde_json::to_string(&ids.unwrap_or_default())?;
    for (key, value) in [
        (":owner", owner),
        (":id", r.id.as_str()),
        (":project", r.project.as_str()),
        (":ai", SOCIAL_AI_USER_ID),
        (":ids", ids.as_str()),
        (":time", before.map(|b| b.0.as_str()).unwrap_or_default()),
        (":message", before.map(|b| b.1.as_str()).unwrap_or_default()),
    ] {
        if let Some(index) = stmt.parameter_index(key)? {
            stmt.raw_bind_parameter(index, value)?;
        }
    }
    stmt.raw_bind_parameter(stmt.parameter_index(":limit")?.unwrap(), limit as i64)?;
    let mut rows = stmt.raw_query();
    let mut result = vec![];
    let mut group_batch = vec![];
    while let Some(row) = rows.next()? {
        let message = match r.kind.as_str() {
            "group" => {
                let mut m = groups::row_to_group_message(row, owner)?;
                m.quote = friend_messages::social_quotes::read(
                    conn,
                    "group",
                    &m.id,
                    m.recalled_at.is_some(),
                )?;
                group_batch.push(m);
                continue;
            }
            "friend" => {
                let mut m = friend_messages::row_to_friend_message(row, owner)?;
                m.quote = friend_messages::social_quotes::read(
                    conn,
                    "friend",
                    &m.id,
                    m.recalled_at.is_some(),
                )?;
                serde_json::to_value(m)?
            }
            "channel" => {
                serde_json::to_value(project_space::project_channel_message_from_row(row, owner)?)?
            }
            _ => {
                let recalled: Option<String> = row.get(10)?;
                json!({"id":row.get::<_,String>(0)?,"project_id":row.get::<_,String>(1)?,"conversation_id":row.get::<_,Option<String>>(2)?,
                    "task_id":row.get::<_,Option<String>>(3)?,"user_id":row.get::<_,Option<String>>(4)?,"sender_name":row.get::<_,Option<String>>(5)?,
                    "role":row.get::<_,String>(6)?,"content":crate::store::message_recall::recalled_content(row.get(7)?,recalled.as_deref()),
                    "created_at":row.get::<_,String>(8)?,"outgoing":row.get::<_,Option<i64>>(9)?.unwrap_or(0)!=0,
                    "recalled_at":recalled,"recalled_by":row.get::<_,Option<String>>(11)?})
            }
        };
        result.push(message);
    }
    if !group_batch.is_empty() {
        crate::store::social_ai_messages::requests::context::decorate(conn, &mut group_batch)?;
        result = group_batch
            .into_iter()
            .map(serde_json::to_value)
            .collect::<serde_json::Result<_>>()?;
    }
    result.reverse();
    Ok(result)
}
