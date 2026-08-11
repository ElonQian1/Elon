mod admission;
mod request;
mod review;

use rusqlite::types::Type;

pub(super) use admission::{
    admission_by_adapter_release_on, admission_by_idempotency_on, admission_by_request_on,
};
pub(super) use request::{request_by_id_on, request_by_idempotency_on};
pub(super) use review::{review_by_idempotency_on, review_by_request_on};

fn decode<T: serde::de::DeserializeOwned>(json: &str, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}
