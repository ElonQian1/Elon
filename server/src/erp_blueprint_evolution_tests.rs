use uuid::Uuid;

use crate::{
    erp_blueprint::{catalog_service, instance_service, model::*, service},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_erp_blueprint_evolution_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("ERP blueprint evolution store should open")
}

#[test]
fn draft_capabilities_are_not_exposed_before_the_first_release() {
    let (store, _, project_id, _) = setup_blueprint();
    let catalog = catalog_service::catalog_for_project(&store, &project_id, None).unwrap();
    assert_eq!(catalog.version, None);
    assert!(catalog.capabilities.is_empty());
    assert_eq!(catalog.unreleased_capability_keys, vec!["catalog.query"]);
}

#[test]
fn blueprint_evolution_is_revision_guarded_and_release_scoped() {
    let (store, owner_id, project_id, blueprint) = setup_blueprint();
    let v1 = release("1.0.0", None, false);
    service::publish_version(
        &store,
        &project_id,
        &blueprint.id,
        &owner_id,
        CreateBlueprintVersionRequest { manifest: v1 },
    )
    .unwrap();
    let instance = create_instance(&store, &owner_id, &project_id, &blueprint.id);

    let evolved = catalog_service::evolve_blueprint(
        &store,
        &project_id,
        &blueprint.id,
        EvolveBlueprintRequest {
            expected_revision: blueprint.definition_revision,
            name: None,
            description: None,
            proposal_threshold: None,
            add_modules: vec![],
            add_capabilities: vec![capability(
                "marketing.poster.generate",
                "生成营销海报",
                "marketing",
            )],
            add_themes: vec![],
            add_extension_points: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        evolved.definition_revision,
        blueprint.definition_revision + 1
    );

    let stale = catalog_service::evolve_blueprint(
        &store,
        &project_id,
        &blueprint.id,
        EvolveBlueprintRequest {
            expected_revision: blueprint.definition_revision,
            name: Some("stale update".into()),
            description: None,
            proposal_threshold: None,
            add_modules: vec![],
            add_capabilities: vec![],
            add_themes: vec![],
            add_extension_points: vec![],
        },
    )
    .unwrap_err();
    assert!(stale.to_string().contains("请刷新后重试"));

    let pinned = catalog_service::catalog_for_project(&store, &instance.project_id, None).unwrap();
    assert_eq!(pinned.version.as_deref(), Some("1.0.0"));
    assert_eq!(pinned.capabilities.len(), 1);
    assert_eq!(
        pinned.unreleased_capability_keys,
        vec!["marketing.poster.generate"]
    );

    service::publish_version(
        &store,
        &project_id,
        &blueprint.id,
        &owner_id,
        CreateBlueprintVersionRequest {
            manifest: release("1.1.0", Some("1.0.0"), true),
        },
    )
    .unwrap();
    let still_pinned =
        catalog_service::catalog_for_project(&store, &instance.project_id, None).unwrap();
    assert_eq!(still_pinned.version.as_deref(), Some("1.0.0"));
    assert_eq!(still_pinned.capabilities.len(), 1);
    let maintainer = catalog_service::catalog_for_project(&store, &project_id, None).unwrap();
    assert_eq!(maintainer.version.as_deref(), Some("1.1.0"));
    assert_eq!(maintainer.capabilities.len(), 2);
}

#[test]
fn merchant_configuration_requires_confirmation_and_current_revision() {
    let (store, owner_id, project_id, blueprint) = setup_blueprint();
    service::publish_version(
        &store,
        &project_id,
        &blueprint.id,
        &owner_id,
        CreateBlueprintVersionRequest {
            manifest: release("1.0.0", None, false),
        },
    )
    .unwrap();
    let instance = create_instance(&store, &owner_id, &project_id, &blueprint.id);
    let private_extension = ErpExtensionRef {
        extension_key: "merchant.loyalty_rule".into(),
        version: "1.0.0".into(),
        extension_point: "order.enrichment".into(),
        requires_modules: vec!["order".into()],
    };
    let denied = instance_service::update_configuration(
        &store,
        &instance.project_id,
        &instance.id,
        UpdateErpInstanceRequest {
            expected_revision: instance.configuration_revision,
            merchant_confirmed: false,
            theme_key: "merchant.dark".into(),
            enabled_modules: vec!["catalog".into(), "order".into()],
            plugins: vec![],
            private_extensions: vec![private_extension.clone()],
        },
    )
    .unwrap_err();
    assert!(denied.to_string().contains("商户未确认"));

    let updated = instance_service::update_configuration(
        &store,
        &instance.project_id,
        &instance.id,
        UpdateErpInstanceRequest {
            expected_revision: instance.configuration_revision,
            merchant_confirmed: true,
            theme_key: "merchant.dark".into(),
            enabled_modules: vec!["catalog".into(), "order".into()],
            plugins: vec![],
            private_extensions: vec![private_extension.clone()],
        },
    )
    .unwrap();
    assert_eq!(
        updated.configuration_revision,
        instance.configuration_revision + 1
    );
    assert_eq!(updated.theme_key, "merchant.dark");
    assert_eq!(updated.private_extensions, vec![private_extension]);

    let unchanged = instance_service::update_configuration(
        &store,
        &instance.project_id,
        &instance.id,
        UpdateErpInstanceRequest {
            expected_revision: updated.configuration_revision,
            merchant_confirmed: true,
            theme_key: updated.theme_key.clone(),
            enabled_modules: updated.enabled_modules.clone(),
            plugins: updated.plugins.clone(),
            private_extensions: updated.private_extensions.clone(),
        },
    )
    .unwrap_err();
    assert!(unchanged.to_string().contains("没有变化"));

    let stale = instance_service::update_configuration(
        &store,
        &instance.project_id,
        &instance.id,
        UpdateErpInstanceRequest {
            expected_revision: instance.configuration_revision,
            merchant_confirmed: true,
            theme_key: "default.clean".into(),
            enabled_modules: vec!["catalog".into(), "order".into()],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap_err();
    assert!(stale.to_string().contains("请刷新后重试"));
}

fn setup_blueprint() -> (Store, String, String, ErpBlueprint) {
    let store = temp_store();
    let owner = store
        .create_user(
            &format!("erp-evolution-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some("ERP Evolution"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "ERP Evolution Blueprint", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &project.id,
        &owner.id,
        CreateBlueprintRequest {
            blueprint_key: "official.erp.evolution".into(),
            name: "Official ERP Evolution".into(),
            description: "Version-scoped capability catalog".into(),
            modules: vec![module("catalog"), module("order"), module("marketing")],
            capabilities: vec![capability("catalog.query", "查询商品", "catalog")],
            themes: vec!["default.clean".into(), "merchant.dark".into()],
            extension_points: vec!["order.enrichment".into()],
            proposal_threshold: 2,
        },
    )
    .unwrap();
    (store, owner.id, project.id, blueprint)
}

fn create_instance(
    store: &Store,
    owner_id: &str,
    project_id: &str,
    blueprint_id: &str,
) -> ErpInstance {
    service::create_instance(
        store,
        project_id,
        blueprint_id,
        owner_id,
        CreateErpInstanceRequest {
            instance_key: format!("merchant.{}", Uuid::new_v4().simple()),
            project_name: format!("Merchant ERP {}", Uuid::new_v4().simple()),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "default.clean".into(),
            enabled_modules: vec!["catalog".into(), "order".into()],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap()
}

fn module(key: &str) -> ErpModuleDefinition {
    ErpModuleDefinition {
        module_key: key.into(),
        version: "1.0.0".into(),
        kind: "core".into(),
        required: key != "marketing",
        dependencies: vec![],
    }
}

fn capability(key: &str, name: &str, module_key: &str) -> ErpCapabilityDefinition {
    ErpCapabilityDefinition {
        capability_key: key.into(),
        display_name: name.into(),
        description: name.into(),
        category: module_key.into(),
        module_key: module_key.into(),
        aliases: vec![name.into()],
        composable_with: vec![],
    }
}

fn release(version: &str, previous: Option<&str>, include_marketing: bool) -> ErpReleaseManifest {
    let mut modules = vec![versioned("catalog", true), versioned("order", true)];
    let mut capabilities = vec!["catalog.query".into()];
    if include_marketing {
        modules.push(versioned("marketing", false));
        capabilities.push("marketing.poster.generate".into());
    }
    ErpReleaseManifest {
        schema: RELEASE_SCHEMA.into(),
        blueprint_key: "official.erp.evolution".into(),
        version: version.into(),
        previous_version: previous.map(str::to_string),
        source_git_commit: "abcdef0123456789".into(),
        modules,
        capabilities,
        extension_points: vec!["order.enrichment".into()],
        migrations: vec![],
        compatibility: ErpReleaseCompatibility {
            minimum_instance_version: "1.0.0".into(),
            required_plugins: vec![],
        },
        rollback: ErpRollbackPlan {
            supported: true,
            instructions: "restore previous version".into(),
        },
    }
}

fn versioned(key: &str, required: bool) -> VersionedErpModule {
    VersionedErpModule {
        module_key: key.into(),
        version: "1.0.0".into(),
        required,
    }
}
