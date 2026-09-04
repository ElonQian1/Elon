use anyhow::Result;
use sha2::{Digest, Sha256};

use super::{
    model::*,
    validation::{digest, label, positive_units},
};
use crate::esk_asset::platform::payment_identity::fingerprint;

pub(crate) fn validate_policy(mut body: SellbackPolicyBody) -> Result<SellbackPolicy> {
    if body.schema != POLICY_SCHEMA
        || !label(&body.revision, 80)
        || !digest(&body.approval_digest)
        || !digest(&body.source_fingerprint)
        || body.eligible_user_ids.is_empty()
        || body.eligible_user_ids.len() > 1000
        || body
            .eligible_user_ids
            .iter()
            .any(|id| !label(id, 96) || id == "local-owner")
        || body.hold_mode != "on_submit"
        || body.cancel_mode != "owner_cancel_until_settlement"
        || body.expiry_mode != "none"
        || body.participation_effect != "not_modified_by_this_feature"
        || !bounded_text(&body.terms_text, 2048)
        || !bounded_text(&body.disabled_account_recovery_text, 1024)
        || !digest(&body.terms_digest)
        || text_digest(&body.terms_text) != body.terms_digest
    {
        return Err(SellbackError::InvalidInput.into());
    }
    body.eligible_user_ids.sort_unstable();
    if body
        .eligible_user_ids
        .windows(2)
        .any(|pair| pair[0] == pair[1])
    {
        return Err(SellbackError::InvalidInput.into());
    }
    let minimum = positive_units(&body.min_request_base_units)?;
    let maximum = positive_units(&body.max_request_base_units)?;
    let per_user = positive_units(&body.max_reserved_base_units_per_user)?;
    let global = positive_units(&body.max_reserved_base_units_global)?;
    positive_units(&body.max_open_requests_per_user)?;
    if minimum > maximum || maximum > per_user || per_user > global {
        return Err(SellbackError::InvalidInput.into());
    }
    let value = serde_json::to_value(&body).map_err(|_| SellbackError::InvalidInput)?;
    let policy_digest = fingerprint(&value).map_err(|_| SellbackError::InvalidInput)?;
    Ok(SellbackPolicy {
        body,
        policy_digest,
    })
}

pub(crate) fn validate_policy_integrity(policy: &SellbackPolicy) -> Result<()> {
    let actual = validate_policy(policy.body.clone()).map_err(|_| SellbackError::Corrupt)?;
    if actual != *policy {
        return Err(SellbackError::Corrupt.into());
    }
    Ok(())
}

pub(crate) fn text_digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && value
            .chars()
            .all(|ch| !ch.is_control() || matches!(ch, '\n' | '\r' | '\t'))
}

pub(crate) fn configuration_from_values(
    mode: Option<&str>,
    raw: Option<&str>,
) -> SellbackConfiguration {
    match mode {
        None | Some("disabled") => SellbackConfiguration::Disabled,
        Some("approved_requests") => {
            let parsed = raw
                .filter(|value| value.len() <= 65536)
                .and_then(|value| serde_json::from_str::<SellbackPolicyBody>(value).ok())
                .and_then(|value| validate_policy(value).ok());
            match parsed {
                Some(policy) => SellbackConfiguration::Enabled(policy),
                None => SellbackConfiguration::Invalid,
            }
        }
        _ => SellbackConfiguration::Invalid,
    }
}

pub(crate) fn load_configuration() -> SellbackConfiguration {
    let read = |name| match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(()),
    };
    match (
        read("ESK_PLATFORM_SELLBACK_MODE"),
        read("ESK_PLATFORM_SELLBACK_POLICY"),
    ) {
        (Ok(mode), Ok(policy)) => configuration_from_values(mode.as_deref(), policy.as_deref()),
        _ => SellbackConfiguration::Invalid,
    }
}

/// Public status depends on the current source and user, never on a client-supplied identity.
pub(crate) fn availability(
    config: &SellbackConfiguration,
    user_id: &str,
    source: Option<&str>,
) -> SellbackAvailability {
    let reason = match config {
        SellbackConfiguration::Disabled => "disabled",
        SellbackConfiguration::Invalid => "configuration_invalid",
        SellbackConfiguration::Enabled(policy) => {
            if validate_policy_integrity(policy).is_err() {
                "configuration_invalid"
            } else if source != Some(policy.body.source_fingerprint.as_str()) {
                "source_mismatch"
            } else if !policy.body.eligible_user_ids.iter().any(|id| id == user_id) {
                "user_not_eligible"
            } else {
                return SellbackAvailability {
                    new_requests_enabled: true,
                    reason: "enabled".into(),
                    policy: Some(policy.clone()),
                };
            }
        }
    };
    SellbackAvailability {
        new_requests_enabled: false,
        reason: reason.into(),
        policy: None,
    }
}
