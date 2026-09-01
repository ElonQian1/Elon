//! Exact frozen-member authority for the eleven q11 Lock raw-state rejection cases.

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::StaticMemberSealV1;
use super::super::LockRunnerExecutionViolationV1;
use super::case::LockRawStateRejectionCaseV1;
use super::RAW_STATE_REJECTION_MEMBER_COUNT;
use super::super::super::super::super::terminal_descriptor::{
    LockCompletionV1, RawStateV1, SourceSiteV1,
};

const HEADER: &str =
    "source_site\traw_state\tcompletion\tcase_key_sha256\tfull_record_sha256";
const MEMBER_CATALOG: &str = include_str!("raw_state_rejection_members.v1.tsv");
const EXPECTED_ROWS_V1: [LockRawStateRejectionCaseV1; RAW_STATE_REJECTION_MEMBER_COUNT] =
    LockRawStateRejectionCaseV1::ALL_V1;

pub(super) fn exact_member_v1(
    rejection: LockRawStateRejectionCaseV1,
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
        let Some(source_site_text) = fields.next() else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(raw_state_text) = fields.next() else {
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
        let Some(row) = parse_case_v1(raw_state_text, completion_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if fields.next().is_some()
            || parse_source_site_v1(source_site_text) != Some(row.source_site_v1())
            || row != expected
        {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
        let Some(case_key_sha256) = parse_digest_v1(case_key_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_sha256) = parse_digest_v1(full_record_text) else {
            return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if row == rejection
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
    if observed_rows != RAW_STATE_REJECTION_MEMBER_COUNT {
        return Err(LockRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    selected.ok_or(LockRunnerExecutionViolationV1::MemberCatalogInvalid)
}

fn expected_row_v1(row: usize) -> Option<LockRawStateRejectionCaseV1> {
    row.checked_sub(1)
        .and_then(|ordinal| EXPECTED_ROWS_V1.get(ordinal))
        .copied()
}

fn parse_source_site_v1(value: &str) -> Option<SourceSiteV1> {
    match value.as_bytes() {
        b"raw-state-abandon" => Some(SourceSiteV1::RawStateAbandon),
        b"adapter-dispatch" => Some(SourceSiteV1::AdapterDispatch),
        _ => None,
    }
}

fn parse_case_v1(raw: &str, completion: &str) -> Option<LockRawStateRejectionCaseV1> {
    LockRawStateRejectionCaseV1::from_typed_v1(
        parse_raw_state_v1(raw)?,
        parse_completion_v1(completion)?,
    )
}

fn parse_raw_state_v1(value: &str) -> Option<RawStateV1> {
    match value.as_bytes() {
        b"null-file" => Some(RawStateV1::NullFile),
        b"uninstalled" => Some(RawStateV1::Uninstalled),
        b"methods-null-state-present" => Some(RawStateV1::MethodsNullStatePresent),
        b"foreign-methods-state-null" => Some(RawStateV1::ForeignMethodsStateNull),
        b"foreign-methods-state-present" => Some(RawStateV1::ForeignMethodsStatePresent),
        b"exact-methods-state-null" => Some(RawStateV1::ExactMethodsStateNull),
        b"other-type-payload-missing" => Some(RawStateV1::OtherTypePayloadMissing),
        b"other-type-payload-present" => Some(RawStateV1::OtherTypePayloadPresent),
        b"expected-type-payload-missing" => Some(RawStateV1::ExpectedTypePayloadMissing),
        b"handle-bound-file-missing" => Some(RawStateV1::HandleBoundFileMissing),
        _ => None,
    }
}

fn parse_completion_v1(value: &str) -> Option<LockCompletionV1> {
    match value.as_bytes() {
        b"direct" => Some(LockCompletionV1::Direct),
        b"raw-drop-completed" => Some(LockCompletionV1::RawDropCompleted),
        b"raw-drop-unwind-caught" => Some(LockCompletionV1::RawDropUnwindCaught),
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
    RAW_STATE_REJECTION_MEMBER_COUNT
}
