// server/src/server_agent_runtime_budget.rs

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::store::Store;

const SECONDS_PER_DAY: u64 = 86_400;
const DEFAULT_DAILY_CALL_LIMIT: Option<usize> = None;
const DAILY_CALL_LIMIT_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ServerRuntimeBudgetConfig {
    daily_call_limit: Option<usize>,
    source: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeBudgetStatus {
    pub enabled: bool,
    pub status: &'static str,
    pub source: &'static str,
    pub used_calls_today: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_call_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_calls_today: Option<usize>,
    pub reset_after_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerRuntimeBudgetError {
    DailyCallLimitReached(ServerRuntimeBudgetStatus),
    StoreUnavailable(ServerRuntimeBudgetStatus),
}

impl ServerRuntimeBudgetConfig {
    fn current() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let Some(raw) = lookup(DAILY_CALL_LIMIT_ENV) else {
            return Self {
                daily_call_limit: DEFAULT_DAILY_CALL_LIMIT,
                source: "default",
            };
        };
        let daily_call_limit = raw.trim().parse::<usize>().ok().filter(|value| *value > 0);
        Self {
            daily_call_limit,
            source: DAILY_CALL_LIMIT_ENV,
        }
    }
}

impl ServerRuntimeBudgetStatus {
    pub(crate) fn ready(&self) -> bool {
        !matches!(self.status, "exhausted" | "unavailable")
    }
}

impl ServerRuntimeBudgetError {
    pub(crate) fn retry_after_secs(&self) -> u64 {
        match self {
            Self::DailyCallLimitReached(status) => status.reset_after_secs,
            Self::StoreUnavailable(_) => 30,
        }
    }

    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::DailyCallLimitReached(status) => {
                let limit = status.daily_call_limit.unwrap_or_default();
                format!(
                    "Route C 远程模型今日平台预算已用完：每日最多 {limit} 次服务器模型调用，请稍后再试或改用本机 CLI / 自带 API Key。"
                )
            }
            Self::StoreUnavailable(_) => {
                "Route C 远程模型预算系统暂时不可用，请稍后再试或改用本机 CLI / 自带 API Key。"
                    .to_string()
            }
        }
    }

    pub(crate) fn status(&self) -> &ServerRuntimeBudgetStatus {
        match self {
            Self::DailyCallLimitReached(status) | Self::StoreUnavailable(status) => status,
        }
    }
}

pub(crate) fn server_runtime_budget_status(store: &Store) -> ServerRuntimeBudgetStatus {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    status_from_store(store, config, now)
}

pub(crate) fn try_record_route_c_call(
    store: &Store,
    user_id: &str,
    request_fingerprint: &str,
) -> Result<ServerRuntimeBudgetStatus, ServerRuntimeBudgetError> {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    record_from_store(store, config, user_id, request_fingerprint, now)
}

fn status_from_store(
    store: &Store,
    config: ServerRuntimeBudgetConfig,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    let route_day = route_day_from_epoch_secs(now_secs);
    match store.route_c_budget_count_for_day(&route_day) {
        Ok(used) => status_for_used_calls(config, used, now_secs),
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                route_day,
                error = %error,
                "Route C daily budget status lookup failed"
            );
            if config.daily_call_limit.is_some() {
                unavailable_status(config, now_secs)
            } else {
                status_for_used_calls(config, 0, now_secs)
            }
        }
    }
}

fn record_from_store(
    store: &Store,
    config: ServerRuntimeBudgetConfig,
    user_id: &str,
    request_fingerprint: &str,
    now_secs: u64,
) -> Result<ServerRuntimeBudgetStatus, ServerRuntimeBudgetError> {
    let route_day = route_day_from_epoch_secs(now_secs);
    match store.route_c_budget_try_record_call(
        user_id,
        request_fingerprint,
        &route_day,
        config.daily_call_limit,
    ) {
        Ok((true, used)) => Ok(status_for_used_calls(config, used, now_secs)),
        Ok((false, used)) => Err(ServerRuntimeBudgetError::DailyCallLimitReached(
            status_for_used_calls(config, used, now_secs),
        )),
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                user_id,
                route_day,
                error = %error,
                "Route C daily budget event write failed"
            );
            if config.daily_call_limit.is_some() {
                Err(ServerRuntimeBudgetError::StoreUnavailable(
                    unavailable_status(config, now_secs),
                ))
            } else {
                Ok(status_for_used_calls(config, 0, now_secs))
            }
        }
    }
}

