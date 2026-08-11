use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use super::{
    read::{admission_by_request_on, request_by_id_on, review_by_request_on},
    review::validate_exact,
    types::ExternalPoolAdapterReleaseDetailReceipt,
};
use crate::store::Store;

impl Store {
    pub(crate) fn external_pool_adapter_release_request(
        &self,
        request_id: &str,
    ) -> Result<ExternalPoolAdapterReleaseDetailReceipt> {
        validate_exact(request_id, "Adapter release request ID", 160)?;
        let connection = self.conn()?;
        detail_on(&connection, request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release request does not exist"))
    }

    pub(crate) fn list_external_pool_adapter_release_requests_for_admin(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ExternalPoolAdapterReleaseDetailReceipt>> {
        validate_status(status)?;
        let connection = self.conn()?;
        let limit = i64::try_from(limit.clamp(1, 100))?;
        let ids = if let Some(status) = status {
            request_ids(
                &connection,
                "WHERE status=?1 ORDER BY submitted_at ASC, request_id ASC LIMIT ?2",
                params![status, limit],
            )?
        } else {
            request_ids(
                &connection,
                "ORDER BY submitted_at DESC, request_id DESC LIMIT ?1",
                params![limit],
            )?
        };
        details_on(&connection, ids)
    }
}

fn detail_on(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<ExternalPoolAdapterReleaseDetailReceipt>> {
    let Some(request) = request_by_id_on(connection, request_id)? else {
        return Ok(None);
    };
    let review =
        review_by_request_on(connection, request_id)?.map(|stored| stored.into_receipt(false));
    let admission =
        admission_by_request_on(connection, request_id)?.map(|stored| stored.into_receipt(false));
    Ok(Some(ExternalPoolAdapterReleaseDetailReceipt {
        request: request.into_receipt(false),
        review,
        admission,
    }))
}

fn details_on(
    connection: &Connection,
    request_ids: Vec<String>,
) -> Result<Vec<ExternalPoolAdapterReleaseDetailReceipt>> {
    request_ids
        .into_iter()
        .map(|request_id| {
            detail_on(connection, &request_id)?.ok_or_else(|| {
                anyhow::anyhow!("listed external-pool Adapter release request disappeared")
            })
        })
        .collect()
}

fn request_ids<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Vec<String>> {
    let mut statement = connection.prepare(&format!(
        "SELECT request_id FROM compute_external_pool_adapter_release_requests {filter}"
    ))?;
    let rows = statement.query_map(parameters, |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn validate_status(status: Option<&str>) -> Result<()> {
    if let Some(status) = status {
        if !matches!(
            status,
            "submitted" | "approved" | "changes_requested" | "rejected" | "staged"
        ) {
            bail!("external-pool Adapter release status filter is unsupported");
        }
    }
    Ok(())
}
