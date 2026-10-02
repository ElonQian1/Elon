use super::*;

pub(super) fn find(conn: &Connection, owner: &str, id: &str) -> Result<Option<Value>> {
    let raw: Option<String> = conn.query_row("SELECT json_object('id',id,'kind',kind,'project',project,'scope',scope,'title',title,'note',note,'anchor',json(anchor),'revision',revision,'deleted',json(CASE WHEN deleted=1 THEN 'true' ELSE 'false' END),'created_at',created_at,'updated_at',updated_at) FROM reading_bookmarks WHERE owner=?1 AND id=?2", params![owner,id], |r| r.get(0)).optional()?;
    raw.map(|v| serde_json::from_str(&v).map_err(Into::into))
        .transpose()
}

pub(super) fn list(conn: &Connection, owner: &str, scope: &Scope, after: &str) -> Result<Value> {
    if after.len() > 100 {
        bail!("invalid_reading_cursor");
    }
    let mut stmt = conn.prepare("SELECT id FROM reading_bookmarks WHERE owner=?1 AND kind=?2 AND project=?3 AND scope=?4 AND deleted=0 AND id>?5 ORDER BY id LIMIT 51")?;
    let ids = stmt
        .query_map(
            params![owner, scope.kind, scope.project, scope.id, after],
            |r| r.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = ids.len() > 50;
    let mut rows = vec![];
    for id in ids.iter().take(50) {
        let mut value = find(conn, owner, id)?.unwrap();
        let target = scope.target(id);
        value["progress"] = progress::read(conn, owner, &target)?.unwrap_or(Value::Null);
        value["candidates"] = progress::candidates(conn, owner, &target, false)?;
        value["previous_positions"] = progress::candidates(conn, owner, &target, true)?;
        rows.push(value);
    }
    Ok(
        json!({"schema":"elon.reading_positions.v1", "bookmarks":rows,
        "next":if more { ids.get(49).cloned() } else { None },
        "conversation_progress":progress::read(conn,owner,&scope.target(""))?,
        "conversation_candidates":progress::candidates(conn,owner,&scope.target(""),false)?,
        "limits":{"per_conversation":100,"per_account":1000}}),
    )
}

pub(super) fn write(conn: &Connection, owner: &str, c: &Command) -> Result<Value> {
    if c.bookmark_id.len() < 8
        || !c
            .bookmark_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("invalid_bookmark_id");
    }
    let existing = find(conn, owner, &c.bookmark_id)?;
    if c.action == "create" {
        if existing.is_some() {
            bail!("reading_id_exists");
        }
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM reading_bookmarks WHERE owner=?1 AND deleted=0",
            [owner],
            |r| r.get(0),
        )?;
        let scoped: i64 = conn.query_row("SELECT count(*) FROM reading_bookmarks WHERE owner=?1 AND kind=?2 AND project=?3 AND scope=?4 AND deleted=0",params![owner,c.scope.kind,c.scope.project,c.scope.id],|r|r.get(0))?;
        if count >= 1000 || scoped >= 100 {
            bail!("reading_limit");
        }
        let anchor = position(conn, owner, &c.scope, &c.position)?;
        let now = crate::store::now();
        let title = if c.title.trim().is_empty() {
            format!("书签 {}", anchor.created_at)
        } else {
            c.title.trim().into()
        };
        conn.execute("INSERT INTO reading_bookmarks(owner,id,kind,project,scope,title,note,anchor,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",params![owner,c.bookmark_id,c.scope.kind,c.scope.project,c.scope.id,title,c.note,serde_json::to_string(&anchor)?,now])?;
        return Ok(json!({"ok":true,"id":c.bookmark_id,"revision":1}));
    }
    let old = existing.ok_or_else(|| anyhow::anyhow!("reading_unavailable"))?;
    if old["kind"] != c.scope.kind
        || old["project"] != c.scope.project
        || old["scope"] != c.scope.id
    {
        bail!("reading_unavailable");
    }
    if c.action == "delete" && old["deleted"] == true {
        return Ok(json!({"ok":true,"deleted":true,"id":c.bookmark_id}));
    }
    if old["deleted"] == true {
        bail!("reading_unavailable");
    }
    if old["revision"].as_i64() != Some(c.base_revision) {
        bail!("reading_revision_conflict");
    }
    if c.action == "delete" {
        conn.execute("UPDATE reading_bookmarks SET deleted=1,title='',note='',anchor='{}',revision=revision+1,updated_at=?3 WHERE owner=?1 AND id=?2",params![owner,c.bookmark_id,crate::store::now()])?;
        let target = c.scope.target(&c.bookmark_id);
        conn.execute(
            "DELETE FROM reading_progress WHERE owner=?1 AND target=?2",
            params![owner, target],
        )?;
        conn.execute(
            "DELETE FROM reading_candidates WHERE owner=?1 AND target=?2",
            params![owner, target],
        )?;
        conn.execute(
            "DELETE FROM reading_operations WHERE owner=?1 AND target=?2",
            params![owner, target],
        )?;
        Ok(json!({"ok":true,"deleted":true,"id":c.bookmark_id}))
    } else {
        conn.execute("UPDATE reading_bookmarks SET title=?3,note=?4,revision=revision+1,updated_at=?5 WHERE owner=?1 AND id=?2",params![owner,c.bookmark_id,c.title.trim(),c.note,crate::store::now()])?;
        Ok(json!({"ok":true,"id":c.bookmark_id,"revision":c.base_revision+1}))
    }
}
