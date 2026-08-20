//! Read-only external_pool source-stage eligibility. It never grants claim or send authority.

use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::super::compute_external_pool_adapter_runtime_bundle::ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority;

pub(super) fn eligible_external_pool_rows_on(
    connection: &Connection,
    checked_at: &str,
) -> Result<usize> {
    let count: i64 = connection.query_row(
        "SELECT count(*)
           FROM compute_attempt_start_outbox outbox
           JOIN compute_providers provider ON provider.provider_id=outbox.provider_id
          WHERE provider.provider_kind='external_pool' AND provider.status='active'
            AND outbox.operation_kind IN ('prepare','commit')
            AND outbox.state='pending' AND outbox.next_attempt_at<=?1
            AND outbox.not_before<=?1 AND ?1<outbox.not_after
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts send
                 WHERE send.outbox_id=outbox.outbox_id)
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_no_start_proofs proof
                 WHERE proof.command_id=outbox.command_id)",
        params![checked_at],
        |row| row.get(0),
    )?;
    ensure!(count >= 0, "V278 eligible row count is negative");
    Ok(usize::try_from(count)?)
}

pub(super) fn next_unadmitted_external_pool_source_provider_on(
    connection: &Connection,
    checked_at: &str,
) -> Result<Option<String>> {
    Ok(connection
        .query_row(
            "SELECT outbox.provider_id
               FROM compute_attempt_start_outbox outbox
               JOIN compute_providers provider ON provider.provider_id=outbox.provider_id
              WHERE provider.provider_kind='external_pool' AND provider.status='active'
                AND outbox.operation_kind IN ('prepare','commit')
                AND outbox.state='pending' AND outbox.next_attempt_at<=?1
                AND outbox.not_before<=?1 AND ?1<outbox.not_after
              ORDER BY outbox.next_attempt_at,outbox.outbox_id LIMIT 1",
            params![checked_at],
            |row| row.get(0),
        )
        .optional()?)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) fn next_eligible_external_pool_provider_on<
    'authority,
    'tx,
    'conn,
    'runtime,
>(
    connection: &'tx rusqlite::Transaction<'conn>,
    authority: &ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<
        'authority,
        'tx,
        'conn,
        'runtime,
    >,
) -> Result<Option<String>> {
    let provider_id = &authority
        .route_authorization()
        .envelope()
        .authorization
        .provider
        .provider_id;
    Ok(connection
        .query_row(
            "SELECT outbox.provider_id
               FROM compute_attempt_start_outbox outbox
               JOIN compute_providers provider ON provider.provider_id=outbox.provider_id
              WHERE outbox.provider_id=?2 AND provider.provider_kind='external_pool'
                AND provider.status='active'
                AND outbox.operation_kind IN ('prepare','commit')
                AND outbox.state='pending' AND outbox.next_attempt_at<=?1
                AND outbox.not_before<=?1 AND ?1<outbox.not_after
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_send_attempts send
                     WHERE send.outbox_id=outbox.outbox_id)
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_no_start_proofs proof
                     WHERE proof.command_id=outbox.command_id)
              ORDER BY outbox.next_attempt_at,outbox.outbox_id LIMIT 1",
            params![authority.checked_at(), provider_id],
            |row| row.get(0),
        )
        .optional()?)
}
