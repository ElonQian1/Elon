//! Domain-separated implementation seals for exact callback route-unknown outcomes.

use sha2::{Digest, Sha256};

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::lock_callback_completion_route_unknown_source_scope::LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_SOURCE_SCOPE_V1;
use super::LockCallbackCompletionRouteUnknownPathV1;

pub(super) fn digest_implementation_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-callback-completion-route-unknown-implementation-v1\0");
    for &(name, source) in LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_SOURCE_SCOPE_V1 {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([path_tag_v1(path), action_tag_v1(action), first, count, mask]);
    Digest32(hasher.finalize().into())
}

const fn path_tag_v1(path: LockCallbackCompletionRouteUnknownPathV1) -> u8 {
    match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention => 1,
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => 2,
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired => 3,
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => 4,
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => 5,
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => 6,
    }
}

const fn action_tag_v1(action: LockActionV1) -> u8 {
    match action {
        LockActionV1::LockShared => 1,
        LockActionV1::LockExclusive => 2,
        LockActionV1::UnlockShared => 3,
        LockActionV1::UnlockExclusive => 4,
    }
}
