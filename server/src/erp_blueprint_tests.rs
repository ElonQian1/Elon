use uuid::Uuid;

use crate::{
    erp_blueprint::{model::*, service},
    store::Store,
};

fn temp_store() -> Store {
    let path =
        std::env::temp_dir().join(format!("elon_erp_blueprint_{}.db", Uuid::new_v4().simple()));
    Store::open(&path).expect("ERP blueprint test store should open")
}

#[test]
fn shared_blueprint_keeps_instances_independent_and_governs_common_proposals() {
    let store = temp_store();
    let owner = store
        .create_user("erp-owner@example.com", "secret1", Some("ERP Owner"), None)
        .unwrap();
    let blueprint_project = store
        .create_project(&owner.id, "Official ERP Blueprint", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &blueprint_project.id,
        &owner.id,
        blueprint_request(2),
    )
    .unwrap();
    service::publish_version(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release("1.0.0", None),
        },
    )
    .unwrap();
    let invalid_previous = service::publish_version(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release("1.2.0", Some("1.0.5")),
        },
    )
    .unwrap_err();
    assert!(invalid_previous.to_string().contains("当前最新发布版本"));
    service::publish_version(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release("1.1.0", Some("1.0.0")),
        },
    )
    .unwrap();

    let coffee_private = ErpExtensionRef {
        extension_key: "cofficethinking.roast_profile".into(),
        version: "1.0.0".into(),
        extension_point: "order.enrichment".into(),
        requires_modules: vec!["order".into()],
    };
    let coffee = service::create_instance(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: "cofficethinking.store".into(),
            project_name: "Coffee Thinking ERP".into(),
            version: "1.0.0".into(),
            industry: "coffee".into(),
            theme_key: "coffee.warm".into(),
            enabled_modules: vec![],
            plugins: vec![ErpExtensionRef {
                extension_key: "coffee.bean_catalog".into(),
                version: "1.0.0".into(),
                extension_point: "catalog.enrichment".into(),
                requires_modules: vec!["catalog".into()],
            }],
            private_extensions: vec![coffee_private.clone()],
        },
    )
    .unwrap();
    let retail = service::create_instance(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: "retail.minimum".into(),
            project_name: "Minimal Retail ERP".into(),
            version: "1.0.0".into(),
            industry: "convenience_retail".into(),
            theme_key: "retail.fresh".into(),
            enabled_modules: vec![],
            plugins: vec![ErpExtensionRef {
                extension_key: "retail.barcode".into(),
                version: "1.0.0".into(),
                extension_point: "catalog.enrichment".into(),
                requires_modules: vec!["catalog".into(), "inventory".into()],
            }],
            private_extensions: vec![],
        },
    )
    .unwrap();
    assert_ne!(coffee.project_id, retail.project_id);
    assert_ne!(coffee.theme_key, retail.theme_key);
    assert_ne!(coffee.plugins, retail.plugins);

    let need_key = "need.expiry_restock";
    submit_signal(
        &store,
        &coffee,
        &owner.id,
        need_key,
        "咖啡豆临期自动补货建议",
    );
    submit_signal(
        &store,
        &coffee,
        &owner.id,
        need_key,
        "咖啡豆临期补货建议更新",
    );
    let one_support = store.list_erp_feature_proposals(&blueprint.id).unwrap();
    assert_eq!(
        one_support[0].support_count, 1,
        "same merchant must only count once"
    );
    submit_signal(&store, &retail, &owner.id, need_key, "商品临期自动补货建议");
    let proposal = store
        .list_erp_feature_proposals(&blueprint.id)
        .unwrap()
        .remove(0);
    assert_eq!(proposal.support_count, 2);

    let (proposal, matter_id) = service::decide_proposal(
        &store,
        &blueprint_project.id,
        &proposal.id,
        &owner.id,
        DecideProposalRequest {
            decision: "accepted".into(),
            note: "两个独立行业实例验证".into(),
            create_matter: true,
        },
    )
    .unwrap();
    assert_eq!(proposal.status, "matter_created");
    assert!(matter_id.is_some());
    let matter = store
        .get_project_ai_matter(&blueprint_project.id, matter_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(matter.status, "plan_ready");
    assert!(matter.brief.contains("不得自动合并"));

    let campaign = service::prepare_upgrade(
        &store,
        &coffee.project_id,
        &coffee.id,
        &owner.id,
        PrepareUpgradeRequest {
            target_version: "1.1.0".into(),
        },
    )
    .unwrap();
    assert_eq!(campaign.status, "ready");
    assert_eq!(
        campaign.private_extensions_snapshot,
        vec![coffee_private.clone()]
    );
    service::decide_upgrade(
        &store,
        &coffee.project_id,
        &campaign.id,
        &owner.id,
        DecideUpgradeRequest {
            action: "adopt".into(),
            reason: String::new(),
        },
    )
    .unwrap();
    assert_eq!(
        store.erp_instance(&coffee.id).unwrap().pinned_version,
        "1.1.0"
    );
    let downgrade = service::prepare_upgrade(
        &store,
        &coffee.project_id,
        &coffee.id,
        &owner.id,
        PrepareUpgradeRequest {
            target_version: "1.0.0".into(),
        },
    )
    .unwrap_err();
    assert!(downgrade.to_string().contains("必须高于"));
    let missing_reason = service::decide_upgrade(
        &store,
        &coffee.project_id,
        &campaign.id,
        &owner.id,
        DecideUpgradeRequest {
            action: "rollback".into(),
            reason: String::new(),
        },
    )
    .unwrap_err();
    assert!(missing_reason.to_string().contains("回滚必须填写原因"));
    service::decide_upgrade(
        &store,
        &coffee.project_id,
        &campaign.id,
        &owner.id,
        DecideUpgradeRequest {
            action: "rollback".into(),
            reason: "reference rollback".into(),
        },
    )
    .unwrap();
    let rolled_back = store.erp_instance(&coffee.id).unwrap();
    assert_eq!(rolled_back.pinned_version, "1.0.0");
    assert_eq!(rolled_back.private_extensions, vec![coffee_private]);
}

