use std::{fmt, io};

use super::super::ManagedObjectBinding;
use super::{
    child_name_utf16, invalid_data, is_zero, MinifilterDigest, MinifilterFenceAuthorityBinding,
    MinifilterFenceFilesystemKind, MinifilterFenceLeaseState, MinifilterFenceNameMatchMode,
    MinifilterId, MINIFILTER_FENCE_MAX_MESSAGE_BYTES,
};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct MinifilterFenceSessionReceipt {
    pub(super) connection_id: MinifilterId,
    pub(super) driver_boot_id: MinifilterId,
    pub(super) driver_build_digest: MinifilterDigest,
    pub(super) wire_schema_digest: MinifilterDigest,
    pub(super) driver_session_generation: u64,
    pub(super) capability_bits: u64,
    pub(super) maximum_message_bytes: u32,
    pub(super) pointer_width_bits: u16,
}

impl MinifilterFenceSessionReceipt {
    pub(super) fn validate(
        &self,
        expected_driver_build_digest: MinifilterDigest,
        expected_wire_schema_digest: MinifilterDigest,
        required_feature_mask: u64,
    ) -> io::Result<()> {
        if is_zero(&self.connection_id)
            || is_zero(&self.driver_boot_id)
            || is_zero(&self.driver_build_digest)
            || is_zero(&self.wire_schema_digest)
            || self.driver_build_digest != expected_driver_build_digest
            || self.wire_schema_digest != expected_wire_schema_digest
            || self.driver_session_generation == 0
            || self.capability_bits & required_feature_mask != required_feature_mask
            || self.maximum_message_bytes < MINIFILTER_FENCE_MAX_MESSAGE_BYTES as u32
            || self.pointer_width_bits != 64
        {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_SESSION_CHANGED"));
        }
        Ok(())
    }
}

impl fmt::Debug for MinifilterFenceSessionReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinifilterFenceSessionReceipt")
            .field("connection", &"<redacted>")
            .field("driver_boot", &"<redacted>")
            .field("driver_build", &"<digest-bound>")
            .field("wire_schema", &"<digest-bound>")
            .field("driver_session_generation", &self.driver_session_generation)
            .field("capability_bits", &self.capability_bits)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct MinifilterFenceScopeReceipt {
    pub(super) volume_instance_id: MinifilterId,
    pub(super) volume_instance_generation: u64,
    pub(super) volume_serial: u64,
    pub(super) parent_file_id: MinifilterId,
    pub(super) filesystem_kind: MinifilterFenceFilesystemKind,
    pub(super) name_match_mode: MinifilterFenceNameMatchMode,
}

/// The secret-bearing grant receipt is deliberately non-Clone. Query and Release requests copy
/// only the exact key fields they need while the owning lease remains linear.
#[derive(PartialEq, Eq)]
pub(in crate::node_agent_managed_fs::namespace) struct MinifilterFenceGrantReceipt {
    pub(super) session: MinifilterFenceSessionReceipt,
    pub(super) scope: MinifilterFenceScopeReceipt,
    pub(super) authority: MinifilterFenceAuthorityBinding,
    pub(super) acquire_nonce: MinifilterId,
    pub(super) fence_id: MinifilterId,
    pub(super) grant_secret: MinifilterDigest,
    pub(super) grant_generation: u64,
    pub(super) grant_sequence: u64,
    pub(super) state_generation: u64,
    pub(super) query_sequence: u64,
    pub(super) blocked_mutation_count: u64,
    pub(super) release_sequence: u64,
    pub(super) state: MinifilterFenceLeaseState,
    pub(super) poison_reason: u16,
    pub(super) release_nonce: Option<MinifilterId>,
    pub(super) requested_name_utf16: Vec<u16>,
}

