use serde::Serialize;

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
        matches!(self.state.as_str(), "starting" | "waiting_for_user")
    }

    pub(crate) fn is_terminal(&self) -> bool {
        !self.is_active()
    }
}