#[test]
fn signal_rejects_secrets_and_existing_capability_avoids_proposal() {
    let store = temp_store();
    let owner = store
        .create_user(
            "erp-safety@example.com",
            "secret1",
            Some("ERP Safety"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "ERP Safety Blueprint", None, None)
        .unwrap()
        .project;
    let blueprint =
        service::create_blueprint(&store, &project.id, &owner.id, blueprint_request(2)).unwrap();
    service::publish_version(
        &store,
        &project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release("1.0.0", None),
        },
    )
    .unwrap();
    let instance = service::create_instance(
        &store,
        &project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: "safety.store".into(),
            project_name: "Safety Merchant".into(),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "default.clean".into(),
            enabled_modules: vec![],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap();
    let resolution = service::resolve_requirement(
        &store,
        &instance.project_id,
        ResolveRequirementRequest {
            instance_id: Some(instance.id.clone()),
            requirement: "帮我查库存".into(),
            expected_scope: Some("potential_common".into()),
        },
    )
    .unwrap();
    assert_eq!(resolution.classification, "existing");
    assert!(!resolution.may_submit_signal);

    let error = service::submit_signal(
        &store,
        &instance.project_id,
        &instance.id,
        &owner.id,
        SubmitFeatureSignalRequest {
            schema: SIGNAL_SCHEMA.into(),
            requirement_summary: "把 api_token secret 上传后分析库存".into(),
            need_key: None,
            industry: "retail".into(),
            requested_outcome: String::new(),
            merchant_authorized: true,
            classification: "sanitized_aggregate".into(),
            evidence: FeatureSignalEvidence::default(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("敏感"));
}

fn submit_signal(store: &Store, instance: &ErpInstance, user_id: &str, need_key: &str, text: &str) {
    service::submit_signal(
        store,
        &instance.project_id,
        &instance.id,
        user_id,
        SubmitFeatureSignalRequest {
            schema: SIGNAL_SCHEMA.into(),
            requirement_summary: text.into(),
            need_key: Some(need_key.into()),
            industry: instance.industry.clone(),
            requested_outcome: "减少人工补货判断".into(),
            merchant_authorized: true,
            classification: "sanitized_aggregate".into(),
            evidence: FeatureSignalEvidence {
                occurrence_count: Some(3),
                affected_workflow: Some("inventory.restock".into()),
                estimated_time_saved_minutes: Some(30),
            },
        },
    )
    .unwrap();
}

fn blueprint_request(threshold: i64) -> CreateBlueprintRequest {
    CreateBlueprintRequest {
        blueprint_key: "official.erp".into(),
        name: "Official ERP".into(),
        description: "Shared core".into(),
        modules: vec![
            module("catalog", true),
            module("order", true),
            module("inventory", true),
        ],
        capabilities: vec![ErpCapabilityDefinition {
            capability_key: "inventory.query".into(),
            display_name: "查询库存".into(),
            description: "查询当前库存".into(),
            category: "inventory".into(),
            module_key: "inventory".into(),
            aliases: vec!["查库存".into()],
            composable_with: vec![],
        }],
        themes: vec![
            "default.clean".into(),
            "coffee.warm".into(),
            "retail.fresh".into(),
        ],
        extension_points: vec!["catalog.enrichment".into(), "order.enrichment".into()],
        proposal_threshold: threshold,
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

fn release(version: &str, previous: Option<&str>) -> ErpReleaseManifest {
    ErpReleaseManifest {
        schema: RELEASE_SCHEMA.into(),
        blueprint_key: "official.erp".into(),
        version: version.into(),
        previous_version: previous.map(str::to_string),
        source_git_commit: "abcdef0123456789".into(),
        modules: vec![
            versioned("catalog"),
            versioned("order"),
            versioned("inventory"),
        ],
        capabilities: vec!["inventory.query".into()],
        extension_points: vec!["catalog.enrichment".into(), "order.enrichment".into()],
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

fn versioned(key: &str) -> VersionedErpModule {
    VersionedErpModule {
        module_key: key.into(),
        version: "1.0.0".into(),
        required: true,
    }
}
