//! Public, merchant-approved directory contracts.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    open_commerce_capability_source_model::OpenCommerceCapabilitySourceLink,
    open_commerce_merchant_identity_model::OpenCommercePublicMerchantIdentityKey,
    open_commerce_model::{OpenCommerceCapability, OpenCommerceMerchant},
};

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
    pub source: OpenCommerceDirectorySourceDeclaration,
    pub freshness: OpenCommerceDirectoryFreshness,
    pub version: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectorySourceDeclaration {
    pub schema: &'static str,
    pub kind: String,
    pub assertion_authority: &'static str,
    pub externally_verified: bool,
    pub integration_receipt_id: Option<String>,
    pub provider_key: Option<String>,
    pub connection_mode: Option<String>,
    pub data_domain: Option<String>,
    pub receipt_status: Option<String>,
    pub receipt_completed_at: Option<String>,
    pub receipt_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryFreshness {
    pub schema: &'static str,
    pub status: &'static str,
    pub declared_seconds: i64,
    pub declaration_updated_at: String,
    pub valid_until: Option<String>,
    pub basis: &'static str,
    pub externally_verified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDirectoryMerchantDetail {
    pub schema: &'static str,
    pub merchant: OpenCommerceDirectoryMerchant,
    pub capabilities: Vec<OpenCommerceDirectoryCapability>,
    pub portable_identity_keys: Vec<OpenCommercePublicMerchantIdentityKey>,
}

impl OpenCommerceDirectoryMerchantDetail {
    pub(crate) fn from_domain(
        merchant: OpenCommerceMerchant,
        capabilities: Vec<OpenCommerceCapability>,
        publication: OpenCommerceDirectoryPublication,
        portable_identity_keys: Vec<OpenCommercePublicMerchantIdentityKey>,
        capability_source_links: Vec<OpenCommerceCapabilitySourceLink>,
    ) -> Self {
        let source_links = capability_source_links
            .into_iter()
            .map(|link| (link.capability_id.clone(), link))
            .collect::<HashMap<_, _>>();
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
                .map(|capability| {
                    let linked_source = source_links.get(&capability.id);
                    let source = source_declaration(&capability.handler_type, linked_source);
                    let (freshness_updated_at, freshness_basis) = linked_source
                        .map(|link| {
                            (
                                link.receipt_completed_at.as_str(),
                                "sync_receipt_completed_at",
                            )
                        })
                        .unwrap_or((
                            capability.updated_at.as_str(),
                            "capability_declaration_updated_at",
                        ));
                    let freshness = directory_freshness(
                        freshness_updated_at,
                        capability.freshness_seconds,
                        freshness_basis,
                    );
                    OpenCommerceDirectoryCapability {
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
                        source,
                        freshness,
                        version: capability.version,
                        updated_at: capability.updated_at,
                    }
                })
                .collect(),
            portable_identity_keys,
        }
    }
}

fn source_declaration(
    handler_type: &str,
    linked_source: Option<&OpenCommerceCapabilitySourceLink>,
) -> OpenCommerceDirectorySourceDeclaration {
    if let Some(link) = linked_source {
        return OpenCommerceDirectorySourceDeclaration {
            schema: "open_commerce.directory_source_declaration.v1",
            kind: "integration_sync_receipt".to_string(),
            assertion_authority: "merchant_project",
            externally_verified: false,
            integration_receipt_id: Some(link.sync_receipt_id.clone()),
            provider_key: Some(link.provider_key.clone()),
            connection_mode: Some(link.connection_mode.clone()),
            data_domain: Some(link.data_domain.clone()),
            receipt_status: Some(link.receipt_status.clone()),
            receipt_completed_at: Some(link.receipt_completed_at.clone()),
            receipt_sha256: Some(link.receipt_sha256.clone()),
        };
    }
    let kind = match handler_type {
        "merchant_profile" => "merchant_profile",
        "static_json" => "merchant_static_data",
        "merchant_runtime" => "merchant_runtime",
        _ => "merchant_declared",
    };
    OpenCommerceDirectorySourceDeclaration {
        schema: "open_commerce.directory_source_declaration.v1",
        kind: kind.to_string(),
        assertion_authority: "merchant_project",
        externally_verified: false,
        integration_receipt_id: None,
        provider_key: None,
        connection_mode: None,
        data_domain: None,
        receipt_status: None,
        receipt_completed_at: None,
        receipt_sha256: None,
    }
}

fn directory_freshness(
    updated_at: &str,
    declared_seconds: i64,
    basis: &'static str,
) -> OpenCommerceDirectoryFreshness {
    let valid_until = (declared_seconds > 0)
        .then(|| {
            DateTime::parse_from_rfc3339(updated_at)
                .ok()?
                .checked_add_signed(Duration::try_seconds(declared_seconds)?)
                .map(|value| value.to_rfc3339())
        })
        .flatten();
    let status = match valid_until
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    {
        Some(value) if value.with_timezone(&Utc) > Utc::now() => "current",
        Some(_) => "stale",
        None => "unknown",
    };
    OpenCommerceDirectoryFreshness {
        schema: "open_commerce.directory_freshness.v1",
        status,
        declared_seconds,
        declaration_updated_at: updated_at.to_string(),
        valid_until,
        basis,
        externally_verified: false,
    }
}
