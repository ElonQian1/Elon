//! Explicit, ignored snapshot generator.  Normal tests never write authority files.

use std::path::{Path, PathBuf};

use super::super::{invariants, lock, map, model::ContractGraph};
use super::{
    adapter::stream_graph_manifest,
    leaf_seal::{encode_leaf_seal_tsv, leaf_seal_tsv_sha256, FrozenLeafSealVerifierV1},
    lock_ranges::validate_lock_ranges,
    manifest_tsv::encode_manifest_tsv,
    map_profiles::validate_map_profiles,
    model::{Digest32, RootOperationV1},
    source_scope::validate_source_scope,
    trusted_current_context, MAP_LEAF_LEDGER_PARTS,
};

const CONFIRM_ENV: &str = "ELON_A2B1_ALLOW_FREEZE_GENERATION";
const CONFIRM_TOKEN: &str = "REVIEWED_SOURCE_LEDGER_GENERATION_ONLY";
const OUTPUT_ENV: &str = "ELON_A2B1_FREEZE_OUTPUT_DIR";
const HEARTBEAT_INTERVAL: u64 = 512;

#[test]
#[ignore = "writes review candidates only when an explicit environment confirmation is present"]
fn generate_review_candidate_leaf_and_manifest_tsv() {
    generate_review_candidates();
}

fn generate_review_candidates() {
    let output = prepare_generation();

    generate_root(&output, RootOperationV1::Map, map::graph());
    generate_root(&output, RootOperationV1::Lock, lock::graph());
}

fn prepare_generation() -> PathBuf {
    require_confirmation();
    let output = output_directory();
    super::super::validate_source_owner_authority()
        .expect("source-owner authority must be exact before generation");
    validate_source_scope().expect("production source scope must be exact before generation");
    validate_map_profiles().expect("Map profile authority must be exact before generation");
    validate_lock_ranges().expect("Lock range authority must be exact before generation");
    output
}

fn generate_root(output: &Path, root: RootOperationV1, graph: ContractGraph) {
    invariants::validate_graph(&graph)
        .expect("graph must pass static invariants before generation");
    let total_leaves = graph.source_leaf_universe.len() as u64;
    eprintln!(
        "a2b1-freeze root={} phase=first-pass start total={total_leaves}",
        root.canonical_name(),
    );

    // The first pass obtains only compact seals.  The nonzero temporary ledger digest cannot
    // escape because that pass's manifest is discarded.  The second pass binds the SHA-256 of the
    // exact canonical TSV bytes into the real root manifest.
    let mut seals = Vec::new();
    let first_context = trusted_current_context(root, Digest32([0xa5; 32]))
        .expect("first-pass context must be bound to trusted current literals");
    let mut first_pass_count = 0_u64;
    stream_graph_manifest(&graph, first_context, |seal| {
        seals.push(seal.clone());
        first_pass_count += 1;
        heartbeat(root, "first-pass", first_pass_count, total_leaves);
        Ok(())
    })
    .expect("graph must stream into neutral source-leaf records");
    eprintln!(
        "a2b1-freeze root={} phase=first-pass complete leaves={first_pass_count} total={total_leaves}",
        root.canonical_name()
    );
    let leaf_tsv = encode_leaf_seal_tsv(&seals).expect("leaf seals must encode canonically");
    let ledger_sha256 = leaf_seal_tsv_sha256(&leaf_tsv)
        .expect("canonical leaf TSV must have a stable SHA-256 binding");

    let mut verifier = FrozenLeafSealVerifierV1::from_tsv(&leaf_tsv, ledger_sha256)
        .expect("freshly encoded leaf TSV must parse canonically");
    let second_context = trusted_current_context(root, ledger_sha256)
        .expect("second-pass context must be bound to trusted current literals");
    let mut second_pass_count = 0_u64;
    eprintln!(
        "a2b1-freeze root={} phase=second-pass start leaves={first_pass_count} total={total_leaves}",
        root.canonical_name()
    );
    let manifest = stream_graph_manifest(&graph, second_context, |seal| {
        verifier.observe(seal)?;
        second_pass_count += 1;
        heartbeat(root, "second-pass", second_pass_count, total_leaves);
        Ok(())
    })
    .expect("second graph pass must reproduce every compact seal");
    verifier
        .finish()
        .expect("second graph pass must reproduce the whole leaf set");
    eprintln!(
        "a2b1-freeze root={} phase=second-pass complete leaves={second_pass_count} total={total_leaves}",
        root.canonical_name()
    );
    let manifest_tsv = encode_manifest_tsv(&manifest).expect("manifest must encode canonically");

    let stem = root.canonical_name();
    write_leaf_review_candidate(output, root, &leaf_tsv);
    std::fs::write(
        output.join(format!("{stem}.source-leaf-manifest.v1.tsv")),
        manifest_tsv,
    )
    .expect("write manifest review candidate");
    eprintln!(
        "a2b1-freeze root={} phase=write complete leaves={second_pass_count}",
        root.canonical_name()
    );
}

