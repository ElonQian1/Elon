//! Single-pass record digest composition for the hot per-leaf sealing path.

use std::collections::BTreeMap;

use super::super::{
    CaseKeyV1, DecisionV1, Digest32, ExclusionProofV1, ExpectedV1, LeafOutcomeV1, LeafRecordV1,
    SourceWitnessV1,
};
use super::{
    digest_decision, digest_leaf_identity, digest_lock_effect, digest_witness, StableHasher,
    CASE_KEY_DOMAIN, EXCLUSION_DOMAIN, EXPECTED_DOMAIN, FULL_RECORD_DOMAIN, SOURCE_BRANCH_DOMAIN,
};

#[derive(Default)]
pub(crate) struct PrimitiveDigestCacheV1 {
    decisions: BTreeMap<DecisionV1, Digest32>,
    witnesses: BTreeMap<SourceWitnessV1, Digest32>,
}

impl PrimitiveDigestCacheV1 {
    fn decision(&mut self, decision: &DecisionV1) -> Digest32 {
        if let Some(digest) = self.decisions.get(decision) {
            return *digest;
        }
        let digest = digest_decision(decision);
        self.decisions.insert(decision.clone(), digest);
        digest
    }

    fn witness(&mut self, witness: &SourceWitnessV1) -> Digest32 {
        if let Some(digest) = self.witnesses.get(witness) {
            return *digest;
        }
        let digest = digest_witness(witness);
        self.witnesses.insert(witness.clone(), digest);
        digest
    }

