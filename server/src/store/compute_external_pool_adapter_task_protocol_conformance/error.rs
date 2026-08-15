use anyhow::Error as AnyError;
use rusqlite::{ffi::ErrorCode, Error as SqliteError};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterTaskProtocolConformanceStoreError {
    #[error("external-pool Adapter task-protocol conformance conflicts")]
    Conflict(#[source] AnyError),
    #[error("external-pool Adapter task-protocol conformance storage failed")]
    Storage(#[source] AnyError),
}

impl From<SqliteError> for ExternalPoolAdapterTaskProtocolConformanceStoreError {
    fn from(error: SqliteError) -> Self {
        Self::classify_write(AnyError::new(error))
    }
}

impl ExternalPoolAdapterTaskProtocolConformanceStoreError {
    pub(super) fn conflict(error: impl Into<AnyError>) -> Self {
        Self::Conflict(error.into())
    }

    pub(super) fn storage(error: impl Into<AnyError>) -> Self {
        Self::Storage(error.into())
    }

    pub(super) fn classify_write(error: AnyError) -> Self {
        if error.chain().any(|cause| {
            cause
                .downcast_ref::<SqliteError>()
                .and_then(SqliteError::sqlite_error_code)
                == Some(ErrorCode::ConstraintViolation)
        }) {
            Self::Conflict(error)
        } else if error
            .chain()
            .any(|cause| cause.downcast_ref::<SqliteError>().is_some())
        {
            Self::Storage(error)
        } else {
            Self::Conflict(error)
        }
    }
}
