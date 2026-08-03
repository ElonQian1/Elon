//! Machine-readable contract shared by connector authors and the control plane.

use serde::Serialize;

pub(crate) const CONNECTOR_SCHEMA: &str = "open_commerce.connector.v1";
pub(crate) const CONNECTOR_CONTRACT_VERSION: &str = "1.0";
pub(crate) const MAX_SYNC_PAGE_RECORDS: usize = 500;
pub(crate) const MAX_RECEIPT_KEY_LENGTH: usize = 128;
pub(crate) const MIN_HANDOFF_LEASE_SECONDS: i64 = 60;
pub(crate) const MAX_HANDOFF_LEASE_SECONDS: i64 = 900;
pub(crate) const MAX_HANDOFF_RESPONSE_BYTES: usize = 256 * 1024;

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
    pub adapter_handoff: ConnectorAdapterHandoffContract,
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

#[derive(Debug, Serialize)]
pub(crate) struct ConnectorAdapterHandoffContract {
    pub enabled_scope: &'static str,
    pub default_enabled: bool,
    pub min_lease_seconds: i64,
    pub max_lease_seconds: i64,
    pub max_total_lease_seconds: i64,
    pub max_response_bytes: usize,
    pub release_reason_codes: [&'static str; 4],
    pub rejected_retry_schedule_seconds: [i64; 6],
    pub max_rejected_attempts: i64,
    pub candidate_order: &'static str,
    pub claim_endpoint: &'static str,
    pub complete_endpoint: &'static str,
    pub release_endpoint: &'static str,
    pub renew_endpoint: &'static str,
    pub resume_endpoint: &'static str,
    pub resume_requires_project_editor: bool,
    pub server_derives_identity: bool,
    pub funds_moved: bool,
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
        adapter_handoff: ConnectorAdapterHandoffContract {
            enabled_scope: "business_handoff.claim",
            default_enabled: false,
            min_lease_seconds: MIN_HANDOFF_LEASE_SECONDS,
            max_lease_seconds: MAX_HANDOFF_LEASE_SECONDS,
            max_total_lease_seconds: 3_600,
            max_response_bytes: MAX_HANDOFF_RESPONSE_BYTES,
            release_reason_codes: [
                "adapter_shutdown",
                "capacity_pressure",
                "transient_failure",
                "manual_release",
            ],
            rejected_retry_schedule_seconds: [30, 60, 120, 240, 480, 900],
            max_rejected_attempts: 6,
            candidate_order: "never_attempted_then_oldest_attempt",
            claim_endpoint: "/api/open-commerce/adapter/business-handoff-claims",
            complete_endpoint:
                "/api/open-commerce/adapter/business-handoff-claims/{claim_id}/complete",
            release_endpoint:
                "/api/open-commerce/adapter/business-handoff-claims/{claim_id}/release",
            renew_endpoint: "/api/open-commerce/adapter/business-handoff-claims/{claim_id}/renew",
            resume_endpoint:
                "/api/projects/{project_id}/open-commerce/adapter-handoff-claims/{claim_id}/resume",
            resume_requires_project_editor: true,
            server_derives_identity: true,
            funds_moved: false,
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
        assert_eq!(value["adapter_handoff"]["max_rejected_attempts"], 6);
        assert_eq!(
            value["adapter_handoff"]["resume_requires_project_editor"],
            true
        );
        assert_eq!(value["security"]["manifest_must_exclude_credentials"], true);
        assert_eq!(value["adapter_handoff"]["default_enabled"], false);
        assert_eq!(value["adapter_handoff"]["max_lease_seconds"], 900);
        assert_eq!(value["adapter_handoff"]["max_total_lease_seconds"], 3_600);
        assert_eq!(
            value["adapter_handoff"]["rejected_retry_schedule_seconds"][5],
            900
        );
        assert_eq!(value["adapter_handoff"]["server_derives_identity"], true);
        assert_eq!(value["adapter_handoff"]["funds_moved"], false);
    }
}
