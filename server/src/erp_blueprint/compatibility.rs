use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    CompatibilityIssue, ErpBlueprintVersion, ErpCompatibilityReport, ErpInstance,
    ErpInstanceConfiguration,
};

pub(crate) struct UpgradePreparation {
    pub report: ErpCompatibilityReport,
    pub target_configuration: ErpInstanceConfiguration,
}

pub(crate) fn check(
    instance: &ErpInstance,
    target: &ErpBlueprintVersion,
) -> ErpCompatibilityReport {
    prepare(instance, target).report
}

pub(crate) fn prepare(instance: &ErpInstance, target: &ErpBlueprintVersion) -> UpgradePreparation {
    let manifest = &target.manifest;
    let mut issues = Vec::new();
    if version_less_than(
        &instance.pinned_version,
        &manifest.compatibility.minimum_instance_version,
    ) {
        issues.push(issue(
            "minimum_version",
            "blocking",
            &instance.pinned_version,
            "当前实例版本低于目标版本允许的最低升级起点",
        ));
    }

    let target_modules: BTreeMap<_, _> = manifest
        .modules
        .iter()
        .map(|module| (module.module_key.as_str(), module))
        .collect();
    for module in &instance.enabled_modules {
        if !target_modules.contains_key(module.as_str()) {
            issues.push(issue(
                "enabled_module_removed",
                "blocking",
                module,
                "目标版本移除了实例当前启用的模块，不能静默丢弃配置",
            ));
        }
    }
    let mut target_enabled_modules = instance.enabled_modules.clone();
    for module in manifest.modules.iter().filter(|module| module.required) {
        if !instance
            .enabled_modules
            .iter()
            .any(|key| key == &module.module_key)
        {
            issues.push(issue(
                "required_module_added",
                "warning",
                &module.module_key,
                "目标版本会增加一个必需公共模块，采用前应由项目开发流程安装并验证",
            ));
            target_enabled_modules.push(module.module_key.clone());
        }
    }
    target_enabled_modules.sort();
    target_enabled_modules.dedup();

    let extension_points: BTreeSet<_> = manifest
        .extension_points
        .iter()
        .map(String::as_str)
        .collect();
    for extension in instance
        .plugins
        .iter()
        .chain(instance.private_extensions.iter())
    {
        if !extension_points.contains(extension.extension_point.as_str()) {
            issues.push(issue(
                "extension_point_removed",
                "blocking",
                &extension.extension_key,
                "目标版本未保留该扩展使用的扩展点",
            ));
        }
        for required in &extension.requires_modules {
            if !target_modules.contains_key(required.as_str()) {
                issues.push(issue(
                    "extension_module_missing",
                    "blocking",
                    &extension.extension_key,
                    &format!("目标版本缺少扩展依赖模块 {required}"),
                ));
            }
        }
    }

    let installed_plugins: BTreeSet<_> = instance
        .plugins
        .iter()
        .map(|plugin| plugin.extension_key.as_str())
        .collect();
    for required in &manifest.compatibility.required_plugins {
        if !installed_plugins.contains(required.as_str()) {
            issues.push(issue(
                "required_plugin_missing",
                "blocking",
                required,
                "目标版本要求的插件尚未安装",
            ));
        }
    }
    if !manifest.rollback.supported {
        issues.push(issue(
            "rollback_not_supported",
            "blocking",
            &manifest.version,
            "该发布清单不支持回滚，V1 不允许商户实例采用",
        ));
    }

    UpgradePreparation {
        report: ErpCompatibilityReport {
            compatible: !issues.iter().any(|item| item.severity == "blocking"),
            from_version: instance.pinned_version.clone(),
            target_version: manifest.version.clone(),
            preserved_private_extensions: instance.private_extensions.clone(),
            issues,
        },
        target_configuration: ErpInstanceConfiguration {
            theme_key: instance.theme_key.clone(),
            enabled_modules: target_enabled_modules,
            plugins: instance.plugins.clone(),
        },
    }
}

fn version_less_than(left: &str, right: &str) -> bool {
    parse_version(left) < parse_version(right)
}

fn parse_version(value: &str) -> (u64, u64, u64) {
    let mut parts = value
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn issue(code: &str, severity: &str, subject: &str, message: &str) -> CompatibilityIssue {
    CompatibilityIssue {
        code: code.to_string(),
        severity: severity.to_string(),
        subject: subject.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erp_blueprint::model::*;

    #[test]
    fn private_extension_is_preserved_and_missing_extension_point_blocks() {
        let private = ErpExtensionRef {
            extension_key: "coffee.roast_profile".into(),
            version: "1.0.0".into(),
            extension_point: "order.enrichment".into(),
            requires_modules: vec!["order".into()],
        };
        let instance = ErpInstance {
            id: "instance".into(),
            instance_key: "coffee.demo".into(),
            project_id: "project".into(),
            blueprint_id: "blueprint".into(),
            pinned_version_id: "v1".into(),
            pinned_version: "1.0.0".into(),
            industry: "coffee".into(),
            theme_key: "coffee.dark".into(),
            enabled_modules: vec!["order".into()],
            plugins: vec![],
            private_extensions: vec![private.clone()],
            configuration_revision: 1,
            bootstrap_matter_id: None,
            status: "active".into(),
            created_by: "owner".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        let target = ErpBlueprintVersion {
            id: "v2".into(),
            blueprint_id: "blueprint".into(),
            manifest: ErpReleaseManifest {
                schema: RELEASE_SCHEMA.into(),
                blueprint_key: "official.erp".into(),
                version: "1.1.0".into(),
                previous_version: Some("1.0.0".into()),
                source_git_commit: "abcdef0".into(),
                modules: vec![VersionedErpModule {
                    module_key: "order".into(),
                    version: "1.1.0".into(),
                    required: true,
                }],
                capabilities: vec![],
                extension_points: vec![],
                migrations: vec![],
                compatibility: ErpReleaseCompatibility {
                    minimum_instance_version: "1.0.0".into(),
                    required_plugins: vec![],
                },
                rollback: ErpRollbackPlan {
                    supported: true,
                    instructions: "restore".into(),
                },
            },
            manifest_sha256: "hash".into(),
            status: "published".into(),
            created_by: "owner".into(),
            created_at: "now".into(),
        };
        let report = check(&instance, &target);
        assert!(!report.compatible);
        assert_eq!(report.preserved_private_extensions, vec![private]);
    }
}
