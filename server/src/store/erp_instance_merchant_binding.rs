use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, ErrorCode};

use super::{now, Store};
use crate::erp_blueprint::model::ErpInstance;

impl Store {
    pub(crate) fn update_erp_instance_open_commerce_merchant(
        &self,
        instance_id: &str,
        expected_revision: i64,
        merchant_id: Option<&str>,
    ) -> Result<ErpInstance> {
        let updated = self
            .conn()?
            .execute(
                "UPDATE erp_instances
                    SET open_commerce_merchant_id=?1,
                        configuration_revision=configuration_revision+1,
                        updated_at=?2
                  WHERE id=?3 AND status='active' AND configuration_revision=?4",
                params![merchant_id, now(), instance_id.trim(), expected_revision],
            )
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(ref details, _)
                    if details.code == ErrorCode::ConstraintViolation =>
                {
                    anyhow!(error).context("该开放商业商户节点已经归属于其他 ERP 实例")
                }
                _ => anyhow!(error),
            })?;
        if updated == 0 {
            bail!("实例配置已变化或实例已归档，请刷新后重试");
        }
        self.erp_instance(instance_id)
    }
}
