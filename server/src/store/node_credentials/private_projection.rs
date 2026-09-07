//! Node identity is derived from the current durable database credential, never payload owner.
use super::legacy_registration::require_legacy_credential_current_on;
use crate::{
    private_read_projection::{
        self as contract,
        storage::{self, Binding},
        Projection,
    },
    store::Store,
};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;

impl Store {
    pub(crate) fn put_private_read_projection(
        &self,
        node: &str,
        secret: &str,
        value: &Projection,
        at: u64,
    ) -> Result<bool> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let hash = contract::digest(secret.as_bytes());
        let (owner, install) = authorize(&tx, node, &hash)?;
        let changed = storage::put(
            &tx,
            &Binding {
                owner: &owner,
                node,
                install: &install,
                credential_hash: &hash,
            },
            value,
            at,
        )?;
        tx.commit()?; // The API cannot acknowledge until the durable commit succeeds.
        Ok(changed)
    }
}
fn authorize(tx: &Transaction<'_>, node: &str, hash: &str) -> Result<(String, String)> {
    let row: Option<(String, String)> = tx
        .query_row(
            "SELECT c.owner_user_id,c.install_id FROM node_credentials c
      JOIN users u ON u.id=c.owner_user_id WHERE c.agent_id=?1 AND u.status='active'
      AND u.id<>'local-owner' AND c.install_id IS NOT NULL AND length(c.install_id)>0",
            params![node],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((owner, install)) = row else {
        bail!("projection_node_unauthorized");
    };
    require_legacy_credential_current_on(tx, node, Some(&owner), Some(&install), Some(hash), false)
        .map_err(|_| anyhow::anyhow!("projection_node_unauthorized"))?;
    Ok((owner, install))
}
pub(in crate::store) fn read_on(tx: &Transaction<'_>, owner: &str, at: u64) -> Result<Vec<Value>> {
    let mut stmt=tx.prepare("SELECT p.node_id,p.install_id,p.credential_hash,p.body FROM private_read_projection_heads p
      JOIN node_credentials c ON c.agent_id=p.node_id AND c.owner_user_id=p.owner_id
      AND c.install_id=p.install_id AND c.secret_hash=p.credential_hash
      JOIN users u ON u.id=p.owner_id AND u.status='active'
      WHERE p.owner_id=?1 ORDER BY p.node_id,p.source,p.connection_id LIMIT 33")?;
    let rows = stmt.query_map(params![owner], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut values = Vec::new();
    for row in rows {
        let (node, install, hash, body) = row?;
        // Revocation, secret rotation and endpoint-root adoption all invalidate old snapshots.
        if require_legacy_credential_current_on(
            tx,
            &node,
            Some(owner),
            Some(&install),
            Some(&hash),
            false,
        )
        .is_err()
        {
            continue;
        }
        let value = contract::parse(body.as_bytes(), at)
            .map_err(|_| anyhow::anyhow!("projection_corrupt"))?;
        let binding = Binding {
            owner,
            node: &node,
            install: &install,
            credential_hash: &hash,
        };
        values.push(value.projected(&binding.projection_id(&value), at));
    }
    if values.len() > contract::MAX_SOURCES
        || serde_json::to_vec(&values)?.len() > contract::MAX_BYTES - 4096
    {
        bail!("projection_read_limit");
    }
    Ok(values)
}
pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    storage::migrate(conn)
}
