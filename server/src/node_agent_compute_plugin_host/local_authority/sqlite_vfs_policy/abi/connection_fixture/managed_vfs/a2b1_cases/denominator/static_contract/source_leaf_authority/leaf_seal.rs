//! Fixed-size per-leaf seals and their checked-in TSV representation.

use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};

use super::{
    canonical,
    model::{Digest32, LeafOutcomeV1, LeafRecordV1, RootOperationV1},
};

pub(crate) const LEAF_SEAL_TSV_HEADER_V1: &str =
    concat!("root\tleaf_id\toutcome\tshard\tcase_key_sha256\tfull_record_sha256");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LeafSealOutcomeV1 {
    Terminal,
    Excluded,
}

impl LeafSealOutcomeV1 {
    const fn canonical_name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Excluded => "excluded",
        }
    }
}

/// A leaf seal retains identity only as root + stable leaf id and fixed-size digests.  It never
/// retains the decision path, source branch, Expected vector, or exclusion proof.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LeafSealV1 {
    pub(crate) root: RootOperationV1,
    pub(crate) leaf_id: String,
    pub(crate) outcome: LeafSealOutcomeV1,
    pub(crate) shard: u8,
    pub(crate) source_leaf_identity_sha256: Digest32,
    pub(crate) case_key_sha256: Digest32,
    pub(crate) source_branch_sha256: Digest32,
    pub(crate) expected_sha256: Option<Digest32>,
    pub(crate) exclusion_sha256: Option<Digest32>,
    pub(crate) full_record_sha256: Digest32,
}

impl LeafSealV1 {
    pub(super) fn from_record(record: &LeafRecordV1) -> Self {
        let digests = canonical::precompute_record_digests(record);
        Self::from_record_digests(record, digests)
    }

    pub(super) fn from_record_cached(
        record: &LeafRecordV1,
        cache: &mut canonical::PrimitiveDigestCacheV1,
    ) -> Self {
        let digests = canonical::precompute_record_digests_cached(record, cache);
        Self::from_record_digests(record, digests)
    }

    fn from_record_digests(
        record: &LeafRecordV1,
        digests: canonical::PrecomputedRecordDigestsV1,
    ) -> Self {
        let outcome = match &record.outcome {
            LeafOutcomeV1::Terminal(_) => LeafSealOutcomeV1::Terminal,
            LeafOutcomeV1::Excluded(_) => LeafSealOutcomeV1::Excluded,
        };
        Self {
            root: record.key.identity.root,
            leaf_id: record.key.identity.leaf_id.clone(),
            outcome,
            shard: digests.source_leaf_identity_sha256.0[0],
            source_leaf_identity_sha256: digests.source_leaf_identity_sha256,
            case_key_sha256: digests.case_key_sha256,
            source_branch_sha256: digests.source_branch_sha256,
            expected_sha256: digests.expected_sha256,
            exclusion_sha256: digests.exclusion_sha256,
            full_record_sha256: digests.full_record_sha256,
        }
    }

    pub(crate) fn to_tsv_row(&self) -> Result<String, String> {
        if self.leaf_id.is_empty()
            || self
                .leaf_id
                .bytes()
                .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        {
            return Err("leaf seal has a non-TSV-safe leaf id".to_owned());
        }
        validate_outcome_digests(self)?;
        Ok(format!(
            "{}\t{}\t{}\t{:03}\t{}\t{}",
            self.root.canonical_name(),
            self.leaf_id,
            self.outcome.canonical_name(),
            self.shard,
            self.case_key_sha256.to_lower_hex(),
            self.full_record_sha256.to_lower_hex(),
        ))
    }
}