fn write_leaf_review_candidate(output: &Path, root: RootOperationV1, leaf_tsv: &str) {
    match root {
        RootOperationV1::Map => {
            let parts = split_map_leaf_tsv(leaf_tsv)
                .expect("canonical Map leaf TSV must split into 16 complete-line parts");
            for (index, part) in parts.into_iter().enumerate() {
                std::fs::write(output.join(map_leaf_part_name(index)), part)
                    .expect("write Map leaf review candidate part");
            }
        }
        RootOperationV1::Lock => {
            std::fs::write(output.join("lock.source-leaves.v1.tsv"), leaf_tsv)
                .expect("write Lock leaf review candidate");
        }
    }
}

fn split_map_leaf_tsv(input: &str) -> Result<Vec<&str>, String> {
    let expected_header = format!("{}\n", super::LEAF_SEAL_TSV_HEADER_V1);
    let newline_ends = input
        .match_indices('\n')
        .map(|(offset, _)| offset + 1)
        .collect::<Vec<_>>();
    if !input.ends_with('\n')
        || newline_ends.len() <= MAP_LEAF_LEDGER_PARTS
        || input.get(..newline_ends[0]) != Some(expected_header.as_str())
    {
        return Err("Map leaf TSV is not a canonical header plus enough complete rows".to_owned());
    }

    let row_count = newline_ends.len() - 1;
    let rows_per_part = row_count / MAP_LEAF_LEDGER_PARTS;
    let remainder = row_count % MAP_LEAF_LEDGER_PARTS;
    let mut parts = Vec::with_capacity(MAP_LEAF_LEDGER_PARTS);
    let mut start = 0;
    let mut completed_rows = 0;
    for index in 0..MAP_LEAF_LEDGER_PARTS {
        completed_rows += rows_per_part + usize::from(index < remainder);
        let end = newline_ends[completed_rows];
        parts.push(&input[start..end]);
        start = end;
    }
    if start != input.len()
        || parts.len() != MAP_LEAF_LEDGER_PARTS
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.ends_with('\n'))
        || parts
            .iter()
            .skip(1)
            .any(|part| part.starts_with(super::LEAF_SEAL_TSV_HEADER_V1))
        || !parts
            .iter()
            .flat_map(|part| part.as_bytes())
            .copied()
            .eq(input.bytes())
    {
        return Err("Map leaf TSV partition is not an exact ordered byte partition".to_owned());
    }
    Ok(parts)
}

fn map_leaf_part_name(index: usize) -> String {
    format!("map.source-leaves.v1.part-{index:02}.tsv")
}

#[test]
fn map_leaf_partition_is_fixed_balanced_and_byte_exact() {
    let mut monolith = format!("{}\n", super::LEAF_SEAL_TSV_HEADER_V1);
    for index in 0..35 {
        monolith.push_str(&format!(
            "map\tleaf-{index:03}\tterminal\t000\tcase\tfull\n"
        ));
    }
    let parts = split_map_leaf_tsv(&monolith).expect("split canonical sample");
    assert_eq!(parts.len(), MAP_LEAF_LEDGER_PARTS);
    assert_eq!(parts.concat(), monolith);
    assert!(parts[0].starts_with(super::LEAF_SEAL_TSV_HEADER_V1));
    assert!(parts
        .iter()
        .skip(1)
        .all(|part| !part.starts_with(super::LEAF_SEAL_TSV_HEADER_V1)));
    let counts = parts
        .iter()
        .enumerate()
        .map(|(index, part)| part.lines().count() - usize::from(index == 0))
        .collect::<Vec<_>>();
    assert_eq!(counts.iter().sum::<usize>(), 35);
    assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);
    assert_eq!(map_leaf_part_name(0), "map.source-leaves.v1.part-00.tsv");
    assert_eq!(map_leaf_part_name(15), "map.source-leaves.v1.part-15.tsv");
}

fn heartbeat(root: RootOperationV1, phase: &str, count: u64, total: u64) {
    if count % HEARTBEAT_INTERVAL == 0 {
        eprintln!(
            "a2b1-freeze root={} phase={phase} leaves={count} total={total}",
            root.canonical_name()
        );
    }
}

fn require_confirmation() {
    let actual = std::env::var(CONFIRM_ENV).unwrap_or_default();
    assert_eq!(
        actual, CONFIRM_TOKEN,
        "set {CONFIRM_ENV}={CONFIRM_TOKEN} only for an explicit source-review snapshot run"
    );
}

fn output_directory() -> PathBuf {
    let value = std::env::var_os(OUTPUT_ENV).expect("freeze output directory env is required");
    let path = PathBuf::from(value);
    assert!(
        path.is_absolute(),
        "{OUTPUT_ENV} must name an explicit absolute directory"
    );
    std::fs::create_dir_all(&path).expect("create explicit freeze output directory");
    path
}
