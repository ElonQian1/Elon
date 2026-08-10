use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};

use super::owner_api::{OwnerEndpointApi, SecretMutationDelivery};
use super::persistence;
use super::types::{
    CurrentEndpointCredential, EndpointAuthorityBinding, EndpointCredentialState,
    PendingEndpointMutation, PendingMutationAction,
};

const MAX_REPLAY_RECOVERY_STEPS: usize = 3;

pub(super) struct BootstrapTarget<'a> {
    pub(super) agent_id: &'a str,
    pub(super) owner_user_id: &'a str,
    pub(super) install_id: &'a str,
    pub(super) current_hint: Option<EndpointAuthorityBinding>,
}

pub(super) async fn bootstrap(
    state: &mut EndpointCredentialState,
    origin: &str,
    bearer: &str,
    password: &str,
    target: BootstrapTarget<'_>,
) -> Result<EndpointAuthorityBinding> {
    let origin = super::normalize_endpoint_https_origin(origin)?;
    validate_target(&target)?;
    require_compatible_state(state, &origin, &target)?;
    reconcile_pending_with_authoritative_hint(state, &origin, target.current_hint.as_ref())?;

    if state.pending_mutation.is_none() {
        if let Some(hint) = target.current_hint.as_ref() {
            if let Some(binding) = usable_current_binding(state, hint) {
                return Ok(binding.clone());
            }
        } else if state.current.is_some() {
            bail!("NODE_ENDPOINT_AUTHORITY_RESPONSE_REQUIRED");
        }
        match target.current_hint.as_ref() {
            Some(binding) => prepare_recovery(state, &origin, binding.clone())?,
            None => prepare_issue(state, &origin, &target)?,
        }
    }

    let api = OwnerEndpointApi::new(&origin)?;
    for _ in 0..MAX_REPLAY_RECOVERY_STEPS {
        let pending = state
            .pending_mutation
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_PENDING_MUTATION_REQUIRED"))?;
        let delivery = api.execute(bearer, password, pending).await?;
        match delivery {
            SecretMutationDelivery::SecretVisible { binding, secret } => {
                persistence::save_parts(
                    true,
                    Some(&origin),
                    Some((&binding, Some(&secret))),
                    None,
                )?;
                state.endpoint_required = true;
                state.endpoint_https_origin = Some(origin);
                state.current = Some(CurrentEndpointCredential {
                    binding: binding.clone(),
                    secret: Some(secret),
                });
                state.pending_mutation = None;
                return Ok(binding);
            }
            SecretMutationDelivery::ReplayWithoutSecret { binding } => {
                prepare_recovery(state, &origin, binding)?;
            }
        }
    }
    bail!("NODE_ENDPOINT_SECRET_REPLAY_RECOVERY_RETRY_REQUIRED")
}

fn reconcile_pending_with_authoritative_hint(
    state: &mut EndpointCredentialState,
    origin: &str,
    current_hint: Option<&EndpointAuthorityBinding>,
) -> Result<()> {
    let (Some(pending), Some(current_hint)) = (state.pending_mutation.as_ref(), current_hint)
    else {
        return Ok(());
    };
    let must_recover_current = match pending.action {
        PendingMutationAction::Issue => true,
        PendingMutationAction::Recover => {
            pending.expected_credential.as_ref().is_none_or(|expected| {
                expected.credential_id != current_hint.credential_id
                    || expected.credential_revision != current_hint.credential_revision
                    || expected.credential_digest != current_hint.credential_digest
            })
        }
    };
    if must_recover_current {
        prepare_recovery(state, origin, current_hint.clone())?;
    }
    Ok(())
}

fn usable_current_binding<'a>(
    state: &'a EndpointCredentialState,
    hint: &EndpointAuthorityBinding,
) -> Option<&'a EndpointAuthorityBinding> {
    let Some(current) = state.current.as_ref() else {
        return None;
    };
    if !current.binding.same_credential(hint) {
        return None;
    }
    if current.binding.status == "active" && current.secret.is_some() {
        Some(&current.binding)
    } else {
        None
    }
}

