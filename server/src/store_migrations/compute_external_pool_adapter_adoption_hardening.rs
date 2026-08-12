use anyhow::{bail, Result};
use rusqlite::Connection;

pub(crate) fn reject_existing_projection_drift(conn: &Connection) -> Result<()> {
    reject_table_drift(
        conn,
        "compute_external_pool_adapter_adoption_receipts",
        ADOPTION_PROJECTIONS,
        "Adapter adoption existing JSON projection mismatch",
    )?;
    reject_table_drift(
        conn,
        "compute_external_pool_adapter_adoption_terminal_receipts",
        TERMINAL_PROJECTIONS,
        "Adapter adoption terminal existing JSON projection mismatch",
    )
}

pub(crate) fn install_projection_guards(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS external_pool_adapter_adoption_json_projection;
         DROP TRIGGER IF EXISTS external_pool_adapter_adoption_terminal_json_projection;
         CREATE TRIGGER external_pool_adapter_adoption_json_projection
         AFTER INSERT ON compute_external_pool_adapter_adoption_receipts
         WHEN {}
         BEGIN SELECT RAISE(ABORT,'Adapter adoption JSON projection mismatch'); END;
         CREATE TRIGGER external_pool_adapter_adoption_terminal_json_projection
         AFTER INSERT ON compute_external_pool_adapter_adoption_terminal_receipts
         WHEN {}
         BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal JSON projection mismatch'); END;",
        projection_mismatch_sql("NEW", ADOPTION_PROJECTIONS),
        projection_mismatch_sql("NEW", TERMINAL_PROJECTIONS),
    ))?;
    Ok(())
}

fn reject_table_drift(
    conn: &Connection,
    table: &str,
    projections: &[Projection],
    message: &str,
) -> Result<()> {
    let drifted: bool = conn.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {table} receipt WHERE {})",
            projection_mismatch_sql("receipt", projections)
        ),
        [],
        |row| row.get(0),
    )?;
    if drifted {
        bail!("{message}");
    }
    Ok(())
}

fn projection_mismatch_sql(row: &str, projections: &[Projection]) -> String {
    format!(
        "NOT ({})",
        projections
            .iter()
            .map(|projection| projection.matches(row))
            .collect::<Vec<_>>()
            .join(" AND ")
    )
}

struct Projection {
    path: &'static str,
    expected: &'static str,
    scalar: bool,
}

impl Projection {
    fn matches(&self, row: &str) -> String {
        let expected = if self.scalar {
            format!("{row}.{}", self.expected)
        } else {
            format!("'{}'", self.expected)
        };
        format!(
            "json_extract({row}.receipt_json,'{}') IS {expected}",
            self.path
        )
    }
}

macro_rules! scalar {
    ($path:literal, $column:literal) => {
        Projection {
            path: $path,
            expected: $column,
            scalar: true,
        }
    };
}

macro_rules! constant {
    ($path:literal, $expected:literal) => {
        Projection {
            path: $path,
            expected: $expected,
            scalar: false,
        }
    };
}

