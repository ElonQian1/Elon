use rusqlite::{params, Connection};

use super::{digest, AT};

pub(in crate::store_migrations::compute_external_pool_adapter_registry::tests) fn create_v247_fixture(
    connection: &Connection,
) {
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys=ON;
            CREATE TABLE compute_external_pool_adapter_release_admissions(
              admission_id TEXT PRIMARY KEY, admission_digest TEXT UNIQUE,
              adapter_id TEXT, release_version TEXT, route_kind TEXT,
              supported_provider_kinds_json TEXT, declared_implementation_sha256 TEXT,
              capabilities_json TEXT, capability_set_digest TEXT,
              verifier_verification_kind TEXT, verifier_id TEXT, verifier_revision INTEGER,
              verifier_digest TEXT, applied_at TEXT);
            CREATE TABLE compute_external_pool_adapter_release_admission_current(
              admission_id TEXT, admission_digest TEXT, current_status TEXT);
            CREATE TABLE compute_external_pool_adapter_artifact_package_receipts(
              package_receipt_id TEXT PRIMARY KEY, package_receipt_digest TEXT UNIQUE,
              package_material_digest TEXT, admission_id TEXT, admission_digest TEXT,
              source_receipt_digest TEXT, archive_sha256 TEXT, archive_size_bytes INTEGER,
              manifest_canonical_json TEXT, manifest_digest TEXT,
              entry_inventory_digest TEXT, entry_count INTEGER,
              total_uncompressed_bytes INTEGER, adapter_id TEXT, release_version TEXT,
              runtime_kind TEXT, supported_capabilities_json TEXT,
              capability_set_digest TEXT, credential_verifier_json TEXT,
              credential_verifier_digest TEXT, inspected_at TEXT);
            CREATE TABLE compute_external_pool_adapter_artifact_package_current(
              package_receipt_id TEXT, package_receipt_digest TEXT,
              admission_id TEXT, admission_digest TEXT, current_status TEXT);
            CREATE TABLE compute_external_pool_adapter_artifact_source_receipts(
              source_receipt_id TEXT PRIMARY KEY, source_receipt_digest TEXT UNIQUE,
              admission_id TEXT, admission_digest TEXT, adapter_id TEXT,
              release_version TEXT, declared_implementation_sha256 TEXT,
              reopened_sha256 TEXT, artifact_size_bytes INTEGER, recorded_at TEXT);
            CREATE TABLE compute_external_pool_onboarding_applications(
              application_id TEXT PRIMARY KEY);
            CREATE TABLE compute_external_pool_adapter_sandbox_conformance_reports(
              sandbox_conformance_receipt_id TEXT PRIMARY KEY);
            CREATE TABLE compute_external_pool_adapter_credential_verification_receipts(
              credential_verification_receipt_id TEXT PRIMARY KEY);
            CREATE TABLE compute_external_pool_adapter_adoption_receipts(
              adoption_receipt_id TEXT PRIMARY KEY, adoption_receipt_digest TEXT UNIQUE,
              adoption_material_digest TEXT, application_id TEXT, application_digest TEXT,
              provider_id TEXT, provider_owner_account_id TEXT,
              provider_policy_revision INTEGER, provider_digest TEXT,
              admission_id TEXT, admission_digest TEXT, adapter_id TEXT,
              adapter_release_version TEXT, adapter_config_revision INTEGER,
              adapter_config_digest TEXT, sandbox_conformance_receipt_id TEXT,
              sandbox_conformance_receipt_digest TEXT,
              credential_verification_receipt_id TEXT,
              credential_verification_receipt_digest TEXT,
              credential_locator_commitment TEXT);
            CREATE TABLE compute_external_pool_adapter_adoption_current(
              adoption_receipt_id TEXT, adoption_receipt_digest TEXT, current_status TEXT);
            CREATE TABLE compute_external_pool_adapter_adoption_terminal_receipts(
              terminal_receipt_id TEXT PRIMARY KEY, adoption_receipt_id TEXT,
              adoption_receipt_digest TEXT);
            CREATE TABLE compute_external_pool_adapter_installation_receipts(
              installation_receipt_id TEXT PRIMARY KEY, installation_receipt_digest TEXT UNIQUE,
              installation_material_digest TEXT, installation_content_digest TEXT,
              application_id TEXT, application_digest TEXT, adoption_receipt_id TEXT,
              adoption_receipt_digest TEXT, adoption_material_digest TEXT,
              provider_id TEXT, provider_owner_account_id TEXT,
              provider_policy_revision INTEGER, provider_digest TEXT, adapter_id TEXT,
              adapter_release_version TEXT, adapter_config_revision INTEGER,
              adapter_config_digest TEXT, admission_id TEXT, admission_digest TEXT,
              package_receipt_id TEXT, package_receipt_digest TEXT,
              package_material_digest TEXT, source_receipt_id TEXT,
              source_receipt_digest TEXT, installed_at TEXT);
            CREATE TABLE compute_external_pool_adapter_installation_current(
              installation_receipt_id TEXT, installation_receipt_digest TEXT,
              current_status TEXT);
            CREATE TABLE compute_external_pool_adapter_installation_terminal_receipts(
              terminal_receipt_id TEXT PRIMARY KEY, installation_receipt_id TEXT,
              installation_receipt_digest TEXT);
            CREATE TABLE compute_providers(
              provider_id TEXT PRIMARY KEY, provider_kind TEXT, owner_account_id TEXT,
              status TEXT, current_policy_revision INTEGER, current_provider_digest TEXT);
            CREATE TABLE compute_provider_versions(
              provider_id TEXT, policy_revision INTEGER, provider_digest TEXT,
              PRIMARY KEY(provider_id,policy_revision));
            CREATE TABLE compute_route_adapters(adapter_id TEXT PRIMARY KEY);
            CREATE TABLE legacy_v247_marker(marker TEXT PRIMARY KEY);
            INSERT INTO legacy_v247_marker VALUES('untouched');
            "#,
        )
        .unwrap();
    seed_global_roots(connection);
    seed_provider_roots(connection, 1);
    seed_provider_roots(connection, 2);
}

