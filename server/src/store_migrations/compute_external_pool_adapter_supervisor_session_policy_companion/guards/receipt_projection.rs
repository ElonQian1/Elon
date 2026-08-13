use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_projection(
        conn,
        "external_pool_adapter_supervisor_session_policy_companion_json_projection",
        "compute_external_pool_adapter_supervisor_session_policy_companions",
        "companion_json",
        "companion",
        72,
        &companion_fields(),
    )?;
    install_projection(
        conn,
        "external_pool_adapter_supervisor_session_policy_companion_revocation_json_projection",
        "compute_external_pool_adapter_supervisor_session_policy_companion_revocations",
        "revocation_json",
        "revocation",
        34,
        &revocation_fields(),
    )
}

#[cfg(test)]
pub(super) fn counts() -> (usize, usize) {
    (companion_fields().len(), revocation_fields().len())
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
         BEGIN SELECT RAISE(ABORT,'V259 canonical receipt JSON projection mismatch'); END;"
    ))?;
    Ok(())
}

enum Kind {
    Text,
    Integer,
    Boolean,
    NullableText,
    JsonObject,
    TopText,
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
            kind: Kind::Text,
        }
    }
    fn integer(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            kind: Kind::Integer,
        }
    }
    fn boolean(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            kind: Kind::Boolean,
        }
    }
    fn nullable(field: &'static str) -> Self {
        Self {
            field,
            column: field,
            kind: Kind::NullableText,
        }
    }
    fn json(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            kind: Kind::JsonObject,
        }
    }
    fn top(field: &'static str, column: &'static str) -> Self {
        Self {
            field,
            column,
            kind: Kind::TopText,
        }
    }

    fn mismatch(&self, json_column: &str, material: &str) -> String {
        let path = match self.kind {
            Kind::TopText => format!("$.{}", self.field),
            _ => format!("$.{material}.{}", self.field),
        };
        match self.kind {
            Kind::NullableText => format!(
                "json_type(NEW.{json_column},'{path}') IS NULL OR json_type(NEW.{json_column},'{path}') NOT IN ('text','null') OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            ),
            Kind::JsonObject => format!(
                "json_type(NEW.{json_column},'{path}') IS NOT 'object' OR json(json_extract(NEW.{json_column},'{path}')) IS NOT json(NEW.{})",
                self.column
            ),
            Kind::Integer => format!(
                "json_type(NEW.{json_column},'{path}') IS NOT 'integer' OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            ),
            Kind::Boolean => format!(
                "json_type(NEW.{json_column},'{path}') IS NOT 'false' OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
                self.column
            ),
            Kind::Text | Kind::TopText => format!(
                "json_type(NEW.{json_column},'{path}') IS NOT 'text' OR json_extract(NEW.{json_column},'{path}') IS NOT NEW.{}",
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

fn companion_fields() -> Vec<Projection> {
    let mut result = envelope(
        "companion_schema",
        "companion_id",
        "companion_digest",
        "companion_material_digest",
    );
    result.extend(
        COMPANION_TEXT
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    result.extend(
        COMPANION_INTEGERS
            .iter()
            .map(|&field| Projection::integer(field)),
    );
    result.extend(
        COMPANION_BOOLEANS
            .iter()
            .map(|&field| Projection::boolean(field)),
    );
    result.push(Projection::nullable("predecessor_companion_id"));
    result.push(Projection::nullable("predecessor_companion_digest"));
    result.push(Projection::json(
        "supervisor_session_policy",
        "supervisor_session_policy_json",
    ));
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
        REVOCATION_TEXT
            .iter()
            .map(|&field| Projection::scalar(field)),
    );
    result.extend(
        REVOCATION_BOOLEANS
            .iter()
            .map(|&field| Projection::boolean(field)),
    );
    result
}

const COMPANION_TEXT: &[&str] = &[
    "profile_id",
    "profile_digest",
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
    "provider_digest",
    "provider_status",
    "logical_adapter_id",
    "release_version",
    "adapter_config_digest",
    "implementation_digest",
    "capability_set_digest",
    "credential_verifier_digest",
    "service_actor_id",
    "launch_policy_digest",
    "process_isolation_policy_id",
    "process_isolation_policy_digest",
    "resource_policy_id",
    "resource_policy_digest",
    "network_egress_policy_id",
    "network_egress_policy_digest",
    "entrypoint_capsule_policy_id",
    "entrypoint_capsule_policy_digest",
    "target_id",
    "target_digest",
    "target_policy_digest",
    "supervisor_session_policy_digest",
    "recorded_by_actor_kind",
    "recorded_by_actor_user_id",
    "recorded_at",
    "idempotency_scope",
    "idempotency_key",
    "confirmation",
    "companion_status",
    "companion_effect",
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

const COMPANION_INTEGERS: &[&str] = &[
    "provider_policy_revision",
    "adapter_config_revision",
    "process_isolation_policy_revision",
    "resource_policy_revision",
    "network_egress_policy_revision",
    "entrypoint_capsule_policy_revision",
    "sequence",
];

const COMPANION_BOOLEANS: &[&str] = &[
    "process_spawn_ready",
    "ipc_session_ready",
    "secret_delivery_ready",
    "broker_connect_ready",
    "upstream_probe_observed",
    "runtime_launch_ready",
    "activation_ready",
];

const REVOCATION_TEXT: &[&str] = &[
    "companion_id",
    "companion_digest",
    "target_id",
    "target_digest",
    "profile_id",
    "profile_digest",
    "provider_binding_id",
    "provider_binding_digest",
    "provider_id",
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

const REVOCATION_BOOLEANS: &[&str] = &[
    "process_spawn_ready",
    "ipc_session_ready",
    "secret_delivery_ready",
    "broker_connect_ready",
    "upstream_probe_observed",
    "runtime_launch_ready",
    "activation_ready",
];
