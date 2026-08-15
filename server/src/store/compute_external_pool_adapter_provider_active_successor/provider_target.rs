use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    compute_federation::{
        external_pool_adapter_provider_active_successor::{
            derive_external_pool_adapter_provider_active_successor_activation_root,
            ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
            ExternalPoolAdapterProviderActiveSuccessorStructuralInput,
        },
        provider::{ComputeProvider, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
    },
    store::compute_provider_registry::{
        validate_compute_provider_contract, ComputeProviderRegistrationReceipt,
    },
};

pub(super) fn derive_target(
    source: &ComputeProviderRegistrationReceipt,
    structural: ExternalPoolAdapterProviderActiveSuccessorStructuralInput,
    checked_at: &str,
) -> Result<(
    ComputeProvider,
    ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
)> {
    canonical_checked_at(checked_at)?;
    let source_json = serde_json::to_string(&source.provider)?;
    if source.provider.status != PROVIDER_STATUS_REGISTERING
        || source.provider_digest != sha256_hex(source_json.as_bytes())
    {
        bail!("provider active-successor source Provider is not exact registering history");
    }
    let root = derive_external_pool_adapter_provider_active_successor_activation_root(
        &source.provider,
        structural,
        checked_at,
    )?;
    let envelope = &root.activation_root;
    if envelope.source_registering_provider_json != source_json
        || envelope.source_registering_provider_digest != source.provider_digest
    {
        bail!("provider active-successor source Provider serialization drifted");
    }
    let target: ComputeProvider = serde_json::from_str(&envelope.initial_active_provider_json)?;
    validate_compute_provider_contract(&target)?;
    if target.provider_id != source.provider.provider_id
        || target.owner_account_id != source.provider.owner_account_id
        || target.created_at != source.provider.created_at
        || target.status != PROVIDER_STATUS_ACTIVE
        || target.policy_revision != source.provider.policy_revision.checked_add(1).unwrap_or(0)
        || target.updated_at != checked_at
        || serde_json::to_string(&target)? != envelope.initial_active_provider_json
        || sha256_hex(envelope.initial_active_provider_json.as_bytes())
            != envelope.initial_active_provider_digest
    {
        bail!("provider active-successor planned target is not exact adjacent projection");
    }
    Ok((target, root))
}

fn canonical_checked_at(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
        || parsed > Utc::now()
    {
        bail!("provider active-successor checked_at is not current canonical UTC nanoseconds");
    }
    Ok(())
}

fn sha256_hex(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(value))
}
