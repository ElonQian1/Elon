//! Exact frozen-member authority for both stored-poison Lock retention completions.

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::StaticMemberSealV1;
use super::super::LockRunnerExecutionViolationV1;
use super::{
    range_mask_v1, LockStoredPoisonCompletionV1, LockStoredPoisonProfileV1,
    STORED_POISON_COMPLETION_MEMBER_COUNT, STORED_POISON_MEMBER_COUNT, STORED_POISON_PROFILES,
};

const HEADER: &str = "action\tfirst\tcount\tmask\tphase\tmutation\tlock_outcome_uncertain\tcase_key_sha256\tfull_record_sha256";
const RETENTION_SUCCEEDED_MEMBER_CATALOG: &str =
    include_str!("stored_poison_retention_succeeded_members.v1.tsv");
const RETENTION_ROUTE_UNKNOWN_MEMBER_CATALOG: &str =
    include_str!("stored_poison_retention_route_unknown_members.v1.tsv");
const PROFILE_COUNT: usize = STORED_POISON_PROFILES.len();
const REQUEST_COUNT: usize = 88;

pub(super) fn exact_member_v1(
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
    profile: LockStoredPoisonProfileV1,
    completion: LockStoredPoisonCompletionV1,
) -> Result<StaticMemberSealV1, LockRunnerExecutionViolationV1> {
    let catalog = match completion {
        LockStoredPoisonCompletionV1::RetentionSucceeded => RETENTION_SUCCEEDED_MEMBER_CATALOG,
        LockStoredPoisonCompletionV1::RetentionRouteUnknown => {
            RETENTION_ROUTE_UNKNOWN_MEMBER_CATALOG
        }
    };
    let mut lines = catalog.lines();
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
        let Some(phase_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(mutation_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(lock_uncertain_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(case_key_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if fields.next().is_some()
            || action_text != action_tag_v1(expected.action)
            || parse_canonical_u8(first_text) != Some(expected.first)
            || parse_canonical_u8(count_text) != Some(expected.count)
            || parse_mask_v1(mask_text) != Some(expected.mask)
            || phase_text != expected.profile.phase().static_name()
            || mutation_text != mutation_tag_v1(expected.profile)
            || parse_bool_v1(lock_uncertain_text) != Some(expected.profile.lock_outcome_uncertain())
        {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
        let Some(case_key_sha256) = parse_digest_v1(case_key_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_sha256) = parse_digest_v1(full_record_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if expected.action == action
            && expected.first == first
            && expected.count == count
            && expected.mask == mask
            && expected.profile == profile
            && selected
                .replace(StaticMemberSealV1 {
                    case_key_sha256,
                    full_record_sha256,
                })
                .is_some()
        {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
    }
    if observed_rows != STORED_POISON_COMPLETION_MEMBER_COUNT {
        return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    selected.ok_or(LockRunnerExecutionViolationV1::MemberCatalogInvalid)
}

#[derive(Clone, Copy)]
struct ExpectedRowV1 {
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
    profile: LockStoredPoisonProfileV1,
}

fn expected_row_v1(row: usize) -> Option<ExpectedRowV1> {
    if row == 0 || row > STORED_POISON_COMPLETION_MEMBER_COUNT {
        return None;
    }
    let zero_based = row - 1;
    let request_ordinal = zero_based / PROFILE_COUNT;
    let profile = STORED_POISON_PROFILES[zero_based % PROFILE_COUNT];
    let (action, first, count) = request_by_ordinal_v1(request_ordinal)?;
    Some(ExpectedRowV1 {
        action,
        first,
        count,
        mask: range_mask_v1(action, first, count)?,
        profile,
    })
}

fn request_by_ordinal_v1(ordinal: usize) -> Option<(LockActionV1, u8, u8)> {
    if ordinal >= REQUEST_COUNT {
        return None;
    }
    if ordinal < 8 {
        return Some((LockActionV1::LockShared, ordinal as u8, 1));
    }
    if ordinal < 44 {
        let (first, count) = exclusive_range_v1(ordinal - 8)?;
        return Some((LockActionV1::LockExclusive, first, count));
    }
    if ordinal < 52 {
        return Some((LockActionV1::UnlockShared, (ordinal - 44) as u8, 1));
    }
    let (first, count) = exclusive_range_v1(ordinal - 52)?;
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

pub(super) const fn action_tag_v1(action: LockActionV1) -> &'static str {
    match action {
        LockActionV1::LockShared => "lock-shared",
        LockActionV1::LockExclusive => "lock-exclusive",
        LockActionV1::UnlockShared => "unlock-shared",
        LockActionV1::UnlockExclusive => "unlock-exclusive",
    }
}

const fn mutation_tag_v1(profile: LockStoredPoisonProfileV1) -> &'static str {
    match profile.mutation() {
        super::super::super::super::super::source_leaf_authority::MutationStateV1::None => "none",
        super::super::super::super::super::source_leaf_authority::MutationStateV1::Uncertain => {
            "uncertain"
        }
        super::super::super::super::super::source_leaf_authority::MutationStateV1::Known => "known",
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

fn parse_bool_v1(value: &str) -> Option<bool> {
    match value.as_bytes() {
        b"false" => Some(false),
        b"true" => Some(true),
        _ => None,
    }
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
pub(super) fn catalog_row_count_for_test() -> usize {
    STORED_POISON_MEMBER_COUNT
}
