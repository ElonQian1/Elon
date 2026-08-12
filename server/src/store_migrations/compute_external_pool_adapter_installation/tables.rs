use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_installation_receipts (
            installation_receipt_id TEXT PRIMARY KEY NOT NULL CHECK(
                length(trim(installation_receipt_id)) BETWEEN 1 AND 200),
            installation_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(installation_receipt_digest)=64
                AND installation_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            installation_receipt_schema TEXT NOT NULL CHECK(installation_receipt_schema=
                'compute_federation.external_pool_adapter_installation_receipt.v1'),
            receipt_json TEXT NOT NULL CHECK(
                json_valid(receipt_json) AND json_type(receipt_json)='object'
                AND length(CAST(receipt_json AS BLOB))<=1048576),
            installation_material_digest TEXT NOT NULL CHECK(
                length(installation_material_digest)=64
                AND installation_material_digest NOT GLOB '*[^0-9a-f]*'),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            adoption_receipt_id TEXT NOT NULL UNIQUE,
            adoption_receipt_digest TEXT NOT NULL CHECK(length(adoption_receipt_digest)=64
                AND adoption_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            adoption_material_digest TEXT NOT NULL CHECK(length(adoption_material_digest)=64
                AND adoption_material_digest NOT GLOB '*[^0-9a-f]*'),
            application_id TEXT NOT NULL,
            application_digest TEXT NOT NULL CHECK(length(application_digest)=64
                AND application_digest NOT GLOB '*[^0-9a-f]*'),
            provider_id TEXT NOT NULL,
            provider_owner_account_id TEXT NOT NULL,
            provider_policy_revision INTEGER NOT NULL CHECK(
                typeof(provider_policy_revision)='integer' AND provider_policy_revision>0),
            provider_digest TEXT NOT NULL CHECK(length(provider_digest)=64
                AND provider_digest NOT GLOB '*[^0-9a-f]*'),
            admission_id TEXT NOT NULL,
            admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64
                AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 200),
            adapter_release_version TEXT NOT NULL CHECK(
                length(trim(adapter_release_version)) BETWEEN 1 AND 200),
            adapter_config_revision INTEGER NOT NULL CHECK(
                typeof(adapter_config_revision)='integer' AND adapter_config_revision>0),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512),
            declared_implementation_sha256 TEXT NOT NULL CHECK(
                length(declared_implementation_sha256)=64
                AND declared_implementation_sha256 NOT GLOB '*[^0-9a-f]*'),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
            credential_locator_commitment TEXT NOT NULL CHECK(
                length(credential_locator_commitment)=64
                AND credential_locator_commitment NOT GLOB '*[^0-9a-f]*'),
            package_receipt_id TEXT NOT NULL,
            package_receipt_digest TEXT NOT NULL CHECK(length(package_receipt_digest)=64
                AND package_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            package_material_digest TEXT NOT NULL CHECK(length(package_material_digest)=64
                AND package_material_digest NOT GLOB '*[^0-9a-f]*'),
            source_receipt_id TEXT NOT NULL,
            source_receipt_digest TEXT NOT NULL CHECK(length(source_receipt_digest)=64
                AND source_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            archive_sha256 TEXT NOT NULL CHECK(length(archive_sha256)=64
                AND archive_sha256 NOT GLOB '*[^0-9a-f]*'),
            archive_size_bytes INTEGER NOT NULL CHECK(typeof(archive_size_bytes)='integer'
                AND archive_size_bytes BETWEEN 1 AND 33554432),
            manifest_digest TEXT NOT NULL CHECK(length(manifest_digest)=64
                AND manifest_digest NOT GLOB '*[^0-9a-f]*'),
            entry_inventory_digest TEXT NOT NULL CHECK(length(entry_inventory_digest)=64
                AND entry_inventory_digest NOT GLOB '*[^0-9a-f]*'),
            entry_count INTEGER NOT NULL CHECK(typeof(entry_count)='integer'
                AND entry_count BETWEEN 1 AND 128),
            total_uncompressed_bytes INTEGER NOT NULL CHECK(
                typeof(total_uncompressed_bytes)='integer'
                AND total_uncompressed_bytes BETWEEN 1 AND 67108864),
            runtime_kind TEXT NOT NULL CHECK(runtime_kind='server_sidecar_v1'),
            entrypoint_path TEXT NOT NULL CHECK(
                length(entrypoint_path) BETWEEN 1 AND 160 AND entrypoint_path=trim(entrypoint_path)),
            entrypoint_sha256 TEXT NOT NULL CHECK(length(entrypoint_sha256)=64
                AND entrypoint_sha256 NOT GLOB '*[^0-9a-f]*'),
            entrypoint_size_bytes INTEGER NOT NULL CHECK(
                typeof(entrypoint_size_bytes)='integer'
                AND entrypoint_size_bytes BETWEEN 1 AND 33554432),
            installation_content_digest TEXT NOT NULL CHECK(
                length(installation_content_digest)=64
                AND installation_content_digest NOT GLOB '*[^0-9a-f]*'),
            storage_namespace TEXT NOT NULL CHECK(storage_namespace=
                'compute-federation/external-pool-adapter-artifacts/v1/installed-inert/sha256'),
            installed_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(installed_by_admin_user_id)) BETWEEN 1 AND 200),
            confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_installation'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 240),
            idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 240),
            installed_at TEXT NOT NULL CHECK(installed_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(installed_at)=30
                AND substr(installed_at,20,1)='.' AND substr(installed_at,30,1)='Z'
                AND julianday(installed_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=installed_at),
            installation_effect TEXT NOT NULL CHECK(
                installation_effect='adapter_bytes_installed_inert'),
            credential_effect TEXT NOT NULL CHECK(credential_effect='none'),
            provider_effect TEXT NOT NULL CHECK(provider_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
            settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
            UNIQUE(idempotency_scope,idempotency_key),
            FOREIGN KEY(adoption_receipt_id)
                REFERENCES compute_external_pool_adapter_adoption_receipts(adoption_receipt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(package_receipt_id)
                REFERENCES compute_external_pool_adapter_artifact_package_receipts(package_receipt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(source_receipt_id)
                REFERENCES compute_external_pool_adapter_artifact_source_receipts(source_receipt_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_installation_files (
            installation_receipt_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK(typeof(ordinal)='integer' AND ordinal BETWEEN 0 AND 127),
            path TEXT NOT NULL CHECK(length(path) BETWEEN 1 AND 160 AND path=trim(path)),
            sha256 TEXT NOT NULL CHECK(length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
            size_bytes INTEGER NOT NULL CHECK(typeof(size_bytes)='integer'
                AND size_bytes BETWEEN 1 AND 33554432),
            role TEXT NOT NULL CHECK(role IN ('entrypoint','resource')),
            PRIMARY KEY(installation_receipt_id,ordinal),
            UNIQUE(installation_receipt_id,path),
            FOREIGN KEY(installation_receipt_id)
                REFERENCES compute_external_pool_adapter_installation_receipts(installation_receipt_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );
        "#,
    )?;
    Ok(())
}
