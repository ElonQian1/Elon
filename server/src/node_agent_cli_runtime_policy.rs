use serde::{Deserialize, Serialize};

pub(crate) const SUPERVISED_CODEX_TIMEOUT_ENV: &str = "ELON_SUPERVISED_CODEX_TIMEOUT_SECS";
pub(crate) const SUPERVISED_CODEX_IDLE_TIMEOUT_ENV: &str =
    "ELON_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS";
pub(crate) const SUPERVISED_CODEX_HEARTBEAT_ENV: &str = "ELON_SUPERVISED_CODEX_HEARTBEAT_SECS";
pub(crate) const DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS: u64 = 6 * 60 * 60;
pub(crate) const DEFAULT_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS: u64 = 15 * 60;
pub(crate) const DEFAULT_SUPERVISED_CODEX_HEARTBEAT_SECS: u64 = 15;
const MAX_SUPERVISED_CODEX_TIMEOUT_SECS: u64 = 24 * 60 * 60;
const MAX_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS: u64 = 2 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CliRuntimePolicy {
    pub(crate) mode: String,
    pub(crate) total_timeout_secs: u64,
    pub(crate) idle_timeout_secs: u64,
    pub(crate) heartbeat_secs: u64,
    pub(crate) progress_aware: bool,
}

impl CliRuntimePolicy {
    pub(crate) fn fixed(total_timeout_secs: u64) -> Self {
        Self {
            mode: "fixed_total".to_string(),
            total_timeout_secs,
            idle_timeout_secs: total_timeout_secs,
            heartbeat_secs: DEFAULT_SUPERVISED_CODEX_HEARTBEAT_SECS,
            progress_aware: false,
        }
    }

    pub(crate) fn supervised_codex_from_env() -> Self {
        Self::supervised_codex_with_config(
            std::env::var(SUPERVISED_CODEX_TIMEOUT_ENV).ok().as_deref(),
            std::env::var(SUPERVISED_CODEX_IDLE_TIMEOUT_ENV)
                .ok()
                .as_deref(),
            std::env::var(SUPERVISED_CODEX_HEARTBEAT_ENV)
                .ok()
                .as_deref(),
        )
    }

    pub(crate) fn supervised_codex_with_config(
        total: Option<&str>,
        idle: Option<&str>,
        heartbeat: Option<&str>,
    ) -> Self {
        let total_timeout_secs = parse_bounded(
            total,
            DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS,
            20 * 60 + 1,
            MAX_SUPERVISED_CODEX_TIMEOUT_SECS,
        );
        let idle_timeout_secs = parse_bounded(
            idle,
            DEFAULT_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS,
            30,
            MAX_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS,
        )
        .min(total_timeout_secs);
        let heartbeat_secs =
            parse_bounded(heartbeat, DEFAULT_SUPERVISED_CODEX_HEARTBEAT_SECS, 1, 60)
                .min(idle_timeout_secs.max(1));
        Self {
            mode: "progress_aware".to_string(),
            total_timeout_secs,
            idle_timeout_secs,
            heartbeat_secs,
            progress_aware: true,
        }
    }
}

pub(crate) fn policy_for(
    cli_name: &str,
    full_access: bool,
    desktop_supervised: bool,
) -> CliRuntimePolicy {
    if cli_name.trim().eq_ignore_ascii_case("codex") && desktop_supervised {
        return CliRuntimePolicy::supervised_codex_from_env();
    }
    let total = match cli_name.trim().to_ascii_lowercase().as_str() {
        "codex" if full_access => 1200,
        "codex" => 300,
        _ => 180,
    };
    CliRuntimePolicy::fixed(total)
}

fn parse_bounded(value: Option<&str>, default: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| (min..=max).contains(value))
        .unwrap_or(default)
}

pub(crate) fn terminate_process_tree(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = std::process::Command::new("taskkill.exe");
        command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        let _ = command.spawn();
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &format!("-{pid}")])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervised_policy_is_progress_aware_and_bounded() {
        let policy =
            CliRuntimePolicy::supervised_codex_with_config(Some("7200"), Some("120"), Some("2"));
        assert_eq!(policy.total_timeout_secs, 7200);
        assert_eq!(policy.idle_timeout_secs, 120);
        assert_eq!(policy.heartbeat_secs, 2);
        assert!(policy.progress_aware);
        assert_eq!(policy.mode, "progress_aware");
    }

    #[test]
    fn supervised_policy_defaults_to_six_hour_total_and_fifteen_minute_idle() {
        let policy = CliRuntimePolicy::supervised_codex_with_config(None, None, None);
        assert_eq!(policy.total_timeout_secs, 6 * 60 * 60);
        assert_eq!(policy.idle_timeout_secs, 15 * 60);
        assert!(policy.progress_aware);
    }

    #[test]
    fn invalid_values_fall_back_without_infinite_timeout() {
        let policy = CliRuntimePolicy::supervised_codex_with_config(Some("0"), Some("bad"), None);
        assert_eq!(
            policy.total_timeout_secs,
            DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS
        );
        assert_eq!(
            policy.idle_timeout_secs,
            DEFAULT_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS
        );
        assert!(policy.total_timeout_secs <= MAX_SUPERVISED_CODEX_TIMEOUT_SECS);
    }
}
