//! Request and inert-output guards for the V259 policy-companion Service.

use anyhow::Error as AnyError;

use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
        SUPERVISOR_SESSION_COMPANION_EFFECT, SUPERVISOR_SESSION_COMPANION_NO_EFFECT,
        SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT, SUPERVISOR_SESSION_COMPANION_STATUS,
    },
    store::{
        ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness,
        ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary,
        ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary,
        ExternalPoolAdapterSupervisorSessionPolicySummary,
    },
};

use super::{
    external_pool_adapter_installation::ExternalPoolAdapterInstallationFsError,
    external_pool_adapter_supervisor_session_policy_companion_service::{
        CreateSupervisorSessionPolicyCompanionBody, RevokeSupervisorSessionPolicyCompanionBody,
        SupervisorSessionPolicyCompanionActor, SupervisorSessionPolicyCompanionServiceError,
    },
};

pub(super) fn validate_create(
    path: [&str; 4],
    body: &CreateSupervisorSessionPolicyCompanionBody,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    validate_path(path, None)?;
    if !body.confirm_supervisor_session_policy_companion {
        return Err(invalid(
            "supervisor/session policy companion requires explicit confirmation",
        ));
    }
    for (value, label) in [
        (&body.expected_target_digest, "target digest"),
        (&body.expected_profile_digest, "profile digest"),
        (&body.expected_candidate_digest, "candidate digest"),
        (
            &body.expected_provider_binding_digest,
            "Provider binding digest",
        ),
        (
            &body.expected_supervisor_session_policy_digest,
            "supervisor/session policy digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    validate_identifier(&body.idempotency_key, "idempotency key")?;
    if let Some(predecessor) = &body.expected_predecessor {
        validate_identifier(&predecessor.companion_id, "predecessor companion ID")?;
        validate_digest(
            &predecessor.companion_digest,
            "predecessor companion digest",
        )?;
    }
    Ok(())
}

pub(super) fn validate_revoke(
    path: [&str; 4],
    companion: &str,
    body: &RevokeSupervisorSessionPolicyCompanionBody,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    validate_path(path, Some(companion))?;
    if !body.confirm_revocation {
        return Err(invalid(
            "supervisor/session policy-companion revocation requires confirmation",
        ));
    }
    for (value, label) in [
        (&body.expected_companion_digest, "companion digest"),
        (&body.expected_target_digest, "target digest"),
        (&body.expected_profile_digest, "profile digest"),
    ] {
        validate_digest(value, label)?;
    }
    validate_identifier(&body.idempotency_key, "idempotency key")?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "supervisor/session policy-companion revocation reason is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_path(
    path: [&str; 4],
    companion: Option<&str>,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    for (value, label) in path.into_iter().zip([
        "Provider binding ID",
        "activation candidate ID",
        "runtime launch-profile ID",
        "upstream transport-target ID",
    ]) {
        validate_identifier(value, label)?;
    }
    if let Some(value) = companion {
        validate_identifier(value, "supervisor/session policy-companion ID")?;
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    label: &'static str,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
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
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(invalid(format!("{label} is invalid")))
    }
}

pub(super) fn require_exact(
    actual: &str,
    expected: &str,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if actual == expected {
        Ok(())
    } else {
        Err(conflict(
            "supervisor/session authority does not belong to the requested path",
        ))
    }
}

pub(super) fn idempotency_scope(
    operation: &str,
    actor: &SupervisorSessionPolicyCompanionActor,
) -> String {
    format!(
        "v259:supervisor-session-policy-companion:{operation}:{}:{}",
        actor.kind(),
        actor.user_id()
    )
}

pub(super) fn classify_filesystem_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> SupervisorSessionPolicyCompanionServiceError {
    match error {
        ExternalPoolAdapterInstallationFsError::Authority(_)
        | ExternalPoolAdapterInstallationFsError::InvalidContentAddress
        | ExternalPoolAdapterInstallationFsError::Package(_)
        | ExternalPoolAdapterInstallationFsError::Missing
        | ExternalPoolAdapterInstallationFsError::UnsafeTarget
        | ExternalPoolAdapterInstallationFsError::ContentDrift => {
            SupervisorSessionPolicyCompanionServiceError::Conflict(AnyError::new(error))
        }
        ExternalPoolAdapterInstallationFsError::Storage(_) => {
            SupervisorSessionPolicyCompanionServiceError::Storage(error)
        }
    }
}

pub(super) fn require_policy_inert(
    value: &ExternalPoolAdapterSupervisorSessionPolicySummary,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    require_inert(
        None,
        &value.companion_effect,
        policy_effects(value),
        policy_readiness(value),
        "supervisor/session policy changed inert contract",
    )
}

pub(super) fn require_companion_inert(
    value: &ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    require_inert(
        Some(&value.companion_status),
        &value.companion_effect,
        companion_effects(value),
        companion_readiness(value),
        "supervisor/session policy companion changed inert contract",
    )
}

pub(super) fn require_revocation_inert(
    value: &ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if value.revocation_effect != SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT {
        return Err(conflict(
            "supervisor/session policy-companion revocation changed effect",
        ));
    }
    if revocation_effects(value)
        .iter()
        .any(|effect| *effect != SUPERVISOR_SESSION_COMPANION_NO_EFFECT)
        || revocation_readiness(value).into_iter().any(|ready| ready)
    {
        Err(conflict(
            "supervisor/session policy-companion revocation changed inert contract",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_currentness_inert(
    value: &ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if currentness_effects(value)
        .iter()
        .any(|effect| *effect != SUPERVISOR_SESSION_COMPANION_NO_EFFECT)
        || currentness_readiness(value).into_iter().any(|ready| ready)
    {
        Err(conflict(
            "supervisor/session policy-companion currentness changed inert contract",
        ))
    } else {
        Ok(())
    }
}

fn require_inert(
    status: Option<&str>,
    effect: &str,
    effects: [&str; 9],
    readiness: [bool; 7],
    message: &'static str,
) -> Result<(), SupervisorSessionPolicyCompanionServiceError> {
    if status.is_some_and(|value| value != SUPERVISOR_SESSION_COMPANION_STATUS)
        || effect != SUPERVISOR_SESSION_COMPANION_EFFECT
        || effects
            .iter()
            .any(|value| *value != SUPERVISOR_SESSION_COMPANION_NO_EFFECT)
        || readiness.into_iter().any(|ready| ready)
    {
        Err(conflict(message))
    } else {
        Ok(())
    }
}

macro_rules! inert_accessors {
    ($effects:ident, $readiness:ident, $type:ty) => {
        fn $effects(value: &$type) -> [&str; 9] {
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
            ]
        }
        fn $readiness(value: &$type) -> [bool; 7] {
            [
                value.process_spawn_ready,
                value.ipc_session_ready,
                value.secret_delivery_ready,
                value.broker_connect_ready,
                value.upstream_probe_observed,
                value.runtime_launch_ready,
                value.activation_ready,
            ]
        }
    };
}
inert_accessors!(
    policy_effects,
    policy_readiness,
    ExternalPoolAdapterSupervisorSessionPolicySummary
);
inert_accessors!(
    companion_effects,
    companion_readiness,
    ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary
);
inert_accessors!(
    revocation_effects,
    revocation_readiness,
    ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary
);
inert_accessors!(
    currentness_effects,
    currentness_readiness,
    ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness
);

pub(super) fn invalid(message: impl Into<String>) -> SupervisorSessionPolicyCompanionServiceError {
    SupervisorSessionPolicyCompanionServiceError::Invalid(anyhow::anyhow!(message.into()))
}
pub(super) fn conflict(message: impl Into<String>) -> SupervisorSessionPolicyCompanionServiceError {
    SupervisorSessionPolicyCompanionServiceError::Conflict(anyhow::anyhow!(message.into()))
}
