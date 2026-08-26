//! Exact stage, source-edge, forwarder-chain and wave provenance for recursive final edges.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::launch_path_discovery::{
        symbol_is_exact, WindowsPreliminaryModuleEdgeLocator,
    },
};

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportEdgeKind,
    WindowsLoaderModuleEdgeLocator, WindowsLoaderModuleNode, WindowsPostLeaseModuleEdgeLocator,
};
use super::{SealedWindowsRecursiveResolutionClosure, WindowsRecursiveResolutionWavePlan};

struct FinalBindingView<'binding> {
    module_request_ordinal: usize,
    global_import_edge_ordinal: usize,
    edge_locator: &'binding WindowsLoaderModuleEdgeLocator,
    importer_parsed_image_ordinal: usize,
    importer: &'binding WindowsLoaderModuleNode,
    edge_kind: WindowsLoaderImportEdgeKind,
    imported_symbol_name: Option<&'binding str>,
    imported_symbol_ordinal: Option<u16>,
    target: WindowsLoaderModuleNode,
    filesystem_image_request_ordinal: Option<usize>,
}

#[derive(Clone, PartialEq, Eq)]
struct ExactSymbol {
    name: Option<String>,
    ordinal: Option<u16>,
}

struct ImportTarget {
    node: WindowsLoaderModuleNode,
    symbol: ExactSymbol,
}

struct ForwarderHop {
    hop_ordinal: usize,
    importer: WindowsLoaderModuleNode,
    source_symbol: ExactSymbol,
    target: WindowsLoaderModuleNode,
    target_symbol: ExactSymbol,
}

pub(super) fn validate_final_edge_provenance(
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let mut bindings = final_binding_views(resolution);
    bindings.sort_by_key(|binding| binding.module_request_ordinal);
    let import_targets = collect_import_targets(&bindings)?;
    let mut import_slots = HashSet::new();
    let mut forwarder_hops: HashMap<usize, Vec<ForwarderHop>> = HashMap::new();

    for (ordinal, binding) in bindings.iter().enumerate() {
        if binding.module_request_ordinal != ordinal
            || binding.global_import_edge_ordinal != ordinal
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_ORDER_CHANGED");
        }
        match binding.edge_locator {
            WindowsLoaderModuleEdgeLocator::BasePrelease {
                preliminary_request_ordinal,
                import_edge_cross_binding_ordinal,
                locator,
            } if ordinal < closure.base_module_request_count => {
                if *preliminary_request_ordinal != ordinal
                    || *import_edge_cross_binding_ordinal != ordinal
                    || binding
                        .filesystem_image_request_ordinal
                        .is_some_and(|request| request >= closure.base_system_image_request_count)
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_EDGE_CHANGED");
                }
                record_base_forwarder(locator, binding, &mut forwarder_hops)?;
            }
            WindowsLoaderModuleEdgeLocator::SystemPostLease {
                wave_ordinal,
                source_parsed_image_ordinal,
                parse_receipt_ordinal,
                locator,
            } if ordinal >= closure.base_module_request_count => {
                let Some(wave) = wave_for_module_request(closure, ordinal) else {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_WAVE_MISSING");
                };
                let Some(receipt) = closure.parse_receipts.get(*parse_receipt_ordinal) else {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_PARSE_RECEIPT_MISSING");
                };
                let system_image_end = wave
                    .first_system_image_request_ordinal
                    .checked_add(wave.system_image_request_count)
                    .ok_or_else(count_overflow)?;
                if *wave_ordinal != wave.wave_ordinal
                    || !wave
                        .source_parse_receipt_ordinals
                        .contains(parse_receipt_ordinal)
                    || receipt.parsed_image_ordinal != *source_parsed_image_ordinal
                    || binding.importer_parsed_image_ordinal != *source_parsed_image_ordinal
                    || binding.importer != &receipt.node
                    || binding
                        .filesystem_image_request_ordinal
                        .is_some_and(|request| request >= system_image_end)
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_SOURCE_CHANGED");
                }
                record_postlease_locator(locator, binding, &mut import_slots, &mut forwarder_hops)?;
            }
            _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_STAGE_CHANGED"),
        }
    }

    super::edge_order::validate_recursive_edge_order(closure, resolution)?;
    validate_forwarder_chains(&import_targets, forwarder_hops)?;
    validate_new_system_owner_waves(closure, resolution)
}

