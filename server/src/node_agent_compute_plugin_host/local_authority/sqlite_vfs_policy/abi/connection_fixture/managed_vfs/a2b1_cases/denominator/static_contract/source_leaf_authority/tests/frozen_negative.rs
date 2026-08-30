use super::*;

struct TinyFrozenFixture {
    records: Vec<LeafRecordV1>,
    seals: Vec<LeafSealV1>,
    leaf_tsv: String,
    manifest: RootManifestV1,
    manifest_tsv: String,
}

#[test]
fn frozen_leaf_verifier_rejects_missing_and_extra_leaf() {
    let fixture = tiny_frozen_fixture();

    let mut missing = verifier(&fixture.leaf_tsv);
    missing
        .observe(&fixture.seals[0])
        .expect("first tiny seal matches");
    let error = missing.finish().unwrap_err();
    assert!(error.contains("omitted 1 frozen source leaves"));

    let one_leaf_tsv =
        encode_leaf_seal_tsv(&fixture.seals[..1]).expect("encode one-leaf frozen ledger");
    let mut extra = verifier(&one_leaf_tsv);
    extra
        .observe(&fixture.seals[0])
        .expect("only frozen tiny seal matches");
    let error = extra.observe(&fixture.seals[1]).unwrap_err();
    assert!(error.contains("unfrozen source leaf"));
}

#[test]
fn frozen_leaf_verifier_rejects_case_and_full_digest_bit_drift() {
    let fixture = tiny_frozen_fixture();
    for (label, column) in [("case-key", 4), ("full-record", 5)] {
        let mutated = mutate_leaf_column(&fixture.leaf_tsv, 0, column, flip_first_hex_nibble);
        let mut verifier = verifier(&mutated);
        let error = verifier.observe(&fixture.seals[0]).unwrap_err();
        assert!(
            error.contains("source-leaf seal drifted"),
            "{label}: {error}"
        );
    }
}

#[test]
fn frozen_leaf_verifier_rejects_outcome_expected_exclusion_and_witness_drift() {
    let fixture = tiny_frozen_fixture();
    let mut variants = Vec::new();

    let mut expected = fixture.records[0].clone();
    terminal_mut(&mut expected).phase.push_str("-drift");
    variants.push(("expected", expected));

    let mut terminal_to_excluded = fixture.records[0].clone();
    terminal_to_excluded.outcome = fixture.records[1].outcome.clone();
    variants.push(("terminal-to-excluded", terminal_to_excluded));

    let mut excluded_to_terminal = fixture.records[1].clone();
    excluded_to_terminal.outcome = fixture.records[0].outcome.clone();
    variants.push(("excluded-to-terminal", excluded_to_terminal));

    let mut exclusion_kind = fixture.records[1].clone();
    let LeafOutcomeV1::Excluded(proof) = &mut exclusion_kind.outcome else {
        unreachable!("second tiny record is excluded")
    };
    proof.kind = ExclusionKindV1::SafetyPremise;
    variants.push(("exclusion-kind", exclusion_kind));

    let mut exclusion_reason = fixture.records[1].clone();
    let LeafOutcomeV1::Excluded(proof) = &mut exclusion_reason.outcome else {
        unreachable!("second tiny record is excluded")
    };
    proof.reason.push_str("-drift");
    variants.push(("exclusion-reason", exclusion_reason));

    let mut witness = fixture.records[0].clone();
    witness.source_branch[0].needle.push_str("-drift");
    variants.push(("source-witness", witness));

    for (label, changed) in variants {
        let mut verifier = verifier(&fixture.leaf_tsv);
        let error = verifier
            .observe(&LeafSealV1::from_record(&changed))
            .unwrap_err();
        assert!(
            error.contains("source-leaf seal drifted"),
            "{label}: {error}"
        );
    }
}

#[test]
fn frozen_manifest_rejects_resealed_context_count_set_and_shard_drift() {
    let fixture = tiny_frozen_fixture();
    type Mutation = fn(&mut RootManifestV1);
    let mutations: &[(&str, Mutation)] = &[
        ("context-ledger", |value| {
            value.context.ledger_sha256.0[0] ^= 1
        }),
        ("context-source", |value| {
            value.context.source_scope_sha256.0[0] ^= 1
        }),
        ("included-count", |value| value.included_count += 1),
        ("excluded-count", |value| value.excluded_count += 1),
        ("identity-set", |value| {
            value.source_leaf_identity_set_sha256.0[0] ^= 1
        }),
        ("case-key-set", |value| value.case_key_set_sha256.0[0] ^= 1),
        ("source-map", |value| {
            value.source_branch_map_sha256.0[0] ^= 1
        }),
        ("expected-map", |value| value.expected_map_sha256.0[0] ^= 1),
        ("exclusion-map", |value| {
            value.exclusion_map_sha256.0[0] ^= 1
        }),
        ("full-record-set", |value| {
            value.full_record_set_sha256.0[0] ^= 1
        }),
        ("shard-vector-count", |value| {
            value.shards.pop();
        }),
        ("shard-index", |value| value.shards[0].index = 1),
        ("shard-included-count", |value| {
            value.shards[0].included_count += 1
        }),
        ("shard-excluded-count", |value| {
            value.shards[0].excluded_count += 1
        }),
        ("shard-identity-set", |value| {
            value.shards[0].source_leaf_identity_set_sha256.0[0] ^= 1
        }),
        ("shard-case-key-set", |value| {
            value.shards[0].case_key_set_sha256.0[0] ^= 1
        }),
        ("shard-source-map", |value| {
            value.shards[0].source_branch_map_sha256.0[0] ^= 1
        }),
        ("shard-expected-map", |value| {
            value.shards[0].expected_map_sha256.0[0] ^= 1
        }),
        ("shard-exclusion-map", |value| {
            value.shards[0].exclusion_map_sha256.0[0] ^= 1
        }),
        ("shard-full-record-set", |value| {
            value.shards[0].full_record_set_sha256.0[0] ^= 1
        }),
    ];

    for (label, mutate) in mutations {
        let mut changed = fixture.manifest.clone();
        mutate(&mut changed);
        reseal(&mut changed);
        assert!(
            matches!(
                validate_derived_manifest_against_frozen(&fixture.manifest, &changed),
                Err(AuthorityValidationError::AuthorityManifestDrift { .. })
            ),
            "resealed {label} drift must not become authority"
        );
    }
}

