use super::super::{
    super::model::{DecisionStage, ExclusionProof},
    builder::MapGraphBuilder,
    witnesses as w,
};

pub(super) fn add_iteration_exclusions(
    graph: &mut MapGraphBuilder,
    iteration: &str,
    prefix: &str,
    ordinal: u16,
) {
    for (suffix, needle, proof) in [
        (
            "node-missing-before-map",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAP",
            "mutex-held node survives post-initialization",
        ),
        (
            "node-missing-during-map",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_MAP",
            "mutex-held node survives loop entry",
        ),
        (
            "region-index-overflow",
            "NODE_MANAGED_SQLITE_SHM_REGION_INDEX_OVERFLOW",
            "authority bounds regions.len() by 256 on 64-bit Windows",
        ),
        (
            "region-offset-overflow",
            "NODE_MANAGED_SQLITE_SHM_REGION_OFFSET_OVERFLOW",
            "accepted region index and size multiply within u64",
        ),
        (
            "view-shift-overflow",
            "NODE_MANAGED_SQLITE_SHM_VIEW_SHIFT_OVERFLOW",
            "shift is below 64-KiB Windows granularity and fits usize",
        ),
        (
            "region-length-overflow",
            "NODE_MANAGED_SQLITE_SHM_REGION_LENGTH_OVERFLOW",
            "accepted u32 region length fits 64-bit usize",
        ),
        (
            "view-length-overflow",
            "NODE_MANAGED_SQLITE_SHM_VIEW_LENGTH_OVERFLOW",
            "shift plus accepted region size is below 128 KiB",
        ),
        (
            "node-missing-before-budget",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_BUDGET",
            "mutex-held node survives checked length arithmetic",
        ),
        (
            "mapped-total-overflow",
            "NODE_MANAGED_SQLITE_SHM_MAPPED_TOTAL_OVERFLOW",
            "24-MiB authority mapped budget keeps accumulation within u64",
        ),
        (
            "node-missing-before-create",
            "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAPPING_CREATE",
            "mutex-held node survives mapped-budget validation",
        ),
    ] {
        add_exclusion(graph, iteration, prefix, ordinal, suffix, needle, proof);
    }
    let budget = format!("{prefix}.ordinal-{ordinal:03}.excluded.mapped-size-budget");
    graph.excluded(
        &budget,
        ExclusionProof::ControlFlow(
            "logical bytes plus at most one sub-granularity shift per region is at most 24 MiB",
        ),
        w::managed_types(
            "fn validate_mapped_total",
            "NODE_MANAGED_SQLITE_SHM_MAPPED_SIZE_BUDGET",
        ),
    );
    graph.edge(
        iteration,
        &budget,
        DecisionStage::ManagedRequest,
        "mapped_size_budget_rejected",
    );
}

fn add_exclusion(
    graph: &mut MapGraphBuilder,
    from: &str,
    prefix: &str,
    ordinal: u16,
    suffix: &str,
    needle: &'static str,
    proof: &'static str,
) {
    let id = format!("{prefix}.ordinal-{ordinal:03}.excluded.{suffix}");
    graph.excluded(&id, ExclusionProof::ControlFlow(proof), w::managed(needle));
    graph.edge(from, &id, DecisionStage::ManagedRequest, suffix);
}

pub(super) fn add_mapping_precondition_exclusions(
    graph: &mut MapGraphBuilder,
    create: &str,
    prefix: &str,
    ordinal: u16,
) {
    for (suffix, needle, proof) in [
        (
            "mapping-size-zero",
            "maximum_size == 0",
            "validated logical_end is nonzero",
        ),
        (
            "mapping-size-above-i64",
            "maximum_size > i64::MAX as u64",
            "validated logical_end is at most 8 MiB",
        ),
    ] {
        let id = format!("{prefix}.ordinal-{ordinal:03}.excluded.{suffix}");
        graph.excluded(
            &id,
            ExclusionProof::ControlFlow(proof),
            w::windows_shm("fn create_mapping", needle),
        );
        graph.edge(create, &id, DecisionStage::NativeCall, suffix);
    }
}

pub(super) fn add_view_precondition_exclusions(
    graph: &mut MapGraphBuilder,
    view: &str,
    prefix: &str,
    ordinal: u16,
) {
    for (suffix, needle, proof) in [
        (
            "view-length-zero",
            "if mapped_length == 0",
            "validated nonzero region length makes mapped_length nonzero",
        ),
        (
            "view-offset-unaligned",
            "if aligned_offset % granularity != 0",
            "aligned_offset is constructed by subtracting offset modulo granularity",
        ),
        (
            "cached-granularity-failed",
            "let granularity = allocation_granularity()?",
            "the same OnceLock granularity already succeeded before the loop",
        ),
    ] {
        let id = format!("{prefix}.ordinal-{ordinal:03}.excluded.{suffix}");
        graph.excluded(
            &id,
            ExclusionProof::ControlFlow(proof),
            w::windows_shm("fn map_view", needle),
        );
        graph.edge(view, &id, DecisionStage::NativeCall, suffix);
    }
}
