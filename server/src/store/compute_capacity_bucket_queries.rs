use anyhow::{anyhow, bail, Result};

use crate::compute_federation::capacity::ComputeCapacityBucketBalance;

use super::{
    compute_capacity_rows::{stored_bucket_on, stored_buckets_for_pool_epoch_limited_on},
    Store,
};

#[derive(Debug, Clone)]
pub(crate) struct ComputeCapacityBucketRead {
    pub balance: ComputeCapacityBucketBalance,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
}

impl Store {
    pub(crate) fn compute_capacity_bucket(
        &self,
        bucket_id: &str,
    ) -> Result<ComputeCapacityBucketRead> {
        self.compute_capacity_bucket_if_exists(bucket_id)?
            .ok_or_else(|| anyhow!("容量 bucket 不存在"))
    }

    pub(crate) fn compute_capacity_bucket_if_exists(
        &self,
        bucket_id: &str,
    ) -> Result<Option<ComputeCapacityBucketRead>> {
        if bucket_id.trim().is_empty() {
            bail!("容量 bucket ID 不能为空");
        }
        Ok(
            stored_bucket_on(&self.conn()?, bucket_id.trim())?.map(|stored| {
                ComputeCapacityBucketRead {
                    balance: stored.balance,
                    starts_at_utc: stored.starts_at,
                    ends_at_utc: stored.ends_at,
                }
            }),
        )
    }

    pub(crate) fn list_compute_capacity_buckets_for_pool(
        &self,
        pool_id: &str,
        capacity_epoch: i64,
        pool_revision: i64,
        limit: usize,
    ) -> Result<Vec<ComputeCapacityBucketRead>> {
        if pool_id.trim().is_empty() || capacity_epoch <= 0 || pool_revision <= 0 {
            bail!("容量池身份、epoch 或 revision 无效");
        }
        Ok(stored_buckets_for_pool_epoch_limited_on(
            &self.conn()?,
            pool_id.trim(),
            capacity_epoch,
            pool_revision,
            limit,
        )?
        .into_iter()
        .map(|stored| ComputeCapacityBucketRead {
            balance: stored.balance,
            starts_at_utc: stored.starts_at,
            ends_at_utc: stored.ends_at,
        })
        .collect())
    }
}
