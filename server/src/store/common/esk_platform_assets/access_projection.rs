use anyhow::Result;

use crate::{
    esk_asset::platform::{
        access::{AccessError, AccessScope},
        sellback::{parse_cursor, SellbackConfiguration, SellbackPage, MAX_PAGE_SIZE},
        DelegatedAssetBalance, DelegatedAssetIdentity, DelegatedAssetPage, DelegatedAssetProgress,
        DelegatedSellbackRow,
    },
    store::Store,
};

use super::{access::verify_read_on, sellback::scan_delegated_on};

impl Store {
    /// Authorization, parent-session validity and every ledger row are observed
    /// in one read transaction. Reading never renews or updates an authorization.
    pub(crate) fn asset_access_esk(
        &self,
        token: &str,
        client_id: &str,
        limit: usize,
        cursor: Option<&str>,
        include_progress: bool,
        config: &SellbackConfiguration,
    ) -> Result<DelegatedAssetPage> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let access = verify_read_on(&tx, token, client_id, "esk.summary.read")?;
        if !(1..=MAX_PAGE_SIZE).contains(&limit) || (!include_progress && cursor.is_some()) {
            return Err(AccessError::InvalidInput.into());
        }
        if include_progress && !access.scopes().contains(&AccessScope::EskProgressRead) {
            return Err(AccessError::InsufficientScope.into());
        }
        let cursor = cursor
            .map(parse_cursor)
            .transpose()
            .map_err(|_| AccessError::InvalidInput)?;
        let page = scan_delegated_on(&tx, &access, config, limit, cursor.as_ref())?;
        let response = project_page(
            access.subject(),
            access.client_id(),
            access.expires_at(),
            page,
            include_progress,
        );
        tx.commit()?;
        Ok(response)
    }
}

fn project_page(
    subject: &str,
    client_id: &str,
    expires_at: &str,
    page: SellbackPage,
    include_progress: bool,
) -> DelegatedAssetPage {
    DelegatedAssetPage {
        schema: "yilong.esk.delegated_asset_page.v1",
        subject: subject.to_owned(),
        client_id: client_id.to_owned(),
        expires_at: expires_at.to_owned(),
        asset: DelegatedAssetIdentity {
            asset_id: "esk",
            symbol: "ESK",
            decimals: 6,
            source: "platform_recorded",
            simulated: false,
            chain_status: "not_deployed",
            funds_moved: false,
        },
        balance: DelegatedAssetBalance {
            total_base_units: page.summary.total_base_units.to_string(),
            reserved_base_units: page.summary.reserved_base_units.to_string(),
            available_base_units: page.summary.available_base_units.to_string(),
        },
        snapshot_digest: page.summary.snapshot_digest,
        progress: include_progress.then(|| DelegatedAssetProgress {
            request_count: page.summary.request_count.to_string(),
            open_count: page.summary.open_request_count.to_string(),
            range_start: page.range_start.to_string(),
            range_end: page.range_end.to_string(),
            requests: page
                .requests
                .into_iter()
                .map(|request| DelegatedSellbackRow {
                    request_id: request.request_id,
                    amount_base_units: request.input.amount_base_units.to_string(),
                    status: if request.canceled_at.is_some() {
                        "canceled"
                    } else {
                        "submitted"
                    },
                    created_at: request.created_at,
                    canceled_at: request.canceled_at,
                })
                .collect(),
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        }),
    }
}
