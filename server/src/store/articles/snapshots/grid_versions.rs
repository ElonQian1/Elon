use super::*;

pub(super) fn validate_previous(
    conn: &Connection,
    user: &str,
    group: &str,
    grid: &grid::GridShare,
) -> Result<()> {
    let Some(previous) = &grid.previous_snapshot_id else {
        return Ok(());
    };
    let prior = readable(conn, user, group, previous)?;
    if prior.owner_id != user
        || prior.document.grid.as_ref().map(|v| v.fields.get("symbol"))
            != Some(grid.fields.get("symbol"))
    {
        return Err(fail(403, "Only the owner can update the same grid share"));
    }
    if next(conn, group, previous, false)?.is_some() {
        return Err(fail(
            409,
            "Snapshot already updated; open its latest version",
        ));
    }
    Ok(())
}

fn next(conn: &Connection, group: &str, id: &str, readable_only: bool) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT c.id FROM social_contents c JOIN social_content_revisions r ON r.content_id=c.id AND r.revision=1
         JOIN social_snapshot_operations o ON o.content_id=c.id
         JOIN friend_group_messages m ON m.id=o.message_id AND m.group_id=o.group_id
         WHERE o.group_id=?1 AND c.kind='ai_snapshot'
           AND json_extract(r.document_json,'$.grid.previous_snapshot_id')=?2
           AND (?3=0 OR (c.status='published' AND m.recalled_at IS NULL)) LIMIT 1",
        params![group, id, readable_only], |r| r.get(0)).optional()?)
}

pub(super) fn latest(conn: &Connection, group: &str, id: &str) -> Result<Option<String>> {
    let mut current = id.to_string();
    for _ in 0..500 {
        match next(conn, group, &current, true)? {
            Some(value) => current = value,
            None => break,
        }
    }
    Ok((current != id).then_some(current))
}

/// Resolve by persisted message ownership; a client-provided card cannot select another snapshot.
pub(crate) fn ai_context(
    conn: &Connection,
    user: &str,
    group: &str,
    message: &str,
    content: &str,
) -> Result<Option<serde_json::Value>> {
    if !content.starts_with(CARD_PREFIX) {
        return Ok(None);
    }
    let card: SnapshotCard = serde_json::from_str(content.trim_start_matches(CARD_PREFIX))?;
    if card.provider != "binance" {
        return Ok(None);
    }
    let id: String = conn.query_row(
        "SELECT content_id FROM social_snapshot_operations WHERE group_id=?1 AND message_id=?2",
        params![group, message],
        |r| r.get(0),
    )?;
    let view = readable(conn, user, group, &id)?;
    let grid = view
        .document
        .grid
        .ok_or_else(|| fail(400, "Grid snapshot unavailable"))?;
    Ok(Some(serde_json::json!({"snapshot_id":id,"grid":grid,
        "scope":"Published historical snapshot only; missing values are unknown, never zero. Private account and live positions were not accessed."})))
}
