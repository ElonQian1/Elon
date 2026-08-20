//! Type-preserving SQLite fingerprints for one V278 V274 refresh INSERT.

use rusqlite::types::{Value, ValueRef};

#[derive(Eq, PartialEq)]
pub(super) struct RefreshPendingPlanFingerprint {
    columns: Vec<RefreshPendingColumnFingerprint>,
}

#[derive(Eq, PartialEq)]
struct RefreshPendingColumnFingerprint {
    ordinal: usize,
    sqlite_type: RefreshPendingSqliteType,
    byte_len: usize,
    value: Vec<u8>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RefreshPendingSqliteType {
    Null,
    Integer,
    Real,
    Text,
    Blob,
}

impl RefreshPendingPlanFingerprint {
    pub(super) fn from_values(values: Vec<Value>) -> Self {
        Self {
            columns: values
                .into_iter()
                .enumerate()
                .map(|(ordinal, value)| RefreshPendingColumnFingerprint::from_value(ordinal, value))
                .collect(),
        }
    }

    pub(super) fn from_context(context: &rusqlite::functions::Context<'_>) -> Self {
        Self {
            columns: (0..context.len())
                .map(|ordinal| {
                    RefreshPendingColumnFingerprint::from_ref(ordinal, context.get_raw(ordinal))
                })
                .collect(),
        }
    }
}

impl RefreshPendingColumnFingerprint {
    fn from_value(ordinal: usize, value: Value) -> Self {
        match value {
            Value::Null => Self::new(ordinal, RefreshPendingSqliteType::Null, Vec::new()),
            Value::Integer(value) => Self::new(
                ordinal,
                RefreshPendingSqliteType::Integer,
                value.to_be_bytes().to_vec(),
            ),
            Value::Real(value) => Self::new(
                ordinal,
                RefreshPendingSqliteType::Real,
                value.to_bits().to_be_bytes().to_vec(),
            ),
            Value::Text(value) => {
                Self::new(ordinal, RefreshPendingSqliteType::Text, value.into_bytes())
            }
            Value::Blob(value) => Self::new(ordinal, RefreshPendingSqliteType::Blob, value),
        }
    }

    fn from_ref(ordinal: usize, value: ValueRef<'_>) -> Self {
        match value {
            ValueRef::Null => Self::new(ordinal, RefreshPendingSqliteType::Null, Vec::new()),
            ValueRef::Integer(value) => Self::new(
                ordinal,
                RefreshPendingSqliteType::Integer,
                value.to_be_bytes().to_vec(),
            ),
            ValueRef::Real(value) => Self::new(
                ordinal,
                RefreshPendingSqliteType::Real,
                value.to_bits().to_be_bytes().to_vec(),
            ),
            ValueRef::Text(value) => {
                Self::new(ordinal, RefreshPendingSqliteType::Text, value.to_vec())
            }
            ValueRef::Blob(value) => {
                Self::new(ordinal, RefreshPendingSqliteType::Blob, value.to_vec())
            }
        }
    }

    fn new(ordinal: usize, sqlite_type: RefreshPendingSqliteType, value: Vec<u8>) -> Self {
        Self {
            ordinal,
            sqlite_type,
            byte_len: value.len(),
            value,
        }
    }
}
