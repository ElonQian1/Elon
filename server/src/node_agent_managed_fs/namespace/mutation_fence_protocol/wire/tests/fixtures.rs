use super::super::{MinifilterWireReplyStatus, AUTHORITY_BYTES, HEADER_BYTES};
use crate::node_agent_managed_fs::namespace::mutation_fence_protocol::{
    requested_name_digest_utf16, MinifilterFenceAcquireRequest, MinifilterFenceAuthorityBinding,
    MinifilterFenceFilesystemKind, MinifilterFenceMessageKind, MinifilterFenceNameMatchMode,
    MinifilterId, MINIFILTER_FENCE_MAX_MESSAGE_BYTES, MINIFILTER_FENCE_PROTOCOL_MAGIC,
    MINIFILTER_FENCE_PROTOCOL_MAJOR, MINIFILTER_FENCE_PROTOCOL_MINOR,
    MINIFILTER_FENCE_REQUIRED_FEATURE_MASK,
};

pub(super) const REQUEST_ID: MinifilterId = [0x11; 16];
pub(super) const TRANSPORT_CONNECTION_ID: MinifilterId = [0x12; 16];
pub(super) const GRANT_OWNER_CONNECTION_ID: MinifilterId = [0x13; 16];
pub(super) const RELEASE_NONCE: MinifilterId = [0x14; 16];

pub(super) fn child_name() -> Vec<u16> {
    "coffee.db".encode_utf16().collect()
}

pub(super) fn authority(name: &[u16]) -> MinifilterFenceAuthorityBinding {
    MinifilterFenceAuthorityBinding {
        installation_id_digest: [0x21; 32],
        cleanup_id_digest: [0x22; 32],
        execution_plan_digest: [0x23; 32],
        authorization_receipt_digest: [0x24; 32],
        expected_object_digest: [0x25; 32],
        expected_object_identity_digest: [0x26; 32],
        expected_parent_identity_digest: [0x27; 32],
        requested_name_digest: requested_name_digest_utf16(name),
        authority_epoch: 31,
        process_owner_epoch: 32,
        step_ordinal: 33,
    }
}

pub(super) fn acquire_request() -> MinifilterFenceAcquireRequest {
    let child_name_utf16 = child_name();
    MinifilterFenceAcquireRequest {
        request_id: REQUEST_ID,
        connection_id: TRANSPORT_CONNECTION_ID,
        acquire_nonce: [0x31; 16],
        parent_handle_value: 0x1234,
        authority: authority(&child_name_utf16),
        child_name_utf16,
    }
}

pub(super) fn session_reply(status: MinifilterWireReplyStatus) -> Vec<u8> {
    let total_bytes = if status == MinifilterWireReplyStatus::Success {
        HEADER_BYTES + 104
    } else {
        HEADER_BYTES
    };
    let mut output = reply_header(
        MinifilterFenceMessageKind::DescribeSessionReply,
        total_bytes,
        REQUEST_ID,
        TRANSPORT_CONNECTION_ID,
        status,
    );
    if status == MinifilterWireReplyStatus::Success {
        output.extend_from_slice(&[0x41; 16]);
        output.extend_from_slice(&[0x42; 32]);
        output.extend_from_slice(&[0x43; 32]);
        put_u64(&mut output, 44);
        put_u64(&mut output, MINIFILTER_FENCE_REQUIRED_FEATURE_MASK);
        put_u32(&mut output, MINIFILTER_FENCE_MAX_MESSAGE_BYTES as u32);
        put_u16(&mut output, 64);
        put_u16(&mut output, 0);
    }
    output
}

