//! Machine-readable contract shared by connector authors and the control plane.

use serde::Serialize;

pub(crate) const CONNECTOR_SCHEMA: &str = "open_commerce.connector.v1";
pub(crate) const CONNECTOR_CONTRACT_VERSION: &str = "1.0";
pub(crate) const MAX_SYNC_PAGE_RECORDS: usize = 500;
pub(crate) const MAX_RECEIPT_KEY_LENGTH: usize = 128;

#[derive(Debug, Serialize)]
pub(crate) struct OpenCommerceConnectorContract {
    pub schema: &'static str,
    pub contract_version: &'static str,
    pub sdk_package: &'static str,
    pub required_methods: [&'static str; 3],
    pub connection_modes: [&'static str; 4],
    pub sync_kinds: [&'static str; 3],
    pub sync_statuses: [&'static str; 3],
    pub health_statuses: [&'static str; 3],
    pub limits: ConnectorLimits,
    pub security: ConnectorSecurityContract,
    pub endpoints: ConnectorEndpoints,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConnectorLimits {
    pub max_sync_page_records: usize,
    pub max_receipt_key_length: usize,
    pub max_scopes: usize,
    pub max_data_domains: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConnectorSecurityContract {
    pub manifest_must_exclude_credentials: bool,
    pub receipt_must_exclude_business_values: bool,
    pub raw_data_owner: &'static str,
    pub secret_transport: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConnectorEndpoints {
    pub create_integration: &'static str,
    pub record_sync_receipt: &'static str,
    pub development_context: &'static str,
}

pub(crate) fn contract() -> OpenCommerceConnectorContract {
    OpenCommerceConnectorContract {
        schema: CONNECTOR_SCHEMA,
        contract_version: CONNECTOR_CONTRACT_VERSION,
        sdk_package: "@elon/open-commerce-connector",
        required_methods: ["describe", "health", "sync"],
        connection_modes: [
            "official_api",
            "merchant_export",
            "local_adapter",
            "manual_import",
        ],
        sync_kinds: ["full", "incremental", "health_check"],
        sync_statuses: ["succeeded", "partial", "failed"],
        health_statuses: ["ready", "degraded", "unavailable"],
        limits: ConnectorLimits {
            max_sync_page_records: MAX_SYNC_PAGE_RECORDS,
            max_receipt_key_length: MAX_RECEIPT_KEY_LENGTH,
            max_scopes: 32,
            max_data_domains: 32,
        },
        security: ConnectorSecurityContract {
            manifest_must_exclude_credentials: true,
            receipt_must_exclude_business_values: true,
            raw_data_owner: "merchant",
            secret_transport: "node_or_server_secret_store_only",
        },
        endpoints: ConnectorEndpoints {
            create_integration: "/api/projects/{project_id}/open-commerce/integrations",
            record_sync_receipt: "/api/projects/{project_id}/open-commerce/sync-receipts",
            development_context: "/api/projects/{project_id}/open-commerce/development-context",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_contract_keeps_raw_values_and_credentials_out_of_receipts() {
        let value = serde_json::to_value(contract()).unwrap();
        assert_eq!(value["schema"], CONNECTOR_SCHEMA);
        assert_eq!(value["contract_version"], CONNECTOR_CONTRACT_VERSION);
        assert_eq!(value["limits"]["max_sync_page_records"], 500);
        assert_eq!(
            value["security"]["receipt_must_exclude_business_values"],
            true
        );
        assert_eq!(value["security"]["manifest_must_exclude_credentials"], true);
    }
}
