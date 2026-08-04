use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_RECORD_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourcePatchProposal {
    pub(super) schema_version: u32,
    pub(super) proposal_id: String,
    pub(super) revision: u64,
    pub(super) writeback_plan_id: String,
    pub(super) draft_id: String,
    pub(super) draft_revision: u64,
    pub(super) source_file: String,
    pub(super) source_sha_before: String,
    pub(super) source_sha_after: String,
    pub(super) edits: Vec<SourcePatchEdit>,
    pub(super) status: String,
    pub(super) decision_reason: Option<String>,
    pub(super) review_artifact_path: String,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) applied_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourcePatchEdit {
    pub(super) start: u64,
    pub(super) end: u64,
    pub(super) before_sha256: String,
    pub(super) replacement_sha256: String,
    pub(super) before: String,
    pub(super) replacement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRollbackPlan {
    schema_version: u32,
    rollback_id: String,
    revision: u64,
    proposal_id: String,
    proposal_revision: u64,
    source_file: String,
    expected_source_revision: String,
    target_source_revision: String,
    edits: Vec<SourcePatchEdit>,
    status: String,
    review_artifact_path: String,
    created_at: String,
}

pub(super) fn build_edits(source: &[u8], values: &Value) -> Result<Vec<SourcePatchEdit>> {
    let values = values.as_array().context("edits 必须是数组")?;
    if values.is_empty() || values.len() > 16 {
        bail!("edits 必须包含 1..16 项");
    }
    let mut edits = Vec::with_capacity(values.len());
    for value in values {
        let start = value
            .get("start")
            .and_then(Value::as_u64)
            .context("edit 缺少 start")?;
        let end = value
            .get("end")
            .and_then(Value::as_u64)
            .context("edit 缺少 end")?;
        if start > end || end > source.len() as u64 || end - start > 64 * 1024 {
            bail!("edit byte range 无效或超过 64 KiB");
        }
        let before_bytes = &source[start as usize..end as usize];
        let before = std::str::from_utf8(before_bytes).context("edit 范围不是 UTF-8 源码")?;
        let expected = required_text(value, "expectedBeforeSha256")?;
        validate_sha(expected)?;
        let actual = source_sha(before_bytes);
        if actual != expected {
            bail!("SOURCE_PATCH_RANGE_DRIFT：edit 范围 SHA-256 不匹配");
        }
        let replacement = required_text_allow_empty(value, "replacement")?;
        if replacement.len() > 64 * 1024 || replacement.contains('\0') {
            bail!("replacement 超过 64 KiB 或包含 NUL");
        }
        edits.push(SourcePatchEdit {
            start,
            end,
            before_sha256: actual,
            replacement_sha256: source_sha(replacement.as_bytes()),
            before: before.to_string(),
            replacement: replacement.to_string(),
        });
    }
    edits.sort_by_key(|edit| edit.start);
    if edits.windows(2).any(|pair| pair[0].end > pair[1].start) {
        bail!("edits byte range 不能重叠");
    }
    Ok(edits)
}

pub(super) fn apply_edits(source: &[u8], edits: &[SourcePatchEdit]) -> Result<Vec<u8>> {
    let mut output = source.to_vec();
    for edit in edits.iter().rev() {
        let start = usize::try_from(edit.start)?;
        let end = usize::try_from(edit.end)?;
        if end > output.len() || source_sha(&output[start..end]) != edit.before_sha256 {
            bail!("SOURCE_PATCH_RANGE_DRIFT：应用前 edit 范围已变化");
        }
        output.splice(start..end, edit.replacement.as_bytes().iter().copied());
    }
    Ok(output)
}

pub(super) fn source_path(root: &Path, value: &str) -> Result<PathBuf> {
    let relative = Path::new(value);
    if value.is_empty()
        || value.len() > 1_000
        || relative.is_absolute()
        || relative.components().any(|part| part.as_os_str() == "..")
    {
        bail!("sourceFile 不是安全相对路径");
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) || !path.is_file() {
        bail!("sourceFile 越出项目或不是普通文件");
    }
    if fs::metadata(&path)?.len() > MAX_SOURCE_BYTES {
        bail!("sourceFile 超过 2 MiB 限制");
    }
    Ok(path)
}

pub(super) fn source_sha(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

pub(super) fn proposal_id(
    writeback_plan_id: &str,
    draft_revision: u64,
    source_sha_before: &str,
    edits: &[SourcePatchEdit],
) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(writeback_plan_id.as_bytes());
    hasher.update(draft_revision.to_le_bytes());
    hasher.update(source_sha_before.as_bytes());
    hasher.update(serde_json::to_vec(edits)?);
    let digest = hasher.finalize();
    Ok(format!("sourcepatch_{}", &hex::encode(digest)[..32]))
}

pub(super) fn persist(root: &Path, proposal: &SourcePatchProposal) -> Result<()> {
    let directory = proposal_directory(root, true)?;
    crate::node_agent_atomic_file::write(
        &directory.join(format!("{}.json", proposal.proposal_id)),
        &serde_json::to_vec_pretty(proposal)?,
    )
}

pub(super) fn read(root: &Path, proposal_id: &str) -> Result<SourcePatchProposal> {
    validate_id(proposal_id, "sourcepatch_")?;
    let path = proposal_directory(root, false)?.join(format!("{proposal_id}.json"));
    let metadata = fs::metadata(&path).context("源码补丁提案不存在")?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        bail!("源码补丁提案无效或过大");
    }
    serde_json::from_slice(&fs::read(path)?).context("源码补丁提案 JSON 无效")
}

