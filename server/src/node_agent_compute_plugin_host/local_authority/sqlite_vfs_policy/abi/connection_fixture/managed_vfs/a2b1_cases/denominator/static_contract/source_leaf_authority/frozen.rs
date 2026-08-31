//! Checked-in Map/Lock source-leaf authority gate.
//!
//! The frozen manifest describes reviewed bytes, but it is never accepted as evidence of the
//! current checkout. We recompute the ledger digest, construct a trusted context from current
//! source/profile/range literals, reject any context drift, and only then traverse the graph.

use super::super::model::ContractGraph;
use super::{
    leaf_seal::{leaf_seal_tsv_sha256, LEAF_SEAL_TSV_HEADER_V1},
    manifest_tsv::parse_manifest_tsv,
    model::{ManifestContextV1, RootManifestV1, RootOperationV1},
    trusted_current_context, validate_graph_against_frozen_with_records, StreamedLeafV1,
    MAP_LEAF_LEDGER_PARTS,
};

const MAP_LEAF_SEAL_PARTS: [&str; MAP_LEAF_LEDGER_PARTS] = [
    include_str!("frozen/map.source-leaves.v1.part-00.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-01.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-02.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-03.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-04.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-05.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-06.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-07.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-08.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-09.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-10.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-11.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-12.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-13.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-14.tsv"),
    include_str!("frozen/map.source-leaves.v1.part-15.tsv"),
];
const MAP_MANIFEST: &str = include_str!("frozen/map.source-leaf-manifest.v1.tsv");
const LOCK_LEAF_SEALS: &str = include_str!("frozen/lock.source-leaves.v1.tsv");
const LOCK_MANIFEST: &str = include_str!("frozen/lock.source-leaf-manifest.v1.tsv");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrozenStaticBindingV1 {
    pub(crate) context: ManifestContextV1,
    pub(crate) included_count: u64,
    pub(crate) excluded_count: u64,
    pub(crate) source_universe_count: u64,
    pub(crate) static_manifest_sha256: super::Digest32,
    pub(crate) included_member_pair_set_sha256: super::Digest32,
}

pub(crate) fn validate_map_graph(graph: &ContractGraph) -> Result<usize, String> {
    validate_map_graph_with_records(graph, |_| Ok(()))
}

pub(crate) fn validate_map_graph_with_records<F>(
    graph: &ContractGraph,
    observe_leaf: F,
) -> Result<usize, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    validate_map_graph_with_records_and_binding(graph, observe_leaf).and_then(binding_count)
}

pub(crate) fn validate_map_graph_with_records_and_binding<F>(
    graph: &ContractGraph,
    observe_leaf: F,
) -> Result<FrozenStaticBindingV1, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    let leaf_seals = concatenate_map_leaf_seal_parts()?;
    validate_root_with_records(
        graph,
        RootOperationV1::Map,
        &leaf_seals,
        MAP_MANIFEST,
        observe_leaf,
    )
}

pub(crate) fn validate_lock_graph(graph: &ContractGraph) -> Result<usize, String> {
    validate_lock_graph_with_records(graph, |_| Ok(()))
}

pub(crate) fn validate_lock_graph_with_records<F>(
    graph: &ContractGraph,
    observe_leaf: F,
) -> Result<usize, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    validate_lock_graph_with_records_and_binding(graph, observe_leaf).and_then(binding_count)
}

pub(crate) fn validate_lock_graph_with_records_and_binding<F>(
    graph: &ContractGraph,
    observe_leaf: F,
) -> Result<FrozenStaticBindingV1, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    validate_root_with_records(
        graph,
        RootOperationV1::Lock,
        LOCK_LEAF_SEALS,
        LOCK_MANIFEST,
        observe_leaf,
    )
}

fn validate_root_with_records<F>(
    graph: &ContractGraph,
    root: RootOperationV1,
    frozen_leaf_seals: &str,
    frozen_manifest_tsv: &str,
    observe_leaf: F,
) -> Result<FrozenStaticBindingV1, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    let (trusted_context, frozen_manifest) =
        load_trusted_frozen_pair(root, frozen_leaf_seals, frozen_manifest_tsv)?;
    let mut included_members = Vec::new();
    let mut observe_leaf = observe_leaf;
    let included = validate_graph_against_frozen_with_records(
        graph,
        trusted_context.clone(),
        frozen_leaf_seals,
        &frozen_manifest,
        |leaf| {
            if let StreamedLeafV1::Terminal { seal, .. } = &leaf {
                included_members.push((seal.case_key_sha256, seal.full_record_sha256));
            }
            observe_leaf(leaf)
        },
    )?;
    if included != frozen_manifest.included_count {
        return Err(format!(
            "{} frozen ingress count {} differs from manifest {}",
            root.canonical_name(),
            included,
            frozen_manifest.included_count
        ));
    }
    Ok(FrozenStaticBindingV1 {
        context: trusted_context,
        included_count: frozen_manifest.included_count,
        excluded_count: frozen_manifest.excluded_count,
        source_universe_count: graph.source_leaf_universe.len() as u64,
        static_manifest_sha256: frozen_manifest.manifest_sha256,
        included_member_pair_set_sha256: super::digest_included_member_pair_set(included_members),
    })
}