impl MinifilterFenceGrantReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_managed_fs::namespace) fn validate_managed_binding(
        &self,
        binding: &ManagedObjectBinding,
        cleanup_id: &str,
        execution_plan_digest: &str,
        authorization_receipt_digest: &str,
        expected_object_digest: &str,
        installation_id_digest: &str,
        authority_epoch: i64,
        process_owner_epoch: i64,
        step_ordinal: u64,
        expected_driver_build_digest: MinifilterDigest,
        expected_wire_schema_digest: MinifilterDigest,
        required_feature_mask: u64,
    ) -> io::Result<()> {
        let expected_authority = MinifilterFenceAuthorityBinding::new(
            installation_id_digest,
            cleanup_id,
            execution_plan_digest,
            authorization_receipt_digest,
            expected_object_digest,
            binding.identity_digest(),
            binding.parent_identity_digest(),
            binding.relative_name(),
            authority_epoch,
            process_owner_epoch,
            step_ordinal,
        )?;
        let expected_name = child_name_utf16(binding.relative_name())?;
        self.session.validate(
            expected_driver_build_digest,
            expected_wire_schema_digest,
            required_feature_mask,
        )?;
        self.validate_active(
            &self.session,
            &expected_authority,
            self.acquire_nonce,
            self.scope.volume_serial,
            self.scope.parent_file_id,
            &expected_name,
        )
    }

    pub(super) fn validate_active(
        &self,
        expected_session: &MinifilterFenceSessionReceipt,
        expected_authority: &MinifilterFenceAuthorityBinding,
        expected_acquire_nonce: MinifilterId,
        expected_volume_serial: u64,
        expected_parent_file_id: MinifilterId,
        expected_name_utf16: &[u16],
    ) -> io::Result<()> {
        if &self.session != expected_session
            || &self.authority != expected_authority
            || self.acquire_nonce != expected_acquire_nonce
            || self.scope.volume_serial != expected_volume_serial
            || self.scope.parent_file_id != expected_parent_file_id
            || is_zero(&self.scope.volume_instance_id)
            || self.scope.volume_instance_generation == 0
            || is_zero(&self.scope.parent_file_id)
            || is_zero(&self.fence_id)
            || is_zero(&self.grant_secret)
            || self.grant_generation == 0
            || self.grant_sequence == 0
            || self.state_generation == 0
            || self.query_sequence != 0
            || self.release_sequence != 0
            || self.state != MinifilterFenceLeaseState::Active
            || self.poison_reason != 0
            || self.release_nonce.is_some()
            || self.requested_name_utf16 != expected_name_utf16
        {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_GRANT_CHANGED"));
        }
        Ok(())
    }

    pub(super) fn validate_exact_query(&self, original: &Self) -> io::Result<()> {
        if !same_immutable_grant(self, original)
            || self.state != MinifilterFenceLeaseState::Active
            || self.poison_reason != 0
            || self.release_nonce.is_some()
            || self.release_sequence != 0
            || self.state_generation != original.state_generation
            || self.query_sequence == 0
            || self.query_sequence < original.query_sequence
            || self.blocked_mutation_count < original.blocked_mutation_count
        {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_QUERY_NOT_EXACT_ACTIVE"));
        }
        Ok(())
    }

    pub(super) fn validate_exact_release(
        &self,
        active: &Self,
        expected_release_nonce: MinifilterId,
    ) -> io::Result<()> {
        if !same_immutable_grant(self, active)
            || self.state != MinifilterFenceLeaseState::Released
            || self.poison_reason != 0
            || self.release_nonce != Some(expected_release_nonce)
            || self.state_generation <= active.state_generation
            || self.query_sequence < active.query_sequence
            || self.blocked_mutation_count < active.blocked_mutation_count
            || self.release_sequence == 0
        {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_RELEASE_NOT_EXACT"));
        }
        Ok(())
    }
}

impl fmt::Debug for MinifilterFenceGrantReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinifilterFenceGrantReceipt")
            .field("session", &self.session)
            .field("scope", &"<kernel-derived-redacted>")
            .field("authority", &"<digest-bound>")
            .field("fence_id", &"<redacted>")
            .field("grant_secret", &"<redacted>")
            .field("grant_generation", &self.grant_generation)
            .field("state_generation", &self.state_generation)
            .field("state", &self.state)
            .field("requested_name", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct MinifilterFenceAcquireRequest {
    pub(super) request_id: MinifilterId,
    pub(super) connection_id: MinifilterId,
    pub(super) acquire_nonce: MinifilterId,
    pub(super) parent_handle_value: u64,
    pub(super) authority: MinifilterFenceAuthorityBinding,
    pub(super) child_name_utf16: Vec<u16>,
}

impl fmt::Debug for MinifilterFenceAcquireRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinifilterFenceAcquireRequest")
            .field("connection_id", &"<redacted>")
            .field("parent_handle", &"<retained>")
            .field("authority", &"<digest-bound>")
            .field("child_name", &"<redacted>")
            .finish()
    }
}

