use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::parser::canonical_project_root;
use super::writeback_receipt_workspace::{
    hashes_for_paths, operation_changed_files, snapshot_workspace, WorkspaceSnapshot,
};

const MAX_ACTIVE_RECEIPTS: usize = 64;
const MAX_CHANGED_FILES: usize = 256;
const MAX_OPERATION_ID: usize = 160;
const MAX_ERROR_LENGTH: usize = 2_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BeginWritebackReceiptRequest {
    pub(crate) operation_id: String,
    pub(crate) project_root: String,
    pub(crate) draft_revision: u64,
    pub(crate) target_platforms: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlatformReceiptUpdate {
    pub(crate) status: String,
    pub(crate) method: String,
    #[serde(default)]
    pub(crate) changed_files: Vec<String>,
    #[serde(default)]
    pub(crate) source_revisions: BTreeMap<String, String>,
    pub(crate) expected_source_revision_before: Option<String>,
    pub(crate) build_evidence: Option<Value>,
    pub(crate) ai_task_id: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompleteWritebackReceiptRequest {
    pub(crate) receipt_id: String,
    pub(crate) project_root: String,
    pub(crate) platform_results: BTreeMap<String, PlatformReceiptUpdate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlatformWritebackResult {
    pub(crate) platform: String,
    pub(crate) status: String,
    pub(crate) method: String,
    pub(crate) changed_files: Vec<String>,
    pub(crate) source_revisions: BTreeMap<String, String>,
    pub(crate) source_hashes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build_evidence: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ai_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    pub(crate) evidence_complete: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WritebackReceipt {
    pub(crate) schema_version: u8,
    pub(crate) receipt_id: String,
    pub(crate) operation_id: String,
    pub(crate) project_root: String,
    pub(crate) draft_revision: u64,
    pub(crate) target_platforms: Vec<String>,
    pub(crate) source_revision_before: String,
    pub(crate) source_revision: String,
    pub(crate) source_hash: String,
    pub(crate) changed_files: Vec<String>,
    pub(crate) source_hashes: BTreeMap<String, String>,
    pub(crate) platform_results: BTreeMap<String, PlatformWritebackResult>,
    pub(crate) status: String,
    pub(crate) complete: bool,
    pub(crate) evidence_complete: bool,
    pub(crate) diagnostics: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReceiptState {
    baseline: WorkspaceSnapshot,
    receipt: WritebackReceipt,
}

static ACTIVE_RECEIPTS: OnceLock<Mutex<HashMap<String, ReceiptState>>> = OnceLock::new();

pub(crate) fn begin_writeback_receipt(
    request: BeginWritebackReceiptRequest,
) -> Result<WritebackReceipt> {
    let operation_id = safe_identifier(&request.operation_id, "operationId")?;
    let target_platforms = normalize_platforms(&request.target_platforms)?;
    let baseline = snapshot_workspace(&request.project_root)?;
    let now = Utc::now().to_rfc3339();
    let receipt_id = format!("uiwr_{}", uuid::Uuid::new_v4().simple());
    let platform_results = target_platforms
        .iter()
        .map(|platform| {
            (
                platform.clone(),
                PlatformWritebackResult {
                    platform: platform.clone(),
                    status: "PREVIEW".into(),
                    method: "PENDING".into(),
                    changed_files: Vec::new(),
                    source_revisions: BTreeMap::new(),
                    source_hashes: BTreeMap::new(),
                    build_evidence: None,
                    ai_task_id: None,
                    error: None,
                    evidence_complete: false,
                },
            )
        })
        .collect();
    let receipt = WritebackReceipt {
        schema_version: 1,
        receipt_id: receipt_id.clone(),
        operation_id,
        project_root: slash_path(&baseline.root),
        draft_revision: request.draft_revision,
        target_platforms,
        source_revision_before: baseline.source_revision.clone(),
        source_revision: baseline.source_revision.clone(),
        source_hash: aggregate_source_hash(&baseline.source_revision, &BTreeMap::new()),
        changed_files: Vec::new(),
        source_hashes: BTreeMap::new(),
        platform_results,
        status: "PREVIEW".into(),
        complete: false,
        evidence_complete: false,
        diagnostics: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };
    let mut active = active_receipts()?;
    if active.len() >= MAX_ACTIVE_RECEIPTS {
        let oldest = active
            .iter()
            .min_by_key(|(_, state)| state.receipt.updated_at.clone())
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            active.remove(&oldest);
        }
    }
    let state = ReceiptState {
        baseline,
        receipt: receipt.clone(),
    };
    persist_receipt_state(&state)?;
    active.insert(receipt_id, state);
    Ok(receipt)
}

pub(crate) fn complete_writeback_receipt(
    request: CompleteWritebackReceiptRequest,
) -> Result<WritebackReceipt> {
    let receipt_id = safe_identifier(&request.receipt_id, "receiptId")?;
    let requested_root = canonical_project_root(&request.project_root)?;
    let mut active = active_receipts()?;
    if !active.contains_key(&receipt_id) {
        active.insert(
            receipt_id.clone(),
            read_receipt_state(&requested_root, &receipt_id)?,
        );
    }
    let state = active.get_mut(&receipt_id).ok_or_else(|| {
        anyhow!("WRITEBACK_RECEIPT_NOT_FOUND：回执不存在或节点已重启，请重新开始写回")
    })?;
    if requested_root != state.baseline.root {
        bail!("WRITEBACK_RECEIPT_ROOT_MISMATCH：回执与当前项目目录不一致");
    }
    let current = snapshot_workspace(&request.project_root)?;
    let operation_files = operation_changed_files(&state.baseline, &current)?;
    let operation_hashes = hashes_for_paths(&current.root, &operation_files)?;
    let mut diagnostics = Vec::new();
    for (platform, update) in request.platform_results {
        if !state.receipt.target_platforms.contains(&platform) {
            diagnostics.push(format!("忽略非目标平台 {platform} 的回执更新"));
            continue;
        }
        let previous = state
            .receipt
            .platform_results
            .get(&platform)
            .cloned()
            .ok_or_else(|| anyhow!("回执缺少目标平台 {platform}"))?;
        let validated = validate_platform_update(
            &platform,
            update,
            &previous,
            &state.receipt.source_revision,
            &current.source_revision,
            &operation_files,
            &operation_hashes,
            &mut diagnostics,
        )?;
        state.receipt.platform_results.insert(platform, validated);
    }
    state.receipt.source_revision = current.source_revision.clone();
    state.receipt.changed_files = operation_files.into_iter().collect();
    state.receipt.source_hashes = operation_hashes;
    state.receipt.source_hash =
        aggregate_source_hash(&current.source_revision, &state.receipt.source_hashes);
    state.receipt.updated_at = Utc::now().to_rfc3339();
    state.receipt.diagnostics = diagnostics;
    settle_receipt(&mut state.receipt);
    persist_receipt_state(state)?;
    Ok(state.receipt.clone())
}

pub(crate) fn get_writeback_receipt(
    project_root: &str,
    receipt_id: &str,
) -> Result<WritebackReceipt> {
    let receipt_id = safe_identifier(receipt_id, "receiptId")?;
    let root = canonical_project_root(project_root)?;
    Ok(read_receipt_state(&root, &receipt_id)?.receipt)
}

fn validate_platform_update(
    platform: &str,
    update: PlatformReceiptUpdate,
    previous: &PlatformWritebackResult,
    recorded_revision: &str,
    current_revision: &str,
    operation_files: &BTreeSet<String>,
    operation_hashes: &BTreeMap<String, String>,
    diagnostics: &mut Vec<String>,
) -> Result<PlatformWritebackResult> {
    let status = normalize_status(&update.status)?;
    let method = normalize_method(&update.method)?;
    let claimed_files = normalize_relative_files(update.changed_files)?;
    let invalid_files = claimed_files
        .iter()
        .filter(|path| !operation_files.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    if !invalid_files.is_empty() {
        bail!(
            "{platform} 回执声明了不属于本次操作的文件: {}",
            invalid_files.join(", ")
        );
    }
    let mut error = clean_optional(update.error, MAX_ERROR_LENGTH);
    let expected_before = update
        .expected_source_revision_before
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(expected) = expected_before {
        if expected != recorded_revision {
            bail!("{platform} 回执的 expectedSourceRevisionBefore 已过期");
        }
        if matches!(
            status.as_str(),
            "SAVED" | "BUILD_VERIFYING" | "BUILD_VERIFIED"
        ) && current_revision == expected
        {
            error = Some("源码 revision 未变化，不能证明本轮 AI/确定性写回已保存".into());
        }
    }
    let mut effective_status = status;
    if matches!(
        effective_status.as_str(),
        "SAVED" | "BUILD_VERIFYING" | "BUILD_VERIFIED"
    ) && claimed_files.is_empty()
    {
        error = Some("缺少 changedFiles，不能显示已保存或已验证".into());
    }
    let build_verified = effective_status == "BUILD_VERIFIED"
        && valid_build_evidence(platform, update.build_evidence.as_ref(), current_revision);
    if effective_status == "BUILD_VERIFIED" && !build_verified {
        error = Some("缺少与当前 sourceRevision 对应的 build/verify evidence".into());
    }
    if error.is_some() && effective_status != "FAILED" {
        effective_status = "EVIDENCE_MISSING".into();
    }
    let source_hashes = claimed_files
        .iter()
        .filter_map(|file| {
            operation_hashes
                .get(file)
                .map(|hash| (file.clone(), hash.clone()))
        })
        .collect();
    let result = PlatformWritebackResult {
        platform: platform.into(),
        status: effective_status,
        method,
        changed_files: claimed_files,
        source_revisions: update.source_revisions,
        source_hashes,
        build_evidence: update.build_evidence,
        ai_task_id: clean_optional(update.ai_task_id, 160),
        error,
        evidence_complete: build_verified,
    };
    if result.status == "EVIDENCE_MISSING" {
        diagnostics.push(format!(
            "{platform} 缺少完成证据：{}",
            result.error.as_deref().unwrap_or("未知")
        ));
    }
    Ok(merge_platform_result(previous, result))
}

fn merge_platform_result(
    previous: &PlatformWritebackResult,
    mut next: PlatformWritebackResult,
) -> PlatformWritebackResult {
    if next.changed_files.is_empty() && next.status == "PREVIEW" {
        next.changed_files = previous.changed_files.clone();
        next.source_hashes = previous.source_hashes.clone();
        next.source_revisions = previous.source_revisions.clone();
    }
    next
}

fn settle_receipt(receipt: &mut WritebackReceipt) {
    let targets = receipt
        .target_platforms
        .iter()
        .filter_map(|platform| receipt.platform_results.get(platform))
        .collect::<Vec<_>>();
    receipt.complete = !targets.is_empty()
        && targets
            .iter()
            .all(|result| result.status == "BUILD_VERIFIED" && result.evidence_complete);
    receipt.evidence_complete = receipt.complete;
    receipt.status = if receipt.complete {
        "COMPLETE"
    } else if targets.iter().any(|result| result.status == "FAILED") {
        if targets.iter().any(|result| {
            matches!(
                result.status.as_str(),
                "SAVED" | "BUILD_VERIFYING" | "BUILD_VERIFIED"
            )
        }) {
            "PARTIAL"
        } else {
            "FAILED"
        }
    } else if targets
        .iter()
        .any(|result| result.status == "EVIDENCE_MISSING")
    {
        "EVIDENCE_MISSING"
    } else if targets.iter().any(|result| result.status != "PREVIEW") {
        "IN_PROGRESS"
    } else {
        "PREVIEW"
    }
    .into();
}

fn valid_build_evidence(platform: &str, evidence: Option<&Value>, current_revision: &str) -> bool {
    let Some(evidence) = evidence else {
        return false;
    };
    if evidence.get("status").and_then(Value::as_str) != Some("BUILD_VERIFIED") {
        return false;
    }
    let evidence_revision = evidence
        .get("sourceRevision")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !evidence_revision.is_empty() && evidence_revision != current_revision {
        return false;
    }
    match platform {
        "web" => {
            evidence.get("browserCaptured").and_then(Value::as_bool) == Some(true)
                && evidence
                    .get("routeRevision")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        }
        "pwa" => {
            evidence.get("runtimeReloaded").and_then(Value::as_bool) == Some(true)
                && evidence
                    .get("routeRevision")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        }
        "tauri" => {
            evidence.get("frontendCaptured").and_then(Value::as_bool) == Some(true)
                && evidence.get("nativeHostVerified").and_then(Value::as_bool) == Some(true)
                && evidence
                    .get("nativeArtifactSha256")
                    .and_then(Value::as_str)
                    .is_some_and(|value| {
                        value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
                    })
        }
        "apk" | "android" => {
            evidence.get("runtimeConnected").and_then(Value::as_bool) == Some(true)
                && evidence
                    .get("apkPath")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        }
        _ => false,
    }
}

fn normalize_platforms(values: &[String]) -> Result<Vec<String>> {
    let mut platforms = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    if platforms.is_empty() || platforms.len() > 4 {
        bail!("targetPlatforms 必须包含 web/pwa/tauri/android 中的一到四项");
    }
    if platforms
        .iter()
        .any(|value| !matches!(value.as_str(), "web" | "pwa" | "tauri" | "android" | "apk"))
    {
        bail!("targetPlatforms 只支持 web、pwa、tauri、android；apk 仅作旧客户端兼容");
    }
    Ok(platforms.iter().map(|value| value.to_string()).collect())
}

fn normalize_status(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase().replace('-', "_");
    if !matches!(
        value.as_str(),
        "PREVIEW"
            | "AI_WRITING"
            | "SAVED"
            | "BUILD_VERIFYING"
            | "BUILD_VERIFIED"
            | "FAILED"
            | "EVIDENCE_MISSING"
    ) {
        bail!("不支持的写回状态 {value}");
    }
    Ok(value)
}

fn normalize_method(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase().replace('-', "_");
    if !matches!(
        value.as_str(),
        "PENDING" | "DETERMINISTIC" | "CODEX" | "MIXED"
    ) {
        bail!("不支持的写回方式 {value}");
    }
    Ok(value)
}

fn normalize_relative_files(values: Vec<String>) -> Result<Vec<String>> {
    if values.len() > MAX_CHANGED_FILES {
        bail!("changedFiles 超过 {MAX_CHANGED_FILES} 项");
    }
    let mut normalized = values
        .iter()
        .map(|value| normalize_relative_path(value))
        .collect::<Result<BTreeSet<_>>>()?;
    Ok(normalized.iter().map(|value| value.to_string()).collect())
}

fn normalize_relative_path(value: &str) -> Result<String> {
    let normalized = value.trim().replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || normalized.len() > 1_000
        || path.is_absolute()
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
    {
        bail!("回执包含不安全的相对路径");
    }
    Ok(normalized)
}

fn safe_identifier(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_OPERATION_ID
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_:.-".contains(character))
    {
        bail!("{field} 为空、超长或包含不安全字符");
    }
    Ok(value.to_string())
}

fn clean_optional(value: Option<String>, max: usize) -> Option<String> {
    value
        .map(|value| value.trim().chars().take(max).collect::<String>())
        .filter(|value| !value.is_empty())
}

fn aggregate_source_hash(
    source_revision: &str,
    source_hashes: &BTreeMap<String, String>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-ui-writeback-receipt-v1\0");
    hasher.update(source_revision.as_bytes());
    for (path, hash) in source_hashes {
        hasher.update([0]);
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(hash.as_bytes());
    }
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn active_receipts() -> Result<std::sync::MutexGuard<'static, HashMap<String, ReceiptState>>> {
    ACTIVE_RECEIPTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| anyhow!("写回回执状态锁已损坏"))
}

fn persist_receipt_state(state: &ReceiptState) -> Result<()> {
    let root = Path::new(&state.receipt.project_root).canonicalize()?;
    let path = receipt_state_path(&root, &state.receipt.receipt_id, true)?;
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn read_receipt_state(root: &Path, receipt_id: &str) -> Result<ReceiptState> {
    let path = receipt_state_path(root, receipt_id, false)?;
    let metadata = fs::metadata(&path).context("WRITEBACK_RECEIPT_NOT_FOUND：回执不存在")?;
    if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
        bail!("WRITEBACK_RECEIPT_INVALID：回执文件无效或过大");
    }
    let state: ReceiptState = serde_json::from_slice(&fs::read(path)?)
        .context("WRITEBACK_RECEIPT_INVALID：回执 JSON 无效")?;
    if state.baseline.root != root || state.receipt.receipt_id != receipt_id {
        bail!("WRITEBACK_RECEIPT_ROOT_MISMATCH：回执与当前项目目录不一致");
    }
    Ok(state)
}

fn receipt_state_path(root: &Path, receipt_id: &str, create: bool) -> Result<std::path::PathBuf> {
    safe_identifier(receipt_id, "receiptId")?;
    let directory = root.join(".elon/ui-tuner/writeback-receipts");
    if create {
        fs::create_dir_all(&directory)?;
    }
    let canonical = directory.canonicalize().context("写回回执目录不存在")?;
    if !canonical.starts_with(root) {
        bail!("写回回执目录越出项目");
    }
    Ok(canonical.join(format!("{receipt_id}.json")))
}
