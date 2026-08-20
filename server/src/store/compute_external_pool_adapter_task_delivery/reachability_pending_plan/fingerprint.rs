//! Type-preserving SQLite fingerprints for one V278 reachability write.

use rusqlite::types::{Value, ValueRef};

use super::ExternalPoolAdapterTaskReachabilityPendingWriteKind;

#[derive(Eq, PartialEq)]
pub(super) struct PendingWriteFingerprint {
    pub(super) kind: ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    pub(super) columns: Vec<PendingColumnFingerprint>,
}

#[derive(Eq, PartialEq)]
pub(super) struct PendingColumnFingerprint {
    ordinal: usize,
    sqlite_type: PendingSqliteType,
    byte_len: usize,
    value: Vec<u8>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PendingSqliteType {
    Null,
    Integer,
    Real,
    Text,
    Blob,
}

impl PendingColumnFingerprint {
    pub(super) fn from_value(ordinal: usize, value: Value) -> Self {
        match value {
            Value::Null => Self::new(ordinal, PendingSqliteType::Null, Vec::new()),
            Value::Integer(value) => Self::new(
                ordinal,
                PendingSqliteType::Integer,
                value.to_be_bytes().to_vec(),
            ),
            Value::Real(value) => Self::new(
                ordinal,
                PendingSqliteType::Real,
                value.to_bits().to_be_bytes().to_vec(),
            ),
            Value::Text(value) => Self::new(ordinal, PendingSqliteType::Text, value.into_bytes()),
            Value::Blob(value) => Self::new(ordinal, PendingSqliteType::Blob, value),
        }
    }

    pub(super) fn from_ref(ordinal: usize, value: ValueRef<'_>) -> Self {
        match value {
            ValueRef::Null => Self::new(ordinal, PendingSqliteType::Null, Vec::new()),
            ValueRef::Integer(value) => Self::new(
                ordinal,
                PendingSqliteType::Integer,
                value.to_be_bytes().to_vec(),
            ),
            ValueRef::Real(value) => Self::new(
                ordinal,
                PendingSqliteType::Real,
                value.to_bits().to_be_bytes().to_vec(),
            ),
            ValueRef::Text(value) => Self::new(ordinal, PendingSqliteType::Text, value.to_vec()),
            ValueRef::Blob(value) => Self::new(ordinal, PendingSqliteType::Blob, value.to_vec()),
        }
    }

    fn new(ordinal: usize, sqlite_type: PendingSqliteType, value: Vec<u8>) -> Self {
        Self {
            ordinal,
            sqlite_type,
            byte_len: value.len(),
            value,
        }
    }
}
