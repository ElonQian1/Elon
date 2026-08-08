use std::{fmt::Debug, io};

use sha2::{Digest, Sha256};

use super::*;
use crate::{
    node_agent_managed_fs::namespace::mutation_fence_protocol::{
        encode_acquire_request, encode_describe_session_request, encode_query_request,
        encode_release_request, MinifilterFenceGrantRequest, MinifilterFenceLeaseState,
        MinifilterFenceMessageKind, MINIFILTER_FENCE_PROTOCOL_MAGIC,
    },
    node_agent_privileged_component::contract::WINDOWS_NAMESPACE_FENCE_WIRE_SCHEMA_SHA256,
};

mod fixtures;

use fixtures::*;

#[test]
fn descriptor_digest_and_fixed_sizes_match_the_compiled_contract() {
    let descriptor: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/distributed-compute/windows-compute-namespace-fence-wire-v1.json"
    )))
    .unwrap();
    let canonical = serde_json::to_vec(&descriptor).unwrap();

    assert_eq!(
        hex::encode(Sha256::digest(canonical)),
        WINDOWS_NAMESPACE_FENCE_WIRE_SCHEMA_SHA256
    );
    assert_eq!(HEADER_BYTES, 64);
    assert_eq!(AUTHORITY_BYTES, 288);
}

#[test]
fn describe_request_has_an_exact_zero_reserved_header() {
    let encoded = encode_describe_session_request(REQUEST_ID).unwrap();

    assert_eq!(encoded.len(), HEADER_BYTES);
    assert_eq!(&encoded[..8], &MINIFILTER_FENCE_PROTOCOL_MAGIC);
    assert_eq!(read_u16(&encoded, 12), 1);
    assert_eq!(read_u16(&encoded, 14), HEADER_BYTES as u16);
    assert_eq!(read_u32(&encoded, 16), HEADER_BYTES as u32);
    assert_eq!(&encoded[20..32], &[0; 12]);
    assert_eq!(&encoded[32..48], &REQUEST_ID);
    assert_eq!(&encoded[48..64], &[0; 16]);

    assert_error(
        encode_describe_session_request([0; 16]),
        "NODE_MINIFILTER_FENCE_REQUEST_ID_INVALID",
    );
}

#[test]
fn acquire_request_binds_the_exact_name_and_authority_layout() {
    let request = acquire_request();
    let encoded = encode_acquire_request(&request).unwrap();
    let expected_name_bytes = request.child_name_utf16.len() * 2;

    assert_eq!(encoded.len(), 384 + expected_name_bytes);
    assert_eq!(read_u16(&encoded, 12), 3);
    assert_eq!(read_u32(&encoded, 88), 384);
    assert_eq!(read_u32(&encoded, 92), expected_name_bytes as u32);
    assert_eq!(&encoded[376..384], &[0; 8]);
    assert_eq!(&encoded[320..352], &request.authority.requested_name_digest);

    let mut changed = request.clone();
    changed.child_name_utf16[0] = b'k' as u16;
    assert_error(
        encode_acquire_request(&changed),
        "NODE_MINIFILTER_FENCE_ACQUIRE_NAME_BINDING_CHANGED",
    );
}

#[test]
fn query_and_release_keep_transport_owner_and_nonce_roles_separate() {
    let snapshot = snapshot_reply(
        MinifilterFenceMessageKind::AcquireReply,
        MinifilterWireReplyStatus::Success,
    );
    let receipt = success(decode_fence_snapshot_reply(
        &snapshot,
        MinifilterFenceMessageKind::AcquireReply,
        REQUEST_ID,
        TRANSPORT_CONNECTION_ID,
    ));
    let query = MinifilterFenceGrantRequest::query([0x71; 16], TRANSPORT_CONNECTION_ID, &receipt);
    let release = MinifilterFenceGrantRequest::release(
        [0x72; 16],
        TRANSPORT_CONNECTION_ID,
        RELEASE_NONCE,
        &receipt,
    );
    let query_bytes = encode_query_request(&query).unwrap();
    let release_bytes = encode_release_request(&release).unwrap();

    assert_eq!(read_u16(&query_bytes, 12), 5);
    assert_eq!(read_u32(&query_bytes, 536), 544);
    assert_eq!(&query_bytes[128..144], &GRANT_OWNER_CONNECTION_ID);
    assert_eq!(&query_bytes[48..64], &TRANSPORT_CONNECTION_ID);
    assert_eq!(read_u16(&release_bytes, 12), 7);
    assert_eq!(&release_bytes[536..552], &RELEASE_NONCE);
    assert_eq!(read_u32(&release_bytes, 552), 560);

    assert_error(
        encode_query_request(&release),
        "NODE_MINIFILTER_FENCE_QUERY_HAS_RELEASE_NONCE",
    );
    assert_error(
        encode_release_request(&query),
        "NODE_MINIFILTER_FENCE_RELEASE_NONCE_MISSING",
    );
}

