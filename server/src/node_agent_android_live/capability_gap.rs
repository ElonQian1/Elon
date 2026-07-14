use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::broker::LiveUiSession;

const SCHEMA_VERSION: u32 = 1;
const MAX_UPGRADE_ROUNDS: u32 = 8;

const SUPPORTED_CAPABILITIES: &[&str] = &[
    "DESKTOP_TASK_IMPORT",
    "PROJECT_UI_PROFILE",
    "TARGET_DESIGN_BINDING",
    "ANNOTATION_MAPPING",
    "NEW_SCREEN_BOOTSTRAP",
    "REAL_ANDROID_RENDERER",
    "LIVE_STYLE_PATCH",
    "LOCAL_VISUAL_SOLVER",
    "PERSISTENT_FIT_RUN",
    "DETERMINISTIC_SOURCE_COMMIT",
    "CODEX_SOURCE_HANDOFF",
    "PATCH_FREE_BUILD_VERIFY",
    "WINDOW_INSETS_SEQUENCE_TRACE",
    "RELATIONAL_LAYOUT_GEOMETRY_TRACE",
];

const KNOWN_PLATFORM_GAPS: &[&str] = &[
    "PWA_CODE_GENERATION",
    "CROSS_PLATFORM_STYLE_WRITEBACK",
    "FREEFORM_COMPOSE_REPARENT",
    "ARBITRARY_COMPOSE_STYLE_REFLECTION",
    "VECTOR_ASSET_GENERATION",
    "PLATFORM_TOOL_DEFECT",
];

