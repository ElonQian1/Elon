use super::*;

#[test]
fn precomputed_leaf_digests_are_byte_identical_to_public_digest_functions() {
    for record in [
        sample_terminal("leaf.precomputed-terminal"),
        sample_exclusion("leaf.precomputed-excluded"),
    ] {
        let precomputed = canonical::precompute_record_digests(&record);
        assert_eq!(
            precomputed.source_leaf_identity_sha256,
            digest_leaf_identity(&record.key.identity)
        );
        assert_eq!(precomputed.case_key_sha256, digest_case_key(&record.key));
        assert_eq!(
            precomputed.source_branch_sha256,
            canonical::digest_source_branch(&record)
        );
        match &record.outcome {
            LeafOutcomeV1::Terminal(expected) => {
                assert_eq!(
                    precomputed.expected_sha256,
                    Some(canonical::digest_expected(&record.key, expected))
                );
                assert_eq!(precomputed.exclusion_sha256, None);
            }
            LeafOutcomeV1::Excluded(proof) => {
                assert_eq!(precomputed.expected_sha256, None);
                assert_eq!(
                    precomputed.exclusion_sha256,
                    Some(canonical::digest_exclusion(&record.key, proof))
                );
            }
        }
        assert_eq!(precomputed.full_record_sha256, digest_full_record(&record));
    }
}

#[test]
fn cached_primitive_digests_match_uncached_across_reused_paths_and_outcomes() {
    let records = [
        sample_terminal("leaf.cached-terminal-first"),
        sample_exclusion("leaf.cached-excluded"),
        sample_terminal("leaf.cached-terminal-second"),
    ];
    let mut cache = canonical::PrimitiveDigestCacheV1::default();

    for record in records {
        assert_eq!(
            canonical::precompute_record_digests_cached(&record, &mut cache),
            canonical::precompute_record_digests(&record)
        );
    }

    assert_eq!(cache.entry_counts(), (1, 1));
}
