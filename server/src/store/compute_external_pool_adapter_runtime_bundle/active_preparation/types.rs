use crate::compute_federation::{
    external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
    external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalPoolAdapterActivePreparationIdentity {
    pub(crate) provider_id: String,
    pub(crate) provider_binding_id: String,
    pub(crate) activation_root_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExternalPoolAdapterActivePreparationCycleDisposition {
    AlreadyCurrent,
    Renewed,
    Refreshed,
    RenewedAndRefreshed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalPoolAdapterActivePreparationCycleOutcome {
    pub(crate) identity: ExternalPoolAdapterActivePreparationIdentity,
    pub(crate) disposition: ExternalPoolAdapterActivePreparationCycleDisposition,
}

pub(super) struct ExternalPoolAdapterActivePreparationCandidate {
    pub(super) identity: ExternalPoolAdapterActivePreparationIdentity,
    pub(super) activation_receipt_id: String,
    pub(super) activation_receipt_digest: String,
    pub(super) activation_genesis_successor_receipt_id: String,
    pub(super) activation_genesis_successor_receipt_digest: String,
    pub(super) installation_binding: ExternalPoolAdapterInstallationBinding,
    pub(super) target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
}

pub(super) struct ExternalPoolAdapterRegisteringActivationCandidate {
    pub(super) provider_id: String,
    pub(super) provider_binding_id: String,
    pub(super) provider_binding_digest: String,
    pub(super) companion_id: String,
    pub(super) companion_digest: String,
    pub(super) runtime_compatibility_verification_receipt_id: String,
    pub(super) runtime_compatibility_verification_receipt_digest: String,
    pub(super) installation_binding: ExternalPoolAdapterInstallationBinding,
}

pub(super) enum ExternalPoolAdapterRegisteringActivationDisposition {
    NoCandidate,
    Deferred,
    Activated,
}
