use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_guard(
        conn,
        "external_pool_adapter_sandbox_reattestation_challenge_projection",
        "compute_external_pool_adapter_sandbox_reattestation_challenges",
        "challenge_json",
        &challenge_projections(),
    )?;
    install_guard(
        conn,
        "external_pool_adapter_sandbox_reattestation_receipt_projection",
        "compute_external_pool_adapter_sandbox_reattestation_receipts",
        "receipt_json",
        &receipt_projections(),
    )?;
    install_guard(
        conn,
        "external_pool_adapter_sandbox_reattestation_revocation_projection",
        "compute_external_pool_adapter_sandbox_reattestation_revocations",
        "receipt_json",
        &revocation_projections(),
    )?;
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
        .map(|projection| projection.mismatch(json_column))
        .collect::<Vec<_>>()
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS {name} BEFORE INSERT ON {table}\n\
         WHEN {mismatch}\n\
         BEGIN SELECT RAISE(ABORT,'V252 canonical JSON projection mismatch'); END;"
    ))?;
    Ok(())
}

struct Projection {
    path: &'static str,
    expected: &'static str,
    json: bool,
    literal: bool,
    require_path: bool,
}

impl Projection {
    fn scalar(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            json: false,
            literal: false,
            require_path: false,
        }
    }

    fn nullable(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            json: false,
            literal: false,
            require_path: true,
        }
    }

    fn fixed(path: &'static str, literal: &'static str) -> Self {
        Self {
            path,
            expected: literal,
            json: false,
            literal: true,
            require_path: false,
        }
    }

    fn json(path: &'static str, column: &'static str) -> Self {
        Self {
            path,
            expected: column,
            json: true,
            literal: false,
            require_path: false,
        }
    }

    fn mismatch(&self, json_column: &str) -> String {
        let mismatch = if self.json {
            format!(
                "json(json_extract(NEW.{json_column},'{}')) IS NOT json(NEW.{})",
                self.path, self.expected
            )
        } else if self.literal {
            format!(
                "json_extract(NEW.{json_column},'{}') IS NOT {}",
                self.path, self.expected
            )
        } else {
            format!(
                "json_extract(NEW.{json_column},'{}') IS NOT NEW.{}",
                self.path, self.expected
            )
        };
        if self.require_path {
            format!(
                "json_type(NEW.{json_column},'{}') IS NULL OR {mismatch}",
                self.path
            )
        } else {
            mismatch
        }
    }
}

fn common_binding(prefix: &str) -> Vec<Projection> {
    let mut fields = Vec::new();
    for (field, column) in [
        ("challenge_id", "challenge_id"),
        ("challenge_nonce_digest", "challenge_nonce_digest"),
        ("registry_release_id", "registry_release_id"),
        ("registry_release_digest", "registry_release_digest"),
        (
            "registry_release_material_digest",
            "registry_release_material_digest",
        ),
        (
            "vulnerability_reattestation_receipt_id",
            "vulnerability_reattestation_receipt_id",
        ),
        (
            "vulnerability_reattestation_receipt_digest",
            "vulnerability_reattestation_receipt_digest",
        ),
        (
            "vulnerability_reattestation_material_digest",
            "vulnerability_reattestation_material_digest",
        ),
        (
            "sandbox_verifier_key_record_id",
            "sandbox_verifier_key_record_id",
        ),
        (
            "sandbox_verifier_key_record_digest",
            "sandbox_verifier_key_record_digest",
        ),
        ("sandbox_verifier_key_id", "sandbox_verifier_key_id"),
        ("sequence", "sequence"),
        ("predecessor_receipt_id", "predecessor_receipt_id"),
        ("predecessor_receipt_digest", "predecessor_receipt_digest"),
    ] {
        let path = path(prefix, field);
        fields.push(if field.starts_with("predecessor_") {
            Projection::nullable(path, column)
        } else {
            Projection::scalar(path, column)
        });
    }
    fields
}

