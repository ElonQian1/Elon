use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (name, table, json_column, projections) in [
        (
            "external_pool_adapter_credential_reattestation_challenge_projection",
            "compute_external_pool_adapter_credential_reattestation_challenges",
            "challenge_json",
            challenge_projections(),
        ),
        (
            "external_pool_adapter_credential_reattestation_receipt_projection",
            "compute_external_pool_adapter_credential_reattestation_receipts",
            "receipt_json",
            receipt_projections(),
        ),
        (
            "external_pool_adapter_credential_reattestation_revocation_projection",
            "compute_external_pool_adapter_credential_reattestation_revocations",
            "receipt_json",
            revocation_projections(),
        ),
    ] {
        install_guard(conn, name, table, json_column, &projections)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn counts() -> (usize, usize, usize) {
    (
        challenge_projections().len(),
        receipt_projections().len(),
        revocation_projections().len(),
    )
}

fn install_guard(
    conn: &Connection,
    name: &str,
    table: &str,
    json_column: &str,
    projections: &[Projection],
) -> Result<()> {
    let mismatch = projections
        .iter()
        .map(|item| item.mismatch(json_column))
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {name} BEFORE INSERT ON {table}\n\
         WHEN {mismatch}\n\
         BEGIN SELECT RAISE(ABORT,'V253 canonical JSON projection mismatch'); END;"
    ))?;
    Ok(())
}

struct Projection {
    path: &'static str,
    expected: &'static str,
    kind: Kind,
}

enum Kind {
    Scalar,
    Nullable,
    Json,
    Fixed,
}

impl Projection {
    fn scalar(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            kind: Kind::Scalar,
        }
    }
    fn nullable(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            kind: Kind::Nullable,
        }
    }
    fn json(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            kind: Kind::Json,
        }
    }
    fn fixed(path: &'static str, literal: &'static str) -> Self {
        Self {
            path,
            expected: literal,
            kind: Kind::Fixed,
        }
    }
    fn mismatch(&self, json_column: &str) -> String {
        match self.kind {
            Kind::Scalar => format!(
                "json_extract(NEW.{json_column},'{}') IS NOT NEW.{}",
                self.path, self.expected
            ),
            Kind::Nullable => format!(
                "json_type(NEW.{json_column},'{}') IS NULL OR json_extract(NEW.{json_column},'{}') IS NOT NEW.{}",
                self.path, self.path, self.expected
            ),
            Kind::Json => format!(
                "json(json_extract(NEW.{json_column},'{}')) IS NOT json(NEW.{})",
                self.path, self.expected
            ),
            Kind::Fixed => format!(
                "json_extract(NEW.{json_column},'{}') IS NOT {}",
                self.path, self.expected
            ),
        }
    }
}

fn challenge_projections() -> Vec<Projection> {
    let mut items = fixed_envelope("challenge");
    items.push(Projection::fixed(
        "$.signature_algorithm",
        "'rsa-pkcs1v15-sha256'",
    ));
    for (field, column) in [
        ("challenge_id", "challenge_id"),
        ("challenge_nonce_base64", "challenge_nonce_base64"),
        ("challenge_nonce_digest", "challenge_nonce_digest"),
        ("provider_binding_id", "provider_binding_id"),
        ("provider_binding_digest", "provider_binding_digest"),
        (
            "provider_binding_material_digest",
            "provider_binding_material_digest",
        ),
        ("registry_release_id", "registry_release_id"),
        ("registry_release_digest", "registry_release_digest"),
        (
            "registry_release_material_digest",
            "registry_release_material_digest",
        ),
        (
            "credential_verifier_key_record_id",
            "credential_verifier_key_record_id",
        ),
        (
            "credential_verifier_key_record_digest",
            "credential_verifier_key_record_digest",
        ),
        ("credential_verifier_key_id", "credential_verifier_key_id"),
        (
            "observed_provider_policy_revision",
            "observed_provider_policy_revision",
        ),
        ("observed_provider_digest", "observed_provider_digest"),
        ("observed_provider_status", "observed_provider_status"),
        ("sequence", "sequence"),
    ] {
        items.push(Projection::scalar(path("$.binding", field), column));
    }
    for (field, column) in [
        ("predecessor_receipt_id", "predecessor_receipt_id"),
        ("predecessor_receipt_digest", "predecessor_receipt_digest"),
    ] {
        items.push(Projection::nullable(path("$.binding", field), column));
    }
    for (path, column) in [
        ("$.signature_message_base64", "signature_message_base64"),
        ("$.signature_message_digest", "signature_message_digest"),
        ("$.binding.challenge_issued_at", "issued_at"),
        ("$.binding.challenge_expires_at", "expires_at"),
    ] {
        items.push(Projection::scalar(path, column));
    }
    items
}

