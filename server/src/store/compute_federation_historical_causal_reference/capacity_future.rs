use std::fmt;

use anyhow::{Error as AnyhowError, Result};
use rusqlite::{Connection, TransactionBehavior};

use crate::compute_federation::capacity_future_settlement_lineage::{
    build_compute_capacity_future_settlement_lineage,
    ProjectedComputeCapacityFutureSettlementLineageV1,
};

use super::{FederationHistoricalLineageAccessScope, Store};

mod capacity;
mod owners;

pub(crate) struct ValidatedComputeCapacityFutureSettlementLineageV1 {
    canonical_json: String,
    lineage_digest: String,
    access_scope: FederationHistoricalLineageAccessScope,
}

pub(crate) enum ComputeCapacityFutureSettlementLineageResolveError {
    Integrity { source: AnyhowError },
    Operational { source: AnyhowError },
}

impl ComputeCapacityFutureSettlementLineageResolveError {
    fn integrity(source: impl Into<AnyhowError>) -> Self {
        Self::Integrity {
            source: source.into(),
        }
    }

    fn operational(source: impl Into<AnyhowError>) -> Self {
        Self::Operational {
            source: source.into(),
        }
    }
}

impl fmt::Display for ComputeCapacityFutureSettlementLineageResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integrity { source } => write!(
                formatter,
                "capacity-future retained lineage integrity failure: {source:#}"
            ),
            Self::Operational { source } => write!(
                formatter,
                "capacity-future retained lineage operational failure: {source:#}"
            ),
        }
    }
}

impl fmt::Debug for ComputeCapacityFutureSettlementLineageResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl ValidatedComputeCapacityFutureSettlementLineageV1 {
    fn from_projected(
        projected: ProjectedComputeCapacityFutureSettlementLineageV1,
        access_scope: FederationHistoricalLineageAccessScope,
    ) -> Result<Self> {
        Ok(Self {
            canonical_json: projected.canonical_json()?,
            lineage_digest: projected.lineage_digest().to_string(),
            access_scope,
        })
    }

    pub(crate) fn canonical_json(&self) -> &str {
        &self.canonical_json
    }

    pub(crate) fn lineage_digest(&self) -> &str {
        &self.lineage_digest
    }

    pub(crate) fn permits_user(&self, user_id: &str) -> bool {
        self.access_scope.permits_user(user_id)
    }

    pub(crate) fn belongs_to_project(&self, project_id: &str) -> bool {
        self.access_scope.belongs_to_project(project_id)
    }
}

impl Store {
    pub(crate) fn resolve_compute_capacity_future_settlement_lineage_for_lease(
        &self,
        lease_id: &str,
    ) -> std::result::Result<
        Option<ValidatedComputeCapacityFutureSettlementLineageV1>,
        ComputeCapacityFutureSettlementLineageResolveError,
    > {
        let mut conn = self
            .conn()
            .map_err(ComputeCapacityFutureSettlementLineageResolveError::operational)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(ComputeCapacityFutureSettlementLineageResolveError::operational)?;
        let resolved = resolve_capacity_future_settlement_lineage_on(&tx, lease_id)
            .map_err(classify_capacity_future_settlement_lineage_owner_error)?;
        tx.commit()
            .map_err(ComputeCapacityFutureSettlementLineageResolveError::operational)?;
        Ok(resolved)
    }
}

pub(super) fn resolve_capacity_future_settlement_lineage_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<ValidatedComputeCapacityFutureSettlementLineageV1>> {
    let Some(owners) = owners::resolve_capacity_future_settlement_owners_on(conn, lease_id)? else {
        return Ok(None);
    };
    capacity::validate_capacity_future_historical_owners(
        owners.instrument_source(),
        owners.allocation_source(),
    )?;
    let projected = {
        let sources = owners.sources();
        build_compute_capacity_future_settlement_lineage(&sources)?
    };
    let access_scope = owners.into_access_scope();
    ValidatedComputeCapacityFutureSettlementLineageV1::from_projected(projected, access_scope)
        .map(Some)
}

fn classify_capacity_future_settlement_lineage_owner_error(
    source: AnyhowError,
) -> ComputeCapacityFutureSettlementLineageResolveError {
    let contains_operational_query_error = source.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .is_some_and(|error| match error {
                rusqlite::Error::QueryReturnedNoRows
                | rusqlite::Error::FromSqlConversionFailure(..)
                | rusqlite::Error::IntegralValueOutOfRange(..)
                | rusqlite::Error::Utf8Error(..)
                | rusqlite::Error::InvalidColumnType(..) => false,
                _ => true,
            })
    });
    if contains_operational_query_error {
        ComputeCapacityFutureSettlementLineageResolveError::operational(source)
    } else {
        ComputeCapacityFutureSettlementLineageResolveError::integrity(source)
    }
}
