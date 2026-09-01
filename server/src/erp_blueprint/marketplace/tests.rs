use uuid::Uuid;

use crate::{
    erp_blueprint::{
        model::{
            CreateBlueprintRequest, CreateBlueprintVersionRequest, ErpModuleDefinition,
            ErpReleaseCompatibility, ErpReleaseManifest, ErpRollbackPlan, VersionedErpModule,
            RELEASE_SCHEMA,
        },
        service,
    },
    store::Store,
};

use super::service::CreateMarketplaceInstanceRequest;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_erp_marketplace_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("marketplace test store should open")
}

#[test]
fn public_blueprint_creates_an_independent_project_for_the_current_user() {
    let store = temp_store();
    let maintainer = store
        .create_user("marketplace-owner@example.com", "secret1", None, None)
        .unwrap();
    let merchant = store
        .create_user("marketplace-merchant@example.com", "secret1", None, None)
        .unwrap();
    let source = store
        .create_project(&maintainer.id, "Merchant ERP", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &source.id,
        &maintainer.id,
        CreateBlueprintRequest {
            blueprint_key: "official.merchant.erp".into(),
            name: "Merchant ERP".into(),
            description: "public blueprint".into(),
            modules: vec![ErpModuleDefinition {
                module_key: "catalog".into(),
                version: "1.0.0".into(),
                kind: "core".into(),
                required: true,
                dependencies: Vec::new(),
            }],
            capabilities: Vec::new(),
            themes: vec!["default.clean".into()],
            extension_points: Vec::new(),
            proposal_threshold: 2,
        },
    )
    .unwrap();
    service::publish_version(
        &store,
        &source.id,
        &blueprint.id,
        &maintainer.id,
        CreateBlueprintVersionRequest {
            manifest: ErpReleaseManifest {
                schema: RELEASE_SCHEMA.into(),
                blueprint_key: "official.merchant.erp".into(),
                version: "1.0.0".into(),
                previous_version: None,
                source_git_commit: "8e2bd5739834efc7565dd0b84454facffe875f22".into(),
                modules: vec![VersionedErpModule {
                    module_key: "catalog".into(),
                    version: "1.0.0".into(),
                    required: true,
                }],
                capabilities: Vec::new(),
                extension_points: Vec::new(),
                migrations: Vec::new(),
                runtime: None,
                compatibility: ErpReleaseCompatibility {
                    minimum_instance_version: "1.0.0".into(),
                    required_plugins: Vec::new(),
                },
                rollback: ErpRollbackPlan {
                    supported: true,
                    instructions: "restore the previous immutable release".into(),
                },
            },
        },
    )
    .unwrap();
    store
        .set_project_visibility(&source.id, true, "readonly")
        .unwrap();

    let result = super::create_instance(
        &store,
        &source.id,
        &merchant.id,
        CreateMarketplaceInstanceRequest {
            project_name: "My Store".into(),
            target_project_id: None,
            industry: Some("local_retail".into()),
            theme_key: None,
        },
    )
    .unwrap();

    assert_ne!(result.instance.project_id, source.id);
    assert_eq!(result.instance.created_by, merchant.id);
    assert_eq!(result.instance.theme_key, "default.clean");
    assert_eq!(
        store
            .get_project_access(&merchant.id, &result.instance.project_id)
            .unwrap()
            .role,
        "owner"
    );
    assert!(store.get_project_access(&merchant.id, &source.id).is_err());
}

#[test]
fn unpublished_project_is_not_an_installable_blueprint() {
    let store = temp_store();
    let owner = store
        .create_user("private-blueprint@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Private Blueprint", None, None)
        .unwrap()
        .project;

    let error = super::create_instance(
        &store,
        &project.id,
        &owner.id,
        CreateMarketplaceInstanceRequest {
            project_name: "Should Not Exist".into(),
            target_project_id: None,
            industry: None,
            theme_key: None,
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("未公开"));
}
