//! Store-private receipt for server-owned external-pool Adapter artifact bytes.
//!
//! This ledger records exact v222 lineage plus sealed quarantine-byte evidence. It does not
//! resolve the candidate reference, verify an Adapter, or make filesystem and SQLite writes one
//! atomic transaction.

mod canonical;
mod read;
mod types;
mod write;

pub(crate) use types::{
    ExternalPoolAdapterArtifactIntakeAuthority, ExternalPoolAdapterArtifactSourceReceipt,
    RecordExternalPoolAdapterArtifactSource,
    EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION,
};

pub(in crate::store) use read::external_pool_adapter_artifact_source_authority_on;
pub(in crate::store) use types::ExternalPoolAdapterArtifactSourceAuthority;
