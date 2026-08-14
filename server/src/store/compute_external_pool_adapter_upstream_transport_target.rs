//! Append-only storage for durable inert brokered upstream transport targets.

mod audit;
mod broker_tls;
mod build;
mod current;
mod input;
mod persistence;
mod policy;
mod read;
mod roots;
mod types;
mod write;

pub(crate) mod api {
    pub(crate) use super::types::{
        CreateExternalPoolAdapterUpstreamTransportTarget,
        ExternalPoolAdapterUpstreamTransportTargetAuditTarget,
        ExternalPoolAdapterUpstreamTransportTargetCurrentness,
        ExternalPoolAdapterUpstreamTransportTargetDraft,
        ExternalPoolAdapterUpstreamTransportTargetPolicySummary,
        ExternalPoolAdapterUpstreamTransportTargetRevocationSummary,
        ExternalPoolAdapterUpstreamTransportTargetRevocationWriteReceipt,
        ExternalPoolAdapterUpstreamTransportTargetSummary,
        ExternalPoolAdapterUpstreamTransportTargetWriteReceipt,
        RevokeExternalPoolAdapterUpstreamTransportTarget,
    };
}

pub(in crate::store) use current::current_external_pool_adapter_upstream_transport_target_authority_on;
pub(in crate::store) use read::historical_external_pool_adapter_upstream_transport_target_authority_on;
pub(in crate::store) use roots::audit_replay_prepared;
pub(in crate::store) use types::CurrentExternalPoolAdapterUpstreamTransportTargetAuthority;
