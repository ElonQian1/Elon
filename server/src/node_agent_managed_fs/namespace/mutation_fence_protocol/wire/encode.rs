use std::io;

use super::{MinifilterWireReplyStatus, AUTHORITY_BYTES, HEADER_BYTES};
use crate::node_agent_managed_fs::namespace::mutation_fence_protocol::{
    invalid_data, is_zero, requested_name_digest_utf16, validate_child_name_utf16,
    MinifilterFenceAcquireRequest, MinifilterFenceAuthorityBinding, MinifilterFenceGrantRequest,
    MinifilterFenceMessageKind, MinifilterId, MINIFILTER_FENCE_MAX_MESSAGE_BYTES,
    MINIFILTER_FENCE_PROTOCOL_MAGIC, MINIFILTER_FENCE_PROTOCOL_MAJOR,
    MINIFILTER_FENCE_PROTOCOL_MINOR,
};

const ACQUIRE_FIXED_BYTES: usize = HEADER_BYTES + 16 + 8 + 4 + 4 + AUTHORITY_BYTES;
const GRANT_KEY_FIXED_BODY_BYTES: usize = 480;

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn encode_describe_session_request(
    request_id: MinifilterId,
) -> io::Result<Vec<u8>> {
    if is_zero(&request_id) {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_REQUEST_ID_INVALID"));
    }
    Ok(encode_header(
        MinifilterFenceMessageKind::DescribeSessionRequest,
        HEADER_BYTES,
        request_id,
        [0; 16],
    ))
}

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn encode_acquire_request(
    request: &MinifilterFenceAcquireRequest,
) -> io::Result<Vec<u8>> {
    validate_common_request_ids(request.request_id, request.connection_id)?;
    if is_zero(&request.acquire_nonce)
        || request.parent_handle_value == 0
        || request.parent_handle_value == u64::MAX
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_ACQUIRE_KEY_INVALID"));
    }
    validate_child_name_utf16(&request.child_name_utf16)?;
    if requested_name_digest_utf16(&request.child_name_utf16)
        != request.authority.requested_name_digest
    {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_ACQUIRE_NAME_BINDING_CHANGED",
        ));
    }
    let child_name_bytes = request.child_name_utf16.len() * 2;
    let total_bytes = ACQUIRE_FIXED_BYTES
        .checked_add(child_name_bytes)
        .filter(|total| *total <= MINIFILTER_FENCE_MAX_MESSAGE_BYTES)
        .ok_or_else(|| invalid_data("NODE_MINIFILTER_FENCE_MESSAGE_TOO_LARGE"))?;
    let mut output = encode_header(
        MinifilterFenceMessageKind::AcquireRequest,
        total_bytes,
        request.request_id,
        request.connection_id,
    );
    output.extend_from_slice(&request.acquire_nonce);
    put_u64(&mut output, request.parent_handle_value);
    put_u32(&mut output, ACQUIRE_FIXED_BYTES as u32);
    put_u32(&mut output, child_name_bytes as u32);
    encode_authority(&mut output, &request.authority);
    encode_utf16(&mut output, &request.child_name_utf16);
    debug_assert_eq!(output.len(), total_bytes);
    Ok(output)
}

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn encode_query_request(
    request: &MinifilterFenceGrantRequest,
) -> io::Result<Vec<u8>> {
    if request.release_nonce.is_some() {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_QUERY_HAS_RELEASE_NONCE",
        ));
    }
    encode_grant_request(MinifilterFenceMessageKind::QueryRequest, request)
}

pub(in crate::node_agent_managed_fs::namespace::mutation_fence_protocol) fn encode_release_request(
    request: &MinifilterFenceGrantRequest,
) -> io::Result<Vec<u8>> {
    if request.release_nonce.is_none() {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_RELEASE_NONCE_MISSING"));
    }
    encode_grant_request(MinifilterFenceMessageKind::ReleaseRequest, request)
}