pub(crate) async fn check_capabilities(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<Value> {
    let requested = requested_capabilities(session, arguments)?;
    let view = session.view().await;
    let profile = super::design_bootstrap::project_profile(session).ok();
    let mut ready = Vec::new();
    let mut preparation = Vec::new();
    let mut preparation_details = Vec::new();
    let mut missing = Vec::new();
    let mut missing_details = Vec::new();
    for capability in requested {
        if !SUPPORTED_CAPABILITIES.contains(&capability.as_str()) {
            missing.push(capability);
            continue;
        }
        if capability == "NEW_SCREEN_BOOTSTRAP" {
            match new_screen_bootstrap_readiness(profile.as_ref()) {
                NewScreenBootstrapReadiness::Ready => ready.push(capability),
                NewScreenBootstrapReadiness::ProfileRequired => {
                    preparation.push(capability.clone());
                    preparation_details.push(json!({
                        "capability": capability,
                        "reason": "PROJECT_UI_PROFILE_REQUIRED",
                        "next": "ui_import_desktop_task"
                    }));
                }
                NewScreenBootstrapReadiness::ToolkitSelectionRequired => {
                    preparation.push(capability.clone());
                    preparation_details.push(json!({
                        "capability": capability,
                        "reason": "ANDROID_UI_TOOLKIT_SELECTION_REQUIRED",
                        "next": "ui_create_android_screen_scaffold",
                        "requiredArgument": "uiToolkit=COMPOSE|VIEWS"
                    }));
                }
                NewScreenBootstrapReadiness::NotAndroid => {
                    missing.push(capability.clone());
                    missing_details.push(json!({
                        "capability": capability,
                        "reason": "ANDROID_PROJECT_NOT_DETECTED"
                    }));
                }
            }
            continue;
        }
        if !view.connected && runtime_required(&capability) {
            preparation.push(capability.clone());
            preparation_details.push(json!({
                "capability": capability,
                "reason": "DEBUG_RUNTIME_NOT_CONNECTED",
                "next": "ui_prepare_debug_runtime"
            }));
        } else {
            ready.push(capability);
        }
    }
    let status = if !missing.is_empty() {
        "PLATFORM_GAP"
    } else if !preparation.is_empty() {
        "PREPARATION_REQUIRED"
    } else {
        "READY"
    };
    let next = if !missing.is_empty() {
        "ui_report_capability_gap"
    } else if preparation_details.iter().any(|detail| {
        detail.get("reason").and_then(Value::as_str) == Some("PROJECT_UI_PROFILE_REQUIRED")
    }) {
        "ui_import_desktop_task"
    } else if preparation_details.iter().any(|detail| {
        detail.get("reason").and_then(Value::as_str)
            == Some("ANDROID_UI_TOOLKIT_SELECTION_REQUIRED")
    }) {
        "ui_create_android_screen_scaffold"
    } else if !preparation.is_empty() {
        "ui_prepare_debug_runtime"
    } else {
        "CONTINUE_UI_WORKFLOW"
    };
    Ok(json!({
        "status": status,
        "ready": ready,
        "preparationRequired": preparation,
        "preparationDetails": preparation_details,
        "missing": missing,
        "missingDetails": missing_details,
        "supportedCapabilities": SUPPORTED_CAPABILITIES,
        "knownPlatformGaps": KNOWN_PLATFORM_GAPS,
        "automaticPlatformUpgrade": {
            "enabled": true,
            "trustedBoundary": "LOCAL_GIT_WORKSPACE",
            "automaticPublish": true,
            "maxRounds": MAX_UPGRADE_ROUNDS,
            "circuitBreakers": [
                "NO_SOURCE_CHANGE",
                "DUPLICATE_COMMIT",
                "REPEATED_FAILURE_SIGNATURE",
                "ROUND_BUDGET_EXHAUSTED"
            ]
        },
        "next": next
    }))
}

fn runtime_required(capability: &str) -> bool {
    matches!(
        capability,
        "REAL_ANDROID_RENDERER"
            | "LIVE_STYLE_PATCH"
            | "LOCAL_VISUAL_SOLVER"
            | "PERSISTENT_FIT_RUN"
            | "PATCH_FREE_BUILD_VERIFY"
            | "RELATIONAL_LAYOUT_GEOMETRY_TRACE"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewScreenBootstrapReadiness {
    Ready,
    ProfileRequired,
    ToolkitSelectionRequired,
    NotAndroid,
}

fn new_screen_bootstrap_readiness(profile: Option<&Value>) -> NewScreenBootstrapReadiness {
    let Some(profile) = profile else {
        return NewScreenBootstrapReadiness::ProfileRequired;
    };
    if !super::design_bootstrap::is_android_project_profile(profile) {
        return NewScreenBootstrapReadiness::NotAndroid;
    }
    let compose = profile.pointer("/capabilities/jetpackCompose") == Some(&Value::Bool(true));
    let views = profile.pointer("/capabilities/androidViews") == Some(&Value::Bool(true));
    if compose || views {
        NewScreenBootstrapReadiness::Ready
    } else {
        NewScreenBootstrapReadiness::ToolkitSelectionRequired
    }
}

pub(crate) fn report_gap(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let task_id = required_id(arguments, "taskId")?;
    let missing_capabilities =
        normalize_capabilities(&string_array(arguments, "missingCapabilities", 1, 16)?);
    if missing_capabilities.iter().all(|item| {
        SUPPORTED_CAPABILITIES.contains(&item.as_str()) && item != "PLATFORM_TOOL_DEFECT"
    }) {
        bail!("这些能力当前均已支持；普通业务结构修改应使用 CODEX_SOURCE_HANDOFF，不应升级平台");
    }
    let evidence = string_array(arguments, "evidence", 1, 32)?;
    let proposed_changes = string_array(arguments, "proposedChanges", 1, 16)?;
    let resume_target = required_text(arguments, "resumeTarget", 2_000)?;
    let now = Utc::now().to_rfc3339();
    let gap = CapabilityGapDocument {
        schema_version: SCHEMA_VERSION,
        gap_id: format!("gap_{}", uuid::Uuid::new_v4().simple()),
        task_id,
        fit_run_id: optional_id(arguments, "fitRunId")?,
        project_root: root.to_string_lossy().to_string(),
        status: CapabilityGapStatus::Approved,
        missing_capabilities,
        evidence,
        proposed_changes,
        resume_target,
        policy: CapabilityUpgradePolicy {
            trusted_boundary: "LOCAL_GIT_WORKSPACE".to_string(),
            automatic_source_upgrade: true,
            automatic_publish: true,
            max_upgrade_rounds: MAX_UPGRADE_ROUNDS,
        },
        upgrade_rounds: 0,
        attempts: Vec::new(),
        failure_signatures: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        last_error: None,
    };
    save_gap(&gap)?;
    Ok(json!({
        "gap": gap,
        "next": "ui_control_capability_gap START_UPGRADE",
        "instruction": "该 Git 工作区已授权自动平台升级与发布；完成发布后必须回报 commit/version 并重新检查原任务。"
    }))
}

pub(crate) fn get_gap(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    if let Some(gap_id) = arguments.get("gapId").and_then(Value::as_str) {
        return Ok(json!({ "gap": load_gap(&root, gap_id)? }));
    }
    Ok(json!({ "gaps": list_gaps(&root)? }))
}

pub(crate) fn control_gap(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let gap_id = required_id(arguments, "gapId")?;
    let action = required_text(arguments, "action", 80)?.to_ascii_uppercase();
    let mut gap = load_gap(&root, &gap_id)?;
    match action.as_str() {
        "START_UPGRADE" => start_upgrade(&mut gap, arguments)?,
        "PUBLISH_COMPLETED" => publish_completed(&mut gap, arguments)?,
        "RECHECK_PASSED" => {
            require_status(&gap, CapabilityGapStatus::Published)?;
            gap.status = CapabilityGapStatus::Resumed;
            gap.last_error = None;
        }
        "RECHECK_FAILED" | "UPGRADE_FAILED" => record_failure(&mut gap, arguments)?,
        "CANCEL" => {
            gap.status = CapabilityGapStatus::HumanRequired;
            gap.last_error = Some("开发者取消了自动平台升级".to_string());
        }
        _ => bail!("未知 capability gap action: {action}"),
    }
    gap.updated_at = Utc::now().to_rfc3339();
    save_gap(&gap)?;
    Ok(json!({
        "gap": gap,
        "resumeOriginalTask": gap.status == CapabilityGapStatus::Resumed,
        "next": next_action(&gap),
    }))
}

pub(crate) fn start_capability_upgrade(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<Value> {
    control_gap(session, &arguments_with_action(arguments, "START_UPGRADE")?)
}

pub(crate) fn complete_capability_upgrade(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<Value> {
    let transition = required_text(arguments, "transition", 80)?.to_ascii_uppercase();
    if !matches!(
        transition.as_str(),
        "PUBLISH_COMPLETED" | "RECHECK_PASSED" | "RECHECK_FAILED" | "UPGRADE_FAILED" | "CANCEL"
    ) {
        bail!("不支持的 capability upgrade completion transition: {transition}")
    }
    control_gap(session, &arguments_with_action(arguments, &transition)?)
}

fn arguments_with_action(arguments: &Value, action: &str) -> Result<Value> {
    let mut object = arguments
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("capability upgrade 参数必须是对象"))?;
    object.remove("transition");
    object.insert("action".to_string(), Value::String(action.to_string()));
    Ok(Value::Object(object))
}

pub(crate) fn list_gaps(root: &Path) -> Result<Vec<CapabilityGapDocument>> {
    let dir = gaps_root(root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut gaps = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|v| v.to_str()) == Some("json"))
        .filter_map(|entry| serde_json::from_slice(&fs::read(entry.path()).ok()?).ok())
        .collect::<Vec<_>>();
    gaps.sort_by(|left: &CapabilityGapDocument, right| right.updated_at.cmp(&left.updated_at));
    Ok(gaps)
}

fn requested_capabilities(session: &LiveUiSession, arguments: &Value) -> Result<Vec<String>> {
    if arguments.get("requiredCapabilities").is_some() {
        return string_array(arguments, "requiredCapabilities", 1, 32)
            .map(|items| normalize_capabilities(&items));
    }
    let task = super::design_bootstrap::design_task(session, arguments).ok();
    let mode = task
        .as_ref()
        .and_then(|value| value.pointer("/task/task/mode"))
        .and_then(Value::as_str)
        .unwrap_or("AUTO");
    let intent = task
        .as_ref()
        .and_then(|value| value.pointer("/task/task/attachmentIntent"))
        .and_then(Value::as_str)
        .unwrap_or("AUTO");
    let mut required = vec![
        "DESKTOP_TASK_IMPORT",
        "PROJECT_UI_PROFILE",
        "CODEX_SOURCE_HANDOFF",
        "PATCH_FREE_BUILD_VERIFY",
    ];
    if mode == "CREATE_NEW" {
        required.push("NEW_SCREEN_BOOTSTRAP");
    }
    if intent == "TARGET_DESIGN" {
        required.extend([
            "TARGET_DESIGN_BINDING",
            "REAL_ANDROID_RENDERER",
            "LOCAL_VISUAL_SOLVER",
            "PERSISTENT_FIT_RUN",
        ]);
    }
    Ok(required.into_iter().map(str::to_string).collect())
}

fn start_upgrade(gap: &mut CapabilityGapDocument, arguments: &Value) -> Result<()> {
    require_status(gap, CapabilityGapStatus::Approved)?;
    if gap.upgrade_rounds >= gap.policy.max_upgrade_rounds {
        gap.status = CapabilityGapStatus::HumanRequired;
        gap.last_error = Some("平台升级轮次已耗尽".to_string());
        return Ok(());
    }
    let source_revision_before = required_text(arguments, "sourceRevisionBefore", 256)?;
    gap.upgrade_rounds += 1;
    gap.attempts.push(CapabilityUpgradeAttempt {
        round: gap.upgrade_rounds,
        started_at: Utc::now().to_rfc3339(),
        source_revision_before,
        source_revision_after: None,
        commit_id: None,
        version: None,
        changed_files: Vec::new(),
    });
    gap.status = CapabilityGapStatus::Upgrading;
    gap.last_error = None;
    Ok(())
}

fn publish_completed(gap: &mut CapabilityGapDocument, arguments: &Value) -> Result<()> {
    require_status(gap, CapabilityGapStatus::Upgrading)?;
    let commit_id = required_text(arguments, "commitId", 256)?;
    let version = required_text(arguments, "version", 256)?;
    let source_revision_after = required_text(arguments, "sourceRevisionAfter", 256)?;
    let changed_files = string_array(arguments, "changedFiles", 1, 128)?;
    let duplicate_release = gap.attempts.iter().any(|item| {
        item.commit_id.as_deref() == Some(commit_id.as_str())
            || item.version.as_deref() == Some(version.as_str())
    });
    let source_revision_before = gap
        .attempts
        .last()
        .ok_or_else(|| anyhow!("缺少升级尝试"))?
        .source_revision_before
        .clone();
    if duplicate_release || source_revision_after == source_revision_before {
        gap.status = CapabilityGapStatus::HumanRequired;
        gap.last_error = Some(if duplicate_release {
            "同一提交或版本不能重复作为平台升级发布".to_string()
        } else {
            "源码 Revision 没有变化，拒绝空发布".to_string()
        });
        return Ok(());
    }
    let attempt = gap
        .attempts
        .last_mut()
        .ok_or_else(|| anyhow!("缺少升级尝试"))?;
    attempt.source_revision_after = Some(source_revision_after);
    attempt.commit_id = Some(commit_id);
    attempt.version = Some(version);
    attempt.changed_files = changed_files;
    gap.status = CapabilityGapStatus::Published;
    Ok(())
}

fn record_failure(gap: &mut CapabilityGapDocument, arguments: &Value) -> Result<()> {
    if !matches!(
        gap.status,
        CapabilityGapStatus::Upgrading | CapabilityGapStatus::Published
    ) {
        bail!("当前状态不能记录升级失败");
    }
    let signature = required_text(arguments, "failureSignature", 500)?;
    let error = required_text(arguments, "error", 2_000)?;
    let repeated = gap
        .failure_signatures
        .iter()
        .any(|value| value == &signature);
    if !repeated {
        gap.failure_signatures.push(signature);
    }
    gap.last_error = Some(error);
    gap.status = if repeated || gap.upgrade_rounds >= gap.policy.max_upgrade_rounds {
        CapabilityGapStatus::HumanRequired
    } else {
        CapabilityGapStatus::Approved
    };
    Ok(())
}

fn save_gap(gap: &CapabilityGapDocument) -> Result<()> {
    let root = PathBuf::from(&gap.project_root).canonicalize()?;
    let dir = gaps_root(&root);
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join(format!("{}.json", gap.gap_id)),
        serde_json::to_vec_pretty(gap)?,
    )?;
    Ok(())
}

fn load_gap(root: &Path, gap_id: &str) -> Result<CapabilityGapDocument> {
    let gap_id = validate_id(gap_id, "gapId")?;
    let path = gaps_root(root).join(format!("{gap_id}.json"));
    let gap: CapabilityGapDocument = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("平台能力缺口不存在: {gap_id}"))?,
    )?;
    if gap.schema_version != SCHEMA_VERSION {
        bail!("不支持的 capability gap schemaVersion");
    }
    Ok(gap)
}

