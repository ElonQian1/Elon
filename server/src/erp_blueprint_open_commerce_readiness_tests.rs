use serde_json::json;
use uuid::Uuid;

use crate::{
    erp_blueprint::{
        instance_service, materialization,
        model::{
            CreateBlueprintRequest, CreateBlueprintVersionRequest, CreateErpInstanceRequest,
            ErpCapabilityDefinition, ErpModuleDefinition, ErpReleaseCompatibility,
            ErpReleaseManifest, ErpRollbackPlan, UpdateErpOpenCommerceMerchantRequest,
            VersionedErpModule, RELEASE_SCHEMA,
        },
        open_commerce_readiness, service,
    },
    erp_blueprint_mcp,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    store::Store,
};

const MANIFEST_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct Fixture {
    store: Store,
    owner_id: String,
    project_id: String,
    instance_id: String,
}

#[test]
fn readiness_projects_existing_truth_without_exposing_runtime_secrets() {
    let fixture = fixture();
    let empty = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        None,
    )
    .unwrap();
    assert!(!empty.consumer_invocation_ready);
    assert_eq!(empty.merchant_selection.status, "merchant_missing");
    assert!(has_blocker(&empty, "merchant_missing"));

    let merchant = create_merchant(&fixture, "coffee-ready");
    let configured = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        None,
    )
    .unwrap();
    assert_eq!(configured.merchant_selection.status, "selected_implicit");
    assert!(has_blocker(&configured, "runtime_binding_missing"));

    fixture
        .store
        .upsert_open_commerce_runtime_binding(
            &fixture.project_id,
            &merchant.id,
            &fixture.owner_id,
            UpsertRuntimeBindingRequest {
                endpoint_base_url: "http://127.0.0.1:30991".into(),
                credential_ref: "OPEN_COMMERCE_RUNTIME_SECRET_READINESS_TEST".into(),
                manifest_sha256: Some(MANIFEST_SHA256.into()),
                timeout_ms: 2_000,
            },
        )
        .unwrap();
    fixture
        .store
        .mark_open_commerce_runtime_verified(&merchant.id, Some(MANIFEST_SHA256))
        .unwrap();
    fixture
        .store
        .create_open_commerce_capability(
            &fixture.project_id,
            &merchant.id,
            CreateCapabilityRequest {
                capability_key: "catalog.search".into(),
                display_name: "搜索在售商品".into(),
                description: String::new(),
                kind: "query".into(),
                access_level: ACCESS_PUBLIC.into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                handler_type: HANDLER_MERCHANT_RUNTIME.into(),
                handler_config: None,
                unit_price_micros: 1_000,
                currency: "CNY".into(),
                freshness_seconds: 30,
            },
        )
        .unwrap();
    fixture
        .store
        .set_open_commerce_directory_publication(
            &fixture.project_id,
            &merchant.id,
            &fixture.owner_id,
            true,
        )
        .unwrap();

    let ready = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        None,
    )
    .unwrap();
    assert!(ready.consumer_invocation_ready);
    assert!(ready.consumer_discovery_ready);
    assert!(!ready.erp_onboarding_ready);
    assert_eq!(ready.overall_state, "consumer_ready_erp_pending");
    assert_eq!(ready.active_runtime_capability_keys, vec!["catalog.search"]);
    let serialized = serde_json::to_string(&ready).unwrap();
    assert!(!serialized.contains("127.0.0.1"));
    assert!(!serialized.contains("OPEN_COMMERCE_RUNTIME_SECRET"));
}

#[test]
fn multiple_merchants_require_an_explicit_non_persistent_selection() {
    let fixture = fixture();
    let first = create_merchant(&fixture, "coffee-one");
    create_merchant(&fixture, "coffee-two");

    let ambiguous = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        None,
    )
    .unwrap();
    assert_eq!(ambiguous.merchant_selection.status, "selection_required");
    assert!(has_blocker(&ambiguous, "merchant_selection_required"));

    let selected = erp_blueprint_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "erp_get_open_commerce_readiness",
        json!({"instance_id":fixture.instance_id,"merchant_id":first.id}),
    )
    .unwrap();
    assert_eq!(
        selected["merchant_selection"]["status"],
        "selected_explicit"
    );
    assert_eq!(selected["merchant_selection"]["selected"]["id"], first.id);
}