fn status_for_used_calls(
    config: ServerRuntimeBudgetConfig,
    used_calls_today: usize,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    let remaining_calls_today = config
        .daily_call_limit
        .map(|limit| limit.saturating_sub(used_calls_today));
    let status = match remaining_calls_today {
        Some(0) => "exhausted",
        Some(_) => "available",
        None => "unlimited",
    };
    ServerRuntimeBudgetStatus {
        enabled: config.daily_call_limit.is_some(),
        status,
        source: config.source,
        used_calls_today,
        daily_call_limit: config.daily_call_limit,
        remaining_calls_today,
        reset_after_secs: seconds_until_next_day(now_secs),
    }
}

fn unavailable_status(
    config: ServerRuntimeBudgetConfig,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    ServerRuntimeBudgetStatus {
        enabled: config.daily_call_limit.is_some(),
        status: "unavailable",
        source: config.source,
        used_calls_today: 0,
        daily_call_limit: config.daily_call_limit,
        remaining_calls_today: None,
        reset_after_secs: seconds_until_next_day(now_secs),
    }
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn route_day_from_epoch_secs(epoch_secs: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(epoch_secs as i64, 0)
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn seconds_until_next_day(epoch_secs: u64) -> u64 {
    let elapsed_today = epoch_secs % SECONDS_PER_DAY;
    if elapsed_today == 0 {
        SECONDS_PER_DAY
    } else {
        SECONDS_PER_DAY - elapsed_today
    }
}

#[cfg(test)]
mod tests {
    use super::{
        route_day_from_epoch_secs, status_for_used_calls, unavailable_status,
        ServerRuntimeBudgetConfig, DAILY_CALL_LIMIT_ENV, SECONDS_PER_DAY,
    };

    #[test]
    fn budget_status_defaults_to_unlimited_but_preserves_usage_visibility() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|_| None);
        let status = status_for_used_calls(config, 2, 10);

        assert!(!status.enabled);
        assert_eq!(status.status, "unlimited");
        assert_eq!(status.used_calls_today, 2);
        assert_eq!(status.remaining_calls_today, None);
    }

    #[test]
    fn budget_status_reports_exhausted_operator_daily_call_limit() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "2".to_string())
        });
        let available = status_for_used_calls(config, 1, 10);
        let exhausted = status_for_used_calls(config, 2, 11);

        assert!(available.ready());
        assert_eq!(available.remaining_calls_today, Some(1));
        assert_eq!(exhausted.status, "exhausted");
        assert_eq!(exhausted.remaining_calls_today, Some(0));
        assert!(!exhausted.ready());
    }

    #[test]
    fn route_day_uses_utc_day_boundary() {
        assert_eq!(route_day_from_epoch_secs(SECONDS_PER_DAY - 1), "1970-01-01");
        assert_eq!(route_day_from_epoch_secs(SECONDS_PER_DAY), "1970-01-02");
    }

    #[test]
    fn unavailable_budget_status_blocks_when_operator_limit_is_configured() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "1".to_string())
        });
        let status = unavailable_status(config, 10);

        assert_eq!(status.status, "unavailable");
        assert!(!status.ready());
        assert_eq!(status.daily_call_limit, Some(1));
    }

    #[test]
    fn budget_ignores_invalid_operator_limit_values() {
        for raw in ["", "0", "not-a-number"] {
            let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
                (name == DAILY_CALL_LIMIT_ENV).then(|| raw.to_string())
            });
            let status = status_for_used_calls(config, 0, 10);

            assert!(!status.enabled);
            assert_eq!(status.status, "unlimited");
            assert_eq!(status.remaining_calls_today, None);
        }
    }
}