fn gaps_root(root: &Path) -> PathBuf {
    root.join(".elon").join("ui-design").join("capability-gaps")
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let value = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("UI capability gap 未绑定项目目录"))?;
    PathBuf::from(value)
        .canonicalize()
        .context("项目目录不存在")
}

fn string_array(value: &Value, field: &str, min: usize, max: usize) -> Result<Vec<String>> {
    let items = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("缺少 {field}"))?;
    if items.len() < min || items.len() > max {
        bail!("{field} 数量必须在 {min}..{max}");
    }
    items
        .iter()
        .map(|item| {
            let text = item.as_str().map(str::trim).unwrap_or_default();
            if text.is_empty() || text.chars().count() > 2_000 {
                bail!("{field} 包含空值或超长值");
            }
            Ok(text.to_string())
        })
        .collect()
}

fn normalize_capabilities(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn required_text(value: &Value, field: &str, max: usize) -> Result<String> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow!("缺少 {field}"))?;
    if text.chars().count() > max {
        bail!("{field} 超过 {max} 字符");
    }
    Ok(text.to_string())
}

fn required_id(value: &Value, field: &str) -> Result<String> {
    validate_id(&required_text(value, field, 128)?, field)
}

fn optional_id(value: &Value, field: &str) -> Result<Option<String>> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(|value| validate_id(value, field))
        .transpose()
}

