use anyhow::{bail, Result};
use rusqlite::{params, Transaction};

use crate::store::NodeComputePluginSharingDispatchIntent;

pub(super) fn sharing_ack_already_consumed(
    tx: &Transaction<'_>,
    sharing: &NodeComputePluginSharingDispatchIntent,
    preparation_id: Option<&str>,
) -> Result<bool> {
    let (event_count, exact_count, intent_count, outcome_count) = tx.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN preparation_id=?2 AND node_id=?3
                  AND consent_receipt_id=?4 AND policy_revision=?5 AND policy_digest=?6
                  THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN preparation_id=?2 AND node_id=?3
                  AND consent_receipt_id=?4 AND policy_revision=?5 AND policy_digest=?6
                  AND event_sequence=1 AND event_kind='intent_committed'
                  AND detail_code IS NULL THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN preparation_id=?2 AND node_id=?3
                  AND consent_receipt_id=?4 AND policy_revision=?5 AND policy_digest=?6
                  AND event_sequence=2 AND event_kind!='intent_committed'
                  AND ((event_kind='dispatched' AND detail_code IS NULL)
                    OR (event_kind!='dispatched' AND detail_code IS NOT NULL))
                  THEN 1 ELSE 0 END), 0)
           FROM node_compute_plugin_install_plan_preparation_delivery_events
          WHERE sharing_delivery_id=?1",
        params![
            sharing.delivery_id,
            preparation_id.unwrap_or(""),
            sharing.node_id,
            sharing.consent_receipt_id,
            sharing.policy_revision,
            sharing.policy_digest
        ],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        },
    )?;
    if event_count == 0 {
        return Ok(false);
    }
    if preparation_id.is_none()
        || !(1..=2).contains(&event_count)
        || exact_count != event_count
        || intent_count != 1
        || outcome_count != event_count - 1
    {
        bail!("算力插件 InstallPlan 准备 ACK 消费记录已损坏");
    }
    Ok(true)
}
