use anyhow::{Context, Result};
use rusqlite::Connection;

mod authority_fences;
mod core;
mod head_guards;
mod receipt_projection;
mod source_projection;

pub(super) fn create_schema_objects_v8(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(core::WORK_ADMISSION_CORE_SCHEMA_V8)
        .context("COMPUTE_PLUGIN_AUTHORITY_WORK_ADMISSION_CORE_SCHEMA_CREATE_V8")?;
    connection
        .execute_batch(head_guards::WORK_ADMISSION_HEAD_GUARDS_SCHEMA_V8)
        .context("COMPUTE_PLUGIN_AUTHORITY_WORK_ADMISSION_HEAD_SCHEMA_CREATE_V8")?;
    connection
        .execute_batch(source_projection::WORK_ADMISSION_SOURCE_PROJECTION_SCHEMA_V8)
        .context("COMPUTE_PLUGIN_AUTHORITY_WORK_ADMISSION_SOURCE_SCHEMA_CREATE_V8")?;
    connection
        .execute_batch(receipt_projection::WORK_ADMISSION_RECEIPT_PROJECTION_SCHEMA_V8)
        .context("COMPUTE_PLUGIN_AUTHORITY_WORK_ADMISSION_RECEIPT_SCHEMA_CREATE_V8")?;
    connection
        .execute_batch(authority_fences::WORK_ADMISSION_AUTHORITY_FENCES_SCHEMA_V8)
        .context("COMPUTE_PLUGIN_AUTHORITY_WORK_ADMISSION_FENCE_SCHEMA_CREATE_V8")?;
    Ok(())
}
