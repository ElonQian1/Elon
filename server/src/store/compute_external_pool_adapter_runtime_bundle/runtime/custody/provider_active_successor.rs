use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use zeroize::Zeroize;

use super::{
    atomic_activation_plan::ExternalPoolAdapterAtomicActivationPendingPlanGuard,
    support::{constant_time_equal, is_lower_hex_sha256, update_field},
    ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
};
use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::{
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
        PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND,
    },
    store::compute_external_pool_adapter_runtime_bundle::locked_bytes::LockedSensitiveBytes,
};

mod validation;

use validation::{valid_id, valid_kind, valid_tuple, validate_input, validate_registry_tuple};

const PROCESS_NONCE_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PROCESS-NONCE-V1";
const PROCESS_SEAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PROCESS-SEAL-V1";
const PROCESS_NONCE_BYTES: usize = 32;
const MAX_LIVE_PROCESS_SEALS: usize = 4_096;

pub(in crate::store) struct ExternalPoolAdapterProviderActiveSuccessorProcessSealInput<'a> {
    pub(in crate::store) kind: &'a str,
    pub(in crate::store) entity_id: &'a str,
    pub(in crate::store) entity_digest: &'a str,
    pub(in crate::store) activation_root_digest: &'a str,
    pub(in crate::store) provider_binding_id: &'a str,
    pub(in crate::store) checked_at: &'a str,
    pub(in crate::store) expires_at: &'a str,
}

/// Process-local purpose seal. It cannot be cloned, formatted, serialized, or imported.
pub(in crate::store) struct ExternalPoolAdapterProviderActiveSuccessorProcessSeal {
    kind: String,
    entity_id: String,
    entity_digest: String,
    process_custody_nonce_digest: String,
    process_custody_seal_digest: String,
    expires_at: String,
}

impl ExternalPoolAdapterProviderActiveSuccessorProcessSeal {
    pub(in crate::store) fn process_custody_nonce_digest(&self) -> &str {
        &self.process_custody_nonce_digest
    }

    pub(in crate::store) fn process_custody_seal_digest(&self) -> &str {
        &self.process_custody_seal_digest
    }
}

impl Drop for ExternalPoolAdapterProviderActiveSuccessorProcessSeal {
    fn drop(&mut self) {
        self.entity_digest.zeroize();
        self.process_custody_nonce_digest.zeroize();
        self.process_custody_seal_digest.zeroize();
    }
}

#[derive(Default)]
pub(super) struct ProviderActiveSuccessorSealRegistry {
    by_identity: HashMap<(String, String), ProviderActiveSuccessorRegistrySeal>,
}

struct ProviderActiveSuccessorRegistrySeal {
    entity_digest: String,
    process_custody_epoch_digest: String,
    process_custody_nonce_digest: String,
    process_custody_seal_digest: String,
    receipt_integrity_digest: String,
    expires_at: String,
    expires_at_utc: DateTime<Utc>,
    committed: bool,
}