fn receipt_projections() -> Vec<Projection> {
    let prefix = "$.reattestation.binding";
    let mut items = fixed_envelope("receipt");
    for (path, column) in [
        ("$.reattestation_receipt_id", "reattestation_receipt_id"),
        (
            "$.reattestation_receipt_digest",
            "reattestation_receipt_digest",
        ),
        (
            "$.reattestation_material_digest",
            "reattestation_material_digest",
        ),
    ] {
        items.push(Projection::scalar(path, column));
    }
    for (field, column) in receipt_binding_fields() {
        items.push(Projection::scalar(path(prefix, field), column));
    }
    items.push(Projection::json(
        "$.reattestation.binding.expected_credential_verifier",
        "expected_credential_verifier_json",
    ));
    for (field, column) in [
        ("predecessor_receipt_id", "predecessor_receipt_id"),
        ("predecessor_receipt_digest", "predecessor_receipt_digest"),
    ] {
        items.push(Projection::nullable(path(prefix, field), column));
    }
    for (path, column) in material_fields() {
        items.push(Projection::scalar(path, column));
    }
    items
}

fn fixed_envelope(kind: &str) -> Vec<Projection> {
    let schema = match kind {
        "challenge" => {
            "'compute_federation.external_pool_adapter_credential_reattestation_challenge.v1'"
        }
        "receipt" => {
            "'compute_federation.external_pool_adapter_credential_reattestation_receipt.v1'"
        }
        _ => unreachable!(),
    };
    let binding = if kind == "challenge" {
        "$.binding"
    } else {
        "$.reattestation.binding"
    };
    vec![
        Projection::fixed("$.schema", schema),
        Projection::fixed("$.canonicalization", "'rfc8785_jcs'"),
        Projection::fixed("$.digest_algorithm", "'sha256'"),
        Projection::fixed(
            path(binding, "schema"),
            "'compute_federation.external_pool_adapter_credential_reattestation_binding.v1'",
        ),
        Projection::fixed(
            path(binding, "signature_algorithm"),
            "'rsa-pkcs1v15-sha256'",
        ),
        Projection::fixed(
            path(binding, "verification_policy_id"),
            "'external_pool_non_bearer_credential_renewable_signed_challenge_v2'",
        ),
    ]
}

fn receipt_binding_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("challenge_id", "challenge_id"),
        ("challenge_nonce_digest", "challenge_nonce_digest"),
        ("provider_binding_id", "provider_binding_id"),
        ("provider_binding_digest", "provider_binding_digest"),
        (
            "provider_binding_material_digest",
            "provider_binding_material_digest",
        ),
        ("registry_release_id", "registry_release_id"),
        ("registry_release_digest", "registry_release_digest"),
        (
            "registry_release_material_digest",
            "registry_release_material_digest",
        ),
        ("route_adapter_projection_id", "route_adapter_projection_id"),
        ("installation_receipt_id", "installation_receipt_id"),
        ("installation_receipt_digest", "installation_receipt_digest"),
        ("installation_content_digest", "installation_content_digest"),
        ("application_id", "application_id"),
        ("application_digest", "application_digest"),
        ("adoption_receipt_id", "adoption_receipt_id"),
        ("adoption_receipt_digest", "adoption_receipt_digest"),
        ("provider_id", "provider_id"),
        ("provider_kind", "provider_kind"),
        ("provider_owner_account_id", "provider_owner_account_id"),
        (
            "observed_settlement_account_id",
            "observed_settlement_account_id",
        ),
        (
            "observed_provider_policy_revision",
            "observed_provider_policy_revision",
        ),
        ("observed_provider_digest", "observed_provider_digest"),
        ("observed_provider_status", "observed_provider_status"),
        ("adapter_id", "adapter_id"),
        ("release_version", "release_version"),
        ("adapter_config_revision", "adapter_config_revision"),
        ("adapter_config_digest", "adapter_config_digest"),
        ("admission_id", "admission_id"),
        ("admission_digest", "admission_digest"),
        (
            "legacy_credential_verification_receipt_id",
            "legacy_credential_verification_receipt_id",
        ),
        (
            "legacy_credential_verification_receipt_digest",
            "legacy_credential_verification_receipt_digest",
        ),
        ("credential_ref_scheme", "credential_ref_scheme"),
        (
            "credential_locator_commitment",
            "credential_locator_commitment",
        ),
        ("credential_verifier_digest", "credential_verifier_digest"),
        (
            "credential_verifier_key_record_id",
            "credential_verifier_key_record_id",
        ),
        (
            "credential_verifier_key_record_digest",
            "credential_verifier_key_record_digest",
        ),
        ("credential_verifier_key_id", "credential_verifier_key_id"),
        (
            "credential_verifier_record_id",
            "credential_verifier_record_id",
        ),
        (
            "credential_verifier_record_digest",
            "credential_verifier_record_digest",
        ),
        ("sequence", "sequence"),
        ("verifier_report_id", "verifier_report_id"),
        ("verification_started_at", "verification_started_at"),
        ("verification_completed_at", "verification_completed_at"),
        ("report_generated_at", "report_generated_at"),
        ("report_expires_at", "report_expires_at"),
        (
            "credential_resolution_outcome",
            "credential_resolution_outcome",
        ),
        (
            "provider_authentication_outcome",
            "provider_authentication_outcome",
        ),
        (
            "provider_response_evidence_digest",
            "provider_response_evidence_digest",
        ),
    ]
}

