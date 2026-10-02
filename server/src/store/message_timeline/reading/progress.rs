use super::*;

pub(super) fn read(conn: &Connection, owner: &str, target: &str) -> Result<Option<Value>> {
    let raw: Option<String> = conn.query_row("SELECT json_object('position',json(position),'furthest',json(furthest),'revision',revision,'device_id',device,'device_seq',device_seq,'updated_at',updated_at) FROM reading_progress WHERE owner=?1 AND target=?2",params![owner,target],|r|r.get(0)).optional()?;
    raw.map(|v| serde_json::from_str(&v).map_err(Into::into))
        .transpose()
}
pub(super) fn candidates(
    conn: &Connection,
    owner: &str,
    target: &str,
    resolved: bool,
) -> Result<Value> {
    let mut stmt=conn.prepare("SELECT json_object('device_id',device,'device_seq',device_seq,'position',json(position),'updated_at',updated_at) FROM reading_candidates WHERE owner=?1 AND target=?2 AND resolved=?3 ORDER BY updated_at DESC LIMIT 10")?;
    let rows = stmt
        .query_map(params![owner, target, resolved], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Value::Array(
        rows.into_iter()
            .map(|r| serde_json::from_str(&r))
            .collect::<serde_json::Result<_>>()?,
    ))
}
fn candidate(
    conn: &Connection,
    owner: &str,
    target: &str,
    device: &str,
    seq: i64,
    p: &Value,
    resolved: bool,
) -> Result<()> {
    let count:i64=conn.query_row("SELECT count(*) FROM reading_candidates WHERE owner=?1 AND target=?2 AND device!=?3 AND resolved=0",params![owner,target,device],|r|r.get(0))?;
    if !resolved && count >= 8 {
        bail!("reading_limit");
    }
    conn.execute("INSERT INTO reading_candidates VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(owner,target,device) DO UPDATE SET device_seq=excluded.device_seq,position=excluded.position,updated_at=excluded.updated_at,resolved=excluded.resolved WHERE excluded.device_seq>=reading_candidates.device_seq",params![owner,target,device,seq,p.to_string(),crate::store::now(),resolved])?;
    Ok(())
}

pub(super) fn write(conn: &Connection, owner: &str, c: &Command) -> Result<Value> {
    if c.device_id.len() < 8 || c.device_seq <= 0 || c.base_revision < 0 {
        bail!("invalid_reading_device");
    }
    let p = position(conn, owner, &c.scope, &c.position)?;
    let target = c.scope.target(&c.bookmark_id);
    let old = read(conn, owner, &target)?;
    let revision = old
        .as_ref()
        .and_then(|p| p["revision"].as_i64())
        .unwrap_or(0);
    let old_device = old
        .as_ref()
        .and_then(|p| p["device_id"].as_str())
        .unwrap_or("");
    let old_seq = old
        .as_ref()
        .and_then(|p| p["device_seq"].as_i64())
        .unwrap_or(0);
    if old_device == c.device_id && c.device_seq <= old_seq {
        return Ok(json!({"ok":true,"stale":true,"progress":old}));
    }
    let candidate_seq:Option<i64>=conn.query_row("SELECT device_seq FROM reading_candidates WHERE owner=?1 AND target=?2 AND device=?3 AND resolved=0",params![owner,target,c.device_id],|r|r.get(0)).optional()?;
    if candidate_seq.is_some_and(|seq| seq >= c.device_seq) {
        return Ok(
            json!({"ok":true,"stale":true,"conflict":true,"progress":old,"candidates":candidates(conn,owner,&target,false)?}),
        );
    }
    let pending:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM reading_candidates WHERE owner=?1 AND target=?2 AND resolved=0)",params![owner,target],|r|r.get(0))?;
    if (revision != c.base_revision && (old_device != c.device_id || c.action == "resolve"))
        || (pending && c.action != "resolve")
    {
        candidate(
            conn,
            owner,
            &target,
            &c.device_id,
            c.device_seq,
            &serde_json::to_value(&p)?,
            false,
        )?;
        return Ok(
            json!({"ok":false,"conflict":true,"progress":old,"candidates":candidates(conn,owner,&target,false)?}),
        );
    }
    if c.action == "resolve" {
        conn.execute(
            "UPDATE reading_candidates SET resolved=1 WHERE owner=?1 AND target=?2",
            params![owner, target],
        )?;
        if let Some(previous) = &old {
            candidate(
                conn,
                owner,
                &target,
                old_device,
                old_seq,
                &previous["position"],
                true,
            )?;
        }
    }
    let previous = old
        .as_ref()
        .and_then(|v| serde_json::from_value::<Position>(v["furthest"].clone()).ok());
    let furthest = previous
        .filter(|v| v.key() > p.key() || (v.key() == p.key() && v.fraction > p.fraction))
        .unwrap_or_else(|| p.clone());
    conn.execute("INSERT INTO reading_progress VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(owner,target) DO UPDATE SET position=excluded.position,furthest=excluded.furthest,revision=excluded.revision,device=excluded.device,device_seq=excluded.device_seq,updated_at=excluded.updated_at",params![owner,target,serde_json::to_string(&p)?,serde_json::to_string(&furthest)?,revision+1,c.device_id,c.device_seq,crate::store::now()])?;
    conn.execute("DELETE FROM reading_candidates WHERE resolved=1 AND updated_at<strftime('%Y-%m-%dT%H:%M:%fZ','now','-30 days')",[])?;
    Ok(json!({"ok":true,"progress":read(conn,owner,&target)?}))
}