fn prepare_issue(
    state: &mut EndpointCredentialState,
    origin: &str,
    target: &BootstrapTarget<'_>,
) -> Result<()> {
    if state.endpoint_required && state.current.is_some() {
        bail!("NODE_ENDPOINT_ISSUE_FORBIDDEN_FOR_EXISTING_AUTHORITY");
    }
    let pending = new_pending(
        PendingMutationAction::Issue,
        target.agent_id,
        target.owner_user_id,
        target.install_id,
        None,
    );
    persistence::save_parts(true, Some(origin), None, Some(&pending))?;
    state.endpoint_required = true;
    state.endpoint_https_origin = Some(origin.to_string());
    state.current = None;
    state.pending_mutation = Some(pending);
    Ok(())
}

fn prepare_recovery(
    state: &mut EndpointCredentialState,
    origin: &str,
    binding: EndpointAuthorityBinding,
) -> Result<()> {
    binding.validate()?;
    if !matches!(binding.status.as_str(), "active" | "revoked") {
        bail!("NODE_ENDPOINT_RECOVERY_REQUIRES_CURRENT_CREDENTIAL");
    }
    let pending = new_pending(
        PendingMutationAction::Recover,
        &binding.agent_id,
        &binding.owner_user_id,
        &binding.install_id,
        Some(binding.expected()),
    );
    persistence::save_parts(true, Some(origin), Some((&binding, None)), Some(&pending))?;
    state.endpoint_required = true;
    state.endpoint_https_origin = Some(origin.to_string());
    state.current = Some(CurrentEndpointCredential {
        binding,
        secret: None,
    });
    state.pending_mutation = Some(pending);
    Ok(())
}

fn new_pending(
    action: PendingMutationAction,
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
    expected_credential: Option<super::types::ExpectedEndpointCredential>,
) -> PendingEndpointMutation {
    PendingEndpointMutation {
        action,
        authorization_issuance_request_id: format!("nai_{}", uuid::Uuid::new_v4().simple()),
        credential_mutation_request_id: format!("ncm_{}", uuid::Uuid::new_v4().simple()),
        agent_id: agent_id.to_string(),
        owner_user_id: owner_user_id.to_string(),
        install_id: install_id.to_string(),
        expected_credential,
        prepared_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

fn require_compatible_state(
    state: &EndpointCredentialState,
    origin: &str,
    target: &BootstrapTarget<'_>,
) -> Result<()> {
    if let Some(existing_origin) = state.endpoint_https_origin.as_deref() {
        if existing_origin != origin {
            bail!("NODE_ENDPOINT_HTTPS_ORIGIN_DRIFT");
        }
    }
    if let Some(current) = state.current.as_ref() {
        require_identity(
            &current.binding.agent_id,
            &current.binding.owner_user_id,
            &current.binding.install_id,
            target,
        )?;
    }
    if let Some(pending) = state.pending_mutation.as_ref() {
        require_identity(
            &pending.agent_id,
            &pending.owner_user_id,
            &pending.install_id,
            target,
        )?;
    }
    if let Some(hint) = target.current_hint.as_ref() {
        hint.validate()?;
        require_identity(
            &hint.agent_id,
            &hint.owner_user_id,
            &hint.install_id,
            target,
        )?;
    }
    Ok(())
}

fn require_identity(
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
    target: &BootstrapTarget<'_>,
) -> Result<()> {
    if agent_id != target.agent_id
        || owner_user_id != target.owner_user_id
        || install_id != target.install_id
    {
        bail!("NODE_ENDPOINT_BOOTSTRAP_IDENTITY_DRIFT");
    }
    Ok(())
}

fn validate_target(target: &BootstrapTarget<'_>) -> Result<()> {
    for (value, max) in [
        (target.agent_id, 160_usize),
        (target.owner_user_id, 160_usize),
        (target.install_id, 512_usize),
    ] {
        if value.is_empty()
            || value != value.trim()
            || value.len() > max
            || value.chars().any(|character| character.is_control())
        {
            bail!("NODE_ENDPOINT_BOOTSTRAP_TARGET_INVALID");
        }
    }
    Ok(())
}
