//! Validation and invariant guards for the V258 transport-target Service.

use anyhow::Error as AnyError;

use crate::{
    compute_federation::external_pool_adapter_upstream_transport_target::{
        validate_upstream_transport_dns_hostname, UPSTREAM_TRANSPORT_TARGET_EFFECT,
        UPSTREAM_TRANSPORT_TARGET_NO_EFFECT, UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT,
        UPSTREAM_TRANSPORT_TARGET_STATUS,
    },
    store::{
        ExternalPoolAdapterUpstreamTransportTargetPolicySummary,
        ExternalPoolAdapterUpstreamTransportTargetRevocationSummary,
        ExternalPoolAdapterUpstreamTransportTargetSummary,
    },
};

use super::{
    external_pool_adapter_installation::ExternalPoolAdapterInstallationFsError,
    external_pool_adapter_upstream_transport_target_service::{
        CreateUpstreamTransportTargetBody, RevokeUpstreamTransportTargetBody,
        UpstreamTransportTargetActor, UpstreamTransportTargetServiceError,
    },
};

pub(super) fn validate_create(
    binding: &str,
    candidate: &str,
    profile: &str,
    body: &CreateUpstreamTransportTargetBody,
) -> Result<(), UpstreamTransportTargetServiceError> {
    validate_path(binding, candidate, profile, None)?;
    if !body.confirm_upstream_transport_target {
        return Err(invalid(
            "upstream transport target requires explicit confirmation",
        ));
    }
    validate_digest(&body.expected_profile_digest, "profile digest")?;
    validate_digest(&body.expected_candidate_digest, "candidate digest")?;
    validate_digest(
        &body.expected_provider_binding_digest,
        "Provider binding digest",
    )?;
    validate_digest(&body.expected_target_policy_digest, "target policy digest")?;
    validate_digest(
        &body.draft.expected_tls_leaf_spki_sha256,
        "expected TLS leaf SPKI digest",
    )?;
    validate_upstream_transport_dns_hostname(&body.draft.dns_hostname)
        .map_err(UpstreamTransportTargetServiceError::Invalid)?;
    if body.draft.port == 0 {
        return Err(invalid("upstream transport target port is invalid"));
    }
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    if let Some(predecessor) = &body.expected_predecessor {
        validate_identifier(&predecessor.target_id, 240, "predecessor target ID")?;
        validate_digest(&predecessor.target_digest, "predecessor target digest")?;
    }
    Ok(())
}

pub(super) fn validate_revoke(
    binding: &str,
    candidate: &str,
    profile: &str,
    target: &str,
    body: &RevokeUpstreamTransportTargetBody,
) -> Result<(), UpstreamTransportTargetServiceError> {
    validate_path(binding, candidate, profile, Some(target))?;
    if !body.confirm_revocation {
        return Err(invalid(
            "upstream transport-target revocation requires confirmation",
        ));
    }
    validate_digest(&body.expected_target_digest, "target digest")?;
    validate_digest(&body.expected_profile_digest, "profile digest")?;
    validate_identifier(&body.idempotency_key, 240, "idempotency key")?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "upstream transport-target revocation reason is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_path(
    binding: &str,
    candidate: &str,
    profile: &str,
    target: Option<&str>,
) -> Result<(), UpstreamTransportTargetServiceError> {
    validate_identifier(binding, 240, "Provider binding ID")?;
    validate_identifier(candidate, 240, "activation candidate ID")?;
    validate_identifier(profile, 240, "runtime launch-profile ID")?;
    if let Some(target) = target {
        validate_identifier(target, 240, "upstream transport-target ID")?;
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        Err(invalid(format!("{label} is invalid")))
    } else {
        Ok(())
    }
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(invalid(format!("{label} is invalid")))
    }
}

pub(super) fn require_exact(
    actual: &str,
    expected: &str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if actual == expected {
        Ok(())
    } else {
        Err(conflict(
            "upstream transport authority does not belong to the requested path",
        ))
    }
}

