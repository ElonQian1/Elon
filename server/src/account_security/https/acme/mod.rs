//! Native ACME IP certificates; no external executables or trust bypass in production.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};
mod challenge;
mod config;
mod diagnostics;
mod issuer;
mod storage;
pub(super) use storage::pair;

pub(super) fn configured_bundle(
    get: impl Fn(&str) -> Option<String>,
) -> Result<Option<std::path::PathBuf>> {
    Ok(config::Config::from_lookup(get)?
        .filter(|c| !c.staging)
        .map(|c| c.bundle()))
}

pub(super) async fn bootstrap(legacy: &super::config::Config) -> Result<()> {
    let Some(config) = config::Config::from_env()? else {
        return Ok(());
    };
    if config.staging || managed_pair().ok().flatten().is_some() {
        return Ok(());
    }
    let prior = fs::read(&legacy.certificate).and_then(|mut bytes| {
        bytes.extend(fs::read(&legacy.key)?);
        Ok(bytes)
    });
    if prior
        .ok()
        .is_some_and(|v| storage::validate(&v, config.ip, true, 0).is_ok())
    {
        return Ok(());
    }
    // Fresh installation can obtain its first trusted pair without another tool.
    // The persisted fence prevents a crash/restart loop from exhausting CA limits.
    let path = config.dir.join("status.json");
    let mut status: Status = fs::read(&path)
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_default();
    let now = chrono::Utc::now().timestamp();
    if status.next_attempt > now {
        anyhow::bail!("ACCOUNT_ACME_BOOTSTRAP_RETRY_LATER");
    }
    status.next_attempt = now + delay(status.failures.saturating_add(1));
    status.failures = status.failures.saturating_add(1);
    status.result = "bootstrap_requesting".into();
    storage::atomic_private(&path, &serde_json::to_vec(&status)?)?;
    let end = tokio::time::timeout(Duration::from_secs(600), issuer::issue(&config)).await??;
    status.expires_at = Some(end);
    status.failures = 0;
    status.next_attempt = end - 48 * 3600;
    status.result = "renewed".into();
    storage::atomic_private(&path, &serde_json::to_vec(&status)?)?;
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
struct Status {
    next_attempt: i64,
    failures: u32,
    expires_at: Option<i64>,
    result: String,
    #[serde(default)]
    last_error: Option<String>,
}
fn delay(failures: u32) -> i64 {
    (900_i64 * (1_i64 << failures.saturating_sub(1).min(4))).min(6 * 3600)
}

pub(super) fn managed_pair() -> Result<Option<Vec<u8>>> {
    let Some(config) = config::Config::from_env()? else {
        return Ok(None);
    };
    if config.staging || !config.bundle().exists() {
        return Ok(None);
    }
    let bytes = fs::read(config.bundle())?;
    storage::validate(&bytes, config.ip, true, 0)?;
    Ok(Some(bytes))
}

pub(super) fn validate_fallback(bytes: &[u8]) -> Result<()> {
    if let Some(config) = config::Config::from_env()? {
        storage::validate(bytes, config.ip, true, 0)?;
    }
    Ok(())
}

/// Starts after the existing HTTPS listener has successfully bound. A failed
/// renewal never interrupts application traffic or overwrites its legacy pair.
pub(super) fn spawn() -> Result<()> {
    let Some(config) = config::Config::from_env()? else {
        return Ok(());
    };
    tokio::spawn(async move {
        let path = config.dir.join("status.json");
        let mut status: Status = fs::read(&path)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        loop {
            let now = chrono::Utc::now().timestamp();
            let expiry = fs::read(config.bundle())
                .ok()
                .and_then(|v| storage::validate(&v, config.ip, !config.staging, 0).ok());
            status.expires_at = expiry;
            if expiry.is_none_or(|end| end - now <= 48 * 3600) && status.next_attempt <= now {
                // Persist a retry fence before network I/O, including across restarts.
                status.next_attempt = now + delay(status.failures + 1);
                status.result = "requesting".into();
                if storage::atomic_private(&path, &serde_json::to_vec(&status).unwrap_or_default())
                    .is_ok()
                {
                    match tokio::time::timeout(Duration::from_secs(600), issuer::issue(&config))
                        .await
                    {
                        Ok(Ok(end)) => {
                            status.expires_at = Some(end);
                            status.failures = 0;
                            status.next_attempt = end - 48 * 3600;
                            status.result = "renewed".into();
                            status.last_error = None;
                            tracing::info!(
                                expires_at = end,
                                staging = config.staging,
                                "ACCOUNT_ACME_RENEWED"
                            );
                        }
                        result => {
                            let code = match result {
                                Ok(Err(error)) => error.to_string(),
                                _ => "ACCOUNT_ACME_REQUEST_TIMEOUT".into(),
                            };
                            status.failures = status.failures.saturating_add(1);
                            status.result = "renewal_failed_previous_certificate_retained".into();
                            status.last_error = Some(code.clone());
                            tracing::warn!(
                                failures = status.failures,
                                staging = config.staging,
                                code,
                                "ACCOUNT_ACME_RENEWAL_FAILED"
                            );
                        }
                    }
                    let _ = storage::atomic_private(
                        &path,
                        &serde_json::to_vec(&status).unwrap_or_default(),
                    );
                } else {
                    tracing::warn!("ACCOUNT_ACME_STATE_WRITE_FAILED");
                }
            }
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn renewal_retries_are_bounded_and_back_off() {
        assert_eq!(super::delay(1), 900);
        assert_eq!(super::delay(2), 1800);
        assert!(super::delay(100) < 86400);
    }
}
