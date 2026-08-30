//! Stable SHA-256 canonicalization for authority records.
//!
//! No digest uses `Debug`, `Hash`, `usize`, enum discriminants, map iteration order or implicit
//! serde layout. Fields are domain separated and length prefixed; sets are sorted by raw digest.

mod precomputed;

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::{
    expected::{ExpectedV1, LockEffectV1},
    lock_ranges::LockRangeV1,
    map_profiles::MapLoopProfileV1,
    model::{
        CaseKeyV1, Digest32, ExclusionProofV1, LeafIdentityV1, LeafOutcomeV1, LeafRecordV1,
        ManifestContextV1, RootManifestV1, ShardManifestV1, SourceScopeFileV1, SourceWitnessV1,
    },
};

pub(super) use precomputed::{
    precompute_record_digests, precompute_record_digests_cached, PrecomputedRecordDigestsV1,
    PrimitiveDigestCacheV1,
};

const IDENTITY_DOMAIN: &str = "ELON-A2B1-VFS-SOURCE-LEAF-IDENTITY-V1";
const CASE_KEY_DOMAIN: &str = "ELON-A2B1-VFS-CASE-KEY-V1";
const SOURCE_BRANCH_DOMAIN: &str = "ELON-A2B1-VFS-SOURCE-BRANCH-V1";
const EXPECTED_DOMAIN: &str = "ELON-A2B1-VFS-EXPECTED-V1";
const EXCLUSION_DOMAIN: &str = "ELON-A2B1-VFS-EXCLUSION-V1";
const FULL_RECORD_DOMAIN: &str = "ELON-A2B1-VFS-FULL-RECORD-V1";
const SOURCE_SCOPE_DOMAIN: &str = "ELON-A2B1-VFS-SOURCE-SCOPE-V1";
const MAP_PROFILE_DOMAIN: &str = "ELON-A2B1-VFS-MAP-PROFILE-V1";
const MAP_ORDINAL_DOMAIN: &str = "ELON-A2B1-VFS-MAP-ORDINAL-DOMAIN-V1";
const LOCK_RANGE_DOMAIN: &str = "ELON-A2B1-VFS-LOCK-RANGE-V1";
const SHARD_DOMAIN: &str = "ELON-A2B1-VFS-MANIFEST-SHARD-V1";
const MANIFEST_DOMAIN: &str = "ELON-A2B1-VFS-ROOT-MANIFEST-V1";

pub(crate) fn digest_leaf_identity(identity: &LeafIdentityV1) -> Digest32 {
    let mut digest = StableHasher::new(IDENTITY_DOMAIN);
    digest.text("root", identity.root.canonical_name());
    digest.text("leaf_id", &identity.leaf_id);
    digest.text("family_id", &identity.family_id);
    digest.u64("coordinate_count", identity.coordinates.len() as u64);
    for coordinate in &identity.coordinates {
        let mut item = StableHasher::new("ELON-A2B1-VFS-COORDINATE-V1");
        item.text("name", &coordinate.name);
        item.text("value", &coordinate.value);
        digest.digest("coordinate", item.finish());
    }
    digest.finish()
}

pub(crate) fn digest_case_key(key: &CaseKeyV1) -> Digest32 {
    precomputed::digest_case_key_with_identity(key, digest_leaf_identity(&key.identity))
}

pub(super) fn digest_source_branch(record: &LeafRecordV1) -> Digest32 {
    precomputed::digest_source_branch_with_case(record, digest_case_key(&record.key))
}

pub(super) fn digest_expected(key: &CaseKeyV1, expected: &ExpectedV1) -> Digest32 {
    precomputed::digest_expected_with_case(expected, digest_case_key(key))
}

pub(super) fn digest_exclusion(key: &CaseKeyV1, proof: &ExclusionProofV1) -> Digest32 {
    precomputed::digest_exclusion_with_case(proof, digest_case_key(key))
}

pub(crate) fn digest_full_record(record: &LeafRecordV1) -> Digest32 {
    let outcome_sha256 = match &record.outcome {
        LeafOutcomeV1::Terminal(expected) => digest_expected(&record.key, expected),
        LeafOutcomeV1::Excluded(proof) => digest_exclusion(&record.key, proof),
    };
    precomputed::digest_full_record_with_parts(
        record,
        digest_case_key(&record.key),
        digest_source_branch(record),
        outcome_sha256,
    )
}

