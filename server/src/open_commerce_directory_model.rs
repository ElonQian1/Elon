//! Public, merchant-approved directory contracts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::open_commerce_model::{OpenCommerceCapability, OpenCommerceMerchant};

pub(crate) const DIRECTORY_STATUS_PUBLISHED: &str = "published";
pub(crate) const DIRECTORY_STATUS_UNPUBLISHED: &str = "unpublished";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryPublication {
    pub merchant_id: String,
    pub project_id: String,
    pub status: String,
    pub revision: i64,
    pub published_by_user_id: Option<String>,
    pub published_at: Option<String>,
    pub unpublished_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SetDirectoryPublicationRequest {
    pub published: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryMerchant {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub public_profile: Value,
    pub directory_revision: i64,
    pub published_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryCapability {
    pub capability_key: String,
    pub display_name: String,
    pub description: String,
    pub kind: String,
    pub access_level: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub unit_price_micros: i64,
    pub currency: String,
    pub freshness_seconds: i64,
    pub version: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryMerchantDetail {
    pub schema: &'static str,
    pub merchant: OpenCommerceDirectoryMerchant,
    pub capabilities: Vec<OpenCommerceDirectoryCapability>,
}

impl OpenCommerceDirectoryMerchantDetail {
    pub(crate) fn from_domain(
        merchant: OpenCommerceMerchant,
        capabilities: Vec<OpenCommerceCapability>,
        publication: OpenCommerceDirectoryPublication,
    ) -> Self {
        Self {
            schema: "open_commerce.directory_merchant.v1",
            merchant: OpenCommerceDirectoryMerchant {
                id: merchant.id,
                slug: merchant.slug,
                display_name: merchant.display_name,
                description: merchant.description,
                public_profile: merchant.public_profile,
                directory_revision: publication.revision,
                published_at: publication.published_at.unwrap_or(publication.updated_at),
                updated_at: merchant.updated_at,
            },
            capabilities: capabilities
                .into_iter()
                .map(|capability| OpenCommerceDirectoryCapability {
                    capability_key: capability.capability_key,
                    display_name: capability.display_name,
                    description: capability.description,
                    kind: capability.kind,
                    access_level: capability.access_level,
                    input_schema: capability.input_schema,
                    output_schema: capability.output_schema,
                    unit_price_micros: capability.unit_price_micros,
                    currency: capability.currency,
                    freshness_seconds: capability.freshness_seconds,
                    version: capability.version,
                    updated_at: capability.updated_at,
                })
                .collect(),
        }
    }
}
