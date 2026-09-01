//! Exact frozen-member authority for callback-completion route-unknown outcomes.

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::StaticMemberSealV1;
use super::super::LockRunnerExecutionViolationV1;
use super::{
    range_mask_v1, LockCallbackCompletionRouteUnknownPathV1,
    CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT,
};

const HEADER: &str =
    "path\taction\tfirst\tcount\tmask\tcompletion\tcase_key_sha256\tfull_record_sha256";
const MEMBER_CATALOG: &str = include_str!("callback_completion_route_unknown_members.v1.tsv");

pub(super) fn exact_member_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> Result<StaticMemberSealV1, LockRunnerExecutionViolationV1> {
    let mut lines = MEMBER_CATALOG.lines();
    if lines.next() != Some(HEADER) {
        return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    let mut selected = None;
    let mut observed_rows = 0_usize;
    for line in lines {
        observed_rows += 1;
        let Some(expected) = expected_row_v1(observed_rows) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let mut fields = line.split('\t');
        let Some(path_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(action_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(first_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(count_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(mask_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(completion_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(case_key_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if fields.next().is_some()
            || path_text != path_tag_v1(expected.path)
            || action_text != action_tag_v1(expected.action)
            || parse_canonical_u8(first_text) != Some(expected.first)
            || parse_canonical_u8(count_text) != Some(expected.count)
            || parse_mask_v1(mask_text) != Some(expected.mask)
            || completion_text != "route-unknown-prior-quarantine"
        {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
        let Some(case_key_sha256) = parse_digest_v1(case_key_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_sha256) = parse_digest_v1(full_record_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if expected.path == path
            && expected.action == action
            && expected.first == first
            && expected.count == count
            && expected.mask == mask
        {
            if selected
                .replace(StaticMemberSealV1 {
                    case_key_sha256,
                    full_record_sha256,
                })
                .is_some()
            {
                return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
            }
        }
    }
    if observed_rows != CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT {
        return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    selected.ok_or(LockRunnerExecutionViolationV1::MemberCatalogInvalid)
}

#[derive(Clone, Copy)]
struct ExpectedRowV1 {
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
}

fn expected_row_v1(row: usize) -> Option<ExpectedRowV1> {
    if row == 0 || row > CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT {
        return None;
    }
    let ordinal = row - 1;
    let (path, path_ordinal) = if ordinal < 44 {
        (
            LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention,
            ordinal,
        )
    } else if ordinal < 88 {
        (
            LockCallbackCompletionRouteUnknownPathV1::NativeRelease,
            ordinal - 44,
        )
    } else if ordinal < 132 {
        (
            LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired,
            ordinal - 88,
        )
    } else if ordinal < 176 {
        (
            LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy,
            ordinal - 132,
        )
    } else if ordinal < 184 {
        (
            LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire,
            ordinal - 176,
        )
    } else {
        (
            LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease,
            ordinal - 184,
        )
    };
    let (action, first, count) = match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => {
            acquire_range_v1(path_ordinal)?
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => release_range_v1(path_ordinal)?,
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => {
            (LockActionV1::LockShared, path_ordinal as u8, 1)
        }
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => {
            (LockActionV1::UnlockShared, path_ordinal as u8, 1)
        }
    };
    Some(ExpectedRowV1 {
        path,
        action,
        first,
        count,
        mask: range_mask_v1(path, action, first, count)?,
    })
}

fn acquire_range_v1(ordinal: usize) -> Option<(LockActionV1, u8, u8)> {
    if ordinal < 8 {
        return Some((LockActionV1::LockShared, ordinal as u8, 1));
    }
    let (first, count) = exclusive_range_v1(ordinal - 8)?;
    Some((LockActionV1::LockExclusive, first, count))
}

fn release_range_v1(ordinal: usize) -> Option<(LockActionV1, u8, u8)> {
    if ordinal < 8 {
        return Some((LockActionV1::UnlockShared, ordinal as u8, 1));
    }
    let (first, count) = exclusive_range_v1(ordinal - 8)?;
    Some((LockActionV1::UnlockExclusive, first, count))
}

fn exclusive_range_v1(mut ordinal: usize) -> Option<(u8, u8)> {
    for count in 1..=8_u8 {
        let width = usize::from(9 - count);
        if ordinal < width {
            return Some((ordinal as u8, count));
        }
        ordinal -= width;
    }
    None
}

pub(super) const fn path_tag_v1(path: LockCallbackCompletionRouteUnknownPathV1) -> &'static str {
    match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention => {
            "local-sibling-contention"
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => "native-release",
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired => {
            "native-acquire-acquired"
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => "native-acquire-busy",
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => "shared-local-acquire",
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => "shared-local-release",
    }
}

const fn action_tag_v1(action: LockActionV1) -> &'static str {
    match action {
        LockActionV1::LockShared => "lock-shared",
        LockActionV1::LockExclusive => "lock-exclusive",
        LockActionV1::UnlockShared => "unlock-shared",
        LockActionV1::UnlockExclusive => "unlock-exclusive",
    }
}

fn parse_canonical_u8(value: &str) -> Option<u8> {
    let parsed = value.parse::<u8>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

fn parse_mask_v1(value: &str) -> Option<u8> {
    if value.len() != 2
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    u8::from_str_radix(value, 16).ok()
}

fn parse_digest_v1(value: &str) -> Option<Digest32> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, slot) in digest.iter_mut().enumerate() {
        let high = hex_nibble(bytes[index * 2])?;
        let low = hex_nibble(bytes[index * 2 + 1])?;
        *slot = (high << 4) | low;
    }
    Some(Digest32(digest))
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
pub(super) const fn catalog_row_count_for_test() -> usize {
    CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
}
