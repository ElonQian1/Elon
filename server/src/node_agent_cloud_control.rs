//! Immutable cloud-authorization boundary for admitted CLI work.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use tokio::sync::watch;
use tracing::warn;

const ABSOLUTE_DEADLINE_RECHECK: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub(crate) struct CloudControlDeadline {
    wall_deadline: DateTime<Utc>,
    monotonic_deadline: Instant,
}

pub(crate) fn freeze_cloud_control_deadline(
    requires_cloud_control: bool,
    server_deadline: Option<&str>,
    server_issued_at: Option<&str>,
    server_ttl_ms: Option<u64>,
    managed_lease_deadline: Option<&str>,
) -> Result<Option<CloudControlDeadline>> {
    freeze_cloud_control_deadline_at(
        requires_cloud_control,
        server_deadline,
        server_issued_at,
        server_ttl_ms,
        managed_lease_deadline,
        Utc::now(),
        Instant::now(),
    )
}

pub(crate) fn validate_registered_cloud_control(
    requires_cloud_control: bool,
    cloud_connected: bool,
    deadline: Option<&CloudControlDeadline>,
) -> Result<()> {
    validate_registered_cloud_control_at(
        requires_cloud_control,
        cloud_connected,
        deadline,
        Utc::now(),
        Instant::now(),
    )
}

pub(crate) fn spawn_absolute_deadline_cancel(
    deadline: Option<CloudControlDeadline>,
    cancel_tx: watch::Sender<bool>,
    req_id: String,
) {
    let Some(deadline) = deadline else {
        return;
    };
    tokio::spawn(async move {
        wait_until_absolute_deadline(&deadline).await;
        warn!(%req_id, "cloud authorization deadline reached; canceling CLI task");
        let _ = cancel_tx.send(true);
    });
}

fn parse_deadline(raw: &str) -> Result<DateTime<Utc>> {
    let clean = raw.trim();
    if clean.is_empty() || clean != raw {
        bail!("云控授权截止时间必须是规范的 RFC3339 时间。");
    }
    DateTime::parse_from_rfc3339(clean)
        .with_context(|| "云控授权截止时间不是有效的 RFC3339 时间")
        .map(|deadline| deadline.with_timezone(&Utc))
}

fn freeze_cloud_control_deadline_at(
    requires_cloud_control: bool,
    server_deadline: Option<&str>,
    server_issued_at: Option<&str>,
    server_ttl_ms: Option<u64>,
    managed_lease_deadline: Option<&str>,
    wall_now: DateTime<Utc>,
    monotonic_now: Instant,
) -> Result<Option<CloudControlDeadline>> {
    if !requires_cloud_control {
        if server_deadline.is_some()
            || server_issued_at.is_some()
            || server_ttl_ms.is_some()
            || managed_lease_deadline.is_some()
        {
            bail!("无需云控的任务不能携带云控授权窗口。");
        }
        return Ok(None);
    }
    let server_deadline = parse_deadline(
        server_deadline.ok_or_else(|| anyhow::anyhow!("云控任务缺少绝对授权截止时间。"))?,
    )?;
    let server_issued_at = parse_deadline(
        server_issued_at.ok_or_else(|| anyhow::anyhow!("云控任务缺少服务器签发时间。"))?,
    )?;
    let server_ttl_ms =
        server_ttl_ms.ok_or_else(|| anyhow::anyhow!("云控任务缺少服务器冻结 TTL。"))?;
    if server_ttl_ms == 0 {
        bail!("云控任务的服务器冻结 TTL 已耗尽。");
    }
    let signed_window_ms = server_deadline
        .signed_duration_since(server_issued_at)
        .num_milliseconds();
    if signed_window_ms <= 0 || server_ttl_ms > signed_window_ms as u64 {
        bail!("云控任务的服务器冻结 TTL 与签发窗口不一致。");
    }
    let managed_lease_deadline = managed_lease_deadline.map(parse_deadline).transpose()?;
    let wall_deadline = managed_lease_deadline
        .map(|managed| server_deadline.min(managed))
        .unwrap_or(server_deadline);
    let wall_remaining = remaining_until(&wall_deadline, wall_now)
        .ok_or_else(|| anyhow::anyhow!("云控授权已到期，已拒绝启动。"))?;
    let monotonic_ttl = Duration::from_millis(server_ttl_ms).min(wall_remaining);
    let monotonic_deadline = monotonic_now
        .checked_add(monotonic_ttl)
        .ok_or_else(|| anyhow::anyhow!("云控任务的服务器冻结 TTL 超出本机计时范围。"))?;
    Ok(Some(CloudControlDeadline {
        wall_deadline,
        monotonic_deadline,
    }))
}

fn validate_registered_cloud_control_at(
    requires_cloud_control: bool,
    cloud_connected: bool,
    deadline: Option<&CloudControlDeadline>,
    wall_now: DateTime<Utc>,
    monotonic_now: Instant,
) -> Result<()> {
    if !requires_cloud_control {
        return Ok(());
    }
    if !cloud_connected {
        bail!("云控任务注册后检测到服务器已断线，已拒绝启动。");
    }
    let deadline = deadline.ok_or_else(|| anyhow::anyhow!("云控任务缺少冻结授权窗口。"))?;
    if deadline_reached_at(deadline, wall_now, monotonic_now) {
        bail!("云控授权已到期，已拒绝启动。");
    }
    Ok(())
}