pub(super) fn idempotency_scope(operation: &str, actor: &UpstreamTransportTargetActor) -> String {
    format!(
        "v258:upstream-transport-target:{operation}:{}:{}",
        actor.kind(),
        actor.user_id()
    )
}

pub(super) fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> UpstreamTransportTargetServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            UpstreamTransportTargetServiceError::Conflict(AnyError::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            UpstreamTransportTargetServiceError::Storage(error)
        }
    }
}

pub(super) fn require_policy_inert(
    value: &ExternalPoolAdapterUpstreamTransportTargetPolicySummary,
) -> Result<(), UpstreamTransportTargetServiceError> {
    require_common_inert(
        &value.target_effect,
        [
            &value.adapter_effect,
            &value.runtime_effect,
            &value.provider_effect,
            &value.credential_effect,
            &value.route_effect,
            &value.execution_effect,
            &value.usage_effect,
            &value.market_effect,
            &value.settlement_effect,
        ],
        value.broker_connect_ready,
        value.upstream_probe_observed,
        value.runtime_launch_ready,
        value.activation_ready,
        "upstream transport policy changed inert effect",
    )
}

pub(super) fn require_target_inert(
    value: &ExternalPoolAdapterUpstreamTransportTargetSummary,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if value.target_status != UPSTREAM_TRANSPORT_TARGET_STATUS {
        return Err(conflict("upstream transport target changed status"));
    }
    require_common_inert(
        &value.target_effect,
        [
            &value.adapter_effect,
            &value.runtime_effect,
            &value.provider_effect,
            &value.credential_effect,
            &value.route_effect,
            &value.execution_effect,
            &value.usage_effect,
            &value.market_effect,
            &value.settlement_effect,
        ],
        value.broker_connect_ready,
        value.upstream_probe_observed,
        value.runtime_launch_ready,
        value.activation_ready,
        "upstream transport target changed inert effect",
    )
}

pub(super) fn require_revocation_inert(
    value: &ExternalPoolAdapterUpstreamTransportTargetRevocationSummary,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if value.revocation_effect != UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT {
        return Err(conflict(
            "upstream transport-target revocation changed effect",
        ));
    }
    require_none_effects_and_readiness(
        [
            &value.adapter_effect,
            &value.runtime_effect,
            &value.provider_effect,
            &value.credential_effect,
            &value.route_effect,
            &value.execution_effect,
            &value.usage_effect,
            &value.market_effect,
            &value.settlement_effect,
        ],
        value.broker_connect_ready,
        value.upstream_probe_observed,
        value.runtime_launch_ready,
        value.activation_ready,
        "upstream transport-target revocation changed inert effect",
    )
}

fn require_common_inert(
    target_effect: &str,
    effects: [&str; 9],
    broker_ready: bool,
    probe_observed: bool,
    launch_ready: bool,
    activation_ready: bool,
    message: &'static str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if target_effect != UPSTREAM_TRANSPORT_TARGET_EFFECT {
        return Err(conflict(message));
    }
    require_none_effects_and_readiness(
        effects,
        broker_ready,
        probe_observed,
        launch_ready,
        activation_ready,
        message,
    )
}

fn require_none_effects_and_readiness(
    effects: [&str; 9],
    broker_ready: bool,
    probe_observed: bool,
    launch_ready: bool,
    activation_ready: bool,
    message: &'static str,
) -> Result<(), UpstreamTransportTargetServiceError> {
    if effects
        .iter()
        .any(|effect| *effect != UPSTREAM_TRANSPORT_TARGET_NO_EFFECT)
        || broker_ready
        || probe_observed
        || launch_ready
        || activation_ready
    {
        Err(conflict(message))
    } else {
        Ok(())
    }
}

pub(super) fn invalid(message: impl Into<String>) -> UpstreamTransportTargetServiceError {
    UpstreamTransportTargetServiceError::Invalid(anyhow::anyhow!(message.into()))
}

pub(super) fn conflict(message: impl Into<String>) -> UpstreamTransportTargetServiceError {
    UpstreamTransportTargetServiceError::Conflict(anyhow::anyhow!(message.into()))
}
