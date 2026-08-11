//! Administrator-only orchestration for append-only Adapter release admission terminals.

use anyhow::Error as AnyError;
use serde::Deserialize;
use thiserror::Error;

use crate::store::{
    CreateExternalPoolAdapterReleaseAdmissionTerminal,
    ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt,
    ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt, Store,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
};

use super::external_pool_adapter_release_lifecycle::{
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateExternalPoolAdapterReleaseAdmissionTerminalBody {
    pub idempotency_key: String,
    pub expected_admission_digest: String,
    pub terminal_status: String,
    #[serde(default)]
    pub successor_admission_id: Option<String>,
    #[serde(default)]
    pub expected_successor_admission_digest: Option<String>,
    pub reason: String,
    pub confirm_terminal: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterReleaseAdmissionLifecycleServiceError {
    #[error("external-pool Adapter release admission was not found")]
    NotFound,
    #[error("external-pool Adapter release admission terminal request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter release admission lifecycle conflicts with immutable state")]
    Conflict(#[source] AnyError),
}

pub(crate) fn create_terminal_for_admin(
    store: &Store,
    admin_user_id: &str,
    admission_id: &str,
    body: CreateExternalPoolAdapterReleaseAdmissionTerminalBody,
) -> Result<
    ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt,
    ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
> {
    if !body.confirm_terminal {
        return Err(
            ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Invalid(anyhow::anyhow!(
                "追加 Adapter release admission 终态前必须显式确认"
            )),
        );
    }
    let confirmation = confirmation_for_status(&body.terminal_status)?;
    store
        .create_external_pool_adapter_release_admission_terminal(
            CreateExternalPoolAdapterReleaseAdmissionTerminal {
                admission_id: admission_id.to_string(),
                expected_admission_digest: body.expected_admission_digest,
                terminal_status: body.terminal_status,
                successor_admission_id: body.successor_admission_id,
                expected_successor_admission_digest: body.expected_successor_admission_digest,
                actor_id: admin_user_id.to_string(),
                reason: body.reason,
                confirmation: confirmation.to_string(),
                idempotency_scope: operation_scope(admin_user_id),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    admission_id: &str,
) -> Result<
    ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt,
    ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
> {
    store
        .external_pool_adapter_release_admission_currentness(admission_id)
        .map_err(ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Conflict)?
        .ok_or(ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::NotFound)
}

fn confirmation_for_status(
    status: &str,
) -> Result<&'static str, ExternalPoolAdapterReleaseAdmissionLifecycleServiceError> {
    match status {
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION)
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION)
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION)
        }
        _ => Err(
            ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Invalid(anyhow::anyhow!(
                "Adapter release admission terminal status is unsupported"
            )),
        ),
    }
}

fn operation_scope(admin_user_id: &str) -> String {
    format!("external-pool-adapter-release-admission-terminal:{admin_user_id}")
}
