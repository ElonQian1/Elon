//! Immutable V244 Adapter adoption authorities and append-only revocation terminals.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    current_external_pool_adapter_adoption_authority_on,
    external_pool_adapter_adoption_is_revoked_on,
    external_pool_adapter_adoption_receipt_authority_on,
};
pub(crate) use types::{
    AdoptExternalPoolAdapter, ExternalPoolAdapterAdoptionCurrentness,
    ExternalPoolAdapterAdoptionWriteReceipt, RevokeExternalPoolAdapterAdoption,
};
pub(in crate::store) use types::{
    CurrentExternalPoolAdapterAdoptionAuthority, HistoricalExternalPoolAdapterAdoptionAuthority,
};
