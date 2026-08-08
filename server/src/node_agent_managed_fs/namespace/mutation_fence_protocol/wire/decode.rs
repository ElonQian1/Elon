use std::io;

use super::{MinifilterWireReply, MinifilterWireReplyStatus, HEADER_BYTES};
use crate::node_agent_managed_fs::namespace::mutation_fence_protocol::{
    invalid_data, is_zero, requested_name_digest_utf16, validate_child_name_utf16,
    MinifilterFenceAuthorityBinding, MinifilterFenceFilesystemKind, MinifilterFenceGrantReceipt,
    MinifilterFenceLeaseState, MinifilterFenceMessageKind, MinifilterFenceNameMatchMode,
    MinifilterFenceScopeReceipt, MinifilterFenceSessionReceipt, MinifilterId,
    MINIFILTER_FENCE_MAX_MESSAGE_BYTES, MINIFILTER_FENCE_PROTOCOL_MAGIC,
    MINIFILTER_FENCE_PROTOCOL_MAJOR, MINIFILTER_FENCE_PROTOCOL_MINOR,
};

const SNAPSHOT_FIXED_BYTES: usize = HEADER_BYTES + 600;
const SESSION_REPLY_BYTES: usize = HEADER_BYTES + 104;

struct WireHeader {
    total_bytes: usize,
    status: MinifilterWireReplyStatus,
    connection_id: MinifilterId,
}

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn decode_session_reply(
    bytes: &[u8],
    expected_request_id: MinifilterId,
) -> io::Result<MinifilterWireReply<MinifilterFenceSessionReceipt>> {
    let header = decode_reply_header(
        bytes,
        MinifilterFenceMessageKind::DescribeSessionReply,
        expected_request_id,
        None,
    )?;
    if header.status != MinifilterWireReplyStatus::Success {
        require_rejected_reply_shape(&header)?;
        return Ok(MinifilterWireReply::Rejected(header.status));
    }
    if header.total_bytes != SESSION_REPLY_BYTES || is_zero(&header.connection_id) {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_SESSION_REPLY_SIZE"));
    }
    let mut reader = Reader::new(&bytes[HEADER_BYTES..]);
    let receipt = MinifilterFenceSessionReceipt {
        connection_id: header.connection_id,
        driver_boot_id: reader.array()?,
        driver_build_digest: reader.array()?,
        wire_schema_digest: reader.array()?,
        driver_session_generation: reader.u64()?,
        capability_bits: reader.u64()?,
        maximum_message_bytes: reader.u32()?,
        pointer_width_bits: reader.u16()?,
    };
    if reader.u16()? != 0 {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_SESSION_RESERVED_NONZERO",
        ));
    }
    reader.finish()?;
    Ok(MinifilterWireReply::Success(receipt))
}

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn decode_fence_snapshot_reply(
    bytes: &[u8],
    expected_kind: MinifilterFenceMessageKind,
    expected_request_id: MinifilterId,
    expected_transport_connection_id: MinifilterId,
) -> io::Result<MinifilterWireReply<MinifilterFenceGrantReceipt>> {
    if is_zero(&expected_request_id) || is_zero(&expected_transport_connection_id) {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_EXPECTED_REPLY_BINDING_INVALID",
        ));
    }
    if !matches!(
        expected_kind,
        MinifilterFenceMessageKind::AcquireReply
            | MinifilterFenceMessageKind::QueryReply
            | MinifilterFenceMessageKind::ReleaseReply
    ) {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_SNAPSHOT_KIND_INVALID"));
    }
    let header = decode_reply_header(
        bytes,
        expected_kind,
        expected_request_id,
        Some(expected_transport_connection_id),
    )?;
    if header.status != MinifilterWireReplyStatus::Success {
        require_rejected_reply_shape(&header)?;
        return Ok(MinifilterWireReply::Rejected(header.status));
    }
    if header.total_bytes < SNAPSHOT_FIXED_BYTES || is_zero(&header.connection_id) {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_SNAPSHOT_SIZE"));
    }
    let mut reader = Reader::new(&bytes[HEADER_BYTES..SNAPSHOT_FIXED_BYTES]);
    let state = MinifilterFenceLeaseState::from_wire(reader.u16()?)?;
    let filesystem_kind = MinifilterFenceFilesystemKind::from_wire(reader.u16()?)?;
    let name_match_mode = MinifilterFenceNameMatchMode::from_wire(reader.u16()?)?;
    let poison_reason = reader.u16()?;
    let driver_boot_id = reader.array()?;
    let grant_owner_connection_id = reader.array()?;
    let driver_build_digest = reader.array()?;
    let wire_schema_digest = reader.array()?;
    let driver_session_generation = reader.u64()?;
    let capability_bits = reader.u64()?;
    let maximum_message_bytes = reader.u32()?;
    let pointer_width_bits = reader.u16()?;
    if reader.u16()? != 0 {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_SNAPSHOT_RESERVED_NONZERO",
        ));
    }
    let volume_instance_id = reader.array()?;
    let volume_instance_generation = reader.u64()?;
    let volume_serial = reader.u64()?;
    let parent_file_id = reader.array()?;
    let acquire_nonce = reader.array()?;
    let fence_id = reader.array()?;
    let grant_secret = reader.array()?;
    let grant_generation = reader.u64()?;
    let grant_sequence = reader.u64()?;
    let state_generation = reader.u64()?;
    let query_sequence = reader.u64()?;
    let blocked_mutation_count = reader.u64()?;
    let release_sequence = reader.u64()?;
    let release_nonce_raw: MinifilterId = reader.array()?;
    let release_nonce = (!is_zero(&release_nonce_raw)).then_some(release_nonce_raw);
    let authority = decode_authority(&mut reader)?;
    let name_offset = reader.u32()? as usize;
    let name_bytes = reader.u32()? as usize;
    if name_offset != SNAPSHOT_FIXED_BYTES
        || name_bytes == 0
        || name_bytes % 2 != 0
        || name_offset.checked_add(name_bytes) != Some(header.total_bytes)
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_NAME_RANGE_INVALID"));
    }
    reader.finish()?;
    let requested_name_utf16 = decode_utf16(&bytes[name_offset..])?;
    if requested_name_digest_utf16(&requested_name_utf16) != authority.requested_name_digest {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_NAME_DIGEST_CHANGED"));
    }
    Ok(MinifilterWireReply::Success(MinifilterFenceGrantReceipt {
        session: MinifilterFenceSessionReceipt {
            connection_id: grant_owner_connection_id,
            driver_boot_id,
            driver_build_digest,
            wire_schema_digest,
            driver_session_generation,
            capability_bits,
            maximum_message_bytes,
            pointer_width_bits,
        },
        scope: MinifilterFenceScopeReceipt {
            volume_instance_id,
            volume_instance_generation,
            volume_serial,
            parent_file_id,
            filesystem_kind,
            name_match_mode,
        },
        authority,
        acquire_nonce,
        fence_id,
        grant_secret,
        grant_generation,
        grant_sequence,
        state_generation,
        query_sequence,
        blocked_mutation_count,
        release_sequence,
        state,
        poison_reason,
        release_nonce,
        requested_name_utf16,
    }))
}

