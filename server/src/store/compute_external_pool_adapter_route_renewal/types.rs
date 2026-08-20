use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::compute_federation::{
    external_pool_adapter_route_renewal::ExternalPoolAdapterRouteRenewalReceipt,
    route_authority::AuthorizedComputeRouteAuthorization,
};
use crate::store::compute_external_pool_adapter_provider_active_successor::HistoricalExternalPoolAdapterAtomicActivationAuthority;

use super::pending::ExternalPoolAdapterRouteRenewalPendingPlanGuard;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::store) enum ExternalPoolAdapterRouteRenewalDecision {
    Current {
        route_renewal_receipt_id: String,
        route_renewal_receipt_digest: String,
    },
    RenewalRequired {
        predecessor_route_renewal_receipt_id: Option<String>,
        predecessor_route_renewal_receipt_digest: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::store) enum ExternalPoolAdapterRouteRenewalDisposition {
    Inserted,
    ExactReplay,
}

/// Historical V277 plus its exact sequence-one V274 witness. It cannot expose a dispatch route.
pub(in crate::store) struct HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn> {
    activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn> {
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
        checked_at: String,
    ) -> Self {
        Self {
            activation,
            checked_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn activation(
        &self,
    ) -> &HistoricalExternalPoolAdapterAtomicActivationAuthority {
        &self.activation
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

/// Uncommitted receipt plus its fully consumed same-connection plan.
pub(in crate::store) struct PendingExternalPoolAdapterRouteRenewalCommit {
    pub(super) receipt: ExternalPoolAdapterRouteRenewalReceipt,
    pub(super) disposition: ExternalPoolAdapterRouteRenewalDisposition,
    pub(super) plan_guard: Option<ExternalPoolAdapterRouteRenewalPendingPlanGuard>,
}

pub(in crate::store) struct CommittedExternalPoolAdapterRouteRenewal {
    receipt: ExternalPoolAdapterRouteRenewalReceipt,
    disposition: ExternalPoolAdapterRouteRenewalDisposition,
}

impl CommittedExternalPoolAdapterRouteRenewal {
    pub(super) fn new(
        receipt: ExternalPoolAdapterRouteRenewalReceipt,
        disposition: ExternalPoolAdapterRouteRenewalDisposition,
    ) -> Self {
        Self {
            receipt,
            disposition,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterRouteRenewalReceipt {
        &self.receipt
    }

    pub(in crate::store) fn disposition(&self) -> ExternalPoolAdapterRouteRenewalDisposition {
        self.disposition
    }
}

/// Fresh, transaction-bound route authority. Immutable receipt history alone cannot mint it.
pub(in crate::store) struct CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn> {
    receipt: ExternalPoolAdapterRouteRenewalReceipt,
    route: AuthorizedComputeRouteAuthorization,
    checked_at: String,
    effective_expires_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn> {
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterRouteRenewalReceipt,
        route: AuthorizedComputeRouteAuthorization,
        checked_at: String,
        effective_expires_at: String,
    ) -> Self {
        Self {
            receipt,
            route,
            checked_at,
            effective_expires_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterRouteRenewalReceipt {
        &self.receipt
    }
    pub(in crate::store) fn route_authorization(&self) -> &AuthorizedComputeRouteAuthorization {
        &self.route
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
    pub(in crate::store) fn effective_expires_at(&self) -> &str {
        &self.effective_expires_at
    }
    pub(in crate::store) fn provider_binding_id(&self) -> &str {
        &self.receipt.renewal.identity.provider_binding_id
    }
    pub(in crate::store) fn provider_binding_digest(&self) -> &str {
        &self.receipt.renewal.identity.provider_binding_digest
    }
    pub(in crate::store) fn activation_receipt_id(&self) -> &str {
        &self
            .receipt
            .renewal
            .activation_witness
            .activation_receipt_id
    }
    pub(in crate::store) fn activation_receipt_digest(&self) -> &str {
        &self
            .receipt
            .renewal
            .activation_witness
            .activation_receipt_digest
    }
    pub(in crate::store) fn activation_genesis_successor_receipt_id(&self) -> &str {
        &self
            .receipt
            .renewal
            .activation_witness
            .activation_genesis_successor_receipt_id
    }
    pub(in crate::store) fn activation_genesis_successor_receipt_digest(&self) -> &str {
        &self
            .receipt
            .renewal
            .activation_witness
            .activation_genesis_successor_receipt_digest
    }
    pub(in crate::store) fn activation_root_digest(&self) -> &str {
        &self.receipt.renewal.identity.activation_root_digest
    }
    pub(in crate::store) fn executor_id(&self) -> &str {
        &self.receipt.renewal.stable_binding.executor_id
    }
    pub(in crate::store) fn stable_executor_binding_digest(&self) -> &str {
        &self
            .receipt
            .renewal
            .stable_binding
            .stable_executor_binding_digest
    }
    pub(in crate::store) fn route_adapter_projection_id(&self) -> &str {
        &self
            .receipt
            .renewal
            .stable_binding
            .route_adapter_projection_id
    }
    pub(in crate::store) fn projected_v211_adapter_binding_digest(&self) -> &str {
        &self
            .receipt
            .renewal
            .stable_binding
            .projected_v211_adapter_binding_digest
    }
}

pub(super) struct BuiltExternalPoolAdapterRouteRenewal {
    pub(super) receipt: ExternalPoolAdapterRouteRenewalReceipt,
    pub(super) route: AuthorizedComputeRouteAuthorization,
}