pub(super) fn write_review_artifact(
    root: &Path,
    id: &str,
    source_file: &str,
    edits: &[SourcePatchEdit],
) -> Result<String> {
    let directory = proposal_directory(root, true)?;
    let relative = format!(".elon/ui-tuner/headless-design/source-patches/{id}.review.patch");
    let mut review = format!("--- a/{source_file}\n+++ b/{source_file}\n");
    for edit in edits {
        review.push_str(&format!("@@ bytes {},{} @@\n", edit.start, edit.end));
        append_review_lines(&mut review, '-', &edit.before);
        append_review_lines(&mut review, '+', &edit.replacement);
    }
    crate::node_agent_atomic_file::write(
        &directory.join(format!("{id}.review.patch")),
        review.as_bytes(),
    )?;
    Ok(relative)
}

pub(super) fn proposal_view(proposal: &SourcePatchProposal) -> Value {
    json!({
        "schemaVersion":proposal.schema_version,"proposalId":proposal.proposal_id,
        "revision":proposal.revision,"writebackPlanId":proposal.writeback_plan_id,
        "draftId":proposal.draft_id,"draftRevision":proposal.draft_revision,
        "sourceFile":proposal.source_file,"sourceShaBefore":proposal.source_sha_before,
        "sourceShaAfter":proposal.source_sha_after,"status":proposal.status,
        "decisionReason":proposal.decision_reason,"reviewArtifactPath":proposal.review_artifact_path,
        "edits":proposal.edits.iter().map(|edit| json!({
            "start":edit.start,"end":edit.end,"beforeSha256":edit.before_sha256,
            "replacementSha256":edit.replacement_sha256,"beforeBytes":edit.before.len(),
            "replacementBytes":edit.replacement.len()
        })).collect::<Vec<_>>(),
        "createdAt":proposal.created_at,"updatedAt":proposal.updated_at,
        "appliedAt":proposal.applied_at,"contentEmbedded":false
    })
}

pub(super) fn plan_rollback(root: &Path, proposal: &SourcePatchProposal) -> Result<Value> {
    let mut delta: i64 = 0;
    let mut inverse = Vec::with_capacity(proposal.edits.len());
    for edit in &proposal.edits {
        let start = (edit.start as i64 + delta) as u64;
        let end = start + edit.replacement.len() as u64;
        inverse.push(SourcePatchEdit {
            start,
            end,
            before_sha256: edit.replacement_sha256.clone(),
            replacement_sha256: edit.before_sha256.clone(),
            before: edit.replacement.clone(),
            replacement: edit.before.clone(),
        });
        delta += edit.replacement.len() as i64 - (edit.end - edit.start) as i64;
    }
    let digest =
        Sha256::digest(format!("{}\0{}", proposal.proposal_id, proposal.revision).as_bytes());
    let rollback_id = format!("rollback_{}", &hex::encode(digest)[..32]);
    let review_artifact_path =
        write_review_artifact(root, &rollback_id, &proposal.source_file, &inverse)?;
    let record = SourceRollbackPlan {
        schema_version: 1,
        rollback_id: rollback_id.clone(),
        revision: 1,
        proposal_id: proposal.proposal_id.clone(),
        proposal_revision: proposal.revision,
        source_file: proposal.source_file.clone(),
        expected_source_revision: proposal.source_sha_after.clone(),
        target_source_revision: proposal.source_sha_before.clone(),
        edits: inverse.clone(),
        status: "PLANNED".to_string(),
        review_artifact_path: review_artifact_path.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let directory = proposal_directory(root, true)?;
    crate::node_agent_atomic_file::write(
        &directory.join(format!("{rollback_id}.json")),
        &serde_json::to_vec_pretty(&record)?,
    )?;
    Ok(json!({
        "schemaVersion":1,"rollbackId":rollback_id,"revision":1,
        "proposalId":proposal.proposal_id,"proposalRevision":proposal.revision,
        "sourceFile":proposal.source_file,"expectedSourceRevision":proposal.source_sha_after,
        "targetSourceRevision":proposal.source_sha_before,"status":"PLANNED",
        "reviewArtifactPath":review_artifact_path,
        "edits":inverse.iter().map(|edit| json!({
            "start":edit.start,"end":edit.end,"beforeSha256":edit.before_sha256,
            "replacementSha256":edit.replacement_sha256,"beforeBytes":edit.before.len(),
            "replacementBytes":edit.replacement.len()
        })).collect::<Vec<_>>(),"contentEmbedded":false
    }))
}

fn proposal_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/source-patches");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("源码补丁提案目录越出项目");
    }
    Ok(canonical)
}

fn validate_id(value: &str, prefix: &str) -> Result<()> {
    if value.len() != prefix.len() + 32
        || !value.starts_with(prefix)
        || !value[prefix.len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("记录 ID 无效");
    }
    Ok(())
}

fn validate_sha(value: &str) -> Result<()> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("expectedBeforeSha256 必须是 sha256:<64 hex>");
    }
    Ok(())
}

fn append_review_lines(output: &mut String, prefix: char, value: &str) {
    if value.is_empty() {
        output.push(prefix);
        output.push('\n');
        return;
    }
    for line in value.lines() {
        output.push(prefix);
        output.push_str(line);
        output.push('\n');
    }
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

fn required_text_allow_empty<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}