pub(crate) fn digest_source_scope(files: &[SourceScopeFileV1]) -> Result<Digest32, String> {
    let mut owners = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut sorted = files.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.owner_id.cmp(&right.owner_id));
    let mut digest = StableHasher::new(SOURCE_SCOPE_DOMAIN);
    digest.u64("file_count", sorted.len() as u64);
    for file in sorted {
        if file.owner_id.is_empty()
            || file.repo_relative_path.is_empty()
            || !owners.insert(file.owner_id.as_str())
            || !paths.insert(file.repo_relative_path.as_str())
            || !lower_hex(&file.git_blob_oid_sha1, 40)
            || !lower_hex(&file.normalized_lf_sha256, 64)
        {
            return Err("source scope has an invalid or duplicate owner snapshot".to_owned());
        }
        let mut symbols = file.symbol_sentinels.iter().collect::<Vec<_>>();
        symbols.sort();
        if symbols.is_empty()
            || symbols.iter().any(|symbol| symbol.is_empty())
            || symbols.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(format!("{} has invalid symbol sentinels", file.owner_id));
        }
        let mut item = StableHasher::new("ELON-A2B1-VFS-SOURCE-SCOPE-FILE-V1");
        item.text("owner_id", &file.owner_id);
        item.text("repo_relative_path", &file.repo_relative_path);
        item.text("git_blob_oid_sha1", &file.git_blob_oid_sha1);
        item.text("normalized_lf_sha256", &file.normalized_lf_sha256);
        item.u64("symbol_count", symbols.len() as u64);
        for symbol in symbols {
            item.text("symbol", symbol);
        }
        digest.digest("file", item.finish());
    }
    Ok(digest.finish())
}

pub(crate) fn digest_map_profile_set(profiles: &[MapLoopProfileV1]) -> Digest32 {
    let records = profiles
        .iter()
        .map(|profile| {
            let mut digest = StableHasher::new(MAP_PROFILE_DOMAIN);
            digest.text("id", profile.id);
            digest.text("mode", profile.mode.canonical_name());
            digest.text("initialization", profile.initialization.canonical_name());
            digest.text("prestate", profile.prestate.canonical_name());
            digest.text("region_size_arm", profile.region_size_arm.canonical_name());
            digest.text("file_path", profile.file_path.canonical_name());
            digest.u16("ordinal_first", profile.ordinals.first);
            digest.u16("ordinal_last_inclusive", profile.ordinals.last_inclusive);
            digest.boolean("prior_mutation", profile.prior_mutation);
            digest.boolean("preexisting_mapping", profile.preexisting_mapping);
            digest.u16("file_grow_count", profile.file_grow_count);
            digest.finish()
        })
        .collect();
    digest_digest_set("ELON-A2B1-VFS-MAP-PROFILE-SET-V1", records)
}

pub(crate) fn digest_map_ordinal_domains(profiles: &[MapLoopProfileV1]) -> Digest32 {
    let records = profiles
        .iter()
        .map(|profile| {
            let mut digest = StableHasher::new(MAP_ORDINAL_DOMAIN);
            digest.text("profile_id", profile.id);
            digest.u16("first", profile.ordinals.first);
            digest.u16("last_inclusive", profile.ordinals.last_inclusive);
            digest.finish()
        })
        .collect();
    digest_digest_set("ELON-A2B1-VFS-MAP-ORDINAL-DOMAIN-SET-V1", records)
}

pub(crate) fn digest_lock_range_set(ranges: &[LockRangeV1]) -> Digest32 {
    let records = ranges
        .iter()
        .map(|range| {
            let mut digest = StableHasher::new(LOCK_RANGE_DOMAIN);
            digest.text("action", range.action.canonical_name());
            digest.u8("first", range.first);
            digest.u8("count", range.count);
            digest.u8("mask", range.mask);
            digest.finish()
        })
        .collect();
    digest_digest_set("ELON-A2B1-VFS-LOCK-RANGE-SET-V1", records)
}

pub(super) fn digest_manifest_body(manifest: &RootManifestV1) -> Digest32 {
    let mut digest = StableHasher::new(MANIFEST_DOMAIN);
    digest.digest("context_sha256", digest_manifest_context(&manifest.context));
    digest.u64("included_count", manifest.included_count);
    digest.u64("excluded_count", manifest.excluded_count);
    digest.digest(
        "source_leaf_identity_set_sha256",
        manifest.source_leaf_identity_set_sha256,
    );
    digest.digest("case_key_set_sha256", manifest.case_key_set_sha256);
    digest.digest(
        "source_branch_map_sha256",
        manifest.source_branch_map_sha256,
    );
    digest.digest("expected_map_sha256", manifest.expected_map_sha256);
    digest.digest("exclusion_map_sha256", manifest.exclusion_map_sha256);
    digest.digest("full_record_set_sha256", manifest.full_record_set_sha256);
    digest.u64("shard_count", manifest.shards.len() as u64);
    for shard in &manifest.shards {
        digest.digest("shard", digest_shard(shard));
    }
    digest.finish()
}

pub(super) fn digest_digest_set(domain: &str, mut values: Vec<Digest32>) -> Digest32 {
    values.sort_unstable();
    let mut digest = StableHasher::new(domain);
    digest.u64("item_count", values.len() as u64);
    for value in values {
        digest.digest("item", value);
    }
    digest.finish()
}

