use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use super::{
    read::{
        application_by_batch_on, batch_by_id_on, bindings_by_application_on, entries_by_batch_on,
        review_by_batch_on,
    },
    review::validate_exact,
    types::ComputePlatformReferencePriceCurveBatchDetailReceipt,
};
use crate::store::Store;

impl Store {
    pub(crate) fn platform_reference_price_curve_batch(
        &self,
        batch_id: &str,
    ) -> Result<ComputePlatformReferencePriceCurveBatchDetailReceipt> {
        validate_exact(batch_id, "reference price curve batch ID", 160)?;
        let connection = self.conn()?;
        detail_on(&connection, batch_id)?
            .ok_or_else(|| anyhow::anyhow!("platform reference price curve batch does not exist"))
    }

    pub(crate) fn list_platform_reference_price_curve_batches_for_admin(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputePlatformReferencePriceCurveBatchDetailReceipt>> {
        validate_status(status)?;
        let connection = self.conn()?;
        let limit = i64::try_from(limit.clamp(1, 100))?;
        let ids = if let Some(status) = status {
            batch_ids(
                &connection,
                "WHERE status=?1 ORDER BY submitted_at ASC, batch_id ASC LIMIT ?2",
                params![status, limit],
            )?
        } else {
            batch_ids(
                &connection,
                "ORDER BY submitted_at DESC, batch_id DESC LIMIT ?1",
                params![limit],
            )?
        };
        details_on(&connection, ids)
    }
}

fn detail_on(
    connection: &Connection,
    batch_id: &str,
) -> Result<Option<ComputePlatformReferencePriceCurveBatchDetailReceipt>> {
    let Some(batch) = batch_by_id_on(connection, batch_id)? else {
        return Ok(None);
    };
    let entries = entries_by_batch_on(connection, batch_id)?
        .into_iter()
        .map(|entry| entry.into_receipt())
        .collect();
    let review = review_by_batch_on(connection, batch_id)?.map(|stored| stored.into_receipt(false));
    let application = if let Some(stored) = application_by_batch_on(connection, batch_id)? {
        let application_id = stored.envelope.application_id.clone();
        let bindings = bindings_by_application_on(connection, &application_id)?
            .into_iter()
            .map(|binding| binding.into_receipt())
            .collect();
        Some(stored.into_receipt(bindings, false))
    } else {
        None
    };
    Ok(Some(ComputePlatformReferencePriceCurveBatchDetailReceipt {
        batch: batch.into_receipt(entries, false),
        review,
        application,
    }))
}

fn details_on(
    connection: &Connection,
    batch_ids: Vec<String>,
) -> Result<Vec<ComputePlatformReferencePriceCurveBatchDetailReceipt>> {
    batch_ids
        .into_iter()
        .map(|batch_id| {
            detail_on(connection, &batch_id)?.ok_or_else(|| {
                anyhow::anyhow!("listed platform reference price curve batch disappeared")
            })
        })
        .collect()
}

fn batch_ids<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Vec<String>> {
    let mut statement = connection.prepare(&format!(
        "SELECT batch_id FROM compute_platform_reference_price_curve_batches {filter}"
    ))?;
    let rows = statement.query_map(parameters, |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn validate_status(status: Option<&str>) -> Result<()> {
    if let Some(status) = status {
        if !matches!(
            status,
            "submitted" | "approved" | "changes_requested" | "rejected" | "applied"
        ) {
            bail!("platform reference price curve status filter is unsupported");
        }
    }
    Ok(())
}
