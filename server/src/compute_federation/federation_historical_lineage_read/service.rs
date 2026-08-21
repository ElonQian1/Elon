use std::fmt;

use crate::{
    compute_federation::federation_historical_causal_reference::FederationHistoricalLineageKindV1,
    store::{
        compute_federation_historical_causal_reference::ValidatedFederationHistoricalLineage, Store,
    },
};

use super::transport::FederationHistoricalLineageReadDocument;

pub(super) const INVALID_LEASE_ID: &str = "FEDERATION_LINEAGE_INVALID_LEASE_ID";
pub(super) const INVALID_REQUEST_INPUT: &str = "FEDERATION_LINEAGE_INVALID_REQUEST_INPUT";
pub(super) const NOT_VISIBLE: &str = "FEDERATION_LINEAGE_NOT_VISIBLE";
pub(super) const PROJECT_FORBIDDEN: &str = "FEDERATION_LINEAGE_PROJECT_FORBIDDEN";
pub(super) const NOT_FOUND: &str = "FEDERATION_LINEAGE_NOT_FOUND";
pub(super) const INTEGRITY_CONFLICT: &str = "FEDERATION_LINEAGE_INTEGRITY_CONFLICT";
pub(super) const ADMIN_FORBIDDEN: &str = "FEDERATION_LINEAGE_ADMIN_FORBIDDEN";
pub(super) const UNAUTHENTICATED: &str = "FEDERATION_LINEAGE_UNAUTHENTICATED";

pub(super) enum FederationHistoricalLineageReadError {
    InvalidLeaseId,
    NotVisible,
    ProjectForbidden,
    NotFound,
    IntegrityConflict,
}

impl FederationHistoricalLineageReadError {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::InvalidLeaseId => INVALID_LEASE_ID,
            Self::NotVisible => NOT_VISIBLE,
            Self::ProjectForbidden => PROJECT_FORBIDDEN,
            Self::NotFound => NOT_FOUND,
            Self::IntegrityConflict => INTEGRITY_CONFLICT,
        }
    }
}

impl fmt::Display for FederationHistoricalLineageReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl fmt::Debug for FederationHistoricalLineageReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FederationHistoricalLineageReadError {}

pub(super) type ReadResult =
    Result<FederationHistoricalLineageReadDocument, FederationHistoricalLineageReadError>;

pub(super) fn read_execution_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
    caller_project_id: Option<&str>,
) -> ReadResult {
    validate_lease_id(lease_id)?;
    let lineage = store
        .resolve_compute_execution_source_lineage_for_lease(lease_id)
        .map_err(|_| FederationHistoricalLineageReadError::NotVisible)?
        .ok_or(FederationHistoricalLineageReadError::NotVisible)?;
    participant_document(
        lineage,
        user_id,
        caller_project_id,
        FederationHistoricalLineageKindV1::ExecutionSourceV1,
    )
}

pub(super) fn read_settlement_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
    caller_project_id: Option<&str>,
) -> ReadResult {
    validate_lease_id(lease_id)?;
    let lineage = store
        .resolve_compute_settlement_source_lineage_for_lease(lease_id)
        .map_err(|_| FederationHistoricalLineageReadError::NotVisible)?
        .ok_or(FederationHistoricalLineageReadError::NotVisible)?;
    participant_document(
        lineage,
        user_id,
        caller_project_id,
        FederationHistoricalLineageKindV1::SettlementSourceV1,
    )
}

pub(super) fn read_execution_for_admin(store: &Store, lease_id: &str) -> ReadResult {
    validate_lease_id(lease_id)?;
    let lineage = store
        .resolve_compute_execution_source_lineage_for_lease(lease_id)
        .map_err(|_| FederationHistoricalLineageReadError::IntegrityConflict)?
        .ok_or(FederationHistoricalLineageReadError::NotFound)?;
    admin_document(
        lineage,
        FederationHistoricalLineageKindV1::ExecutionSourceV1,
    )
}

pub(super) fn read_settlement_for_admin(store: &Store, lease_id: &str) -> ReadResult {
    validate_lease_id(lease_id)?;
    let lineage = store
        .resolve_compute_settlement_source_lineage_for_lease(lease_id)
        .map_err(|_| FederationHistoricalLineageReadError::IntegrityConflict)?
        .ok_or(FederationHistoricalLineageReadError::NotFound)?;
    admin_document(
        lineage,
        FederationHistoricalLineageKindV1::SettlementSourceV1,
    )
}

fn participant_document(
    lineage: ValidatedFederationHistoricalLineage,
    user_id: &str,
    caller_project_id: Option<&str>,
    expected_kind: FederationHistoricalLineageKindV1,
) -> ReadResult {
    if lineage.kind() != expected_kind || !lineage.permits_user(user_id) {
        return Err(FederationHistoricalLineageReadError::NotVisible);
    }
    if caller_project_id.is_some_and(|project_id| !lineage.belongs_to_project(project_id)) {
        return Err(FederationHistoricalLineageReadError::ProjectForbidden);
    }
    Ok(FederationHistoricalLineageReadDocument::from_validated(
        lineage,
    ))
}

fn admin_document(
    lineage: ValidatedFederationHistoricalLineage,
    expected_kind: FederationHistoricalLineageKindV1,
) -> ReadResult {
    if lineage.kind() != expected_kind {
        return Err(FederationHistoricalLineageReadError::IntegrityConflict);
    }
    Ok(FederationHistoricalLineageReadDocument::from_validated(
        lineage,
    ))
}

fn validate_lease_id(lease_id: &str) -> Result<(), FederationHistoricalLineageReadError> {
    if lease_id.is_empty()
        || lease_id != lease_id.trim()
        || lease_id.len() > 200
        || lease_id.chars().any(char::is_control)
    {
        return Err(FederationHistoricalLineageReadError::InvalidLeaseId);
    }
    Ok(())
}
