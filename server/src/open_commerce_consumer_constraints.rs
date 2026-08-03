use anyhow::{bail, Result};

use crate::{
    open_commerce_consumer_model::{ConsumerDiscoveryRequest, ConsumerPreferenceConstraints},
    open_commerce_directory_model::OpenCommerceDirectoryMerchantDetail,
};

pub(crate) fn validate(request: &ConsumerDiscoveryRequest) -> Result<()> {
    if request.require_city_match && request.preferences.city.is_none() {
        bail!("启用城市硬约束前必须填写城市");
    }
    if request.require_category_match && request.preferences.categories.is_empty() {
        bail!("启用经营类别硬约束前必须填写至少一个类别");
    }
    if request.require_all_tags_match && request.preferences.tags.is_empty() {
        bail!("启用全部标签硬约束前必须填写至少一个标签");
    }
    Ok(())
}

pub(crate) fn response(request: &ConsumerDiscoveryRequest) -> ConsumerPreferenceConstraints {
    ConsumerPreferenceConstraints {
        require_city_match: request.require_city_match,
        require_category_match: request.require_category_match,
        require_all_tags_match: request.require_all_tags_match,
    }
}

pub(crate) fn evaluate(
    detail: &OpenCommerceDirectoryMerchantDetail,
    request: &ConsumerDiscoveryRequest,
) -> Option<Vec<String>> {
    let profile = &detail.merchant.public_profile;
    let mut reasons = Vec::new();
    if request.require_city_match {
        let expected = request.preferences.city.as_deref()?;
        let actual = profile.get("city").and_then(|value| value.as_str())?;
        if !actual.eq_ignore_ascii_case(expected) {
            return None;
        }
        reasons.push(format!("硬性城市条件匹配 {actual}"));
    }
    if request.require_category_match {
        let actual = profile.get("category").and_then(|value| value.as_str())?;
        if !contains_ignore_case(&request.preferences.categories, actual) {
            return None;
        }
        reasons.push(format!("硬性经营类别条件匹配 {actual}"));
    }
    if request.require_all_tags_match {
        let merchant_tags = profile
            .get("tags")
            .and_then(|value| value.as_array())?
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        if !request.preferences.tags.iter().all(|required| {
            merchant_tags
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(required))
        }) {
            return None;
        }
        reasons.push(format!(
            "硬性标签条件全部匹配 {} 项",
            request.preferences.tags.len()
        ));
    }
    Some(reasons)
}

fn contains_ignore_case(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(needle))
}
