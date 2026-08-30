//! Bounded-memory manifest accumulation.
//!
//! Paths and source branches are consumed one record at a time.  The retained working set is
//! limited to fixed-size digests, two uniqueness sets, and 256 shard accumulators; no complete
//! decision path or source branch is retained after `push` returns.

use std::collections::{BTreeSet, HashSet};

use super::{
    canonical,
    leaf_seal::LeafSealV1,
    model::{
        Digest32, LeafOutcomeV1, LeafRecordV1, ManifestContextV1, RootManifestV1, RootOperationV1,
        ShardManifestV1,
    },
    AUTHORITY_SCHEMA_V1, MANIFEST_SHARDS, PRODUCTION_WINDOWS_X64_SCOPE,
};

const IDENTITY_SET_DOMAIN: &str = "ELON-A2B1-VFS-SOURCE-LEAF-IDENTITY-SET-V1";
const CASE_KEY_SET_DOMAIN: &str = "ELON-A2B1-VFS-CASE-KEY-SET-V1";
const SOURCE_BRANCH_MAP_DOMAIN: &str = "ELON-A2B1-VFS-SOURCE-BRANCH-MAP-V1";
const EXPECTED_MAP_DOMAIN: &str = "ELON-A2B1-VFS-EXPECTED-MAP-V1";
const EXCLUSION_MAP_DOMAIN: &str = "ELON-A2B1-VFS-EXCLUSION-MAP-V1";
const FULL_RECORD_SET_DOMAIN: &str = "ELON-A2B1-VFS-FULL-RECORD-SET-V1";

pub(crate) struct ManifestAccumulatorV1 {
    context: ManifestContextV1,
    seen_case_keys: HashSet<Digest32>,
    seen_identities: HashSet<Digest32>,
    primitive_digest_cache: canonical::PrimitiveDigestCacheV1,
    global: DigestAccumulator,
    shards: Vec<DigestAccumulator>,
}

impl ManifestAccumulatorV1 {
    pub(crate) fn new(context: ManifestContextV1) -> Result<Self, String> {
        validate_context(&context)?;
        Ok(Self {
            context,
            seen_case_keys: HashSet::new(),
            seen_identities: HashSet::new(),
            primitive_digest_cache: canonical::PrimitiveDigestCacheV1::default(),
            global: DigestAccumulator::default(),
            shards: (0..MANIFEST_SHARDS)
                .map(|_| DigestAccumulator::default())
                .collect(),
        })
    }

    pub(crate) fn push(&mut self, record: &LeafRecordV1) -> Result<LeafSealV1, String> {
        validate_record(&self.context, record)?;
        let seal = LeafSealV1::from_record_cached(record, &mut self.primitive_digest_cache);
        if !self.seen_case_keys.insert(seal.case_key_sha256) {
            return Err(format!(
                "source-leaf stream repeats a CaseKey digest: {}",
                seal.case_key_sha256.to_lower_hex()
            ));
        }
        if !self
            .seen_identities
            .insert(seal.source_leaf_identity_sha256)
        {
            return Err(format!(
                "source-leaf stream gives one identity two paths: {}",
                seal.source_leaf_identity_sha256.to_lower_hex()
            ));
        }
        self.global.add_seal(&seal);
        self.shards[usize::from(seal.shard)].add_seal(&seal);
        Ok(seal)
    }

    pub(crate) fn finish(self) -> Result<RootManifestV1, String> {
        if self.seen_case_keys.is_empty() {
            return Err("source-leaf stream contains no records".to_owned());
        }
        let global = self.global.finish();
        let shards = self
            .shards
            .into_iter()
            .enumerate()
            .map(|(index, accumulator)| accumulator.finish_shard(index as u8))
            .collect::<Vec<_>>();
        let mut manifest = RootManifestV1 {
            context: self.context,
            included_count: global.included_count,
            excluded_count: global.excluded_count,
            source_leaf_identity_set_sha256: global.source_leaf_identity_set_sha256,
            case_key_set_sha256: global.case_key_set_sha256,
            source_branch_map_sha256: global.source_branch_map_sha256,
            expected_map_sha256: global.expected_map_sha256,
            exclusion_map_sha256: global.exclusion_map_sha256,
            full_record_set_sha256: global.full_record_set_sha256,
            shards,
            manifest_sha256: Digest32::ZERO,
        };
        manifest.manifest_sha256 = canonical::digest_manifest_body(&manifest);
        Ok(manifest)
    }
}

pub(super) fn validate_context(context: &ManifestContextV1) -> Result<(), String> {
    if context.schema != AUTHORITY_SCHEMA_V1
        || context.target_scope != PRODUCTION_WINDOWS_X64_SCOPE
        || !lower_hex(&context.source_baseline_commit_sha1, 40)
        || context.source_scope_sha256 == Digest32::ZERO
        || context.ledger_sha256 == Digest32::ZERO
    {
        return Err("authority manifest context is unbound or malformed".to_owned());
    }
    match context.root {
        RootOperationV1::Map
            if context.map_profile_set_sha256.is_some()
                && context.map_ordinal_domain_sha256.is_some()
                && context.lock_range_set_sha256.is_none()
                && context.lock_range_count.is_none() =>
        {
            Ok(())
        }
        RootOperationV1::Lock
            if context.map_profile_set_sha256.is_none()
                && context.map_ordinal_domain_sha256.is_none()
                && context.lock_range_set_sha256.is_some()
                && context.lock_range_count == Some(88) =>
        {
            Ok(())
        }
        _ => Err("authority manifest has the wrong Map/Lock domain binding".to_owned()),
    }
}

