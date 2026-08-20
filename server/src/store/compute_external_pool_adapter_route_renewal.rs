//! Store-private V278 historical recovery, ordered renewal, and current route authority.

mod builder;
mod pending;
mod persistence;
mod read;
mod receipt;
mod types;
mod writes;

pub(in crate::store) use builder::build_external_pool_adapter_route_renewal_receipt;
pub(crate) use pending::register_external_pool_adapter_route_renewal_pending_plan_function;
pub(in crate::store) use persistence::{
    finalize_external_pool_adapter_route_renewal_after_commit_on,
    renew_external_pool_adapter_route_on,
};
pub(in crate::store) use read::{
    external_pool_adapter_route_renewal_decision_on,
    external_pool_adapter_route_renewal_head_identity_on,
    historical_external_pool_adapter_route_recovery_authority_on,
    require_current_external_pool_adapter_renewed_route_on,
};
pub(crate) use receipt::RECEIPT_COLUMNS;
pub(in crate::store) use types::{
    CommittedExternalPoolAdapterRouteRenewal, CurrentExternalPoolAdapterRenewedRouteAuthority,
    ExternalPoolAdapterRouteRenewalDecision, ExternalPoolAdapterRouteRenewalDisposition,
    HistoricalExternalPoolAdapterRouteRecoveryAuthority,
    PendingExternalPoolAdapterRouteRenewalCommit,
};
