use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::{
    compute_federation::workload::ComputeOutputContract,
    store::{
        compute_attempt_activations::compute_attempt_activation_on,
        compute_attempt_leases::current_lease_state_on,
        compute_capacity_claim_rows::stored_claim_on,
        compute_job_registry::current_registered_job_on,
        compute_reservation_registry::current_registered_reservation_on, Store,
    },
};

use super::{
    ensure_active_bindings, ensure_live_running_lease_owner,
    support::{build_contract, latest_declaration_on, validate_exact},
};

const COMPUTE_ATTEMPT_USAGE_TEMPLATE_SCHEMA: &str = "compute_federation.attempt_usage_template.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptUsageTemplateLine {
    pub meter: String,
    pub reserved_quantity: i64,
    pub previous_cumulative_quantity: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptUsageTemplateSnapshot {
    pub snapshot_id: String,
    pub sequence_no: i64,
    pub cumulative_usage_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptUsageTemplateReceipt {
    pub schema: &'static str,
    pub lease_id: String,
    pub provider_id: String,
    pub lease_revision: i64,
    pub lease_digest: String,
    pub fencing_generation: i64,
    pub task_kind: String,
    pub output_contract: ComputeOutputContract,
    pub next_sequence_no: i64,
    pub meters: Vec<ComputeAttemptUsageTemplateLine>,
    pub latest_snapshot: Option<ComputeAttemptUsageTemplateSnapshot>,
    pub read_effect: &'static str,
}

impl Store {
    pub(crate) fn compute_attempt_usage_template(
        &self,
        provider_id: &str,
        user_id: &str,
        lease_id: &str,
    ) -> Result<ComputeAttemptUsageTemplateReceipt> {
        validate_exact("算力 Provider ID", provider_id, 200)?;
        validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        let current = current_lease_state_on(&conn, lease_id)?
            .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
        ensure_live_running_lease_owner(&conn, provider_id, user_id, &current)?;

        let activation = compute_attempt_activation_on(&conn, lease_id)?;
        let job = current_registered_job_on(&conn, &current.lease.job_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Job 不存在"))?;
        let reservation = current_registered_reservation_on(&conn, &current.lease.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Reservation 不存在"))?;
        let claim = stored_claim_on(&conn, &activation.active_claim.claim_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Capacity Claim 不存在"))?;
        ensure_active_bindings(&current, &activation, &job, &reservation, &claim)?;

        let contract = build_contract(&claim.lines)?;
        let latest = latest_declaration_on(&conn, lease_id)?
            .map(|stored| stored.into_receipt(false))
            .transpose()?;
        let next_sequence_no = match latest.as_ref() {
            Some(receipt) => receipt
                .sequence_no
                .checked_add(1)
                .ok_or_else(|| anyhow!("Attempt 用量声明序号溢出"))?,
            None => 1,
        };
        let meters = contract
            .into_iter()
            .map(|line| ComputeAttemptUsageTemplateLine {
                previous_cumulative_quantity: latest
                    .as_ref()
                    .and_then(|receipt| {
                        receipt
                            .cumulative_declared_usage
                            .iter()
                            .find(|reading| reading.meter == line.meter)
                    })
                    .map(|reading| reading.quantity)
                    .unwrap_or(0),
                meter: line.meter,
                reserved_quantity: line.reserved_quantity,
            })
            .collect();
        let latest_snapshot = latest.map(|receipt| ComputeAttemptUsageTemplateSnapshot {
            snapshot_id: receipt.snapshot_id,
            sequence_no: receipt.sequence_no,
            cumulative_usage_digest: receipt.cumulative_usage_digest,
        });

        Ok(ComputeAttemptUsageTemplateReceipt {
            schema: COMPUTE_ATTEMPT_USAGE_TEMPLATE_SCHEMA,
            lease_id: current.lease.lease_id,
            provider_id: current.provider_id,
            lease_revision: current.lease_revision,
            lease_digest: current.lease_digest,
            fencing_generation: current.lease.fencing_generation,
            task_kind: job.job.workload.task_kind,
            output_contract: job.job.workload.output,
            next_sequence_no,
            meters,
            latest_snapshot,
            read_effect: "none",
        })
    }
}
