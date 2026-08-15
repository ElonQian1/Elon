use anyhow::Result;

use super::*;

pub(crate) fn build_external_pool_adapter_provider_runtime_readiness_receipt(
    readiness_receipt_id: String,
    readiness: ExternalPoolAdapterProviderRuntimeReadinessMaterial,
) -> Result<ExternalPoolAdapterProviderRuntimeReadinessReceipt> {
    validate_provider_runtime_readiness_material(&readiness)?;
    let readiness_material_digest = provider_runtime_readiness_material_digest(&readiness)?;
    let mut receipt = ExternalPoolAdapterProviderRuntimeReadinessReceipt {
        schema: PROVIDER_RUNTIME_READINESS_RECEIPT_SCHEMA.into(),
        readiness_receipt_id,
        readiness_receipt_digest: String::new(),
        readiness_material_digest,
        canonicalization: PROVIDER_RUNTIME_READINESS_CANONICALIZATION.into(),
        digest_algorithm: PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM.into(),
        readiness,
    };
    receipt.readiness_receipt_digest =
        canonical_provider_runtime_readiness_receipt_json_and_digest(&receipt)?.1;
    validate_provider_runtime_readiness_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn build_external_pool_adapter_provider_runtime_readiness_revocation_receipt(
    revocation_receipt_id: String,
    revocation: ExternalPoolAdapterProviderRuntimeReadinessRevocationMaterial,
) -> Result<ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt> {
    validate_provider_runtime_readiness_revocation_material(&revocation)?;
    let revocation_material_digest =
        provider_runtime_readiness_revocation_material_digest(&revocation)?;
    let mut receipt = ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt {
        schema: PROVIDER_RUNTIME_READINESS_REVOCATION_RECEIPT_SCHEMA.into(),
        revocation_receipt_id,
        revocation_receipt_digest: String::new(),
        revocation_material_digest,
        canonicalization: PROVIDER_RUNTIME_READINESS_CANONICALIZATION.into(),
        digest_algorithm: PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM.into(),
        revocation,
    };
    receipt.revocation_receipt_digest =
        canonical_provider_runtime_readiness_revocation_json_and_digest(&receipt)?.1;
    validate_provider_runtime_readiness_revocation_receipt(&receipt)?;
    Ok(receipt)
}
