//! Exact member authority for the two bounded Map region-loop success families.
//!
//! The checked-in table intentionally contains no leaf ids or display text. Admission first
//! classifies the complete typed descriptor and expected vector, then resolves one exact frozen
//! member pair by family and `regions_to_create`.

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::StaticMemberSealV1;
use super::super::MapRunnerExecutionViolationV1;
use super::{MapRegionLoopFamilyV1, EMPTY_EXTEND_COUNT, MISSING_EXTEND_COUNT};

const HEADER: &str = "family\tregions_to_create\tcase_key_sha256\tfull_record_sha256";
const MEMBER_CATALOG: &str = include_str!("region_loop_members.v1.tsv");
const CATALOG_ROW_COUNT: usize = EMPTY_EXTEND_COUNT as usize + MISSING_EXTEND_COUNT as usize;

pub(super) fn exact_member_v1(
    family: MapRegionLoopFamilyV1,
    regions_to_create: u16,
) -> Result<StaticMemberSealV1, MapRunnerExecutionViolationV1> {
    if regions_to_create == 0 || regions_to_create > family.max_regions() {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    let mut lines = MEMBER_CATALOG.lines();
    if lines.next() != Some(HEADER) {
        return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    let mut selected = None;
    let mut observed_rows = 0_usize;
    for line in lines {
        observed_rows += 1;
        let Some((expected_family, expected_ordinal)) = expected_row_v1(observed_rows) else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let mut fields = line.split('\t');
        let Some(family_tag) = fields.next() else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(ordinal_text) = fields.next() else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(case_key_text) = fields.next() else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_text) = fields.next() else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if fields.next().is_some()
            || family_tag != expected_family.tag()
            || parse_canonical_u16(ordinal_text) != Some(expected_ordinal)
        {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        }
        let Some(case_key_sha256) = parse_digest_v1(case_key_text) else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        let Some(full_record_sha256) = parse_digest_v1(full_record_text) else {
            return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
        };
        if expected_family == family && expected_ordinal == regions_to_create {
            if selected
                .replace(StaticMemberSealV1 {
                    case_key_sha256,
                    full_record_sha256,
                })
                .is_some()
            {
                return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
            }
        }
    }
    if observed_rows != CATALOG_ROW_COUNT {
        return Err(MapRunnerExecutionViolationV1::MemberCatalogInvalid);
    }
    selected.ok_or(MapRunnerExecutionViolationV1::MemberCatalogInvalid)
}

const fn expected_row_v1(row: usize) -> Option<(MapRegionLoopFamilyV1, u16)> {
    if row == 0 || row > CATALOG_ROW_COUNT {
        return None;
    }
    if row <= EMPTY_EXTEND_COUNT as usize {
        return Some((MapRegionLoopFamilyV1::EmptyExtend, row as u16));
    }
    Some((
        MapRegionLoopFamilyV1::MissingExtend,
        (row - EMPTY_EXTEND_COUNT as usize) as u16,
    ))
}

fn parse_canonical_u16(value: &str) -> Option<u16> {
    let parsed = value.parse::<u16>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
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
    CATALOG_ROW_COUNT
}
