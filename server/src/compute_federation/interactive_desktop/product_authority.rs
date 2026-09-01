use serde::{Deserialize, Serialize};

use super::{
    offer::{InteractiveDesktopProductMode, InteractiveDesktopTitlePolicyBinding},
    session::InteractiveDesktopViewerRelationship,
    INTERACTIVE_DESKTOP_SERVICE_CLASS,
};

pub(crate) const INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_SCHEMA: &str =
    "compute_federation.interactive_desktop.product_authority.v1";
pub(crate) const INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-PRODUCT-AUTHORITY-V1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopProductAuthorityCurrentness {
    Current,
    Superseded,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "proof_kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum InteractiveDesktopProductAuthorityProof {
    SameOwnerAccount {
        ownership_snapshot_id: String,
        ownership_snapshot_digest: String,
        account_id: String,
    },
    HostInvitation {
        invitation_id: String,
        invitation_revision: u64,
        invitation_digest: String,
        inviter_account_id: String,
        invitee_account_id: String,
    },
    MarketplaceEntitlement {
        entitlement_id: String,
        entitlement_revision: u64,
        entitlement_digest: String,
        consumer_account_id: String,
        title_policy_snapshot_digest: String,
    },
}

/// Bounded relationship proof. Runtime callers must resolve and verify the referenced authority;
/// a structurally valid object is not, by itself, proof that an account or invitation is current.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopProductAuthorityBinding {
    pub schema: String,
    pub service_class: String,
    pub authority_id: String,
    pub authority_revision: u64,
    pub authority_digest: String,
    pub issuer_id: String,
    pub issuer_policy_digest: String,
    pub session_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub consumer_account_id: String,
    pub product_mode: InteractiveDesktopProductMode,
    pub viewer_relationship: InteractiveDesktopViewerRelationship,
    pub currentness: InteractiveDesktopProductAuthorityCurrentness,
    pub proof: InteractiveDesktopProductAuthorityProof,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
}

impl InteractiveDesktopProductAuthorityBinding {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn structurally_authorizes(
        &self,
        session_id: &str,
        provider_id: &str,
        consumer_account_id: &str,
        product_mode: InteractiveDesktopProductMode,
        viewer_relationship: InteractiveDesktopViewerRelationship,
        title_policy: Option<&InteractiveDesktopTitlePolicyBinding>,
        now_ms: i64,
    ) -> bool {
        self.schema == INTERACTIVE_DESKTOP_PRODUCT_AUTHORITY_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && !self.authority_id.is_empty()
            && self.authority_revision > 0
            && !self.authority_digest.is_empty()
            && !self.issuer_id.is_empty()
            && !self.issuer_policy_digest.is_empty()
            && !self.session_id.is_empty()
            && self.session_id == session_id
            && !self.provider_owner_account_id.is_empty()
            && self.provider_id == provider_id
            && self.consumer_account_id == consumer_account_id
            && self.product_mode == product_mode
            && self.viewer_relationship == viewer_relationship
            && self.currentness == InteractiveDesktopProductAuthorityCurrentness::Current
            && self.issued_at_ms <= now_ms
            && now_ms < self.expires_at_ms
            && match (&self.proof, product_mode, viewer_relationship) {
                (
                    InteractiveDesktopProductAuthorityProof::SameOwnerAccount {
                        ownership_snapshot_id,
                        ownership_snapshot_digest,
                        account_id,
                    },
                    InteractiveDesktopProductMode::SameOwnerRemoteAccess,
                    InteractiveDesktopViewerRelationship::SameOwner,
                ) => {
                    !ownership_snapshot_id.is_empty()
                        && !ownership_snapshot_digest.is_empty()
                        && account_id == &self.provider_owner_account_id
                        && account_id == consumer_account_id
                }
                (
                    InteractiveDesktopProductAuthorityProof::HostInvitation {
                        invitation_id,
                        invitation_revision,
                        invitation_digest,
                        inviter_account_id,
                        invitee_account_id,
                    },
                    InteractiveDesktopProductMode::FriendCoPlay,
                    InteractiveDesktopViewerRelationship::TrustedFriend,
                ) => {
                    !invitation_id.is_empty()
                        && *invitation_revision > 0
                        && !invitation_digest.is_empty()
                        && inviter_account_id == &self.provider_owner_account_id
                        && invitee_account_id == consumer_account_id
                        && self.provider_owner_account_id.as_str() != consumer_account_id
                }
                (
                    InteractiveDesktopProductAuthorityProof::MarketplaceEntitlement {
                        entitlement_id,
                        entitlement_revision,
                        entitlement_digest,
                        consumer_account_id: entitled_consumer,
                        title_policy_snapshot_digest,
                    },
                    InteractiveDesktopProductMode::LicensedCloudSeat,
                    InteractiveDesktopViewerRelationship::MarketplaceStranger,
                ) => {
                    !entitlement_id.is_empty()
                        && *entitlement_revision > 0
                        && !entitlement_digest.is_empty()
                        && entitled_consumer == consumer_account_id
                        && self.provider_owner_account_id.as_str() != consumer_account_id
                        && title_policy.is_some_and(|policy| {
                            !title_policy_snapshot_digest.is_empty()
                                && title_policy_snapshot_digest
                                    == &policy.title_policy_snapshot_digest
                                && now_ms < policy.valid_until_ms
                        })
                }
                _ => false,
            }
    }
}
