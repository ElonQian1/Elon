use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_installation_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter installation receipts are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_installation_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter installation receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_installation_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_installation_receipts old
             WHERE old.installation_receipt_id=NEW.installation_receipt_id
                OR old.installation_receipt_digest=NEW.installation_receipt_digest
                OR old.adoption_receipt_id=NEW.adoption_receipt_id
                OR (old.idempotency_scope=NEW.idempotency_scope
                    AND old.idempotency_key=NEW.idempotency_key)
        )
        BEGIN SELECT RAISE(ABORT,'Adapter installation cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_files_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_installation_files
        BEGIN SELECT RAISE(ABORT,'Adapter installation files are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_files_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_installation_files
        BEGIN SELECT RAISE(ABORT,'Adapter installation files are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_files_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_installation_files
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_installation_receipts sealed
             WHERE sealed.installation_receipt_id=NEW.installation_receipt_id
        ) OR EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_installation_files old
             WHERE old.installation_receipt_id=NEW.installation_receipt_id
               AND (old.ordinal=NEW.ordinal OR old.path=NEW.path)
        )
        BEGIN SELECT RAISE(ABORT,'Adapter installation file cannot replace inventory'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_installation_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_adoption_receipts adoption
              JOIN compute_external_pool_adapter_artifact_package_receipts package
                ON package.package_receipt_id=NEW.package_receipt_id
               AND package.package_receipt_digest=NEW.package_receipt_digest
              JOIN compute_external_pool_adapter_artifact_source_receipts source
                ON source.source_receipt_id=NEW.source_receipt_id
               AND source.source_receipt_digest=NEW.source_receipt_digest
             WHERE adoption.adoption_receipt_id=NEW.adoption_receipt_id
               AND adoption.adoption_receipt_digest=NEW.adoption_receipt_digest
               AND adoption.adoption_material_digest=NEW.adoption_material_digest
               AND adoption.application_id=NEW.application_id
               AND adoption.application_digest=NEW.application_digest
               AND adoption.provider_id=NEW.provider_id
               AND adoption.provider_owner_account_id=NEW.provider_owner_account_id
               AND adoption.provider_policy_revision=NEW.provider_policy_revision
               AND adoption.provider_digest=NEW.provider_digest
               AND adoption.admission_id=NEW.admission_id
               AND adoption.admission_digest=NEW.admission_digest
               AND adoption.adapter_id=NEW.adapter_id
               AND adoption.adapter_release_version=NEW.adapter_release_version
               AND adoption.adapter_config_revision=NEW.adapter_config_revision
               AND adoption.adapter_config_digest=NEW.adapter_config_digest
               AND adoption.declared_implementation_sha256=NEW.declared_implementation_sha256
               AND adoption.capability_set_digest=NEW.capability_set_digest
               AND adoption.credential_locator_commitment=NEW.credential_locator_commitment
               AND package.package_material_digest=NEW.package_material_digest
               AND package.admission_id=NEW.admission_id
               AND package.admission_digest=NEW.admission_digest
               AND package.source_receipt_digest=NEW.source_receipt_digest
               AND package.archive_sha256=NEW.archive_sha256
               AND package.archive_size_bytes=NEW.archive_size_bytes
               AND package.manifest_digest=NEW.manifest_digest
               AND package.entry_inventory_digest=NEW.entry_inventory_digest
               AND package.entry_count=NEW.entry_count
               AND package.total_uncompressed_bytes=NEW.total_uncompressed_bytes
               AND package.adapter_id=NEW.adapter_id
               AND package.release_version=NEW.adapter_release_version
               AND package.runtime_kind=NEW.runtime_kind
               AND package.runtime_entrypoint=NEW.entrypoint_path
               AND package.capability_set_digest=NEW.capability_set_digest
               AND source.admission_id=NEW.admission_id
               AND source.admission_digest=NEW.admission_digest
               AND source.adapter_id=NEW.adapter_id
               AND source.release_version=NEW.adapter_release_version
               AND source.reopened_sha256=NEW.archive_sha256
               AND source.artifact_size_bytes=NEW.archive_size_bytes
        )
        BEGIN SELECT RAISE(ABORT,'Adapter installation lacks exact V244/V232/V227 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_exact_files
        BEFORE INSERT ON compute_external_pool_adapter_installation_receipts
        WHEN json_type(NEW.receipt_json,'$.installation.binding.installed_files') IS NOT 'array'
          OR json_array_length(NEW.receipt_json,
                '$.installation.binding.installed_files') NOT BETWEEN 1 AND 128
          OR NEW.entry_count<>json_array_length(NEW.receipt_json,
                '$.installation.binding.installed_files')
          OR NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_installation_files entrypoint
             WHERE entrypoint.installation_receipt_id=NEW.installation_receipt_id
               AND entrypoint.path=NEW.entrypoint_path
               AND entrypoint.sha256=NEW.entrypoint_sha256
               AND entrypoint.size_bytes=NEW.entrypoint_size_bytes
               AND entrypoint.role='entrypoint'
        ) OR (SELECT count(*) FROM compute_external_pool_adapter_installation_files file
               WHERE file.installation_receipt_id=NEW.installation_receipt_id)
                <>NEW.entry_count
          OR (SELECT count(*) FROM compute_external_pool_adapter_installation_files file
               WHERE file.installation_receipt_id=NEW.installation_receipt_id
                 AND file.role='entrypoint')<>1
          OR (SELECT count(DISTINCT file.ordinal)
                FROM compute_external_pool_adapter_installation_files file
               WHERE file.installation_receipt_id=NEW.installation_receipt_id)
                <>json_array_length(NEW.receipt_json,
                    '$.installation.binding.installed_files')
          OR COALESCE((SELECT min(file.ordinal)
                         FROM compute_external_pool_adapter_installation_files file
                        WHERE file.installation_receipt_id=NEW.installation_receipt_id),-1)<>0
          OR COALESCE((SELECT max(file.ordinal)
                         FROM compute_external_pool_adapter_installation_files file
                        WHERE file.installation_receipt_id=NEW.installation_receipt_id),-1)
                <>json_array_length(NEW.receipt_json,
                    '$.installation.binding.installed_files')-1
          OR COALESCE((SELECT sum(file.size_bytes)
                         FROM compute_external_pool_adapter_installation_files file
                        WHERE file.installation_receipt_id=NEW.installation_receipt_id),-1)
                <>COALESCE((SELECT sum(json_extract(item.value,'$.size_bytes'))
                              FROM json_each(NEW.receipt_json,
                                   '$.installation.binding.installed_files') item),-1)
          OR EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_installation_files file
                 WHERE file.installation_receipt_id=NEW.installation_receipt_id
                   AND NOT EXISTS (
                       SELECT 1 FROM json_each(NEW.receipt_json,
                            '$.installation.binding.installed_files') item
                        WHERE CAST(item.key AS INTEGER)=file.ordinal
                          AND json_extract(item.value,'$.path')=file.path
                          AND json_extract(item.value,'$.sha256')=file.sha256
                          AND json_extract(item.value,'$.size_bytes')=file.size_bytes
                          AND json_extract(item.value,'$.role')=file.role
                   )
          )
        BEGIN SELECT RAISE(ABORT,'Adapter installation file inventory is not exact'); END;
        "#,
    )?;
    install_projection_guard(conn)?;
    Ok(())
}

