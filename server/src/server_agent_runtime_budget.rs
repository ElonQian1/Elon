// server/src/server_agent_runtime_budget.rs

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::store::{route_c_budget::RouteCBudgetRecordResult, Store};

const SECONDS_PER_DAY: u64 = 86_400;
const DEFAULT_DAILY_CALL_LIMIT: Option<usize> = None;
const DEFAULT_PER_USER_DAILY_CALL_LIMIT: Option<usize> = None;
const DAILY_CALL_LIMIT_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT";
const PER_USER_DAILY_CALL_LIMIT_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_PER_USER_DAILY_CALL_LIMIT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ServerRuntimeBudgetConfig {
    daily_call_limit: Option<usize>,
    per_user_daily_call_limit: Option<usize>,
    source: &'static str,
    per_user_source: &'static str,
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
    pub per_user_enabled: bool,
    pub per_user_source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_calls_today_for_user: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_user_daily_call_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_calls_today_for_user: Option<usize>,
    pub reset_after_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerRuntimeBudgetError {
    DailyCallLimitReached(ServerRuntimeBudgetStatus),
    UserDailyCallLimitReached(ServerRuntimeBudgetStatus),
    StoreUnavailable(ServerRuntimeBudgetStatus),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerRuntimeBudgetRecord {
    pub status: ServerRuntimeBudgetStatus,
    pub event_id: String,
}

impl ServerRuntimeBudgetConfig {
    fn current() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let raw_daily = lookup(DAILY_CALL_LIMIT_ENV);
        let raw_per_user = lookup(PER_USER_DAILY_CALL_LIMIT_ENV);
        let daily_call_limit = raw_daily
            .as_deref()
            .and_then(parse_limit)
            .or(DEFAULT_DAILY_CALL_LIMIT);
        let per_user_daily_call_limit = raw_per_user
            .as_deref()
            .and_then(parse_limit)
            .or(DEFAULT_PER_USER_DAILY_CALL_LIMIT);

        Self {
            daily_call_limit,
            per_user_daily_call_limit,
            source: if raw_daily.is_some() {
                DAILY_CALL_LIMIT_ENV
            } else {
                "default"
            },
            per_user_source: if raw_per_user.is_some() {
                PER_USER_DAILY_CALL_LIMIT_ENV
            } else {
                "default"
            },
        }
    }
}

impl ServerRuntimeBudgetStatus {
    pub(crate) fn ready(&self) -> bool {
        !matches!(self.status, "exhausted" | "user_exhausted" | "unavailable")
    }
}

impl ServerRuntimeBudgetError {
    pub(crate) fn retry_after_secs(&self) -> u64 {
        match self {
            Self::DailyCallLimitReached(status) => status.reset_after_secs,
            Self::UserDailyCallLimitReached(status) => status.reset_after_secs,
            Self::StoreUnavailable(_) => 30,
        }
    }

    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::DailyCallLimitReached(status) => {
                let limit = status.daily_call_limit.unwrap_or_default();
                format!(
                    "平台AI今日平台预算已用完：每日最多 {limit} 次平台模型调用，请稍后再试，或改用本机AI / 我的 API key。"
                )
            }
            Self::UserDailyCallLimitReached(status) => {
                let limit = status.per_user_daily_call_limit.unwrap_or_default();
                format!(
                    "平台AI今日个人额度已用完：每个用户每日最多 {limit} 次平台模型调用，请稍后再试，或改用本机AI / 我的 API key。"
                )
            }
            Self::StoreUnavailable(_) => {
                "平台AI预算系统暂时不可用，请稍后再试，或改用本机AI / 我的 API key。".to_string()
            }
        }
    }

    pub(crate) fn status(&self) -> &ServerRuntimeBudgetStatus {
        match self {
            Self::DailyCallLimitReached(status)
            | Self::UserDailyCallLimitReached(status)
            | Self::StoreUnavailable(status) => status,
        }
    }
}

pub(crate) fn server_runtime_budget_status(store: &Store) -> ServerRuntimeBudgetStatus {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    status_from_store(store, config, None, now)
}

pub(crate) fn server_runtime_budget_status_for_user(
    store: &Store,
    user_id: &str,
) -> ServerRuntimeBudgetStatus {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    status_from_store(store, config, Some(user_id), now)
}

pub(crate) fn try_record_route_c_call(
    store: &Store,
    user_id: &str,
    request_fingerprint: &str,
) -> Result<ServerRuntimeBudgetRecord, ServerRuntimeBudgetError> {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    record_from_store(store, config, user_id, request_fingerprint, now)
}

