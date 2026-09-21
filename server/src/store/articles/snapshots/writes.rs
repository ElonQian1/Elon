use super::*;
use sha2::{Digest, Sha256};

impl Store {
    pub(crate) fn create_ai_snapshot(
        &self,
        user: &str,
        group: &str,
        key: &str,
        mut document: SnapshotDocument,
    ) -> Result<SnapshotCreated> {
        if !(8..=128).contains(&key.len()) || !model::opaque(key) {
            return Err(fail(400, "Idempotency key must be 8-128 opaque characters"));
        }
        let request = document.validate()?;
        let request_hash = format!("{:x}", Sha256::digest(request.as_bytes()));
        let key_hash = format!("{:x}", Sha256::digest(key.as_bytes()));
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        member(&tx, user, group)?;
        let prior: Option<(String, String, String)> = tx
            .query_row(
                "SELECT content_id,message_id,request_hash FROM social_snapshot_operations
             WHERE owner_id=?1 AND group_id=?2 AND key_hash=?3",
                params![user, group, key_hash],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((id, message_id, hash)) = prior {
            if hash != request_hash {
                return Err(fail(409, "Idempotency key payload mismatch"));
            }
            readable(&tx, user, group, &id)?;
            let message = read_message(&tx, user, group, &message_id)?;
            tx.commit()?;
            return Ok(SnapshotCreated {
                snapshot_id: id,
                group_id: group.into(),
                message,
                replayed: true,
            });
        }
        let (count, total): (i64, i64) = tx.query_row(
            "SELECT COUNT(*),COALESCE(SUM(length(CAST(r.document_json AS BLOB))),0)
             FROM social_contents c JOIN social_content_revisions r ON r.content_id=c.id
             WHERE c.owner_id=?1 AND c.kind='ai_snapshot'",
            [user],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if count >= 500 || total + request.len() as i64 > 64 * 1024 * 1024 {
            return Err(fail(413, "Snapshot owner quota exceeded"));
        }
        for asset in document.asset_ids() {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM social_snapshot_asset_grants g
                 JOIN social_content_media m ON m.id=g.media_id AND m.owner_id=g.owner_id
                 WHERE g.owner_id=?1 AND g.group_id=?2 AND g.media_id=?3)",
                params![user, group, asset],
                |r| r.get(0),
            )?;
            if !valid {
                return Err(fail(400, "Asset not uploaded by this owner to this group"));
            }
        }
        document.assign_public_ids();
        let json = serde_json::to_string(&document)?;
        if json.len() > 2 * 1024 * 1024 || total + json.len() as i64 > 64 * 1024 * 1024 {
            return Err(fail(413, "Snapshot JSON exceeds 2 MiB"));
        }
        let id = new_id("ai_snapshot");
        let time = now();
        let card = SnapshotCard {
            schema: SCHEMA.into(),
            snapshot_id: id.clone(),
            group_id: group.into(),
            title: document.title.clone(),
            summary: document.summary.clone(),
            sender_name: super::super::author_name(&tx, user)?,
            cover_asset_id: document.cover_asset_id.clone(),
            provider: document.provider.clone(),
            message_count: document.messages.len(),
        };
        tx.execute(
            "INSERT INTO social_contents
            (id,owner_id,kind,draft_json,edit_version,status,created_at,updated_at)
            VALUES (?1,?2,'ai_snapshot','{}',1,'published',?3,?3)",
            params![id, user, time],
        )?;
        tx.execute(
            "INSERT INTO social_content_revisions VALUES (?1,1,?2,?3)",
            params![id, json, time],
        )?;
        let message = crate::store::groups::send::insert_snapshot_message(&tx, user, group, &card)?;
        tx.execute(
            "INSERT INTO social_content_distributions VALUES (?1,1,'group',?2,?3,?4)",
            params![id, group, message.id, time],
        )?;
        tx.execute(
            "INSERT INTO social_snapshot_operations VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![user, group, key_hash, request_hash, id, message.id, time],
        )?;
        tx.commit()?;
        Ok(SnapshotCreated {
            snapshot_id: id,
            group_id: group.into(),
            message,
            replayed: false,
        })
    }
}

fn read_message(
    conn: &Connection,
    user: &str,
    group: &str,
    id: &str,
) -> Result<FriendGroupMessage> {
    Ok(conn.query_row(
        "SELECT m.id,m.group_id,m.sender_user_id,COALESCE(u.nickname,u.email,u.phone,u.id),
         m.content,m.created_at,m.revision,m.edited_at FROM friend_group_messages m
         JOIN users u ON u.id=m.sender_user_id WHERE m.id=?1 AND m.group_id=?2 AND m.recalled_at IS NULL",
        params![id,group], |r| Ok(FriendGroupMessage {
            ai_reply: None,
            id: r.get(0)?, group_id: r.get(1)?, sender_user_id: r.get(2)?, sender_name: r.get(3)?,
            content: r.get(4)?, created_at: r.get(5)?, revision: r.get(6)?, edited_at: r.get(7)?,
            attachments: vec![], outgoing: r.get::<_,String>(2)? == user,
            recalled_at: None, recalled_by: None,
        }),
    )?)
}
