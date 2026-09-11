use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v293(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT game_rewards_v293")?;
    let result = (|| -> Result<()> {
        conn.execute_batch(include_str!("schema.sql"))?;
        for (table, collision) in [
            ("game_reward_policy", "old.singleton=NEW.singleton OR old.digest=NEW.digest"),
            ("game_reward_reports", "old.digest=NEW.digest OR (old.user_id=NEW.user_id AND old.sequence=NEW.sequence)"),
            ("game_reward_intents", "old.allocation_hash=NEW.allocation_hash OR old.digest=NEW.digest"),
            ("game_reward_funding", "old.allocation_hash=NEW.allocation_hash OR old.budget_id=NEW.budget_id OR old.evidence_digest=NEW.evidence_digest"),
        ] {
            // BEFORE INSERT also prevents INSERT OR REPLACE, even with recursive triggers disabled.
            conn.execute_batch(&format!(
                "CREATE TRIGGER IF NOT EXISTS {table}_no_update BEFORE UPDATE ON {table} BEGIN SELECT RAISE(ABORT,'reward records are immutable'); END;
                 CREATE TRIGGER IF NOT EXISTS {table}_no_delete BEFORE DELETE ON {table} BEGIN SELECT RAISE(ABORT,'reward records are immutable'); END;
                 CREATE TRIGGER IF NOT EXISTS {table}_no_replace BEFORE INSERT ON {table}
                 WHEN EXISTS(SELECT 1 FROM {table} old WHERE {collision}) BEGIN SELECT RAISE(ABORT,'reward record already exists'); END;"
            ))?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        conn.execute_batch("ROLLBACK TO game_rewards_v293; RELEASE game_rewards_v293")?;
        return Err(error);
    }
    conn.execute_batch("RELEASE game_rewards_v293")?;
    Ok(())
}
