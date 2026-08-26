use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::super::{SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportBindingRef};
use super::source_projection::{
    expected_frontier_for_range, source_owner_matches_producer_binding,
};
use super::*;

impl SealedWindowsRecursiveResolutionClosure {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_against(
        &self,
        base_prelease_parsed_image_count: usize,
        base_module_request_count: usize,
        base_searched_name_count: usize,
        base_system_image_request_count: usize,
        expected_parser_policy_digest: &str,
        expected_limit_policy_source_digest: &str,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        self.validate_digests(expected_parser_policy_digest, resolution)?;
        if self.base_prelease_parsed_image_count != base_prelease_parsed_image_count
            || self.base_module_request_count != base_module_request_count
            || self.base_searched_name_count != base_searched_name_count
            || self.base_system_image_request_count != base_system_image_request_count
            || self.limit_policy_source_digest != expected_limit_policy_source_digest
            || self.waves.len() > self.max_wave_count
            || resolution.pe_import_graph.parsed_images.len() > self.max_parsed_image_count
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_CLOSURE_BASE_CHANGED");
        }

        let (module_count, searched_name_count, system_image_count) = self.validate_waves()?;
        let final_module_count = resolution
            .package_module_bindings
            .len()
            .checked_add(resolution.system_module_bindings.len())
            .ok_or_else(count_overflow)?;
        if module_count != final_module_count
            || searched_name_count != resolution.searched_names.len()
            || system_image_count != resolution.resolved_filesystem_system_images.len()
            || module_count > self.max_module_request_count
            || searched_name_count > self.max_searched_name_count
            || system_image_count > self.max_system_image_request_count
            || edge_projection::maximum_forwarder_hop_depth(resolution)?
                > self.max_forwarder_hop_count
            || self
                .base_prelease_parsed_image_count
                .checked_add(self.parse_receipts.len())
                .is_none_or(|count| count != resolution.pe_import_graph.parsed_images.len())
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_CLOSURE_FINAL_COUNT_CHANGED");
        }

        self.validate_parse_receipts(resolution)?;
        edge_projection::validate_final_edge_provenance(self, resolution)?;
        self.validate_recursive_search_projection(resolution)
    }

    fn validate_digests(
        &self,
        expected_parser_policy_digest: &str,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        for receipt in &self.parse_receipts {
            if receipt.parser_policy_digest != expected_parser_policy_digest
                || receipt.receipt_digest != digest::parse_receipt_digest(receipt)?
                || [
                    &receipt.source_owner_binding_digest,
                    &receipt.image_material_identity_digest,
                    &receipt.parser_policy_digest,
                    &receipt.import_table_digest,
                    &receipt.same_owner_parse_receipt_digest,
                    &receipt.receipt_digest,
                ]
                .into_iter()
                .any(|value| !is_sha256(value))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PARSE_RECEIPT_DIGEST_CHANGED");
            }
        }
        for wave in &self.waves {
            if wave.wave_digest != digest::wave_digest(wave)?
                || wave.parsed_edge_set_digest
                    != projection_digest::edge_set_digest(wave, resolution)?
                || wave.searched_name_disposition_set_digest
                    != projection_digest::searched_name_set_digest(wave, resolution)?
                || wave.acquired_system_image_set_digest
                    != projection_digest::system_image_set_digest(wave, resolution)?
                || [
                    &wave.parsed_edge_set_digest,
                    &wave.searched_name_disposition_set_digest,
                    &wave.acquired_system_image_set_digest,
                    &wave.wave_digest,
                ]
                .into_iter()
                .any(|value| !is_sha256(value))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_DIGEST_CHANGED");
            }
        }
        if self.closure_digest != digest::closure_digest(self)?
            || [
                &self.limit_policy_source_digest,
                &self.file_identity_dedupe_receipt_digest,
                &self.module_cache_collision_closure_receipt_digest,
                &self.forwarder_cycle_closure_receipt_digest,
                &self.terminal_empty_frontier_receipt_digest,
                &self.closure_digest,
            ]
            .into_iter()
            .any(|value| !is_sha256(value))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_CLOSURE_DIGEST_CHANGED");
        }
        Ok(())
    }

    fn validate_waves(&self) -> Result<(usize, usize, usize)> {
        let mut next_module = self.base_module_request_count;
        let mut next_name = self.base_searched_name_count;
        let mut next_system_image = self.base_system_image_request_count;
        let mut used_receipts = HashSet::new();
        let mut expected_frontier: Option<&[usize]> = None;

        for (index, wave) in self.waves.iter().enumerate() {
            let wave_ordinal = index.checked_add(1).ok_or_else(count_overflow)?;
            if wave.wave_ordinal != wave_ordinal
                || wave.source_parse_receipt_ordinals.is_empty()
                || !strictly_increasing(&wave.source_parse_receipt_ordinals)
                || !strictly_increasing(&wave.next_frontier_parse_receipt_ordinals)
                || expected_frontier
                    .is_some_and(|frontier| frontier != wave.source_parse_receipt_ordinals)
                || wave.first_module_request_ordinal != next_module
                || wave.first_searched_name_ordinal != next_name
                || wave.first_system_image_request_ordinal != next_system_image
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_ORDER_CHANGED");
            }
            for receipt_ordinal in &wave.source_parse_receipt_ordinals {
                if !used_receipts.insert(*receipt_ordinal)
                    || !self
                        .parse_receipts
                        .get(*receipt_ordinal)
                        .is_some_and(|receipt| receipt.wave_ordinal == wave_ordinal)
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_FRONTIER_CHANGED");
                }
            }
            next_module = next_module
                .checked_add(wave.module_request_count)
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
                })?;
            next_name = next_name
                .checked_add(wave.searched_name_count)
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
                })?;
            next_system_image = next_system_image
                .checked_add(wave.system_image_request_count)
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
                })?;
            expected_frontier = Some(&wave.next_frontier_parse_receipt_ordinals);
        }
        if used_receipts.len() != self.parse_receipts.len()
            || self
                .waves
                .last()
                .is_some_and(|wave| !wave.next_frontier_parse_receipt_ordinals.is_empty())
            || (self.waves.is_empty() && !self.parse_receipts.is_empty())
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FIXPOINT_CHANGED");
        }
        Ok((next_module, next_name, next_system_image))
    }

    fn validate_parse_receipts(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        let base_final_ordinals = resolution
            .pe_import_graph
            .pre_post_cross_binding
            .parsed_image_cross_bindings
            .iter()
            .map(|cross| cross.postlease_parsed_image_ordinal)
            .collect::<HashSet<_>>();
        if base_final_ordinals.len() != self.base_prelease_parsed_image_count {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_IMAGE_COVERAGE_CHANGED");
        }
        let mut recursive_final_ordinals = HashSet::new();
        for (ordinal, receipt) in self.parse_receipts.iter().enumerate() {
            let Some(parsed) = resolution
                .pe_import_graph
                .parsed_images
                .get(receipt.parsed_image_ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PARSED_IMAGE_MISSING");
            };
            if receipt.parse_receipt_ordinal != ordinal
                || parsed.parsed_image_ordinal != receipt.parsed_image_ordinal
                || parsed.node != receipt.node
                || parsed.source_binding_digest != receipt.receipt_digest
                || parsed.image_material_identity_digest != receipt.image_material_identity_digest
                || parsed.import_table_digest != receipt.import_table_digest
                || parsed.normal_import_count != receipt.normal_import_count
                || parsed.delay_import_count != receipt.delay_import_count
                || parsed.forwarder_count != receipt.forwarder_count
                || parsed.source
                    != (WindowsPeParsedImageSource::RecursiveExpansion {
                        parse_receipt_ordinal: ordinal,
                    })
                || base_final_ordinals.contains(&receipt.parsed_image_ordinal)
                || !recursive_final_ordinals.insert(receipt.parsed_image_ordinal)
                || !source_owner_matches_producer_binding(
                    &receipt.source_owner,
                    &receipt.node,
                    receipt.producer_module_request_ordinal,
                    resolution,
                )
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PARSE_SOURCE_CHANGED");
            }
        }
        for cross in &resolution
            .pe_import_graph
            .pre_post_cross_binding
            .parsed_image_cross_bindings
        {
            let Some(parsed) = resolution
                .pe_import_graph
                .parsed_images
                .get(cross.postlease_parsed_image_ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_IMAGE_MISSING");
            };
            if parsed.source
                != (WindowsPeParsedImageSource::BasePreleasePackage {
                    prelease_parsed_image_ordinal: cross.prelease_parsed_image_ordinal,
                })
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_SOURCE_CHANGED");
            }
        }
        self.validate_frontier_projection(resolution)?;
        Ok(())
    }

    fn validate_frontier_projection(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        let mut first_request = 0;
        let mut request_count = self.base_module_request_count;
        for producer_wave_ordinal in 0..=self.waves.len() {
            let next_wave_ordinal = producer_wave_ordinal
                .checked_add(1)
                .ok_or_else(count_overflow)?;
            let expected = expected_frontier_for_range(
                first_request,
                request_count,
                next_wave_ordinal,
                resolution,
            )?;
            let actual = if producer_wave_ordinal == 0 {
                self.waves
                    .first()
                    .map(|wave| wave.source_parse_receipt_ordinals.as_slice())
                    .unwrap_or_default()
            } else {
                self.waves[producer_wave_ordinal - 1]
                    .next_frontier_parse_receipt_ordinals
                    .as_slice()
            };
            if expected != actual {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_PROJECTION_CHANGED");
            }
            let Some(wave) = self.waves.get(producer_wave_ordinal) else {
                break;
            };
            first_request = wave.first_module_request_ordinal;
            request_count = wave.module_request_count;
        }
        Ok(())
    }

    fn validate_recursive_search_projection(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        for sequence in resolution
            .pe_import_graph
            .search_sequences
            .iter()
            .skip(self.base_module_request_count)
        {
            let Some(module_request_ordinal) =
                import_binding_module_request_ordinal(&sequence.import_binding, resolution)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_BINDING_MISSING");
            };
            let Some(wave) = self.wave_for_module_request(module_request_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_WAVE_MISSING");
            };
            let searched_end = wave
                .first_searched_name_ordinal
                .checked_add(wave.searched_name_count)
                .ok_or_else(count_overflow)?;
            if sequence.sequence_ordinal != module_request_ordinal
                || sequence.searched_name_ordinals.iter().any(|ordinal| {
                    *ordinal < wave.first_searched_name_ordinal || *ordinal >= searched_end
                })
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_SEQUENCE_CHANGED");
            }
        }
        Ok(())
    }

    fn wave_for_module_request(
        &self,
        module_request_ordinal: usize,
    ) -> Option<&WindowsRecursiveResolutionWavePlan> {
        self.waves.iter().find(|wave| {
            wave.first_module_request_ordinal
                .checked_add(wave.module_request_count)
                .is_some_and(|end| {
                    module_request_ordinal >= wave.first_module_request_ordinal
                        && module_request_ordinal < end
                })
        })
    }
}

fn strictly_increasing(values: &[usize]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn count_overflow() -> anyhow::Error {
    anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
}

fn import_binding_module_request_ordinal(
    binding: &WindowsLoaderImportBindingRef,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Option<usize> {
    match binding {
        WindowsLoaderImportBindingRef::Package { binding_ordinal } => resolution
            .package_module_bindings
            .get(*binding_ordinal)
            .map(|binding| binding.module_request_ordinal),
        WindowsLoaderImportBindingRef::System { binding_ordinal } => resolution
            .system_module_bindings
            .get(*binding_ordinal)
            .map(|binding| binding.module_request_ordinal),
    }
}
