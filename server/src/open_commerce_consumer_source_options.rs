use std::collections::BTreeMap;

use crate::{
    open_commerce_consumer_model::{ConsumerSourceFilterOption, ConsumerSourceFilterOptions},
    open_commerce_directory_model::OpenCommerceDirectoryMerchantDetail,
};

pub(crate) fn collect_source_filter_options(
    candidates: &[OpenCommerceDirectoryMerchantDetail],
    capability_key: Option<&str>,
) -> ConsumerSourceFilterOptions {
    let mut providers = BTreeMap::<String, usize>::new();
    let mut data_domains = BTreeMap::<String, usize>::new();
    for capability in candidates
        .iter()
        .flat_map(|candidate| candidate.capabilities.iter())
        .filter(|capability| {
            capability_key
                .map(|key| capability.capability_key == key)
                .unwrap_or(true)
        })
        .filter(|capability| capability.source.kind == "integration_sync_receipt")
    {
        if let Some(provider) = capability.source.provider_key.as_ref() {
            *providers.entry(provider.clone()).or_default() += 1;
        }
        if let Some(data_domain) = capability.source.data_domain.as_ref() {
            *data_domains.entry(data_domain.clone()).or_default() += 1;
        }
    }
    ConsumerSourceFilterOptions {
        schema: "open_commerce.consumer_source_filter_options.v1",
        scope: "current_operator_candidate_window.v1",
        operator_exhaustive: false,
        providers: into_options(providers),
        data_domains: into_options(data_domains),
    }
}

fn into_options(values: BTreeMap<String, usize>) -> Vec<ConsumerSourceFilterOption> {
    values
        .into_iter()
        .map(|(value, capability_count)| ConsumerSourceFilterOption {
            value,
            capability_count,
        })
        .collect()
}