#[test]
fn frozen_manifest_parser_rejects_index_shard_count_and_self_digest_drift() {
    let fixture = tiny_frozen_fixture();

    let bad_index = mutate_manifest_column(&fixture.manifest_tsv, 3, 1, |_| "001".to_owned());
    assert!(parse_manifest_tsv(&bad_index).is_err());

    let mut missing_shard = fixture
        .manifest_tsv
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(missing_shard.pop().is_some());
    let missing_shard = canonical_lines(missing_shard);
    assert!(parse_manifest_tsv(&missing_shard).is_err());

    let bad_self_digest =
        mutate_manifest_column(&fixture.manifest_tsv, 2, 9, flip_first_hex_nibble_owned);
    assert!(parse_manifest_tsv(&bad_self_digest).is_err());

    let mut invalid_self_digest = fixture.manifest.clone();
    invalid_self_digest.manifest_sha256.0[0] ^= 1;
    assert!(matches!(
        validate_derived_manifest_against_frozen(&fixture.manifest, &invalid_self_digest),
        Err(AuthorityValidationError::FrozenManifestSelfDigest { .. })
    ));
}

fn tiny_frozen_fixture() -> TinyFrozenFixture {
    let records = vec![
        sample_terminal("leaf.first"),
        sample_exclusion("leaf.second"),
    ];
    let seals = records
        .iter()
        .map(LeafSealV1::from_record)
        .collect::<Vec<_>>();
    let leaf_tsv = encode_leaf_seal_tsv(&seals).expect("encode tiny frozen leaf ledger");
    let mut context = sample_map_context();
    context.ledger_sha256 = leaf_seal_tsv_sha256(&leaf_tsv).expect("digest tiny leaf ledger");
    let manifest = build_manifest(context, &records).expect("build tiny frozen manifest");
    let manifest_tsv = encode_manifest_tsv(&manifest).expect("encode tiny frozen manifest");
    TinyFrozenFixture {
        records,
        seals,
        leaf_tsv,
        manifest,
        manifest_tsv,
    }
}

fn verifier(input: &str) -> FrozenLeafSealVerifierV1 {
    let ledger_sha256 = leaf_seal_tsv_sha256(input).expect("digest canonical tiny leaf TSV");
    FrozenLeafSealVerifierV1::from_tsv(input, ledger_sha256)
        .expect("parse ledger-bound tiny leaf TSV")
}

fn mutate_leaf_column(
    input: &str,
    data_row: usize,
    column: usize,
    mutate: impl FnOnce(&mut String),
) -> String {
    let mut lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
    let line = lines.get_mut(data_row + 1).expect("tiny leaf row exists");
    let mut fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
    mutate(fields.get_mut(column).expect("tiny leaf column exists"));
    *line = fields.join("\t");
    canonical_lines(lines)
}

fn mutate_manifest_column(
    input: &str,
    row: usize,
    column: usize,
    mutate: impl FnOnce(&str) -> String,
) -> String {
    let mut lines = input.lines().map(str::to_owned).collect::<Vec<_>>();
    let line = lines.get_mut(row).expect("tiny manifest row exists");
    let mut fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
    let value = fields.get_mut(column).expect("tiny manifest column exists");
    *value = mutate(value);
    *line = fields.join("\t");
    canonical_lines(lines)
}

fn flip_first_hex_nibble(value: &mut String) {
    *value = flip_first_hex_nibble_owned(value);
}

fn flip_first_hex_nibble_owned(value: &str) -> String {
    let mut changed = value.to_owned();
    let replacement = if changed.starts_with('0') { "1" } else { "0" };
    changed.replace_range(..1, replacement);
    changed
}

fn canonical_lines(lines: Vec<String>) -> String {
    let mut result = lines.join("\n");
    result.push('\n');
    result
}

fn reseal(manifest: &mut RootManifestV1) {
    manifest.manifest_sha256 = super::super::canonical::digest_manifest_body(manifest);
}