fn status_from_store(
    store: &Store,
    config: ServerRuntimeBudgetConfig,
    user_id: Option<&str>,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    let route_day = route_day_from_epoch_secs(now_secs);
    let used_calls_today = match store.route_c_budget_count_for_day(&route_day) {
        Ok(used) => used,
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                route_day,
                error = %error,
                "Route C daily budget status lookup failed"
            );
            return if config.daily_call_limit.is_some()
                || config.per_user_daily_call_limit.is_some()
            {
                unavailable_status(config, now_secs)
            } else {
                status_for_used_calls(config, 0, None, now_secs)
            };
        }
    };
    let used_calls_today_for_user = match user_id {
        Some(user_id) => match store.route_c_budget_count_for_day_and_user(&route_day, user_id) {
            Ok(used) => Some(used),
            Err(error) => {
                tracing::warn!(
                    target: "server_agent_runtime",
                    user_id,
                    route_day,
                    error = %error,
                    "Route C per-user daily budget status lookup failed"
                );
                return if config.per_user_daily_call_limit.is_some() {
                    unavailable_status(config, now_secs)
                } else {
                    status_for_used_calls(config, used_calls_today, None, now_secs)
                };
            }
        },
        None => None,
    };

    status_for_used_calls(
        config,
        used_calls_today,
        used_calls_today_for_user,
        now_secs,
    )
}

fn parse_limit(raw: &str) -> Option<usize> {
    raw.trim().parse::<usize>().ok().filter(|value| *value > 0)
}

fn record_from_store(
    store: &Store,
    config: ServerRuntimeBudgetConfig,
    user_id: &str,
    request_fingerprint: &str,
    now_secs: u64,
) -> Result<ServerRuntimeBudgetRecord, ServerRuntimeBudgetError> {
    let route_day = route_day_from_epoch_secs(now_secs);
    match store.route_c_budget_try_record_call(
        user_id,
        request_fingerprint,
        &route_day,
        config.daily_call_limit,
        config.per_user_daily_call_limit,
    ) {
        Ok(RouteCBudgetRecordResult::Recorded {
            event_id,
            total_used,
            user_used,
        }) => Ok(ServerRuntimeBudgetRecord {
            event_id,
            status: status_for_used_calls(config, total_used, Some(user_used), now_secs),
        }),
        Ok(RouteCBudgetRecordResult::PlatformLimitReached {
            total_used,
            user_used,
        }) => Err(ServerRuntimeBudgetError::DailyCallLimitReached(
            status_for_used_calls(config, total_used, Some(user_used), now_secs),
        )),
        Ok(RouteCBudgetRecordResult::UserLimitReached {
            total_used,
            user_used,
        }) => Err(ServerRuntimeBudgetError::UserDailyCallLimitReached(
            status_for_used_calls(config, total_used, Some(user_used), now_secs),
        )),
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                user_id,
                route_day,
                error = %error,
                "Route C daily budget event write failed"
            );
            if config.daily_call_limit.is_some() || config.per_user_daily_call_limit.is_some() {
                Err(ServerRuntimeBudgetError::StoreUnavailable(
                    unavailable_status(config, now_secs),
                ))
            } else {
                Ok(ServerRuntimeBudgetRecord {
                    event_id: String::new(),
                    status: status_for_used_calls(config, 0, Some(0), now_secs),
                })
            }
        }
    }
}

fn status_for_used_calls(
    config: ServerRuntimeBudgetConfig,
    used_calls_today: usize,
    used_calls_today_for_user: Option<usize>,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    let remaining_calls_today = config
        .daily_call_limit
        .map(|limit| limit.saturating_sub(used_calls_today));
    let remaining_calls_today_for_user = used_calls_today_for_user.and_then(|used| {
        config
            .per_user_daily_call_limit
            .map(|limit| limit.saturating_sub(used))
    });
    let status = match remaining_calls_today {
        Some(0) => "exhausted",
        _ if remaining_calls_today_for_user == Some(0) => "user_exhausted",
        Some(_) => "available",
        None if config.per_user_daily_call_limit.is_some() => "available",
        None => "unlimited",
    };
    ServerRuntimeBudgetStatus {
        enabled: config.daily_call_limit.is_some() || config.per_user_daily_call_limit.is_some(),
        status,
        source: config.source,
        used_calls_today,
        daily_call_limit: config.daily_call_limit,
        remaining_calls_today,
        per_user_enabled: config.per_user_daily_call_limit.is_some(),
        per_user_source: config.per_user_source,
        used_calls_today_for_user,
        per_user_daily_call_limit: config.per_user_daily_call_limit,
        remaining_calls_today_for_user,
        reset_after_secs: seconds_until_next_day(now_secs),
    }
}

fn unavailable_status(
    config: ServerRuntimeBudgetConfig,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    ServerRuntimeBudgetStatus {
        enabled: config.daily_call_limit.is_some() || config.per_user_daily_call_limit.is_some(),
        status: "unavailable",
        source: config.source,
        used_calls_today: 0,
        daily_call_limit: config.daily_call_limit,
        remaining_calls_today: None,
        per_user_enabled: config.per_user_daily_call_limit.is_some(),
        per_user_source: config.per_user_source,
        used_calls_today_for_user: None,
        per_user_daily_call_limit: config.per_user_daily_call_limit,
        remaining_calls_today_for_user: None,
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
#[path = "server_agent_runtime_budget_tests.rs"]
mod tests;
