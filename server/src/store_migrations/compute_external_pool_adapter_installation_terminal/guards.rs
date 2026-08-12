use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_terminal_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_installation_terminal_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter installation terminal receipts are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_terminal_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_installation_terminal_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter installation terminal receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_terminal_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_installation_terminal_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts old
             WHERE old.terminal_receipt_id=NEW.terminal_receipt_id
                OR old.terminal_receipt_digest=NEW.terminal_receipt_digest
                OR old.installation_receipt_id=NEW.installation_receipt_id
                OR (old.idempotency_scope=NEW.idempotency_scope
                    AND old.idempotency_key=NEW.idempotency_key)
        )
        BEGIN SELECT RAISE(ABORT,'Adapter installation terminal cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_terminal_exact_root
        BEFORE INSERT ON compute_external_pool_adapter_installation_terminal_receipts
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_installation_receipts installation
             WHERE installation.installation_receipt_id=NEW.installation_receipt_id
               AND installation.installation_receipt_digest=NEW.installation_receipt_digest
               AND installation.installed_at<=NEW.revoked_at
        )
        BEGIN SELECT RAISE(ABORT,'Adapter installation terminal requires exact prior installation root'); END;
        "#,
    )?;
    install_projection_guard(conn)?;
    Ok(())
}

fn install_projection_guard(conn: &Connection) -> Result<()> {
    let projections = [
        ("$.schema", "terminal_receipt_schema"),
        ("$.terminal_receipt_id", "terminal_receipt_id"),
        ("$.terminal_receipt_digest", "terminal_receipt_digest"),
        ("$.terminal_material_digest", "terminal_material_digest"),
        ("$.canonicalization", "canonicalization"),
        ("$.digest_algorithm", "digest_algorithm"),
        (
            "$.terminal.installation_receipt_id",
            "installation_receipt_id",
        ),
        (
            "$.terminal.installation_receipt_digest",
            "installation_receipt_digest",
        ),
        ("$.terminal.terminal_kind", "terminal_kind"),
        (
            "$.terminal.revoked_by_admin_user_id",
            "revoked_by_admin_user_id",
        ),
        ("$.terminal.reason", "reason"),
        ("$.terminal.confirmation", "confirmation"),
        ("$.terminal.idempotency_scope", "idempotency_scope"),
        ("$.terminal.idempotency_key", "idempotency_key"),
        ("$.terminal.revoked_at", "revoked_at"),
        ("$.terminal.recorded_at", "recorded_at"),
        ("$.terminal.installation_effect", "installation_effect"),
        ("$.terminal.credential_effect", "credential_effect"),
        ("$.terminal.provider_effect", "provider_effect"),
        ("$.terminal.route_effect", "route_effect"),
        ("$.terminal.execution_effect", "execution_effect"),
        ("$.terminal.settlement_effect", "settlement_effect"),
    ];
    let mismatch = projections
        .iter()
        .map(|(path, column)| {
            format!("json_extract(NEW.receipt_json,'{path}') IS NOT NEW.{column}")
        })
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS external_pool_adapter_installation_terminal_json_projection
         BEFORE INSERT ON compute_external_pool_adapter_installation_terminal_receipts
         WHEN {mismatch}
         BEGIN SELECT RAISE(ABORT,'Adapter installation terminal JSON projection mismatch'); END;"
    ))?;
    Ok(())
}