pub(super) fn maximum_forwarder_hop_depth(
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<usize> {
    final_binding_views(resolution)
        .into_iter()
        .filter_map(|binding| match binding.edge_locator {
            WindowsLoaderModuleEdgeLocator::BasePrelease {
                locator:
                    WindowsPreliminaryModuleEdgeLocator::Forwarder {
                        forwarder_hop_ordinal,
                        ..
                    },
                ..
            }
            | WindowsLoaderModuleEdgeLocator::SystemPostLease {
                locator:
                    WindowsPostLeaseModuleEdgeLocator::Forwarder {
                        forwarder_hop_ordinal,
                        ..
                    },
                ..
            } => Some(*forwarder_hop_ordinal),
            _ => None,
        })
        .try_fold(0usize, |depth, ordinal| {
            ordinal
                .checked_add(1)
                .map(|candidate| depth.max(candidate))
                .ok_or_else(count_overflow)
        })
}

fn collect_import_targets(
    bindings: &[FinalBindingView<'_>],
) -> Result<HashMap<usize, ImportTarget>> {
    let mut targets = HashMap::new();
    for binding in bindings.iter().filter(|binding| {
        matches!(
            binding.edge_kind,
            WindowsLoaderImportEdgeKind::NormalImport | WindowsLoaderImportEdgeKind::DelayImport
        )
    }) {
        let symbol = exact_symbol(
            binding.imported_symbol_name,
            binding.imported_symbol_ordinal,
        )?;
        if targets
            .insert(
                binding.global_import_edge_ordinal,
                ImportTarget {
                    node: binding.target.clone(),
                    symbol,
                },
            )
            .is_some()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORT_ROOT_CHANGED");
        }
    }
    Ok(targets)
}

fn final_binding_views(
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Vec<FinalBindingView<'_>> {
    resolution
        .package_module_bindings
        .iter()
        .map(|binding| FinalBindingView {
            module_request_ordinal: binding.module_request_ordinal,
            global_import_edge_ordinal: binding.global_import_edge_ordinal,
            edge_locator: &binding.edge_locator,
            importer_parsed_image_ordinal: binding.importer_parsed_image_ordinal,
            importer: &binding.importer,
            edge_kind: binding.edge_kind,
            imported_symbol_name: binding.imported_symbol_name.as_deref(),
            imported_symbol_ordinal: binding.imported_symbol_ordinal,
            target: WindowsLoaderModuleNode::PackageFile {
                package_file_ordinal: binding.resolved_package_file_ordinal,
            },
            filesystem_image_request_ordinal: None,
        })
        .chain(resolution.system_module_bindings.iter().map(|binding| {
            FinalBindingView {
                module_request_ordinal: binding.module_request_ordinal,
                global_import_edge_ordinal: binding.global_import_edge_ordinal,
                edge_locator: &binding.edge_locator,
                importer_parsed_image_ordinal: binding.importer_parsed_image_ordinal,
                importer: &binding.importer,
                edge_kind: binding.edge_kind,
                imported_symbol_name: binding.imported_symbol_name.as_deref(),
                imported_symbol_ordinal: binding.imported_symbol_ordinal,
                target: super::super::super::pe_graph_validation::system_binding_target_node(
                    binding,
                ),
                filesystem_image_request_ordinal: binding
                    .filesystem_image_ref
                    .as_ref()
                    .map(|image_ref| image_ref.resolution_request_ordinal),
            }
        }))
        .collect()
}

fn record_base_forwarder(
    locator: &WindowsPreliminaryModuleEdgeLocator,
    binding: &FinalBindingView<'_>,
    forwarder_hops: &mut HashMap<usize, Vec<ForwarderHop>>,
) -> Result<()> {
    match (locator, binding.edge_kind) {
        (
            WindowsPreliminaryModuleEdgeLocator::Import { .. },
            WindowsLoaderImportEdgeKind::NormalImport | WindowsLoaderImportEdgeKind::DelayImport,
        ) => Ok(()),
        (
            WindowsPreliminaryModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                source_export_name,
                source_export_ordinal,
                hop_evidence_digest,
            },
            WindowsLoaderImportEdgeKind::Forwarder,
        ) => record_forwarder_hop(
            *source_import_edge_ordinal,
            *forwarder_hop_ordinal,
            source_export_name.as_deref(),
            *source_export_ordinal,
            hop_evidence_digest,
            binding,
            forwarder_hops,
        ),
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_LOCATOR_CHANGED"),
    }
}

fn record_postlease_locator(
    locator: &WindowsPostLeaseModuleEdgeLocator,
    binding: &FinalBindingView<'_>,
    import_slots: &mut HashSet<(usize, u8, usize, usize)>,
    forwarder_hops: &mut HashMap<usize, Vec<ForwarderHop>>,
) -> Result<()> {
    let target_symbol = exact_symbol(
        binding.imported_symbol_name,
        binding.imported_symbol_ordinal,
    )?;
    match (locator, binding.edge_kind) {
        (
            WindowsPostLeaseModuleEdgeLocator::Import {
                source_import_edge_ordinal,
                descriptor_ordinal,
                thunk_ordinal,
                edge_evidence_digest,
            },
            WindowsLoaderImportEdgeKind::NormalImport | WindowsLoaderImportEdgeKind::DelayImport,
        ) if *source_import_edge_ordinal == binding.global_import_edge_ordinal
            && is_sha256(edge_evidence_digest)
            && import_slots.insert((
                binding.importer_parsed_image_ordinal,
                direct_import_rank(binding.edge_kind),
                *descriptor_ordinal,
                *thunk_ordinal,
            )) =>
        {
            Ok(())
        }
        (
            WindowsPostLeaseModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                source_export_name,
                source_export_ordinal,
                hop_evidence_digest,
            },
            WindowsLoaderImportEdgeKind::Forwarder,
        ) if *source_import_edge_ordinal < binding.global_import_edge_ordinal => {
            let source_symbol =
                exact_symbol(source_export_name.as_deref(), *source_export_ordinal)?;
            if !is_sha256(hop_evidence_digest) {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_LOCATOR_CHANGED");
            }
            forwarder_hops
                .entry(*source_import_edge_ordinal)
                .or_default()
                .push(ForwarderHop {
                    hop_ordinal: *forwarder_hop_ordinal,
                    importer: binding.importer.clone(),
                    source_symbol,
                    target: binding.target.clone(),
                    target_symbol,
                });
            Ok(())
        }
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_LOCATOR_CHANGED"),
    }
}