fn install_projection_guard(conn: &Connection) -> Result<()> {
    let projections = [
        ("$.schema", "installation_receipt_schema"),
        ("$.installation_receipt_id", "installation_receipt_id"),
        (
            "$.installation_receipt_digest",
            "installation_receipt_digest",
        ),
        (
            "$.installation_material_digest",
            "installation_material_digest",
        ),
        ("$.canonicalization", "canonicalization"),
        ("$.digest_algorithm", "digest_algorithm"),
        ("$.installation.binding.application_id", "application_id"),
        (
            "$.installation.binding.application_digest",
            "application_digest",
        ),
        ("$.installation.binding.provider_id", "provider_id"),
        (
            "$.installation.binding.provider_owner_account_id",
            "provider_owner_account_id",
        ),
        (
            "$.installation.binding.provider_policy_revision",
            "provider_policy_revision",
        ),
        ("$.installation.binding.provider_digest", "provider_digest"),
        ("$.installation.binding.admission_id", "admission_id"),
        (
            "$.installation.binding.admission_digest",
            "admission_digest",
        ),
        ("$.installation.binding.adapter_id", "adapter_id"),
        (
            "$.installation.binding.adapter_release_version",
            "adapter_release_version",
        ),
        (
            "$.installation.binding.adapter_config_revision",
            "adapter_config_revision",
        ),
        (
            "$.installation.binding.adapter_config_digest",
            "adapter_config_digest",
        ),
        (
            "$.installation.binding.declared_implementation_sha256",
            "declared_implementation_sha256",
        ),
        (
            "$.installation.binding.capability_set_digest",
            "capability_set_digest",
        ),
        (
            "$.installation.binding.credential_locator_commitment",
            "credential_locator_commitment",
        ),
        (
            "$.installation.binding.adoption_receipt_id",
            "adoption_receipt_id",
        ),
        (
            "$.installation.binding.adoption_receipt_digest",
            "adoption_receipt_digest",
        ),
        (
            "$.installation.binding.adoption_material_digest",
            "adoption_material_digest",
        ),
        (
            "$.installation.binding.package_receipt_id",
            "package_receipt_id",
        ),
        (
            "$.installation.binding.package_receipt_digest",
            "package_receipt_digest",
        ),
        (
            "$.installation.binding.package_material_digest",
            "package_material_digest",
        ),
        (
            "$.installation.binding.source_receipt_id",
            "source_receipt_id",
        ),
        (
            "$.installation.binding.source_receipt_digest",
            "source_receipt_digest",
        ),
        ("$.installation.binding.archive_sha256", "archive_sha256"),
        (
            "$.installation.binding.archive_size_bytes",
            "archive_size_bytes",
        ),
        ("$.installation.binding.manifest_digest", "manifest_digest"),
        (
            "$.installation.binding.entry_inventory_digest",
            "entry_inventory_digest",
        ),
        ("$.installation.binding.entry_count", "entry_count"),
        (
            "$.installation.binding.total_uncompressed_bytes",
            "total_uncompressed_bytes",
        ),
        ("$.installation.binding.runtime_kind", "runtime_kind"),
        ("$.installation.binding.entrypoint_path", "entrypoint_path"),
        (
            "$.installation.binding.entrypoint_sha256",
            "entrypoint_sha256",
        ),
        (
            "$.installation.binding.entrypoint_size_bytes",
            "entrypoint_size_bytes",
        ),
        (
            "$.installation.binding.installation_content_digest",
            "installation_content_digest",
        ),
        (
            "$.installation.binding.storage_namespace",
            "storage_namespace",
        ),
        (
            "$.installation.installed_by_admin_user_id",
            "installed_by_admin_user_id",
        ),
        ("$.installation.confirmation", "confirmation"),
        ("$.installation.idempotency_scope", "idempotency_scope"),
        ("$.installation.idempotency_key", "idempotency_key"),
        ("$.installation.installed_at", "installed_at"),
        ("$.installation.recorded_at", "recorded_at"),
        ("$.installation.installation_effect", "installation_effect"),
        ("$.installation.credential_effect", "credential_effect"),
        ("$.installation.provider_effect", "provider_effect"),
        ("$.installation.route_effect", "route_effect"),
        ("$.installation.execution_effect", "execution_effect"),
        ("$.installation.settlement_effect", "settlement_effect"),
    ];
    let mismatch = projections
        .iter()
        .map(|(path, column)| {
            format!("json_extract(NEW.receipt_json,'{path}') IS NOT NEW.{column}")
        })
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_json_projection
         BEFORE INSERT ON compute_external_pool_adapter_installation_receipts
         WHEN {mismatch}
         BEGIN SELECT RAISE(ABORT,'Adapter installation JSON projection mismatch'); END;"
    ))?;
    Ok(())
}
