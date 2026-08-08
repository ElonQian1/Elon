use std::io;

use super::invalid_data;

mod decode;
mod encode;

pub(super) use decode::{decode_fence_snapshot_reply, decode_session_reply};
pub(super) use encode::{
    encode_acquire_request, encode_describe_session_request, encode_query_request,
    encode_release_request,
};

pub(super) const HEADER_BYTES: usize = 64;
pub(super) const AUTHORITY_BYTES: usize = 288;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum MinifilterWireReplyStatus {
    Request = 0,
    Success = 1,
    ProtocolMismatch = 2,
    AccessDenied = 3,
    UnsupportedFilesystem = 4,
    ScopeConflict = 5,
    NonceConflict = 6,
    UnknownGrant = 7,
    GenerationMismatch = 8,
    Poisoned = 9,
    InternalFailure = 10,
}

impl MinifilterWireReplyStatus {
    pub(super) fn from_wire(value: u32) -> io::Result<Self> {
        match value {
            0 => Ok(Self::Request),
            1 => Ok(Self::Success),
            2 => Ok(Self::ProtocolMismatch),
            3 => Ok(Self::AccessDenied),
            4 => Ok(Self::UnsupportedFilesystem),
            5 => Ok(Self::ScopeConflict),
            6 => Ok(Self::NonceConflict),
            7 => Ok(Self::UnknownGrant),
            8 => Ok(Self::GenerationMismatch),
            9 => Ok(Self::Poisoned),
            10 => Ok(Self::InternalFailure),
            _ => Err(invalid_data("NODE_MINIFILTER_FENCE_REPLY_STATUS_INVALID")),
        }
    }
}

#[derive(Debug)]
pub(super) enum MinifilterWireReply<T> {
    Success(T),
    Rejected(MinifilterWireReplyStatus),
}
