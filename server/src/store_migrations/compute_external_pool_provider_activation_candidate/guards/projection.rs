use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_guard(
        conn,
        "external_pool_provider_activation_delegation_json_projection",
        "compute_external_pool_provider_activation_delegations",
        "delegation_json",
        "delegation",
        34,
        &delegation_projections(),
    )?;
    install_guard(
        conn,
        "external_pool_provider_activation_candidate_json_projection",
        "compute_external_pool_provider_activation_candidates",
        "candidate_json",
        "candidate",
        39,
        &candidate_projections(),
    )?;
    install_guard(
        conn,
        "external_pool_provider_activation_revocation_json_projection",
        "compute_external_pool_provider_activation_delegation_revocations",
        "revocation_json",
        "revocation",
        21,
        &revocation_projections(),
    )
}

fn install_guard(
    conn: &Connection,
    name: &str,
    table: &str,
    json_column: &str,
    material: &str,
    material_field_count: usize,
    projections: &[Projection],
) -> Result<()> {
    let scalar_mismatch = projections
        .iter()
        .map(|projection| projection.mismatch(json_column, material))
        .collect::<Vec<_>>()
        .join("\n          OR ");
    let mismatch = format!(
        "json_type(NEW.{json_column},'$.{material}') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.{json_column}))!=7
          OR (SELECT COUNT(*) FROM json_each(NEW.{json_column},'$.{material}'))!={material_field_count}
          OR {scalar_mismatch}"
    );
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {name}
         BEFORE INSERT ON {table}
         WHEN {mismatch}
         BEGIN SELECT RAISE(ABORT,'V254 canonical JSON projection mismatch'); END;"
    ))?;
    Ok(())
}

struct Projection {
    field: &'static str,
    column: &'static str,
    top_level: bool,
    json_value: bool,
    nullable: bool,
}

impl Projection {
    fn scalar(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            top_level: false,
            json_value: false,
            nullable: false,
        }
    }
    fn top(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            top_level: true,
            json_value: false,
            nullable: false,
        }
    }
    fn json(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            top_level: false,
            json_value: true,
            nullable: false,
        }
    }
    fn nullable(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            top_level: false,
            json_value: false,
            nullable: true,
        }
    }
    fn mismatch(&self, json_column: &str, material: &str) -> String {
        let path = if self.top_level {
            format!("$.{}", self.field)
        } else {
            format!("$.{material}.{}", self.field)
        };
        if self.nullable {
            format!(
                "json_type(NEW.{json_column},'{path}') IS NULL OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            )
        } else if self.json_value {
            format!(
                "json(json_extract(NEW.{json_column},'{path}')) IS NOT json(NEW.{})",
                self.column
            )
        } else {
            format!(
                "json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            )
        }
    }
}

fn receipt_prefix(
    schema: &'static str,
    id: &'static str,
    digest: &'static str,
    material: &'static str,
) -> Vec<Projection> {
    vec![
        Projection::top("schema", schema),
        Projection::top(id, id),
        Projection::top(digest, digest),
        Projection::top(material, material),
        Projection::top("canonicalization", "canonicalization"),
        Projection::top("digest_algorithm", "digest_algorithm"),
    ]
}

fn delegation_projections() -> Vec<Projection> {
    let mut fields = receipt_prefix(
        "delegation_schema",
        "delegation_id",
        "delegation_digest",
        "delegation_material_digest",
    );
    fields.extend(
        DELEGATION_FIELDS
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    fields.push(Projection::nullable("predecessor_delegation_id"));
    fields.push(Projection::nullable("predecessor_delegation_digest"));
    fields.push(Projection::json(
        "allowed_route_kinds",
        "allowed_route_kinds_json",
    ));
    fields.push(Projection::json(
        "allowed_actor_phases",
        "allowed_actor_phases_json",
    ));
    fields
}

fn candidate_projections() -> Vec<Projection> {
    let mut fields = receipt_prefix(
        "candidate_schema",
        "candidate_id",
        "candidate_digest",
        "candidate_material_digest",
    );
    fields.extend(
        CANDIDATE_FIELDS
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    fields.push(Projection::nullable("predecessor_candidate_id"));
    fields.push(Projection::nullable("predecessor_candidate_digest"));
    fields
}

fn revocation_projections() -> Vec<Projection> {
    let mut fields = receipt_prefix(
        "revocation_schema",
        "revocation_id",
        "revocation_digest",
        "revocation_material_digest",
    );
    fields.extend(
        REVOCATION_FIELDS
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    fields
}

const DELEGATION_FIELDS: &[&str] = &[
    "provider_binding_id",
    "provider_binding_digest",
    "registry_release_id",
    "registry_release_digest",
    "route_adapter_projection_id",
    "provider_id",
    "provider_owner_account_id",
    "provider_policy_revision",
    "provider_digest",
    "provider_status",
    "logical_adapter_id",
    "release_version",
    "adapter_config_revision",
    "adapter_config_digest",
    "service_actor_id",
    "service_actor_kind",
    "issued_by_owner_user_id",
    "issued_at",
    "recorded_at",
    "sequence",
    "idempotency_scope",
    "idempotency_key",
    "confirmation",
    "delegation_effect",
    "provider_effect",
    "credential_effect",
    "route_effect",
    "execution_effect",
    "market_effect",
    "settlement_effect",
];

const CANDIDATE_FIELDS: &[&str] = &[
    "delegation_id",
    "delegation_digest",
    "provider_binding_id",
    "provider_binding_digest",
    "registry_release_id",
    "registry_release_digest",
    "installation_receipt_id",
    "installation_receipt_digest",
    "installation_content_digest",
    "route_adapter_projection_id",
    "provider_id",
    "provider_owner_account_id",
    "provider_policy_revision",
    "provider_digest",
    "provider_status",
    "logical_adapter_id",
    "release_version",
    "adapter_config_revision",
    "adapter_config_digest",
    "implementation_digest",
    "capability_set_digest",
    "credential_verifier_digest",
    "logical_adapter_binding_digest",
    "logical_projection_compatibility_digest",
    "service_actor_id",
    "sequence",
    "checked_at",
    "recorded_at",
    "candidate_status",
    "activation_closure_status",
    "candidate_effect",
    "provider_effect",
    "credential_effect",
    "route_effect",
    "execution_effect",
    "market_effect",
    "settlement_effect",
];

const REVOCATION_FIELDS: &[&str] = &[
    "delegation_id",
    "delegation_digest",
    "candidate_id",
    "candidate_digest",
    "provider_binding_id",
    "provider_binding_digest",
    "provider_id",
    "revoked_by_owner_user_id",
    "reason",
    "revoked_at",
    "recorded_at",
    "idempotency_scope",
    "idempotency_key",
    "confirmation",
    "revocation_effect",
    "provider_effect",
    "credential_effect",
    "route_effect",
    "execution_effect",
    "market_effect",
    "settlement_effect",
];
