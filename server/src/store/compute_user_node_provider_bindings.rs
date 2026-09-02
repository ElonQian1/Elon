use std::marker::PhantomData;

use crate::compute_federation::user_node_provider_binding::UserNodeProviderBindingReceiptV1;

use super::{
    node_compute_plugin_sharing::NodeComputePluginSharingDispatchIntent,
    node_credentials::CurrentNodeEndpointCredentialForUserNodeProviderBinding,
    ComputeProviderRegistrationReceipt, Store,
};

mod read;
mod reproof;
mod write;

pub(super) use reproof::{
    current_user_node_provider_binding_by_digest_on, current_user_node_provider_binding_on,
    require_user_node_provider_activation_binding_on,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UserNodeProviderBindingDisposition {
    Inserted,
    ExactReplay,
}

pub(crate) struct CommittedUserNodeProviderBinding {
    receipt: UserNodeProviderBindingReceiptV1,
    disposition: UserNodeProviderBindingDisposition,
}

impl CommittedUserNodeProviderBinding {
    pub(crate) fn receipt(&self) -> &UserNodeProviderBindingReceiptV1 {
        &self.receipt
    }

    pub(crate) fn disposition(&self) -> UserNodeProviderBindingDisposition {
        self.disposition
    }
}

pub(crate) struct UserNodeProviderBindingInspection {
    receipt: UserNodeProviderBindingReceiptV1,
    current: bool,
}

impl UserNodeProviderBindingInspection {
    pub(crate) fn receipt(&self) -> &UserNodeProviderBindingReceiptV1 {
        &self.receipt
    }

    pub(crate) fn current(&self) -> bool {
        self.current
    }

    pub(crate) fn current_blocker(&self) -> Option<&'static str> {
        (!self.current).then_some("current_binding_sources_changed")
    }
}

/// Current binding continuity only. This authority is not Ready, route, lease, or execution
/// authority and cannot outlive the transaction that proved its source roots.
pub(in crate::store) struct CurrentUserNodeProviderBindingAuthority<'tx, 'conn> {
    receipt: UserNodeProviderBindingReceiptV1,
    provider: ComputeProviderRegistrationReceipt,
    endpoint: CurrentNodeEndpointCredentialForUserNodeProviderBinding<'tx>,
    consent: NodeComputePluginSharingDispatchIntent,
    _transaction: PhantomData<&'tx rusqlite::Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentUserNodeProviderBindingAuthority<'tx, 'conn> {
    pub(in crate::store) fn receipt(&self) -> &UserNodeProviderBindingReceiptV1 {
        &self.receipt
    }

    pub(in crate::store) fn provider(&self) -> &ComputeProviderRegistrationReceipt {
        &self.provider
    }

    pub(in crate::store) fn endpoint(
        &self,
    ) -> &CurrentNodeEndpointCredentialForUserNodeProviderBinding<'tx> {
        &self.endpoint
    }

    pub(in crate::store) fn consent(&self) -> &NodeComputePluginSharingDispatchIntent {
        &self.consent
    }
}

impl Store {
    pub(crate) fn bind_user_node_provider(
        &self,
        owner_user_id: &str,
        node_id: &str,
        provider_id: &str,
        idempotency_key: &str,
        confirmation: &str,
    ) -> anyhow::Result<CommittedUserNodeProviderBinding> {
        write::bind(
            self,
            owner_user_id,
            node_id,
            provider_id,
            idempotency_key,
            confirmation,
        )
    }

    pub(crate) fn inspect_user_node_provider_binding_for_owner(
        &self,
        owner_user_id: &str,
        provider_id: &str,
    ) -> anyhow::Result<Option<UserNodeProviderBindingInspection>> {
        read::inspect_for_owner(self, owner_user_id, provider_id)
    }
}

pub(super) fn committed(
    receipt: UserNodeProviderBindingReceiptV1,
    disposition: UserNodeProviderBindingDisposition,
) -> CommittedUserNodeProviderBinding {
    CommittedUserNodeProviderBinding {
        receipt,
        disposition,
    }
}

pub(super) fn inspection(
    receipt: UserNodeProviderBindingReceiptV1,
    current: bool,
) -> UserNodeProviderBindingInspection {
    UserNodeProviderBindingInspection { receipt, current }
}

pub(super) fn current_authority<'tx, 'conn>(
    receipt: UserNodeProviderBindingReceiptV1,
    provider: ComputeProviderRegistrationReceipt,
    endpoint: CurrentNodeEndpointCredentialForUserNodeProviderBinding<'tx>,
    consent: NodeComputePluginSharingDispatchIntent,
) -> CurrentUserNodeProviderBindingAuthority<'tx, 'conn> {
    CurrentUserNodeProviderBindingAuthority {
        receipt,
        provider,
        endpoint,
        consent,
        _transaction: PhantomData,
    }
}
