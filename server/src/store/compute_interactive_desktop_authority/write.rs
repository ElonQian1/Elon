use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::compute_federation::interactive_desktop::{
    authority_record::InteractiveDesktopAuthorityRecord,
    session::{InteractiveDesktopAction, InteractiveDesktopSessionState},
};

use super::{
    committed,
    read::{ensure_expected_head, head_on, version_on, StoredAuthorityHead},
    sources::require_same_owner_sources_on,
    InteractiveDesktopAuthorityCommitDisposition, InteractiveDesktopAuthorityHeadExpectation,
};
use crate::store::{node_credentials::NodeEndpointSessionPermit, Store};

pub(super) fn commit(
    store: &Store,
    record: &InteractiveDesktopAuthorityRecord,
    expected_head: Option<&InteractiveDesktopAuthorityHeadExpectation>,
    host_endpoint_session: &NodeEndpointSessionPermit,
    consumer_bearer_token: &str,
    observed_viewer_device_key_digest: &str,
    observed_viewer_transport_identity_digest: &str,
) -> Result<super::CommittedInteractiveDesktopAuthority> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let observed_at = Utc::now();

    if let Some(existing) = version_on(
        &transaction,
        &record.session.session_id,
        record.session.session_revision,
    )? {
        let head = head_on(&transaction, &record.session.session_id)?
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_MISSING"))?;
        ensure_exact_replay(record, &existing, &head, expected_head)?;
        if record.session.state == InteractiveDesktopSessionState::Active {
            verify_sources_and_structure(
                &transaction,
                record,
                host_endpoint_session,
                consumer_bearer_token,
                observed_viewer_device_key_digest,
                observed_viewer_transport_identity_digest,
                observed_at,
            )?;
        } else {
            verify_non_authorizing_replay(&transaction, record, observed_at.timestamp_millis())?;
        }
        let current = expectation(record);
        transaction.commit()?;
        return Ok(committed(
            current,
            InteractiveDesktopAuthorityCommitDisposition::ExactCurrentReplay,
        ));
    }

    let previous = head_on(&transaction, &record.session.session_id)?;
    validate_successor(&transaction, record, previous.as_ref(), expected_head)?;
    if record.session.state == InteractiveDesktopSessionState::Active {
        verify_sources_and_structure(
            &transaction,
            record,
            host_endpoint_session,
            consumer_bearer_token,
            observed_viewer_device_key_digest,
            observed_viewer_transport_identity_digest,
            observed_at,
        )?;
    } else {
        let head = previous
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_ACTIVE_ROOT_REQUIRED"))?;
        let prior = version_on(&transaction, &head.session_id, head.session_revision)?
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_PREDECESSOR_MISSING"))?;
        verify_non_authorizing_successor(record, &prior.record, observed_at.timestamp_millis())?;
    }
    let recorded_at_ms = match previous.as_ref() {
        Some(head) => observed_at.timestamp_millis().max(
            head.updated_at_ms
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_TIME_EXHAUSTED"))?,
        ),
        None => observed_at.timestamp_millis().max(0),
    };
    let (record_json, record_digest) = record.canonical_json_and_digest()?;
    if record_digest != record.record_digest {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_RECORD_DIGEST_MISMATCH");
    }
    insert_version(&transaction, record, &record_json, recorded_at_ms)?;
    move_head(&transaction, record, previous.as_ref(), recorded_at_ms)?;
    let stored = version_on(
        &transaction,
        &record.session.session_id,
        record.session.session_revision,
    )?
    .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_VERSION_READBACK_MISSING"))?;
    let head = head_on(&transaction, &record.session.session_id)?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_READBACK_MISSING"))?;
    ensure_stored_exact(record, &stored, &head, recorded_at_ms)?;
    let current = expectation(record);
    transaction.commit()?;
    Ok(committed(
        current,
        InteractiveDesktopAuthorityCommitDisposition::Inserted,
    ))
}

fn verify_sources_and_structure(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    host_endpoint_session: &NodeEndpointSessionPermit,
    consumer_bearer_token: &str,
    observed_viewer_device_key_digest: &str,
    observed_viewer_transport_identity_digest: &str,
    observed_at: chrono::DateTime<Utc>,
) -> Result<()> {
    let sources = require_same_owner_sources_on(
        transaction,
        record,
        host_endpoint_session,
        consumer_bearer_token,
        observed_at,
    )?;
    record.verify_canonical_and_structure(
        &sources.account_id,
        &sources.account_session_digest,
        sources.account_auth_epoch,
        observed_viewer_device_key_digest,
        observed_viewer_transport_identity_digest,
        InteractiveDesktopAction::ViewVideo,
        observed_at.timestamp_millis(),
    )
}

