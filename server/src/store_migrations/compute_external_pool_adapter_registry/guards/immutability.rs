use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_release_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_registry_releases
        BEGIN SELECT RAISE(ABORT,'Provider-neutral registry releases are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_release_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_registry_releases
        BEGIN SELECT RAISE(ABORT,'Provider-neutral registry releases are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_release_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_registry_releases
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_registry_releases old
           WHERE old.registry_release_id=NEW.registry_release_id
              OR old.registry_release_digest=NEW.registry_release_digest
              OR old.registry_release_material_digest=NEW.registry_release_material_digest
              OR (old.adapter_id=NEW.adapter_id AND old.release_version=NEW.release_version)
        )
        BEGIN SELECT RAISE(ABORT,'Provider-neutral registry release cannot replace immutable identity'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_provider_binding_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_registry_provider_bindings
        BEGIN SELECT RAISE(ABORT,'Registry Provider bindings are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_provider_binding_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_registry_provider_bindings
        BEGIN SELECT RAISE(ABORT,'Registry Provider bindings are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_registry_provider_binding_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_registry_provider_bindings
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings old
           WHERE old.provider_binding_id=NEW.provider_binding_id
              OR old.provider_binding_digest=NEW.provider_binding_digest
              OR old.provider_binding_material_digest=NEW.provider_binding_material_digest
              OR old.installation_receipt_id=NEW.installation_receipt_id
              OR old.route_adapter_projection_id=NEW.route_adapter_projection_id
              OR (old.registry_release_id=NEW.registry_release_id AND old.provider_id=NEW.provider_id)
              OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)
        )
        BEGIN SELECT RAISE(ABORT,'Registry Provider binding cannot replace immutable identity'); END;
        "#,
    )?;
    Ok(())
}
