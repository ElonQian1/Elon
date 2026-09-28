use super::*;

impl Store {
    // Validate access before answering a conditional read; never load the document or blob here.
    pub(crate) fn chat_record_version(
        &self,
        user: &str,
        group: &str,
        record: &str,
        asset: Option<&str>,
    ) -> Result<String> {
        let conn = self.conn()?;
        member(&conn, user, group)?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT r.owner_id,r.request_hash FROM social_chat_records r
             JOIN friend_group_messages m ON m.id=r.message_id AND m.group_id=r.group_id
             WHERE r.id=?1 AND r.group_id=?2 AND r.revoked=0 AND m.recalled_at IS NULL",
                params![record, group],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (owner, hash) = row.ok_or_else(|| fail(404, "聊天记录已撤回或不可访问"))?;
        let hash = match asset {
            None => hash,
            Some(asset) => conn
                .query_row(
                    "SELECT a.sha256 FROM social_chat_record_assets a
                 JOIN social_chat_record_asset_refs r ON r.asset_id=a.id
                 WHERE r.record_id=?1 AND a.id=?2 AND a.group_id=?3 AND a.owner_id=?4",
                    params![record, asset, group, owner],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| fail(404, "附件不可访问"))?,
        };
        Ok(format!("\"chat-record-v1-{hash}\""))
    }
}
