use uuid::Uuid;

use crate::{
    erp_blueprint::{model::*, service},
    group_ai::types::ProjectAiBot,
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_erp_blueprint_workflow_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("ERP blueprint workflow store should open")
}

#[test]
fn merchant_upgrade_is_attested_revision_guarded_and_fully_reversible() {
    let (store, owner_id, blueprint_project_id, blueprint, instance) = setup();
    service::publish_version(
        &store,
        &blueprint_project_id,
        &blueprint.id,
        &owner_id,
        CreateBlueprintVersionRequest {
            manifest: release("1.1.0", Some("1.0.0"), true),
        },
    )
    .unwrap();
    let campaign = service::prepare_upgrade(
        &store,
        &blueprint_project_id,
        &instance.id,
        &owner_id,
        PrepareUpgradeRequest {
            target_version: "1.1.0".into(),
        },
    )
    .unwrap();
    assert_eq!(campaign.instance_revision, instance.configuration_revision);
    assert!(campaign
        .target_configuration
        .enabled_modules
        .contains(&"marketing".to_string()));

    let maintainer_denied = service::decide_upgrade(
        &store,
        &blueprint_project_id,
        &campaign.id,
        &owner_id,
        adopt_request(true),
    )
    .unwrap_err();
    assert!(maintainer_denied
        .to_string()
        .contains("只有商户实例所属项目"));

    let unattested = service::decide_upgrade(
        &store,
        &instance.project_id,
        &campaign.id,
        &owner_id,
        adopt_request(false),
    )
    .unwrap_err();
    assert!(unattested.to_string().contains("真实开发"));

    let adopted = service::decide_upgrade(
        &store,
        &instance.project_id,
        &campaign.id,
        &owner_id,
        adopt_request(true),
    )
    .unwrap();
    assert_eq!(adopted.status, "adopted");
    assert!(adopted.adoption_evidence.is_some());
    let upgraded = store.erp_instance(&instance.id).unwrap();
    assert_eq!(upgraded.pinned_version, "1.1.0");
    assert!(upgraded.enabled_modules.contains(&"marketing".to_string()));
    assert_eq!(
        upgraded.configuration_revision,
        instance.configuration_revision + 1
    );

    service::decide_upgrade(
        &store,
        &instance.project_id,
        &campaign.id,
        &owner_id,
        DecideUpgradeRequest {
            action: "rollback".into(),
            reason: "商户验收未达到预期，恢复完整实例配置".into(),
            merchant_confirmed: true,
            execution_attested: false,
            verification_summary: String::new(),
            deployed_commit: None,
        },
    )
    .unwrap();
    let rolled_back = store.erp_instance(&instance.id).unwrap();
    assert_eq!(rolled_back.pinned_version, "1.0.0");
    assert_eq!(rolled_back.theme_key, instance.theme_key);
    assert_eq!(rolled_back.enabled_modules, instance.enabled_modules);
    assert_eq!(rolled_back.plugins, instance.plugins);
    assert_eq!(rolled_back.private_extensions, instance.private_extensions);
}

#[test]
fn instance_bootstrap_matter_is_atomic_and_idempotent() {
    let (store, owner_id, _, _, instance) = setup();
    let bots = vec![ProjectAiBot {
        bot_id: "bot-erp-builder".into(),
        project_id: instance.project_id.clone(),
        provider_user_id: owner_id.clone(),
        node_id: "node-erp-builder".into(),
        display_name: "ERP Builder".into(),
        runtime_route: "codex-cli".into(),
        cli_name: "codex".into(),
        capabilities: vec!["code".into()],
        risk_level: "project_write".into(),
        online: true,
        cli_connected: true,
    }];
    let first = service::create_instance_bootstrap_matter(
        &store,
        &instance.project_id,
        &instance.id,
        &owner_id,
        &bots,
    )
    .unwrap();
    let second = service::create_instance_bootstrap_matter(
        &store,
        &instance.project_id,
        &instance.id,
        &owner_id,
        &bots,
    )
    .unwrap();
    assert_eq!(first.1, second.1);
    assert_eq!(
        second.0.bootstrap_matter_id.as_deref(),
        Some(first.1.as_str())
    );
    let matters = store
        .list_project_ai_matters(&instance.project_id, 100)
        .unwrap();
    assert_eq!(matters.len(), 1);
    assert!(matters[0].brief.contains("不得复制其他商户数据"));
    assert_eq!(matters[0].plan["roles"].as_array().unwrap().len(), 1);
}

