use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CloudControlDispatchWindow {
    pub(crate) issued_at: String,
    pub(crate) ttl_ms: u64,
}

pub(crate) fn freeze_cloud_control_dispatch_window(
    deadline: &str,
) -> Result<CloudControlDispatchWindow> {
    freeze_cloud_control_dispatch_window_at(deadline, chrono::Utc::now())
}

pub(super) fn freeze_cloud_control_dispatch_window_at(
    deadline: &str,
    issued_at: chrono::DateTime<chrono::Utc>,
) -> Result<CloudControlDispatchWindow> {
    let deadline = chrono::DateTime::parse_from_rfc3339(deadline)
        .map_err(|_| anyhow!("cloud authorization deadline is not valid RFC3339"))?
        .with_timezone(&chrono::Utc);
    let ttl_ms = deadline.signed_duration_since(issued_at).num_milliseconds();
    if ttl_ms <= 0 {
        return Err(anyhow!("cloud authorization deadline has expired"));
    }
    Ok(CloudControlDispatchWindow {
        issued_at: issued_at.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        ttl_ms: ttl_ms as u64,
    })
}