fn verify_non_authorizing_replay(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    now_ms: i64,
) -> Result<()> {
    if record.session.session_revision <= 1 {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_ACTIVE_ROOT_REQUIRED");
    }
    let prior = version_on(
        transaction,
        &record.session.session_id,
        record.session.session_revision - 1,
    )?
    .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_PREDECESSOR_MISSING"))?;
    verify_non_authorizing_successor(record, &prior.record, now_ms)
}

/// A non-active revision never authorizes media or input. It may close a live head after an
/// external source expires, but it cannot replace any frozen authority object while doing so.
fn verify_non_authorizing_successor(
    record: &InteractiveDesktopAuthorityRecord,
    prior: &InteractiveDesktopAuthorityRecord,
    now_ms: i64,
) -> Result<()> {
    record.verify_canonical_digests()?;
    if record.request != prior.request
        || record.profile != prior.profile
        || record.reservation != prior.reservation
        || record.host_lease != prior.host_lease
        || record.viewer_grant != prior.viewer_grant
        || record.media_epoch != prior.media_epoch
        || record.control_epoch != prior.control_epoch
    {
        bail!("INTERACTIVE_DESKTOP_NON_AUTHORIZING_REVISION_MUTATED_AUTHORITY");
    }

    let mut expected_session = prior.session.clone();
    expected_session.session_revision = record.session.session_revision;
    expected_session.session_digest = record.session.session_digest.clone();
    expected_session.state = record.session.state;
    expected_session.updated_at_ms = record.session.updated_at_ms;
    expected_session.terminal_reason_code = record.session.terminal_reason_code.clone();
    if record.session != expected_session
        || record.session.updated_at_ms <= prior.session.updated_at_ms
        || record.session.updated_at_ms > now_ms
    {
        bail!("INTERACTIVE_DESKTOP_NON_AUTHORIZING_SESSION_MISMATCH");
    }

    if record.session.state.is_terminal() && record.session.terminal_reason_code.is_none() {
        bail!("INTERACTIVE_DESKTOP_TERMINAL_REASON_REQUIRED");
    }
    if let Some(reason) = record.session.terminal_reason_code.as_deref() {
        if reason.is_empty()
            || reason.len() > 128
            || !reason
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            bail!("INTERACTIVE_DESKTOP_TERMINAL_REASON_INVALID");
        }
    }
    if !record.session.state.is_terminal()
        && record.session.state != InteractiveDesktopSessionState::Ending
        && record.session.terminal_reason_code.is_some()
    {
        bail!("INTERACTIVE_DESKTOP_NON_TERMINAL_REASON_FORBIDDEN");
    }
    Ok(())
}

fn validate_successor(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    previous: Option<&StoredAuthorityHead>,
    expected: Option<&InteractiveDesktopAuthorityHeadExpectation>,
) -> Result<()> {
    match (previous, expected) {
        (None, None)
            if record.session.session_revision == 1
                && record.session.state == InteractiveDesktopSessionState::Active =>
        {
            Ok(())
        }
        (Some(head), Some(expected)) => {
            ensure_expected_head(head, expected)?;
            if head.is_terminal
                || record.session.session_root_digest != head.session_root_digest
                || record.session.session_revision != head.session_revision + 1
            {
                bail!("INTERACTIVE_DESKTOP_AUTHORITY_SUCCESSOR_MISMATCH");
            }
            let prior = version_on(transaction, &head.session_id, head.session_revision)?
                .ok_or_else(|| {
                    anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_PREDECESSOR_MISSING")
                })?;
            let prior_state = prior.record.session.state;
            if !(prior_state == record.session.state
                || prior_state.allows_transition(record.session.state))
                || prior.record.session.created_at_ms != record.session.created_at_ms
            {
                bail!("INTERACTIVE_DESKTOP_AUTHORITY_STATE_TRANSITION_REJECTED");
            }
            Ok(())
        }
        _ => bail!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_EXPECTATION_REQUIRED"),
    }
}

fn ensure_exact_replay(
    record: &InteractiveDesktopAuthorityRecord,
    stored: &super::read::StoredAuthorityVersion,
    head: &StoredAuthorityHead,
    expected: Option<&InteractiveDesktopAuthorityHeadExpectation>,
) -> Result<()> {
    let (json, digest) = record.canonical_json_and_digest()?;
    if stored.record != *record
        || stored.record_json != json
        || stored.record_digest != digest
        || head.session_revision != record.session.session_revision
        || head.session_digest != record.session.session_digest
        || head.record_digest != record.record_digest
    {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_HISTORICAL_OR_CONFLICTING_REPLAY");
    }
    if let Some(expected) = expected {
        ensure_expected_head(head, expected)?;
    }
    Ok(())
}

