use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};

use crate::{
    compute_federation::external_pool_adapter_installation::{
        ExternalPoolAdapterInstallationBinding, ExternalPoolAdapterInstallationTarget,
    },
    store::{
        compute_external_pool_adapter_adoption::{
            current_external_pool_adapter_adoption_authority_on,
            external_pool_adapter_adoption_receipt_authority_on,
        },
        compute_external_pool_adapter_artifact_package::current_artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        Store,
    },
};

use super::{read::*, types::is_sha256};

impl Store {
    pub(crate) fn external_pool_adapter_installation_target(
        &self,
        adoption_receipt_id: &str,
        expected_adoption_receipt_digest: &str,
        expected_package_receipt_digest: &str,
        expected_source_receipt_digest: &str,
    ) -> Result<Option<ExternalPoolAdapterInstallationTarget>> {
        validate_target_input(
            adoption_receipt_id,
            expected_adoption_receipt_digest,
            expected_package_receipt_digest,
            expected_source_receipt_digest,
        )?;
        let connection = self.conn()?;
        if external_pool_adapter_adoption_receipt_authority_on(
            &connection,
            adoption_receipt_id,
            expected_adoption_receipt_digest,
        )?
        .is_none()
        {
            return Ok(None);
        }
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let adoption = current_external_pool_adapter_adoption_authority_on(
            &connection,
            adoption_receipt_id,
            expected_adoption_receipt_digest,
            &checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("Adapter adoption became missing during target read"))?;
        let binding = &adoption.receipt().adoption.binding;
        let package = current_artifact_package_authority_on(
            &connection,
            &binding.admission_id,
            expected_package_receipt_digest,
        )?
        .ok_or_else(|| anyhow::anyhow!("current Adapter package was not found"))?;
        let source = external_pool_adapter_artifact_source_authority_on(
            &connection,
            &binding.admission_id,
            &binding.admission_digest,
            expected_source_receipt_digest,
        )?
        .ok_or_else(|| anyhow::anyhow!("exact Adapter source was not found"))?;
        let package_receipt = package.receipt();
        let package_item = &package_receipt.package;
        if adoption.checked_at() != checked_at
            || package_receipt.package_receipt_digest != expected_package_receipt_digest
            || package_item.admission_id != binding.admission_id
            || package_item.admission_digest != binding.admission_digest
            || package_item.manifest.adapter_id != binding.adapter_id
            || package_item.manifest.release_version != binding.adapter_release_version
            || package_item.manifest.capability_set_digest != binding.capability_set_digest
            || package_item.archive_sha256 != binding.declared_implementation_sha256
            || package_item.source_receipt_digest != expected_source_receipt_digest
            || source.source_receipt_digest() != expected_source_receipt_digest
            || source.admission_id() != binding.admission_id
            || source.admission_digest() != binding.admission_digest
            || source.adapter_id() != binding.adapter_id
            || source.release_version() != binding.adapter_release_version
            || source.artifact_sha256() != package_item.archive_sha256
            || source.artifact_size_bytes() != package_item.archive_size_bytes
        {
            bail!("Adapter installation target authorities drifted");
        }
        Ok(Some(ExternalPoolAdapterInstallationTarget {
            adoption_receipt: adoption.receipt().clone(),
            package_receipt: package_receipt.clone(),
            source_receipt_id: source.source_receipt_id().to_string(),
            source_receipt_digest: source.source_receipt_digest().to_string(),
        }))
    }

    pub(crate) fn external_pool_adapter_installation_audit_target(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ExternalPoolAdapterInstallationBinding>> {
        let connection = self.conn()?;
        Ok(receipt_by_id_on(&connection, receipt_id)?
            .map(|stored| stored.receipt.installation.binding))
    }

    pub(crate) fn external_pool_adapter_installation_replay_target(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<ExternalPoolAdapterInstallationBinding>> {
        if idempotency_scope.is_empty()
            || idempotency_scope != idempotency_scope.trim()
            || idempotency_scope.len() > 240
            || idempotency_key.is_empty()
            || idempotency_key != idempotency_key.trim()
            || idempotency_key.len() > 240
        {
            bail!("Adapter installation replay target idempotency is invalid");
        }
        let connection = self.conn()?;
        Ok(
            receipt_by_idempotency_on(&connection, idempotency_scope, idempotency_key)?
                .map(|stored| stored.receipt.installation.binding),
        )
    }
}

fn validate_target_input(
    adoption_receipt_id: &str,
    adoption_digest: &str,
    package_digest: &str,
    source_digest: &str,
) -> Result<()> {
    if adoption_receipt_id.is_empty()
        || adoption_receipt_id != adoption_receipt_id.trim()
        || adoption_receipt_id.len() > 200
    {
        bail!("Adapter installation target adoption receipt id is invalid");
    }
    for (label, value) in [
        ("adoption receipt digest", adoption_digest),
        ("package receipt digest", package_digest),
        ("source receipt digest", source_digest),
    ] {
        if !is_sha256(value) {
            bail!("Adapter installation target {label} is invalid");
        }
    }
    Ok(())
}
