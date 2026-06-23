// server/src/server_agent_runtime_budget.rs

use serde::Serialize;
use std::{
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

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
}

#[derive(Debug, Default)]
struct RuntimeBudgetState {
    day: u64,
    used_calls: usize,
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
        self.status != "exhausted"
    }
}

impl ServerRuntimeBudgetError {
    pub(crate) fn retry_after_secs(&self) -> u64 {
        match self {
            Self::DailyCallLimitReached(status) => status.reset_after_secs,
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
        }
    }

    pub(crate) fn status(&self) -> &ServerRuntimeBudgetStatus {
        match self {
            Self::DailyCallLimitReached(status) => status,
        }
    }
}

pub(crate) fn server_runtime_budget_status() -> ServerRuntimeBudgetStatus {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    let mut state = budget_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    status_for_state(config, &mut state, now)
}

pub(crate) fn try_record_route_c_call(
) -> Result<ServerRuntimeBudgetStatus, ServerRuntimeBudgetError> {
    let config = ServerRuntimeBudgetConfig::current();
    let now = current_epoch_secs();
    let mut state = budget_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    record_for_state(config, &mut state, now)
}

fn record_for_state(
    config: ServerRuntimeBudgetConfig,
    state: &mut RuntimeBudgetState,
    now_secs: u64,
) -> Result<ServerRuntimeBudgetStatus, ServerRuntimeBudgetError> {
    reset_state_if_needed(state, now_secs);
    if config
        .daily_call_limit
        .is_some_and(|limit| state.used_calls >= limit)
    {
        return Err(ServerRuntimeBudgetError::DailyCallLimitReached(
            status_for_state(config, state, now_secs),
        ));
    }

    state.used_calls += 1;
    Ok(status_for_state(config, state, now_secs))
}

fn status_for_state(
    config: ServerRuntimeBudgetConfig,
    state: &mut RuntimeBudgetState,
    now_secs: u64,
) -> ServerRuntimeBudgetStatus {
    reset_state_if_needed(state, now_secs);
    let remaining_calls_today = config
        .daily_call_limit
        .map(|limit| limit.saturating_sub(state.used_calls));
    let status = match remaining_calls_today {
        Some(0) => "exhausted",
        Some(_) => "available",
        None => "unlimited",
    };
    ServerRuntimeBudgetStatus {
        enabled: config.daily_call_limit.is_some(),
        status,
        source: config.source,
        used_calls_today: state.used_calls,
        daily_call_limit: config.daily_call_limit,
        remaining_calls_today,
        reset_after_secs: seconds_until_next_day(now_secs),
    }
}

fn reset_state_if_needed(state: &mut RuntimeBudgetState, now_secs: u64) {
    let day = day_from_epoch_secs(now_secs);
    if state.day != day {
        state.day = day;
        state.used_calls = 0;
    }
}

fn budget_state() -> &'static Mutex<RuntimeBudgetState> {
    static STATE: OnceLock<Mutex<RuntimeBudgetState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RuntimeBudgetState::default()))
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn day_from_epoch_secs(epoch_secs: u64) -> u64 {
    epoch_secs / SECONDS_PER_DAY
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
        record_for_state, status_for_state, RuntimeBudgetState, ServerRuntimeBudgetConfig,
        ServerRuntimeBudgetError, DAILY_CALL_LIMIT_ENV, SECONDS_PER_DAY,
    };

    #[test]
    fn budget_defaults_to_unlimited_but_counts_visibility() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|_| None);
        let mut state = RuntimeBudgetState::default();

        let first = record_for_state(config, &mut state, 10).unwrap();
        let second = record_for_state(config, &mut state, 20).unwrap();

        assert!(!second.enabled);
        assert_eq!(second.status, "unlimited");
        assert_eq!(first.used_calls_today, 1);
        assert_eq!(second.used_calls_today, 2);
        assert_eq!(second.remaining_calls_today, None);
    }

    #[test]
    fn budget_enforces_operator_daily_call_limit() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "2".to_string())
        });
        let mut state = RuntimeBudgetState::default();

        let first = record_for_state(config, &mut state, 10).unwrap();
        let second = record_for_state(config, &mut state, 11).unwrap();
        let exhausted = record_for_state(config, &mut state, 12).unwrap_err();

        assert!(first.enabled);
        assert_eq!(first.remaining_calls_today, Some(1));
        assert_eq!(second.status, "exhausted");
        assert_eq!(second.remaining_calls_today, Some(0));
        assert!(matches!(
            exhausted,
            ServerRuntimeBudgetError::DailyCallLimitReached(_)
        ));
        assert!(exhausted.public_message().contains("每日最多 2 次"));
    }

    #[test]
    fn budget_resets_on_next_utc_day() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "1".to_string())
        });
        let mut state = RuntimeBudgetState::default();

        let first = record_for_state(config, &mut state, SECONDS_PER_DAY - 2).unwrap();
        let exhausted = status_for_state(config, &mut state, SECONDS_PER_DAY - 1);
        let next_day = status_for_state(config, &mut state, SECONDS_PER_DAY + 1);

        assert_eq!(first.remaining_calls_today, Some(0));
        assert_eq!(exhausted.status, "exhausted");
        assert_eq!(next_day.used_calls_today, 0);
        assert_eq!(next_day.status, "available");
    }

    #[test]
    fn budget_ignores_invalid_operator_limit_values() {
        for raw in ["", "0", "not-a-number"] {
            let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
                (name == DAILY_CALL_LIMIT_ENV).then(|| raw.to_string())
            });
            let mut state = RuntimeBudgetState::default();
            let status = status_for_state(config, &mut state, 10);

            assert!(!status.enabled);
            assert_eq!(status.status, "unlimited");
            assert_eq!(status.remaining_calls_today, None);
        }
    }
}
