use serde::{Deserialize, Serialize};

use super::session::{
    InteractiveDesktopControlEpoch, InteractiveDesktopHostLease, InteractiveDesktopMediaEpoch,
    InteractiveDesktopViewerGrant,
};

/// Exact current authority objects selected by one Session revision. IDs or generations alone are
/// insufficient because an object with the same shallow identity but different content must fail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopAuthorityHead {
    pub host_lease_id: String,
    pub host_lease_digest: String,
    pub viewer_grant_id: String,
    pub viewer_grant_digest: String,
    pub viewer_grant_generation: u64,
    pub media_epoch_id: String,
    pub media_epoch_digest: String,
    pub media_epoch_sequence: u64,
    pub control_epoch_id: String,
    pub control_epoch_digest: String,
    pub control_epoch_sequence: u64,
    pub selected_surface_digest: String,
    pub viewer_transport_identity_digest: String,
    pub fencing_generation: u64,
}

impl InteractiveDesktopAuthorityHead {
    pub(crate) fn has_complete_reference(&self) -> bool {
        !self.host_lease_id.is_empty()
            && !self.host_lease_digest.is_empty()
            && !self.viewer_grant_id.is_empty()
            && !self.viewer_grant_digest.is_empty()
            && self.viewer_grant_generation > 0
            && !self.media_epoch_id.is_empty()
            && !self.media_epoch_digest.is_empty()
            && self.media_epoch_sequence > 0
            && !self.control_epoch_id.is_empty()
            && !self.control_epoch_digest.is_empty()
            && self.control_epoch_sequence > 0
            && !self.selected_surface_digest.is_empty()
            && !self.viewer_transport_identity_digest.is_empty()
            && self.fencing_generation > 0
    }

    pub(crate) fn matches(
        &self,
        lease: &InteractiveDesktopHostLease,
        grant: &InteractiveDesktopViewerGrant,
        media: &InteractiveDesktopMediaEpoch,
        control: &InteractiveDesktopControlEpoch,
    ) -> bool {
        self.has_complete_reference()
            && self.host_lease_id == lease.host_lease_id
            && self.host_lease_digest == lease.host_lease_digest
            && self.viewer_grant_id == grant.viewer_grant_id
            && self.viewer_grant_digest == grant.viewer_grant_digest
            && self.viewer_grant_generation == grant.grant_generation
            && self.media_epoch_id == media.media_epoch_id
            && self.media_epoch_digest == media.media_epoch_digest
            && self.media_epoch_sequence == media.epoch_sequence
            && self.control_epoch_id == control.control_epoch_id
            && self.control_epoch_digest == control.control_epoch_digest
            && self.control_epoch_sequence == control.epoch_sequence
            && self.selected_surface_digest == lease.selected_surface.selection_digest
            && self.selected_surface_digest == media.selected_surface_digest
            && self.selected_surface_digest == control.selected_surface_digest
            && self.viewer_transport_identity_digest == grant.viewer_transport_identity_digest
            && self.viewer_transport_identity_digest == media.viewer_transport_identity_digest
            && self.viewer_transport_identity_digest == control.viewer_transport_identity_digest
            && self.fencing_generation == lease.fencing_generation
            && self.fencing_generation == media.fencing_generation
            && self.fencing_generation == control.fencing_generation
            && control.media_epoch_id == media.media_epoch_id
            && control.media_epoch_digest == media.media_epoch_digest
            && control.media_epoch_sequence == media.epoch_sequence
    }
}
