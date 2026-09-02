use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::{ComputeCapacityClaim, ComputeCapacityClaimKind, ComputeCapacityClaimState},
        interactive_desktop::{
            authority_record::InteractiveDesktopAuthorityRecord,
            offer::InteractiveDesktopProductMode,
            product_authority::InteractiveDesktopProductAuthorityProof,
        },
        provider::{PROVIDER_KIND_USER_NODE, PROVIDER_STATUS_ACTIVE},
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::super::{
    compute_capacity_claim_rows::stored_claim_on,
    compute_capacity_pool_queries::audited_compute_capacity_pool_version_on,
    compute_provider_registry::current_registered_provider_on,
    compute_reservation_registry::current_registered_reservation_on,
    compute_user_node_provider_bindings::current_user_node_provider_binding_by_digest_on,
    hash_token,
    node_credentials::{
        require_current_node_endpoint_runtime_session_on, NodeEndpointSessionPermit,
    },
};

const ACCOUNT_SESSION_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-INTERACTIVE-DESKTOP-ACCOUNT-SESSION-V1";
const MAX_ACCOUNT_SESSION_JSON_BYTES: usize = 16 * 1024;
const MAX_IJSON_SAFE_AUTH_EPOCH: u64 = (1_u64 << 53) - 1;

pub(super) struct CurrentSameOwnerSources {
    pub account_id: String,
    pub account_session_digest: String,
    pub account_auth_epoch: u64,
}