#[test]
fn session_reply_decodes_success_and_bodyless_rejection() {
    let receipt = success(decode_session_reply(
        &session_reply(MinifilterWireReplyStatus::Success),
        REQUEST_ID,
    ));
    assert_eq!(receipt.connection_id, TRANSPORT_CONNECTION_ID);
    assert_eq!(receipt.driver_session_generation, 44);

    let rejected = decode_session_reply(
        &session_reply(MinifilterWireReplyStatus::AccessDenied),
        REQUEST_ID,
    )
    .unwrap();
    assert!(matches!(
        rejected,
        MinifilterWireReply::Rejected(MinifilterWireReplyStatus::AccessDenied)
    ));
}

#[test]
fn session_reply_rejects_unknown_status_reserved_and_stale_request() {
    let mut unknown = session_reply(MinifilterWireReplyStatus::Success);
    set_u32(&mut unknown, 24, 99);
    assert_error(
        decode_session_reply(&unknown, REQUEST_ID),
        "NODE_MINIFILTER_FENCE_REPLY_STATUS_INVALID",
    );

    let mut reserved = session_reply(MinifilterWireReplyStatus::Success);
    set_u32(&mut reserved, 28, 1);
    assert_error(
        decode_session_reply(&reserved, REQUEST_ID),
        "NODE_MINIFILTER_FENCE_REPLY_HEADER_INVALID",
    );

    assert_error(
        decode_session_reply(
            &session_reply(MinifilterWireReplyStatus::Success),
            [0x7f; 16],
        ),
        "NODE_MINIFILTER_FENCE_REPLY_HEADER_INVALID",
    );
}

#[test]
fn snapshot_reply_decodes_the_exact_grant_binding() {
    let receipt = success(decode_fence_snapshot_reply(
        &snapshot_reply(
            MinifilterFenceMessageKind::AcquireReply,
            MinifilterWireReplyStatus::Success,
        ),
        MinifilterFenceMessageKind::AcquireReply,
        REQUEST_ID,
        TRANSPORT_CONNECTION_ID,
    ));

    assert_eq!(receipt.state, MinifilterFenceLeaseState::Active);
    assert_eq!(receipt.session.connection_id, GRANT_OWNER_CONNECTION_ID);
    assert_eq!(receipt.scope.volume_serial, 57);
    assert_eq!(receipt.grant_generation, 60);
    assert_eq!(receipt.grant_sequence, 61);
    assert_eq!(receipt.state_generation, 62);
    assert_eq!(receipt.requested_name_utf16, child_name());
}

#[test]
fn snapshot_reply_rejects_reserved_range_and_name_digest_tampering() {
    let baseline = snapshot_reply(
        MinifilterFenceMessageKind::AcquireReply,
        MinifilterWireReplyStatus::Success,
    );
    let mut reserved = baseline.clone();
    set_u16(&mut reserved, 190, 1);
    assert_error(
        decode_snapshot(&reserved),
        "NODE_MINIFILTER_FENCE_SNAPSHOT_RESERVED_NONZERO",
    );

    let mut range = baseline.clone();
    set_u32(&mut range, 656, 663);
    assert_error(
        decode_snapshot(&range),
        "NODE_MINIFILTER_FENCE_NAME_RANGE_INVALID",
    );

    let mut digest = baseline;
    digest[592] ^= 1;
    assert_error(
        decode_snapshot(&digest),
        "NODE_MINIFILTER_FENCE_NAME_DIGEST_CHANGED",
    );
}

#[test]
fn rejected_snapshot_forbids_a_body_and_wrong_reply_kind() {
    let mut rejected = snapshot_reply(
        MinifilterFenceMessageKind::QueryReply,
        MinifilterWireReplyStatus::UnknownGrant,
    );
    rejected.extend_from_slice(&[0; 8]);
    let rejected_len = rejected.len() as u32;
    set_u32(&mut rejected, 16, rejected_len);
    assert_error(
        decode_fence_snapshot_reply(
            &rejected,
            MinifilterFenceMessageKind::QueryReply,
            REQUEST_ID,
            TRANSPORT_CONNECTION_ID,
        ),
        "NODE_MINIFILTER_FENCE_REJECTION_BODY_FORBIDDEN",
    );

    assert_error(
        decode_fence_snapshot_reply(
            &snapshot_reply(
                MinifilterFenceMessageKind::AcquireReply,
                MinifilterWireReplyStatus::Success,
            ),
            MinifilterFenceMessageKind::AcquireRequest,
            REQUEST_ID,
            TRANSPORT_CONNECTION_ID,
        ),
        "NODE_MINIFILTER_FENCE_SNAPSHOT_KIND_INVALID",
    );
}

fn decode_snapshot(
    bytes: &[u8],
) -> io::Result<
    MinifilterWireReply<
        crate::node_agent_managed_fs::namespace::mutation_fence_protocol::MinifilterFenceGrantReceipt,
    >,
>
{
    decode_fence_snapshot_reply(
        bytes,
        MinifilterFenceMessageKind::AcquireReply,
        REQUEST_ID,
        TRANSPORT_CONNECTION_ID,
    )
}

fn success<T: Debug>(result: io::Result<MinifilterWireReply<T>>) -> T {
    match result.unwrap() {
        MinifilterWireReply::Success(value) => value,
        MinifilterWireReply::Rejected(status) => panic!("unexpected rejection: {status:?}"),
    }
}

fn assert_error<T: Debug>(result: io::Result<T>, expected: &str) {
    assert_eq!(result.unwrap_err().to_string(), expected);
}