/// Produces a canonical, root/leaf-sorted TSV.  Only fixed-size seals are sorted; graph paths are
/// never retained by this representation.
pub(crate) fn encode_leaf_seal_tsv(seals: &[LeafSealV1]) -> Result<String, String> {
    if seals.is_empty() {
        return Err("cannot encode an empty leaf-seal ledger".to_owned());
    }
    let mut sorted = seals.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        (left.root, left.leaf_id.as_str()).cmp(&(right.root, right.leaf_id.as_str()))
    });
    if sorted.windows(2).any(|pair| {
        (pair[0].root, pair[0].leaf_id.as_str()) == (pair[1].root, pair[1].leaf_id.as_str())
    }) {
        return Err("leaf-seal ledger repeats a root/leaf identity".to_owned());
    }
    let mut result = String::new();
    result.push_str(LEAF_SEAL_TSV_HEADER_V1);
    result.push('\n');
    for seal in sorted {
        result.push_str(&seal.to_tsv_row()?);
        result.push('\n');
    }
    Ok(result)
}

pub(crate) fn parse_leaf_seal_tsv(input: &str) -> Result<Vec<FrozenLeafSealV1>, String> {
    if input.starts_with('\u{feff}') {
        return Err("leaf-seal TSV must be UTF-8 without BOM".to_owned());
    }
    let mut lines = input.lines();
    if lines.next() != Some(LEAF_SEAL_TSV_HEADER_V1) {
        return Err("leaf-seal TSV header/schema mismatch".to_owned());
    }
    let mut seals = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.is_empty() {
            return Err(format!(
                "leaf-seal TSV has an empty row at line {}",
                offset + 2
            ));
        }
        seals.push(parse_row(line).map_err(|error| format!("line {}: {error}", offset + 2))?);
    }
    if seals.is_empty() {
        return Err("leaf-seal TSV contains no rows".to_owned());
    }
    if seals.windows(2).any(|pair| {
        (pair[0].root, pair[0].leaf_id.as_str()) >= (pair[1].root, pair[1].leaf_id.as_str())
    }) {
        return Err("leaf-seal TSV rows are duplicate or not canonically sorted".to_owned());
    }
    if encode_frozen_leaf_seal_tsv(&seals) != input {
        return Err(
            "leaf-seal TSV bytes are not the canonical LF/trailing-newline form".to_owned(),
        );
    }
    Ok(seals)
}

pub(crate) fn leaf_seal_tsv_sha256(input: &str) -> Result<Digest32, String> {
    parse_leaf_seal_tsv(input)?;
    Ok(Digest32(Sha256::digest(input.as_bytes()).into()))
}

/// Compares a graph stream against a checked-in per-leaf TSV without retaining graph paths.
pub(crate) struct FrozenLeafSealVerifierV1 {
    remaining: BTreeMap<(RootOperationV1, String), FrozenLeafSealV1>,
    observed: u64,
}

impl FrozenLeafSealVerifierV1 {
    pub(crate) fn from_tsv(input: &str, expected_ledger_sha256: Digest32) -> Result<Self, String> {
        let actual_ledger_sha256 = leaf_seal_tsv_sha256(input)?;
        if actual_ledger_sha256 != expected_ledger_sha256 {
            return Err(format!(
                "canonical leaf TSV SHA-256 is not the manifest ledger binding; expected={}, actual={}",
                expected_ledger_sha256.to_lower_hex(),
                actual_ledger_sha256.to_lower_hex()
            ));
        }
        let remaining = parse_leaf_seal_tsv(input)?
            .into_iter()
            .map(|seal| ((seal.root, seal.leaf_id.clone()), seal))
            .collect::<BTreeMap<_, _>>();
        Ok(Self {
            remaining,
            observed: 0,
        })
    }