fn decode_reply_header(
    bytes: &[u8],
    expected_kind: MinifilterFenceMessageKind,
    expected_request_id: MinifilterId,
    expected_connection_id: Option<MinifilterId>,
) -> io::Result<WireHeader> {
    if bytes.len() < HEADER_BYTES || bytes.len() > MINIFILTER_FENCE_MAX_MESSAGE_BYTES {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_REPLY_SIZE_INVALID"));
    }
    let mut reader = Reader::new(&bytes[..HEADER_BYTES]);
    if reader.array::<8>()? != MINIFILTER_FENCE_PROTOCOL_MAGIC
        || reader.u16()? != MINIFILTER_FENCE_PROTOCOL_MAJOR
        || reader.u16()? != MINIFILTER_FENCE_PROTOCOL_MINOR
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_PROTOCOL_MISMATCH"));
    }
    let kind = MinifilterFenceMessageKind::from_wire(reader.u16()?)?;
    let header_bytes = reader.u16()? as usize;
    let total_bytes = reader.u32()? as usize;
    let flags = reader.u32()?;
    let status = MinifilterWireReplyStatus::from_wire(reader.u32()?)?;
    let reserved = reader.u32()?;
    let request_id = reader.array()?;
    let connection_id = reader.array()?;
    reader.finish()?;
    if kind != expected_kind
        || header_bytes != HEADER_BYTES
        || total_bytes != bytes.len()
        || flags != 0
        || reserved != 0
        || status == MinifilterWireReplyStatus::Request
        || request_id != expected_request_id
        || is_zero(&request_id)
        || expected_connection_id.is_some_and(|expected| connection_id != expected)
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_REPLY_HEADER_INVALID"));
    }
    Ok(WireHeader {
        total_bytes,
        status,
        connection_id,
    })
}

fn require_rejected_reply_shape(header: &WireHeader) -> io::Result<()> {
    if header.total_bytes != HEADER_BYTES {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_REJECTION_BODY_FORBIDDEN",
        ));
    }
    Ok(())
}

fn decode_authority(reader: &mut Reader<'_>) -> io::Result<MinifilterFenceAuthorityBinding> {
    let value = MinifilterFenceAuthorityBinding {
        installation_id_digest: reader.array()?,
        cleanup_id_digest: reader.array()?,
        execution_plan_digest: reader.array()?,
        authorization_receipt_digest: reader.array()?,
        expected_object_digest: reader.array()?,
        expected_object_identity_digest: reader.array()?,
        expected_parent_identity_digest: reader.array()?,
        requested_name_digest: reader.array()?,
        authority_epoch: reader.u64()?,
        process_owner_epoch: reader.u64()?,
        step_ordinal: reader.u64()?,
    };
    if reader.u64()? != 0 {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_AUTHORITY_RESERVED_NONZERO",
        ));
    }
    Ok(value)
}

fn decode_utf16(bytes: &[u8]) -> io::Result<Vec<u16>> {
    if bytes.len() % 2 != 0 {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_UTF16_LENGTH_INVALID"));
    }
    let value = bytes
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    validate_child_name_utf16(&value)?;
    Ok(value)
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let end = self
            .offset
            .checked_add(N)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| invalid_data("NODE_MINIFILTER_FENCE_REPLY_TRUNCATED"))?;
        let value = self.bytes[self.offset..end]
            .try_into()
            .map_err(|_| invalid_data("NODE_MINIFILTER_FENCE_REPLY_TRUNCATED"))?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn finish(self) -> io::Result<()> {
        if self.offset != self.bytes.len() {
            return Err(invalid_data("NODE_MINIFILTER_FENCE_REPLY_TRAILING_BYTES"));
        }
        Ok(())
    }
}
