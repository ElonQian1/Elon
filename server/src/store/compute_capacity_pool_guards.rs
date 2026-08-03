use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::capacity::ComputeCapacityPoolBinding;

#[derive(Debug, Clone, Copy)]
pub(super) enum ComputeCapacityPoolOperation {
    ConfigureBucket,
    AddSupply,
    HoldClaim,
    WithdrawSupply,
}

pub(super) fn ensure_pool_operation_allowed_on(
    conn: &Connection,
    binding: &ComputeCapacityPoolBinding,
    operation: ComputeCapacityPoolOperation,
) -> Result<()> {
    let pool = conn
        .query_row(
            "SELECT status, current_capacity_epoch
               FROM compute_capacity_pools WHERE pool_id=?1",
            params![binding.pool_id.trim()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    let Some((status, current_capacity_epoch)) = pool else {
        bail!("容量池不存在");
    };
    if current_capacity_epoch != binding.capacity_epoch {
        bail!("容量写入只能使用容量池当前 epoch");
    }
    let allowed = match operation {
        ComputeCapacityPoolOperation::ConfigureBucket | ComputeCapacityPoolOperation::AddSupply => {
            matches!(status.as_str(), "registering" | "active")
        }
        ComputeCapacityPoolOperation::HoldClaim => status == "active",
        ComputeCapacityPoolOperation::WithdrawSupply => {
            matches!(status.as_str(), "active" | "draining" | "quarantined")
        }
    };
    if !allowed {
        bail!("容量池当前状态 {status} 不允许执行该容量操作");
    }
    Ok(())
}
