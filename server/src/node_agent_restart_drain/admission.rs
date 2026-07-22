use serde_json::{json, Value};

use super::{classification::DrainClassification, load_checkpoint, save_checkpoint};
use super::{RestartCheckpoint, CHECKPOINT_PROTOCOL};

pub(super) fn admit_running_update(
    source: &str,
    download_url: Option<&str>,
    target_release_identity: Option<&str>,
    classification: &DrainClassification,
) -> Result<Value, String> {
    let existing = load_checkpoint()
        .map_err(|error| format!("读取正在排空的更新检查点失败，已拒绝重复广播：{error:#}"))?
        .ok_or_else(|| "更新排空锁已占用但检查点缺失，已拒绝重复广播。".to_string())?;
    if update_request_matches(&existing, source, download_url, target_release_identity) {
        return Ok(json!({
            "ok": true,
            "deferred": true,
            "coalesced": true,
            "restart_recovery": existing.payload(),
        }));
    }
    if existing.state != "draining" || !exact_target_identity(target_release_identity) {
        return Err(format!(
            "已有更新目标 {} 正在 {}，已拒绝把它复用到目标 {}。",
            existing
                .target_release_identity
                .as_deref()
                .unwrap_or("<missing>"),
            existing.state,
            target_release_identity.unwrap_or("<missing>")
        ));
    }

    let previous_update_id = existing.update_id;
    let mut replacement = RestartCheckpoint::draining(
        source,
        classification.blocking.clone(),
        download_url.map(str::to_string),
        target_release_identity.map(str::to_string),
    );
    replacement.recoverable_task_ids = classification.recoverable.clone();
    replacement.stale_registry_task_ids = classification.stale.clone();
    replacement.stale_cancel_proofs = classification.stale_cancel_proofs.clone();
    replacement.superseded_update_id = Some(previous_update_id.clone());
    replacement.message =
        "收到新的精确更新目标；旧排空事务已被可审计替换，继续等待当前阻塞任务。".to_string();
    save_checkpoint(&replacement)?;
    Ok(json!({
        "ok": true,
        "deferred": true,
        "retargeted": true,
        "superseded_update_id": previous_update_id,
        "restart_recovery": replacement.payload(),
    }))
}

pub(super) fn update_request_matches(
    checkpoint: &RestartCheckpoint,
    source: &str,
    download_url: Option<&str>,
    target_release_identity: Option<&str>,
) -> bool {
    checkpoint.protocol == CHECKPOINT_PROTOCOL
        && checkpoint.source == source
        && checkpoint.download_url.as_deref().map(str::trim) == download_url.map(str::trim)
        && checkpoint.target_release_identity.as_deref().map(str::trim)
            == target_release_identity.map(str::trim)
}

pub(super) fn exact_target_identity(value: Option<&str>) -> bool {
    let Some((version, git_sha)) = value
        .map(str::trim)
        .and_then(|value| value.rsplit_once('+'))
    else {
        return false;
    };
    !version.trim().is_empty()
        && git_sha.trim().len() >= 7
        && git_sha.trim().bytes().all(|byte| byte.is_ascii_hexdigit())
}
