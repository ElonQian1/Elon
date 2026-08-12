use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_registry::*,
        external_pool_adapter_release::{
            COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND,
            COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
        },
    },
    store::{
        compute_external_pool_adapter_artifact_package::current_artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_installation::{
            current_external_pool_adapter_installation_authority_on,
            external_pool_adapter_installation_receipt_authority_on,
        },
        compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
        new_id, Store,
    },
};

use super::{persistence::*, projection::route_adapter_projection_id, read::*, types::*};

impl Store {
    pub(crate) fn register_external_pool_adapter_installed_instance(
        &self,
        input: RegisterExternalPoolAdapterInstalledInstance,
    ) -> Result<ExternalPoolAdapterRegistryWriteReceipt> {
        validate_input(&input)?;
        let RegisterExternalPoolAdapterInstalledInstance {
            prepared,
            expected_installation_receipt_id,
            expected_installation_receipt_digest,
            bound_by_admin_user_id,
            idempotency_scope,
            idempotency_key,
            confirmation,
        } = input;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(binding) = binding_by_idempotency_on(&tx, &idempotency_scope, &idempotency_key)?
        {
            ensure_replay(
                &tx,
                &binding,
                &prepared,
                &expected_installation_receipt_id,
                &expected_installation_receipt_digest,
                &bound_by_admin_user_id,
                &confirmation,
                &idempotency_scope,
                &idempotency_key,
            )?;
            let release = release_by_id_on(&tx, &binding.receipt.binding.registry_release_id)?
                .ok_or_else(|| anyhow::anyhow!("registry replay lost release"))?;
            let output = output(&release, &binding, true);
            tx.commit()?;
            return Ok(output);
        }
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let authority = current_external_pool_adapter_installation_authority_on(
            &tx,
            &expected_installation_receipt_id,
            &expected_installation_receipt_digest,
            &checked_at,
            prepared,
        )?
        .ok_or_else(|| anyhow::anyhow!("current exact Adapter installation was not found"))?;
        let release = get_or_create_release(&tx, &authority, &checked_at)?;
        let binding = create_binding(
            &tx,
            &release,
            &authority,
            &bound_by_admin_user_id,
            &confirmation,
            &idempotency_scope,
            &idempotency_key,
            &checked_at,
        )?;
        let output = output(&release, &binding, false);
        tx.commit()?;
        Ok(output)
    }
}

