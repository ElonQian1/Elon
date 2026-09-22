//! Group-scoped immutable AI snapshots reuse social publications and media storage.
use super::{fail, member};
use crate::store::{new_id, now, FriendGroupMessage, Store};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
mod media;
#[cfg(test)]
mod media_tests;
pub(crate) mod migration;
mod model;
pub(crate) mod orphan_media;
#[cfg(test)]
mod orphan_media_tests;
mod privacy;
pub(crate) use privacy::validate_text as validate_shared_text;
mod rich_card;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;
mod writes;
pub(crate) use model::*;

pub(crate) const SCHEMA: &str = "elon.ai_conversation_share.v1";
pub(crate) const CARD_PREFIX: &str = "\u{3010}\u{4e00}\u{9f99}AI\u{5bf9}\u{8bdd}\u{3011}\n";

pub(crate) fn message_preview(content: &str) -> Option<String> {
    let card: SnapshotCard = serde_json::from_str(content.strip_prefix(CARD_PREFIX)?).ok()?;
    if card.schema != SCHEMA || !card.snapshot_id.starts_with("ai_snapshot_") {
        return None;
    }
    Some(format!(
        "[AI\u{5bf9}\u{8bdd}] {}",
        card.title.chars().take(120).collect::<String>()
    ))
}

fn readable(conn: &Connection, user: &str, group: &str, id: &str) -> Result<SnapshotView> {
    member(conn, user, group)?;
    let found: Option<(String, String, String)> = conn
        .query_row(
            "SELECT c.owner_id,r.published_at,r.document_json FROM social_contents c
         JOIN social_content_revisions r ON r.content_id=c.id AND r.revision=1
         JOIN social_content_distributions d ON d.content_id=c.id AND d.revision=1
         JOIN friend_group_messages m ON m.id=d.message_id AND m.group_id=d.target_id
         JOIN social_snapshot_operations o ON o.content_id=c.id AND o.message_id=m.id
         WHERE c.id=?1 AND c.kind='ai_snapshot' AND c.status='published'
           AND d.target_kind='group' AND d.target_id=?2 AND o.group_id=?2
           AND m.sender_user_id=c.owner_id AND o.owner_id=c.owner_id AND m.recalled_at IS NULL",
            params![id, group],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (owner_id, created_at, json) = found.ok_or_else(|| fail(404, "Snapshot unavailable"))?;
    let owner_name = super::author_name(conn, &owner_id)?;
    Ok(SnapshotView {
        snapshot_id: id.into(),
        group_id: group.into(),
        owner_id,
        owner_name,
        created_at,
        document: serde_json::from_str(&json)?,
    })
}

impl Store {
    pub(crate) fn read_ai_snapshot(
        &self,
        user: &str,
        group: &str,
        id: &str,
    ) -> Result<SnapshotView> {
        let conn = self.conn()?;
        readable(&conn, user, group, id)
    }

    pub(crate) fn revoke_ai_snapshot(&self, user: &str, group: &str, id: &str) -> Result<()> {
        let conn = self.conn()?;
        // The owner may revoke even after leaving; this never grants read access.
        let owned: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM social_snapshot_operations o
             JOIN social_contents c ON c.id=o.content_id
             WHERE o.content_id=?1 AND o.owner_id=?2 AND o.group_id=?3 AND c.kind='ai_snapshot')",
            params![id, user, group],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(fail(404, "Snapshot unavailable"));
        }
        conn.execute(
            "UPDATE social_contents SET status='withdrawn',updated_at=?2
            WHERE id=?1 AND status='published'",
            params![id, now()],
        )?;
        Ok(())
    }
}