#[test]
fn confirmed_binding_becomes_the_instance_identity_and_revisioned_contract() {
    let fixture = fixture();
    let first = create_merchant(&fixture, "coffee-bound");
    let second = create_merchant(&fixture, "coffee-preview-only");

    let bound = instance_service::update_open_commerce_merchant(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        UpdateErpOpenCommerceMerchantRequest {
            expected_revision: 1,
            merchant_confirmed: true,
            merchant_id: Some(first.id.clone()),
        },
    )
    .unwrap();
    assert_eq!(bound.configuration_revision, 2);
    assert_eq!(
        bound.open_commerce_merchant_id.as_deref(),
        Some(first.id.as_str())
    );

    let readiness = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        None,
    )
    .unwrap();
    assert_eq!(readiness.merchant_selection.status, "selected_binding");
    assert_eq!(readiness.merchant_selection.selected.unwrap().id, first.id);

    let override_error = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        Some(&second.id),
    )
    .unwrap_err();
    assert!(override_error
        .to_string()
        .contains("不能通过查询参数临时覆盖"));

    let blueprint = fixture.store.erp_blueprint(&bound.blueprint_id).unwrap();
    let version = fixture
        .store
        .erp_blueprint_version(&bound.pinned_version_id)
        .unwrap();
    let contract = materialization::build_contract(&blueprint, &version, &bound);
    assert_eq!(contract.configuration.revision, 2);
    assert_eq!(
        contract.configuration.open_commerce_merchant_id,
        bound.open_commerce_merchant_id
    );

    let stale = instance_service::update_open_commerce_merchant(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        UpdateErpOpenCommerceMerchantRequest {
            expected_revision: 1,
            merchant_confirmed: true,
            merchant_id: None,
        },
    )
    .unwrap_err();
    assert!(stale.to_string().contains("实例配置已变化"));
}

#[test]
fn binding_rejects_unconfirmed_and_cross_project_merchants() {
    let fixture = fixture();
    let local = create_merchant(&fixture, "coffee-local");
    let unconfirmed = instance_service::update_open_commerce_merchant(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        UpdateErpOpenCommerceMerchantRequest {
            expected_revision: 1,
            merchant_confirmed: false,
            merchant_id: Some(local.id),
        },
    )
    .unwrap_err();
    assert!(unconfirmed.to_string().contains("商户未确认"));

    let other_project = fixture
        .store
        .create_project(&fixture.owner_id, "Other Binding Project", None, None)
        .unwrap()
        .project;
    let other = fixture
        .store
        .create_open_commerce_merchant(
            &other_project.id,
            &fixture.owner_id,
            merchant_request("coffee-other-binding"),
        )
        .unwrap();
    let cross_project = instance_service::update_open_commerce_merchant(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        UpdateErpOpenCommerceMerchantRequest {
            expected_revision: 1,
            merchant_confirmed: true,
            merchant_id: Some(other.id),
        },
    )
    .unwrap_err();
    assert!(cross_project
        .to_string()
        .contains("当前项目中不存在该商户节点"));
}