fn setup() -> (Store, String, String, ErpBlueprint, ErpInstance) {
    let store = temp_store();
    let owner = store
        .create_user(
            &format!("erp-workflow-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some("ERP Workflow"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "ERP Workflow Blueprint", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &project.id,
        &owner.id,
        CreateBlueprintRequest {
            blueprint_key: "official.erp.workflow".into(),
            name: "Official ERP Workflow".into(),
            description: "Merchant-controlled upgrades".into(),
            modules: vec![
                module("catalog", true),
                module("order", true),
                module("marketing", false),
            ],
            capabilities: vec![capability("catalog.query", "catalog")],
            themes: vec!["merchant.clean".into()],
            extension_points: vec!["order.enrichment".into()],
            proposal_threshold: 2,
        },
    )
    .unwrap();
    service::publish_version(
        &store,
        &project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release("1.0.0", None, false),
        },
    )
    .unwrap();
    let instance = service::create_instance(
        &store,
        &project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: format!("merchant.{}", Uuid::new_v4().simple()),
            project_name: format!("Workflow Merchant {}", Uuid::new_v4().simple()),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "merchant.clean".into(),
            enabled_modules: vec!["catalog".into(), "order".into()],
            plugins: vec![],
            private_extensions: vec![ErpExtensionRef {
                extension_key: "merchant.order_note".into(),
                version: "1.0.0".into(),
                extension_point: "order.enrichment".into(),
                requires_modules: vec!["order".into()],
            }],
        },
    )
    .unwrap();
    (store, owner.id, project.id, blueprint, instance)
}

fn adopt_request(attested: bool) -> DecideUpgradeRequest {
    DecideUpgradeRequest {
        action: "adopt".into(),
        reason: String::new(),
        merchant_confirmed: true,
        execution_attested: attested,
        verification_summary: "已完成参考实例测试、迁移验证与商户人工验收".into(),
        deployed_commit: Some("abcdef0123456789".into()),
    }
}

fn module(key: &str, required: bool) -> ErpModuleDefinition {
    ErpModuleDefinition {
        module_key: key.into(),
        version: "1.0.0".into(),
        kind: "core".into(),
        required,
        dependencies: vec![],
    }
}

fn capability(key: &str, module_key: &str) -> ErpCapabilityDefinition {
    ErpCapabilityDefinition {
        capability_key: key.into(),
        display_name: key.into(),
        description: key.into(),
        category: module_key.into(),
        module_key: module_key.into(),
        aliases: vec![],
        composable_with: vec![],
    }
}

fn release(version: &str, previous: Option<&str>, require_marketing: bool) -> ErpReleaseManifest {
    let mut modules = vec![versioned("catalog", true), versioned("order", true)];
    if require_marketing {
        modules.push(versioned("marketing", true));
    }
    ErpReleaseManifest {
        schema: RELEASE_SCHEMA.into(),
        blueprint_key: "official.erp.workflow".into(),
        version: version.into(),
        previous_version: previous.map(str::to_string),
        source_git_commit: "abcdef0123456789".into(),
        modules,
        capabilities: vec!["catalog.query".into()],
        extension_points: vec!["order.enrichment".into()],
        migrations: vec![],
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

fn versioned(key: &str, required: bool) -> VersionedErpModule {
    VersionedErpModule {
        module_key: key.into(),
        version: "1.0.0".into(),
        required,
    }
}
