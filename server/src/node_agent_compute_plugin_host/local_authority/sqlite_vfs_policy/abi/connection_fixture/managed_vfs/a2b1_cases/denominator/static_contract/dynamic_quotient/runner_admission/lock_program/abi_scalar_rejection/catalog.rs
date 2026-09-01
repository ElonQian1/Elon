//! Exact frozen-member authority for the seven Lock ABI scalar rejections.

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::{LockAbiScalarV1, ValidityV1};
use super::super::super::super::StaticMemberSealV1;
use super::super::LockRunnerExecutionViolationV1;
use super::ABI_SCALAR_REJECTION_MEMBER_COUNT;

const HEADER: &str = "offset\tcount\tflags\tcase_key_sha256\tfull_record_sha256";
const MEMBER_CATALOG: &str = include_str!("abi_scalar_rejection_members.v1.tsv");

const EXPECTED_ROWS_V1: [LockAbiScalarV1; ABI_SCALAR_REJECTION_MEMBER_COUNT] = [
    scalar(ValidityV1::Invalid, ValidityV1::Invalid, ValidityV1::Invalid),
    scalar(ValidityV1::Invalid, ValidityV1::Invalid, ValidityV1::Valid),
    scalar(ValidityV1::Invalid, ValidityV1::Valid, ValidityV1::Invalid),
    scalar(ValidityV1::Invalid, ValidityV1::Valid, ValidityV1::Valid),
    scalar(ValidityV1::Valid, ValidityV1::Invalid, ValidityV1::Invalid),
    scalar(ValidityV1::Valid, ValidityV1::Invalid, ValidityV1::Valid),
    scalar(ValidityV1::Valid, ValidityV1::Valid, ValidityV1::Invalid),
];

pub(super) fn exact_member_v1(
    scalar: LockAbiScalarV1,
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
        let Some(offset_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(count_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(flags_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(case_key_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if fields.next().is_some()
            || parse_validity_v1(offset_text) != Some(expected.offset)
            || parse_validity_v1(count_text) != Some(expected.count)
            || parse_validity_v1(flags_text) != Some(expected.flags)
        {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
        let Some(case_key_sha256) = parse_digest_v1(case_key_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_sha256) = parse_digest_v1(full_record_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if expected == scalar
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
    if observed_rows != ABI_SCALAR_REJECTION_MEMBER_COUNT {
        return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    selected.ok_or(LockRunnerExecutionViolationV1::MemberCatalogInvalid)
}

const fn scalar(
    offset: ValidityV1,
    count: ValidityV1,
    flags: ValidityV1,
) -> LockAbiScalarV1 {
    LockAbiScalarV1 {
        offset,
        count,
        flags,
    }
}

fn expected_row_v1(row: usize) -> Option<LockAbiScalarV1> {
    row.checked_sub(1)
        .and_then(|ordinal| EXPECTED_ROWS_V1.get(ordinal))
        .copied()
}

fn parse_validity_v1(value: &str) -> Option<ValidityV1> {
    match value.as_bytes() {
        b"invalid" => Some(ValidityV1::Invalid),
        b"valid" => Some(ValidityV1::Valid),
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
pub(super) const fn catalog_row_count_for_test() -> usize {
    ABI_SCALAR_REJECTION_MEMBER_COUNT
}
