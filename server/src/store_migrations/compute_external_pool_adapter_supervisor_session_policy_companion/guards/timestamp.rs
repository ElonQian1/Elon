use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (name, table, column) in [
        (
            "external_pool_adapter_supervisor_session_policy_companion_timestamp",
            "compute_external_pool_adapter_supervisor_session_policy_companions",
            "recorded_at",
        ),
        (
            "external_pool_adapter_supervisor_session_policy_companion_revocation_timestamp",
            "compute_external_pool_adapter_supervisor_session_policy_companion_revocations",
            "revoked_at",
        ),
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS {name} BEFORE INSERT ON {table}
             WHEN substr(NEW.{column},5,1)<>'-'
               OR substr(NEW.{column},8,1)<>'-'
               OR substr(NEW.{column},11,1)<>'T'
               OR substr(NEW.{column},14,1)<>':'
               OR substr(NEW.{column},17,1)<>':'
               OR substr(NEW.{column},20,1)<>'.'
               OR substr(NEW.{column},30,1)<>'Z'
               OR substr(NEW.{column},1,4) GLOB '*[^0-9]*'
               OR substr(NEW.{column},6,2) GLOB '*[^0-9]*'
               OR substr(NEW.{column},9,2) GLOB '*[^0-9]*'
               OR substr(NEW.{column},12,2) GLOB '*[^0-9]*'
               OR substr(NEW.{column},15,2) GLOB '*[^0-9]*'
               OR substr(NEW.{column},18,2) GLOB '*[^0-9]*'
               OR substr(NEW.{column},21,9) GLOB '*[^0-9]*'
               OR CAST(substr(NEW.{column},6,2) AS INTEGER) NOT BETWEEN 1 AND 12
               OR CAST(substr(NEW.{column},9,2) AS INTEGER) NOT BETWEEN 1 AND
                 CASE CAST(substr(NEW.{column},6,2) AS INTEGER)
                   WHEN 2 THEN CASE
                     WHEN CAST(substr(NEW.{column},1,4) AS INTEGER)%400=0
                       OR (CAST(substr(NEW.{column},1,4) AS INTEGER)%4=0
                           AND CAST(substr(NEW.{column},1,4) AS INTEGER)%100<>0)
                     THEN 29 ELSE 28 END
                   WHEN 4 THEN 30 WHEN 6 THEN 30 WHEN 9 THEN 30 WHEN 11 THEN 30
                   ELSE 31 END
               OR CAST(substr(NEW.{column},12,2) AS INTEGER) NOT BETWEEN 0 AND 23
               OR CAST(substr(NEW.{column},15,2) AS INTEGER) NOT BETWEEN 0 AND 59
               OR CAST(substr(NEW.{column},18,2) AS INTEGER) NOT BETWEEN 0 AND 59
               OR julianday(NEW.{column})<julianday('now','-5 minutes')
               OR julianday(NEW.{column})>julianday('now','+5 minutes')
             BEGIN SELECT RAISE(ABORT,'V259 timestamp is not a canonical civil UTC instant'); END;"
        ))?;
    }
    Ok(())
}