    #[cfg(test)]
    pub(crate) fn entry_counts(&self) -> (usize, usize) {
        (self.decisions.len(), self.witnesses.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrecomputedRecordDigestsV1 {
    pub(crate) source_leaf_identity_sha256: Digest32,
    pub(crate) case_key_sha256: Digest32,
    pub(crate) source_branch_sha256: Digest32,
    pub(crate) expected_sha256: Option<Digest32>,
    pub(crate) exclusion_sha256: Option<Digest32>,
    pub(crate) full_record_sha256: Digest32,
}

pub(crate) fn precompute_record_digests(record: &LeafRecordV1) -> PrecomputedRecordDigestsV1 {
    let source_leaf_identity_sha256 = digest_leaf_identity(&record.key.identity);
    let case_key_sha256 = digest_case_key_with_identity(&record.key, source_leaf_identity_sha256);
    let source_branch_sha256 = digest_source_branch_with_case(record, case_key_sha256);
    let (expected_sha256, exclusion_sha256, outcome_sha256) = match &record.outcome {
        LeafOutcomeV1::Terminal(expected) => {
            let digest = digest_expected_with_case(expected, case_key_sha256);
            (Some(digest), None, digest)
        }
        LeafOutcomeV1::Excluded(proof) => {
            let digest = digest_exclusion_with_case(proof, case_key_sha256);
            (None, Some(digest), digest)
        }
    };
    let full_record_sha256 = digest_full_record_with_parts(
        record,
        case_key_sha256,
        source_branch_sha256,
        outcome_sha256,
    );
    PrecomputedRecordDigestsV1 {
        source_leaf_identity_sha256,
        case_key_sha256,
        source_branch_sha256,
        expected_sha256,
        exclusion_sha256,
        full_record_sha256,
    }
}

pub(crate) fn precompute_record_digests_cached(
    record: &LeafRecordV1,
    cache: &mut PrimitiveDigestCacheV1,
) -> PrecomputedRecordDigestsV1 {
    let source_leaf_identity_sha256 = digest_leaf_identity(&record.key.identity);
    let case_key_sha256 =
        digest_case_key_with_identity_cached(&record.key, source_leaf_identity_sha256, cache);
    let source_branch_sha256 =
        digest_source_branch_with_case_cached(record, case_key_sha256, cache);
    let (expected_sha256, exclusion_sha256, outcome_sha256) = match &record.outcome {
        LeafOutcomeV1::Terminal(expected) => {
            let digest = digest_expected_with_case(expected, case_key_sha256);
            (Some(digest), None, digest)
        }
        LeafOutcomeV1::Excluded(proof) => {
            let digest = digest_exclusion_with_case(proof, case_key_sha256);
            (None, Some(digest), digest)
        }
    };
    let full_record_sha256 = digest_full_record_with_parts(
        record,
        case_key_sha256,
        source_branch_sha256,
        outcome_sha256,
    );
    PrecomputedRecordDigestsV1 {
        source_leaf_identity_sha256,
        case_key_sha256,
        source_branch_sha256,
        expected_sha256,
        exclusion_sha256,
        full_record_sha256,
    }
}

pub(super) fn digest_case_key_with_identity(
    key: &CaseKeyV1,
    identity_sha256: Digest32,
) -> Digest32 {
    let mut digest = StableHasher::new(CASE_KEY_DOMAIN);
    digest.digest("identity_sha256", identity_sha256);
    digest.u64("decision_count", key.decisions.len() as u64);
    for decision in &key.decisions {
        digest.digest("decision", digest_decision(decision));
    }
    digest.finish()
}

fn digest_case_key_with_identity_cached(
    key: &CaseKeyV1,
    identity_sha256: Digest32,
    cache: &mut PrimitiveDigestCacheV1,
) -> Digest32 {
    let mut digest = StableHasher::new(CASE_KEY_DOMAIN);
    digest.digest("identity_sha256", identity_sha256);
    digest.u64("decision_count", key.decisions.len() as u64);
    for decision in &key.decisions {
        digest.digest("decision", cache.decision(decision));
    }
    digest.finish()
}

pub(super) fn digest_source_branch_with_case(
    record: &LeafRecordV1,
    case_key_sha256: Digest32,
) -> Digest32 {
    let mut digest = StableHasher::new(SOURCE_BRANCH_DOMAIN);
    digest.digest("case_key_sha256", case_key_sha256);
    digest.u64("witness_count", record.source_branch.len() as u64);
    for witness in &record.source_branch {
        digest.digest("witness", digest_witness(witness));
    }
    digest.finish()
}

fn digest_source_branch_with_case_cached(
    record: &LeafRecordV1,
    case_key_sha256: Digest32,
    cache: &mut PrimitiveDigestCacheV1,
) -> Digest32 {
    let mut digest = StableHasher::new(SOURCE_BRANCH_DOMAIN);
    digest.digest("case_key_sha256", case_key_sha256);
    digest.u64("witness_count", record.source_branch.len() as u64);
    for witness in &record.source_branch {
        digest.digest("witness", cache.witness(witness));
    }
    digest.finish()
}

pub(super) fn digest_expected_with_case(
    expected: &ExpectedV1,
    case_key_sha256: Digest32,
) -> Digest32 {
    let mut digest = StableHasher::new(EXPECTED_DOMAIN);
    digest.digest("case_key_sha256", case_key_sha256);
    digest.text("sqlite", expected.sqlite.canonical_name());
    digest.text("disposition", expected.disposition.canonical_name());
    digest.text("phase", &expected.phase);
    digest.text("failure", expected.failure.canonical_name());
    digest.text("mutation", expected.mutation.canonical_name());
    digest.boolean("lock_outcome_uncertain", expected.lock_outcome_uncertain);
    digest.digest("lock_effect", digest_lock_effect(expected.lock_effect));
    digest.text("dms_lock", expected.dms_lock.canonical_name());
    digest.text("raw_slots", expected.raw_slots.canonical_name());
    digest.text("route", expected.route.canonical_name());
    digest.text("callback", expected.callback.canonical_name());
    digest.text("file", expected.file.canonical_name());
    digest.text("mapping", expected.mapping.canonical_name());
    digest.text("view", expected.view.canonical_name());
    digest.text("payload", expected.payload.canonical_name());
    digest.u16("callback_begin", expected.counts.callback_begin);
    digest.u16("callback_complete", expected.counts.callback_complete);
    digest.u16("native_lock", expected.counts.native_lock);
    digest.u16("native_unlock", expected.counts.native_unlock);
    digest.u16("file_grow", expected.counts.file_grow);
    digest.u16("mapping_create", expected.counts.mapping_create);
    digest.u16("view_map", expected.counts.view_map);
    digest.finish()
}

pub(super) fn digest_exclusion_with_case(
    proof: &ExclusionProofV1,
    case_key_sha256: Digest32,
) -> Digest32 {
    let mut digest = StableHasher::new(EXCLUSION_DOMAIN);
    digest.digest("case_key_sha256", case_key_sha256);
    digest.text("kind", proof.kind.canonical_name());
    digest.text("reason", &proof.reason);
    digest.finish()
}

pub(super) fn digest_full_record_with_parts(
    record: &LeafRecordV1,
    case_key_sha256: Digest32,
    source_branch_sha256: Digest32,
    outcome_sha256: Digest32,
) -> Digest32 {
    let mut digest = StableHasher::new(FULL_RECORD_DOMAIN);
    digest.digest("case_key_sha256", case_key_sha256);
    digest.digest("source_branch_sha256", source_branch_sha256);
    digest.text("outcome_kind", record.outcome.canonical_name());
    digest.digest("outcome_sha256", outcome_sha256);
    digest.finish()
}
