use anyhow::{bail, Result};

use crate::store::{
    ComputePlatformSettlementAccountView, ComputeSettlementAccountView,
    ComputeSettlementWithdrawalQueuePage, Store,
};

pub(crate) fn get_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<ComputeSettlementAccountView> {
    let view = store.compute_settlement_account_view(provider_id)?;
    if view.owner_user_id != user_id {
        bail!("算力 Provider 结算账户不属于当前登录用户");
    }
    Ok(view)
}

pub(crate) fn list_withdrawal_queue_for_platform_admin(
    store: &Store,
    status: &str,
    limit: usize,
) -> Result<ComputeSettlementWithdrawalQueuePage> {
    store.list_compute_settlement_withdrawal_queue(status, limit)
}

pub(crate) fn get_for_platform_admin(
    store: &Store,
) -> Result<ComputePlatformSettlementAccountView> {
    store.compute_platform_settlement_account_view()
}
