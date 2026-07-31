use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    CreateBlueprintRequest, ErpBlueprintDefinition, ErpCapabilityDefinition, ErpExtensionRef,
    ErpReleaseManifest, FeatureSignalEvidence, SubmitFeatureSignalRequest, BLUEPRINT_SCHEMA,
    RELEASE_SCHEMA, SIGNAL_SCHEMA,
};

pub(crate) fn build_definition(
    project_id: &str,
    mut request: CreateBlueprintRequest,
) -> Result<ErpBlueprintDefinition> {
    let blueprint_key = normalize_key(&request.blueprint_key, "blueprint_key")?;
    let name = required_text(&request.name, "蓝图名称", 120)?;
    if !(2..=100).contains(&request.proposal_threshold) {
        bail!("通用提案阈值必须在 2 到 100 之间");
    }
    if request.modules.is_empty() || request.themes.is_empty() {
        bail!("蓝图至少需要一个模块和一个主题");
    }
    for module in &mut request.modules {
        module.module_key = normalize_key(&module.module_key, "module_key")?;
        module.kind = module.kind.trim().to_ascii_lowercase();
        module.version = module.version.trim().to_string();
        module.dependencies =
            normalize_key_list(std::mem::take(&mut module.dependencies), "模块依赖")?;
        validate_version(&module.version)?;
        if !matches!(module.kind.as_str(), "core" | "industry" | "integration") {
            bail!("模块 kind 只能是 core、industry 或 integration");
        }
    }
    unique_keys(
        request.modules.iter().map(|item| item.module_key.as_str()),
        "模块",
    )?;
    for capability in &mut request.capabilities {
        capability.capability_key = normalize_key(&capability.capability_key, "capability_key")?;
        capability.category = normalize_key(&capability.category, "category")?;
        capability.module_key = normalize_key(&capability.module_key, "module_key")?;
        capability.display_name = required_text(&capability.display_name, "能力名称", 120)?;
        capability.description = capability.description.trim().chars().take(500).collect();
        capability.aliases = normalize_aliases(std::mem::take(&mut capability.aliases))?;
        capability.composable_with = normalize_key_list(
            std::mem::take(&mut capability.composable_with),
            "可组合能力",
        )?;
    }
    unique_keys(
        request
            .capabilities
            .iter()
            .map(|item| item.capability_key.as_str()),
        "能力",
    )?;
    let module_keys: BTreeSet<_> = request
        .modules
        .iter()
        .map(|m| m.module_key.as_str())
        .collect();
    for module in &request.modules {
        for dependency in &module.dependencies {
            if dependency == &module.module_key || !module_keys.contains(dependency.as_str()) {
                bail!(
                    "模块 {} 的依赖 {} 不存在或形成自依赖",
                    module.module_key,
                    dependency
                );
            }
        }
    }
    validate_module_dependency_graph(&request.modules)?;
    let capability_keys: BTreeSet<_> = request
        .capabilities
        .iter()
        .map(|item| item.capability_key.as_str())
        .collect();
    for capability in &request.capabilities {
        validate_capability(capability, &module_keys)?;
        for dependency in &capability.composable_with {
            if dependency == &capability.capability_key
                || !capability_keys.contains(dependency.as_str())
            {
                bail!(
                    "能力 {} 的组合依赖 {} 不存在或形成自依赖",
                    capability.capability_key,
                    dependency
                );
            }
        }
    }
    let themes = normalize_key_list(request.themes, "主题")?;
    let extension_points = normalize_key_list(request.extension_points, "扩展点")?;
    Ok(ErpBlueprintDefinition {
        schema: BLUEPRINT_SCHEMA.to_string(),
        blueprint_key,
        name,
        description: request.description.trim().chars().take(2000).collect(),
        source_project_id: project_id.trim().to_string(),
        modules: request.modules,
        capabilities: request.capabilities,
        themes,
        extension_points,
        proposal_threshold: request.proposal_threshold,
    })
}

