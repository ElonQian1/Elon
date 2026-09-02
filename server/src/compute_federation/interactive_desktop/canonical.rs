use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    authority::{
        InteractiveDesktopHostConsentBinding, InteractiveDesktopRelayAuthorityBinding,
        INTERACTIVE_DESKTOP_HOST_CONSENT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_DIGEST_DOMAIN,
    },
    authority_head::InteractiveDesktopAuthorityHead,
    authority_record::InteractiveDesktopAuthorityRecord,
    offer::{InteractiveDesktopOfferProfile, INTERACTIVE_DESKTOP_OFFER_PROFILE_DIGEST_DOMAIN},
    product_authority::{
        InteractiveDesktopProductAuthorityBinding,
        INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_DIGEST_DOMAIN,
    },
    reservation::{
        InteractiveDesktopSessionReservation, INTERACTIVE_DESKTOP_SESSION_RESERVATION_DIGEST_DOMAIN,
    },
    session::{
        InteractiveDesktopControlEpoch, InteractiveDesktopHostLease, InteractiveDesktopMediaEpoch,
        InteractiveDesktopSession, InteractiveDesktopSessionRequest, InteractiveDesktopViewerGrant,
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_HOST_LEASE_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_DIGEST_DOMAIN, INTERACTIVE_DESKTOP_SESSION_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_VIEWER_GRANT_DIGEST_DOMAIN,
    },
};

const MAX_INTERACTIVE_DESKTOP_JSON_BYTES: usize = 512 * 1024;
pub(crate) const INTERACTIVE_DESKTOP_AUTHORITY_HEAD_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-AUTHORITY-HEAD-V1";
pub(crate) const INTERACTIVE_DESKTOP_AUTHORITY_RECORD_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-AUTHORITY-RECORD-V1";

pub(crate) fn canonical_interactive_desktop_session_request_json_and_digest(
    request: &InteractiveDesktopSessionRequest,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        request,
        &[&["request_digest"]],
        INTERACTIVE_DESKTOP_SESSION_REQUEST_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_offer_profile_json_and_digest(
    profile: &InteractiveDesktopOfferProfile,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        profile,
        &[&["profile_digest"], &["offer", "profile_digest"]],
        INTERACTIVE_DESKTOP_OFFER_PROFILE_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_product_authority_json_and_digest(
    authority: &InteractiveDesktopProductAuthorityBinding,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        authority,
        &[&["authority_digest"]],
        INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_session_reservation_json_and_digest(
    reservation: &InteractiveDesktopSessionReservation,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        reservation,
        &[&["session_reservation", "session_reservation_digest"]],
        INTERACTIVE_DESKTOP_SESSION_RESERVATION_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_session_json_and_digest(
    session: &InteractiveDesktopSession,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        session,
        &[&["session_digest"]],
        INTERACTIVE_DESKTOP_SESSION_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_host_lease_json_and_digest(
    lease: &InteractiveDesktopHostLease,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        lease,
        &[&["host_lease_digest"]],
        INTERACTIVE_DESKTOP_HOST_LEASE_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_viewer_grant_json_and_digest(
    grant: &InteractiveDesktopViewerGrant,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        grant,
        &[&["viewer_grant_digest"]],
        INTERACTIVE_DESKTOP_VIEWER_GRANT_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_media_epoch_json_and_digest(
    epoch: &InteractiveDesktopMediaEpoch,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        epoch,
        &[&["media_epoch_digest"]],
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_control_epoch_json_and_digest(
    epoch: &InteractiveDesktopControlEpoch,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        epoch,
        &[&["control_epoch_digest"]],
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_host_consent_json_and_digest(
    consent: &InteractiveDesktopHostConsentBinding,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        consent,
        &[&["consent_digest"]],
        INTERACTIVE_DESKTOP_HOST_CONSENT_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_relay_authority_json_and_digest(
    authority: &InteractiveDesktopRelayAuthorityBinding,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        authority,
        &[&["relay_authority_digest"]],
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_authority_head_json_and_digest(
    head: &InteractiveDesktopAuthorityHead,
) -> Result<(String, String)> {
    canonical_material_json_and_digest(
        head,
        INTERACTIVE_DESKTOP_AUTHORITY_HEAD_DIGEST_DOMAIN.as_bytes(),
    )
}

pub(crate) fn canonical_interactive_desktop_authority_record_json_and_digest(
    record: &InteractiveDesktopAuthorityRecord,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(
        record,
        &[&["record_digest"]],
        INTERACTIVE_DESKTOP_AUTHORITY_RECORD_DIGEST_DOMAIN.as_bytes(),
    )
}

fn canonical_envelope_json_and_digest<T: Serialize>(
    envelope: &T,
    digest_paths: &[&[&str]],
    domain: &[u8],
) -> Result<(String, String)> {
    let mut projection = serde_json::to_value(envelope)?;
    for path in digest_paths {
        blank_string_field(&mut projection, path)?;
    }
    Ok((
        canonical_json(envelope)?,
        domain_digest(domain, &projection)?,
    ))
}

fn canonical_material_json_and_digest<T: Serialize>(
    material: &T,
    domain: &[u8],
) -> Result<(String, String)> {
    Ok((canonical_json(material)?, domain_digest(domain, material)?))
}

fn blank_string_field(value: &mut Value, path: &[&str]) -> Result<()> {
    let (field, parents) = path
        .split_last()
        .ok_or_else(|| anyhow!("interactive desktop digest path must not be empty"))?;
    let mut target = value;
    for parent in parents {
        target = target
            .as_object_mut()
            .and_then(|object| object.get_mut(*parent))
            .ok_or_else(|| anyhow!("interactive desktop digest projection lacks {parent}"))?;
    }
    let digest = target
        .as_object_mut()
        .and_then(|object| object.get_mut(*field))
        .ok_or_else(|| anyhow!("interactive desktop digest projection lacks {field}"))?;
    match digest {
        Value::String(value) => value.clear(),
        _ => bail!("interactive desktop digest projection {field} is not a string"),
    }
    Ok(())
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_INTERACTIVE_DESKTOP_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