fn material_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "$.reattestation.signature_message_digest",
            "signature_message_digest",
        ),
        ("$.reattestation.signature_base64", "signature_base64"),
        ("$.reattestation.signature_digest", "signature_digest"),
        (
            "$.reattestation.recorded_by_admin_user_id",
            "recorded_by_admin_user_id",
        ),
        ("$.reattestation.confirmation", "confirmation"),
        ("$.reattestation.idempotency_scope", "idempotency_scope"),
        ("$.reattestation.idempotency_key", "idempotency_key"),
        ("$.reattestation.verified_at", "verified_at"),
        ("$.reattestation.recorded_at", "recorded_at"),
        ("$.reattestation.evidence_scope", "evidence_scope"),
        (
            "$.reattestation.credential_reattestation_effect",
            "credential_reattestation_effect",
        ),
        ("$.reattestation.adapter_effect", "adapter_effect"),
        ("$.reattestation.provider_effect", "provider_effect"),
        ("$.reattestation.route_effect", "route_effect"),
        ("$.reattestation.execution_effect", "execution_effect"),
        ("$.reattestation.usage_effect", "usage_effect"),
        ("$.reattestation.settlement_effect", "settlement_effect"),
    ]
}

fn revocation_projections() -> Vec<Projection> {
    let mut items = vec![Projection::fixed(
        "$.schema",
        "'compute_federation.external_pool_adapter_credential_reattestation_revocation_receipt.v1'",
    )];
    for (path, column) in [
        ("$.revocation_receipt_id", "revocation_receipt_id"),
        ("$.revocation_receipt_digest", "revocation_receipt_digest"),
        ("$.revocation_material_digest", "revocation_material_digest"),
        (
            "$.revocation.reattestation_receipt_id",
            "reattestation_receipt_id",
        ),
        (
            "$.revocation.reattestation_receipt_digest",
            "reattestation_receipt_digest",
        ),
        ("$.revocation.provider_binding_id", "provider_binding_id"),
        (
            "$.revocation.provider_binding_digest",
            "provider_binding_digest",
        ),
        (
            "$.revocation.revoked_by_admin_user_id",
            "revoked_by_admin_user_id",
        ),
        ("$.revocation.reason", "reason"),
        ("$.revocation.confirmation", "confirmation"),
        ("$.revocation.idempotency_scope", "idempotency_scope"),
        ("$.revocation.idempotency_key", "idempotency_key"),
        ("$.revocation.revoked_at", "revoked_at"),
        ("$.revocation.recorded_at", "recorded_at"),
        ("$.revocation.revocation_effect", "revocation_effect"),
        ("$.revocation.adapter_effect", "adapter_effect"),
        ("$.revocation.provider_effect", "provider_effect"),
        ("$.revocation.route_effect", "route_effect"),
        ("$.revocation.execution_effect", "execution_effect"),
        ("$.revocation.usage_effect", "usage_effect"),
        ("$.revocation.settlement_effect", "settlement_effect"),
    ] {
        items.push(Projection::scalar(path, column));
    }
    items.push(Projection::fixed("$.canonicalization", "'rfc8785_jcs'"));
    items.push(Projection::fixed("$.digest_algorithm", "'sha256'"));
    items
}

fn path(prefix: &str, field: &str) -> &'static str {
    Box::leak(format!("{prefix}.{field}").into_boxed_str())
}