fn binding_count(binding: FrozenStaticBindingV1) -> Result<usize, String> {
    usize::try_from(binding.included_count).map_err(|_| {
        format!(
            "{} frozen included count does not fit usize: {included}",
            binding.context.root.canonical_name(),
            included = binding.included_count,
        )
    })
}

fn load_trusted_frozen_pair(
    root: RootOperationV1,
    frozen_leaf_seals: &str,
    frozen_manifest_tsv: &str,
) -> Result<(ManifestContextV1, RootManifestV1), String> {
    let frozen_manifest = parse_manifest_tsv(frozen_manifest_tsv).map_err(|error| {
        format!(
            "{} frozen manifest is invalid: {error}",
            root.canonical_name()
        )
    })?;
    let ledger_sha256 = leaf_seal_tsv_sha256(frozen_leaf_seals).map_err(|error| {
        format!(
            "{} frozen source-leaf ledger is invalid: {error}",
            root.canonical_name()
        )
    })?;

    // This is intentionally constructed from live, independently reviewed literals. The context
    // parsed from the frozen manifest must never be passed off as the current authority context.
    let trusted_context = trusted_current_context(root, ledger_sha256).map_err(|error| {
        format!(
            "{} trusted current authority context is invalid: {error}",
            root.canonical_name()
        )
    })?;
    require_frozen_context_matches_current(&frozen_manifest, &trusted_context)?;
    Ok((trusted_context, frozen_manifest))
}

fn concatenate_map_leaf_seal_parts() -> Result<String, String> {
    let mut total_bytes = 0_usize;
    for (index, part) in MAP_LEAF_SEAL_PARTS.iter().enumerate() {
        if part.is_empty()
            || !part.ends_with('\n')
            || part.contains('\r')
            || part.starts_with('\u{feff}')
            || (index == 0
                && !part
                    .strip_prefix(LEAF_SEAL_TSV_HEADER_V1)
                    .and_then(|rest| rest.strip_prefix('\n'))
                    .is_some_and(|rest| rest.starts_with("map\t")))
            || (index != 0 && !part.starts_with("map\t"))
        {
            return Err(format!(
                "Map frozen source-leaf part {index:02} is empty or not a complete canonical line partition"
            ));
        }
        total_bytes = total_bytes
            .checked_add(part.len())
            .ok_or("Map frozen source-leaf parts exceed addressable bytes")?;
    }
    let header_rows = MAP_LEAF_SEAL_PARTS
        .iter()
        .flat_map(|part| part.lines())
        .filter(|line| *line == LEAF_SEAL_TSV_HEADER_V1)
        .count();
    if header_rows != 1 {
        return Err(format!(
            "Map frozen source-leaf parts contain {header_rows} headers instead of exactly one"
        ));
    }
    let mut joined = String::with_capacity(total_bytes);
    for part in MAP_LEAF_SEAL_PARTS {
        joined.push_str(part);
    }
    if joined.len() != total_bytes {
        return Err("Map frozen source-leaf concatenation length drifted".to_owned());
    }
    Ok(joined)
}