fn get_or_create_release(
    tx: &Transaction<'_>,
    installation: &crate::store::compute_external_pool_adapter_installation::CurrentExternalPoolAdapterInstallationAuthority,
    now: &str,
) -> Result<StoredRegistryRelease> {
    let root = installation.receipt();
    let b = &root.installation.binding;
    let admission = current_external_pool_adapter_release_admission_authority_on(
        tx,
        &b.admission_id,
        &b.admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry release lost current admission"))?;
    let package =
        current_artifact_package_authority_on(tx, &b.admission_id, &b.package_receipt_digest)?
            .ok_or_else(|| anyhow::anyhow!("registry release lost current package"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        tx,
        &b.admission_id,
        &b.admission_digest,
        &b.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry release lost source"))?;
    let p = package.receipt();
    let item = &p.package;
    let manifest = &item.manifest;
    if admission.declared_implementation_sha256() != b.declared_implementation_sha256
        || admission.declared_implementation_sha256() != b.archive_sha256
        || admission.supported_capabilities() != manifest.supported_capabilities.as_slice()
        || admission.capability_set_digest() != b.capability_set_digest
        || manifest.capability_set_digest != b.capability_set_digest
        || admission.expected_credential_verifier() != &manifest.credential_verifier
        || p.package_receipt_id != b.package_receipt_id
        || p.package_receipt_digest != b.package_receipt_digest
        || p.package_material_digest != b.package_material_digest
        || item.admission_id != b.admission_id
        || item.admission_digest != b.admission_digest
        || item.source_receipt_digest != b.source_receipt_digest
        || item.archive_sha256 != b.archive_sha256
        || item.archive_size_bytes != b.archive_size_bytes
        || item.manifest_digest != b.manifest_digest
        || item.entry_inventory_digest != b.entry_inventory_digest
        || item.entry_count != b.entry_count
        || item.total_uncompressed_bytes != b.total_uncompressed_bytes
        || manifest.adapter_id != b.adapter_id
        || manifest.release_version != b.adapter_release_version
        || source.source_receipt_id() != b.source_receipt_id
        || source.source_receipt_digest() != b.source_receipt_digest
        || source.admission_id() != b.admission_id
        || source.admission_digest() != b.admission_digest
        || source.adapter_id() != b.adapter_id
        || source.release_version() != b.adapter_release_version
        || source.artifact_sha256() != b.archive_sha256
        || source.artifact_size_bytes() != b.archive_size_bytes
        || installation.prepared().installation_content_digest() != b.installation_content_digest
    {
        bail!("registry release exact roots drifted");
    }
    let material = ExternalPoolAdapterRegistryReleaseMaterial {
        admission_id: b.admission_id.clone(),
        admission_digest: b.admission_digest.clone(),
        package_receipt_id: b.package_receipt_id.clone(),
        package_receipt_digest: b.package_receipt_digest.clone(),
        package_material_digest: b.package_material_digest.clone(),
        source_receipt_id: b.source_receipt_id.clone(),
        source_receipt_digest: b.source_receipt_digest.clone(),
        adapter_id: b.adapter_id.clone(),
        release_version: b.adapter_release_version.clone(),
        route_kind: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND.to_string(),
        supported_provider_kinds: vec![
            COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND.to_string()
        ],
        implementation_digest: b.archive_sha256.clone(),
        declared_implementation_sha256: b.declared_implementation_sha256.clone(),
        supported_capabilities: manifest.supported_capabilities.clone(),
        capability_set_digest: b.capability_set_digest.clone(),
        credential_verifier: manifest.credential_verifier.clone(),
        credential_verifier_digest: manifest.credential_verifier.verifier_digest.clone(),
        archive_sha256: b.archive_sha256.clone(),
        archive_size_bytes: b.archive_size_bytes,
        manifest: manifest.clone(),
        manifest_digest: b.manifest_digest.clone(),
        entry_inventory_digest: b.entry_inventory_digest.clone(),
        entry_count: b.entry_count,
        total_uncompressed_bytes: b.total_uncompressed_bytes,
        installation_content_digest: b.installation_content_digest.clone(),
        registered_at: now.to_string(),
        recorded_at: now.to_string(),
        registry_effect: REGISTRY_RELEASE_EFFECT.to_string(),
        provider_effect: REGISTRY_NO_EFFECT.to_string(),
        credential_effect: REGISTRY_NO_EFFECT.to_string(),
        route_effect: REGISTRY_NO_EFFECT.to_string(),
        execution_effect: REGISTRY_NO_EFFECT.to_string(),
        settlement_effect: REGISTRY_NO_EFFECT.to_string(),
    };
    if let Some(existing) =
        release_by_adapter_version_on(tx, &b.adapter_id, &b.adapter_release_version)?
    {
        // Registration time belongs to the first immutable neutral receipt; a later Provider may
        // reuse it only when every provider-neutral identity field remains byte-for-byte exact.
        let mut expected = existing.receipt.release.clone();
        expected.registered_at = material.registered_at.clone();
        expected.recorded_at = material.recorded_at.clone();
        if expected != material {
            bail!("global Adapter release identity conflicts with immutable registry history");
        }
        return Ok(existing);
    }
    let mut receipt = ExternalPoolAdapterRegistryReleaseReceipt {
        schema: REGISTRY_RELEASE_RECEIPT_SCHEMA.to_string(),
        registry_release_id: new_id("external_pool_adapter_registry_release"),
        registry_release_digest: String::new(),
        registry_release_material_digest: registry_release_material_digest(&material)?,
        canonicalization: REGISTRY_CANONICALIZATION.to_string(),
        digest_algorithm: REGISTRY_DIGEST_ALGORITHM.to_string(),
        release: material,
    };
    receipt.registry_release_digest =
        canonical_registry_release_receipt_json_and_digest(&receipt)?.1;
    validate_registry_release_receipt(&receipt)?;
    insert_release(tx, &receipt)?;
    release_by_id_on(tx, &receipt.registry_release_id)?
        .ok_or_else(|| anyhow::anyhow!("registry release disappeared"))
}

fn create_binding(
    tx: &Transaction<'_>,
    release: &StoredRegistryRelease,
    installation: &crate::store::compute_external_pool_adapter_installation::CurrentExternalPoolAdapterInstallationAuthority,
    actor: &str,
    confirmation: &str,
    scope: &str,
    key: &str,
    now: &str,
) -> Result<StoredRegistryProviderBinding> {
    let receipt = installation.receipt();
    let b = &receipt.installation.binding;
    let adoption = crate::store::compute_external_pool_adapter_adoption::external_pool_adapter_adoption_receipt_authority_on(
        tx,
        &b.adoption_receipt_id,
        &b.adoption_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registry binding lost adoption history"))?;
    let a = &adoption.receipt().adoption.binding;
    let route_adapter_projection_id = route_adapter_projection_id(
        &b.provider_id,
        b.provider_policy_revision,
        &b.provider_digest,
        &release.receipt.registry_release_id,
        &release.receipt.registry_release_digest,
        &receipt.installation_receipt_id,
        &receipt.installation_receipt_digest,
    )?;
    let material = ExternalPoolAdapterRegistryProviderBindingMaterial {
        registry_release_id: release.receipt.registry_release_id.clone(),
        registry_release_digest: release.receipt.registry_release_digest.clone(),
        route_adapter_projection_id,
        installation_receipt_id: receipt.installation_receipt_id.clone(),
        installation_receipt_digest: receipt.installation_receipt_digest.clone(),
        installation_material_digest: receipt.installation_material_digest.clone(),
        installation_content_digest: b.installation_content_digest.clone(),
        application_id: b.application_id.clone(),
        application_digest: b.application_digest.clone(),
        adoption_receipt_id: b.adoption_receipt_id.clone(),
        adoption_receipt_digest: b.adoption_receipt_digest.clone(),
        adoption_material_digest: b.adoption_material_digest.clone(),
        provider_id: b.provider_id.clone(),
        provider_owner_account_id: b.provider_owner_account_id.clone(),
        provider_policy_revision: b.provider_policy_revision,
        provider_digest: b.provider_digest.clone(),
        adapter_id: b.adapter_id.clone(),
        release_version: b.adapter_release_version.clone(),
        adapter_config_revision: b.adapter_config_revision,
        adapter_config_digest: b.adapter_config_digest.clone(),
        admission_id: b.admission_id.clone(),
        admission_digest: b.admission_digest.clone(),
        package_receipt_id: b.package_receipt_id.clone(),
        package_receipt_digest: b.package_receipt_digest.clone(),
        package_material_digest: b.package_material_digest.clone(),
        source_receipt_id: b.source_receipt_id.clone(),
        source_receipt_digest: b.source_receipt_digest.clone(),
        sandbox_conformance_receipt_id: a.sandbox_conformance_receipt_id.clone(),
        sandbox_conformance_receipt_digest: a.sandbox_conformance_receipt_digest.clone(),
        credential_verification_receipt_id: a.credential_verification_receipt_id.clone(),
        credential_verification_receipt_digest: a.credential_verification_receipt_digest.clone(),
        credential_locator_commitment: a.credential_locator_commitment.clone(),
        bound_by_admin_user_id: actor.to_string(),
        confirmation: confirmation.to_string(),
        checked_at: now.to_string(),
        bound_at: now.to_string(),
        recorded_at: now.to_string(),
        idempotency_scope: scope.to_string(),
        idempotency_key: key.to_string(),
        registry_effect: REGISTRY_BINDING_EFFECT.to_string(),
        provider_effect: REGISTRY_NO_EFFECT.to_string(),
        credential_effect: REGISTRY_NO_EFFECT.to_string(),
        route_effect: REGISTRY_NO_EFFECT.to_string(),
        execution_effect: REGISTRY_NO_EFFECT.to_string(),
        settlement_effect: REGISTRY_NO_EFFECT.to_string(),
    };
    let mut receipt = ExternalPoolAdapterRegistryProviderBindingReceipt {
        schema: REGISTRY_PROVIDER_BINDING_RECEIPT_SCHEMA.to_string(),
        provider_binding_id: new_id("external_pool_adapter_registry_binding"),
        provider_binding_digest: String::new(),
        provider_binding_material_digest: registry_provider_binding_material_digest(&material)?,
        canonicalization: REGISTRY_CANONICALIZATION.to_string(),
        digest_algorithm: REGISTRY_DIGEST_ALGORITHM.to_string(),
        binding: material,
    };
    receipt.provider_binding_digest =
        canonical_registry_provider_binding_receipt_json_and_digest(&receipt)?.1;
    validate_registry_provider_binding_receipt(&receipt)?;
    insert_binding(tx, &receipt)?;
    binding_by_id_on(tx, &receipt.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("registry binding disappeared"))
}

fn ensure_replay(
    tx: &Transaction<'_>,
    stored: &StoredRegistryProviderBinding,
    prepared: &crate::compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    expected_installation_receipt_id: &str,
    expected_installation_receipt_digest: &str,
    actor: &str,
    confirmation: &str,
    scope: &str,
    key: &str,
) -> Result<()> {
    let item = &stored.receipt.binding;
    let historical = external_pool_adapter_installation_receipt_authority_on(
        tx,
        expected_installation_receipt_id,
        expected_installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter registry replay lost installation history"))?;
    let receipt = historical.receipt();
    if prepared.binding() != &receipt.installation.binding
        || item.installation_receipt_id != receipt.installation_receipt_id
        || item.installation_receipt_digest != receipt.installation_receipt_digest
        || item.installation_material_digest != receipt.installation_material_digest
        || item.installation_content_digest != prepared.installation_content_digest()
        || item.bound_by_admin_user_id != actor
        || item.confirmation != confirmation
        || item.idempotency_scope != scope
        || item.idempotency_key != key
    {
        bail!("Adapter registry idempotency replay conflicts with immutable history");
    }
    Ok(())
}

fn output(
    r: &StoredRegistryRelease,
    b: &StoredRegistryProviderBinding,
    replayed: bool,
) -> ExternalPoolAdapterRegistryWriteReceipt {
    ExternalPoolAdapterRegistryWriteReceipt {
        release: r.summary(),
        binding: b.summary(),
        replayed,
    }
}
fn validate_input(input: &RegisterExternalPoolAdapterInstalledInstance) -> Result<()> {
    if input.confirmation != REGISTRY_BINDING_CONFIRMATION {
        bail!("Adapter registry confirmation is invalid");
    }
    for (v, n) in [
        (&input.expected_installation_receipt_id, 200),
        (&input.bound_by_admin_user_id, 200),
        (&input.idempotency_scope, 240),
        (&input.idempotency_key, 240),
    ] {
        if v.is_empty() || v.trim() != v || v.chars().count() > n || v.chars().any(char::is_control)
        {
            bail!("Adapter registry input identifier is invalid");
        }
    }
    if input.expected_installation_receipt_digest.len() != 64
        || !input
            .expected_installation_receipt_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("Adapter registry installation digest is invalid");
    }
    Ok(())
}