const ADOPTION_PROJECTIONS: &[Projection] = &[
    constant!(
        "$.schema",
        "compute_federation.external_pool_adapter_adoption_receipt.v1"
    ),
    scalar!("$.adoption_receipt_id", "adoption_receipt_id"),
    scalar!("$.adoption_receipt_digest", "adoption_receipt_digest"),
    scalar!("$.adoption_material_digest", "adoption_material_digest"),
    constant!("$.canonicalization", "rfc8785_jcs"),
    constant!("$.digest_algorithm", "sha256"),
    scalar!("$.adoption.binding.application_id", "application_id"),
    scalar!(
        "$.adoption.binding.application_digest",
        "application_digest"
    ),
    scalar!("$.adoption.binding.provider_id", "provider_id"),
    scalar!(
        "$.adoption.binding.provider_owner_account_id",
        "provider_owner_account_id"
    ),
    scalar!(
        "$.adoption.binding.provider_policy_revision",
        "provider_policy_revision"
    ),
    scalar!("$.adoption.binding.provider_digest", "provider_digest"),
    scalar!("$.adoption.binding.admission_id", "admission_id"),
    scalar!("$.adoption.binding.admission_digest", "admission_digest"),
    scalar!("$.adoption.binding.adapter_id", "adapter_id"),
    scalar!(
        "$.adoption.binding.adapter_release_version",
        "adapter_release_version"
    ),
    scalar!(
        "$.adoption.binding.adapter_config_revision",
        "adapter_config_revision"
    ),
    scalar!(
        "$.adoption.binding.adapter_config_digest",
        "adapter_config_digest"
    ),
    scalar!(
        "$.adoption.binding.declared_implementation_sha256",
        "declared_implementation_sha256"
    ),
    scalar!(
        "$.adoption.binding.capability_set_digest",
        "capability_set_digest"
    ),
    scalar!(
        "$.adoption.binding.sandbox_conformance_receipt_id",
        "sandbox_conformance_receipt_id"
    ),
    scalar!(
        "$.adoption.binding.sandbox_conformance_receipt_digest",
        "sandbox_conformance_receipt_digest"
    ),
    scalar!(
        "$.adoption.binding.sandbox_report_expires_at",
        "sandbox_report_expires_at"
    ),
    scalar!(
        "$.adoption.binding.credential_verification_receipt_id",
        "credential_verification_receipt_id"
    ),
    scalar!(
        "$.adoption.binding.credential_verification_receipt_digest",
        "credential_verification_receipt_digest"
    ),
    scalar!(
        "$.adoption.binding.credential_locator_commitment",
        "credential_locator_commitment"
    ),
    scalar!(
        "$.adoption.binding.credential_report_expires_at",
        "credential_report_expires_at"
    ),
    scalar!(
        "$.adoption.adopted_by_admin_user_id",
        "adopted_by_admin_user_id"
    ),
    scalar!("$.adoption.confirmation", "confirmation"),
    scalar!("$.adoption.idempotency_scope", "idempotency_scope"),
    scalar!("$.adoption.idempotency_key", "idempotency_key"),
    scalar!("$.adoption.adopted_at", "adopted_at"),
    scalar!("$.adoption.recorded_at", "recorded_at"),
    scalar!("$.adoption.adoption_effect", "adoption_effect"),
    scalar!("$.adoption.install_effect", "install_effect"),
    scalar!("$.adoption.provider_effect", "provider_effect"),
    scalar!("$.adoption.route_effect", "route_effect"),
    scalar!("$.adoption.execution_effect", "execution_effect"),
    scalar!("$.adoption.settlement_effect", "settlement_effect"),
];

const TERMINAL_PROJECTIONS: &[Projection] = &[
    constant!(
        "$.schema",
        "compute_federation.external_pool_adapter_adoption_terminal_receipt.v1"
    ),
    scalar!("$.terminal_receipt_id", "terminal_receipt_id"),
    scalar!("$.terminal_receipt_digest", "terminal_receipt_digest"),
    scalar!("$.terminal_material_digest", "terminal_material_digest"),
    constant!("$.canonicalization", "rfc8785_jcs"),
    constant!("$.digest_algorithm", "sha256"),
    scalar!("$.terminal.adoption_receipt_id", "adoption_receipt_id"),
    scalar!(
        "$.terminal.adoption_receipt_digest",
        "adoption_receipt_digest"
    ),
    scalar!(
        "$.terminal.revoked_by_admin_user_id",
        "revoked_by_admin_user_id"
    ),
    scalar!("$.terminal.reason", "reason"),
    scalar!("$.terminal.confirmation", "confirmation"),
    scalar!("$.terminal.idempotency_scope", "idempotency_scope"),
    scalar!("$.terminal.idempotency_key", "idempotency_key"),
    scalar!("$.terminal.revoked_at", "revoked_at"),
    scalar!("$.terminal.recorded_at", "recorded_at"),
    scalar!("$.terminal.adoption_effect", "adoption_effect"),
    scalar!("$.terminal.provider_effect", "provider_effect"),
    scalar!("$.terminal.route_effect", "route_effect"),
    scalar!("$.terminal.execution_effect", "execution_effect"),
    scalar!("$.terminal.settlement_effect", "settlement_effect"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projections_cover_every_materialized_scalar_and_signed_constant() {
        assert_eq!(ADOPTION_PROJECTIONS.len(), 39);
        assert_eq!(TERMINAL_PROJECTIONS.len(), 20);
        for (projections, required) in [
            (
                ADOPTION_PROJECTIONS,
                ["provider_owner_account_id", "credential_report_expires_at"],
            ),
            (TERMINAL_PROJECTIONS, ["confirmation", "settlement_effect"]),
        ] {
            let mismatch = projection_mismatch_sql("receipt", projections);
            for name in [
                "$.schema",
                "$.canonicalization",
                "$.digest_algorithm",
                required[0],
                required[1],
            ] {
                assert!(mismatch.contains(name), "missing projection {name}");
            }
        }
    }
}
