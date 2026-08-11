mod application;
mod batch;
mod review;

use rusqlite::types::Type;

pub(super) use application::{
    application_by_batch_on, application_by_idempotency_on, bindings_by_application_on,
    snapshot_binding_by_snapshot_on,
};
pub(super) use batch::{
    batch_by_curve_on, batch_by_id_on, batch_by_idempotency_on, entries_by_batch_on, entry_by_id_on,
};
pub(super) use review::{review_by_batch_on, review_by_idempotency_on};

fn decode<T: serde::de::DeserializeOwned>(json: &str, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}
