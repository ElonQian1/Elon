//! Deterministic merge order for post-lease parser edges across one or more frontier images.

use anyhow::{anyhow, bail, Result};

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportEdgeKind,
    WindowsLoaderModuleEdgeLocator, WindowsPostLeaseModuleEdgeLocator,
};
use super::SealedWindowsRecursiveResolutionClosure;

struct EdgeOrderView<'edge> {
    module_request_ordinal: usize,
    importer_graph_edge_ordinal: usize,
    edge_kind: WindowsLoaderImportEdgeKind,
    edge_locator: &'edge WindowsLoaderModuleEdgeLocator,
}

pub(super) fn validate_recursive_edge_order(
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let edges = final_edges(resolution);
    validate_wave_merge_order(closure, &edges)?;
    for receipt in &closure.parse_receipts {
        let mut importer_edges = edges
            .iter()
            .filter(|edge| {
                matches!(
                    edge.edge_locator,
                    WindowsLoaderModuleEdgeLocator::SystemPostLease {
                        parse_receipt_ordinal,
                        ..
                    } if *parse_receipt_ordinal == receipt.parse_receipt_ordinal
                )
            })
            .collect::<Vec<_>>();
        importer_edges.sort_by_key(|edge| edge.importer_graph_edge_ordinal);
        let expected_count = receipt
            .normal_import_count
            .checked_add(receipt.delay_import_count)
            .and_then(|count| count.checked_add(receipt.forwarder_count))
            .ok_or_else(count_overflow)?;
        if importer_edges.len() != expected_count {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORTER_EDGE_COUNT_CHANGED");
        }
        let mut prior_key = None;
        for (ordinal, edge) in importer_edges.into_iter().enumerate() {
            let key = local_edge_key(edge)?;
            if edge.importer_graph_edge_ordinal != ordinal
                || prior_key.is_some_and(|prior| prior >= key)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORTER_EDGE_ORDER_CHANGED");
            }
            prior_key = Some(key);
        }
    }
    Ok(())
}

fn validate_wave_merge_order(
    closure: &SealedWindowsRecursiveResolutionClosure,
    edges: &[EdgeOrderView<'_>],
) -> Result<()> {
    for wave in &closure.waves {
        let end = wave
            .first_module_request_ordinal
            .checked_add(wave.module_request_count)
            .ok_or_else(count_overflow)?;
        let mut wave_edges = edges
            .iter()
            .filter(|edge| {
                edge.module_request_ordinal >= wave.first_module_request_ordinal
                    && edge.module_request_ordinal < end
            })
            .collect::<Vec<_>>();
        wave_edges.sort_by_key(|edge| edge.module_request_ordinal);
        let mut prior_key = None;
        for edge in wave_edges {
            let WindowsLoaderModuleEdgeLocator::SystemPostLease {
                wave_ordinal,
                parse_receipt_ordinal,
                ..
            } = edge.edge_locator
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_EDGE_STAGE_CHANGED");
            };
            let source_position = wave
                .source_parse_receipt_ordinals
                .binary_search(parse_receipt_ordinal)
                .map_err(|_| {
                    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_EDGE_SOURCE_CHANGED")
                })?;
            let key = (source_position, edge.importer_graph_edge_ordinal);
            if *wave_ordinal != wave.wave_ordinal || prior_key.is_some_and(|prior| prior >= key) {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_EDGE_MERGE_CHANGED");
            }
            prior_key = Some(key);
        }
    }
    Ok(())
}

fn local_edge_key(edge: &EdgeOrderView<'_>) -> Result<(u8, usize, usize)> {
    let WindowsLoaderModuleEdgeLocator::SystemPostLease { locator, .. } = edge.edge_locator else {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORTER_EDGE_STAGE_CHANGED");
    };
    match (edge.edge_kind, locator) {
        (
            WindowsLoaderImportEdgeKind::NormalImport,
            WindowsPostLeaseModuleEdgeLocator::Import {
                descriptor_ordinal,
                thunk_ordinal,
                ..
            },
        ) => Ok((0, *descriptor_ordinal, *thunk_ordinal)),
        (
            WindowsLoaderImportEdgeKind::DelayImport,
            WindowsPostLeaseModuleEdgeLocator::Import {
                descriptor_ordinal,
                thunk_ordinal,
                ..
            },
        ) => Ok((1, *descriptor_ordinal, *thunk_ordinal)),
        (
            WindowsLoaderImportEdgeKind::Forwarder,
            WindowsPostLeaseModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                ..
            },
        ) => Ok((2, *source_import_edge_ordinal, *forwarder_hop_ordinal)),
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORTER_EDGE_LOCATOR_CHANGED"),
    }
}

fn final_edges(resolution: &SealedWindowsLoaderResolutionAuthority) -> Vec<EdgeOrderView<'_>> {
    resolution
        .package_module_bindings
        .iter()
        .map(|binding| EdgeOrderView {
            module_request_ordinal: binding.module_request_ordinal,
            importer_graph_edge_ordinal: binding.importer_graph_edge_ordinal,
            edge_kind: binding.edge_kind,
            edge_locator: &binding.edge_locator,
        })
        .chain(
            resolution
                .system_module_bindings
                .iter()
                .map(|binding| EdgeOrderView {
                    module_request_ordinal: binding.module_request_ordinal,
                    importer_graph_edge_ordinal: binding.importer_graph_edge_ordinal,
                    edge_kind: binding.edge_kind,
                    edge_locator: &binding.edge_locator,
                }),
        )
        .collect()
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
}
