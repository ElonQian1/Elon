use rusqlite::{params, Connection};

use super::{capabilities_json, digest, manifest_json, verifier_json, AT};

pub(super) fn insert_release(
    connection: &Connection,
    json_installation_content_digest: &str,
) -> rusqlite::Result<usize> {
    let receipt = release_json(json_installation_content_digest);
    connection.execute(
        "INSERT INTO compute_external_pool_adapter_registry_releases VALUES(
          'registry-release-1',?1,?2,?3,?4,'rfc8785_jcs','sha256','admission-1',?5,
          'package-1',?6,?7,'source-1',?8,?9,'adapter-1','1.0.0','server_adapter',
          '[\"external_pool\"]',?10,?10,?11,?12,?13,?14,?10,4096,?15,?16,?17,6,
          8192,?18,?18,'provider_neutral_release_registered','none','none','none','none','none')",
        params![
            "compute_federation.external_pool_adapter_registry_release_receipt.v1",
            digest('4'),
            receipt,
            digest('5'),
            digest('a'),
            digest('e'),
            digest('f'),
            digest('1'),
            digest('6'),
            digest('b'),
            capabilities_json(),
            digest('c'),
            verifier_json(),
            digest('d'),
            manifest_json(),
            digest('2'),
            digest('3'),
            AT
        ],
    )
}

fn release_json(content_digest: &str) -> String {
    serde_json::json!({
      "schema":"compute_federation.external_pool_adapter_registry_release_receipt.v1",
      "registry_release_id":"registry-release-1",
      "registry_release_digest":digest('4'),
      "registry_release_material_digest":digest('5'),
      "canonicalization":"rfc8785_jcs","digest_algorithm":"sha256",
      "release":{
        "admission_id":"admission-1","admission_digest":digest('a'),
        "package_receipt_id":"package-1","package_receipt_digest":digest('e'),
        "package_material_digest":digest('f'),"source_receipt_id":"source-1",
        "source_receipt_digest":digest('1'),"installation_content_digest":content_digest,
        "adapter_id":"adapter-1","release_version":"1.0.0","route_kind":"server_adapter",
        "supported_provider_kinds":["external_pool"],"implementation_digest":digest('b'),
        "declared_implementation_sha256":digest('b'),
        "supported_capabilities":[0,1,2,3,4,5],"capability_set_digest":digest('c'),
        "credential_verifier":{"verification_kind":"signed_challenge","verifier_id":"verifier-1","verifier_revision":1},
        "credential_verifier_digest":digest('d'),"archive_sha256":digest('b'),
        "archive_size_bytes":4096,"manifest":{"adapter_id":"adapter-1","release_version":"1.0.0"},
        "manifest_digest":digest('2'),"entry_inventory_digest":digest('3'),
        "entry_count":6,"total_uncompressed_bytes":8192,"registered_at":AT,"recorded_at":AT,
        "registry_effect":"provider_neutral_release_registered","provider_effect":"none",
        "credential_effect":"none","route_effect":"none","execution_effect":"none",
        "settlement_effect":"none"
      }
    }).to_string()
}

pub(super) fn insert_binding(
    connection: &Connection,
    ordinal: usize,
    json_confirmation: &str,
) -> rusqlite::Result<usize> {
    let values = ProviderValues::new(ordinal);
    let receipt = binding_json(&values, json_confirmation);
    connection.execute(
        "INSERT INTO compute_external_pool_adapter_registry_provider_bindings(
          provider_binding_id,provider_binding_schema,provider_binding_digest,receipt_json,
          provider_binding_material_digest,canonicalization,digest_algorithm,
          registry_release_id,registry_release_digest,route_adapter_projection_id,
          installation_receipt_id,installation_receipt_digest,installation_material_digest,
          installation_content_digest,application_id,application_digest,adoption_receipt_id,
          adoption_receipt_digest,adoption_material_digest,provider_id,
          provider_owner_account_id,provider_policy_revision,provider_digest,adapter_id,
          release_version,adapter_config_revision,adapter_config_digest,admission_id,
          admission_digest,package_receipt_id,package_receipt_digest,package_material_digest,
          source_receipt_id,source_receipt_digest,sandbox_conformance_receipt_id,
          sandbox_conformance_receipt_digest,credential_verification_receipt_id,
          credential_verification_receipt_digest,credential_locator_commitment,
          bound_by_admin_user_id,confirmation,checked_at,bound_at,recorded_at,
          idempotency_scope,idempotency_key,registry_effect,provider_effect,credential_effect,
          route_effect,execution_effect,settlement_effect)
        VALUES(?1,?2,?3,?4,?5,'rfc8785_jcs','sha256','registry-release-1',?6,?7,
          ?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,1,?19,'adapter-1','1.0.0',1,
          'opaque-config','admission-1',?20,'package-1',?21,?22,'source-1',?23,
          ?24,?25,?26,?27,?28,'admin-1','confirm_external_pool_adapter_registry_binding',
          ?29,?29,?29,'registry',?30,'installed_instance_companion_recorded','none','none',
          'none','none','none')",
        params![
            values.binding_id,
            "compute_federation.external_pool_adapter_registry_provider_binding_receipt.v1",
            values.binding_digest,
            receipt,
            values.binding_material_digest,
            digest('4'),
            values.projection_id,
            values.installation_id,
            values.installation_digest,
            digest('1'),
            digest('6'),
            values.application_id,
            digest('8'),
            values.adoption_id,
            values.adoption_digest,
            digest('6'),
            values.provider_id,
            values.owner_id,
            values.provider_digest,
            digest('a'),
            digest('e'),
            digest('f'),
            digest('1'),
            values.sandbox_id,
            digest('9'),
            values.credential_id,
            digest('0'),
            digest('5'),
            AT,
            values.idempotency_key
        ],
    )
}