pub(super) fn snapshot_reply(
    kind: MinifilterFenceMessageKind,
    status: MinifilterWireReplyStatus,
) -> Vec<u8> {
    if status != MinifilterWireReplyStatus::Success {
        return reply_header(
            kind,
            HEADER_BYTES,
            REQUEST_ID,
            TRANSPORT_CONNECTION_ID,
            status,
        );
    }
    let name = child_name();
    let name_bytes = name.len() * 2;
    let fixed_bytes = HEADER_BYTES + 600;
    let total_bytes = fixed_bytes + name_bytes;
    let mut output = reply_header(
        kind,
        total_bytes,
        REQUEST_ID,
        TRANSPORT_CONNECTION_ID,
        status,
    );
    put_u16(&mut output, 1);
    put_u16(&mut output, MinifilterFenceFilesystemKind::Ntfs as u16);
    put_u16(
        &mut output,
        MinifilterFenceNameMatchMode::CaseInsensitive as u16,
    );
    put_u16(&mut output, 0);
    output.extend_from_slice(&[0x51; 16]);
    output.extend_from_slice(&GRANT_OWNER_CONNECTION_ID);
    output.extend_from_slice(&[0x52; 32]);
    output.extend_from_slice(&[0x53; 32]);
    put_u64(&mut output, 54);
    put_u64(&mut output, MINIFILTER_FENCE_REQUIRED_FEATURE_MASK);
    put_u32(&mut output, MINIFILTER_FENCE_MAX_MESSAGE_BYTES as u32);
    put_u16(&mut output, 64);
    put_u16(&mut output, 0);
    output.extend_from_slice(&[0x55; 16]);
    put_u64(&mut output, 56);
    put_u64(&mut output, 57);
    output.extend_from_slice(&[0x58; 16]);
    output.extend_from_slice(&[0x59; 16]);
    output.extend_from_slice(&[0x5a; 16]);
    output.extend_from_slice(&[0x5b; 32]);
    put_u64(&mut output, 60);
    put_u64(&mut output, 61);
    put_u64(&mut output, 62);
    put_u64(&mut output, 0);
    put_u64(&mut output, 0);
    put_u64(&mut output, 0);
    output.extend_from_slice(&[0; 16]);
    encode_authority(&mut output, &authority(&name));
    put_u32(&mut output, fixed_bytes as u32);
    put_u32(&mut output, name_bytes as u32);
    for unit in name {
        put_u16(&mut output, unit);
    }
    assert_eq!(output.len(), total_bytes);
    output
}

pub(super) fn reply_header(
    kind: MinifilterFenceMessageKind,
    total_bytes: usize,
    request_id: MinifilterId,
    connection_id: MinifilterId,
    status: MinifilterWireReplyStatus,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(total_bytes);
    output.extend_from_slice(&MINIFILTER_FENCE_PROTOCOL_MAGIC);
    put_u16(&mut output, MINIFILTER_FENCE_PROTOCOL_MAJOR);
    put_u16(&mut output, MINIFILTER_FENCE_PROTOCOL_MINOR);
    put_u16(&mut output, kind as u16);
    put_u16(&mut output, HEADER_BYTES as u16);
    put_u32(&mut output, total_bytes as u32);
    put_u32(&mut output, 0);
    put_u32(&mut output, status as u32);
    put_u32(&mut output, 0);
    output.extend_from_slice(&request_id);
    output.extend_from_slice(&connection_id);
    assert_eq!(output.len(), HEADER_BYTES);
    output
}

pub(super) fn set_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn set_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn encode_authority(output: &mut Vec<u8>, value: &MinifilterFenceAuthorityBinding) {
    let start = output.len();
    output.extend_from_slice(&value.installation_id_digest);
    output.extend_from_slice(&value.cleanup_id_digest);
    output.extend_from_slice(&value.execution_plan_digest);
    output.extend_from_slice(&value.authorization_receipt_digest);
    output.extend_from_slice(&value.expected_object_digest);
    output.extend_from_slice(&value.expected_object_identity_digest);
    output.extend_from_slice(&value.expected_parent_identity_digest);
    output.extend_from_slice(&value.requested_name_digest);
    put_u64(output, value.authority_epoch);
    put_u64(output, value.process_owner_epoch);
    put_u64(output, value.step_ordinal);
    put_u64(output, 0);
    assert_eq!(output.len() - start, AUTHORITY_BYTES);
}

fn put_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}
