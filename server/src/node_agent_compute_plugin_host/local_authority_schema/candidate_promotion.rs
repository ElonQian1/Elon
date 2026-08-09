use anyhow::{Context, Result};
use rusqlite::Connection;

mod core;
mod projection_guards;
mod receipt_projection_guards;
mod replacement_guards;

pub(super) fn create_schema_objects_v7(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(core::CANDIDATE_PROMOTION_CORE_SCHEMA_V7)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_PROMOTION_SCHEMA_CREATE_V7")?;
    connection
        .execute_batch(projection_guards::CANDIDATE_PROMOTION_PROJECTION_SCHEMA_V7)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_PROJECTION_SCHEMA_CREATE_V7")?;
    connection
        .execute_batch(receipt_projection_guards::CANDIDATE_PROMOTION_RECEIPT_PROJECTION_SCHEMA_V7)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_RECEIPT_SCHEMA_CREATE_V7")?;
    connection
        .execute_batch(replacement_guards::CANDIDATE_PROMOTION_REPLACEMENT_GUARDS_V7)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_REPLACEMENT_GUARDS_V7")?;
    Ok(())
}
