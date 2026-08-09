use anyhow::{bail, Context, Result};
use rusqlite::{Connection, TransactionBehavior};

use super::{
    create_schema_objects_v4_additions, create_schema_objects_v5_additions,
    create_schema_objects_v6_additions, create_schema_objects_v7,
    create_schema_objects_v7_additions, schema_integrity,
};

/// The singleton row shape remains V3. Database `user_version` advances independently because V4
/// through V7 add append-only journals and exact transition triggers without rebuilding that row.
pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION: i64 =
    3;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION: i64 = 5;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V4: i64 = 4;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V6: i64 = 6;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7: i64 = 7;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID: i64 = 0x454c_4350;

pub(in crate::node_agent_compute_plugin_host) fn ensure_schema(
    connection: &mut Connection,
) -> Result<()> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_VERSION_READ")?;
    let application_id = connection
        .pragma_query_value(None, "application_id", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID_READ")?;
    match version {
        0 if application_id == 0 => install_schema_v7(connection),
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            migrate_schema_v3_to_v7(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V4
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            migrate_schema_v4_to_v7(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            migrate_schema_v5_to_v7(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V6
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            migrate_schema_v6_to_v7(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            verify_required_objects_v7(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
        | COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V4
        | COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION
        | COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V6
        | COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7 => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID: database belongs to another application"
        ),
        0 => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID: unversioned database is already claimed"
        ),
        other => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNSUPPORTED: database version {other} is not supported"
        ),
    }
}

fn install_schema_v7(connection: &mut Connection) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_BEGIN")?;
    require_unclaimed_database(&transaction)?;
    create_schema_objects_v7(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_VERSION_WRITE")?;
    transaction
        .pragma_update(
            None,
            "application_id",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_COMMIT")?;
    verify_required_objects_v7(connection)
}

fn migrate_schema_v3_to_v7(connection: &mut Connection) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_BEGIN")?;
    // The legacy fingerprint and foreign-key proof must be acquired under the same write fence as
    // the DDL. Verifying before BEGIN IMMEDIATE would leave a schema-drift TOCTOU window.
    verify_required_objects_v3(&transaction)?;
    create_schema_objects_v4_additions(&transaction)?;
    create_schema_objects_v5_additions(&transaction)?;
    create_schema_objects_v6_additions(&transaction)?;
    create_schema_objects_v7_additions(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_VERSION_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_COMMIT")?;
    verify_required_objects_v7(connection)
}

fn migrate_schema_v4_to_v7(connection: &mut Connection) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_BEGIN")?;
    verify_required_objects_v4(&transaction)?;
    create_schema_objects_v5_additions(&transaction)?;
    create_schema_objects_v6_additions(&transaction)?;
    create_schema_objects_v7_additions(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_VERSION_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_COMMIT")?;
    verify_required_objects_v7(connection)
}

fn migrate_schema_v5_to_v7(connection: &mut Connection) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_BEGIN")?;
    verify_required_objects_v5(&transaction)?;
    create_schema_objects_v6_additions(&transaction)?;
    create_schema_objects_v7_additions(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_VERSION_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_COMMIT")?;
    verify_required_objects_v7(connection)
}

fn migrate_schema_v6_to_v7(connection: &mut Connection) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_BEGIN")?;
    verify_required_objects_v6(&transaction)?;
    create_schema_objects_v7_additions(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_DATABASE_VERSION_V7,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_VERSION_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_V7_MIGRATION_COMMIT")?;
    verify_required_objects_v7(connection)
}

fn verify_required_objects_v3(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_v3(connection)?;
    verify_foreign_keys(connection)
}

fn verify_required_objects_v4(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_v4(connection)?;
    verify_foreign_keys(connection)
}

fn verify_required_objects_v5(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_v5(connection)?;
    verify_foreign_keys(connection)
}

fn verify_required_objects_v6(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_v6(connection)?;
    verify_foreign_keys(connection)
}

fn verify_required_objects_v7(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_v7(connection)?;
    verify_foreign_keys(connection)
}

fn verify_foreign_keys(connection: &Connection) -> Result<()> {
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK_PREPARE")?;
    let mut violations = statement
        .query([])
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK")?;
    if violations
        .next()
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK_READ")?
        .is_some()
    {
        bail!("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_VIOLATION");
    }
    Ok(())
}

fn require_unclaimed_database(connection: &Connection) -> Result<()> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_CLAIM_VERSION_READ")?;
    let application_id = connection
        .pragma_query_value(None, "application_id", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_CLAIM_APPLICATION_ID_READ")?;
    let user_objects = connection
        .query_row(
            r#"SELECT COUNT(*) FROM sqlite_schema
               WHERE type IN ('table', 'index', 'trigger', 'view')
                 AND name NOT GLOB 'sqlite_*'"#,
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_CLAIM_OBJECTS_READ")?;
    if version != 0 || application_id != 0 || user_objects != 0 {
        bail!(
            "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNVERSIONED: refusing to adopt an existing database"
        );
    }
    Ok(())
}