fn insert_version(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    record_json: &str,
    recorded_at_ms: i64,
) -> Result<()> {
    let session = &record.session;
    let binding = &session.binding;
    transaction.execute(
        "INSERT INTO compute_interactive_desktop_authority_versions (
            authority_record_schema,authority_record_digest,authority_record_json,
            canonicalization,digest_algorithm,session_id,session_root_digest,session_revision,
            session_digest,session_state,is_terminal,session_reservation_id,
            session_reservation_revision,session_reservation_digest,binding_digest,provider_id,
            provider_policy_revision,provider_digest,provider_owner_account_id,consumer_account_id,
            host_lease_id,host_lease_digest,fencing_generation,viewer_grant_id,
            viewer_grant_digest,viewer_grant_generation,media_epoch_id,media_epoch_digest,
            media_epoch_sequence,control_epoch_id,control_epoch_digest,control_epoch_sequence,
            selected_surface_digest,viewer_transport_identity_digest,recorded_at_ms
         ) VALUES (
            ?1,?2,?3,'rfc8785_jcs','sha256',?4,?5,?6,?7,?8,?9,?10,?11,?12,
            ?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,
            ?28,?29,?30,?31,?32,?33
         )",
        params![
            record.schema,
            record.record_digest,
            record_json,
            session.session_id,
            session.session_root_digest,
            session.session_revision,
            session.session_digest,
            super::session_state_name(session.state),
            i64::from(session.state.is_terminal()),
            session.session_reservation.session_reservation_id,
            session.session_reservation.session_reservation_revision,
            session.session_reservation.session_reservation_digest,
            binding.binding_digest,
            binding.provider_id,
            i64::try_from(binding.provider_policy_revision)?,
            binding.provider_digest,
            binding.provider_owner_account_id,
            binding.consumer_account_id,
            record.host_lease.host_lease_id,
            record.host_lease.host_lease_digest,
            i64::try_from(record.host_lease.fencing_generation)?,
            record.viewer_grant.viewer_grant_id,
            record.viewer_grant.viewer_grant_digest,
            i64::try_from(record.viewer_grant.grant_generation)?,
            record.media_epoch.media_epoch_id,
            record.media_epoch.media_epoch_digest,
            i64::try_from(record.media_epoch.epoch_sequence)?,
            record.control_epoch.control_epoch_id,
            record.control_epoch.control_epoch_digest,
            i64::try_from(record.control_epoch.epoch_sequence)?,
            record.host_lease.selected_surface.selection_digest,
            record.viewer_grant.viewer_transport_identity_digest,
            recorded_at_ms,
        ],
    )?;
    Ok(())
}

fn move_head(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    previous: Option<&StoredAuthorityHead>,
    recorded_at_ms: i64,
) -> Result<()> {
    let session = &record.session;
    let terminal = i64::from(session.state.is_terminal());
    match previous {
        None => {
            transaction.execute(
                "INSERT INTO compute_interactive_desktop_authority_heads (
                    session_id,session_root_digest,current_session_revision,
                    current_session_digest,current_authority_record_digest,session_state,
                    is_terminal,created_at_ms,updated_at_ms
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
                params![
                    session.session_id,
                    session.session_root_digest,
                    session.session_revision,
                    session.session_digest,
                    record.record_digest,
                    super::session_state_name(session.state),
                    terminal,
                    recorded_at_ms,
                ],
            )?;
        }
        Some(previous) => {
            let changed = transaction.execute(
                "UPDATE compute_interactive_desktop_authority_heads SET
                    current_session_revision=?1,current_session_digest=?2,
                    current_authority_record_digest=?3,session_state=?4,is_terminal=?5,
                    updated_at_ms=?6
                  WHERE session_id=?7 AND current_session_revision=?8
                    AND current_session_digest=?9 AND current_authority_record_digest=?10",
                params![
                    session.session_revision,
                    session.session_digest,
                    record.record_digest,
                    super::session_state_name(session.state),
                    terminal,
                    recorded_at_ms,
                    previous.session_id,
                    previous.session_revision,
                    previous.session_digest,
                    previous.record_digest,
                ],
            )?;
            if changed != 1 {
                bail!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_CAS_MISMATCH");
            }
        }
    }
    Ok(())
}

fn ensure_stored_exact(
    record: &InteractiveDesktopAuthorityRecord,
    stored: &super::read::StoredAuthorityVersion,
    head: &StoredAuthorityHead,
    recorded_at_ms: i64,
) -> Result<()> {
    if stored.record != *record
        || stored.recorded_at_ms != recorded_at_ms
        || head.session_revision != record.session.session_revision
        || head.session_digest != record.session.session_digest
        || head.record_digest != record.record_digest
        || head.updated_at_ms != recorded_at_ms
    {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_COMMIT_READBACK_MISMATCH");
    }
    Ok(())
}

fn expectation(
    record: &InteractiveDesktopAuthorityRecord,
) -> InteractiveDesktopAuthorityHeadExpectation {
    InteractiveDesktopAuthorityHeadExpectation::new(
        record.session.session_id.clone(),
        record.session.session_revision,
        record.session.session_digest.clone(),
        record.record_digest.clone(),
    )
}