pub(crate) fn validate_release(
    definition: &ErpBlueprintDefinition,
    manifest: &ErpReleaseManifest,
) -> Result<()> {
    if manifest.schema != RELEASE_SCHEMA || manifest.blueprint_key != definition.blueprint_key {
        bail!("发布清单 schema 或 blueprint_key 与蓝图不一致");
    }
    validate_version(&manifest.version)?;
    if let Some(previous) = manifest.previous_version.as_deref() {
        validate_version(previous)?;
        if !version_is_newer(&manifest.version, previous) {
            bail!("发布版本必须高于 previous_version");
        }
    }
    if !(7..=64).contains(&manifest.source_git_commit.len())
        || !manifest
            .source_git_commit
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        bail!("source_git_commit 必须是 7 到 64 位十六进制提交标识");
    }
    if manifest.modules.is_empty() {
        bail!("发布清单至少需要一个模块");
    }
    unique_keys(
        manifest.modules.iter().map(|item| item.module_key.as_str()),
        "发布模块",
    )?;
    for module in &manifest.modules {
        require_normalized_key(&module.module_key, "module_key")?;
        validate_version(&module.version)?;
    }
    let definition_modules: BTreeMap<_, _> = definition
        .modules
        .iter()
        .map(|module| (module.module_key.as_str(), module))
        .collect();
    let release_modules: BTreeSet<_> = manifest
        .modules
        .iter()
        .map(|module| module.module_key.as_str())
        .collect();
    for module in &manifest.modules {
        if !definition_modules.contains_key(module.module_key.as_str()) {
            bail!("发布模块 {} 尚未登记到蓝图定义", module.module_key);
        }
    }
    for module in definition.modules.iter().filter(|module| module.required) {
        if !release_modules.contains(module.module_key.as_str()) {
            bail!("发布清单缺少蓝图必需模块 {}", module.module_key);
        }
    }
    unique_keys(manifest.capabilities.iter().map(String::as_str), "发布能力")?;
    let definition_capabilities: BTreeMap<_, _> = definition
        .capabilities
        .iter()
        .map(|capability| (capability.capability_key.as_str(), capability))
        .collect();
    for capability in &manifest.capabilities {
        require_normalized_key(capability, "capability_key")?;
        let definition_capability = definition_capabilities
            .get(capability.as_str())
            .ok_or_else(|| anyhow::anyhow!("发布能力 {capability} 尚未登记到蓝图定义"))?;
        if !release_modules.contains(definition_capability.module_key.as_str()) {
            bail!(
                "发布能力 {} 所属模块 {} 未包含在发布清单中",
                capability,
                definition_capability.module_key
            );
        }
    }
    unique_keys(
        manifest.extension_points.iter().map(String::as_str),
        "发布扩展点",
    )?;
    for extension_point in &manifest.extension_points {
        require_normalized_key(extension_point, "extension_point")?;
        if !definition.extension_points.contains(extension_point) {
            bail!("发布扩展点 {extension_point} 尚未登记到蓝图定义");
        }
    }
    validate_version(&manifest.compatibility.minimum_instance_version)?;
    unique_keys(
        manifest
            .compatibility
            .required_plugins
            .iter()
            .map(String::as_str),
        "必需插件",
    )?;
    for plugin in &manifest.compatibility.required_plugins {
        require_normalized_key(plugin, "required_plugin")?;
    }
    unique_keys(
        manifest
            .migrations
            .iter()
            .map(|item| item.migration_key.as_str()),
        "迁移",
    )?;
    for migration in &manifest.migrations {
        require_normalized_key(&migration.migration_key, "migration_key")?;
    }
    if manifest.rollback.instructions.trim().is_empty() {
        bail!("发布清单必须提供回滚说明");
    }
    if manifest.rollback.supported && manifest.migrations.iter().any(|step| !step.reversible) {
        bail!("发布清单声明支持回滚时，所有迁移步骤都必须可逆");
    }
    Ok(())
}