fn validate_record(context: &ManifestContextV1, record: &LeafRecordV1) -> Result<(), String> {
    let identity = &record.key.identity;
    if identity.root != context.root
        || identity.leaf_id.is_empty()
        || identity.family_id.is_empty()
        || record.key.decisions.is_empty()
        || record.source_branch.is_empty()
    {
        return Err(format!(
            "authority record has an incomplete identity/path: {identity:?}"
        ));
    }
    let mut coordinates = BTreeSet::new();
    if identity.coordinates.iter().any(|coordinate| {
        coordinate.name.is_empty()
            || coordinate.value.is_empty()
            || !coordinates.insert(coordinate.name.as_str())
    }) || identity
        .coordinates
        .windows(2)
        .any(|pair| pair[0].name >= pair[1].name)
    {
        return Err(format!(
            "authority record has invalid or noncanonical coordinates: {identity:?}"
        ));
    }
    if record
        .key
        .decisions
        .iter()
        .any(|decision| decision.branch.is_empty())
        || record.source_branch.iter().any(|witness| {
            witness.owner_id.is_empty()
                || witness.symbol.is_empty()
                || witness.needle.is_empty()
                || witness.occurrence == 0
        })
    {
        return Err(format!(
            "authority record has an empty decision/witness: {identity:?}"
        ));
    }
    match &record.outcome {
        LeafOutcomeV1::Terminal(expected) if expected.phase.is_empty() => Err(format!(
            "authority Expected has an empty phase: {identity:?}"
        )),
        LeafOutcomeV1::Excluded(proof) if proof.reason.is_empty() => Err(format!(
            "authority exclusion has an empty proof: {identity:?}"
        )),
        _ => Ok(()),
    }
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Default)]
struct DigestAccumulator {
    included_count: u64,
    excluded_count: u64,
    identities: Vec<Digest32>,
    case_keys: Vec<Digest32>,
    source_branches: Vec<Digest32>,
    expected: Vec<Digest32>,
    exclusions: Vec<Digest32>,
    full_records: Vec<Digest32>,
}

impl DigestAccumulator {
    fn add_seal(&mut self, seal: &LeafSealV1) {
        self.identities.push(seal.source_leaf_identity_sha256);
        self.case_keys.push(seal.case_key_sha256);
        self.source_branches.push(seal.source_branch_sha256);
        match (seal.expected_sha256, seal.exclusion_sha256) {
            (Some(expected), None) => {
                self.included_count += 1;
                self.expected.push(expected);
            }
            (None, Some(exclusion)) => {
                self.excluded_count += 1;
                self.exclusions.push(exclusion);
            }
            _ => unreachable!("LeafSealV1 construction enforces one outcome digest"),
        }
        self.full_records.push(seal.full_record_sha256);
    }

    fn finish(self) -> FinishedAccumulator {
        FinishedAccumulator {
            included_count: self.included_count,
            excluded_count: self.excluded_count,
            source_leaf_identity_set_sha256: canonical::digest_digest_set(
                IDENTITY_SET_DOMAIN,
                self.identities,
            ),
            case_key_set_sha256: canonical::digest_digest_set(CASE_KEY_SET_DOMAIN, self.case_keys),
            source_branch_map_sha256: canonical::digest_digest_set(
                SOURCE_BRANCH_MAP_DOMAIN,
                self.source_branches,
            ),
            expected_map_sha256: canonical::digest_digest_set(EXPECTED_MAP_DOMAIN, self.expected),
            exclusion_map_sha256: canonical::digest_digest_set(
                EXCLUSION_MAP_DOMAIN,
                self.exclusions,
            ),
            full_record_set_sha256: canonical::digest_digest_set(
                FULL_RECORD_SET_DOMAIN,
                self.full_records,
            ),
        }
    }

    fn finish_shard(self, index: u8) -> ShardManifestV1 {
        let finished = self.finish();
        ShardManifestV1 {
            index,
            included_count: finished.included_count,
            excluded_count: finished.excluded_count,
            source_leaf_identity_set_sha256: finished.source_leaf_identity_set_sha256,
            case_key_set_sha256: finished.case_key_set_sha256,
            source_branch_map_sha256: finished.source_branch_map_sha256,
            expected_map_sha256: finished.expected_map_sha256,
            exclusion_map_sha256: finished.exclusion_map_sha256,
            full_record_set_sha256: finished.full_record_set_sha256,
        }
    }
}

struct FinishedAccumulator {
    included_count: u64,
    excluded_count: u64,
    source_leaf_identity_set_sha256: Digest32,
    case_key_set_sha256: Digest32,
    source_branch_map_sha256: Digest32,
    expected_map_sha256: Digest32,
    exclusion_map_sha256: Digest32,
    full_record_set_sha256: Digest32,
}
