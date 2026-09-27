//! Imported records retain their tree and original text, independent of AI snapshots.
use super::{fail, member};
use crate::store::{new_id, now, Store};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
mod media;
pub(crate) mod migration;
mod model;
pub(crate) use model::*;
#[cfg(test)]
mod tests;

pub(super) fn readable(conn: &Connection, user: &str, group: &str, id: &str) -> Result<View> {
    member(conn, user, group)?;
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT r.owner_id,r.document_json FROM social_chat_records r
         JOIN friend_group_messages m ON m.id=r.message_id AND m.group_id=r.group_id
         WHERE r.id=?1 AND r.group_id=?2 AND r.revoked=0 AND m.recalled_at IS NULL",
            params![id, group],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (owner_id, json) = row.ok_or_else(|| fail(404, "聊天记录已撤回或不可访问"))?;
    let document: Document = serde_json::from_str(&json)?;
    Ok(View {
        card: document.card(id, group),
        owner_id,
        document,
    })
}

impl Store {
    pub(crate) fn create_chat_record(
        &self,
        user: &str,
        group: &str,
        operation: &str,
        doc: Document,
    ) -> Result<Created> {
        if !model::opaque(operation) || operation.len() < 8 {
            return Err(fail(400, "操作标识无效"));
        }
        let json = doc.validate()?;
        let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        member(&tx, user, group)?;
        let prior: Option<(String, String, String)> = tx.query_row(
            "SELECT id,message_id,request_hash FROM social_chat_records WHERE owner_id=?1 AND group_id=?2 AND operation=?3",
            params![user, group, operation], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?;
        if let Some((id, message_id, old_hash)) = prior {
            if hash != old_hash {
                return Err(fail(409, "重复操作的内容不一致"));
            }
            let view = readable(&tx, user, group, &id)?;
            let message = super::snapshots::writes::read_message(&tx, user, group, &message_id)?;
            return Ok(Created {
                card: view.card,
                message,
                replayed: true,
            });
        }
        let (count, total): (i64, i64) = tx.query_row(
            "SELECT COUNT(*),COALESCE(SUM(length(CAST(document_json AS BLOB))),0) FROM social_chat_records WHERE owner_id=?1",
            [user], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if count >= 1000 || total + json.len() as i64 > 128 * 1024 * 1024 {
            return Err(fail(413, "聊天记录存储额度已满"));
        }
        let mut total_assets = 0i64;
        let mut counted = std::collections::BTreeSet::new();
        for row in &doc.messages {
            if let Some(asset) = &row.asset_id {
                let found: Option<(String, i64)> = tx.query_row(
                    "SELECT mime_type,length(bytes) FROM social_chat_record_assets WHERE id=?1 AND owner_id=?2 AND group_id=?3",
                    params![asset, user, group], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
                let (mime, size) = found.ok_or_else(|| fail(400, "附件未上传到此群聊"))?;
                if (row.kind == "image" && !mime.starts_with("image/"))
                    || (row.kind == "video" && !mime.starts_with("video/"))
                {
                    return Err(fail(400, "附件类型不匹配"));
                }
                if counted.insert(asset) {
                    total_assets += size;
                }
            }
        }
        if total_assets > 64 * 1024 * 1024 {
            return Err(fail(413, "附件总量超过 64 MiB"));
        }
        let id = new_id("chat_record");
        let card = doc.card(&id, group);
        let message =
            crate::store::groups::send::insert_chat_record_message(&tx, user, group, &card)?;
        tx.execute("INSERT INTO social_chat_records(id,owner_id,group_id,message_id,operation,request_hash,document_json,created_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![id,user,group,message.id,operation,hash,json,now()])?;
        for asset in doc.asset_ids() {
            tx.execute(
                "INSERT INTO social_chat_record_asset_refs VALUES(?1,?2)",
                params![id, asset],
            )?;
        }
        tx.commit()?;
        Ok(Created {
            card,
            message,
            replayed: false,
        })
    }

    pub(crate) fn read_chat_record(&self, user: &str, group: &str, id: &str) -> Result<View> {
        let conn = self.conn()?;
        readable(&conn, user, group, id)
    }

    pub(crate) fn revoke_chat_record(&self, user: &str, group: &str, id: &str) -> Result<()> {
        let count = self.conn()?.execute(
            "UPDATE social_chat_records SET revoked=1 WHERE id=?1 AND owner_id=?2 AND group_id=?3",
            params![id, user, group],
        )?;
        if count == 0 {
            return Err(fail(404, "聊天记录不可访问"));
        }
        Ok(())
    }
}