fn direct_import_rank(edge_kind: WindowsLoaderImportEdgeKind) -> u8 {
    match edge_kind {
        WindowsLoaderImportEdgeKind::NormalImport => 0,
        WindowsLoaderImportEdgeKind::DelayImport => 1,
        WindowsLoaderImportEdgeKind::Forwarder => 2,
    }
}

fn record_forwarder_hop(
    source_import_edge_ordinal: usize,
    forwarder_hop_ordinal: usize,
    source_export_name: Option<&str>,
    source_export_ordinal: Option<u16>,
    evidence_digest: &str,
    binding: &FinalBindingView<'_>,
    forwarder_hops: &mut HashMap<usize, Vec<ForwarderHop>>,
) -> Result<()> {
    if source_import_edge_ordinal >= binding.global_import_edge_ordinal
        || !is_sha256(evidence_digest)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_LOCATOR_CHANGED");
    }
    forwarder_hops
        .entry(source_import_edge_ordinal)
        .or_default()
        .push(ForwarderHop {
            hop_ordinal: forwarder_hop_ordinal,
            importer: binding.importer.clone(),
            source_symbol: exact_symbol(source_export_name, source_export_ordinal)?,
            target: binding.target.clone(),
            target_symbol: exact_symbol(
                binding.imported_symbol_name,
                binding.imported_symbol_ordinal,
            )?,
        });
    Ok(())
}

fn validate_forwarder_chains(
    import_targets: &HashMap<usize, ImportTarget>,
    mut forwarder_hops: HashMap<usize, Vec<ForwarderHop>>,
) -> Result<()> {
    for (source_import_edge_ordinal, hops) in &mut forwarder_hops {
        let Some(root) = import_targets.get(source_import_edge_ordinal) else {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_ROOT_MISSING");
        };
        hops.sort_by_key(|hop| hop.hop_ordinal);
        let mut expected_node = root.node.clone();
        let mut expected_symbol = root.symbol.clone();
        let mut visited = vec![(expected_node.clone(), expected_symbol.clone())];
        for (ordinal, hop) in hops.iter().enumerate() {
            if hop.hop_ordinal != ordinal
                || hop.importer != expected_node
                || hop.source_symbol != expected_symbol
                || visited.contains(&(hop.target.clone(), hop.target_symbol.clone()))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_CHAIN_CHANGED");
            }
            expected_node = hop.target.clone();
            expected_symbol = hop.target_symbol.clone();
            visited.push((expected_node.clone(), expected_symbol.clone()));
        }
    }
    Ok(())
}

fn exact_symbol(name: Option<&str>, ordinal: Option<u16>) -> Result<ExactSymbol> {
    if !symbol_is_exact(name, ordinal) {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SYMBOL_CHANGED");
    }
    Ok(ExactSymbol {
        name: name.map(str::to_owned),
        ordinal,
    })
}

fn validate_new_system_owner_waves(
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    for wave in &closure.waves {
        let module_end = wave
            .first_module_request_ordinal
            .checked_add(wave.module_request_count)
            .ok_or_else(count_overflow)?;
        let system_image_end = wave
            .first_system_image_request_ordinal
            .checked_add(wave.system_image_request_count)
            .ok_or_else(count_overflow)?;
        for request_ordinal in wave.first_system_image_request_ordinal..system_image_end {
            if !resolution.system_module_bindings.iter().any(|binding| {
                binding.module_request_ordinal >= wave.first_module_request_ordinal
                    && binding.module_request_ordinal < module_end
                    && binding
                        .filesystem_image_ref
                        .as_ref()
                        .is_some_and(|image_ref| {
                            image_ref.resolution_request_ordinal == request_ordinal
                        })
            }) {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SYSTEM_OWNER_WAVE_CHANGED");
            }
        }
    }
    Ok(())
}

fn wave_for_module_request(
    closure: &SealedWindowsRecursiveResolutionClosure,
    module_request_ordinal: usize,
) -> Option<&WindowsRecursiveResolutionWavePlan> {
    closure.waves.iter().find(|wave| {
        wave.first_module_request_ordinal
            .checked_add(wave.module_request_count)
            .is_some_and(|end| {
                module_request_ordinal >= wave.first_module_request_ordinal
                    && module_request_ordinal < end
            })
    })
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
}
