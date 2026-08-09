//! Pure fail-closed policy for the future handle-bound SQLite VFS.
//!
//! This module deliberately contains no SQLite connection, callback, live registry, or filesystem
//! operation. Possessing one of these values is not proof that a VFS has been registered or that
//! an authority database has been opened.
//!
//! The future ABI layer must keep the authorizer installed for the full connection lifetime,
//! disable extension loading at the connection and VFS layers, and route every `xOpen` through
//! the exact-name gate. This kernel alone cannot make SQLite open available.

use std::{ffi::CStr, fmt};

mod abi;
mod authorizer;
mod name;
mod registry;
mod types;

pub(super) use authorizer::{
    ManagedSqliteAuthorizerAction, ManagedSqliteAuthorizerDecision, ManagedSqliteAuthorizerRequest,
    ManagedSqliteAuthorizerTransitionError, ManagedSqliteTempSchemaAction,
};
pub(super) use name::ManagedSqliteLogicalNameRejection;
pub(super) use types::{
    ManagedSqliteLogicalFileRole, ManagedSqliteRootOpenFlags, ManagedSqliteVfsAccess,
    ManagedSqliteVfsOpenFlagRejection, ManagedSqliteVfsOpenRequest,
};

use authorizer::ManagedSqliteAuthorizerPolicy;
use name::{ManagedSqliteLogicalNames, ManagedSqliteOneShotToken};
use types::ManagedSqliteVfsOpenFlagPolicy;

/// A linear policy value bound to one opaque registry token.
///
/// Its constructor accepts no path, directory, raw handle, SQLite connection, or arbitrary
/// logical name. The future one-shot registry must mint the token and consume this policy together
/// with the exact pinned namespace custody.
#[must_use = "dropping the policy abandons its unconsumed logical-name authority"]
pub(super) struct SealedHandleBoundSqlitePolicy {
    logical_names: ManagedSqliteLogicalNames,
    authorizer: ManagedSqliteAuthorizerPolicy,
}

impl SealedHandleBoundSqlitePolicy {
    /// Kept private so only a future child registry can mint a bound policy. The token type and
    /// its constructor are not re-exported to the rest of `local_authority`.
    fn from_registry_nonce(nonce: [u8; 16]) -> Result<Self, ManagedSqliteLogicalNameRejection> {
        let token = ManagedSqliteOneShotToken::from_registry_nonce(nonce)?;
        Ok(Self {
            logical_names: ManagedSqliteLogicalNames::from_one_shot_token(token)?,
            authorizer: ManagedSqliteAuthorizerPolicy::bootstrap(),
        })
    }

    /// Exact root flags for the future `sqlite3_open_v2` call. URI is intentionally absent, but
    /// the bundled SQLite build enables URI interpretation globally, so the non-`file:` logical
    /// name remains an independent mandatory gate.
    pub(super) fn root_open_flags(&self) -> ManagedSqliteRootOpenFlags {
        ManagedSqliteRootOpenFlags::handle_bound_authority()
    }

    pub(super) fn logical_name(&self, role: ManagedSqliteLogicalFileRole) -> &CStr {
        self.logical_names.get(role)
    }

    fn classify_logical_name(
        &self,
        candidate_name: Option<&[u8]>,
    ) -> Result<ManagedSqliteLogicalFileRole, ManagedSqliteLogicalNameRejection> {
        self.logical_names.classify(candidate_name)
    }

    /// Validates the name and VFS flags as one inseparable request. A valid name can never bless
    /// flags for a different SQLite object role.
    pub(super) fn authorize_vfs_open(
        &self,
        candidate_name: Option<&[u8]>,
        raw_flags: i32,
    ) -> Result<ManagedSqliteVfsOpenRequest, ManagedSqliteVfsOpenFlagRejection> {
        let role = self
            .logical_names
            .classify(candidate_name)
            .map_err(ManagedSqliteVfsOpenFlagRejection::LogicalName)?;
        ManagedSqliteVfsOpenFlagPolicy::authorize(role, raw_flags)
    }

    pub(super) fn authorize_sql(
        &self,
        request: ManagedSqliteAuthorizerRequest<'_>,
    ) -> ManagedSqliteAuthorizerDecision {
        self.authorizer.authorize(request)
    }

    /// Consumes bootstrap privileges so they cannot be retained beside schema privileges.
    pub(super) fn into_schema_migration(
        self,
    ) -> Result<Self, ManagedSqliteAuthorizerTransitionError> {
        Ok(Self {
            logical_names: self.logical_names,
            authorizer: self.authorizer.into_schema_migration()?,
        })
    }

    /// Consumes schema privileges. Runtime policy permits no PRAGMA.
    pub(super) fn into_runtime(self) -> Result<Self, ManagedSqliteAuthorizerTransitionError> {
        Ok(Self {
            logical_names: self.logical_names,
            authorizer: self.authorizer.into_runtime()?,
        })
    }
}

impl fmt::Debug for SealedHandleBoundSqlitePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedHandleBoundSqlitePolicy")
            .field("logical_names", &"<opaque-one-shot>")
            .field("authorizer_phase", &self.authorizer.phase())
            .finish()
    }
}