pub(super) fn require_same_owner_sources_on(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    host_endpoint_session: &NodeEndpointSessionPermit,
    consumer_bearer_token: &str,
    checked_at: DateTime<Utc>,
) -> Result<CurrentSameOwnerSources> {
    if record.profile.offer.product_mode != InteractiveDesktopProductMode::SameOwnerRemoteAccess {
        bail!("INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_STORE_UNAVAILABLE");
    }
    let binding = &record.session.binding;
    let provider = current_registered_provider_on(transaction, &binding.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_PROVIDER_NOT_CURRENT"))?;
    if provider.provider.provider_kind != PROVIDER_KIND_USER_NODE
        || provider.provider.status != PROVIDER_STATUS_ACTIVE
        || provider.provider.owner_account_id != binding.provider_owner_account_id
        || provider.provider.policy_revision != i64::try_from(binding.provider_policy_revision)?
        || provider.provider_digest != binding.provider_digest
    {
        bail!("INTERACTIVE_DESKTOP_PROVIDER_AUTHORITY_MISMATCH");
    }

    let node_binding = current_user_node_provider_binding_by_digest_on(
        transaction,
        &binding.provider_id,
        &record.host_lease.provider_node_binding_digest,
        &binding.provider_owner_account_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_NODE_BINDING_NOT_CURRENT"))?;
    let node_material = node_binding.receipt().binding();
    if node_material.node_id() != record.host_lease.host_node_id {
        bail!("INTERACTIVE_DESKTOP_HOST_NODE_BINDING_MISMATCH");
    }

    let endpoint = require_current_node_endpoint_runtime_session_on(
        transaction,
        host_endpoint_session,
        checked_at,
    )?;
    if endpoint.owner_user_id() != binding.provider_owner_account_id
        || endpoint.binding().agent_id() != record.host_lease.host_node_id
        || endpoint.binding().credential_digest() != record.host_lease.endpoint_credential_digest
        || endpoint.installation_binding_digest()
            != node_material.endpoint_installation_binding_digest()
    {
        bail!("INTERACTIVE_DESKTOP_HOST_ENDPOINT_AUTHORITY_MISMATCH");
    }
    require_host_consent_endpoint_binding(record, &endpoint)?;
    require_shared_federation_sources(transaction, record, checked_at)?;
    require_same_owner_product_authority(record, &provider)?;

    let account = current_account_session_on(
        transaction,
        consumer_bearer_token,
        &binding.consumer_account_id,
        checked_at,
    )?;
    if account.account_id != binding.provider_owner_account_id
        || account.account_session_digest != record.viewer_grant.consumer_account_session_digest
        || record.viewer_grant.account_auth_epoch != account.account_auth_epoch
    {
        bail!("INTERACTIVE_DESKTOP_SAME_OWNER_ACCOUNT_AUTHORITY_MISMATCH");
    }
    require_unavailable_interactive_sources(record)?;
    Ok(account)
}

/// Binds the asserted consent to this endpoint session. It is not evidence that the Provider
/// approved screen, audio, or input; the unavailable-source gate below requires that separate
/// Store before an active authority can pass.
fn require_host_consent_endpoint_binding(
    record: &InteractiveDesktopAuthorityRecord,
    endpoint: &NodeEndpointSessionPermit,
) -> Result<()> {
    let consent = &record.host_lease.host_consent;
    if consent.policy_id != endpoint.binding().session_id()
        || consent.policy_revision != endpoint.binding().session_generation()
        || consent.policy_digest != endpoint.binding().authentication_digest()
    {
        bail!("INTERACTIVE_DESKTOP_HOST_CONSENT_ISSUER_MISMATCH");
    }
    Ok(())
}

fn require_same_owner_product_authority(
    record: &InteractiveDesktopAuthorityRecord,
    provider: &super::super::ComputeProviderRegistrationReceipt,
) -> Result<()> {
    let authority = &record.reservation.product_authority;
    let InteractiveDesktopProductAuthorityProof::SameOwnerAccount {
        ownership_snapshot_id,
        ownership_snapshot_digest,
        account_id,
    } = &authority.proof
    else {
        bail!("INTERACTIVE_DESKTOP_SAME_OWNER_PROOF_REQUIRED");
    };
    if authority.issuer_id != provider.provider.owner_account_id
        || authority.issuer_policy_digest != provider.provider_digest
        || ownership_snapshot_id != &provider.provider.provider_id
        || ownership_snapshot_digest != &provider.provider_digest
        || account_id != &provider.provider.owner_account_id
    {
        bail!("INTERACTIVE_DESKTOP_SAME_OWNER_PROOF_MISMATCH");
    }
    Ok(())
}

fn require_shared_federation_sources(
    transaction: &Transaction<'_>,
    record: &InteractiveDesktopAuthorityRecord,
    checked_at: DateTime<Utc>,
) -> Result<()> {
    let binding = &record.session.binding;
    let reserved =
        current_registered_reservation_on(transaction, &binding.reservation.reservation_id)?
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_RESERVATION_NOT_CURRENT"))?;
    let reservation = &reserved.reservation;
    if reserved.revision != binding.reservation.reservation_revision
        || reserved.reservation_digest != binding.reservation.reservation_digest
        || reservation.status != "active"
        || reservation.offer.provider_id != binding.provider_id
        || reservation.offer.offer_id != binding.offer.offer_id
        || reservation.offer.offer_version != binding.offer.offer_version
        || reservation.offer.offer_digest != binding.offer.offer_digest
        || reservation.price_snapshot.snapshot_id != binding.price_snapshot.price_snapshot_id
        || reservation.price_snapshot.snapshot_digest
            != binding.price_snapshot.price_snapshot_digest
        || reservation.capacity_claim != binding.capacity_claim
        || parse_time(&reservation.expires_at)? <= checked_at
    {
        bail!("INTERACTIVE_DESKTOP_RESERVATION_AUTHORITY_MISMATCH");
    }
    let stored_consumer = transaction
        .query_row(
            "SELECT consumer_account_id FROM compute_reservations WHERE reservation_id=?1",
            params![binding.reservation.reservation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_RESERVATION_CONSUMER_MISSING"))?;
    if stored_consumer != binding.consumer_account_id {
        bail!("INTERACTIVE_DESKTOP_RESERVATION_CONSUMER_MISMATCH");
    }

    let claim = stored_claim_on(transaction, &binding.capacity_claim.claim_id)?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_CAPACITY_CLAIM_NOT_CURRENT"))?;
    let claim_expired = claim
        .expires_at
        .as_deref()
        .map(parse_time)
        .transpose()?
        .is_some_and(|expires_at| expires_at <= checked_at);
    if claim.revision != binding.capacity_claim.claim_revision
        || claim.claim_digest != binding.capacity_claim.claim_digest
        || claim.pool != binding.capacity_pool
        || claim.claim_kind != ComputeCapacityClaimKind::Reservation
        || claim.subject_kind != "compute_reservation"
        || !matches!(
            claim.state,
            ComputeCapacityClaimState::Held | ComputeCapacityClaimState::Active
        )
        || claim.subject_id != binding.reservation.reservation_id
        || claim_expired
    {
        bail!("INTERACTIVE_DESKTOP_CAPACITY_CLAIM_MISMATCH");
    }
    let pool = audited_compute_capacity_pool_version_on(
        transaction,
        &binding.capacity_pool.pool_id,
        binding.capacity_pool.capacity_epoch,
        binding.capacity_pool.pool_revision,
    )?
    .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_CAPACITY_POOL_VERSION_MISSING"))?;
    if pool.binding != binding.capacity_pool
        || pool.provider_id != binding.provider_id
        || pool.resource_scope_digest != record.reservation.resource_scope_digest
        || pool.region_or_data_zone != record.profile.region_or_data_zone
    {
        bail!("INTERACTIVE_DESKTOP_CAPACITY_POOL_AUTHORITY_MISMATCH");
    }
    require_exact_interactive_meter_reservation(record, &claim, &pool.meter_policies)?;
    Ok(())
}

fn require_exact_interactive_meter_reservation(
    record: &InteractiveDesktopAuthorityRecord,
    claim: &ComputeCapacityClaim,
    meter_policies: &[crate::compute_federation::capacity::ComputeCapacityMeterPolicy],
) -> Result<()> {
    let boundary = &record.profile.resource_boundary;
    let required = [
        boundary.gpu_meter.as_str(),
        boundary.encoder_slot_meter.as_str(),
        boundary.network_egress_meter.as_str(),
        boundary.interactive_login_slot_meter.as_str(),
    ];
    if record.reservation.meter_budgets.len() != required.len()
        || claim.lines.len() != required.len()
        || meter_policies.len() < required.len()
    {
        bail!("INTERACTIVE_DESKTOP_CAPACITY_METER_CARDINALITY_MISMATCH");
    }
    for meter in required {
        let budget = record
            .reservation
            .meter_budgets
            .iter()
            .find(|budget| budget.meter == meter)
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_CAPACITY_METER_BUDGET_MISSING"))?;
        let line = claim
            .lines
            .iter()
            .find(|line| line.bucket.meter == meter)
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_CAPACITY_CLAIM_LINE_MISSING"))?;
        let policy = meter_policies
            .iter()
            .find(|policy| policy.meter == meter)
            .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_CAPACITY_METER_POLICY_MISSING"))?;
        if line.quantity_units != i64::try_from(budget.maximum_quantity)?
            || line.bucket.pool != record.session.binding.capacity_pool
            || line.bucket.meter_mode != policy.meter_mode
            || line.bucket.quantum_units != policy.quantum_units
            || line.bucket.meter_policy_digest != policy.policy_digest
        {
            bail!("INTERACTIVE_DESKTOP_CAPACITY_METER_AUTHORITY_MISMATCH");
        }
    }
    Ok(())
}

/// These independent producers do not exist in C1. Keeping this explicit gate after all
/// available reproofs prevents a future API from treating the source-only kernel as a permit.
fn require_unavailable_interactive_sources(
    record: &InteractiveDesktopAuthorityRecord,
) -> Result<()> {
    let suffix = if record.media_epoch.relay_authority.is_some() {
        "PROFILE_HOST_CONSENT_VIEWER_AND_RELAY_STORES_UNAVAILABLE"
    } else {
        "PROFILE_HOST_CONSENT_AND_VIEWER_STORES_UNAVAILABLE"
    };
    bail!("INTERACTIVE_DESKTOP_C1_{suffix}")
}

#[derive(Serialize)]
struct AccountSessionDigestMaterial<'a> {
    session_id: &'a str,
    account_id: &'a str,
    token_hash: &'a str,
    expires_at: &'a str,
    created_at: &'a str,
    trusted_device: bool,
}

fn current_account_session_on(
    transaction: &Transaction<'_>,
    bearer_token: &str,
    expected_account_id: &str,
    checked_at: DateTime<Utc>,
) -> Result<CurrentSameOwnerSources> {
    let token = bearer_token.trim();
    if token.is_empty() {
        bail!("INTERACTIVE_DESKTOP_CONSUMER_BEARER_REQUIRED");
    }
    let token_hash = hash_token(token);
    let row = transaction
        .query_row(
            "SELECT s.id,s.user_id,s.token_hash,s.expires_at,s.created_at,s.trusted_device
               FROM sessions s JOIN users u ON u.id=s.user_id
              WHERE s.token_hash=?1 AND s.user_id=?2 AND s.revoked_at IS NULL
                AND u.status='active'",
            params![token_hash, expected_account_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("INTERACTIVE_DESKTOP_ACCOUNT_SESSION_NOT_CURRENT"))?;
    if parse_time(&row.3)? <= checked_at {
        bail!("INTERACTIVE_DESKTOP_ACCOUNT_SESSION_EXPIRED");
    }
    let material = AccountSessionDigestMaterial {
        session_id: &row.0,
        account_id: &row.1,
        token_hash: &row.2,
        expires_at: &row.3,
        created_at: &row.4,
        trusted_device: row.5 == 1,
    };
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(&material, MAX_ACCOUNT_SESSION_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(ACCOUNT_SESSION_DIGEST_DOMAIN);
    digest.update([0]);
    digest.update(json.as_bytes());
    let digest = digest.finalize();
    let mut epoch_bytes = [0_u8; 8];
    epoch_bytes.copy_from_slice(&digest[..8]);
    let account_auth_epoch = (u64::from_be_bytes(epoch_bytes) & MAX_IJSON_SAFE_AUTH_EPOCH).max(1);
    Ok(CurrentSameOwnerSources {
        account_id: row.1,
        account_session_digest: hex::encode(digest),
        account_auth_epoch,
    })
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid RFC3339 authority time: {value}"))?
        .with_timezone(&Utc))
}
