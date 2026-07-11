use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::broker::{LiveCommitSnapshot, LiveUiSession};
use super::protocol::{LivePatchTarget, LivePropertyValue, LiveStylePatch, LiveUiNode};
use super::source_json::{read_json, replace_json_pointer, resolve_json_binding, write_json};
use super::source_xml::{
    canonical_project_root, element_attribute, find_layout_element, find_value_resource, read_xml,
    reference_impact_count, replace_element_attribute, replace_value_resource, source_revision,
    write_xml, LayoutElement, ValueResource,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceCommitPlan {
    pub(crate) session_id: String,
    pub(crate) project_root: String,
    pub(crate) source_revision: String,
    pub(crate) deterministic_count: usize,
    pub(crate) codex_count: usize,
    pub(crate) entries: Vec<SourceCommitEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceCommitEntry {
    pub(crate) definition_id: String,
    pub(crate) resource_id: Option<String>,
    pub(crate) scope: String,
    pub(crate) property: String,
    pub(crate) value: LivePropertyValue,
    pub(crate) source_file: Option<String>,
    pub(crate) source_key: Option<String>,
    pub(crate) old_value: Option<String>,
    pub(crate) commit_mode: String,
    pub(crate) impact_count: usize,
    pub(crate) reason: String,
    #[serde(skip)]
    binding: Option<WriteBinding>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceCommitRequest {
    pub(crate) source_revision: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceCommitResult {
    pub(crate) status: &'static str,
    pub(crate) committed_count: usize,
    pub(crate) deferred_count: usize,
    pub(crate) changed_files: Vec<String>,
    pub(crate) source_revision_before: String,
    pub(crate) source_revision_after: String,
    pub(crate) deferred: Vec<SourceCommitEntry>,
}

#[derive(Debug, Clone)]
enum WriteBinding {
    Attribute {
        file: PathBuf,
        element: LayoutElement,
        attribute: String,
    },
    Resource {
        file: PathBuf,
        resource: ValueResource,
    },
    Json {
        file: PathBuf,
        pointer: String,
    },
}

pub(crate) async fn build_source_commit_plan(
    session: Arc<LiveUiSession>,
) -> Result<SourceCommitPlan> {
    let snapshot = session.commit_snapshot().await;
    build_plan(&session.id, snapshot)
}

pub(crate) async fn commit_source(
    session: Arc<LiveUiSession>,
    request: SourceCommitRequest,
) -> Result<SourceCommitResult> {
    let plan = build_source_commit_plan(session).await?;
    apply_source_commit_plan(plan, request)
}

pub(super) fn apply_source_commit_plan(
    plan: SourceCommitPlan,
    request: SourceCommitRequest,
) -> Result<SourceCommitResult> {
    if plan.source_revision != request.source_revision {
        bail!(
            "源码已在 Live 会话期间变化，拒绝覆盖（期望 {}，当前 {}）",
            request.source_revision,
            plan.source_revision
        );
    }
    let root = PathBuf::from(&plan.project_root)
        .canonicalize()
        .context("重新确认项目目录失败")?;
    let deterministic = plan
        .entries
        .iter()
        .filter(|entry| entry.commit_mode == "DETERMINISTIC")
        .collect::<Vec<_>>();
    if deterministic.is_empty() {
        bail!("当前 Live 修改没有可确定性写回的 XML/values 绑定");
    }

    let mut contents = BTreeMap::<PathBuf, String>::new();
    let mut json_paths = BTreeSet::<PathBuf>::new();
    for entry in &deterministic {
        let binding = entry
            .binding
            .as_ref()
            .ok_or_else(|| anyhow!("Commit Plan 缺少内部写入绑定"))?;
        let path = binding.path().to_path_buf();
        if !contents.contains_key(&path) {
            let content = match binding {
                WriteBinding::Json { .. } => read_json(&path)?,
                _ => read_xml(&path)?,
            };
            contents.insert(path.clone(), content);
        }
        let content = contents.get(&path).cloned().unwrap_or_default();
        let source_value = source_value(&entry.value)?;
        let updated = match binding {
            WriteBinding::Attribute {
                element, attribute, ..
            } => replace_element_attribute(&content, element, attribute, &source_value)?,
            WriteBinding::Resource { resource, .. } => {
                replace_value_resource(&content, resource, &source_value)?
            }
            WriteBinding::Json { pointer, .. } => {
                json_paths.insert(path.clone());
                replace_json_pointer(&content, pointer, &entry.value.value)?
            }
        };
        contents.insert(path, updated);
    }

    for (path, content) in &contents {
        if json_paths.contains(path) {
            write_json(&root, path, content)?;
        } else {
            write_xml(&root, path, content)?;
        }
    }
    let changed_files = contents
        .keys()
        .map(|path| relative_path(&root, path))
        .collect::<Vec<_>>();
    let source_revision_after = source_revision(&root, contents.keys().cloned())?;
    let committed_count = deterministic.len();
    drop(deterministic);
    let deferred = plan
        .entries
        .into_iter()
        .filter(|entry| entry.commit_mode != "DETERMINISTIC")
        .collect::<Vec<_>>();
    Ok(SourceCommitResult {
        status: "SOURCE_SAVED",
        committed_count,
        deferred_count: deferred.len(),
        changed_files,
        source_revision_before: request.source_revision,
        source_revision_after,
        deferred,
    })
}

pub(super) fn build_plan(
    session_id: &str,
    snapshot: LiveCommitSnapshot,
) -> Result<SourceCommitPlan> {
    let root = canonical_project_root(snapshot.project_root.as_deref())?;
    let operations = collapsed_operations(&snapshot.patches);
    let mut entries = Vec::new();
    for (target, property, value) in operations {
        entries.push(plan_entry(
            &root,
            &snapshot.nodes,
            &target,
            property,
            value,
        )?);
    }
    let paths = entries
        .iter()
        .filter_map(|entry| {
            entry
                .binding
                .as_ref()
                .map(|binding| binding.path().to_path_buf())
        })
        .collect::<BTreeSet<_>>();
    let revision = source_revision(&root, paths)?;
    let deterministic_count = entries
        .iter()
        .filter(|entry| entry.commit_mode == "DETERMINISTIC")
        .count();
    let codex_count = entries.len().saturating_sub(deterministic_count);
    Ok(SourceCommitPlan {
        session_id: session_id.to_string(),
        project_root: root.display().to_string(),
        source_revision: revision,
        deterministic_count,
        codex_count,
        entries,
    })
}

fn plan_entry(
    root: &Path,
    nodes: &[LiveUiNode],
    target: &LivePatchTarget,
    property: String,
    value: LivePropertyValue,
) -> Result<SourceCommitEntry> {
    let node = resolve_node(nodes, target);
    let definition_id = target
        .definition_id
        .clone()
        .or_else(|| node.map(|node| node.definition_id.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    let resource_id = node.and_then(|node| node.resource_id.clone());
    let scope = target.scope.trim().to_ascii_uppercase();
    let mut entry = deferred_entry(
        definition_id,
        resource_id.clone(),
        scope.clone(),
        property.clone(),
        value.clone(),
        "找不到可安全写回的 Android XML 绑定",
    );
    let Some(node) = node else {
        return Ok(entry);
    };
    if scope == "INSTANCE"
        && nodes
            .iter()
            .filter(|candidate| candidate.definition_id == node.definition_id)
            .count()
            > 1
    {
        entry.commit_mode = "SESSION_ONLY".to_string();
        entry.reason =
            "重复组件的单实例修改无法直接写回共享 XML，请交给 Codex 生成业务状态覆盖".to_string();
        return Ok(entry);
    }
    if let Some(property_snapshot) = node.properties.get(&property) {
        if let Some(binding) = property_snapshot.binding.as_ref() {
            let binding_kind = binding
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_ascii_uppercase();
            if matches!(binding_kind.as_str(), "STYLE_JSON" | "TOKEN") {
                if !property_snapshot
                    .commit_mode
                    .eq_ignore_ascii_case("DETERMINISTIC")
                {
                    entry.commit_mode = property_snapshot.commit_mode.to_ascii_uppercase();
                    entry.reason = format!(
                        "属性声明为 {}，不允许确定性写回 {}",
                        property_snapshot.commit_mode, binding_kind
                    );
                    return Ok(entry);
                }
                match resolve_json_binding(root, binding) {
                    Ok(Some(json_binding)) => {
                        entry.source_file = Some(json_binding.relative_file);
                        entry.source_key = Some(json_binding.source_key);
                        entry.old_value = Some(display_json_value(&json_binding.old_value));
                        entry.commit_mode = "DETERMINISTIC".to_string();
                        entry.impact_count = 1;
                        entry.reason = if json_binding.kind == "TOKEN" {
                            "精确写回绑定的设计 Token JSON".to_string()
                        } else {
                            "精确写回受控 Compose Style JSON".to_string()
                        };
                        entry.binding = Some(WriteBinding::Json {
                            file: json_binding.file,
                            pointer: json_binding.pointer,
                        });
                        return Ok(entry);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        entry.reason = format!("样式 JSON 绑定无效: {error}");
                        return Ok(entry);
                    }
                }
            }
            if binding_kind == "KOTLIN_SYMBOL" {
                entry.reason = "属性绑定 Kotlin Symbol，需要 Codex/PSI 修改源码".to_string();
                return Ok(entry);
            }
            if matches!(binding_kind.as_str(), "SESSION_ONLY" | "COMPUTED") {
                entry.commit_mode = "SESSION_ONLY".to_string();
                entry.reason = "属性只允许当前设计会话预览，不能直接写回源码".to_string();
                return Ok(entry);
            }
        }
    }
    let Some(resource_id) = resource_id else {
        entry.commit_mode = "SESSION_ONLY".to_string();
        entry.reason = "节点只有运行时路径 ID；请先添加稳定 resource-id/uiNode 绑定".to_string();
        return Ok(entry);
    };
    let Some(element) = find_layout_element(root, &resource_id)? else {
        entry.reason = "resource-id 只在代码中创建或引用，需由 Codex 修改 Kotlin/Java".to_string();
        return Ok(entry);
    };
    let Some(attribute) = property_attribute(&property, node, &element.tag_name) else {
        entry.reason = "该属性依赖 drawable、布局结构或运行时代码，需由 Codex 修改".to_string();
        return Ok(entry);
    };
    let content = read_xml(&element.file)?;
    let old_value = element_attribute(&content, &element, attribute);
    if let Some(reference) = old_value.as_deref().filter(|value| value.starts_with('@')) {
        if let Some(resource) = find_value_resource(root, reference)? {
            if value_matches_resource(&value, &resource.resource_type) {
                let resource_content = read_xml(&resource.file)?;
                let resource_old = resource_content
                    .get(resource.value_start..resource.value_end)
                    .map(str::trim)
                    .map(str::to_string);
                entry.source_file = Some(resource.relative_file.clone());
                entry.source_key = Some(format!(
                    "{}:{}",
                    resource.resource_type, resource.resource_name
                ));
                entry.old_value = resource_old;
                entry.commit_mode = "DETERMINISTIC".to_string();
                entry.impact_count = reference_impact_count(root, reference);
                entry.reason = if entry.impact_count > 1 {
                    format!(
                        "写回共享资源 {reference}，将影响 {} 处引用",
                        entry.impact_count
                    )
                } else {
                    format!("保持现有资源绑定 {reference} 并更新其值")
                };
                entry.binding = Some(WriteBinding::Resource {
                    file: resource.file.clone(),
                    resource,
                });
                return Ok(entry);
            }
        }
        entry.reason = format!("现有属性引用 {reference}，无法安全确定性改写");
        return Ok(entry);
    }
    if old_value
        .as_deref()
        .is_some_and(|value| value.starts_with('?'))
    {
        entry.reason = "属性来自 Theme/Style，需要 Codex 追踪主题继承后修改".to_string();
        return Ok(entry);
    }
    entry.source_file = Some(element.relative_file.clone());
    entry.source_key = Some(format!("{}#{attribute}", element.resource_name));
    entry.old_value = old_value;
    entry.commit_mode = "DETERMINISTIC".to_string();
    entry.impact_count = 1;
    entry.reason = "精确写回 resource-id 对应 XML 属性".to_string();
    entry.binding = Some(WriteBinding::Attribute {
        file: element.file.clone(),
        element,
        attribute: attribute.to_string(),
    });
    Ok(entry)
}

fn collapsed_operations(
    patches: &[LiveStylePatch],
) -> Vec<(LivePatchTarget, String, LivePropertyValue)> {
    let mut values = BTreeMap::<String, (LivePatchTarget, String, LivePropertyValue)>::new();
    for patch in patches {
        for operation in &patch.operations {
            let key = format!(
                "{}|{}|{}|{}",
                patch.target.scope,
                patch.target.runtime_node_id.as_deref().unwrap_or_default(),
                patch.target.definition_id.as_deref().unwrap_or_default(),
                operation.property
            );
            values.insert(
                key,
                (
                    patch.target.clone(),
                    operation.property.clone(),
                    operation.value.clone(),
                ),
            );
        }
    }
    values.into_values().collect()
}

fn resolve_node<'a>(nodes: &'a [LiveUiNode], target: &LivePatchTarget) -> Option<&'a LiveUiNode> {
    target
        .runtime_node_id
        .as_deref()
        .and_then(|runtime_id| nodes.iter().find(|node| node.runtime_node_id == runtime_id))
        .or_else(|| {
            target.definition_id.as_deref().and_then(|definition_id| {
                nodes
                    .iter()
                    .find(|node| node.definition_id == definition_id)
            })
        })
}

fn property_attribute(property: &str, node: &LiveUiNode, tag_name: &str) -> Option<&'static str> {
    let material = tag_name.contains("Material") || node.class_name.contains("Material");
    match property {
        "width" => Some("android:layout_width"),
        "height" => Some("android:layout_height"),
        "minWidth" => Some("android:minWidth"),
        "minHeight" => Some("android:minHeight"),
        "margin.start" => Some("android:layout_marginStart"),
        "margin.top" => Some("android:layout_marginTop"),
        "margin.end" => Some("android:layout_marginEnd"),
        "margin.bottom" => Some("android:layout_marginBottom"),
        "padding.start" => Some("android:paddingStart"),
        "padding.top" => Some("android:paddingTop"),
        "padding.end" => Some("android:paddingEnd"),
        "padding.bottom" => Some("android:paddingBottom"),
        "backgroundColor" if node.kind == "material.card" => Some("app:cardBackgroundColor"),
        "backgroundColor" => Some("android:background"),
        "contentColor" => Some("android:textColor"),
        "borderColor" if material => Some("app:strokeColor"),
        "borderWidth" if material => Some("app:strokeWidth"),
        "cornerRadius.all" if node.kind == "material.card" => Some("app:cardCornerRadius"),
        "cornerRadius.all" if material => Some("app:cornerRadius"),
        "text" => Some("android:text"),
        "textSize" => Some("android:textSize"),
        "opacity" => Some("android:alpha"),
        "visibility" => Some("android:visibility"),
        "translationX" => Some("android:translationX"),
        "translationY" => Some("android:translationY"),
        "scaleX" => Some("android:scaleX"),
        "scaleY" => Some("android:scaleY"),
        _ => None,
    }
}

fn value_matches_resource(value: &LivePropertyValue, resource_type: &str) -> bool {
    matches!(
        (
            value.value_type.to_ascii_lowercase().as_str(),
            resource_type
        ),
        ("argb" | "color", "color") | ("dp" | "sp" | "dimension", "dimen") | ("text", "string")
    )
}

fn source_value(value: &LivePropertyValue) -> Result<String> {
    let value_type = value.value_type.to_ascii_lowercase();
    match value_type.as_str() {
        "dp" | "sp" => {
            let number = value
                .value
                .as_f64()
                .ok_or_else(|| anyhow!("样式值不是数值"))?;
            Ok(format!("{}{}", trim_number(number), value_type))
        }
        "float" => Ok(trim_number(
            value
                .value
                .as_f64()
                .ok_or_else(|| anyhow!("样式值不是数值"))?,
        )),
        "argb" | "color" | "text" | "enum" | "dimension" => value
            .value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("样式值不是字符串")),
        "bool" => value
            .value
            .as_bool()
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow!("样式值不是布尔值")),
        _ => bail!("不支持写回值类型: {}", value.value_type),
    }
}

fn trim_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn display_json_value(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn deferred_entry(
    definition_id: String,
    resource_id: Option<String>,
    scope: String,
    property: String,
    value: LivePropertyValue,
    reason: &str,
) -> SourceCommitEntry {
    SourceCommitEntry {
        definition_id,
        resource_id,
        scope,
        property,
        value,
        source_file: None,
        source_key: None,
        old_value: None,
        commit_mode: "CODEX".to_string(),
        impact_count: 0,
        reason: reason.to_string(),
        binding: None,
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

impl WriteBinding {
    fn path(&self) -> &Path {
        match self {
            Self::Attribute { file, .. }
            | Self::Resource { file, .. }
            | Self::Json { file, .. } => file,
        }
    }
}