    pub(crate) fn observe(&mut self, actual: &LeafSealV1) -> Result<(), String> {
        let key = (actual.root, actual.leaf_id.clone());
        let frozen = self.remaining.remove(&key).ok_or_else(|| {
            format!(
                "graph emitted an unfrozen source leaf: {}::{}",
                actual.root.canonical_name(),
                actual.leaf_id
            )
        })?;
        if !frozen.matches(actual) {
            return Err(format!(
                "source-leaf seal drifted for {}::{}; frozen={frozen:?}, actual={actual:?}",
                actual.root.canonical_name(),
                actual.leaf_id
            ));
        }
        self.observed += 1;
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<u64, String> {
        if self.remaining.is_empty() {
            return Ok(self.observed);
        }
        let missing = self
            .remaining
            .keys()
            .take(8)
            .map(|(root, leaf)| format!("{}::{leaf}", root.canonical_name()))
            .collect::<Vec<_>>();
        Err(format!(
            "graph omitted {} frozen source leaves; first={missing:?}",
            self.remaining.len()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FrozenLeafSealV1 {
    pub(crate) root: RootOperationV1,
    pub(crate) leaf_id: String,
    pub(crate) outcome: LeafSealOutcomeV1,
    pub(crate) shard: u8,
    pub(crate) case_key_sha256: Digest32,
    pub(crate) full_record_sha256: Digest32,
}

impl FrozenLeafSealV1 {
    fn matches(&self, actual: &LeafSealV1) -> bool {
        self.root == actual.root
            && self.leaf_id == actual.leaf_id
            && self.outcome == actual.outcome
            && self.shard == actual.shard
            && self.case_key_sha256 == actual.case_key_sha256
            && self.full_record_sha256 == actual.full_record_sha256
    }

    fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{:03}\t{}\t{}",
            self.root.canonical_name(),
            self.leaf_id,
            self.outcome.canonical_name(),
            self.shard,
            self.case_key_sha256.to_lower_hex(),
            self.full_record_sha256.to_lower_hex(),
        )
    }
}

fn parse_row(line: &str) -> Result<FrozenLeafSealV1, String> {
    let fields = line.split('\t').collect::<Vec<_>>();
    let [root, leaf_id, outcome, shard, case_key, full] = fields.as_slice() else {
        return Err("leaf-seal row must contain exactly 6 columns".to_owned());
    };
    let root = match *root {
        "map" => RootOperationV1::Map,
        "lock" => RootOperationV1::Lock,
        _ => return Err("leaf-seal root is not map or lock".to_owned()),
    };
    if leaf_id.is_empty() {
        return Err("leaf-seal leaf_id is empty".to_owned());
    }
    let outcome = match *outcome {
        "terminal" => LeafSealOutcomeV1::Terminal,
        "excluded" => LeafSealOutcomeV1::Excluded,
        _ => return Err("leaf-seal outcome is not terminal or excluded".to_owned()),
    };
    if shard.len() != 3 {
        return Err("leaf-seal shard must use three decimal digits".to_owned());
    }
    let shard = shard
        .parse::<u8>()
        .map_err(|_| "leaf-seal shard is outside 000..255".to_owned())?;
    let seal = FrozenLeafSealV1 {
        root,
        leaf_id: (*leaf_id).to_owned(),
        outcome,
        shard,
        case_key_sha256: parse_digest(case_key)?,
        full_record_sha256: parse_digest(full)?,
    };
    Ok(seal)
}

fn encode_frozen_leaf_seal_tsv(seals: &[FrozenLeafSealV1]) -> String {
    let mut result = String::new();
    result.push_str(LEAF_SEAL_TSV_HEADER_V1);
    result.push('\n');
    for seal in seals {
        result.push_str(&seal.to_tsv_row());
        result.push('\n');
    }
    result
}

fn validate_outcome_digests(seal: &LeafSealV1) -> Result<(), String> {
    match (seal.outcome, seal.expected_sha256, seal.exclusion_sha256) {
        (LeafSealOutcomeV1::Terminal, Some(_), None)
        | (LeafSealOutcomeV1::Excluded, None, Some(_)) => Ok(()),
        _ => Err("leaf seal does not contain exactly its outcome digest".to_owned()),
    }
}

pub(super) fn optional_digest(value: Option<Digest32>) -> String {
    value.map_or_else(|| "-".to_owned(), Digest32::to_lower_hex)
}

pub(super) fn parse_optional_digest(value: &str) -> Result<Option<Digest32>, String> {
    if value == "-" {
        Ok(None)
    } else {
        parse_digest(value).map(Some)
    }
}

pub(super) fn parse_digest(value: &str) -> Result<Digest32, String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("leaf-seal digest is not lowercase SHA-256".to_owned());
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(Digest32(bytes))
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err("leaf-seal digest contains a non-hex byte".to_owned()),
    }
}
