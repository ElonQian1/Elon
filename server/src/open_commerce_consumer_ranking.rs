use std::cmp::Ordering;

use anyhow::{bail, Result};

use crate::{
    open_commerce_consumer_model::{
        ConsumerDiscoveryMatch, ConsumerPreferences, ConsumerRankingPolicyDescriptor,
    },
    open_commerce_directory_model::OpenCommerceDirectoryCapability,
    open_commerce_model::{ACCESS_AUTHORIZED, ACCESS_PUBLIC},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ConsumerRankingPolicy {
    PreferenceMatch,
    LowestUnitPrice,
    PublicAccessFirst,
    RecentlyUpdated,
    MerchantName,
}

impl ConsumerRankingPolicy {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("transparent_preference_match.v1") => Ok(Self::PreferenceMatch),
            Some("lowest_unit_price.v1") => Ok(Self::LowestUnitPrice),
            Some("public_access_first.v1") => Ok(Self::PublicAccessFirst),
            Some("recently_updated.v1") => Ok(Self::RecentlyUpdated),
            Some("merchant_name.v1") => Ok(Self::MerchantName),
            Some(_) => bail!("消费者发现排序策略不受支持"),
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::PreferenceMatch => "transparent_preference_match.v1",
            Self::LowestUnitPrice => "lowest_unit_price.v1",
            Self::PublicAccessFirst => "public_access_first.v1",
            Self::RecentlyUpdated => "recently_updated.v1",
            Self::MerchantName => "merchant_name.v1",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PreferenceMatch => "偏好匹配",
            Self::LowestUnitPrice => "最低调用价",
            Self::PublicAccessFirst => "公开能力优先",
            Self::RecentlyUpdated => "最近更新",
            Self::MerchantName => "商户名称",
        }
    }

    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::PreferenceMatch => "按公开类别、城市、标签、访问方式和调用价格计算匹配分",
            Self::LowestUnitPrice => "先按能力调用单价从低到高，再按偏好匹配分排序",
            Self::PublicAccessFirst => "先展示无需额外授权的公开能力，再按偏好匹配分排序",
            Self::RecentlyUpdated => "先展示公开目录中最近更新的能力，再按偏好匹配分排序",
            Self::MerchantName => "按公开商户名称稳定排序，不使用付费位置",
        }
    }

    pub(crate) fn ranking_reason(self) -> String {
        format!("当前使用{}排序器", self.label())
    }

    pub(crate) fn select_capability<'a>(
        self,
        capabilities: Vec<&'a OpenCommerceDirectoryCapability>,
        preferences: &ConsumerPreferences,
        preference_score: impl Fn(&OpenCommerceDirectoryCapability, &ConsumerPreferences) -> i64,
    ) -> Option<&'a OpenCommerceDirectoryCapability> {
        capabilities.into_iter().min_by(|left, right| match self {
            Self::PreferenceMatch => preference_score(right, preferences)
                .cmp(&preference_score(left, preferences))
                .then_with(|| capability_tie_break(left, right)),
            Self::LowestUnitPrice => left
                .unit_price_micros
                .cmp(&right.unit_price_micros)
                .then_with(|| {
                    preference_score(right, preferences).cmp(&preference_score(left, preferences))
                })
                .then_with(|| capability_tie_break(left, right)),
            Self::PublicAccessFirst => access_rank(&left.access_level)
                .cmp(&access_rank(&right.access_level))
                .then_with(|| left.unit_price_micros.cmp(&right.unit_price_micros))
                .then_with(|| capability_tie_break(left, right)),
            Self::RecentlyUpdated => right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| capability_tie_break(left, right)),
            Self::MerchantName => capability_tie_break(left, right),
        })
    }

    pub(crate) fn sort_matches(self, matches: &mut [ConsumerDiscoveryMatch]) {
        matches.sort_by(|left, right| {
            let primary = match self {
                Self::PreferenceMatch => right.score.cmp(&left.score),
                Self::LowestUnitPrice => left
                    .capability
                    .unit_price_micros
                    .cmp(&right.capability.unit_price_micros),
                Self::PublicAccessFirst => access_rank(&left.capability.access_level)
                    .cmp(&access_rank(&right.capability.access_level)),
                Self::RecentlyUpdated => {
                    right.capability.updated_at.cmp(&left.capability.updated_at)
                }
                Self::MerchantName => left.merchant.display_name.cmp(&right.merchant.display_name),
            };
            primary
                .then_with(|| right.score.cmp(&left.score))
                .then_with(|| {
                    left.capability
                        .unit_price_micros
                        .cmp(&right.capability.unit_price_micros)
                })
                .then_with(|| left.merchant.display_name.cmp(&right.merchant.display_name))
                .then_with(|| left.merchant.id.cmp(&right.merchant.id))
                .then_with(|| {
                    left.capability
                        .capability_key
                        .cmp(&right.capability.capability_key)
                })
        });
    }
}

pub(crate) fn available_ranking_policies() -> Vec<ConsumerRankingPolicyDescriptor> {
    [
        ConsumerRankingPolicy::PreferenceMatch,
        ConsumerRankingPolicy::LowestUnitPrice,
        ConsumerRankingPolicy::PublicAccessFirst,
        ConsumerRankingPolicy::RecentlyUpdated,
        ConsumerRankingPolicy::MerchantName,
    ]
    .into_iter()
    .map(|policy| ConsumerRankingPolicyDescriptor {
        key: policy.key().to_string(),
        label: policy.label().to_string(),
        explanation: policy.explanation().to_string(),
        paid_placement: false,
    })
    .collect()
}

fn access_rank(value: &str) -> i32 {
    match value {
        ACCESS_PUBLIC => 0,
        ACCESS_AUTHORIZED => 1,
        _ => 2,
    }
}

fn capability_tie_break(
    left: &OpenCommerceDirectoryCapability,
    right: &OpenCommerceDirectoryCapability,
) -> Ordering {
    left.capability_key
        .cmp(&right.capability_key)
        .then_with(|| left.version.cmp(&right.version))
}
