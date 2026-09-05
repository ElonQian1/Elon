use anyhow::Result;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::esk_asset::platform::{sellback::*, PlatformHistoryPage, PlatformPolicy};

use super::super::{access::AuthorizedAssetRead, history::scan_history_on};

use super::{add, platform_error, policy_on, records::visit_on, scan_authenticated_history_on};

pub(super) enum Selection<'a> {
    Page(usize, Option<&'a SellbackCursor>),
    Id(&'a str),
    Key(&'a str),
}

pub(super) struct Snapshot {
    pub page: SellbackPage,
    pub selected: Option<SellbackRecord>,
    pub formal: Option<PlatformPolicy>,
}

impl Snapshot {
    pub fn result(self, replayed: bool) -> Result<SellbackResult> {
        Ok(SellbackResult {
            request: self.selected.ok_or(SellbackError::NotFound)?,
            summary: self.page.summary,
            replayed,
        })
    }
}

/// Reuses the authenticated full formal scan on the caller's connection/transaction.
/// All own requests, including off-page and canceled records, affect the digest.
pub(super) fn scan_on(
    conn: &Connection,
    user: &str,
    token: &str,
    config: &SellbackConfiguration,
    selection: Selection<'_>,
) -> Result<Snapshot> {
    let formal_page =
        scan_authenticated_history_on(conn, user, token, 1, None).map_err(platform_error)?;
    scan_core_on(conn, user, config, selection, formal_page)
}

/// The capability can only come from the delegated verifier on the caller's
/// SQLite snapshot. No public bare-user read path is added.
pub(in super::super) fn scan_delegated_on(
    conn: &Connection,
    access: &AuthorizedAssetRead,
    config: &SellbackConfiguration,
    limit: usize,
    cursor: Option<&SellbackCursor>,
) -> Result<SellbackPage> {
    let formal_page = scan_history_on(conn, access.user_id(), 1, None).map_err(platform_error)?;
    Ok(scan_core_on(
        conn,
        access.user_id(),
        config,
        Selection::Page(limit, cursor),
        formal_page,
    )?
    .page)
}

fn scan_core_on(
    conn: &Connection,
    user: &str,
    config: &SellbackConfiguration,
    selection: Selection<'_>,
    formal_page: PlatformHistoryPage,
) -> Result<Snapshot> {
    let formal = policy_on(conn).map_err(platform_error)?;
    let mut hash = Fingerprint::new(user, &formal_page.snapshot_digest, config);
    let mut page = SellbackPage {
        summary: SellbackSummary {
            snapshot_digest: String::new(),
            total_base_units: formal_page.total_base_units,
            reserved_base_units: 0,
            available_base_units: 0,
            open_request_count: 0,
            request_count: 0,
            availability: availability(
                config,
                user,
                formal.as_ref().map(|p| p.source_fingerprint.as_str()),
            ),
        },
        requests: Vec::new(),
        range_start: 0,
        range_end: 0,
        has_more: false,
        next_cursor: None,
    };
    let mut selected = None;
    let mut anchor = None;
    visit_on(conn, Some(user), formal.as_ref(), |record| {
        page.summary.request_count = add(page.summary.request_count, 1)?;
        if record.canceled_at.is_none() {
            page.summary.reserved_base_units = add(
                page.summary.reserved_base_units,
                record.input.amount_base_units,
            )?;
            page.summary.open_request_count = add(page.summary.open_request_count, 1)?;
        }
        hash.record(&record);
        match &selection {
            Selection::Page(limit, cursor) => {
                if (cursor.is_none() || anchor.is_some()) && page.requests.len() < *limit {
                    if page.requests.is_empty() {
                        page.range_start = page.summary.request_count;
                    }
                    page.range_end = page.summary.request_count;
                    page.requests.push(record.clone());
                }
                if cursor.is_some_and(|value| value.after_request_id == record.request_id) {
                    anchor = Some(page.summary.request_count);
                }
            }
            Selection::Id(id) if record.request_id == *id => selected = Some(record),
            Selection::Key(key) if record.input.idempotency_key == *key => selected = Some(record),
            _ => {}
        }
        Ok(())
    })?;
    page.summary.available_base_units = page
        .summary
        .total_base_units
        .checked_sub(page.summary.reserved_base_units)
        .filter(|value| *value >= 0)
        .ok_or(SellbackError::Corrupt)?;
    page.summary.snapshot_digest = hash.finish();
    if let Selection::Page(_, cursor) = selection {
        if let Some(cursor) = cursor {
            if cursor.snapshot_digest != page.summary.snapshot_digest
                || anchor.is_none()
                || anchor == Some(page.summary.request_count)
            {
                return Err(SellbackError::SnapshotChanged.into());
            }
        }
        page.has_more = page.range_end < page.summary.request_count;
        if page.has_more {
            let last = page.requests.last().ok_or(SellbackError::Corrupt)?;
            page.next_cursor = Some(format!(
                "esbr1.{}.{}",
                page.summary.snapshot_digest, last.request_id
            ));
        }
    }
    Ok(Snapshot {
        page,
        selected,
        formal,
    })
}

/// Length-prefixed UTF-8, fixed v1 domain, authenticated user, formal snapshot,
/// current configuration identity, then ordered requests and cancellations.
struct Fingerprint(Sha256);
impl Fingerprint {
    fn new(user: &str, formal: &str, config: &SellbackConfiguration) -> Self {
        let mut hash = Self(Sha256::new());
        hash.text("yilong.esk.platform_sellback.snapshot.v1");
        hash.text(user);
        hash.text(formal);
        match config {
            SellbackConfiguration::Disabled => hash.text("disabled"),
            SellbackConfiguration::Enabled(policy) if validate_policy_integrity(policy).is_ok() => {
                hash.text("approved-policy");
                hash.text(&policy.policy_digest);
            }
            _ => hash.text("configuration-invalid"),
        }
        hash
    }
    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value.as_bytes());
    }
    fn record(&mut self, record: &SellbackRecord) {
        self.text(&record.request_id);
        self.text(&record.request_digest);
        self.text(&record.policy.policy_digest);
        self.text(&record.created_at);
        match (&record.cancel_event_id, &record.canceled_at) {
            (Some(id), Some(at)) => {
                self.text("canceled");
                self.text(id);
                self.text(at);
            }
            _ => self.text("submitted"),
        }
    }
    fn finish(self) -> String {
        format!("{:x}", self.0.finalize())
    }
}
