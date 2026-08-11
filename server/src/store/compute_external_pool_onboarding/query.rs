use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use super::{
    read::{
        application_by_request_on, application_receipt, request_by_id_on, request_receipt,
        review_by_request_on, review_receipt,
    },
    review::validate_exact,
    types::ExternalPoolOnboardingDetailReceipt,
};
use crate::store::Store;

impl Store {
    pub(crate) fn external_pool_onboarding_request(
        &self,
        request_id: &str,
    ) -> Result<ExternalPoolOnboardingDetailReceipt> {
        validate_exact(request_id, "onboarding request ID", 160)?;
        let connection = self.conn()?;
        detail_on(&connection, request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding request does not exist"))
    }

    pub(crate) fn list_external_pool_onboarding_requests_for_owner(
        &self,
        owner_user_id: &str,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ExternalPoolOnboardingDetailReceipt>> {
        validate_exact(owner_user_id, "onboarding owner", 160)?;
        validate_status(status)?;
        let connection = self.conn()?;
        let limit = i64::try_from(limit.clamp(1, 100))?;
        let ids = if let Some(status) = status {
            request_ids(
                &connection,
                "WHERE provider_owner_account_id=?1 AND status=?2
                 ORDER BY requested_at DESC, request_id DESC LIMIT ?3",
                params![owner_user_id, status, limit],
            )?
        } else {
            request_ids(
                &connection,
                "WHERE provider_owner_account_id=?1
                 ORDER BY requested_at DESC, request_id DESC LIMIT ?2",
                params![owner_user_id, limit],
            )?
        };
        details_on(&connection, ids)
    }

    pub(crate) fn list_external_pool_onboarding_requests_for_admin(
        &self,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ExternalPoolOnboardingDetailReceipt>> {
        validate_status(status)?;
        let connection = self.conn()?;
        let limit = i64::try_from(limit.clamp(1, 100))?;
        let ids = if let Some(status) = status {
            request_ids(
                &connection,
                "WHERE status=?1 ORDER BY requested_at ASC, request_id ASC LIMIT ?2",
                params![status, limit],
            )?
        } else {
            request_ids(
                &connection,
                "ORDER BY requested_at DESC, request_id DESC LIMIT ?1",
                params![limit],
            )?
        };
        details_on(&connection, ids)
    }
}

fn detail_on(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<ExternalPoolOnboardingDetailReceipt>> {
    let Some(request) = request_by_id_on(connection, request_id)? else {
        return Ok(None);
    };
    let review =
        review_by_request_on(connection, request_id)?.map(|stored| review_receipt(stored, false));
    let application = application_by_request_on(connection, request_id)?
        .map(|stored| application_receipt(stored, false));
    Ok(Some(ExternalPoolOnboardingDetailReceipt {
        request: request_receipt(request, false),
        review,
        application,
    }))
}

fn details_on(
    connection: &Connection,
    request_ids: Vec<String>,
) -> Result<Vec<ExternalPoolOnboardingDetailReceipt>> {
    request_ids
        .into_iter()
        .map(|request_id| {
            detail_on(connection, &request_id)?.ok_or_else(|| {
                anyhow::anyhow!("listed external-pool onboarding request disappeared")
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
        "SELECT request_id FROM compute_external_pool_onboarding_requests {filter}"
    ))?;
    let rows = statement.query_map(parameters, |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn validate_status(status: Option<&str>) -> Result<()> {
    if let Some(status) = status {
        if !matches!(
            status,
            "submitted" | "approved" | "changes_requested" | "rejected" | "canceled" | "applied"
        ) {
            bail!("external-pool onboarding status filter is unsupported");
        }
    }
    Ok(())
}
