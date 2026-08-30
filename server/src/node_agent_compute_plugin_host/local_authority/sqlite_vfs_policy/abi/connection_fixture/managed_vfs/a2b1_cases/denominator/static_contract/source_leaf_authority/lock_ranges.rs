//! Literal Lock action/range authority.
//!
//! All 88 action/range/mask rows are written here independently. This module must never call the
//! graph's `representatives`, `derived_mask`, request constructor or lock-outcome helper.

use std::collections::{BTreeMap, BTreeSet};

use super::{canonical, model::Digest32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockActionV1 {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

impl LockActionV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::LockShared => "lock-shared",
            Self::LockExclusive => "lock-exclusive",
            Self::UnlockShared => "unlock-shared",
            Self::UnlockExclusive => "unlock-exclusive",
        }
    }

    pub(crate) const fn is_shared(self) -> bool {
        matches!(self, Self::LockShared | Self::UnlockShared)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LockRangeV1 {
    pub(crate) action: LockActionV1,
    pub(crate) first: u8,
    pub(crate) count: u8,
    pub(crate) mask: u8,
}

macro_rules! range {
    ($action:ident, $first:literal, $count:literal, $mask:literal) => {
        LockRangeV1 {
            action: LockActionV1::$action,
            first: $first,
            count: $count,
            mask: $mask,
        }
    };
}

pub(crate) const LOCK_RANGES: &[LockRangeV1; 88] = &[
    range!(LockShared, 0, 1, 0x01),
    range!(LockShared, 1, 1, 0x02),
    range!(LockShared, 2, 1, 0x04),
    range!(LockShared, 3, 1, 0x08),
    range!(LockShared, 4, 1, 0x10),
    range!(LockShared, 5, 1, 0x20),
    range!(LockShared, 6, 1, 0x40),
    range!(LockShared, 7, 1, 0x80),
    range!(UnlockShared, 0, 1, 0x01),
    range!(UnlockShared, 1, 1, 0x02),
    range!(UnlockShared, 2, 1, 0x04),
    range!(UnlockShared, 3, 1, 0x08),
    range!(UnlockShared, 4, 1, 0x10),
    range!(UnlockShared, 5, 1, 0x20),
    range!(UnlockShared, 6, 1, 0x40),
    range!(UnlockShared, 7, 1, 0x80),
    range!(LockExclusive, 0, 1, 0x01),
    range!(LockExclusive, 1, 1, 0x02),
    range!(LockExclusive, 2, 1, 0x04),
    range!(LockExclusive, 3, 1, 0x08),
    range!(LockExclusive, 4, 1, 0x10),
    range!(LockExclusive, 5, 1, 0x20),
    range!(LockExclusive, 6, 1, 0x40),
    range!(LockExclusive, 7, 1, 0x80),
    range!(LockExclusive, 0, 2, 0x03),
    range!(LockExclusive, 1, 2, 0x06),
    range!(LockExclusive, 2, 2, 0x0c),
    range!(LockExclusive, 3, 2, 0x18),
    range!(LockExclusive, 4, 2, 0x30),
    range!(LockExclusive, 5, 2, 0x60),
    range!(LockExclusive, 6, 2, 0xc0),
    range!(LockExclusive, 0, 3, 0x07),
    range!(LockExclusive, 1, 3, 0x0e),
    range!(LockExclusive, 2, 3, 0x1c),
    range!(LockExclusive, 3, 3, 0x38),
    range!(LockExclusive, 4, 3, 0x70),
    range!(LockExclusive, 5, 3, 0xe0),
    range!(LockExclusive, 0, 4, 0x0f),
    range!(LockExclusive, 1, 4, 0x1e),
    range!(LockExclusive, 2, 4, 0x3c),
    range!(LockExclusive, 3, 4, 0x78),
    range!(LockExclusive, 4, 4, 0xf0),
    range!(LockExclusive, 0, 5, 0x1f),
    range!(LockExclusive, 1, 5, 0x3e),
    range!(LockExclusive, 2, 5, 0x7c),
    range!(LockExclusive, 3, 5, 0xf8),
    range!(LockExclusive, 0, 6, 0x3f),
    range!(LockExclusive, 1, 6, 0x7e),
    range!(LockExclusive, 2, 6, 0xfc),
    range!(LockExclusive, 0, 7, 0x7f),
    range!(LockExclusive, 1, 7, 0xfe),
    range!(LockExclusive, 0, 8, 0xff),
    range!(UnlockExclusive, 0, 1, 0x01),
    range!(UnlockExclusive, 1, 1, 0x02),
    range!(UnlockExclusive, 2, 1, 0x04),
    range!(UnlockExclusive, 3, 1, 0x08),
    range!(UnlockExclusive, 4, 1, 0x10),
    range!(UnlockExclusive, 5, 1, 0x20),
    range!(UnlockExclusive, 6, 1, 0x40),
    range!(UnlockExclusive, 7, 1, 0x80),
    range!(UnlockExclusive, 0, 2, 0x03),
    range!(UnlockExclusive, 1, 2, 0x06),
    range!(UnlockExclusive, 2, 2, 0x0c),
    range!(UnlockExclusive, 3, 2, 0x18),
    range!(UnlockExclusive, 4, 2, 0x30),
    range!(UnlockExclusive, 5, 2, 0x60),
    range!(UnlockExclusive, 6, 2, 0xc0),
    range!(UnlockExclusive, 0, 3, 0x07),
    range!(UnlockExclusive, 1, 3, 0x0e),
    range!(UnlockExclusive, 2, 3, 0x1c),
    range!(UnlockExclusive, 3, 3, 0x38),
    range!(UnlockExclusive, 4, 3, 0x70),
    range!(UnlockExclusive, 5, 3, 0xe0),
    range!(UnlockExclusive, 0, 4, 0x0f),
    range!(UnlockExclusive, 1, 4, 0x1e),
    range!(UnlockExclusive, 2, 4, 0x3c),
    range!(UnlockExclusive, 3, 4, 0x78),
    range!(UnlockExclusive, 4, 4, 0xf0),
    range!(UnlockExclusive, 0, 5, 0x1f),
    range!(UnlockExclusive, 1, 5, 0x3e),
    range!(UnlockExclusive, 2, 5, 0x7c),
    range!(UnlockExclusive, 3, 5, 0xf8),
    range!(UnlockExclusive, 0, 6, 0x3f),
    range!(UnlockExclusive, 1, 6, 0x7e),
    range!(UnlockExclusive, 2, 6, 0xfc),
    range!(UnlockExclusive, 0, 7, 0x7f),
    range!(UnlockExclusive, 1, 7, 0xfe),
    range!(UnlockExclusive, 0, 8, 0xff),
];

pub(crate) fn validate_lock_ranges() -> Result<(), String> {
    let mut actual = BTreeSet::new();
    let mut counts = BTreeMap::new();
    for range in LOCK_RANGES {
        if range.count == 0
            || range
                .first
                .checked_add(range.count)
                .is_none_or(|end| end > 8)
            || (range.action.is_shared() && range.count != 1)
            || range.mask != expected_mask(range.first, range.count)
            || !actual.insert((range.action, range.first, range.count))
        {
            return Err(format!(
                "invalid or duplicate Lock authority range {range:?}"
            ));
        }
        *counts.entry(range.action).or_insert(0_usize) += 1;
    }
    let expected = expected_action_ranges();
    if actual != expected
        || counts.get(&LockActionV1::LockShared) != Some(&8)
        || counts.get(&LockActionV1::UnlockShared) != Some(&8)
        || counts.get(&LockActionV1::LockExclusive) != Some(&36)
        || counts.get(&LockActionV1::UnlockExclusive) != Some(&36)
    {
        return Err("Lock authority is not the exact 8/36/8/36 range partition".to_owned());
    }
    Ok(())
}

fn expected_action_ranges() -> BTreeSet<(LockActionV1, u8, u8)> {
    let mut expected = BTreeSet::new();
    for action in [LockActionV1::LockShared, LockActionV1::UnlockShared] {
        for first in 0..8 {
            expected.insert((action, first, 1));
        }
    }
    for action in [LockActionV1::LockExclusive, LockActionV1::UnlockExclusive] {
        for count in 1..=8 {
            for first in 0..=(8 - count) {
                expected.insert((action, first, count));
            }
        }
    }
    expected
}

fn expected_mask(first: u8, count: u8) -> u8 {
    let low = 1_u16 << first;
    let high = 1_u16 << (first + count);
    (high - low) as u8
}

pub(crate) fn lock_range_set_sha256() -> Digest32 {
    canonical::digest_lock_range_set(LOCK_RANGES)
}
