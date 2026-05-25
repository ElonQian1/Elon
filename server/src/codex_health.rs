use serde::Serialize;
use std::{
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{process::Command, sync::Mutex as AsyncMutex};

use crate::types::AppState;

const DEFAULT_PROBE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_STALE_AFTER_SECS: u64 = 60;
const DEFAULT_UNHEALTHY_COOLDOWN_SECS: u64 = 60;
const DEFAULT_PERIODIC_INTERVAL_SECS: u64 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNetworkStatus {
    Unknown,
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexNetworkHealthSnapshot {
    pub enabled: bool,
    pub status: CodexNetworkStatus,
    pub circuit_open: bool,
    pub checked_at_unix_ms: Option<u64>,
    pub age_ms: Option<u64>,
    pub next_check_after_unix_ms: Option<u64>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub last_probe_ms: Option<u128>,
    pub last_source: Option<String>,
}

#[derive(Debug)]
struct CodexNetworkHealthInner {
    status: CodexNetworkStatus,
    checked_at: Option<Instant>,
    checked_at_unix_ms: Option<u64>,
    consecutive_failures: u32,
    last_error: Option<String>,
    last_probe_ms: Option<u128>,
    last_source: Option<String>,
}

pub struct CodexNetworkHealth {
    enabled: bool,
    probe_timeout: Duration,
    stale_after: Duration,
    unhealthy_cooldown: Duration,
    periodic_interval: Duration,
    inner: AsyncMutex<CodexNetworkHealthInner>,
    probe_lock: AsyncMutex<()>,
}

impl CodexNetworkHealth {
    pub fn from_env() -> Self {
        Self {
            enabled: env_bool("CODEX_NETWORK_HEALTH_ENABLED", true),
            probe_timeout: Duration::from_secs(env_secs(
                "CODEX_NETWORK_HEALTH_PROBE_TIMEOUT_SECS",
                DEFAULT_PROBE_TIMEOUT_SECS,
            )),
            stale_after: Duration::from_secs(env_secs(
                "CODEX_NETWORK_HEALTH_STALE_SECS",
                DEFAULT_STALE_AFTER_SECS,
            )),
            unhealthy_cooldown: Duration::from_secs(env_secs(
                "CODEX_NETWORK_HEALTH_UNHEALTHY_COOLDOWN_SECS",
                DEFAULT_UNHEALTHY_COOLDOWN_SECS,
            )),
            periodic_interval: Duration::from_secs(env_secs(
                "CODEX_NETWORK_HEALTH_PERIODIC_SECS",
                DEFAULT_PERIODIC_INTERVAL_SECS,
            )),
            inner: AsyncMutex::new(CodexNetworkHealthInner {
                status: CodexNetworkStatus::Unknown,
                checked_at: None,
                checked_at_unix_ms: None,
                consecutive_failures: 0,
                last_error: None,
                last_probe_ms: None,
                last_source: None,
            }),
            probe_lock: AsyncMutex::new(()),
        }
    }

    pub fn periodic_interval(&self) -> Duration {
        self.periodic_interval
    }

    pub async fn snapshot(&self) -> CodexNetworkHealthSnapshot {
        let inner = self.inner.lock().await;
        self.snapshot_from_inner(&inner)
    }

    pub async fn ensure_ready(&self, source: &'static str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(error) = self.cached_gate_error().await {
            return Err(error);
        }

        let _probe_guard = self.probe_lock.lock().await;
        if let Some(error) = self.cached_gate_error().await {
            return Err(error);
        }

        let result = self.refresh(source).await;
        result.map(|_| ())
    }

    pub async fn refresh(
        &self,
        source: &'static str,
    ) -> Result<CodexNetworkHealthSnapshot, String> {
        if !self.enabled {
            return Ok(self.snapshot().await);
        }

        let started = Instant::now();
        let probe_result = run_codex_doctor_probe(self.probe_timeout).await;
        let probe_ms = started.elapsed().as_millis();
        let mut inner = self.inner.lock().await;
        inner.checked_at = Some(Instant::now());
        inner.checked_at_unix_ms = Some(current_unix_millis());
        inner.last_probe_ms = Some(probe_ms);
        inner.last_source = Some(source.to_string());

        match probe_result {
            Ok(()) => {
                inner.status = CodexNetworkStatus::Healthy;
                inner.consecutive_failures = 0;
                inner.last_error = None;
                Ok(self.snapshot_from_inner(&inner))
            }
            Err(error) => {
                let error = truncate_chars(&error, 700);
                inner.status = CodexNetworkStatus::Unhealthy;
                inner.consecutive_failures = inner.consecutive_failures.saturating_add(1);
                inner.last_error = Some(error.clone());
                Err(format!("Codex CLI network unhealthy: {error}"))
            }
        }
    }

    pub async fn mark_cli_success(&self, source: &'static str) {
        if !self.enabled {
            return;
        }
        let mut inner = self.inner.lock().await;
        inner.status = CodexNetworkStatus::Healthy;
        inner.checked_at = Some(Instant::now());
        inner.checked_at_unix_ms = Some(current_unix_millis());
        inner.consecutive_failures = 0;
        inner.last_error = None;
        inner.last_source = Some(source.to_string());
    }

    pub async fn mark_cli_failure(&self, source: &'static str, error: &str) {
        if !self.enabled {
            return;
        }
        let mut inner = self.inner.lock().await;
        inner.status = CodexNetworkStatus::Unhealthy;
        inner.checked_at = Some(Instant::now());
        inner.checked_at_unix_ms = Some(current_unix_millis());
        inner.consecutive_failures = inner.consecutive_failures.saturating_add(1);
        inner.last_error = Some(truncate_chars(error, 700));
        inner.last_source = Some(source.to_string());
    }

    async fn cached_gate_error(&self) -> Option<String> {
        let inner = self.inner.lock().await;
        if inner.status == CodexNetworkStatus::Healthy {
            if inner
                .checked_at
                .map(|checked_at| checked_at.elapsed() < self.stale_after)
                .unwrap_or(false)
            {
                return None;
            }
            return None;
        }

        if inner.status == CodexNetworkStatus::Unhealthy
            && inner
                .checked_at
                .map(|checked_at| checked_at.elapsed() < self.unhealthy_cooldown)
                .unwrap_or(false)
        {
            let error = inner
                .last_error
                .as_deref()
                .unwrap_or("recent Codex network probe failed");
            return Some(format!("Codex CLI network unhealthy: {error}"));
        }

        None
    }

    fn snapshot_from_inner(&self, inner: &CodexNetworkHealthInner) -> CodexNetworkHealthSnapshot {
        let now = current_unix_millis();
        let age_ms = inner
            .checked_at_unix_ms
            .map(|checked_at| now.saturating_sub(checked_at));
        let next_check_after_unix_ms =
            inner
                .checked_at_unix_ms
                .map(|checked_at| match inner.status {
                    CodexNetworkStatus::Healthy => checked_at + self.stale_after.as_millis() as u64,
                    CodexNetworkStatus::Unhealthy => {
                        checked_at + self.unhealthy_cooldown.as_millis() as u64
                    }
                    CodexNetworkStatus::Unknown => checked_at,
                });
        let circuit_open = self.enabled
            && inner.status == CodexNetworkStatus::Unhealthy
            && inner
                .checked_at
                .map(|checked_at| checked_at.elapsed() < self.unhealthy_cooldown)
                .unwrap_or(false);

        CodexNetworkHealthSnapshot {
            enabled: self.enabled,
            status: inner.status,
            circuit_open,
            checked_at_unix_ms: inner.checked_at_unix_ms,
            age_ms,
            next_check_after_unix_ms,
            consecutive_failures: inner.consecutive_failures,
            last_error: inner.last_error.clone(),
            last_probe_ms: inner.last_probe_ms,
            last_source: inner.last_source.clone(),
        }
    }
}

pub fn spawn_codex_network_monitor(state: Arc<AppState>) {
    if !state.ai_cli.enabled
        || !state.ai_cli.options.iter().any(|option| {
            option.provider.eq_ignore_ascii_case("codex")
                || option.id.to_ascii_lowercase().contains("codex")
                || option.bin.to_ascii_lowercase().contains("codex")
        })
    {
        return;
    }

    tokio::spawn(async move {
        match state.codex_network.refresh("startup").await {
            Ok(snapshot) => tracing::info!(?snapshot, "Codex network health probe passed"),
            Err(error) => tracing::warn!(error, "Codex network health probe failed"),
        }

        let mut interval = tokio::time::interval(state.codex_network.periodic_interval());
        loop {
            interval.tick().await;
            match state.codex_network.refresh("periodic").await {
                Ok(snapshot) => tracing::info!(?snapshot, "Codex network health probe passed"),
                Err(error) => tracing::warn!(error, "Codex network health probe failed"),
            }
        }
    });
}

async fn run_codex_doctor_probe(timeout: Duration) -> Result<(), String> {
    let mut cmd = Command::new("codex");
    cmd.args(["doctor", "--summary", "--no-color"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| format!("codex doctor timed out after {}s", timeout.as_secs()))?
        .map_err(|error| format!("failed to run codex doctor: {error}"))?;

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    classify_codex_doctor_output(output.status.success(), &combined)
}

fn classify_codex_doctor_output(status_success: bool, output: &str) -> Result<(), String> {
    let lower = output.to_ascii_lowercase();
    let looks_healthy = lower.contains("0 fail")
        && lower.contains("websocket")
        && lower.contains("reachability")
        && lower.contains("reachable")
        && !looks_like_codex_network_error(&lower);

    if status_success && looks_healthy {
        Ok(())
    } else {
        Err(truncate_chars(compact_probe_output(output), 700))
    }
}

pub fn is_codex_network_error_text(text: &str) -> bool {
    looks_like_codex_network_error(&text.to_ascii_lowercase())
}

fn looks_like_codex_network_error(lower: &str) -> bool {
    lower.contains("responses websocket failed")
        || lower.contains("tls handshake eof")
        || lower.contains("ssl_error_syscall")
        || lower.contains("failed to refresh available models")
        || lower.contains("reachability") && lower.contains("unreachable")
        || lower.contains("request timed out")
        || lower.contains("connection timed out")
        || lower.contains("connection reset")
        || lower.contains("network is unreachable")
        || lower.contains("failed to connect")
        || lower.contains("error sending request")
        || lower.contains("http/request failed")
        || lower.contains("stream disconnected before completion")
        || lower.contains("proxy connect")
        || lower.contains("websocket")
            && (lower.contains("failed") || lower.contains("reconnecting"))
}

fn compact_probe_output(output: &str) -> &str {
    output
        .lines()
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("websocket")
                || lower.contains("reachability")
                || lower.contains("failed")
                || lower.contains("unreachable")
                || lower.contains("timed out")
                || lower.contains("tls")
        })
        .unwrap_or_else(|| output.trim())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_secs(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| (1..=3600).contains(secs))
        .unwrap_or(default)
}

fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for ch in text.chars().take(max_chars) {
        output.push(ch);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_output_healthy_when_zero_failures_and_reachable() {
        let output = "Connectivity\n  websocket    connected (HTTP 101 Switching Protocols)\n  reachability active provider endpoints are reachable over HTTP\n13 ok · 0 fail ok";
        assert!(classify_codex_doctor_output(true, output).is_ok());
    }

    #[test]
    fn doctor_output_unhealthy_when_websocket_fails() {
        let output = "Connectivity\n  websocket    Responses WebSocket failed\n  reachability one or more required provider endpoints are unreachable over HTTP\n10 ok · 1 fail failed";
        assert!(classify_codex_doctor_output(true, output).is_err());
    }

    #[test]
    fn network_error_text_matches_codex_websocket_failures() {
        assert!(is_codex_network_error_text(
            "failed to connect to websocket: IO error: tls handshake eof"
        ));
        assert!(is_codex_network_error_text(
            "failed to refresh available models: timeout waiting for child process to exit"
        ));
    }
}
