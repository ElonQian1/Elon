use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use sha2::Sha256;

use crate::{
    open_commerce_portability_import_model::{
        ConsumerPortabilityPackageSignature, CreateConsumerPortabilityImportRequest,
        CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS, CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS,
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS, CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM,
    },
    open_commerce_portability_import_service,
    open_commerce_portability_model::CONSUMER_PORTABILITY_EXPORT_SCHEMA,
    open_commerce_portability_service,
    open_commerce_portability_trust_model::CreateConsumerPortabilityTrustKeyRequest,
    open_commerce_portability_trust_service,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

use super::{export_request, fixture, Fixture};

#[test]
fn v5_import_preserves_evidence_in_an_isolated_idempotent_snapshot() {
    let fixture = fixture(true);
    let package = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        export_request("v5-isolated-import"),
    )
    .unwrap();
    let destination_project_id = destination_project(&fixture, "V5 isolated destination");
    let actor = destination_actor(&fixture);
    let before = business_table_counts(&fixture.store);

    let imported = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request("operator-a.example", package.clone(), None),
    )
    .unwrap();
    let replay = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request("operator-a.example", package.clone(), None),
    )
    .unwrap();

    assert_eq!(replay.id, imported.id);
    assert_eq!(imported.package.schema, CONSUMER_PORTABILITY_EXPORT_SCHEMA);
    assert_eq!(imported.package.payload.data_erasure_evidence.len(), 1);
    assert_eq!(
        imported.trust_status,
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS
    );
    assert_eq!(
        imported.merge_status,
        CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS
    );
    assert!(imported.signature.is_none());
    assert_eq!(
        imported.package_json,
        open_commerce_portability_service::canonical_export_json(&package).unwrap()
    );
    assert_eq!(before, business_table_counts(&fixture.store));

    let summaries = open_commerce_portability_import_service::list_imports(
        &fixture.store,
        &destination_project_id,
        &actor,
        20,
    )
    .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].data_erasure_evidence_count, 1);
    assert_eq!(summaries[0].relationship_count, 1);
    assert_eq!(summaries[0].data_request_count, 1);
}

#[test]
fn signed_v5_import_records_operator_trust_and_revoked_keys_fail_closed() {
    let fixture = fixture(true);
    let package = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        export_request("v5-signed-import"),
    )
    .unwrap();
    let destination_project_id = destination_project(&fixture, "V5 signed destination");
    let actor = destination_actor(&fixture);
    let source_operator = "signed-operator.example";
    let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let trust_key = create_trust_key(
        &fixture.store,
        &destination_project_id,
        &actor,
        source_operator,
        &private_key,
    );
    let signature = sign_package(source_operator, &trust_key.key_id, &package, &private_key);

    let imported = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request(source_operator, package.clone(), Some(signature.clone())),
    )
    .unwrap();

    assert_eq!(
        imported.trust_status,
        CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS
    );
    assert_eq!(imported.source_operator, source_operator);
    assert_eq!(imported.package.payload.data_erasure_evidence.len(), 1);
    assert_eq!(
        imported.signature.as_ref().unwrap().key_id,
        trust_key.key_id
    );
    assert!(imported.signature_verified_at.is_some());

    open_commerce_portability_trust_service::revoke_trust_key(
        &fixture.store,
        &destination_project_id,
        &trust_key.id,
        &actor,
    )
    .unwrap();
    let error = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request(source_operator, package, Some(signature)),
    )
    .unwrap_err();
    assert!(error.to_string().contains("有效信任公钥"));
}

#[test]
fn identical_v5_envelope_cannot_be_relabelled_to_another_operator() {
    let fixture = fixture(true);
    let package = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        export_request("v5-operator-binding"),
    )
    .unwrap();
    let destination_project_id = destination_project(&fixture, "V5 operator binding");
    let actor = destination_actor(&fixture);
    let first = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request("operator-a.example", package.clone(), None),
    )
    .unwrap();
    let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let second_operator = "operator-b.example";
    let trust_key = create_trust_key(
        &fixture.store,
        &destination_project_id,
        &actor,
        second_operator,
        &private_key,
    );
    let signature = sign_package(second_operator, &trust_key.key_id, &package, &private_key);

    let error = open_commerce_portability_import_service::create_import(
        &fixture.store,
        &destination_project_id,
        &actor,
        import_request(second_operator, package, Some(signature)),
    )
    .unwrap_err();
    assert!(error.to_string().contains("不能更换来源运营方身份"));

    let persisted = open_commerce_portability_import_service::get_import(
        &fixture.store,
        &destination_project_id,
        &first.id,
        &actor,
    )
    .unwrap();
    assert_eq!(persisted.source_operator, "operator-a.example");
    assert_eq!(
        persisted.trust_status,
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS
    );
    assert!(persisted.signature.is_none());
}

fn destination_project(fixture: &Fixture, name: &str) -> String {
    fixture
        .store
        .create_project(&fixture.consumer_user_id, name, None, None)
        .unwrap()
        .project
        .id
}

fn destination_actor(fixture: &Fixture) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id: &fixture.consumer_user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}

fn import_request(
    source_operator: &str,
    package: crate::open_commerce_portability_model::ConsumerPortabilityExport,
    signature: Option<ConsumerPortabilityPackageSignature>,
) -> CreateConsumerPortabilityImportRequest {
    CreateConsumerPortabilityImportRequest {
        source_operator: source_operator.to_string(),
        package,
        signature,
    }
}

fn create_trust_key(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    source_operator: &str,
    private_key: &RsaPrivateKey,
) -> crate::open_commerce_portability_trust_model::ConsumerPortabilityTrustKey {
    open_commerce_portability_trust_service::create_trust_key(
        store,
        destination_project_id,
        actor,
        CreateConsumerPortabilityTrustKeyRequest {
            source_operator: source_operator.to_string(),
            public_key_pem: private_key
                .to_public_key()
                .to_public_key_pem(LineEnding::LF)
                .unwrap(),
        },
    )
    .unwrap()
}

fn sign_package(
    source_operator: &str,
    key_id: &str,
    package: &crate::open_commerce_portability_model::ConsumerPortabilityExport,
    private_key: &RsaPrivateKey,
) -> ConsumerPortabilityPackageSignature {
    let message = open_commerce_portability_trust_service::signature_message(
        source_operator,
        key_id,
        package,
    );
    let signature = SigningKey::<Sha256>::new(private_key.clone()).sign(message.as_bytes());
    ConsumerPortabilityPackageSignature {
        algorithm: CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM.to_string(),
        key_id: key_id.to_string(),
        signature_base64: BASE64.encode(signature.to_bytes()),
    }
}

fn business_table_counts(store: &Store) -> Vec<i64> {
    [
        "open_commerce_consumer_relationships",
        "open_commerce_grants",
        "open_commerce_consumer_data_requests",
        "open_commerce_data_erasure_evidence",
    ]
    .into_iter()
    .map(|table| {
        store
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    })
    .collect()
}