fn require_frozen_context_matches_current(
    frozen_manifest: &RootManifestV1,
    trusted_current: &ManifestContextV1,
) -> Result<(), String> {
    if &frozen_manifest.context == trusted_current {
        return Ok(());
    }
    Err(format!(
        "{} frozen manifest context differs from trusted current authority; frozen_manifest_sha256={}",
        trusted_current.root.canonical_name(),
        frozen_manifest.manifest_sha256.to_lower_hex(),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{Digest32, ShardManifestV1};
    use super::*;

    fn manifest(context: ManifestContextV1) -> RootManifestV1 {
        RootManifestV1 {
            context,
            included_count: 1,
            excluded_count: 0,
            source_leaf_identity_set_sha256: Digest32([2; 32]),
            case_key_set_sha256: Digest32([3; 32]),
            source_branch_map_sha256: Digest32([4; 32]),
            expected_map_sha256: Digest32([5; 32]),
            exclusion_map_sha256: Digest32([6; 32]),
            full_record_set_sha256: Digest32([7; 32]),
            shards: (0..=u8::MAX)
                .map(|index| ShardManifestV1 {
                    index,
                    included_count: u64::from(index == 0),
                    excluded_count: 0,
                    source_leaf_identity_set_sha256: Digest32([8; 32]),
                    case_key_set_sha256: Digest32([9; 32]),
                    source_branch_map_sha256: Digest32([10; 32]),
                    expected_map_sha256: Digest32([11; 32]),
                    exclusion_map_sha256: Digest32([12; 32]),
                    full_record_set_sha256: Digest32([13; 32]),
                })
                .collect(),
            manifest_sha256: Digest32([14; 32]),
        }
    }

    fn context(root: RootOperationV1) -> ManifestContextV1 {
        let (map_profiles, map_ordinals, lock_ranges, lock_count) = match root {
            RootOperationV1::Map => (
                Some(Digest32([16; 32])),
                Some(Digest32([17; 32])),
                None,
                None,
            ),
            RootOperationV1::Lock => (None, None, Some(Digest32([18; 32])), Some(88)),
        };
        ManifestContextV1 {
            schema: "schema".to_owned(),
            root,
            target_scope: "target".to_owned(),
            source_baseline_commit_sha1: "a".repeat(40),
            source_scope_sha256: Digest32([1; 32]),
            ledger_sha256: Digest32([15; 32]),
            map_profile_set_sha256: map_profiles,
            map_ordinal_domain_sha256: map_ordinals,
            lock_range_set_sha256: lock_ranges,
            lock_range_count: lock_count,
        }
    }

    #[test]
    fn frozen_context_gate_rejects_drift_before_graph_validation() {
        let trusted = context(RootOperationV1::Map);
        let mut frozen = manifest(trusted.clone());
        assert_eq!(
            require_frozen_context_matches_current(&frozen, &trusted),
            Ok(())
        );

        frozen.context.source_scope_sha256 = Digest32([99; 32]);
        let error = require_frozen_context_matches_current(&frozen, &trusted).unwrap_err();
        assert!(error.contains("differs from trusted current authority"));
    }

    #[test]
    fn frozen_context_gate_rejects_ledger_and_domain_drift() {
        let trusted = context(RootOperationV1::Lock);
        let mut frozen = manifest(trusted.clone());
        frozen.context.ledger_sha256 = Digest32([98; 32]);
        assert!(require_frozen_context_matches_current(&frozen, &trusted).is_err());

        frozen.context = trusted.clone();
        frozen.context.lock_range_count = Some(89);
        assert!(require_frozen_context_matches_current(&frozen, &trusted).is_err());

        frozen.context = trusted.clone();
        frozen.context.lock_range_set_sha256 = Some(Digest32([97; 32]));
        assert!(require_frozen_context_matches_current(&frozen, &trusted).is_err());

        let trusted = context(RootOperationV1::Map);
        frozen = manifest(trusted.clone());
        frozen.context.map_profile_set_sha256 = Some(Digest32([96; 32]));
        assert!(require_frozen_context_matches_current(&frozen, &trusted).is_err());

        frozen.context = trusted.clone();
        frozen.context.map_ordinal_domain_sha256 = Some(Digest32([95; 32]));
        assert!(require_frozen_context_matches_current(&frozen, &trusted).is_err());
    }

    #[test]
    fn checked_in_pairs_bind_trusted_current_context_without_graph_traversal() {
        let map_leaf_seals = concatenate_map_leaf_seal_parts()
            .expect("Map frozen parts form one canonical leaf ledger");
        let (_, map_manifest) =
            load_trusted_frozen_pair(RootOperationV1::Map, &map_leaf_seals, MAP_MANIFEST)
                .expect("Map frozen pair binds trusted current literals");
        let (_, lock_manifest) =
            load_trusted_frozen_pair(RootOperationV1::Lock, LOCK_LEAF_SEALS, LOCK_MANIFEST)
                .expect("Lock frozen pair binds trusted current literals");
        assert!(map_manifest.included_count > 0);
        assert!(lock_manifest.included_count > 0);
    }

    #[test]
    fn map_frozen_leaf_parts_form_one_fixed_canonical_monolith() {
        assert_eq!(MAP_LEAF_SEAL_PARTS.len(), 16);
        let joined = concatenate_map_leaf_seal_parts()
            .expect("Map frozen parts form one complete-line concatenation");
        leaf_seal_tsv_sha256(&joined).expect("concatenated Map leaf TSV is canonical");
        assert_eq!(
            joined
                .lines()
                .filter(|line| *line == LEAF_SEAL_TSV_HEADER_V1)
                .count(),
            1
        );
    }
}
