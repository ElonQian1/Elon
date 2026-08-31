use super::super::{
    invariants,
    model::ContractGraph,
    source_leaf_authority::{
        validate_lock_graph_with_records_and_binding, validate_map_graph_with_records_and_binding,
        FrozenStaticBindingV1, RootOperationV1, StreamedLeafV1,
    },
    validate_source_owner_authority,
};
use super::{
    build_dynamic_manifest_v1, CatalogErrorV1, DynamicCatalogBuilderV1, DynamicManifestBundleV1,
    ManifestBuildErrorV1,
};

/// Atomic, in-memory candidate generation. No manifest bytes escape unless both frozen-static
/// passes, the exact catalog partition and every manifest invariant have completed successfully.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DynamicCandidateErrorV1 {
    StaticIngress(String),
    StaticBindingDrift,
    Catalog(CatalogErrorV1),
    Manifest(ManifestBuildErrorV1),
    CountOverflow,
}

pub(crate) fn build_map_dynamic_candidate_v1(
    graph: &ContractGraph,
) -> Result<DynamicManifestBundleV1, DynamicCandidateErrorV1> {
    build_dynamic_candidate_v1(graph, RootOperationV1::Map)
}

pub(crate) fn build_lock_dynamic_candidate_v1(
    graph: &ContractGraph,
) -> Result<DynamicManifestBundleV1, DynamicCandidateErrorV1> {
    build_dynamic_candidate_v1(graph, RootOperationV1::Lock)
}

fn build_dynamic_candidate_v1(
    graph: &ContractGraph,
    root: RootOperationV1,
) -> Result<DynamicManifestBundleV1, DynamicCandidateErrorV1> {
    validate_source_owner_authority().map_err(DynamicCandidateErrorV1::StaticIngress)?;
    let invariant_count =
        invariants::validate_graph(graph).map_err(DynamicCandidateErrorV1::StaticIngress)?;

    // Pass one must finish the complete frozen ledger/root-manifest validation before any
    // dynamic record is observed. Pass two repeats the same gate while keeping all candidate
    // state private and discardable until the final binding is proven identical.
    let trusted_binding = validate_frozen_pass(graph, root, |_| Ok(()))?;
    let invariant_count =
        u64::try_from(invariant_count).map_err(|_| DynamicCandidateErrorV1::CountOverflow)?;
    if trusted_binding.included_count != invariant_count {
        return Err(DynamicCandidateErrorV1::StaticBindingDrift);
    }

    let mut catalog = DynamicCatalogBuilderV1::from_frozen_static_binding(&trusted_binding)
        .map_err(DynamicCandidateErrorV1::Catalog)?;
    let mut catalog_error = None;
    let observed_binding = validate_frozen_pass(graph, root, |leaf| {
        catalog.observe(leaf).map_err(|error| {
            catalog_error = Some(error);
            format!("dynamic quotient catalog observer rejected a frozen leaf: {error:?}")
        })
    });
    if let Some(error) = catalog_error {
        return Err(DynamicCandidateErrorV1::Catalog(error));
    }
    let observed_binding = observed_binding?;
    if observed_binding != trusted_binding {
        return Err(DynamicCandidateErrorV1::StaticBindingDrift);
    }

    let catalog = catalog.finish().map_err(DynamicCandidateErrorV1::Catalog)?;
    build_dynamic_manifest_v1(&trusted_binding, &catalog).map_err(DynamicCandidateErrorV1::Manifest)
}

pub(super) fn validate_frozen_pass<F>(
    graph: &ContractGraph,
    root: RootOperationV1,
    observe_leaf: F,
) -> Result<FrozenStaticBindingV1, DynamicCandidateErrorV1>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    match root {
        RootOperationV1::Map => validate_map_graph_with_records_and_binding(graph, observe_leaf),
        RootOperationV1::Lock => validate_lock_graph_with_records_and_binding(graph, observe_leaf),
    }
    .map_err(DynamicCandidateErrorV1::StaticIngress)
}