struct ProviderValues {
    binding_id: String,
    binding_digest: String,
    binding_material_digest: String,
    projection_id: String,
    installation_id: String,
    application_id: String,
    adoption_id: String,
    adoption_digest: String,
    installation_digest: String,
    provider_id: String,
    owner_id: String,
    provider_digest: String,
    sandbox_id: String,
    credential_id: String,
    idempotency_key: String,
}
impl ProviderValues {
    fn new(ordinal: usize) -> Self {
        let digit = char::from_digit(ordinal as u32 + 3, 10).unwrap();
        Self {
            binding_id: format!("binding-{ordinal}"),
            binding_digest: digest(if ordinal == 1 { '7' } else { '8' }),
            binding_material_digest: digest(if ordinal == 1 { '9' } else { '0' }),
            projection_id: format!("projection-{ordinal}"),
            installation_id: format!("installation-{ordinal}"),
            application_id: format!("application-{ordinal}"),
            adoption_id: format!("adoption-{ordinal}"),
            adoption_digest: digest(if ordinal == 1 { '7' } else { '8' }),
            installation_digest: digest(if ordinal == 1 { '9' } else { '0' }),
            provider_id: format!("provider-{ordinal}"),
            owner_id: format!("owner-{ordinal}"),
            provider_digest: digest(digit),
            sandbox_id: format!("sandbox-{ordinal}"),
            credential_id: format!("credential-{ordinal}"),
            idempotency_key: format!("key-{ordinal}"),
        }
    }
}

fn binding_json(value: &ProviderValues, confirmation: &str) -> String {
    serde_json::json!({
      "schema":"compute_federation.external_pool_adapter_registry_provider_binding_receipt.v1",
      "provider_binding_id":value.binding_id,"provider_binding_digest":value.binding_digest,
      "provider_binding_material_digest":value.binding_material_digest,
      "canonicalization":"rfc8785_jcs","digest_algorithm":"sha256","binding":{
        "registry_release_id":"registry-release-1","registry_release_digest":digest('4'),
        "route_adapter_projection_id":value.projection_id,"installation_receipt_id":value.installation_id,
        "installation_receipt_digest":value.installation_digest,"installation_material_digest":digest('1'),
        "installation_content_digest":digest('6'),"application_id":value.application_id,
        "application_digest":digest('8'),"adoption_receipt_id":value.adoption_id,
        "adoption_receipt_digest":value.adoption_digest,"adoption_material_digest":digest('6'),
        "provider_id":value.provider_id,"provider_owner_account_id":value.owner_id,
        "provider_policy_revision":1,"provider_digest":value.provider_digest,
        "adapter_id":"adapter-1","release_version":"1.0.0","adapter_config_revision":1,
        "adapter_config_digest":"opaque-config","admission_id":"admission-1",
        "admission_digest":digest('a'),"package_receipt_id":"package-1",
        "package_receipt_digest":digest('e'),"package_material_digest":digest('f'),
        "source_receipt_id":"source-1","source_receipt_digest":digest('1'),
        "sandbox_conformance_receipt_id":value.sandbox_id,"sandbox_conformance_receipt_digest":digest('9'),
        "credential_verification_receipt_id":value.credential_id,
        "credential_verification_receipt_digest":digest('0'),"credential_locator_commitment":digest('5'),
        "bound_by_admin_user_id":"admin-1","confirmation":confirmation,"checked_at":AT,
        "bound_at":AT,"recorded_at":AT,"idempotency_scope":"registry",
        "idempotency_key":value.idempotency_key,"registry_effect":"installed_instance_companion_recorded",
        "provider_effect":"none","credential_effect":"none","route_effect":"none",
        "execution_effect":"none","settlement_effect":"none"
      }
    }).to_string()
}
