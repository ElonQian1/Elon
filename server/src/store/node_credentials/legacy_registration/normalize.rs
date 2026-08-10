use anyhow::{bail, Result};

use super::super::clean_optional;
use super::LegacyNodeRegistrationRequest;

pub(super) struct NormalizedRegistrationRequest<'a> {
    pub(super) owner_user_id: &'a str,
    pub(super) proposed_agent_id: &'a str,
    pub(super) new_secret_hash: &'a str,
    pub(super) existing_agent_id: Option<&'a str>,
    pub(super) existing_secret_hash: Option<&'a str>,
    pub(super) install_id: Option<&'a str>,
    pub(super) label: Option<&'a str>,
    pub(super) device_name: Option<&'a str>,
    pub(super) current_bearer_token: Option<&'a str>,
}

impl<'a> NormalizedRegistrationRequest<'a> {
    pub(super) fn new(request: LegacyNodeRegistrationRequest<'a>) -> Result<Self> {
        Ok(Self {
            owner_user_id: required_trimmed(
                request.owner_user_id,
                "LEGACY_NODE_REGISTRATION_OWNER_INVALID",
            )?,
            proposed_agent_id: required_trimmed(
                request.proposed_agent_id,
                "LEGACY_NODE_REGISTRATION_AGENT_INVALID",
            )?,
            new_secret_hash: required_trimmed(
                request.new_secret_hash,
                "LEGACY_NODE_REGISTRATION_SECRET_HASH_INVALID",
            )?,
            existing_agent_id: clean_optional(request.existing_agent_id),
            existing_secret_hash: clean_optional(request.existing_secret_hash),
            install_id: clean_optional(request.install_id),
            label: clean_optional(request.label),
            device_name: clean_optional(request.device_name),
            current_bearer_token: clean_optional(request.current_bearer_token),
        })
    }
}

pub(super) fn required_trimmed<'a>(value: &'a str, error: &'static str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!(error);
    }
    Ok(value)
}
