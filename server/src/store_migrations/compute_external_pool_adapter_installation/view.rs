use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_installation_current;
        CREATE VIEW compute_external_pool_adapter_installation_current AS
        SELECT installation.installation_receipt_id,
               installation.installation_receipt_digest,
               CASE WHEN adoption.current_status='adopted_current'
                          AND package.current_status='verified_current'
                          AND source.source_receipt_id IS NOT NULL
                          AND files.file_count=installation.entry_count
                          AND installation.entry_count=json_array_length(installation.receipt_json,
                               '$.installation.binding.installed_files')
                          AND files.total_bytes=(
                              SELECT sum(json_extract(item.value,'$.size_bytes'))
                                FROM json_each(installation.receipt_json,
                                     '$.installation.binding.installed_files') item)
                    THEN 'installed_upstreams_current' ELSE 'historical_only' END AS current_status,
               COALESCE(adoption.current_status,'not_current') AS adoption_status,
               COALESCE(package.current_status,'not_current') AS package_status,
               CASE WHEN source.source_receipt_id IS NULL THEN 'not_exact'
                    ELSE 'exact' END AS source_status,
               CASE WHEN files.file_count=installation.entry_count
                          AND installation.entry_count=json_array_length(installation.receipt_json,
                                '$.installation.binding.installed_files')
                          AND files.total_bytes=(
                              SELECT sum(json_extract(item.value,'$.size_bytes'))
                                FROM json_each(installation.receipt_json,
                                     '$.installation.binding.installed_files') item)
                    THEN 'exact' ELSE 'not_exact' END AS file_inventory_status
          FROM compute_external_pool_adapter_installation_receipts installation
          LEFT JOIN compute_external_pool_adapter_adoption_current adoption
            ON adoption.adoption_receipt_id=installation.adoption_receipt_id
           AND adoption.adoption_receipt_digest=installation.adoption_receipt_digest
          LEFT JOIN compute_external_pool_adapter_artifact_package_current package
            ON package.package_receipt_id=installation.package_receipt_id
           AND package.package_receipt_digest=installation.package_receipt_digest
          LEFT JOIN compute_external_pool_adapter_artifact_source_receipts source
            ON source.source_receipt_id=installation.source_receipt_id
           AND source.source_receipt_digest=installation.source_receipt_digest
          LEFT JOIN (
                SELECT installation_receipt_id, count(*) AS file_count,
                       sum(size_bytes) AS total_bytes
                  FROM compute_external_pool_adapter_installation_files
                 GROUP BY installation_receipt_id
          ) files ON files.installation_receipt_id=installation.installation_receipt_id;
        "#,
    )?;
    Ok(())
}
