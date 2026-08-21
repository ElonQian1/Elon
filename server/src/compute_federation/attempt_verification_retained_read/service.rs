use std::fmt;

use crate::store::{
    compute_federation_historical_causal_reference::ComputeAttemptRetainedVerificationResolveError,
    ComputeAttemptVerificationDecisionReceipt, Store,
};

pub(super) const INVALID_LEASE_ID: &str = "ATTEMPT_VERIFICATION_RETAINED_INVALID_LEASE_ID";
pub(super) const INVALID_REQUEST_INPUT: &str =
    "ATTEMPT_VERIFICATION_RETAINED_INVALID_REQUEST_INPUT";
pub(super) const UNAUTHENTICATED: &str = "ATTEMPT_VERIFICATION_RETAINED_UNAUTHENTICATED";
pub(super) const NOT_VISIBLE: &str = "ATTEMPT_VERIFICATION_RETAINED_NOT_VISIBLE";
pub(super) const PROJECT_FORBIDDEN: &str = "ATTEMPT_VERIFICATION_RETAINED_PROJECT_FORBIDDEN";
pub(super) const NOT_FOUND: &str = "ATTEMPT_VERIFICATION_RETAINED_NOT_FOUND";
pub(super) const INTEGRITY_CONFLICT: &str = "ATTEMPT_VERIFICATION_RETAINED_INTEGRITY_CONFLICT";
pub(super) const ADMIN_FORBIDDEN: &str = "ATTEMPT_VERIFICATION_RETAINED_ADMIN_FORBIDDEN";
pub(super) const INTERNAL_ERROR: &str = "ATTEMPT_VERIFICATION_RETAINED_INTERNAL_ERROR";

pub(super) enum AttemptVerificationRetainedReadError {
    InvalidLeaseId,
    NotVisible,
    ProjectForbidden,
    NotFound,
    IntegrityConflict,
    Unavailable,
}

impl AttemptVerificationRetainedReadError {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::InvalidLeaseId => INVALID_LEASE_ID,
            Self::NotVisible => NOT_VISIBLE,
            Self::ProjectForbidden => PROJECT_FORBIDDEN,
            Self::NotFound => NOT_FOUND,
            Self::IntegrityConflict => INTEGRITY_CONFLICT,
            Self::Unavailable => INTERNAL_ERROR,
        }
    }
}

impl fmt::Display for AttemptVerificationRetainedReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl fmt::Debug for AttemptVerificationRetainedReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for AttemptVerificationRetainedReadError {}

pub(super) type ReadResult =
    Result<ComputeAttemptVerificationDecisionReceipt, AttemptVerificationRetainedReadError>;

pub(super) fn read_for_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
    caller_project_id: Option<&str>,
) -> ReadResult {
    validate_lease_id(lease_id)?;
    let retained = store
        .resolve_compute_attempt_retained_verification(lease_id)
        .map_err(participant_resolve_error)?
        .ok_or(AttemptVerificationRetainedReadError::NotVisible)?;
    if !retained.permits_user(user_id) {
        return Err(AttemptVerificationRetainedReadError::NotVisible);
    }
    if caller_project_id.is_some_and(|project_id| !retained.belongs_to_project(project_id)) {
        return Err(AttemptVerificationRetainedReadError::ProjectForbidden);
    }
    Ok(retained.into_receipt())
}

pub(super) fn read_for_admin(store: &Store, lease_id: &str) -> ReadResult {
    validate_lease_id(lease_id)?;
    store
        .resolve_compute_attempt_retained_verification(lease_id)
        .map_err(admin_resolve_error)?
        .ok_or(AttemptVerificationRetainedReadError::NotFound)
        .map(|retained| retained.into_receipt())
}

fn participant_resolve_error(
    error: ComputeAttemptRetainedVerificationResolveError,
) -> AttemptVerificationRetainedReadError {
    match error {
        ComputeAttemptRetainedVerificationResolveError::Integrity { .. } => {
            AttemptVerificationRetainedReadError::NotVisible
        }
        ComputeAttemptRetainedVerificationResolveError::Operational { .. } => {
            AttemptVerificationRetainedReadError::Unavailable
        }
    }
}

fn admin_resolve_error(
    error: ComputeAttemptRetainedVerificationResolveError,
) -> AttemptVerificationRetainedReadError {
    match error {
        ComputeAttemptRetainedVerificationResolveError::Integrity { .. } => {
            AttemptVerificationRetainedReadError::IntegrityConflict
        }
        ComputeAttemptRetainedVerificationResolveError::Operational { .. } => {
            AttemptVerificationRetainedReadError::Unavailable
        }
    }
}

fn validate_lease_id(lease_id: &str) -> Result<(), AttemptVerificationRetainedReadError> {
    if lease_id.is_empty()
        || lease_id != lease_id.trim()
        || lease_id.len() > 200
        || lease_id.chars().any(char::is_control)
    {
        return Err(AttemptVerificationRetainedReadError::InvalidLeaseId);
    }
    Ok(())
}
