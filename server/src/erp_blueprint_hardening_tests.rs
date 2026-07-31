use std::cmp::Ordering;

use uuid::Uuid;

use crate::{
    erp_blueprint::{model::*, service, validation},
    store::Store,
};

fn temp_store() -> Store {
    let path =
        std::env::temp_dir().join(format!("elon_erp_hardening_{}.db", Uuid::new_v4().simple()));
    Store::open(&path).expect("ERP hardening test store should open")
}

#[test]
fn existing_project_name_is_never_reused_or_purged_for_instance_creation() {
    let store = temp_store();
    let owner = store
        .create_user("erp-reuse@example.com", "secret1", Some("Owner"), None)
        .unwrap();
    let blueprint_project = store
        .create_project(&owner.id, "ERP Blueprint Hardening", None, None)
        .unwrap()
        .project;
    let blueprint = service::create_blueprint(
        &store,
        &blueprint_project.id,
        &owner.id,
        blueprint_request(vec![module("catalog", true, vec![])]),
    )
    .unwrap();
    service::publish_version(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateBlueprintVersionRequest {
            manifest: release(vec!["catalog"], vec!["catalog.search"]),
        },
    )
    .unwrap();
    let existing = store
        .create_project(&owner.id, "Existing Merchant Project", None, None)
        .unwrap()
        .project;

    let error = service::create_instance(
        &store,
        &blueprint_project.id,
        &blueprint.id,
        &owner.id,
        CreateErpInstanceRequest {
            instance_key: "merchant.existing".into(),
            project_name: existing.name.clone(),
            version: "1.0.0".into(),
            industry: "retail".into(),
            theme_key: "default.clean".into(),
            enabled_modules: vec![],
            plugins: vec![],
            private_extensions: vec![],
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("同名项目已经存在"));
    assert_eq!(
        store
            .get_project_access(&owner.id, &existing.id)
            .unwrap()
            .id,
        existing.id
    );
}

#[test]
fn blueprint_rejects_dependency_cycles() {
    let store = temp_store();
    let owner = store
        .create_user("erp-cycle@example.com", "secret1", Some("Owner"), None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "ERP Cycle Blueprint", None, None)
        .unwrap()
        .project;
    let error = service::create_blueprint(
        &store,
        &project.id,
        &owner.id,
        blueprint_request(vec![
            module("catalog", true, vec!["inventory"]),
            module("inventory", true, vec!["catalog"]),
        ]),
    )
    .unwrap_err();
    assert!(error.to_string().contains("循环"));
}

#[test]
fn release_must_only_publish_declared_and_reversible_contracts() {
    let definition = validation::build_definition(
        "project",
        blueprint_request(vec![module("catalog", true, vec![])]),
    )
    .unwrap();

    let mut unknown_capability = release(vec!["catalog"], vec!["unknown.capability"]);
    assert!(validation::validate_release(&definition, &unknown_capability).is_err());

    unknown_capability.capabilities = vec!["catalog.search".into()];
    unknown_capability.migrations = vec![ErpMigrationStep {
        migration_key: "catalog.rebuild".into(),
        reversible: false,
    }];
    assert!(
        validation::validate_release(&definition, &unknown_capability)
            .unwrap_err()
            .to_string()
            .contains("所有迁移步骤都必须可逆")
    );
}

#[test]
fn versions_use_numeric_order_and_strict_three_part_format() {
    assert_eq!(
        validation::version_cmp("1.10.0", "1.9.9"),
        Ordering::Greater
    );
    assert!(validation::validate_version("1.0").is_err());
    assert!(validation::validate_version("1.0.0-alpha").is_err());
    assert!(validation::validate_version("01.0.0").is_err());
}

fn blueprint_request(modules: Vec<ErpModuleDefinition>) -> CreateBlueprintRequest {
    CreateBlueprintRequest {
        blueprint_key: "official.hardening".into(),
        name: "ERP Hardening".into(),
        description: String::new(),
        modules,
        capabilities: vec![ErpCapabilityDefinition {
            capability_key: "catalog.search".into(),
            display_name: "查询商品".into(),
            description: "查询商品目录".into(),
            category: "catalog".into(),
            module_key: "catalog".into(),
            aliases: vec!["查商品".into()],
            composable_with: vec![],
        }],
        themes: vec!["default.clean".into()],
        extension_points: vec!["catalog.enrichment".into()],
        proposal_threshold: 2,
    }
}

fn module(key: &str, required: bool, dependencies: Vec<&str>) -> ErpModuleDefinition {
    ErpModuleDefinition {
        module_key: key.into(),
        version: "1.0.0".into(),
        kind: "core".into(),
        required,
        dependencies: dependencies.into_iter().map(str::to_string).collect(),
    }
}

fn release(modules: Vec<&str>, capabilities: Vec<&str>) -> ErpReleaseManifest {
    ErpReleaseManifest {
        schema: RELEASE_SCHEMA.into(),
        blueprint_key: "official.hardening".into(),
        version: "1.0.0".into(),
        previous_version: None,
        source_git_commit: "abcdef0123456789".into(),
        modules: modules
            .into_iter()
            .map(|module_key| VersionedErpModule {
                module_key: module_key.into(),
                version: "1.0.0".into(),
                required: true,
            })
            .collect(),
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
        extension_points: vec!["catalog.enrichment".into()],
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