fn validate_id(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("{field} 非法");
    }
    Ok(value.to_string())
}

fn require_status(gap: &CapabilityGapDocument, expected: CapabilityGapStatus) -> Result<()> {
    if gap.status != expected {
        bail!("能力缺口状态不允许此操作: {:?}", gap.status);
    }
    Ok(())
}

fn next_action(gap: &CapabilityGapDocument) -> &'static str {
    match gap.status {
        CapabilityGapStatus::Approved => "START_UPGRADE",
        CapabilityGapStatus::Upgrading => "PUBLISH_COMPLETED",
        CapabilityGapStatus::Published => "RECHECK_ORIGINAL_TASK",
        CapabilityGapStatus::Resumed => "RESUME_ORIGINAL_UI_TASK",
        CapabilityGapStatus::HumanRequired => "HUMAN_DECISION_REQUIRED",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum CapabilityGapStatus {
    Approved,
    Upgrading,
    Published,
    Resumed,
    HumanRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityUpgradePolicy {
    trusted_boundary: String,
    automatic_source_upgrade: bool,
    automatic_publish: bool,
    max_upgrade_rounds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityUpgradeAttempt {
    round: u32,
    started_at: String,
    source_revision_before: String,
    source_revision_after: Option<String>,
    commit_id: Option<String>,
    version: Option<String>,
    changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityGapDocument {
    schema_version: u32,
    gap_id: String,
    task_id: String,
    fit_run_id: Option<String>,
    project_root: String,
    status: CapabilityGapStatus,
    missing_capabilities: Vec<String>,
    evidence: Vec<String>,
    proposed_changes: Vec<String>,
    resume_target: String,
    policy: CapabilityUpgradePolicy,
    upgrade_rounds: u32,
    attempts: Vec<CapabilityUpgradeAttempt>,
    failure_signatures: Vec<String>,
    created_at: String,
    updated_at: String,
    last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        arguments_with_action, new_screen_bootstrap_readiness, normalize_capabilities,
        publish_completed, CapabilityGapDocument, CapabilityGapStatus, CapabilityUpgradeAttempt,
        CapabilityUpgradePolicy, NewScreenBootstrapReadiness, SUPPORTED_CAPABILITIES,
    };
    use serde_json::json;

    #[test]
    fn capability_names_are_deduplicated_and_normalized() {
        assert_eq!(
            normalize_capabilities(&["live_style_patch".into(), "LIVE_STYLE_PATCH".into()]),
            vec!["LIVE_STYLE_PATCH"]
        );
        assert!(SUPPORTED_CAPABILITIES.contains(&"PERSISTENT_FIT_RUN"));
        assert!(SUPPORTED_CAPABILITIES.contains(&"WINDOW_INSETS_SEQUENCE_TRACE"));
        assert!(SUPPORTED_CAPABILITIES.contains(&"RELATIONAL_LAYOUT_GEOMETRY_TRACE"));
    }

    #[test]
    fn explicit_upgrade_api_maps_transition_to_state_machine_action() {
        let value = arguments_with_action(
            &json!({"gapId":"gap_1","transition":"PUBLISH_COMPLETED"}),
            "PUBLISH_COMPLETED",
        )
        .unwrap();
        assert_eq!(value["action"], "PUBLISH_COMPLETED");
        assert!(value.get("transition").is_none());
    }

    #[test]
    fn empty_platform_release_becomes_persistable_human_required_state() {
        let mut gap = test_gap();
        publish_completed(
            &mut gap,
            &json!({
                "commitId": "abc123",
                "version": "v1",
                "sourceRevisionAfter": "before",
                "changedFiles": ["server/src/example.rs"]
            }),
        )
        .unwrap();
        assert_eq!(gap.status, CapabilityGapStatus::HumanRequired);
        assert!(gap.last_error.unwrap().contains("空发布"));
    }

    #[test]
    fn blank_android_project_requests_toolkit_instead_of_platform_upgrade() {
        let profile = json!({
            "android": {"namespace":"com.example.blank"},
            "capabilities": {"jetpackCompose":false, "androidViews":false}
        });
        assert_eq!(
            new_screen_bootstrap_readiness(Some(&profile)),
            NewScreenBootstrapReadiness::ToolkitSelectionRequired
        );
        assert_eq!(
            new_screen_bootstrap_readiness(None),
            NewScreenBootstrapReadiness::ProfileRequired
        );
    }

    #[test]
    fn non_android_project_is_a_real_new_screen_capability_gap() {
        let profile = json!({
            "android": {"namespace":null, "applicationId":null},
            "capabilities": {"jetpackCompose":false, "androidViews":false}
        });
        assert_eq!(
            new_screen_bootstrap_readiness(Some(&profile)),
            NewScreenBootstrapReadiness::NotAndroid
        );
    }

    fn test_gap() -> CapabilityGapDocument {
        CapabilityGapDocument {
            schema_version: 1,
            gap_id: "gap_test".into(),
            task_id: "task_test".into(),
            fit_run_id: None,
            project_root: ".".into(),
            status: CapabilityGapStatus::Upgrading,
            missing_capabilities: vec!["PLATFORM_TOOL_DEFECT".into()],
            evidence: vec!["test".into()],
            proposed_changes: vec!["test".into()],
            resume_target: "resume".into(),
            policy: CapabilityUpgradePolicy {
                trusted_boundary: "LOCAL_GIT_WORKSPACE".into(),
                automatic_source_upgrade: true,
                automatic_publish: true,
                max_upgrade_rounds: 8,
            },
            upgrade_rounds: 1,
            attempts: vec![CapabilityUpgradeAttempt {
                round: 1,
                started_at: "now".into(),
                source_revision_before: "before".into(),
                source_revision_after: None,
                commit_id: None,
                version: None,
                changed_files: vec![],
            }],
            failure_signatures: vec![],
            created_at: "now".into(),
            updated_at: "now".into(),
            last_error: None,
        }
    }
}
