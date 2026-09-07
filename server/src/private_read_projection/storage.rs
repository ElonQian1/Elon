//! The same monotonic persistence rules serve the node outbox and cloud read model.
use super::{digest, now_ms, parse, Projection, MAX_BYTES, MAX_SOURCES};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS private_read_projection_heads (
      owner_id TEXT NOT NULL, node_id TEXT NOT NULL, install_id TEXT NOT NULL,
      credential_hash TEXT NOT NULL, source TEXT NOT NULL, connection_id TEXT NOT NULL,
      revision TEXT NOT NULL, generation INTEGER NOT NULL CHECK(generation>0),
      observed_at_ms INTEGER NOT NULL CHECK(observed_at_ms>0), body TEXT NOT NULL,
      received_at_ms INTEGER NOT NULL, synced_at_ms INTEGER, last_success_at_ms INTEGER, rejected INTEGER NOT NULL DEFAULT 0,
      last_error_code TEXT, PRIMARY KEY(owner_id,node_id,source,connection_id),
      CHECK(length(body)<=262144));
      CREATE INDEX IF NOT EXISTS idx_private_read_owner ON private_read_projection_heads(owner_id);")?;
    Ok(())
}
pub(crate) struct Binding<'a> {
    pub owner: &'a str,
    pub node: &'a str,
    pub install: &'a str,
    pub credential_hash: &'a str,
}
impl Binding<'_> {
    pub(crate) fn projection_id(&self, value: &Projection) -> String {
        digest(
            serde_json::to_string(&[self.owner, self.node, &value.source, &value.connection_id])
                .unwrap()
                .as_bytes(),
        )
    }
}
/// Caller must hold a transaction which also checks its current credential authority.
pub(crate) fn put(
    conn: &Connection,
    binding: &Binding<'_>,
    value: &Projection,
    at: u64,
) -> Result<bool> {
    let body = serde_json::to_string(value)?;
    let previous: Option<(String, u64, u64, String)> = conn
        .query_row(
            "SELECT revision,generation,observed_at_ms,body FROM private_read_projection_heads
        WHERE owner_id=?1 AND node_id=?2 AND source=?3 AND connection_id=?4",
            params![
                binding.owner,
                binding.node,
                value.source,
                value.connection_id
            ],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    if let Some((revision, generation, observed, previous_body)) = previous {
        let previous_value: Projection = serde_json::from_str(&previous_body)?;
        if revision == value.revision {
            if previous_value.revision != revision
                || previous_value.expected_revision().ok().as_deref() != Some(revision.as_str())
            {
                bail!("projection_revision_conflict");
            }
            return Ok(false);
        }
        if value.generation <= generation || value.observed_at_ms < observed {
            bail!("projection_out_of_order");
        }
        if value.observed_at_ms == observed && value.fresh_until_ms > previous_value.fresh_until_ms
        {
            bail!("projection_freshness_conflict");
        }
    } else {
        let count: usize = conn.query_row(
            "SELECT count(*) FROM private_read_projection_heads WHERE owner_id=?1",
            params![binding.owner],
            |r| r.get(0),
        )?;
        if count >= MAX_SOURCES {
            bail!("projection_source_limit");
        }
    }
    let other_bytes: usize = conn.query_row("SELECT COALESCE(sum(length(CAST(body AS BLOB))),0)
      FROM private_read_projection_heads WHERE owner_id=?1 AND NOT(node_id=?2 AND source=?3 AND connection_id=?4)",
      params![binding.owner,binding.node,value.source,value.connection_id], |r| r.get(0))?;
    // Leave space for the bounded read envelope/opaque source identifiers.
    if other_bytes.saturating_add(body.len()) > MAX_BYTES - 16 * 1024 {
        bail!("projection_total_limit");
    }
    conn.execute("INSERT INTO private_read_projection_heads
      (owner_id,node_id,install_id,credential_hash,source,connection_id,revision,generation,observed_at_ms,body,received_at_ms)
      VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
      ON CONFLICT(owner_id,node_id,source,connection_id) DO UPDATE SET
      install_id=excluded.install_id,credential_hash=excluded.credential_hash,revision=excluded.revision,
      generation=excluded.generation,observed_at_ms=excluded.observed_at_ms,body=excluded.body,
      received_at_ms=excluded.received_at_ms,synced_at_ms=NULL,rejected=0,last_error_code=NULL",
      params![binding.owner,binding.node,binding.install,binding.credential_hash,value.source,
      value.connection_id,value.revision,value.generation,value.observed_at_ms,body,at])?;
    Ok(true)
}

pub(crate) fn pending(conn: &Connection, binding: &Binding<'_>) -> Result<Vec<Projection>> {
    let mut stmt = conn.prepare(
        "SELECT body,revision,connection_id,generation,observed_at_ms FROM private_read_projection_heads WHERE
      owner_id=?1 AND node_id=?2 AND install_id=?3 AND credential_hash=?4 AND synced_at_ms IS NULL
      AND rejected=0 ORDER BY observed_at_ms LIMIT 32",
    )?;
    let rows = stmt.query_map(
        params![
            binding.owner,
            binding.node,
            binding.install,
            binding.credential_hash
        ],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, u64>(3)?,
                r.get::<_, u64>(4)?,
            ))
        },
    )?;
    rows.map(|row| {
        let (body, revision, connection, generation, observed) = row?;
        let value =
            parse(body.as_bytes(), now_ms()).map_err(|_| anyhow::anyhow!("projection_corrupt"))?;
        if value.revision != revision
            || value.connection_id != connection
            || value.generation != generation
            || value.observed_at_ms != observed
        {
            bail!("projection_corrupt");
        }
        Ok(value)
    })
    .collect()
}

pub(crate) fn settle(
    conn: &Connection,
    binding: &Binding<'_>,
    value: &Projection,
    at: u64,
    accepted: bool,
    rejected: bool,
    code: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE private_read_projection_heads SET synced_at_ms=?1,
      last_success_at_ms=COALESCE(?1,last_success_at_ms),rejected=?2,last_error_code=?3
      WHERE owner_id=?4 AND node_id=?5 AND install_id=?6 AND credential_hash=?7 AND source=?8
      AND connection_id=?9 AND revision=?10",
        params![
            accepted.then_some(at),
            rejected,
            code,
            binding.owner,
            binding.node,
            binding.install,
            binding.credential_hash,
            value.source,
            value.connection_id,
            value.revision
        ],
    )?;
    Ok(())
}