#[test]
fn one_open_commerce_identity_cannot_belong_to_two_erp_instances() {
    let first = fixture();
    let merchant = create_merchant(&first, "coffee-exclusive");
    instance_service::update_open_commerce_merchant(
        &first.store,
        &first.project_id,
        &first.instance_id,
        UpdateErpOpenCommerceMerchantRequest {
            expected_revision: 1,
            merchant_confirmed: true,
            merchant_id: Some(merchant.id.clone()),
        },
    )
    .unwrap();

    let original = first.store.erp_instance(&first.instance_id).unwrap();
    let blueprint = first.store.erp_blueprint(&original.blueprint_id).unwrap();
    let duplicate_project = first
        .store
        .create_project(&first.owner_id, "Duplicate ERP Merchant", None, None)
        .unwrap()
        .project;
    let duplicate = service::create_instance(
        &first.store,
        &blueprint.definition.source_project_id,
        &blueprint.id,
        &first.owner_id,
        CreateErpInstanceRequest {
            instance_key: format!("duplicate.{}", Uuid::new_v4().simple()),
            project_name: String::new(),
            target_project_id: Some(duplicate_project.id),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "merchant.clean".into(),
            enabled_modules: vec!["catalog".into()],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap();
    let error = first
        .store
        .update_erp_instance_open_commerce_merchant(&duplicate.id, 1, Some(&merchant.id))
        .unwrap_err();
    assert!(error.to_string().contains("已经归属于其他 ERP 实例"));
}

#[test]
fn explicit_merchant_selection_cannot_cross_project_boundaries() {
    let fixture = fixture();
    let other_project = fixture
        .store
        .create_project(&fixture.owner_id, "Other Merchant Project", None, None)
        .unwrap()
        .project;
    let other_merchant = fixture
        .store
        .create_open_commerce_merchant(
            &other_project.id,
            &fixture.owner_id,
            merchant_request("other-coffee"),
        )
        .unwrap();

    let error = open_commerce_readiness::get(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        Some(&other_merchant.id),
    )
    .unwrap_err();
    assert!(error.to_string().contains("当前项目中不存在指定的商户节点"));
}

fn has_blocker(readiness: &open_commerce_readiness::ErpOpenCommerceReadiness, code: &str) -> bool {
    readiness.blockers.iter().any(|item| item.code == code)
}

fn create_merchant(
    fixture: &Fixture,
    slug: &str,
) -> crate::open_commerce_model::OpenCommerceMerchant {
    fixture
        .store
        .create_open_commerce_merchant(
            &fixture.project_id,
            &fixture.owner_id,
            merchant_request(slug),
        )
        .unwrap()
}

fn merchant_request(slug: &str) -> CreateMerchantRequest {
    CreateMerchantRequest {
        display_name: format!("Readiness {slug}"),
        slug: Some(slug.into()),
        description: String::new(),
        node_mode: "self_hosted".into(),
        public_profile: json!({"category":"coffee"}),
    }
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_erp_open_commerce_readiness_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("readiness test store should open");
    let owner = store
        .create_user(
            &format!("erp-readiness-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some("ERP Readiness"),
            None,
        )
        .unwrap();
    let blueprint_project = store
        .create_project(&owner.id, "ERP Readiness Blueprint", None, None)
        .unwrap()
        .project;
    let merchant_project = store
        .create_project(&owner.id, "ERP Readiness Merchant", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &blueprint_project.id,
        &owner.id,
        CreateBlueprintRequest {
            blueprint_key: "official.erp.readiness".into(),
            name: "Official ERP Readiness".into(),
            description: String::new(),
            modules: vec![ErpModuleDefinition {
                module_key: "catalog".into(),
                version: "1.0.0".into(),
                kind: "core".into(),
                required: true,
                dependencies: vec![],
            }],
            capabilities: vec![ErpCapabilityDefinition {
                capability_key: "catalog.query".into(),
                display_name: "Catalog query".into(),
                description: String::new(),
                category: "catalog".into(),
                module_key: "catalog".into(),
                aliases: vec![],
                composable_with: vec![],
            }],
            themes: vec!["merchant.clean".into()],
            extension_points: vec![],
            proposal_threshold: 2,
        },
    )
    .unwrap();
    service::publish_version(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release(),
        },
    )
    .unwrap();
    let instance = service::create_instance(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: format!("merchant.{}", Uuid::new_v4().simple()),
            project_name: String::new(),
            target_project_id: Some(merchant_project.id.clone()),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "merchant.clean".into(),
            enabled_modules: vec!["catalog".into()],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap();
    Fixture {
        store,
        owner_id: owner.id,
        project_id: merchant_project.id,
        instance_id: instance.id,
    }
}

fn release() -> ErpReleaseManifest {
    ErpReleaseManifest {
        schema: RELEASE_SCHEMA.into(),
        blueprint_key: "official.erp.readiness".into(),
        version: "1.0.0".into(),
        previous_version: None,
        source_git_commit: "abcdef0123456789".into(),
        modules: vec![VersionedErpModule {
            module_key: "catalog".into(),
            version: "1.0.0".into(),
            required: true,
        }],
        capabilities: vec!["catalog.query".into()],
        extension_points: vec![],
        migrations: vec![],
        runtime: None,
        compatibility: ErpReleaseCompatibility {
            minimum_instance_version: "1.0.0".into(),
            required_plugins: vec![],
        },
        rollback: ErpRollbackPlan {
            supported: true,
            instructions: "restore previous instance snapshot".into(),
        },
    }
}
