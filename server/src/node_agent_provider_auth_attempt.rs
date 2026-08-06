use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::node_agent_provider_auth_attempt_store::ProviderAuthAttemptStore;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProviderLoginAttempt {
    pub(crate) schema_version: u32,
    pub(crate) login_id: String,
    pub(crate) provider_id: String,
    pub(crate) flow: String,
    pub(crate) state: String,
    pub(crate) request_id: Option<String>,
    pub(crate) verification_url: Option<String>,
    pub(crate) user_code: Option<String>,
    pub(crate) auth_url: Option<String>,
    pub(crate) remote_compatible: bool,
    pub(crate) recovered: bool,
    pub(crate) error: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) started_at_ms: u64,
    pub(crate) updated_at_ms: u64,
}

impl ProviderLoginAttempt {
    pub(crate) fn is_active(&self) -> bool {
        ProviderAttemptState::parse(&self.state).is_some_and(ProviderAttemptState::is_active)
    }

    pub(crate) fn is_terminal(&self) -> bool {
        !self.is_active()
    }

    pub(crate) fn retryable(&self) -> bool {
        ProviderAttemptState::parse(&self.state).is_some_and(ProviderAttemptState::is_retryable)
    }

    pub(crate) fn next_action(&self) -> &'static str {
        match ProviderAttemptState::parse(&self.state) {
            Some(ProviderAttemptState::Starting | ProviderAttemptState::WaitingForUser) => {
                "wait_or_cancel"
            }
            Some(ProviderAttemptState::Completed) => "refresh_provider_probe",
            Some(
                ProviderAttemptState::Failed
                | ProviderAttemptState::Canceled
                | ProviderAttemptState::Expired,
            ) => "start_new_login",
            None => "inspect_diagnostics",
        }
    }

    pub(crate) fn transition(
        &mut self,
        next: &str,
        error: Option<String>,
        error_code: Option<&str>,
        updated_at_ms: u64,
    ) -> Result<bool, ProviderAttemptTransitionError> {
        let current = ProviderAttemptState::parse(&self.state)
            .ok_or_else(|| ProviderAttemptTransitionError::UnknownState(self.state.clone()))?;
        let next = ProviderAttemptState::parse(next)
            .ok_or_else(|| ProviderAttemptTransitionError::UnknownState(next.to_string()))?;
        if current == next {
            return Ok(false);
        }
        if !current.can_transition_to(next) {
            return Err(ProviderAttemptTransitionError::InvalidTransition {
                from: current.as_str(),
                to: next.as_str(),
            });
        }
        self.state = next.as_str().to_string();
        self.error = error;
        self.error_code = error_code.map(ToOwned::to_owned);
        self.updated_at_ms = updated_at_ms;
        Ok(true)
    }

    pub(crate) fn recover_after_restart(&mut self, updated_at_ms: u64) {
        if self.is_active() {
            let _ = self.transition(
                "failed",
                Some("节点重启中断了厂商登录，请重新发起。".to_string()),
                Some("node_restarted"),
                updated_at_ms,
            );
        }
        self.recovered = true;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderAttemptState {
    Starting,
    WaitingForUser,
    Completed,
    Failed,
    Canceled,
    Expired,
}

impl ProviderAttemptState {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "starting" => Some(Self::Starting),
            "waiting_for_user" => Some(Self::WaitingForUser),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::WaitingForUser => "waiting_for_user",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
        }
    }

    fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::WaitingForUser)
    }

    fn is_retryable(self) -> bool {
        matches!(self, Self::Failed | Self::Canceled | Self::Expired)
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Starting,
                Self::WaitingForUser | Self::Failed | Self::Canceled | Self::Expired
            ) | (
                Self::WaitingForUser,
                Self::Completed | Self::Failed | Self::Canceled | Self::Expired
            )
        )
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProviderAttemptTransitionError {
    #[error("未知厂商登录状态: {0}")]
    UnknownState(String),
    #[error("厂商登录状态不能从 {from} 转换到 {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
}

pub(crate) async fn transition_attempt(
    view: &Arc<RwLock<ProviderLoginAttempt>>,
    state: &str,
    error: Option<String>,
    error_code: Option<&str>,
    journal: &ProviderAuthAttemptStore,
) {
    let mut view = view.write().await;
    match view.transition(
        state,
        error,
        error_code,
        crate::node_agent_provider_auth_runtime::now_ms(),
    ) {
        Ok(true) => journal.upsert(&view),
        Ok(false) => {}
        Err(error) => tracing::warn!(
            login_id = %view.login_id,
            provider_id = %view.provider_id,
            error = %error,
            "拒绝无效的厂商登录状态转换"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting() -> ProviderLoginAttempt {
        ProviderLoginAttempt {
            schema_version: 2,
            login_id: "login-1".to_string(),
            provider_id: "codex_cli".to_string(),
            flow: "device_code".to_string(),
            state: "waiting_for_user".to_string(),
            request_id: Some("request-1".to_string()),
            verification_url: None,
            user_code: None,
            auth_url: None,
            remote_compatible: true,
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn terminal_states_are_immutable_and_retries_are_explicit() {
        let mut attempt = waiting();
        assert!(attempt.transition("completed", None, None, 2).unwrap());
        assert!(!attempt.retryable());
        assert!(attempt
            .transition("failed", Some("late".to_string()), Some("late"), 3)
            .is_err());
        assert_eq!(attempt.state, "completed");
    }

    #[test]
    fn restart_recovery_is_terminal_retryable_and_secret_free() {
        let mut attempt = waiting();
        attempt.user_code = Some("SECRET".to_string());
        attempt.recover_after_restart(2);
        attempt.user_code = None;
        attempt.auth_url = None;
        assert_eq!(attempt.state, "failed");
        assert_eq!(attempt.error_code.as_deref(), Some("node_restarted"));
        assert!(attempt.retryable());
        assert_eq!(attempt.next_action(), "start_new_login");
    }
}
