use super::*;
use sha2::Digest as _;

mod frozen_negative;
mod precomputed_equivalence;

#[test]
fn map_profiles_freeze_exact_independent_ordinal_domains() {
    validate_map_profiles().expect("valid source-first Map profile ledger");
    assert_eq!(MAP_LOOP_PROFILES.len(), 21);
    assert_eq!(
        MAP_LOOP_PROFILES
            .iter()
            .map(|profile| usize::from(profile.ordinals.width()))
            .sum::<usize>(),
        5_373
    );
    assert_ne!(
        digest_map_profile_set(MAP_LOOP_PROFILES),
        digest_map_ordinal_domains(MAP_LOOP_PROFILES)
    );
}

#[test]
fn lock_ranges_freeze_exact_literal_eighty_eight() {
    validate_lock_ranges().expect("valid source-first Lock range ledger");
    assert_eq!(LOCK_RANGES.len(), 88);
    assert_ne!(digest_lock_range_set(LOCK_RANGES), Digest32::ZERO);
}

#[test]
fn canonical_expected_digest_covers_every_expected_field() {
    assert_expected_change(|expected| expected.sqlite = SqliteResultV1::Busy);
    assert_expected_change(|expected| expected.disposition = TerminalDispositionV1::Quarantined);
    assert_expected_change(|expected| expected.phase.push_str("-changed"));
    assert_expected_change(|expected| expected.failure = FailureClassV1::MutatedButKnown);
    assert_expected_change(|expected| expected.mutation = MutationStateV1::Known);
    assert_expected_change(|expected| expected.lock_outcome_uncertain = true);
    assert_expected_change(|expected| {
        expected.lock_effect = LockEffectV1::OutcomeUncertain {
            mode: LockModeV1::Exclusive,
            mask: 0x02,
        }
    });
    assert_expected_change(|expected| expected.dms_lock = DmsLockCustodyV1::UnknownRetained);
    assert_expected_change(|expected| expected.raw_slots = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.route = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.callback = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.file = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.mapping = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.view = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.payload = CustodyStateV1::Released);
    assert_expected_change(|expected| expected.counts.callback_begin += 1);
    assert_expected_change(|expected| expected.counts.callback_complete += 1);
    assert_expected_change(|expected| expected.counts.native_lock += 1);
    assert_expected_change(|expected| expected.counts.native_unlock += 1);
    assert_expected_change(|expected| expected.counts.file_grow += 1);
    assert_expected_change(|expected| expected.counts.mapping_create += 1);
    assert_expected_change(|expected| expected.counts.view_map += 1);
}

#[test]
fn canonical_lock_effect_digest_covers_mode_mask_and_native() {
    assert_lock_effect_change(
        LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask: 0x01,
            native: false,
        },
        LockEffectV1::Acquired {
            mode: LockModeV1::Exclusive,
            mask: 0x01,
            native: false,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask: 0x01,
            native: false,
        },
        LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask: 0x02,
            native: false,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask: 0x01,
            native: false,
        },
        LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask: 0x01,
            native: true,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask: 0x04,
            native: false,
        },
        LockEffectV1::Released {
            mode: LockModeV1::Exclusive,
            mask: 0x04,
            native: false,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask: 0x04,
            native: false,
        },
        LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask: 0x08,
            native: false,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask: 0x04,
            native: false,
        },
        LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask: 0x04,
            native: true,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::OutcomeUncertain {
            mode: LockModeV1::Shared,
            mask: 0x10,
        },
        LockEffectV1::OutcomeUncertain {
            mode: LockModeV1::Exclusive,
            mask: 0x10,
        },
    );
    assert_lock_effect_change(
        LockEffectV1::OutcomeUncertain {
            mode: LockModeV1::Shared,
            mask: 0x10,
        },
        LockEffectV1::OutcomeUncertain {
            mode: LockModeV1::Shared,
            mask: 0x20,
        },
    );
}

