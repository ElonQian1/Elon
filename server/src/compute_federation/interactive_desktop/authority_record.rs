use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::{
    canonical::{
        canonical_interactive_desktop_authority_head_json_and_digest,
        canonical_interactive_desktop_authority_record_json_and_digest,
        canonical_interactive_desktop_control_epoch_json_and_digest,
        canonical_interactive_desktop_host_consent_json_and_digest,
        canonical_interactive_desktop_host_lease_json_and_digest,
        canonical_interactive_desktop_media_epoch_json_and_digest,
        canonical_interactive_desktop_offer_profile_json_and_digest,
        canonical_interactive_desktop_product_authority_json_and_digest,
        canonical_interactive_desktop_relay_authority_json_and_digest,
        canonical_interactive_desktop_session_json_and_digest,
        canonical_interactive_desktop_session_request_json_and_digest,
        canonical_interactive_desktop_session_reservation_json_and_digest,
        canonical_interactive_desktop_viewer_grant_json_and_digest,
    },
    offer::{InteractiveDesktopOfferProfile, InteractiveDesktopProductMode},
    product_authority::InteractiveDesktopProductAuthorityProof,
    reservation::InteractiveDesktopSessionReservation,
    session::{
        InteractiveDesktopAction, InteractiveDesktopControlEpoch, InteractiveDesktopHostLease,
        InteractiveDesktopMediaEpoch, InteractiveDesktopSession, InteractiveDesktopSessionRequest,
        InteractiveDesktopViewerGrant, InteractiveDesktopViewerRelationship,
    },
};

pub(crate) const INTERACTIVE_DESKTOP_AUTHORITY_RECORD_SCHEMA: &str =
    "compute_federation.interactive_desktop.authority_record.v1";

/// Canonical authority snapshot for one interactive Session revision.
///
/// External Provider, account-session, endpoint-credential and ownership records are intentionally
/// referenced rather than copied here; a Store-local resolver must verify those authorities in the
/// same transaction before this record can become current.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopAuthorityRecord {
    pub schema: String,
    pub record_digest: String,
    pub request: InteractiveDesktopSessionRequest,
    pub profile: InteractiveDesktopOfferProfile,
    pub reservation: InteractiveDesktopSessionReservation,
    pub session: InteractiveDesktopSession,
    pub host_lease: InteractiveDesktopHostLease,
    pub viewer_grant: InteractiveDesktopViewerGrant,
    pub media_epoch: InteractiveDesktopMediaEpoch,
    pub control_epoch: InteractiveDesktopControlEpoch,
}