fn encode_grant_request(
    kind: MinifilterFenceMessageKind,
    request: &MinifilterFenceGrantRequest,
) -> io::Result<Vec<u8>> {
    validate_common_request_ids(request.request_id, request.transport_connection_id)?;
    if is_zero(&request.fence_id)
        || is_zero(&request.grant_secret)
        || is_zero(&request.driver_boot_id)
        || is_zero(&request.grant_owner_connection_id)
        || is_zero(&request.volume_instance_id)
        || is_zero(&request.acquire_nonce)
        || is_zero(&request.parent_file_id)
        || request.driver_session_generation == 0
        || request.volume_instance_generation == 0
        || request.grant_generation == 0
        || request.grant_sequence == 0
        || request.expected_state_generation == 0
        || request.release_nonce.as_ref().is_some_and(is_zero)
    {
        return Err(invalid_data("NODE_MINIFILTER_FENCE_GRANT_KEY_INVALID"));
    }
    validate_child_name_utf16(&request.requested_name_utf16)?;
    if requested_name_digest_utf16(&request.requested_name_utf16)
        != request.authority.requested_name_digest
    {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_GRANT_NAME_BINDING_CHANGED",
        ));
    }
    let release_bytes = usize::from(request.release_nonce.is_some()) * 16;
    let fixed_bytes = HEADER_BYTES + GRANT_KEY_FIXED_BODY_BYTES + release_bytes;
    let name_bytes = request.requested_name_utf16.len() * 2;
    let total_bytes = fixed_bytes
        .checked_add(name_bytes)
        .filter(|total| *total <= MINIFILTER_FENCE_MAX_MESSAGE_BYTES)
        .ok_or_else(|| invalid_data("NODE_MINIFILTER_FENCE_MESSAGE_TOO_LARGE"))?;
    let mut output = encode_header(
        kind,
        total_bytes,
        request.request_id,
        request.transport_connection_id,
    );
    output.extend_from_slice(&request.fence_id);
    output.extend_from_slice(&request.grant_secret);
    output.extend_from_slice(&request.driver_boot_id);
    output.extend_from_slice(&request.grant_owner_connection_id);
    output.extend_from_slice(&request.volume_instance_id);
    output.extend_from_slice(&request.acquire_nonce);
    output.extend_from_slice(&request.parent_file_id);
    put_u64(&mut output, request.driver_session_generation);
    put_u64(&mut output, request.volume_instance_generation);
    put_u64(&mut output, request.volume_serial);
    put_u64(&mut output, request.grant_generation);
    put_u64(&mut output, request.grant_sequence);
    put_u64(&mut output, request.expected_state_generation);
    put_u16(&mut output, request.filesystem_kind as u16);
    put_u16(&mut output, request.name_match_mode as u16);
    put_u32(&mut output, 0);
    encode_authority(&mut output, &request.authority);
    if let Some(release_nonce) = request.release_nonce {
        output.extend_from_slice(&release_nonce);
    }
    put_u32(&mut output, fixed_bytes as u32);
    put_u32(&mut output, name_bytes as u32);
    encode_utf16(&mut output, &request.requested_name_utf16);
    debug_assert_eq!(output.len(), total_bytes);
    Ok(output)
}

fn encode_header(
    kind: MinifilterFenceMessageKind,
    total_bytes: usize,
    request_id: MinifilterId,
    connection_id: MinifilterId,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(total_bytes);
    output.extend_from_slice(&MINIFILTER_FENCE_PROTOCOL_MAGIC);
    put_u16(&mut output, MINIFILTER_FENCE_PROTOCOL_MAJOR);
    put_u16(&mut output, MINIFILTER_FENCE_PROTOCOL_MINOR);
    put_u16(&mut output, kind as u16);
    put_u16(&mut output, HEADER_BYTES as u16);
    put_u32(&mut output, total_bytes as u32);
    put_u32(&mut output, 0);
    put_u32(&mut output, MinifilterWireReplyStatus::Request as u32);
    put_u32(&mut output, 0);
    output.extend_from_slice(&request_id);
    output.extend_from_slice(&connection_id);
    debug_assert_eq!(output.len(), HEADER_BYTES);
    output
}

fn encode_authority(output: &mut Vec<u8>, value: &MinifilterFenceAuthorityBinding) {
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
}

fn validate_common_request_ids(
    request_id: MinifilterId,
    connection_id: MinifilterId,
) -> io::Result<()> {
    if is_zero(&request_id) || is_zero(&connection_id) {
        return Err(invalid_data(
            "NODE_MINIFILTER_FENCE_REQUEST_BINDING_INVALID",
        ));
    }
    Ok(())
}

fn encode_utf16(output: &mut Vec<u8>, value: &[u16]) {
    for unit in value {
        output.extend_from_slice(&unit.to_le_bytes());
    }
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
