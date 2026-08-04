use anyhow::Result;
use rusqlite::TransactionBehavior;
use serde::Serialize;

use crate::compute_federation::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeJobVersionBinding, ComputeReservedCapacity},
};

use super::Store;

mod orchestrate;
mod receipt;
mod validation;

use orchestrate::reserve_new_broker_contract_on;
use receipt::replay_broker_reserve_on;
use validation::normalize_broker_reserve_request;

pub(super) const BROKER_BUDGET_ADAPTER: &str = "platform_balance_cny";
pub(super) const BROKER_BILLING_FEATURE: &str = "compute_federation_reservation";
pub(super) const BROKER_BILLING_USAGE_MODE: &str = "platform_balance_cny";

#[derive(Debug, Clone)]
pub(crate) struct ReserveComputeBrokerRequest {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub job_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub reserved_capacity: Vec<ComputeReservedCapacity>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeBrokerReservationReceipt {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub budget_adapter: String,
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub reserved_job: ComputeJobVersionBinding,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub status: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn reserve_compute_broker(
        &self,
        request: &ReserveComputeBrokerRequest,
    ) -> Result<ComputeBrokerReservationReceipt> {
        let normalized = normalize_broker_reserve_request(request)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay_broker_reserve_on(&tx, &normalized)? {
            tx.commit()?;
            return Ok(receipt);
        }
        let receipt = reserve_new_broker_contract_on(&tx, &normalized)?;
        tx.commit()?;
        Ok(receipt)
    }
}
