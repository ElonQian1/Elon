//! Consented task-result publications. No provider credentials or whole conversation snapshots.
use super::{new_id, now, Store};
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
pub(crate) mod model;
use model::{Share, Sync};

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS group_assistant_bindings (
      id TEXT PRIMARY KEY, group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
      owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      provider TEXT NOT NULL CHECK(provider='chatgpt'), account_scope TEXT NOT NULL,
      task_id TEXT NOT NULL, title TEXT NOT NULL, created_at TEXT NOT NULL,
      revoked_at TEXT, sync_state TEXT NOT NULL DEFAULT 'no_update',
      checked_at TEXT, synced_at TEXT);
      CREATE UNIQUE INDEX IF NOT EXISTS group_assistant_active_source
        ON group_assistant_bindings(group_id,owner_id,provider,account_scope,task_id) WHERE revoked_at IS NULL;
      CREATE TABLE IF NOT EXISTS group_assistant_updates (
        sequence INTEGER PRIMARY KEY AUTOINCREMENT, binding_id TEXT NOT NULL REFERENCES group_assistant_bindings(id) ON DELETE CASCADE,
        result_id TEXT NOT NULL, created_at TEXT NOT NULL, imported_at TEXT NOT NULL,
        content TEXT NOT NULL, UNIQUE(binding_id,result_id));
      CREATE INDEX IF NOT EXISTS group_assistant_update_page ON group_assistant_updates(binding_id,sequence);
      CREATE TRIGGER IF NOT EXISTS group_assistant_owner_left AFTER DELETE ON friend_group_members BEGIN
        UPDATE group_assistant_bindings SET revoked_at=COALESCE(revoked_at,strftime('%Y-%m-%dT%H:%M:%fZ','now'))
          WHERE group_id=OLD.group_id AND owner_id=OLD.user_id;
        DELETE FROM group_assistant_updates WHERE binding_id IN
          (SELECT id FROM group_assistant_bindings WHERE group_id=OLD.group_id AND owner_id=OLD.user_id);
      END;")?;
    Ok(())
}
fn member(conn: &Connection, user: &str, group: &str) -> Result<()> {
    ensure!(
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id=?1 AND user_id=?2)",
            params![group, user],
            |r| r.get::<_, bool>(0)
        )?,
        "membership required"
    );
    Ok(())
}
// Called within the same transaction as writes, so revoke and a late upload cannot race.
fn owner(conn: &Connection, user: &str, group: &str, id: &str) -> Result<(String, String)> {
    member(conn, user, group)?;
    Ok(conn.query_row(
        "SELECT account_scope,task_id FROM group_assistant_bindings
      WHERE id=?1 AND group_id=?2 AND owner_id=?3 AND revoked_at IS NULL",
        params![id, group, user],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}
impl Store {
    pub(crate) fn group_assistant_owned(&self, user: &str) -> Result<Value> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT b.id,b.group_id,b.task_id,b.account_scope,b.title FROM group_assistant_bindings b
          JOIN friend_group_members m ON m.group_id=b.group_id AND m.user_id=b.owner_id
          WHERE b.owner_id=?1 AND b.revoked_at IS NULL ORDER BY COALESCE(b.checked_at,''),b.id LIMIT 100")?;
        let items = stmt.query_map([user], |r| Ok(json!({"id":r.get::<_,String>(0)?,"group_id":r.get::<_,String>(1)?,
          "task_id":r.get::<_,String>(2)?,"account_scope":r.get::<_,String>(3)?,"title":r.get::<_,String>(4)?})))?
          .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"items":items}))
    }
    pub(crate) fn group_assistant_share(
        &self,
        user: &str,
        group: &str,
        body: &Share,
    ) -> Result<Value> {
        body.validate()?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        member(&tx, user, group)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM group_assistant_bindings WHERE group_id=?1
          AND owner_id=?2 AND account_scope=?3 AND task_id=?4 AND revoked_at IS NULL",
                params![group, user, body.account_scope, body.task_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(json!({"id":id,"replayed":true}));
        }
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM group_assistant_bindings WHERE owner_id=?1 AND revoked_at IS NULL",
          [user], |r| r.get(0))?;
        ensure!(count < 100, "subscription limit");
        let group_count: i64 = tx.query_row("SELECT COUNT(*) FROM group_assistant_bindings WHERE group_id=?1 AND revoked_at IS NULL",
          [group], |r| r.get(0))?;
        ensure!(group_count < 500, "group subscription limit");
        let id = new_id("gassist");
        tx.execute("INSERT INTO group_assistant_bindings(id,group_id,owner_id,provider,account_scope,task_id,title,created_at)
          VALUES(?1,?2,?3,'chatgpt',?4,?5,?6,?7)", params![id,group,user,body.account_scope,body.task_id,body.title,now()])?;
        tx.commit()?;
        Ok(json!({"id":id,"replayed":false}))
    }
    pub(crate) fn group_assistant_list(&self, user: &str, group: &str) -> Result<Value> {
        let conn = self.conn()?;
        member(&conn, user, group)?;
        let mut stmt = conn.prepare(
            "SELECT b.id,b.owner_id,b.title,b.created_at,b.sync_state,b.checked_at,b.synced_at,
          b.task_id,b.account_scope,COALESCE(NULLIF(u.nickname,''),'群友'),
          (SELECT COUNT(*) FROM group_assistant_updates x WHERE x.binding_id=b.id AND x.content<>'')
          FROM group_assistant_bindings b JOIN users u ON u.id=b.owner_id
          JOIN friend_group_members m ON m.group_id=b.group_id AND m.user_id=b.owner_id
          WHERE b.group_id=?1 AND b.revoked_at IS NULL ORDER BY b.created_at DESC,b.id LIMIT 500",
        )?;
        let items = stmt.query_map([group], |r| {
            let owned = r.get::<_,String>(1)? == user;
            Ok(json!({"id":r.get::<_,String>(0)?,"owned":owned,"title":r.get::<_,String>(2)?,
              "created_at":r.get::<_,String>(3)?,"state":r.get::<_,String>(4)?,"checked_at":r.get::<_,Option<String>>(5)?,
              "synced_at":r.get::<_,Option<String>>(6)?,"owner_name":r.get::<_,String>(9)?,"provider":"chatgpt",
              "task_id":if owned {r.get::<_,Option<String>>(7)?} else {None},
              "account_scope":if owned {r.get::<_,Option<String>>(8)?} else {None},"update_count":r.get::<_,i64>(10)?}))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!({"items":items}))
    }
    pub(crate) fn group_assistant_updates(
        &self,
        user: &str,
        group: &str,
        id: &str,
        before: i64,
    ) -> Result<Value> {
        let conn = self.conn()?;
        member(&conn, user, group)?;
        conn.query_row("SELECT b.id FROM group_assistant_bindings b JOIN friend_group_members m
          ON m.group_id=b.group_id AND m.user_id=b.owner_id WHERE b.id=?1 AND b.group_id=?2 AND b.revoked_at IS NULL",
          params![id,group], |_| Ok(()))?;
        let mut stmt = conn.prepare(
            "SELECT sequence,result_id,created_at,imported_at,content FROM group_assistant_updates
          WHERE binding_id=?1 AND content<>'' AND (?2=0 OR sequence<?2) ORDER BY sequence DESC LIMIT 21",
        )?;
        let mut items = stmt.query_map(params![id,before], |r| Ok(json!({"sequence":r.get::<_,i64>(0)?,
          "id":r.get::<_,String>(1)?,"created_at":r.get::<_,String>(2)?,"imported_at":r.get::<_,String>(3)?,
          "content":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let more = items.len() > 20;
        items.truncate(20);
        let cursor = if more {
            items.last().and_then(|x| x["sequence"].as_i64())
        } else {
            None
        };
        Ok(json!({"items":items,"next_cursor":cursor}))
    }
    pub(crate) fn group_assistant_sync(
        &self,
        user: &str,
        group: &str,
        id: &str,
        body: &Sync,
    ) -> Result<Value> {
        body.validate()?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let (scope, task) = owner(&tx, user, group, id)?;
        ensure!(
            scope == body.account_scope && task == body.task_id,
            "source changed"
        );
        let at = now();
        let inserted = if let Some(update) = &body.update {
            // The same provider result is immutable. Retries never create duplicate feed entries.
            tx.execute("INSERT OR IGNORE INTO group_assistant_updates(binding_id,result_id,created_at,imported_at,content)
              VALUES(?1,?2,?3,?4,?5)",params![id,update.id,update.created_at,at,update.content])? > 0
        } else {
            false
        };
        let success = ["ready", "no_update", "requires_action"].contains(&body.state.as_str());
        tx.execute(
            "UPDATE group_assistant_bindings SET sync_state=?1,checked_at=?2,
          synced_at=CASE WHEN ?3 THEN ?2 ELSE synced_at END WHERE id=?4",
            params![body.state, at, success, id],
        )?;
        // Keep only tiny result-id receipts after pruning bodies, so an old response cannot reappear.
        tx.execute("UPDATE group_assistant_updates SET content='' WHERE binding_id=?1 AND content<>'' AND sequence NOT IN
          (SELECT sequence FROM group_assistant_updates WHERE binding_id=?1 ORDER BY sequence DESC LIMIT 100)",[id])?;
        tx.commit()?;
        Ok(json!({"inserted":inserted,"state":body.state,"checked_at":at}))
    }
    pub(crate) fn group_assistant_revoke(
        &self,
        user: &str,
        group: &str,
        id: &str,
    ) -> Result<Value> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        member(&tx, user, group)?;
        // Group owners may remove a contribution, but may never read its provider binding.
        tx.query_row(
            "SELECT b.id FROM group_assistant_bindings b JOIN friend_groups g ON g.id=b.group_id
          WHERE b.id=?1 AND b.group_id=?2 AND (b.owner_id=?3 OR g.owner_user_id=?3)",
            params![id, group, user],
            |_| Ok(()),
        )?;
        tx.execute(
            "UPDATE group_assistant_bindings SET revoked_at=COALESCE(revoked_at,?1) WHERE id=?2",
            params![now(), id],
        )?;
        tx.execute(
            "DELETE FROM group_assistant_updates WHERE binding_id=?1",
            [id],
        )?;
        tx.commit()?;
        Ok(json!({"revoked":true}))
    }
}