impl ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
    pub(in crate::store) fn seal_provider_active_successor(
        &self,
        input: &ExternalPoolAdapterProviderActiveSuccessorProcessSealInput<'_>,
    ) -> Result<ExternalPoolAdapterProviderActiveSuccessorProcessSeal> {
        validate_input(input, Utc::now())?;
        let nonce = LockedSensitiveBytes::random(PROCESS_NONCE_BYTES)
            .map_err(|_| anyhow!("active-successor process nonce is unavailable"))?;
        let process_custody_nonce_digest = self.with_commitment(PROCESS_NONCE_DOMAIN, |mac| {
            update_field(mac, nonce.as_slice());
        })?;
        let process_custody_seal_digest = self.with_commitment(PROCESS_SEAL_DOMAIN, |mac| {
            update_field(mac, input.kind.as_bytes());
            update_field(mac, input.entity_id.as_bytes());
            update_field(mac, input.entity_digest.as_bytes());
            update_field(mac, input.activation_root_digest.as_bytes());
            update_field(mac, input.provider_binding_id.as_bytes());
            update_field(mac, process_custody_nonce_digest.as_bytes());
            update_field(mac, input.checked_at.as_bytes());
            update_field(mac, input.expires_at.as_bytes());
        })?;
        Ok(ExternalPoolAdapterProviderActiveSuccessorProcessSeal {
            kind: input.kind.to_owned(),
            entity_id: input.entity_id.to_owned(),
            entity_digest: input.entity_digest.to_owned(),
            process_custody_nonce_digest,
            process_custody_seal_digest,
            expires_at: input.expires_at.to_owned(),
        })
    }

    /// Registers an exact pending tuple. This method intentionally has no V274 caller before V277.
    pub(in crate::store) fn remember_pending_provider_active_successor_process_seal(
        &self,
        receipt_integrity_digest: &str,
        seal: &ExternalPoolAdapterProviderActiveSuccessorProcessSeal,
    ) -> Result<()> {
        let now = Utc::now();
        let expires_at_utc = validate_registry_tuple(
            seal.kind.as_str(),
            seal.entity_id.as_str(),
            seal.entity_digest.as_str(),
            self.custody_epoch_digest(),
            seal.process_custody_nonce_digest.as_str(),
            seal.process_custody_seal_digest.as_str(),
            receipt_integrity_digest,
            seal.expires_at.as_str(),
            now.clone(),
        )?;
        let key = (seal.kind.clone(), seal.entity_id.clone());
        let mut registry = self.registry()?;
        registry.prune(now);
        if let Some(existing) = registry.by_identity.get(&key) {
            if existing.matches(
                seal.entity_digest.as_str(),
                self.custody_epoch_digest(),
                seal.process_custody_nonce_digest.as_str(),
                seal.process_custody_seal_digest.as_str(),
                receipt_integrity_digest,
                seal.expires_at.as_str(),
            ) {
                return Ok(());
            }
            bail!("active-successor identity has a different pending process seal");
        }
        if registry.by_identity.len() >= MAX_LIVE_PROCESS_SEALS {
            bail!("active-successor process seal registry reached its fixed bound");
        }
        registry.by_identity.insert(
            key,
            ProviderActiveSuccessorRegistrySeal {
                entity_digest: seal.entity_digest.clone(),
                process_custody_epoch_digest: self.custody_epoch_digest().to_owned(),
                process_custody_nonce_digest: seal.process_custody_nonce_digest.clone(),
                process_custody_seal_digest: seal.process_custody_seal_digest.clone(),
                receipt_integrity_digest: receipt_integrity_digest.to_owned(),
                expires_at: seal.expires_at.clone(),
                expires_at_utc,
                committed: false,
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn attests_pending_provider_active_successor_process_seal(
        &self,
        kind: &str,
        entity_id: &str,
        entity_digest: &str,
        process_custody_epoch_digest: &str,
        process_custody_nonce_digest: &str,
        process_custody_seal_digest: &str,
        receipt_integrity_digest: &str,
    ) -> Result<bool> {
        if !valid_tuple(
            kind,
            entity_id,
            entity_digest,
            process_custody_epoch_digest,
            process_custody_nonce_digest,
            process_custody_seal_digest,
            receipt_integrity_digest,
        ) || !self.attests_custody_epoch_digest(process_custody_epoch_digest)
        {
            return Ok(false);
        }
        let now = Utc::now();
        let mut registry = self.registry()?;
        registry.prune(now);
        Ok(registry
            .by_identity
            .get(&(kind.to_owned(), entity_id.to_owned()))
            .is_some_and(|stored| {
                !stored.committed
                    && stored.matches_without_expiry(
                        entity_digest,
                        process_custody_epoch_digest,
                        process_custody_nonce_digest,
                        process_custody_seal_digest,
                        receipt_integrity_digest,
                    )
            }))
    }

    pub(in crate::store) fn promote_provider_active_successor_process_seal(
        &self,
        connection: &Connection,
        plan_guard: &ExternalPoolAdapterAtomicActivationPendingPlanGuard,
        kind: &str,
        entity_id: &str,
        receipt_integrity_digest: &str,
    ) -> Result<bool> {
        ensure!(
            connection.is_autocommit(),
            "V274 process seal promotion requires a committed autocommit connection"
        );
        plan_guard.ensure_same_connection(connection)?;
        plan_guard.ensure_fully_consumed()?;
        if !valid_kind(kind)
            || !valid_id(entity_id)
            || !is_lower_hex_sha256(receipt_integrity_digest)
        {
            return Ok(false);
        }
        let now = Utc::now();
        let mut registry = self.registry()?;
        registry.prune(now);
        let key = (kind.to_owned(), entity_id.to_owned());
        let Some(stored) = registry.by_identity.get_mut(&key) else {
            return Ok(false);
        };
        if !constant_time_equal(&stored.receipt_integrity_digest, receipt_integrity_digest) {
            return Ok(false);
        }
        if kind == PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND {
            registry.by_identity.remove(&key);
        } else {
            stored.committed = true;
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) fn attests_committed_provider_active_successor_process_seal(
        &self,
        kind: &str,
        entity_id: &str,
        entity_digest: &str,
        process_custody_epoch_digest: &str,
        process_custody_nonce_digest: &str,
        process_custody_seal_digest: &str,
        receipt_integrity_digest: &str,
        expires_at: &str,
    ) -> Result<bool> {
        let now = Utc::now();
        if validate_registry_tuple(
            kind,
            entity_id,
            entity_digest,
            process_custody_epoch_digest,
            process_custody_nonce_digest,
            process_custody_seal_digest,
            receipt_integrity_digest,
            expires_at,
            now.clone(),
        )
        .is_err()
            || !self.attests_custody_epoch_digest(process_custody_epoch_digest)
        {
            return Ok(false);
        }
        let mut registry = self.registry()?;
        registry.prune(now);
        Ok(registry
            .by_identity
            .get(&(kind.to_owned(), entity_id.to_owned()))
            .is_some_and(|stored| {
                stored.committed
                    && stored.matches(
                        entity_digest,
                        process_custody_epoch_digest,
                        process_custody_nonce_digest,
                        process_custody_seal_digest,
                        receipt_integrity_digest,
                        expires_at,
                    )
            }))
    }

    pub(in crate::store) fn discard_pending_provider_active_successor_process_seal(
        &self,
        kind: &str,
        entity_id: &str,
        receipt_integrity_digest: &str,
    ) -> Result<bool> {
        let mut registry = self.registry()?;
        let key = (kind.to_owned(), entity_id.to_owned());
        let removable = registry.by_identity.get(&key).is_some_and(|stored| {
            !stored.committed
                && constant_time_equal(&stored.receipt_integrity_digest, receipt_integrity_digest)
        });
        if removable {
            registry.by_identity.remove(&key);
        }
        Ok(removable)
    }

    fn registry(&self) -> Result<std::sync::MutexGuard<'_, ProviderActiveSuccessorSealRegistry>> {
        self.provider_active_successor_seals
            .lock()
            .map_err(|_| anyhow!("active-successor process seal registry lock was poisoned"))
    }
}

impl ProviderActiveSuccessorSealRegistry {
    fn prune(&mut self, now: DateTime<Utc>) {
        self.by_identity.retain(|_, seal| seal.expires_at_utc > now);
    }
}

impl ProviderActiveSuccessorRegistrySeal {
    fn matches_without_expiry(
        &self,
        entity_digest: &str,
        epoch: &str,
        nonce: &str,
        seal: &str,
        integrity: &str,
    ) -> bool {
        constant_time_equal(&self.entity_digest, entity_digest)
            & constant_time_equal(&self.process_custody_epoch_digest, epoch)
            & constant_time_equal(&self.process_custody_nonce_digest, nonce)
            & constant_time_equal(&self.process_custody_seal_digest, seal)
            & constant_time_equal(&self.receipt_integrity_digest, integrity)
    }

    fn matches(
        &self,
        entity_digest: &str,
        epoch: &str,
        nonce: &str,
        seal: &str,
        integrity: &str,
        expires_at: &str,
    ) -> bool {
        self.matches_without_expiry(entity_digest, epoch, nonce, seal, integrity)
            & (self.expires_at == expires_at)
    }
}
