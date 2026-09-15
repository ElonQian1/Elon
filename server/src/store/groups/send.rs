use super::super::social_ai_messages::requests::web::{self, WebGroupRequest};
use super::*;

pub(in crate::store) fn insert_message(
    conn: &Connection,
    user: &str,
    group: &str,
    content: &str,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> Result<FriendGroupMessage> {
    let member: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id=?1 AND user_id=?2)",
        params![group, user],
        |r| r.get(0),
    )?;
    if !member {
        return Err(anyhow!("你不在这个群聊中"));
    }
    let content = content.trim();
    let attachments_json = attachments_to_json(attachments)?;
    if content.is_empty() && attachments_json.is_none() {
        return Err(anyhow!("消息不能为空"));
    }
    if content.chars().count() > 4000 {
        return Err(anyhow!("消息过长"));
    }
    let id = new_id("gmsg");
    let created_at = now();
    let sender_name = conn.query_row(
        "SELECT COALESCE(nickname,email,phone,id) FROM users WHERE id=?1",
        [user],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO friend_group_messages (id,group_id,sender_user_id,content,attachments_json,created_at)
         VALUES (?1,?2,?3,?4,?5,?6)", params![id,group,user,content,attachments_json,created_at],
    )?;
    conn.execute(
        "UPDATE friend_groups SET updated_at=?1 WHERE id=?2",
        params![created_at, group],
    )?;
    mark_group_messages_read(conn, user, group)?;
    Ok(FriendGroupMessage {
        id,
        group_id: group.to_owned(),
        sender_user_id: user.to_owned(),
        sender_name,
        content: content.to_owned(),
        attachments: attachments.unwrap_or(&[]).to_vec(),
        created_at,
        outgoing: true,
        recalled_at: None,
        recalled_by: None,
        revision: 1,
        edited_at: None,
    })
}

impl Store {
    pub(crate) fn send_group_web_ai_message(
        &self,
        user: &str,
        group: &str,
        content: &str,
        attachments: Option<&[ProjectAttachmentRef]>,
        operation: &str,
    ) -> Result<(FriendGroupMessage, WebGroupRequest)> {
        let hash = web::operation_hash(operation)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let prior: Option<(String,String)> = tx.query_row(
            "SELECT id,trigger_message_id FROM group_ai_reply_requests WHERE operation_hash=?1 AND requester_id=?2 AND group_id=?3",
            params![hash,user,group], |r| Ok((r.get(0)?,r.get(1)?)),
        ).optional()?;
        let result = if let Some((id, source)) = prior {
            let request = web::read_owned(&tx, user, group, &id, operation)?;
            let message = tx.query_row(
                "SELECT m.id,m.group_id,m.sender_user_id,COALESCE(u.nickname,u.email,u.phone,u.id),m.content,m.attachments_json,m.created_at,m.recalled_at,m.recalled_by,m.revision,m.edited_at
                 FROM friend_group_messages m JOIN users u ON u.id=m.sender_user_id WHERE m.id=?1 AND m.group_id=?2",
                params![source,group], |r| row_to_group_message(r,user),
            )?;
            if message.content != content.trim()
                || attachments_to_json(Some(&message.attachments))?
                    != attachments_to_json(attachments)?
            {
                return Err(anyhow!("重复操作的消息内容不一致"));
            }
            (message, request)
        } else {
            let message = insert_message(&tx, user, group, content, attachments)?;
            let request = web::prepare_in_transaction(&tx, user, group, &message.id, operation)?;
            (message, request)
        };
        tx.commit()?;
        Ok(result)
    }
}