pub(crate) fn validate_extensions(
    values: &[ErpExtensionRef],
    extension_points: &BTreeSet<&str>,
    enabled_modules: &BTreeSet<&str>,
) -> Result<()> {
    unique_keys(
        values.iter().map(|item| item.extension_key.as_str()),
        "扩展",
    )?;
    for value in values {
        require_normalized_key(&value.extension_key, "extension_key")?;
        validate_version(&value.version)?;
        require_normalized_key(&value.extension_point, "extension_point")?;
        if !extension_points.contains(value.extension_point.as_str()) {
            bail!("扩展 {} 使用了蓝图未声明的扩展点", value.extension_key);
        }
        for module in &value.requires_modules {
            require_normalized_key(module, "requires_module")?;
            if !enabled_modules.contains(module.as_str()) {
                bail!("扩展 {} 依赖未启用模块 {}", value.extension_key, module);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_signal(request: &SubmitFeatureSignalRequest) -> Result<()> {
    if request.schema != SIGNAL_SCHEMA {
        bail!("需求信号 schema 必须是 {SIGNAL_SCHEMA}");
    }
    if !request.merchant_authorized {
        bail!("商户未明确授权，不能提交通用需求信号");
    }
    if !matches!(
        request.classification.as_str(),
        "sanitized_aggregate" | "public_requirement"
    ) {
        bail!("需求信号只能是脱敏汇总或公开需求");
    }
    required_text(&request.requirement_summary, "需求摘要", 500)?;
    required_text(&request.industry, "行业", 80)?;
    reject_sensitive_text(&request.requirement_summary)?;
    reject_sensitive_text(&request.requested_outcome)?;
    validate_evidence(&request.evidence)
}

pub(crate) fn normalize_key(value: &str, field: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() < 2 || value.len() > 80 {
        bail!("{field} 长度必须在 2 到 80 之间");
    }
    if !value.chars().enumerate().all(|(index, ch)| {
        ch.is_ascii_lowercase() || ch.is_ascii_digit() || (index > 0 && "._-".contains(ch))
    }) {
        bail!("{field} 只能使用小写字母、数字、点、下划线和连字符");
    }
    Ok(value)
}

pub(crate) fn stable_need_key(summary: &str) -> String {
    let normalized = summary
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    format!("need.{}", &hex::encode(digest)[..16])
}

pub(crate) fn manifest_hash(manifest: &ErpReleaseManifest) -> Result<String> {
    let encoded = serde_json::to_vec(manifest)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub(crate) fn validate_version(value: &str) -> Result<()> {
    if value.trim() != value {
        bail!("版本不能包含首尾空格");
    }
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || part.parse::<u64>().is_err()
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        bail!("版本必须使用 x.y.z 格式");
    }
    Ok(())
}

pub(crate) fn version_is_newer(candidate: &str, base: &str) -> bool {
    version_tuple(candidate) > version_tuple(base)
}

pub(crate) fn version_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    version_tuple(left).cmp(&version_tuple(right))
}

fn version_tuple(value: &str) -> (u64, u64, u64) {
    let mut parts = value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn validate_capability(
    capability: &ErpCapabilityDefinition,
    module_keys: &BTreeSet<&str>,
) -> Result<()> {
    require_normalized_key(&capability.capability_key, "capability_key")?;
    required_text(&capability.display_name, "能力名称", 120)?;
    if !module_keys.contains(capability.module_key.as_str()) {
        bail!("能力 {} 引用了不存在的模块", capability.capability_key);
    }
    Ok(())
}

fn require_normalized_key(value: &str, field: &str) -> Result<()> {
    let normalized = normalize_key(value, field)?;
    if normalized != value {
        bail!("{field} 必须使用规范化小写标识：{normalized}");
    }
    Ok(())
}

fn normalize_aliases(values: Vec<String>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        let value = required_text(&value, "能力别名", 80)?;
        let dedupe_key = value.to_lowercase();
        if seen.insert(dedupe_key) {
            out.push(value);
        }
    }
    Ok(out)
}

fn validate_module_dependency_graph(modules: &[super::model::ErpModuleDefinition]) -> Result<()> {
    let dependencies: BTreeMap<_, _> = modules
        .iter()
        .map(|module| (module.module_key.as_str(), module.dependencies.as_slice()))
        .collect();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for module in dependencies.keys() {
        visit_module(module, &dependencies, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_module<'a>(
    module: &'a str,
    dependencies: &BTreeMap<&'a str, &'a [String]>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if visited.contains(module) {
        return Ok(());
    }
    if !visiting.insert(module) {
        bail!("模块依赖形成循环：{module}");
    }
    if let Some(children) = dependencies.get(module) {
        for child in *children {
            visit_module(child, dependencies, visiting, visited)?;
        }
    }
    visiting.remove(module);
    visited.insert(module);
    Ok(())
}

fn normalize_key_list(values: Vec<String>, label: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        let value = normalize_key(&value, label)?;
        if !seen.insert(value.clone()) {
            bail!("{label}标识重复：{value}");
        }
        out.push(value);
    }
    Ok(out)
}

fn unique_keys<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            bail!("{label}标识重复：{value}");
        }
    }
    Ok(())
}

fn required_text(value: &str, label: &str, max: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        bail!("{label}不能为空且不能超过 {max} 个字符");
    }
    Ok(value.to_string())
}

fn reject_sensitive_text(value: &str) -> Result<()> {
    let lower = value.to_lowercase();
    let forbidden = [
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "手机号",
        "身份证",
        "客户姓名",
        "订单明细",
        "数据库转储",
        "私有源码",
    ];
    if forbidden.iter().any(|needle| lower.contains(needle)) {
        bail!("需求信号包含敏感字段，请只提交脱敏后的业务缺口");
    }
    let longest_digits = lower
        .chars()
        .fold((0usize, 0usize), |(current, longest), ch| {
            let current = if ch.is_ascii_digit() { current + 1 } else { 0 };
            (current, longest.max(current))
        })
        .1;
    if longest_digits >= 8 {
        bail!("需求信号疑似包含手机号、订单号或其他长标识");
    }
    Ok(())
}

fn validate_evidence(evidence: &FeatureSignalEvidence) -> Result<()> {
    if evidence
        .occurrence_count
        .is_some_and(|value| !(1..=100_000).contains(&value))
    {
        bail!("occurrence_count 超出允许范围");
    }
    if evidence
        .estimated_time_saved_minutes
        .is_some_and(|value| !(0..=100_000).contains(&value))
    {
        bail!("estimated_time_saved_minutes 超出允许范围");
    }
    if evidence
        .affected_workflow
        .as_ref()
        .is_some_and(|value| value.chars().count() > 120)
    {
        bail!("affected_workflow 不能超过 120 个字符");
    }
    Ok(())
}