impl InteractiveDesktopAuthorityRecord {
    /// Recomputes every digest owned by this record before applying cross-object checks.
    /// Referenced external authority digests still require a transaction-scoped Store resolver.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn verify_canonical_and_structure(
        &self,
        viewer_account_id: &str,
        viewer_account_session_digest: &str,
        viewer_account_auth_epoch: u64,
        viewer_device_key_digest: &str,
        viewer_transport_identity_digest: &str,
        action: InteractiveDesktopAction,
        now_ms: i64,
    ) -> Result<()> {
        self.verify_canonical_digests()?;
        self.verify_same_owner_only()?;
        self.verify_cross_object_structure(
            viewer_account_id,
            viewer_account_session_digest,
            viewer_account_auth_epoch,
            viewer_device_key_digest,
            viewer_transport_identity_digest,
            action,
            now_ms,
        )?;
        Ok(())
    }

    pub(crate) fn canonical_json_and_digest(&self) -> Result<(String, String)> {
        canonical_interactive_desktop_authority_record_json_and_digest(self)
    }

    pub(crate) fn authority_head_json_and_digest(&self) -> Result<(String, String)> {
        canonical_interactive_desktop_authority_head_json_and_digest(&self.session.authority_head)
    }

    pub(crate) fn verify_canonical_digests(&self) -> Result<()> {
        if self.schema != INTERACTIVE_DESKTOP_AUTHORITY_RECORD_SCHEMA {
            bail!("interactive desktop authority record schema is unsupported");
        }
        verify_digest(
            "session request",
            &self.request.request_digest,
            canonical_interactive_desktop_session_request_json_and_digest(&self.request)?.1,
        )?;
        verify_digest(
            "offer profile",
            &self.profile.profile_digest,
            canonical_interactive_desktop_offer_profile_json_and_digest(&self.profile)?.1,
        )?;
        verify_digest(
            "product authority",
            &self.reservation.product_authority.authority_digest,
            canonical_interactive_desktop_product_authority_json_and_digest(
                &self.reservation.product_authority,
            )?
            .1,
        )?;
        verify_digest(
            "session reservation",
            &self
                .reservation
                .session_reservation
                .session_reservation_digest,
            canonical_interactive_desktop_session_reservation_json_and_digest(&self.reservation)?.1,
        )?;
        verify_digest(
            "session",
            &self.session.session_digest,
            canonical_interactive_desktop_session_json_and_digest(&self.session)?.1,
        )?;
        verify_digest(
            "host consent",
            &self.host_lease.host_consent.consent_digest,
            canonical_interactive_desktop_host_consent_json_and_digest(
                &self.host_lease.host_consent,
            )?
            .1,
        )?;
        verify_digest(
            "host lease",
            &self.host_lease.host_lease_digest,
            canonical_interactive_desktop_host_lease_json_and_digest(&self.host_lease)?.1,
        )?;
        verify_digest(
            "viewer grant",
            &self.viewer_grant.viewer_grant_digest,
            canonical_interactive_desktop_viewer_grant_json_and_digest(&self.viewer_grant)?.1,
        )?;
        if let Some(authority) = &self.media_epoch.relay_authority {
            verify_digest(
                "relay authority",
                &authority.relay_authority_digest,
                canonical_interactive_desktop_relay_authority_json_and_digest(authority)?.1,
            )?;
        }
        verify_digest(
            "media epoch",
            &self.media_epoch.media_epoch_digest,
            canonical_interactive_desktop_media_epoch_json_and_digest(&self.media_epoch)?.1,
        )?;
        verify_digest(
            "control epoch",
            &self.control_epoch.control_epoch_digest,
            canonical_interactive_desktop_control_epoch_json_and_digest(&self.control_epoch)?.1,
        )?;

        // The authority head has no self-carried digest field. Computing it here still enforces
        // I-JSON bounds; the enclosing canonical record digest commits the exact head content.
        let (_, head_digest) = self.authority_head_json_and_digest()?;
        if head_digest.is_empty() {
            bail!("interactive desktop authority head digest is empty");
        }
        verify_digest(
            "authority record",
            &self.record_digest,
            self.canonical_json_and_digest()?.1,
        )
    }

    fn verify_same_owner_only(&self) -> Result<()> {
        for mode in [
            self.request.requested_product_mode,
            self.profile.offer.product_mode,
            self.reservation.binding.offer.product_mode,
            self.reservation.product_authority.product_mode,
        ] {
            match mode {
                InteractiveDesktopProductMode::SameOwnerRemoteAccess => {}
                InteractiveDesktopProductMode::FriendCoPlay => {
                    bail!("FriendCoPlay authority is not implemented and fails closed")
                }
                InteractiveDesktopProductMode::LicensedCloudSeat => {
                    bail!("LicensedCloudSeat authority is not implemented and fails closed")
                }
            }
        }
        for relationship in [
            self.request.viewer_relationship,
            self.reservation.product_authority.viewer_relationship,
            self.session.viewer_relationship,
        ] {
            match relationship {
                InteractiveDesktopViewerRelationship::SameOwner => {}
                InteractiveDesktopViewerRelationship::TrustedFriend => {
                    bail!("FriendCoPlay relationship authority is unavailable")
                }
                InteractiveDesktopViewerRelationship::MarketplaceStranger => {
                    bail!("LicensedCloudSeat entitlement authority is unavailable")
                }
            }
        }
        match &self.reservation.product_authority.proof {
            InteractiveDesktopProductAuthorityProof::SameOwnerAccount { .. } => Ok(()),
            InteractiveDesktopProductAuthorityProof::HostInvitation { .. } => {
                bail!("FriendCoPlay invitation authority is unavailable")
            }
            InteractiveDesktopProductAuthorityProof::MarketplaceEntitlement { .. } => {
                bail!("LicensedCloudSeat entitlement authority is unavailable")
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn verify_cross_object_structure(
        &self,
        viewer_account_id: &str,
        viewer_account_session_digest: &str,
        viewer_account_auth_epoch: u64,
        viewer_device_key_digest: &str,
        viewer_transport_identity_digest: &str,
        action: InteractiveDesktopAction,
        now_ms: i64,
    ) -> Result<()> {
        if !self.request.has_safe_request_shape() {
            bail!("interactive desktop session request has an unsafe shape");
        }
        if !self.profile.has_v1_shape() {
            bail!("interactive desktop offer profile has an unsafe shape");
        }
        if !self
            .reservation
            .cross_validates_request(&self.request, &self.profile, now_ms)
        {
            bail!("interactive desktop reservation does not bind request and profile");
        }
        if !self.session.structurally_authorizes(
            &self.profile,
            &self.reservation,
            &self.host_lease,
            &self.viewer_grant,
            &self.media_epoch,
            &self.control_epoch,
            viewer_account_id,
            viewer_account_session_digest,
            viewer_account_auth_epoch,
            viewer_device_key_digest,
            viewer_transport_identity_digest,
            action,
            now_ms,
        ) {
            bail!("interactive desktop authority objects do not form one current authority set");
        }
        Ok(())
    }
}

fn verify_digest(label: &str, claimed: &str, computed: String) -> Result<()> {
    if claimed.is_empty() || claimed != computed {
        bail!("interactive desktop {label} canonical digest mismatch");
    }
    Ok(())
}
