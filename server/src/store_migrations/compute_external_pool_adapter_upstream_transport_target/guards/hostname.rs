use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_hostname
        BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_targets
        WHEN length(NEW.dns_hostname)<>length(CAST(NEW.dns_hostname AS BLOB))
          OR NEW.dns_hostname GLOB '*[^a-z0-9.-]*'
          OR NEW.dns_hostname NOT GLOB '*[a-z]*'
          OR instr(NEW.dns_hostname,'.')=0
          OR substr(NEW.dns_hostname,-1,1)='.'
          OR EXISTS (
            WITH RECURSIVE labels(rest,label) AS (
              SELECT NEW.dns_hostname || '.', NULL
              UNION ALL
              SELECT substr(rest,instr(rest,'.')+1), substr(rest,1,instr(rest,'.')-1)
                FROM labels WHERE rest<>''
            )
            SELECT 1 FROM labels
             WHERE label IS NOT NULL
               AND (length(CAST(label AS BLOB)) NOT BETWEEN 1 AND 63
                    OR substr(label,1,1) NOT GLOB '[a-z0-9]'
                    OR substr(label,-1,1) NOT GLOB '[a-z0-9]'
                    OR label GLOB '*[^a-z0-9-]*'))
        BEGIN SELECT RAISE(ABORT,'V258 hostname is not a canonical lowercase A-label DNS name'); END;
        "#,
    )?;
    Ok(())
}
