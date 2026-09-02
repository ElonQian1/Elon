use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row, Transaction};

use crate::compute_federation::interactive_desktop::{
    authority_record::InteractiveDesktopAuthorityRecord, session::InteractiveDesktopAction,
};

use super::{
    current_authority, sources::require_same_owner_sources_on, CurrentInteractiveDesktopAuthority,
    InteractiveDesktopAuthorityHeadExpectation,
};
use crate::store::node_credentials::NodeEndpointSessionPermit;

pub(super) struct StoredAuthorityVersion {
    pub record: InteractiveDesktopAuthorityRecord,
    pub record_json: String,
    pub record_digest: String,
    pub recorded_at_ms: i64,
}

pub(super) struct StoredAuthorityHead {
    pub session_id: String,
    pub session_root_digest: String,
    pub session_revision: i64,
    pub session_digest: String,
    pub record_digest: String,
    pub session_state: String,
    pub is_terminal: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub(super) fn version_on(
    transaction: &Transaction<'_>,
    session_id: &str,
    session_revision: i64,
) -> Result<Option<StoredAuthorityVersion>> {
    transaction
        .query_row(
            VERSION_SELECT,
            params![session_id, session_revision],
            map_version,
        )
        .optional()?
        .map(audit_version)
        .transpose()
}

pub(super) fn head_on(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<Option<StoredAuthorityHead>> {
    transaction
        .query_row(
            "SELECT session_id,session_root_digest,current_session_revision,
                    current_session_digest,current_authority_record_digest,session_state,
                    is_terminal,created_at_ms,updated_at_ms
               FROM compute_interactive_desktop_authority_heads WHERE session_id=?1",
            params![session_id],
            |row| {
                Ok(StoredAuthorityHead {
                    session_id: row.get(0)?,
                    session_root_digest: row.get(1)?,
                    session_revision: row.get(2)?,
                    session_digest: row.get(3)?,
                    record_digest: row.get(4)?,
                    session_state: row.get(5)?,
                    is_terminal: row.get::<_, i64>(6)? == 1,
                    created_at_ms: row.get(7)?,
                    updated_at_ms: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub(super) fn require_current_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    expected: &InteractiveDesktopAuthorityHeadExpectation,
    host_endpoint_session: &NodeEndpointSessionPermit,
    consumer_bearer_token: &str,
    observed_viewer_device_key_digest: &str,
    observed_viewer_transport_identity_digest: &str,
    action: InteractiveDesktopAction,
) -> Result<CurrentInteractiveDesktopAuthority<'tx, 'conn>> {
    let head = head_on(transaction, expected.session_id())?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_MISSING"))?;
    ensure_expected_head(&head, expected)?;
    if head.is_terminal {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_TERMINAL");
    }
    let stored = version_on(transaction, &head.session_id, head.session_revision)?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_AUTHORITY_VERSION_MISSING"))?;
    let now = Utc::now();
    let sources = require_same_owner_sources_on(
        transaction,
        &stored.record,
        host_endpoint_session,
        consumer_bearer_token,
        now,
    )?;
    stored.record.verify_canonical_and_structure(
        &sources.account_id,
        &sources.account_session_digest,
        sources.account_auth_epoch,
        observed_viewer_device_key_digest,
        observed_viewer_transport_identity_digest,
        action,
        now.timestamp_millis(),
    )?;
    Ok(current_authority(stored.record))
}

pub(super) fn ensure_expected_head(
    head: &StoredAuthorityHead,
    expected: &InteractiveDesktopAuthorityHeadExpectation,
) -> Result<()> {
    if head.session_id != expected.session_id()
        || head.session_revision != expected.session_revision()
        || head.session_digest != expected.session_digest()
        || head.record_digest != expected.authority_record_digest()
    {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_HEAD_CAS_MISMATCH");
    }
    Ok(())
}

fn map_version(row: &Row<'_>) -> rusqlite::Result<RawAuthorityVersion> {
    Ok(RawAuthorityVersion {
        record_schema: row.get(0)?,
        record_digest: row.get(1)?,
        record_json: row.get(2)?,
        canonicalization: row.get(3)?,
        digest_algorithm: row.get(4)?,
        session_id: row.get(5)?,
        session_root_digest: row.get(6)?,
        session_revision: row.get(7)?,
        session_digest: row.get(8)?,
        session_state: row.get(9)?,
        is_terminal: row.get::<_, i64>(10)? == 1,
        session_reservation_id: row.get(11)?,
        session_reservation_revision: row.get(12)?,
        session_reservation_digest: row.get(13)?,
        binding_digest: row.get(14)?,
        provider_id: row.get(15)?,
        provider_policy_revision: row.get(16)?,
        provider_digest: row.get(17)?,
        provider_owner_account_id: row.get(18)?,
        consumer_account_id: row.get(19)?,
        host_lease_id: row.get(20)?,
        host_lease_digest: row.get(21)?,
        fencing_generation: row.get(22)?,
        viewer_grant_id: row.get(23)?,
        viewer_grant_digest: row.get(24)?,
        viewer_grant_generation: row.get(25)?,
        media_epoch_id: row.get(26)?,
        media_epoch_digest: row.get(27)?,
        media_epoch_sequence: row.get(28)?,
        control_epoch_id: row.get(29)?,
        control_epoch_digest: row.get(30)?,
        control_epoch_sequence: row.get(31)?,
        selected_surface_digest: row.get(32)?,
        viewer_transport_identity_digest: row.get(33)?,
        recorded_at_ms: row.get(34)?,
    })
}

fn audit_version(raw: RawAuthorityVersion) -> Result<StoredAuthorityVersion> {
    let record: InteractiveDesktopAuthorityRecord = serde_json::from_str(&raw.record_json)
        .context("interactive desktop authority record JSON is invalid")?;
    let (canonical_json, canonical_digest) = record.canonical_json_and_digest()?;
    record.verify_canonical_digests()?;
    if raw.record_schema != record.schema
        || raw.record_digest != record.record_digest
        || raw.record_digest != canonical_digest
        || raw.record_json != canonical_json
        || raw.canonicalization != "rfc8785_jcs"
        || raw.digest_algorithm != "sha256"
        || !raw.matches(&record)
    {
        bail!("INTERACTIVE_DESKTOP_AUTHORITY_VERSION_READBACK_MISMATCH");
    }
    Ok(StoredAuthorityVersion {
        record,
        record_json: raw.record_json,
        record_digest: raw.record_digest,
        recorded_at_ms: raw.recorded_at_ms,
    })
}

struct RawAuthorityVersion {
    record_schema: String,
    record_digest: String,
    record_json: String,
    canonicalization: String,
    digest_algorithm: String,
    session_id: String,
    session_root_digest: String,
    session_revision: i64,
    session_digest: String,
    session_state: String,
    is_terminal: bool,
    session_reservation_id: String,
    session_reservation_revision: i64,
    session_reservation_digest: String,
    binding_digest: String,
    provider_id: String,
    provider_policy_revision: i64,
    provider_digest: String,
    provider_owner_account_id: String,
    consumer_account_id: String,
    host_lease_id: String,
    host_lease_digest: String,
    fencing_generation: i64,
    viewer_grant_id: String,
    viewer_grant_digest: String,
    viewer_grant_generation: i64,
    media_epoch_id: String,
    media_epoch_digest: String,
    media_epoch_sequence: i64,
    control_epoch_id: String,
    control_epoch_digest: String,
    control_epoch_sequence: i64,
    selected_surface_digest: String,
    viewer_transport_identity_digest: String,
    recorded_at_ms: i64,
}

impl RawAuthorityVersion {
    fn matches(&self, record: &InteractiveDesktopAuthorityRecord) -> bool {
        let session = &record.session;
        let binding = &session.binding;
        self.session_id == session.session_id
            && self.session_root_digest == session.session_root_digest
            && self.session_revision == session.session_revision
            && self.session_digest == session.session_digest
            && self.session_state == super::session_state_name(session.state)
            && self.is_terminal == session.state.is_terminal()
            && self.session_reservation_id == session.session_reservation.session_reservation_id
            && self.session_reservation_revision
                == session.session_reservation.session_reservation_revision
            && self.session_reservation_digest
                == session.session_reservation.session_reservation_digest
            && self.binding_digest == binding.binding_digest
            && self.provider_id == binding.provider_id
            && exact_u64(
                self.provider_policy_revision,
                binding.provider_policy_revision,
            )
            && self.provider_digest == binding.provider_digest
            && self.provider_owner_account_id == binding.provider_owner_account_id
            && self.consumer_account_id == binding.consumer_account_id
            && self.host_lease_id == record.host_lease.host_lease_id
            && self.host_lease_digest == record.host_lease.host_lease_digest
            && exact_u64(
                self.fencing_generation,
                record.host_lease.fencing_generation,
            )
            && self.viewer_grant_id == record.viewer_grant.viewer_grant_id
            && self.viewer_grant_digest == record.viewer_grant.viewer_grant_digest
            && exact_u64(
                self.viewer_grant_generation,
                record.viewer_grant.grant_generation,
            )
            && self.media_epoch_id == record.media_epoch.media_epoch_id
            && self.media_epoch_digest == record.media_epoch.media_epoch_digest
            && exact_u64(self.media_epoch_sequence, record.media_epoch.epoch_sequence)
            && self.control_epoch_id == record.control_epoch.control_epoch_id
            && self.control_epoch_digest == record.control_epoch.control_epoch_digest
            && exact_u64(
                self.control_epoch_sequence,
                record.control_epoch.epoch_sequence,
            )
            && self.selected_surface_digest == record.host_lease.selected_surface.selection_digest
            && self.viewer_transport_identity_digest
                == record.viewer_grant.viewer_transport_identity_digest
    }
}

fn exact_u64(stored: i64, expected: u64) -> bool {
    u64::try_from(stored).ok() == Some(expected)
}

const VERSION_SELECT: &str = "SELECT authority_record_schema,authority_record_digest,
 authority_record_json,canonicalization,digest_algorithm,session_id,session_root_digest,
 session_revision,session_digest,session_state,is_terminal,session_reservation_id,
 session_reservation_revision,session_reservation_digest,binding_digest,provider_id,
 provider_policy_revision,provider_digest,provider_owner_account_id,consumer_account_id,
 host_lease_id,host_lease_digest,fencing_generation,viewer_grant_id,viewer_grant_digest,
 viewer_grant_generation,media_epoch_id,media_epoch_digest,media_epoch_sequence,
 control_epoch_id,control_epoch_digest,control_epoch_sequence,selected_surface_digest,
 viewer_transport_identity_digest,recorded_at_ms
 FROM compute_interactive_desktop_authority_versions
 WHERE session_id=?1 AND session_revision=?2";