/// Query/Release use the current transport connection in the header while retaining the original
/// owner connection in the exact grant key. This permits a recovery connection to inspect a
/// poisoned lease without pretending it became the original owner.
#[derive(PartialEq, Eq)]
pub(super) struct MinifilterFenceGrantRequest {
    pub(super) request_id: MinifilterId,
    pub(super) transport_connection_id: MinifilterId,
    pub(super) grant_owner_connection_id: MinifilterId,
    pub(super) fence_id: MinifilterId,
    pub(super) grant_secret: MinifilterDigest,
    pub(super) driver_boot_id: MinifilterId,
    pub(super) driver_session_generation: u64,
    pub(super) volume_instance_id: MinifilterId,
    pub(super) volume_instance_generation: u64,
    pub(super) volume_serial: u64,
    pub(super) parent_file_id: MinifilterId,
    pub(super) filesystem_kind: MinifilterFenceFilesystemKind,
    pub(super) name_match_mode: MinifilterFenceNameMatchMode,
    pub(super) grant_generation: u64,
    pub(super) grant_sequence: u64,
    pub(super) expected_state_generation: u64,
    pub(super) acquire_nonce: MinifilterId,
    pub(super) authority: MinifilterFenceAuthorityBinding,
    pub(super) requested_name_utf16: Vec<u16>,
    pub(super) release_nonce: Option<MinifilterId>,
}

impl MinifilterFenceGrantRequest {
    pub(super) fn query(
        request_id: MinifilterId,
        transport_connection_id: MinifilterId,
        receipt: &MinifilterFenceGrantReceipt,
    ) -> Self {
        Self::new(request_id, transport_connection_id, receipt, None)
    }

    pub(super) fn release(
        request_id: MinifilterId,
        transport_connection_id: MinifilterId,
        release_nonce: MinifilterId,
        receipt: &MinifilterFenceGrantReceipt,
    ) -> Self {
        Self::new(
            request_id,
            transport_connection_id,
            receipt,
            Some(release_nonce),
        )
    }

    fn new(
        request_id: MinifilterId,
        transport_connection_id: MinifilterId,
        receipt: &MinifilterFenceGrantReceipt,
        release_nonce: Option<MinifilterId>,
    ) -> Self {
        Self {
            request_id,
            transport_connection_id,
            grant_owner_connection_id: receipt.session.connection_id,
            fence_id: receipt.fence_id,
            grant_secret: receipt.grant_secret,
            driver_boot_id: receipt.session.driver_boot_id,
            driver_session_generation: receipt.session.driver_session_generation,
            volume_instance_id: receipt.scope.volume_instance_id,
            volume_instance_generation: receipt.scope.volume_instance_generation,
            volume_serial: receipt.scope.volume_serial,
            parent_file_id: receipt.scope.parent_file_id,
            filesystem_kind: receipt.scope.filesystem_kind,
            name_match_mode: receipt.scope.name_match_mode,
            grant_generation: receipt.grant_generation,
            grant_sequence: receipt.grant_sequence,
            expected_state_generation: receipt.state_generation,
            acquire_nonce: receipt.acquire_nonce,
            authority: receipt.authority.clone(),
            requested_name_utf16: receipt.requested_name_utf16.clone(),
            release_nonce,
        }
    }
}

impl fmt::Debug for MinifilterFenceGrantRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinifilterFenceGrantRequest")
            .field("transport_connection", &"<redacted>")
            .field("grant_owner_connection", &"<redacted>")
            .field("fence_id", &"<redacted>")
            .field("grant_secret", &"<redacted>")
            .field("grant_generation", &self.grant_generation)
            .field("release", &self.release_nonce.is_some())
            .finish()
    }
}

fn same_immutable_grant(
    left: &MinifilterFenceGrantReceipt,
    right: &MinifilterFenceGrantReceipt,
) -> bool {
    left.session == right.session
        && left.scope == right.scope
        && left.authority == right.authority
        && left.acquire_nonce == right.acquire_nonce
        && left.fence_id == right.fence_id
        && left.grant_secret == right.grant_secret
        && left.grant_generation == right.grant_generation
        && left.grant_sequence == right.grant_sequence
        && left.requested_name_utf16 == right.requested_name_utf16
}