async fn wait_until_absolute_deadline(deadline: &CloudControlDeadline) {
    loop {
        let wall_remaining = remaining_until(&deadline.wall_deadline, Utc::now());
        let monotonic_remaining = deadline
            .monotonic_deadline
            .checked_duration_since(Instant::now());
        let Some(remaining) = min_remaining(wall_remaining, monotonic_remaining) else {
            return;
        };
        tokio::time::sleep(remaining.min(ABSOLUTE_DEADLINE_RECHECK)).await;
    }
}

fn deadline_reached_at(
    deadline: &CloudControlDeadline,
    wall_now: DateTime<Utc>,
    monotonic_now: Instant,
) -> bool {
    wall_now >= deadline.wall_deadline || monotonic_now >= deadline.monotonic_deadline
}

fn min_remaining(first: Option<Duration>, second: Option<Duration>) -> Option<Duration> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        _ => None,
    }
}

fn remaining_until(deadline: &DateTime<Utc>, now: DateTime<Utc>) -> Option<Duration> {
    deadline
        .signed_duration_since(now)
        .to_std()
        .ok()
        .filter(|remaining| !remaining.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn freezes_earliest_server_and_managed_lease_deadline() {
        let monotonic_now = Instant::now();
        let frozen = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:10:00Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(600_000),
            Some("2030-01-01T00:05:00Z"),
            at("2030-01-01T00:01:00Z"),
            monotonic_now,
        )
        .unwrap()
        .unwrap();

        assert_eq!(frozen.wall_deadline, at("2030-01-01T00:05:00Z"));
        assert_eq!(
            frozen.monotonic_deadline.duration_since(monotonic_now),
            Duration::from_secs(240)
        );
    }

    #[test]
    fn unmanaged_cloud_task_fails_closed_if_disconnect_wins_registration_race() {
        let monotonic_now = Instant::now();
        let deadline = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:10:00Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(600_000),
            None,
            at("2030-01-01T00:00:00Z"),
            monotonic_now,
        )
        .unwrap();

        let error = validate_registered_cloud_control_at(
            true,
            false,
            deadline.as_ref(),
            at("2030-01-01T00:00:00Z"),
            monotonic_now,
        )
        .unwrap_err();
        assert!(error.to_string().contains("服务器已断线"));
    }

    #[test]
    fn receipt_freezes_shorter_wall_remaining_after_network_delay() {
        let monotonic_now = Instant::now();
        let deadline = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:00:10Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(10_000),
            None,
            at("2030-01-01T00:00:07Z"),
            monotonic_now,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            deadline.monotonic_deadline.duration_since(monotonic_now),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn midrun_adoption_rejects_expired_authorization_before_retry() {
        let monotonic_now = Instant::now();
        let deadline = CloudControlDeadline {
            wall_deadline: at("2030-01-01T00:00:10Z"),
            monotonic_deadline: monotonic_now + Duration::from_secs(10),
        };
        let error = validate_registered_cloud_control_at(
            true,
            true,
            Some(&deadline),
            at("2030-01-01T00:00:10Z"),
            monotonic_now,
        )
        .unwrap_err();

        assert!(error.to_string().contains("已到期"));
    }

    #[test]
    fn controlled_task_without_server_ttl_fails_closed() {
        let error = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:00:10Z"),
            Some("2030-01-01T00:00:00Z"),
            None,
            None,
            at("2030-01-01T00:00:00Z"),
            Instant::now(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("缺少服务器冻结 TTL"));
    }

    #[test]
    fn clock_rollback_after_receipt_cannot_extend_monotonic_deadline() {
        let monotonic_now = Instant::now();
        let deadline = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:00:10Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(10_000),
            None,
            at("2030-01-01T00:00:02Z"),
            monotonic_now,
        )
        .unwrap()
        .unwrap();

        assert!(!deadline_reached_at(
            &deadline,
            at("1900-01-01T00:00:00Z"),
            monotonic_now + Duration::from_secs(7)
        ));
        assert!(deadline_reached_at(
            &deadline,
            at("1900-01-01T00:00:00Z"),
            monotonic_now + Duration::from_secs(8)
        ));
    }

    #[test]
    fn node_clock_behind_server_never_grants_more_than_server_ttl() {
        let monotonic_now = Instant::now();
        let deadline = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:00:10Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(10_000),
            None,
            at("2029-12-31T23:00:00Z"),
            monotonic_now,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            deadline.monotonic_deadline.duration_since(monotonic_now),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn ttl_larger_than_signed_issue_window_fails_closed() {
        let error = freeze_cloud_control_deadline_at(
            true,
            Some("2030-01-01T00:00:10Z"),
            Some("2030-01-01T00:00:00Z"),
            Some(10_001),
            None,
            at("2030-01-01T00:00:00Z"),
            Instant::now(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("签发窗口不一致"));
    }

    #[tokio::test]
    async fn midrun_adoption_absolute_deadline_cancels_active_handle() {
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let deadline = CloudControlDeadline {
            wall_deadline: Utc::now() + chrono::Duration::milliseconds(20),
            monotonic_deadline: Instant::now() + Duration::from_millis(20),
        };

        spawn_absolute_deadline_cancel(Some(deadline), cancel_tx, "req-midrun".to_string());

        tokio::time::timeout(Duration::from_secs(2), cancel_rx.changed())
            .await
            .expect("absolute deadline timer should fire")
            .expect("cancel sender should stay alive through deadline");
        assert!(*cancel_rx.borrow());
    }
}
