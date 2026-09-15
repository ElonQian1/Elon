use super::*;

const ORPHAN_TTL_SECONDS: i64 = 24 * 60 * 60;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "social_snapshot_asset_grants",
        "uploaded_at",
        "uploaded_at INTEGER",
    )?;
    // Never infer ownership for earlier uploads: unknown provenance is retained.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS social_snapshot_orphan_media (
        media_id TEXT PRIMARY KEY REFERENCES social_content_media(id) ON DELETE CASCADE,
        owner_id TEXT NOT NULL REFERENCES users(id), created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS social_snapshot_orphan_owner
        ON social_snapshot_orphan_media(owner_id,created_at);
        CREATE INDEX IF NOT EXISTS social_snapshot_grant_expiry
        ON social_snapshot_asset_grants(owner_id,uploaded_at);",
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn cleanup_ai_snapshot_orphans(&self, owner: &str) -> Result<usize> {
        cleanup(self, owner, chrono::Utc::now().timestamp())
    }
}

pub(super) fn cleanup(store: &Store, owner: &str, timestamp: i64) -> Result<usize> {
    let cutoff = timestamp - ORPHAN_TTL_SECONDS;
    let mut conn = store.conn()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let candidates: Vec<String> = {
        let mut query = tx.prepare("SELECT media_id FROM social_snapshot_asset_grants
            WHERE owner_id=?1 AND uploaded_at<?2
            UNION SELECT media_id FROM social_snapshot_orphan_media WHERE owner_id=?1 AND created_at<?2
            ORDER BY media_id LIMIT 300")?;
        let rows = query
            .query_map(params![owner, cutoff], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let mut removed = 0;
    for id in candidates {
        if referenced(&tx, &id)? {
            continue;
        }
        tx.execute(
            "DELETE FROM social_snapshot_asset_grants
            WHERE owner_id=?1 AND media_id=?2 AND uploaded_at<?3",
            params![owner, id, cutoff],
        )?;
        let disposable: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM social_snapshot_orphan_media o
             JOIN social_content_media m ON m.id=o.media_id AND m.owner_id=o.owner_id
             WHERE o.media_id=?1 AND o.owner_id=?2 AND o.created_at<?3
             AND NOT EXISTS(SELECT 1 FROM social_snapshot_asset_grants g WHERE g.media_id=o.media_id))",
            params![id,owner,cutoff], |r| r.get(0),
        )?;
        if disposable {
            // Any additional FK reference aborts this transaction, preserving grants and media.
            removed += tx.execute(
                "DELETE FROM social_content_media WHERE id=?1 AND owner_id=?2",
                params![id, owner],
            )?;
        }
    }
    tx.commit()?;
    Ok(removed)
}

fn referenced(conn: &Connection, media: &str) -> Result<bool> {
    let document_reference: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM social_contents c WHERE NOT json_valid(c.draft_json)
            OR EXISTS(SELECT 1 FROM json_tree(CASE WHEN json_valid(c.draft_json) THEN c.draft_json ELSE '{}' END) j
                      WHERE j.type='text' AND j.value=?1))
         OR EXISTS(SELECT 1 FROM social_content_revisions r WHERE NOT json_valid(r.document_json)
            OR EXISTS(SELECT 1 FROM json_tree(CASE WHEN json_valid(r.document_json) THEN r.document_json ELSE '{}' END) j
                      WHERE j.type='text' AND j.value=?1))",
        [media], |r| r.get(0),
    )?;
    if document_reference {
        return Ok(true);
    }
    let has_videos: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='article_square_videos')",
        [], |r| r.get(0),
    )?;
    if has_videos {
        return Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM article_square_videos WHERE cover_id=?1)",
            [media],
            |r| r.get(0),
        )?);
    }
    Ok(false)
}