fn seed_global_roots(connection: &Connection) {
    let capabilities = capabilities_json();
    let verifier = verifier_json();
    let manifest = manifest_json();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_release_admissions VALUES(
          'admission-1',?1,'adapter-1','1.0.0','server_adapter','[\"external_pool\"]',
          ?2,?3,?4,'signed_challenge','verifier-1',1,?5,?6)",
            params![
                digest('a'),
                digest('b'),
                capabilities,
                digest('c'),
                digest('d'),
                AT
            ],
        )
        .unwrap();
    connection.execute("INSERT INTO compute_external_pool_adapter_release_admission_current VALUES('admission-1',?1,'staged')",[digest('a')]).unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_artifact_package_receipts VALUES(
          'package-1',?1,?2,'admission-1',?3,?4,?5,4096,?6,?7,?8,6,8192,
          'adapter-1','1.0.0','server_sidecar_v1',?9,?10,?11,?12,?13)",
            params![
                digest('e'),
                digest('f'),
                digest('a'),
                digest('1'),
                digest('b'),
                manifest,
                digest('2'),
                digest('3'),
                capabilities,
                digest('c'),
                verifier,
                digest('d'),
                AT
            ],
        )
        .unwrap();
    connection.execute("INSERT INTO compute_external_pool_adapter_artifact_package_current VALUES('package-1',?1,'admission-1',?2,'verified_current')",params![digest('e'),digest('a')]).unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_artifact_source_receipts VALUES(
          'source-1',?1,'admission-1',?2,'adapter-1','1.0.0',?3,?3,4096,?4)",
            params![digest('1'), digest('a'), digest('b'), AT],
        )
        .unwrap();
}

fn seed_provider_roots(connection: &Connection, ordinal: usize) {
    let provider = format!("provider-{ordinal}");
    let owner = format!("owner-{ordinal}");
    let application = format!("application-{ordinal}");
    let adoption = format!("adoption-{ordinal}");
    let installation = format!("installation-{ordinal}");
    let sandbox = format!("sandbox-{ordinal}");
    let credential = format!("credential-{ordinal}");
    let provider_digest = digest(char::from_digit(ordinal as u32 + 3, 10).unwrap());
    let adoption_digest = digest(if ordinal == 1 { '7' } else { '8' });
    let installation_digest = digest(if ordinal == 1 { '9' } else { '0' });
    connection
        .execute(
            "INSERT INTO compute_external_pool_onboarding_applications VALUES(?1)",
            [&application],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_sandbox_conformance_reports VALUES(?1)",
            [&sandbox],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_credential_verification_receipts VALUES(?1)",
            [&credential],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_providers VALUES(?1,'external_pool',?2,'registering',1,?3)",
            params![provider, owner, provider_digest],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_provider_versions VALUES(?1,1,?2)",
            params![provider, provider_digest],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_adoption_receipts VALUES(
          ?1,?2,?3,?4,?5,?6,?7,1,?8,'admission-1',?9,'adapter-1','1.0.0',1,
          'opaque-config',?10,?11,?12,?13,?14)",
            params![
                adoption,
                adoption_digest,
                digest('6'),
                application,
                digest('8'),
                provider,
                owner,
                provider_digest,
                digest('a'),
                sandbox,
                digest('9'),
                credential,
                digest('0'),
                digest('5')
            ],
        )
        .unwrap();
    connection.execute("INSERT INTO compute_external_pool_adapter_adoption_current VALUES(?1,?2,'adopted_current')",params![adoption,adoption_digest]).unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_installation_receipts VALUES(
          ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,?12,'adapter-1','1.0.0',1,
          'opaque-config','admission-1',?13,'package-1',?14,?15,'source-1',?16,?17)",
            params![
                installation,
                installation_digest,
                digest('1'),
                digest('6'),
                application,
                digest('8'),
                adoption,
                adoption_digest,
                digest('6'),
                provider,
                owner,
                provider_digest,
                digest('a'),
                digest('e'),
                digest('f'),
                digest('1'),
                AT
            ],
        )
        .unwrap();
    connection.execute("INSERT INTO compute_external_pool_adapter_installation_current VALUES(?1,?2,'installed_upstreams_current')",params![installation,installation_digest]).unwrap();
}

pub(super) fn capabilities_json() -> String {
    serde_json::json!([0, 1, 2, 3, 4, 5]).to_string()
}
pub(super) fn verifier_json() -> String {
    serde_json::json!({
        "verification_kind":"signed_challenge",
        "verifier_id":"verifier-1",
        "verifier_revision":1,
        "verifier_digest":digest('d')
    })
    .to_string()
}
pub(super) fn manifest_json() -> String {
    serde_json::json!({"adapter_id":"adapter-1","release_version":"1.0.0"}).to_string()
}