fn challenge_projections() -> Vec<Projection> {
    let mut items = vec![
        Projection::fixed(
            "$.schema",
            "'compute_federation.external_pool_adapter_sandbox_reattestation_challenge.v1'",
        ),
        Projection::fixed("$.canonicalization", "'rfc8785_jcs'"),
        Projection::fixed("$.digest_algorithm", "'sha256'"),
        Projection::fixed("$.signature_algorithm", "'rsa-pkcs1v15-sha256'"),
        Projection::scalar("$.signature_message_base64", "signature_message_base64"),
        Projection::scalar("$.signature_message_digest", "signature_message_digest"),
        Projection::fixed(
            "$.binding.schema",
            "'compute_federation.external_pool_adapter_sandbox_reattestation_binding.v1'",
        ),
        Projection::scalar("$.binding.challenge_nonce_base64", "challenge_nonce_base64"),
        Projection::scalar("$.binding.challenge_issued_at", "issued_at"),
        Projection::scalar("$.binding.challenge_expires_at", "expires_at"),
        Projection::fixed("$.binding.signature_algorithm", "'rsa-pkcs1v15-sha256'"),
        Projection::fixed(
            "$.binding.sandbox_policy_id",
            "'external_pool_adapter_six_capability_offline_sandbox_v1'",
        ),
        Projection::fixed(
            "$.binding.isolation_profile_id",
            "'offline_readonly_ephemeral_no_child_process_v1'",
        ),
        Projection::fixed("$.binding.external_network_attempt_count", "0"),
        Projection::fixed("$.binding.write_outside_ephemeral_count", "0"),
        Projection::fixed("$.binding.child_process_attempt_count", "0"),
        Projection::fixed("$.binding.passed_capability_count", "6"),
        Projection::fixed("$.binding.policy_violation_count", "0"),
    ];
    items.extend(common_binding("$.binding"));
    items
}

fn receipt_projections() -> Vec<Projection> {
    let prefix = "$.reattestation.binding";
    let mut items = vec![
        Projection::fixed(
            "$.schema",
            "'compute_federation.external_pool_adapter_sandbox_reattestation_receipt.v1'",
        ),
        Projection::scalar("$.reattestation_receipt_id", "reattestation_receipt_id"),
        Projection::scalar(
            "$.reattestation_receipt_digest",
            "reattestation_receipt_digest",
        ),
        Projection::scalar(
            "$.reattestation_material_digest",
            "reattestation_material_digest",
        ),
        Projection::fixed("$.canonicalization", "'rfc8785_jcs'"),
        Projection::fixed("$.digest_algorithm", "'sha256'"),
        Projection::fixed(
            "$.reattestation.binding.schema",
            "'compute_federation.external_pool_adapter_sandbox_reattestation_binding.v1'",
        ),
        Projection::fixed(
            "$.reattestation.binding.signature_algorithm",
            "'rsa-pkcs1v15-sha256'",
        ),
        Projection::fixed(
            "$.reattestation.binding.sandbox_policy_id",
            "'external_pool_adapter_six_capability_offline_sandbox_v1'",
        ),
    ];
    items.extend(common_binding(prefix));
    for (field, column) in receipt_scalar_fields() {
        items.push(Projection::scalar(path(prefix, field), column));
    }
    for (field, column) in [
        ("supported_provider_kinds", "supported_provider_kinds_json"),
        ("supported_capabilities", "supported_capabilities_json"),
        ("expected_credential_verifier", "credential_verifier_json"),
        ("test_plan", "test_plan_json"),
        ("observations", "observations_json"),
    ] {
        items.push(Projection::json(path(prefix, field), column));
    }
    for (path, column) in material_fields() {
        items.push(Projection::scalar(path, column));
    }
    items
}

