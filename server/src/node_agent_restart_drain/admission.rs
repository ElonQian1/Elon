use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use super::{classification::DrainClassification, load_checkpoint, save_checkpoint};
use super::{RestartCheckpoint, CHECKPOINT_PROTOCOL};

pub(super) struct AlreadyCurrentResolution {
    pub(super) checkpoint: RestartCheckpoint,
    pub(super) persist: bool,
    pub(super) preserved_pending_target: bool,
}

pub(super) fn handle_already_current_update(
    source: &str,
    download_url: Option<&str>,
    target_release_identity: Option<&str>,
) -> Result<Option<Value>, String> {
    let current_release_identity = crate::node_agent_release_identity::current();
    let _admission = super::drain_admission_lock()
        .lock()
        .map_err(|_| "更新排空准入锁已损坏，已拒绝更新。".to_string())?;
    let existing =
        load_checkpoint().map_err(|error| format!("读取更新检查点失败，已拒绝更新：{error:#}"))?;
    let Some(resolution) = converge_already_current_update(
        existing,
        source,
        download_url,
        target_release_identity,
        &current_release_identity,
    ) else {
        return Ok(None);
    };
    if resolution.persist {
        save_checkpoint(&resolution.checkpoint)?;
        super::drain_running().store(false, Ordering::Release);
    }
    Ok(Some(json!({
        "ok": true,
        "deferred": false,
        "already_current": true,
        "preserved_pending_target": resolution.preserved_pending_target,
        "restart_recovery": resolution.checkpoint.payload(),
    })))
}

pub(super) fn converge_already_current_update(
    existing: Option<RestartCheckpoint>,
    source: &str,
    download_url: Option<&str>,
    target_release_identity: Option<&str>,
    current_release_identity: &str,
) -> Option<AlreadyCurrentResolution> {
    let target = target_release_identity.map(str::trim)?;
    if !exact_target_identity(Some(target))
        || !restart_target_matches(target, current_release_identity)
    {
        return None;
    }

    let same_target = |checkpoint: &RestartCheckpoint| {
        checkpoint.target_release_identity.as_deref().map(str::trim) == Some(target)
    };
    if let Some(checkpoint) = existing.as_ref() {
        let different_target_is_pending = !same_target(checkpoint)
            && matches!(
                checkpoint.state.as_str(),
                "draining" | "applying" | "restart_scheduled"
            );
        if different_target_is_pending {
            return Some(AlreadyCurrentResolution {
                checkpoint: checkpoint.clone(),
                persist: false,
                preserved_pending_target: true,
            });
        }
    }

    let mut checkpoint = match existing {
        Some(mut checkpoint) if same_target(&checkpoint) => {
            checkpoint.source = source.to_string();
            checkpoint.download_url = download_url.map(str::to_string);
            checkpoint
        }
        Some(previous) => {
            let mut replacement = RestartCheckpoint::draining(
                source,
                Vec::new(),
                download_url.map(str::to_string),
                Some(target.to_string()),
            );
            replacement.superseded_update_id = Some(previous.update_id);
            replacement
        }
        None => RestartCheckpoint::draining(
            source,
            Vec::new(),
            download_url.map(str::to_string),
            Some(target.to_string()),
        ),
    };
    checkpoint.target_release_identity = Some(target.to_string());
    checkpoint.active_task_ids.clear();
    checkpoint.recoverable_task_ids.clear();
    checkpoint.stale_registry_task_ids.clear();
    checkpoint.stale_cancel_proofs.clear();
    checkpoint.transition(
        "runtime_online",
        "目标发布身份已经在线；重复更新已安全收敛，未重启或中断当前任务。",
    );
    Some(AlreadyCurrentResolution {
        checkpoint,
        persist: true,
        preserved_pending_target: false,
    })
}

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

pub(super) fn restart_target_matches(target: &str, current: &str) -> bool {
    let target = target.trim();
    let current = current.trim();
    current == target || current.starts_with(&format!("{target}+"))
}

pub(super) fn checkpoint_should_continue_draining(checkpoint: &RestartCheckpoint) -> bool {
    checkpoint.state == "draining"
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: &str = "0.3.69+aaaaaaaa";

    #[test]
    fn exact_target_already_online_converges_same_target_checkpoint() {
        let mut existing = RestartCheckpoint::draining(
            "cloud_broadcast",
            vec!["stale-task".to_string()],
            Some("https://example.invalid/old.zip".to_string()),
            Some(CURRENT.to_string()),
        );
        existing.recoverable_task_ids = vec!["recoverable-task".to_string()];
        existing.stale_registry_task_ids = vec!["stale-registry-task".to_string()];
        let existing_update_id = existing.update_id.clone();

        let resolution = converge_already_current_update(
            Some(existing),
            "codex_mcp",
            None,
            Some(CURRENT),
            CURRENT,
        )
        .expect("the exact online target should converge");

        assert!(resolution.persist);
        assert!(!resolution.preserved_pending_target);
        assert_eq!(resolution.checkpoint.update_id, existing_update_id);
        assert_eq!(resolution.checkpoint.state, "runtime_online");
        assert!(resolution.checkpoint.active_task_ids.is_empty());
        assert!(resolution.checkpoint.recoverable_task_ids.is_empty());
        assert!(resolution.checkpoint.stale_registry_task_ids.is_empty());
        assert!(resolution.checkpoint.stale_cancel_proofs.is_empty());
        assert!(resolution.checkpoint.message.contains("已经在线"));
    }

    #[test]
    fn exact_target_online_supersedes_only_inactive_different_checkpoint() {
        let mut old = RestartCheckpoint::draining(
            "cloud_broadcast",
            Vec::new(),
            None,
            Some("0.3.68+cccccccc".to_string()),
        );
        old.transition("failed", "old update failed");
        let old_update_id = old.update_id.clone();

        let resolution =
            converge_already_current_update(Some(old), "codex_mcp", None, Some(CURRENT), CURRENT)
                .expect("the exact online target should replace inactive history");

        assert!(resolution.persist);
        assert_eq!(resolution.checkpoint.state, "runtime_online");
        assert_eq!(
            resolution.checkpoint.superseded_update_id.as_deref(),
            Some(old_update_id.as_str())
        );
    }

    #[test]
    fn exact_target_online_does_not_overwrite_different_pending_target() {
        let pending = RestartCheckpoint::draining(
            "cloud_broadcast",
            vec!["live-task".to_string()],
            None,
            Some("0.3.70+bbbbbbbb".to_string()),
        );
        let pending_update_id = pending.update_id.clone();

        let resolution = converge_already_current_update(
            Some(pending),
            "codex_mcp",
            None,
            Some(CURRENT),
            CURRENT,
        )
        .expect("the already-current request should be an explicit no-op");

        assert!(!resolution.persist);
        assert!(resolution.preserved_pending_target);
        assert_eq!(resolution.checkpoint.update_id, pending_update_id);
        assert_eq!(resolution.checkpoint.state, "draining");
        assert_eq!(resolution.checkpoint.active_task_ids, ["live-task"]);
    }

    #[test]
    fn drain_loop_stops_after_checkpoint_converges() {
        let draining = RestartCheckpoint::draining("test", Vec::new(), None, None);
        assert!(checkpoint_should_continue_draining(&draining));

        for state in ["applying", "restart_scheduled", "runtime_online", "failed"] {
            let mut checkpoint = draining.clone();
            checkpoint.transition(state, "done");
            assert!(!checkpoint_should_continue_draining(&checkpoint));
        }
    }
}
