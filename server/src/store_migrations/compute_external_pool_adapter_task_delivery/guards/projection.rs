use anyhow::Result;
use rusqlite::Connection;

mod events;
mod exchange;
mod polls;

pub(super) fn install(conn: &Connection) -> Result<()> {
    exchange::install(conn)?;
    polls::install(conn)?;
    events::install(conn)?;
    Ok(())
}

pub(super) fn install_projection(
    conn: &Connection,
    trigger: &str,
    table: &str,
    json_column: &str,
    fields: &[(&str, &str)],
) -> Result<()> {
    let mismatch = fields
        .iter()
        .map(|(column, path)| {
            format!(
                "json_extract(NEW.{json_column},'{}') IS NOT NEW.{column}",
                path.replace('\'', "''")
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {trigger}
         BEFORE INSERT ON {table}
         WHEN {mismatch}
         BEGIN SELECT RAISE(ABORT,'V273 canonical JSON scalar projection mismatch'); END;"
    ))?;
    Ok(())
}