fn digest_decision(decision: &super::model::DecisionV1) -> Digest32 {
    let mut digest = StableHasher::new("ELON-A2B1-VFS-DECISION-V1");
    digest.text("stage", decision.stage.canonical_name());
    digest.text("branch", &decision.branch);
    digest.finish()
}

fn digest_witness(witness: &SourceWitnessV1) -> Digest32 {
    let mut digest = StableHasher::new("ELON-A2B1-VFS-SOURCE-WITNESS-V1");
    digest.text("owner_id", &witness.owner_id);
    digest.text("symbol", &witness.symbol);
    digest.text("needle", &witness.needle);
    digest.u8("occurrence", witness.occurrence);
    digest.finish()
}

fn digest_lock_effect(effect: LockEffectV1) -> Digest32 {
    let mut digest = StableHasher::new("ELON-A2B1-VFS-LOCK-EFFECT-V1");
    digest.text("kind", effect.canonical_name());
    match effect {
        LockEffectV1::Acquired { mode, mask, native }
        | LockEffectV1::Released { mode, mask, native } => {
            digest.text("mode", mode.canonical_name());
            digest.u8("mask", mask);
            digest.boolean("native", native);
        }
        LockEffectV1::OutcomeUncertain { mode, mask } => {
            digest.text("mode", mode.canonical_name());
            digest.u8("mask", mask);
        }
        LockEffectV1::NotReached | LockEffectV1::Unchanged => {}
    }
    digest.finish()
}

fn digest_manifest_context(context: &ManifestContextV1) -> Digest32 {
    let mut digest = StableHasher::new("ELON-A2B1-VFS-MANIFEST-CONTEXT-V1");
    digest.text("schema", &context.schema);
    digest.text("root", context.root.canonical_name());
    digest.text("target_scope", &context.target_scope);
    digest.text(
        "source_baseline_commit_sha1",
        &context.source_baseline_commit_sha1,
    );
    digest.digest("source_scope_sha256", context.source_scope_sha256);
    digest.digest("ledger_sha256", context.ledger_sha256);
    digest.optional_digest("map_profile_set_sha256", context.map_profile_set_sha256);
    digest.optional_digest(
        "map_ordinal_domain_sha256",
        context.map_ordinal_domain_sha256,
    );
    digest.optional_digest("lock_range_set_sha256", context.lock_range_set_sha256);
    digest.optional_u64("lock_range_count", context.lock_range_count);
    digest.finish()
}

fn digest_shard(shard: &ShardManifestV1) -> Digest32 {
    let mut digest = StableHasher::new(SHARD_DOMAIN);
    digest.u8("index", shard.index);
    digest.u64("included_count", shard.included_count);
    digest.u64("excluded_count", shard.excluded_count);
    digest.digest(
        "source_leaf_identity_set_sha256",
        shard.source_leaf_identity_set_sha256,
    );
    digest.digest("case_key_set_sha256", shard.case_key_set_sha256);
    digest.digest("source_branch_map_sha256", shard.source_branch_map_sha256);
    digest.digest("expected_map_sha256", shard.expected_map_sha256);
    digest.digest("exclusion_map_sha256", shard.exclusion_map_sha256);
    digest.digest("full_record_set_sha256", shard.full_record_set_sha256);
    digest.finish()
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

struct StableHasher(Sha256);

impl StableHasher {
    fn new(domain: &str) -> Self {
        let mut digest = Sha256::new();
        digest.update(domain.as_bytes());
        digest.update([0]);
        Self(digest)
    }

    fn text(&mut self, label: &str, value: &str) {
        self.bytes(label, value.as_bytes());
    }

    fn boolean(&mut self, label: &str, value: bool) {
        self.bytes(label, &[u8::from(value)]);
    }

    fn u8(&mut self, label: &str, value: u8) {
        self.bytes(label, &[value]);
    }

    fn u16(&mut self, label: &str, value: u16) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn u64(&mut self, label: &str, value: u64) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn digest(&mut self, label: &str, value: Digest32) {
        self.bytes(label, &value.0);
    }

    fn optional_digest(&mut self, label: &str, value: Option<Digest32>) {
        self.boolean(&format!("{label}_present"), value.is_some());
        if let Some(value) = value {
            self.digest(label, value);
        }
    }

    fn optional_u64(&mut self, label: &str, value: Option<u64>) {
        self.boolean(&format!("{label}_present"), value.is_some());
        if let Some(value) = value {
            self.u64(label, value);
        }
    }

    fn bytes(&mut self, label: &str, value: &[u8]) {
        self.0.update(label.as_bytes());
        self.0.update([0]);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }

    fn finish(self) -> Digest32 {
        Digest32(self.0.finalize().into())
    }
}
