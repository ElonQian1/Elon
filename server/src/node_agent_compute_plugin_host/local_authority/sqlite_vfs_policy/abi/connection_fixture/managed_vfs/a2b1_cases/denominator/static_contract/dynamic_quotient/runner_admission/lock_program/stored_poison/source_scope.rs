//! Direct implementation seal for the exact stored-poison retention-succeeded programs.

use sha2::{Digest, Sha256};

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::lock_stored_poison_source_scope::LOCK_STORED_POISON_SOURCE_SCOPE_V1;
use super::LockStoredPoisonProfileV1;

pub(super) fn digest_implementation_v1(
    action: LockActionV1,
    first: u8,
    count: u8,
    profile: LockStoredPoisonProfileV1,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-stored-poison-retention-succeeded-implementation-v1\0");
    for &(name, source) in LOCK_STORED_POISON_SOURCE_SCOPE_V1 {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([action_tag_v1(action), first, count, profile.ordinal()]);
    Digest32(hasher.finalize().into())
}

const fn action_tag_v1(action: LockActionV1) -> u8 {
    match action {
        LockActionV1::LockShared => 1,
        LockActionV1::LockExclusive => 2,
        LockActionV1::UnlockShared => 3,
        LockActionV1::UnlockExclusive => 4,
    }
}
