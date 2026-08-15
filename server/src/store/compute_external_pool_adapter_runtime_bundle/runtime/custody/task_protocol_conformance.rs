use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use zeroize::Zeroize;

use super::{
    support::{constant_time_equal, is_lower_hex_sha256, update_field},
    ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
};

const PROCESS_SEAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-PROCESS-SEAL-V1";
pub(super) const CUSTODY_EPOCH_DIGEST_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-CUSTODY-EPOCH-DIGEST-V1";
const MAX_PROCESS_SEAL_TTL_MS: i64 = 15_000;
const MAX_LIVE_PROCESS_SEALS: usize = 4_096;

pub(in crate::store) struct ExternalPoolAdapterTaskProtocolConformanceSealInput<'a> {
    pub(in crate::store) run_receipt_digest: &'a str,
    pub(in crate::store) task_observation_root: &'a str,
    pub(in crate::store) session_roots_digest: &'a str,
    pub(in crate::store) session_transcript_digest: &'a str,
    pub(in crate::store) delivery_inventory_digest: &'a str,
    pub(in crate::store) exchange_inventory_digest: &'a str,
    pub(in crate::store) post_cleanup_checked_at: &'a str,
    pub(in crate::store) expires_at: &'a str,
}

/// Ephemeral same-process seal. It is intentionally non-Clone, non-Debug and non-Serde.
pub(in crate::store) struct TaskProtocolConformanceProcessSeal {
    seal: String,
    run_receipt_digest: String,
    expires_at: String,
}

impl TaskProtocolConformanceProcessSeal {
    pub(in crate::store) fn seal_hex(&self) -> &str {
        &self.seal
    }
}

impl Drop for TaskProtocolConformanceProcessSeal {
    fn drop(&mut self) {
        self.seal.zeroize();
        self.run_receipt_digest.zeroize();
        self.expires_at.zeroize();
    }
}

#[derive(Default)]
pub(super) struct TaskProtocolConformanceSealRegistry {
    by_receipt_id: HashMap<String, TaskProtocolConformanceRegistrySeal>,
}

struct TaskProtocolConformanceRegistrySeal {
    receipt_integrity_digest: String,
    run_receipt_digest: String,
    process_hmac_seal: String,
    expires_at: String,
    expires_at_utc: DateTime<Utc>,
    committed: bool,
}

impl ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
    pub(in crate::store) fn generate_task_protocol_conformance() -> Result<Self> {
        Self::generate_for_epoch_domain(CUSTODY_EPOCH_DIGEST_DOMAIN, "task protocol conformance")
    }

    pub(in crate::store) fn seal_task_protocol_conformance(
        &self,
        input: &ExternalPoolAdapterTaskProtocolConformanceSealInput<'_>,
    ) -> Result<TaskProtocolConformanceProcessSeal> {
        validate_seal_input(input, Utc::now())?;
        let seal = self.with_commitment(PROCESS_SEAL_DOMAIN, |mac| {
            update_field(mac, input.run_receipt_digest.as_bytes());
            update_field(mac, input.task_observation_root.as_bytes());
            update_field(mac, input.session_roots_digest.as_bytes());
            update_field(mac, input.session_transcript_digest.as_bytes());
            update_field(mac, input.delivery_inventory_digest.as_bytes());
            update_field(mac, input.exchange_inventory_digest.as_bytes());
            update_field(mac, input.post_cleanup_checked_at.as_bytes());
            update_field(mac, input.expires_at.as_bytes());
            update_field(mac, self.custody_epoch_digest().as_bytes());
        })?;
        Ok(TaskProtocolConformanceProcessSeal {
            seal,
            run_receipt_digest: input.run_receipt_digest.to_owned(),
            expires_at: input.expires_at.to_owned(),
        })
    }

    /// Remembers an exact pending tuple. Pending state can never attest current authority.
    pub(in crate::store) fn remember_pending_task_protocol_conformance_seal(
        &self,
        receipt_id: &str,
        receipt_integrity_digest: &str,
        process_hmac_seal: &TaskProtocolConformanceProcessSeal,
    ) -> Result<()> {
        let run_receipt_digest = process_hmac_seal.run_receipt_digest.as_str();
        let expires_at = process_hmac_seal.expires_at.as_str();
        let process_hmac_seal = process_hmac_seal.seal_hex();
        let now = Utc::now();
        let expires_at_utc = validate_registry_material(
            receipt_id,
            receipt_integrity_digest,
            run_receipt_digest,
            process_hmac_seal,
            expires_at,
            now.clone(),
        )?;
        let mut registry = self
            .task_protocol_conformance_seals
            .lock()
            .map_err(|_| anyhow!("task conformance seal registry lock was poisoned"))?;
        registry.prune(now);
        if let Some(existing) = registry.by_receipt_id.get(receipt_id) {
            if existing.matches(
                receipt_integrity_digest,
                run_receipt_digest,
                process_hmac_seal,
                expires_at,
            ) {
                return Ok(());
            }
            bail!("task conformance receipt identity has a different process seal");
        }
        if registry.by_receipt_id.len() >= MAX_LIVE_PROCESS_SEALS {
            bail!("task conformance process seal registry reached its fixed bound");
        }
        registry.by_receipt_id.insert(
            receipt_id.to_owned(),
            TaskProtocolConformanceRegistrySeal {
                receipt_integrity_digest: receipt_integrity_digest.to_owned(),
                run_receipt_digest: run_receipt_digest.to_owned(),
                process_hmac_seal: process_hmac_seal.to_owned(),
                expires_at: expires_at.to_owned(),
                expires_at_utc,
                committed: false,
            },
        );
        Ok(())
    }

    /// Promotes only the exact pending tuple after the enclosing database commit succeeds.
    pub(in crate::store) fn promote_task_protocol_conformance_seal(
        &self,
        receipt_id: &str,
        receipt_integrity_digest: &str,
    ) -> Result<bool> {
        if !valid_receipt_id(receipt_id) || !is_lower_hex_sha256(receipt_integrity_digest) {
            return Ok(false);
        }
        let now = Utc::now();
        let mut registry = self
            .task_protocol_conformance_seals
            .lock()
            .map_err(|_| anyhow!("task conformance seal registry lock was poisoned"))?;
        registry.prune(now);
        let Some(seal) = registry.by_receipt_id.get_mut(receipt_id) else {
            return Ok(false);
        };
        if !constant_time_equal(&seal.receipt_integrity_digest, receipt_integrity_digest) {
            return Ok(false);
        }
        seal.committed = true;
        Ok(true)
    }

    /// Verifies exact committed in-process custody; database fields alone cannot recreate it.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) fn attests_task_protocol_conformance_seal(
        &self,
        receipt_id: &str,
        receipt_integrity_digest: &str,
        run_receipt_digest: &str,
        runtime_custody_epoch_digest: &str,
        process_hmac_seal: &str,
        expires_at: &str,
    ) -> Result<bool> {
        let now = Utc::now();
        if !self.attests_custody_epoch_digest(runtime_custody_epoch_digest)
            || validate_registry_material(
                receipt_id,
                receipt_integrity_digest,
                run_receipt_digest,
                process_hmac_seal,
                expires_at,
                now.clone(),
            )
            .is_err()
        {
            return Ok(false);
        }
        let mut registry = self
            .task_protocol_conformance_seals
            .lock()
            .map_err(|_| anyhow!("task conformance seal registry lock was poisoned"))?;
        registry.prune(now);
        Ok(registry.by_receipt_id.get(receipt_id).is_some_and(|seal| {
            seal.committed
                && seal.matches(
                    receipt_integrity_digest,
                    run_receipt_digest,
                    process_hmac_seal,
                    expires_at,
                )
        }))
    }
}