#[test]
fn dms_lock_custody_freezes_all_eight_states() {
    let names = [
        DmsLockCustodyV1::NotReached,
        DmsLockCustodyV1::UnknownRetained,
        DmsLockCustodyV1::UnobservedRetained,
        DmsLockCustodyV1::ExistingShared,
        DmsLockCustodyV1::AcquiredShared,
        DmsLockCustodyV1::Released,
        DmsLockCustodyV1::ExclusiveKnown,
        DmsLockCustodyV1::ExclusiveOutcomeUncertain,
    ]
    .map(DmsLockCustodyV1::canonical_name);
    assert_eq!(
        names
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
}

#[test]
fn exact_comparison_reports_missing_extra_and_valid_expected_drift() {
    let first = sample_terminal("leaf.first");
    let second = sample_exclusion("leaf.second");
    let authority = vec![first.clone(), second.clone()];
    let extra = sample_terminal("leaf.extra");
    let diff = compare_exact_records(&authority, &[first.clone(), extra]).unwrap_err();
    assert_eq!(diff.missing_keys, vec![second.key.clone()]);
    assert_eq!(diff.extra_keys.len(), 1);
    assert!(diff.changed_keys.is_empty());

    let mut changed = first.clone();
    terminal_mut(&mut changed).phase = "still-valid-but-different".to_owned();
    let diff = compare_exact_records(&authority, &[changed, second]).unwrap_err();
    assert_eq!(diff.changed_keys, vec![first.key]);
    assert!(diff.missing_keys.is_empty() && diff.extra_keys.is_empty());
}

#[test]
fn frozen_manifest_separates_ledger_and_actual_graph_drift() {
    let authority = vec![
        sample_terminal("leaf.first"),
        sample_exclusion("leaf.second"),
    ];
    let context = sample_map_context();
    let frozen = build_manifest(context.clone(), &authority).expect("build authority manifest");
    assert_eq!(frozen.included_count, 1);
    assert_eq!(frozen.excluded_count, 1);
    assert_eq!(frozen.shards.len(), MANIFEST_SHARDS);
    assert_ne!(frozen.manifest_sha256, Digest32::ZERO);
    assert_eq!(
        validate_actual_against_frozen(context.clone(), &authority, &authority, &frozen),
        Ok(1)
    );

    let mut actual = authority.clone();
    terminal_mut(&mut actual[0]).phase = "valid-actual-drift".to_owned();
    assert!(matches!(
        validate_actual_against_frozen(context.clone(), &authority, &actual, &frozen),
        Err(AuthorityValidationError::ActualRecordMismatch(_))
    ));

    let mut changed_ledger = authority.clone();
    terminal_mut(&mut changed_ledger[0]).phase = "ledger-drift".to_owned();
    assert!(matches!(
        validate_actual_against_frozen(context, &changed_ledger, &changed_ledger, &frozen),
        Err(AuthorityValidationError::AuthorityManifestDrift { .. })
    ));
}

#[test]
fn compact_leaf_tsv_is_canonical_and_rejects_full_record_drift() {
    let records = [
        sample_terminal("leaf.first"),
        sample_exclusion("leaf.second"),
    ];
    let seals = records
        .iter()
        .map(LeafSealV1::from_record)
        .collect::<Vec<_>>();
    let tsv = encode_leaf_seal_tsv(&seals).expect("encode compact leaf seals");
    assert!(tsv
        .lines()
        .skip(1)
        .all(|line| line.split('\t').count() == 6));
    assert_eq!(
        leaf_seal_tsv_sha256(&tsv),
        Ok(Digest32(sha2::Sha256::digest(tsv.as_bytes()).into()))
    );
    assert!(leaf_seal_tsv_sha256(&tsv.replace('\n', "\r\n")).is_err());

    let ledger_sha256 = leaf_seal_tsv_sha256(&tsv).expect("digest frozen leaf TSV");
    assert!(FrozenLeafSealVerifierV1::from_tsv(&tsv, Digest32([9; 32])).is_err());
    let mut verifier = FrozenLeafSealVerifierV1::from_tsv(&tsv, ledger_sha256)
        .expect("parse ledger-bound frozen leaf TSV");
    verifier.observe(&seals[0]).expect("first seal matches");
    let mut changed = records[1].clone();
    match &mut changed.outcome {
        LeafOutcomeV1::Excluded(proof) => proof.reason.push_str("-drift"),
        LeafOutcomeV1::Terminal(_) => unreachable!(),
    }
    assert!(verifier
        .observe(&LeafSealV1::from_record(&changed))
        .is_err());
}

#[test]
fn root_manifest_tsv_round_trips_all_256_shards() {
    let records = [
        sample_terminal("leaf.first"),
        sample_exclusion("leaf.second"),
    ];
    let seals = records
        .iter()
        .map(LeafSealV1::from_record)
        .collect::<Vec<_>>();
    let leaf_tsv = encode_leaf_seal_tsv(&seals).expect("encode compact leaf seals");
    let mut context = sample_map_context();
    context.ledger_sha256 = leaf_seal_tsv_sha256(&leaf_tsv).expect("digest compact leaf TSV");
    let manifest = build_manifest(context, &records).expect("build root manifest");
    let text = encode_manifest_tsv(&manifest).expect("encode root manifest");
    assert_eq!(parse_manifest_tsv(&text), Ok(manifest));
    assert!(parse_manifest_tsv(&text.replace('\n', "\r\n")).is_err());
}

#[test]
fn source_scope_hash_is_order_independent_but_content_bound() {
    let first = SourceScopeFileV1 {
        owner_id: "owner-a".to_owned(),
        repo_relative_path: "src/a.rs".to_owned(),
        git_blob_oid_sha1: "1".repeat(40),
        normalized_lf_sha256: "2".repeat(64),
        symbol_sentinels: vec!["fn z".to_owned(), "fn a".to_owned()],
    };
    let second = SourceScopeFileV1 {
        owner_id: "owner-b".to_owned(),
        repo_relative_path: "src/b.rs".to_owned(),
        git_blob_oid_sha1: "3".repeat(40),
        normalized_lf_sha256: "4".repeat(64),
        symbol_sentinels: vec!["fn b".to_owned()],
    };
    let forward = digest_source_scope(&[first.clone(), second.clone()]).unwrap();
    let reverse = digest_source_scope(&[second.clone(), first.clone()]).unwrap();
    assert_eq!(forward, reverse);
    let mut changed = second;
    changed.symbol_sentinels.push("fn c".to_owned());
    assert_ne!(
        forward,
        digest_source_scope(&[first, changed]).expect("changed scope is still valid")
    );
}

#[test]
fn production_source_scope_freezes_all_twenty_nine_owner_files() {
    validate_source_scope().expect("valid production source scope");
    assert_eq!(PRODUCTION_SOURCE_SCOPE.len(), 29);
    assert_eq!(source_scope_files().len(), 29);
    assert_ne!(source_scope_sha256().unwrap(), Digest32::ZERO);
    assert_eq!(SOURCE_BASELINE_COMMIT_SHA1.len(), 40);

    let namespace_types = PRODUCTION_SOURCE_SCOPE
        .iter()
        .find(|snapshot| snapshot.owner_id == "managed-namespace-types")
        .expect("sqlite namespace types is independently in source scope");
    assert_eq!(
        namespace_types.git_blob_oid_sha1,
        "c7e8672ab8639b70b1577df5d04f7669fc50794c"
    );
    assert_eq!(
        namespace_types.normalized_lf_sha256,
        "25fe5b2f880a002448f038517cc6741dc778d54907edfb021e1a1824f15d84cd"
    );
}

#[test]
fn source_witness_validation_is_scoped_to_a_symbol() {
    let valid = SourceWitnessV1 {
        owner_id: "managed-coordinator".to_owned(),
        symbol: "fn poisoned_failure".to_owned(),
        needle: "self.mark_domain_terminal()".to_owned(),
        occurrence: 1,
    };
    validate_source_witness(&valid).expect("source-local witness is present");

    let outside_symbol = SourceWitnessV1 {
        needle: "fn mark_poisoned".to_owned(),
        ..valid
    };
    assert!(validate_source_witness(&outside_symbol).is_err());
}

fn assert_expected_change(change: impl FnOnce(&mut ExpectedV1)) {
    let baseline = sample_terminal("leaf.expected-coverage");
    let baseline_digest = digest_full_record(&baseline);
    let mut changed = baseline;
    change(terminal_mut(&mut changed));
    assert_ne!(baseline_digest, digest_full_record(&changed));
}

fn assert_lock_effect_change(baseline: LockEffectV1, changed: LockEffectV1) {
    let mut baseline_record = sample_terminal("leaf.lock-effect-coverage");
    terminal_mut(&mut baseline_record).lock_effect = baseline;
    let mut changed_record = baseline_record.clone();
    terminal_mut(&mut changed_record).lock_effect = changed;
    assert_ne!(
        digest_full_record(&baseline_record),
        digest_full_record(&changed_record)
    );
}

fn terminal_mut(record: &mut LeafRecordV1) -> &mut ExpectedV1 {
    match &mut record.outcome {
        LeafOutcomeV1::Terminal(expected) => expected,
        LeafOutcomeV1::Excluded(_) => panic!("expected a terminal sample"),
    }
}

fn sample_terminal(leaf_id: &str) -> LeafRecordV1 {
    sample_record(
        leaf_id,
        LeafOutcomeV1::Terminal(ExpectedV1 {
            sqlite: SqliteResultV1::Ok,
            disposition: TerminalDispositionV1::Returned,
            phase: "Success".to_owned(),
            failure: FailureClassV1::None,
            mutation: MutationStateV1::None,
            lock_outcome_uncertain: false,
            lock_effect: LockEffectV1::Unchanged,
            dms_lock: DmsLockCustodyV1::NotReached,
            raw_slots: CustodyStateV1::Retained,
            route: CustodyStateV1::Retained,
            callback: CustodyStateV1::Retained,
            file: CustodyStateV1::Retained,
            mapping: CustodyStateV1::Retained,
            view: CustodyStateV1::Retained,
            payload: CustodyStateV1::Retained,
            counts: ObservableCountsV1::default(),
        }),
    )
}

fn sample_exclusion(leaf_id: &str) -> LeafRecordV1 {
    sample_record(
        leaf_id,
        LeafOutcomeV1::Excluded(ExclusionProofV1 {
            kind: ExclusionKindV1::ControlFlow,
            reason: "source-reviewed domination proof".to_owned(),
        }),
    )
}

fn sample_record(leaf_id: &str, outcome: LeafOutcomeV1) -> LeafRecordV1 {
    LeafRecordV1 {
        key: CaseKeyV1 {
            identity: LeafIdentityV1 {
                root: RootOperationV1::Map,
                leaf_id: leaf_id.to_owned(),
                family_id: "map.sample.family".to_owned(),
                coordinates: vec![CoordinateV1 {
                    name: "ordinal".to_owned(),
                    value: "1".to_owned(),
                }],
            },
            decisions: vec![DecisionV1 {
                stage: DecisionStageV1::ManagedRequest,
                branch: "sample-branch".to_owned(),
            }],
        },
        source_branch: vec![SourceWitnessV1 {
            owner_id: "managed-mapping".to_owned(),
            symbol: "fn map".to_owned(),
            needle: "match source branch".to_owned(),
            occurrence: 1,
        }],
        outcome,
    }
}

fn sample_map_context() -> ManifestContextV1 {
    ManifestContextV1 {
        schema: AUTHORITY_SCHEMA_V1.to_owned(),
        root: RootOperationV1::Map,
        target_scope: PRODUCTION_WINDOWS_X64_SCOPE.to_owned(),
        source_baseline_commit_sha1: "a".repeat(40),
        source_scope_sha256: Digest32([1; 32]),
        ledger_sha256: Digest32([2; 32]),
        map_profile_set_sha256: Some(digest_map_profile_set(MAP_LOOP_PROFILES)),
        map_ordinal_domain_sha256: Some(digest_map_ordinal_domains(MAP_LOOP_PROFILES)),
        lock_range_set_sha256: None,
        lock_range_count: None,
    }
}
