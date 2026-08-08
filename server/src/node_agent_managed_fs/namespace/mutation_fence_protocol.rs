use std::io;

mod binding;
mod receipt;
mod wire;

use binding::{
    child_name_utf16, decode_sha256, requested_name_digest_utf16, validate_child_name_utf16,
    MinifilterFenceAuthorityBinding,
};
pub(super) use receipt::MinifilterFenceGrantReceipt;
use receipt::{
    MinifilterFenceAcquireRequest, MinifilterFenceGrantRequest, MinifilterFenceScopeReceipt,
    MinifilterFenceSessionReceipt,
};
use wire::{
    decode_fence_snapshot_reply, decode_session_reply, encode_acquire_request,
    encode_describe_session_request, encode_query_request, encode_release_request,
    MinifilterWireReply, MinifilterWireReplyStatus,
};

pub(super) const MINIFILTER_FENCE_PROTOCOL_MAGIC: [u8; 8] = *b"ELONFNC1";
pub(super) const MINIFILTER_FENCE_PROTOCOL_MAJOR: u16 = 1;
pub(super) const MINIFILTER_FENCE_PROTOCOL_MINOR: u16 = 0;
pub(super) const MINIFILTER_FENCE_MAX_MESSAGE_BYTES: usize = 4 * 1024;
pub(super) const MINIFILTER_FENCE_MAX_CHILD_NAME_UTF16_UNITS: usize = 255;
pub(super) const MINIFILTER_FENCE_REQUIRED_FEATURE_MASK: u64 = 0x0000_0000_0000_ffff;

pub(super) type MinifilterId = [u8; 16];
pub(super) type MinifilterDigest = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum MinifilterFenceMessageKind {
    DescribeSessionRequest = 1,
    DescribeSessionReply = 2,
    AcquireRequest = 3,
    AcquireReply = 4,
    QueryRequest = 5,
    QueryReply = 6,
    ReleaseRequest = 7,
    ReleaseReply = 8,
}

impl MinifilterFenceMessageKind {
    pub(super) fn from_wire(value: u16) -> io::Result<Self> {
        match value {
            1 => Ok(Self::DescribeSessionRequest),
            2 => Ok(Self::DescribeSessionReply),
            3 => Ok(Self::AcquireRequest),
            4 => Ok(Self::AcquireReply),
            5 => Ok(Self::QueryRequest),
            6 => Ok(Self::QueryReply),
            7 => Ok(Self::ReleaseRequest),
            8 => Ok(Self::ReleaseReply),
            _ => Err(invalid_data("NODE_MINIFILTER_FENCE_MESSAGE_KIND_INVALID")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum MinifilterFenceLeaseState {
    Active = 1,
    PoisonedConnectionLost = 2,
    PoisonedAmbiguousMutation = 3,
    PoisonedInstanceTeardown = 4,
    PoisonedDriverTeardown = 5,
    PoisonedInternal = 6,
    Releasing = 7,
    Released = 8,
}

impl MinifilterFenceLeaseState {
    pub(super) fn from_wire(value: u16) -> io::Result<Self> {
        match value {
            1 => Ok(Self::Active),
            2 => Ok(Self::PoisonedConnectionLost),
            3 => Ok(Self::PoisonedAmbiguousMutation),
            4 => Ok(Self::PoisonedInstanceTeardown),
            5 => Ok(Self::PoisonedDriverTeardown),
            6 => Ok(Self::PoisonedInternal),
            7 => Ok(Self::Releasing),
            8 => Ok(Self::Released),
            _ => Err(invalid_data("NODE_MINIFILTER_FENCE_STATE_INVALID")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum MinifilterFenceFilesystemKind {
    Ntfs = 1,
    Refs = 2,
}

impl MinifilterFenceFilesystemKind {
    pub(super) fn from_wire(value: u16) -> io::Result<Self> {
        match value {
            1 => Ok(Self::Ntfs),
            2 => Ok(Self::Refs),
            _ => Err(invalid_data("NODE_MINIFILTER_FENCE_FILESYSTEM_INVALID")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum MinifilterFenceNameMatchMode {
    CaseInsensitive = 1,
    CaseSensitive = 2,
}

impl MinifilterFenceNameMatchMode {
    pub(super) fn from_wire(value: u16) -> io::Result<Self> {
        match value {
            1 => Ok(Self::CaseInsensitive),
            2 => Ok(Self::CaseSensitive),
            _ => Err(invalid_data("NODE_MINIFILTER_FENCE_NAME_MODE_INVALID")),
        }
    }
}

pub(super) fn is_zero<const N: usize>(value: &[u8; N]) -> bool {
    value.iter().all(|byte| *byte == 0)
}

pub(super) fn invalid_data(code: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, code)
}
