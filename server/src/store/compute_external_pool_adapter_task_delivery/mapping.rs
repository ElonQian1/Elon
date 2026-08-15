mod events;
mod exchange;
mod polls;

use anyhow::Result;
use rusqlite::types::Value;
use serde::Serialize;

use crate::compute_federation::external_pool_adapter_task_protocol_production::TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES;

pub(super) use events::*;
pub(super) use exchange::*;
pub(super) use polls::*;

pub(super) fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

pub(super) fn optional_text(value: Option<&str>) -> Value {
    value.map(text).unwrap_or(Value::Null)
}

pub(super) fn integer(value: u64) -> Result<Value> {
    Ok(Value::Integer(i64::try_from(value)?))
}

pub(super) fn canonical_value<T: Serialize + ?Sized>(value: &T) -> Result<Value> {
    Ok(Value::Text(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES,
        )?
        .0,
    ))
}
