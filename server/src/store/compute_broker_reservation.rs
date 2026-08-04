use anyhow::Result;
use rusqlite::TransactionBehavior;
use serde::Serialize;

use crate::compute_federation::{
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeJobVersionBinding, ComputeReservedCapacity},
};

use super::Store;

mod finish;
mod finish_receipt;
mod finish_validation;
mod orchestrate;
mod receipt;
mod validation;

use finish::finish_new_broker_contract_on;
use finish_receipt::replay_broker_finish_on;
use finish_validation::normalize_broker_finish_request;
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeBrokerFinishAction {
    Release,
    Expire,
}

#[derive(Debug, Clone)]
pub(crate) struct FinishComputeBrokerRequest {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub action: ComputeBrokerFinishAction,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeBrokerFinishReceipt {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub action: ComputeBrokerFinishAction,
    pub budget_reservation_id: String,
    pub budget_refunded_fen: i64,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub terminal_job: ComputeJobVersionBinding,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub status: String,
    pub recorded_at: String,
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

    pub(crate) fn finish_compute_broker(
        &self,
        request: &FinishComputeBrokerRequest,
    ) -> Result<ComputeBrokerFinishReceipt> {
        let normalized = normalize_broker_finish_request(request)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(receipt) = replay_broker_finish_on(&tx, &normalized)? {
            tx.commit()?;
            return Ok(receipt);
        }
        let receipt = finish_new_broker_contract_on(&tx, &normalized)?;
        tx.commit()?;
        Ok(receipt)
    }
}
