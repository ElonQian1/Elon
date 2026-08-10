use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{node_register_api::RegisterNodeRequest, project_auth::LoginRequest};

const MAX_ACCOUNT_BYTES: usize = 320;
const MAX_PASSWORD_BYTES: usize = 4096;
const MAX_INSTALL_ID_BYTES: usize = 512;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BootstrapLoginRequest {
    account: String,
    password: String,
}

impl BootstrapLoginRequest {
    pub(super) fn into_login_request(self) -> Result<LoginRequest> {
        let account = self.account.trim();
        if account.is_empty()
            || account.len() > MAX_ACCOUNT_BYTES
            || self.password.is_empty()
            || self.password.len() > MAX_PASSWORD_BYTES
        {
            bail!("NODE_ENDPOINT_BOOTSTRAP_LOGIN_REQUEST_INVALID");
        }
        Ok(LoginRequest {
            account: account.to_string(),
            password: self.password,
            device_name: None,
            apk_version: None,
            remember_device: false,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BootstrapNodeRegistrationRequest {
    label: Option<String>,
    device_name: Option<String>,
    install_id: String,
    existing_agent_id: Option<String>,
    existing_secret: Option<String>,
}

impl BootstrapNodeRegistrationRequest {
    pub(super) fn into_register_node_request(self) -> Result<RegisterNodeRequest> {
        let install_id = self.install_id.trim();
        if install_id.is_empty()
            || install_id.len() > MAX_INSTALL_ID_BYTES
            || install_id != self.install_id.as_str()
            || install_id.chars().any(char::is_control)
        {
            bail!("NODE_ENDPOINT_BOOTSTRAP_INSTALL_ID_INVALID");
        }
        Ok(RegisterNodeRequest {
            label: self.label,
            device_name: self.device_name,
            install_id: Some(self.install_id),
            existing_agent_id: self.existing_agent_id,
            existing_secret: self.existing_secret,
        })
    }
}

#[derive(Serialize)]
pub(super) struct BootstrapNodeRegistrationResponse {
    agent_id: String,
    owner_user_id: String,
}

impl BootstrapNodeRegistrationResponse {
    pub(super) fn new(agent_id: String, owner_user_id: String) -> Self {
        Self {
            agent_id,
            owner_user_id,
        }
    }
}