impl TaskProtocolConformanceSealRegistry {
    fn prune(&mut self, now: DateTime<Utc>) {
        self.by_receipt_id
            .retain(|_, seal| seal.expires_at_utc > now);
    }
}

impl TaskProtocolConformanceRegistrySeal {
    fn matches(
        &self,
        receipt_integrity_digest: &str,
        run_receipt_digest: &str,
        process_hmac_seal: &str,
        expires_at: &str,
    ) -> bool {
        constant_time_equal(&self.receipt_integrity_digest, receipt_integrity_digest)
            & constant_time_equal(&self.run_receipt_digest, run_receipt_digest)
            & constant_time_equal(&self.process_hmac_seal, process_hmac_seal)
            & (self.expires_at == expires_at)
    }
}

fn validate_seal_input(
    input: &ExternalPoolAdapterTaskProtocolConformanceSealInput<'_>,
    now: DateTime<Utc>,
) -> Result<()> {
    if [
        input.run_receipt_digest,
        input.task_observation_root,
        input.session_roots_digest,
        input.session_transcript_digest,
        input.delivery_inventory_digest,
        input.exchange_inventory_digest,
    ]
    .into_iter()
    .any(|digest| !is_lower_hex_sha256(digest))
    {
        bail!("task conformance process seal roots are invalid");
    }
    validate_expiry(input.post_cleanup_checked_at, input.expires_at, now)?;
    Ok(())
}

fn validate_registry_material(
    receipt_id: &str,
    receipt_integrity_digest: &str,
    run_receipt_digest: &str,
    process_hmac_seal: &str,
    expires_at: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>> {
    if !valid_receipt_id(receipt_id)
        || !is_lower_hex_sha256(receipt_integrity_digest)
        || !is_lower_hex_sha256(run_receipt_digest)
        || !is_lower_hex_sha256(process_hmac_seal)
    {
        bail!("task conformance process seal material is invalid");
    }
    let expires_at_utc = parse_canonical_time(expires_at)?;
    if expires_at_utc <= now || expires_at_utc > now + ChronoDuration::milliseconds(15_000) {
        bail!("task conformance process seal expiry is outside its live window");
    }
    Ok(expires_at_utc)
}

fn validate_expiry(checked_at: &str, expires_at: &str, now: DateTime<Utc>) -> Result<()> {
    let checked_at = parse_canonical_time(checked_at)?;
    let expires_at = parse_canonical_time(expires_at)?;
    if checked_at > now
        || expires_at <= now
        || expires_at <= checked_at
        || expires_at > checked_at + ChronoDuration::milliseconds(MAX_PROCESS_SEAL_TTL_MS)
    {
        bail!("task conformance process seal timestamps are outside the fixed live window");
    }
    Ok(())
}

fn parse_canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("task conformance process seal time is not canonical UTC nanoseconds");
    }
    Ok(parsed.with_timezone(&Utc))
}

fn valid_receipt_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