fn receipt_scalar_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("admission_id", "admission_id"),
        ("admission_digest", "admission_digest"),
        ("package_receipt_id", "package_receipt_id"),
        ("package_receipt_digest", "package_receipt_digest"),
        ("source_receipt_id", "source_receipt_id"),
        ("source_receipt_digest", "source_receipt_digest"),
        ("adapter_id", "adapter_id"),
        ("release_version", "release_version"),
        ("route_kind", "route_kind"),
        ("implementation_digest", "implementation_digest"),
        (
            "declared_implementation_sha256",
            "declared_implementation_sha256",
        ),
        ("capability_set_digest", "capability_set_digest"),
        ("credential_verifier_digest", "credential_verifier_digest"),
        ("archive_sha256", "archive_sha256"),
        ("archive_size_bytes", "archive_size_bytes"),
        ("manifest_digest", "manifest_digest"),
        ("entry_inventory_digest", "entry_inventory_digest"),
        ("entry_count", "entry_count"),
        ("total_uncompressed_bytes", "total_uncompressed_bytes"),
        ("installation_content_digest", "installation_content_digest"),
        (
            "vulnerability_reattestation_sequence",
            "vulnerability_reattestation_sequence",
        ),
        (
            "vulnerability_reattestation_verified_at",
            "vulnerability_reattestation_verified_at",
        ),
        (
            "vulnerability_intelligence_snapshot_digest",
            "vulnerability_intelligence_snapshot_digest",
        ),
        (
            "vulnerability_intelligence_expires_at",
            "vulnerability_intelligence_expires_at",
        ),
        ("security_receipt_id", "security_receipt_id"),
        ("security_receipt_digest", "security_receipt_digest"),
        ("security_material_digest", "security_material_digest"),
        ("sbom_digest", "sbom_digest"),
        ("component_inventory_digest", "component_inventory_digest"),
        ("component_count", "component_count"),
        ("dependency_inventory_digest", "dependency_inventory_digest"),
        ("sandbox_verifier_operator", "sandbox_verifier_operator"),
        ("sandbox_verifier_product", "sandbox_verifier_product"),
        ("verifier_report_id", "verifier_report_id"),
        ("sandbox_runtime_id", "sandbox_runtime_id"),
        ("runtime_image_digest", "runtime_image_digest"),
        ("isolation_profile_id", "isolation_profile_id"),
        ("run_started_at", "run_started_at"),
        ("run_completed_at", "run_completed_at"),
        ("report_generated_at", "report_generated_at"),
        ("report_expires_at", "report_expires_at"),
        (
            "external_network_attempt_count",
            "external_network_attempt_count",
        ),
        (
            "write_outside_ephemeral_count",
            "write_outside_ephemeral_count",
        ),
        ("child_process_attempt_count", "child_process_attempt_count"),
        ("peak_memory_bytes", "peak_memory_bytes"),
        ("cpu_time_ms", "cpu_time_ms"),
        ("test_plan_digest", "test_plan_digest"),
        (
            "observation_inventory_digest",
            "observation_inventory_digest",
        ),
        ("passed_capability_count", "passed_capability_count"),
        ("policy_violation_count", "policy_violation_count"),
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
            "$.reattestation.sandbox_reattestation_effect",
            "sandbox_reattestation_effect",
        ),
        ("$.reattestation.adapter_effect", "adapter_effect"),
        ("$.reattestation.provider_effect", "provider_effect"),
        ("$.reattestation.credential_effect", "credential_effect"),
        ("$.reattestation.route_effect", "route_effect"),
        ("$.reattestation.execution_effect", "execution_effect"),
        ("$.reattestation.settlement_effect", "settlement_effect"),
    ]
}

fn revocation_projections() -> Vec<Projection> {
    let mut items = vec![Projection::fixed(
        "$.schema",
        "'compute_federation.external_pool_adapter_sandbox_reattestation_revocation_receipt.v1'",
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
        ("$.revocation.registry_release_id", "registry_release_id"),
        (
            "$.revocation.registry_release_digest",
            "registry_release_digest",
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
        ("$.revocation.credential_effect", "credential_effect"),
        ("$.revocation.route_effect", "route_effect"),
        ("$.revocation.execution_effect", "execution_effect"),
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
