use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_projection(
        conn,
        "external_pool_adapter_runtime_launch_profile_json_projection",
        "compute_external_pool_adapter_runtime_launch_profiles",
        "profile_json",
        "profile",
        56,
        &profile_fields(),
    )?;
    install_projection(
        conn,
        "external_pool_adapter_runtime_launch_profile_revocation_json_projection",
        "compute_external_pool_adapter_runtime_launch_profile_revocations",
        "revocation_json",
        "revocation",
        24,
        &revocation_fields(),
    )
}

#[cfg(test)]
pub(super) fn counts() -> (usize, usize) {
    (profile_fields().len(), revocation_fields().len())
}

fn install_projection(
    conn: &Connection,
    name: &str,
    table: &str,
    json_column: &str,
    material: &str,
    material_count: usize,
    fields: &[Projection],
) -> Result<()> {
    let mismatch = fields
        .iter()
        .map(|field| field.mismatch(json_column, material))
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {name} BEFORE INSERT ON {table}
         WHEN json_type(NEW.{json_column},'$.{material}') IS NOT 'object'
           OR (SELECT COUNT(*) FROM json_each(NEW.{json_column}))!=7
           OR (SELECT COUNT(*) FROM json_each(NEW.{json_column},'$.{material}'))!={material_count}
           OR {mismatch}
         BEGIN SELECT RAISE(ABORT,'V255 canonical receipt JSON projection mismatch'); END;"
    ))?;
    Ok(())
}

enum Kind {
    Scalar,
    Nullable,
    Json,
    Top,
}

struct Projection {
    field: &'static str,
    column: &'static str,
    kind: Kind,
}

impl Projection {
    fn scalar(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            kind: Kind::Scalar,
        }
    }
    fn nullable(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            kind: Kind::Nullable,
        }
    }
    fn json(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            kind: Kind::Json,
        }
    }
    fn top(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            kind: Kind::Top,
        }
    }
    fn mismatch(&self, json_column: &str, material: &str) -> String {
        let path = match self.kind {
            Kind::Top => format!("$.{}", self.field),
            _ => format!("$.{material}.{}", self.field),
        };
        match self.kind {
            Kind::Nullable => format!(
                "json_type(NEW.{json_column},'{path}') IS NULL OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            ),
            Kind::Json => format!(
                "json(json_extract(NEW.{json_column},'{path}')) IS NOT json(NEW.{})",
                self.column
            ),
            Kind::Scalar | Kind::Top => format!(
                "json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            ),
        }
    }
}

fn envelope(
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

fn profile_fields() -> Vec<Projection> {
    let mut result = envelope(
        "profile_schema",
        "profile_id",
        "profile_digest",
        "profile_material_digest",
    );
    result.extend(
        PROFILE_SCALARS
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    result.push(Projection::nullable("predecessor_profile_id"));
    result.push(Projection::nullable("predecessor_profile_digest"));
    result.push(Projection::json("launch_policy", "launch_policy_json"));
    result
}

fn revocation_fields() -> Vec<Projection> {
    let mut result = envelope(
        "revocation_schema",
        "revocation_id",
        "revocation_digest",
        "revocation_material_digest",
    );
    result.extend(
        REVOCATION_SCALARS
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    result
}

const PROFILE_SCALARS: &[&str] = &[
    "candidate_id",
    "candidate_digest",
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
    "service_actor_id",
    "entrypoint_relative_path",
    "entrypoint_path_digest",
    "credential_ref_scheme",
    "credential_locator_commitment",
    "entrypoint_sha256",
    "entrypoint_size_bytes",
    "entry_inventory_digest",
    "installed_file_count",
    "installed_total_bytes",
    "launch_policy_digest",
    "sequence",
    "recorded_by_actor_kind",
    "recorded_by_actor_user_id",
    "recorded_at",
    "idempotency_scope",
    "idempotency_key",
    "confirmation",
    "profile_status",
    "profile_effect",
    "adapter_effect",
    "runtime_effect",
    "provider_effect",
    "credential_effect",
    "route_effect",
    "execution_effect",
    "usage_effect",
    "market_effect",
    "settlement_effect",
];

const REVOCATION_SCALARS: &[&str] = &[
    "profile_id",
    "profile_digest",
    "provider_binding_id",
    "provider_binding_digest",
    "candidate_id",
    "candidate_digest",
    "revoked_by_actor_kind",
    "revoked_by_actor_user_id",
    "reason",
    "revoked_at",
    "recorded_at",
    "idempotency_scope",
    "idempotency_key",
    "confirmation",
    "revocation_effect",
    "adapter_effect",
    "runtime_effect",
    "provider_effect",
    "credential_effect",
    "route_effect",
    "execution_effect",
    "usage_effect",
    "market_effect",
    "settlement_effect",
];
