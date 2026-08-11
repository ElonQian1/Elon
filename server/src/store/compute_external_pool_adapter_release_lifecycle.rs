//! Store authority for append-only external-pool Adapter release admission terminals.
//!
//! The immutable v222 admission remains historical `staged` material. This module overlays the
//! unique v229 terminal and exposes a sealed current-admission authority to Store consumers.

mod canonical;
mod read;
mod types;
mod write;

pub(crate) use types::{
    CreateExternalPoolAdapterReleaseAdmissionTerminal,
    ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt,
    ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
};

pub(in crate::store) use read::current_external_pool_adapter_release_admission_authority_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterReleaseAdmissionAuthority;
