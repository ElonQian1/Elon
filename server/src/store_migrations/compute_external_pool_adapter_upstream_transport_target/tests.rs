use sha2::{Digest, Sha256};

use super::*;

const TABLES: &str = include_str!("tables.sql");
const VIEW: &str = include_str!("view.sql");
const VIEW_INSTALLER: &str = include_str!("view.rs");
const HOSTNAME: &str = include_str!("guards/hostname.rs");
const IMMUTABILITY: &str = include_str!("guards/immutability.rs");
const LINEAGE: &str = include_str!("guards/lineage.rs");
const POLICY_PROJECTION: &str = include_str!("guards/policy_projection.rs");
const RECEIPT_PROJECTION: &str = include_str!("guards/receipt_projection.rs");
const ROOTS: &str = include_str!("guards/roots.rs");
const TIMESTAMP: &str = include_str!("guards/timestamp.rs");
const DOMAIN_POLICY: &str = include_str!(
    "../../compute_federation/external_pool_adapter_upstream_transport_target/policy.rs"
);
const PERSISTENCE: &str = include_str!(
    "../../store/compute_external_pool_adapter_upstream_transport_target/persistence.rs"
);
const REGISTRY: &str = include_str!("../../store_migrations.rs");
const V254_FENCES: &str =
    include_str!("../compute_external_pool_provider_activation_candidate/guards/fences.rs");

#[test]
fn v258_store_insert_columns_match_frozen_tables_exactly() {
    for (table, expected) in [
        (
            "compute_external_pool_adapter_upstream_transport_targets",
            68,
        ),
        (
            "compute_external_pool_adapter_upstream_transport_target_revocations",
            36,
        ),
    ] {
        let columns = insert_columns(PERSISTENCE, table);
        assert_eq!(columns.len(), expected, "{table} persistence ABI drift");
        assert_eq!(columns, ddl_columns(TABLES, table));
    }
}

#[test]
fn v258_registration_and_full_receipt_projection_are_frozen() {
    assert!(REGISTRY.contains("mod compute_external_pool_adapter_upstream_transport_target;"));
    assert!(REGISTRY
        .contains("compute_external_pool_adapter_upstream_transport_target::migration_v258"));
    assert_eq!(guards::receipt_projection_counts(), (67, 35));
    for (table, json_column) in [
        (
            "compute_external_pool_adapter_upstream_transport_targets",
            "target_json",
        ),
        (
            "compute_external_pool_adapter_upstream_transport_target_revocations",
            "revocation_json",
        ),
    ] {
        for column in insert_columns(PERSISTENCE, table)
            .into_iter()
            .filter(|column| column != json_column && column != "target_policy_json")
        {
            assert!(
                RECEIPT_PROJECTION.contains(&format!("\"{column}\"")),
                "receipt projection lacks {table}.{column}"
            );
        }
    }
    for required in [
        "COUNT(*) FROM json_each(NEW.{json_column}))!=7",
        "material_count",
        "json(json_extract",
        "target_policy_json",
        "dns_hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "broker_connect_ready",
        "upstream_probe_observed",
        "runtime_launch_ready",
        "activation_ready",
    ] {
        assert!(
            RECEIPT_PROJECTION.contains(required),
            "missing full receipt projection fence {required}"
        );
    }
}

#[test]
fn v258_policy_projection_uses_the_only_server_catalog() {
    assert_eq!(guards::policy_projection_count(), 24);
    for required in [
        "server_upstream_transport_target_policy_catalog",
        "POLICY_FIELD_COUNT: usize = 24",
        "COUNT(*) FROM json_each(NEW.target_policy_json)",
        "NEW.target_policy_digest",
        "NEW.target_policy_json IS NOT",
        "canonical_compute_plugin_ijson_and_sha256",
    ] {
        assert!(POLICY_PROJECTION.contains(required));
    }
    assert!(DOMAIN_POLICY.contains("server_upstream_transport_target_policy_catalog"));
    for field in [
        "port_policy",
        "tls_trust_anchor_policy",
        "future_broker_webpki_chain_hostname_and_time_at_connect_v1",
        "expected_leaf_spki_sha256_pin_and_future_webpki_observation_v1",
    ] {
        assert!(DOMAIN_POLICY.contains(field), "Domain policy lacks {field}");
    }
    for required in [
        "server_upstream_transport_target_policy_catalog",
        "canonical_compute_plugin_ijson_and_sha256",
        "policy.policy_id",
        "policy.policy_revision",
        "policy_digest",
        "policy_json",
    ] {
        assert!(
            VIEW_INSTALLER.contains(required),
            "view installer lacks {required}"
        );
    }
}

#[test]
fn v258_direct_sql_hostname_and_scalar_writes_fail_closed() {
    for required in [
        "BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_targets",
        "WITH RECURSIVE labels",
        "NEW.dns_hostname GLOB '*[^a-z0-9.-]*'",
        "length(NEW.dns_hostname)<>length(CAST(NEW.dns_hostname AS BLOB))",
        "NEW.dns_hostname NOT GLOB '*[a-z]*'",
        "length(CAST(label AS BLOB)) NOT BETWEEN 1 AND 63",
        "substr(label,1,1) NOT GLOB '[a-z0-9]'",
        "substr(label,-1,1) NOT GLOB '[a-z0-9]'",
    ] {
        assert!(
            HOSTNAME.contains(required),
            "missing hostname fence {required}"
        );
    }
    for required in [
        "port BETWEEN 1 AND 65535",
        "tls_server_name=dns_hostname",
        "expected_tls_leaf_spki_sha256",
        "broker_connect_ready=0",
        "upstream_probe_observed=0",
        "runtime_launch_ready=0",
        "activation_ready=0",
    ] {
        assert!(TABLES.contains(required), "missing scalar fence {required}");
    }
}

#[test]
fn v258_roots_lineage_and_causal_time_are_exact() {
    for required in [
        "compute_external_pool_adapter_runtime_launch_profile_current",
        "current_profile.current_status='launch_profile_current_inert'",
        "current_profile.head_status='head'",
        "current_profile.revocation_status='none'",
        "current_profile.runtime_launch_ready=0",
        "profile.recorded_at<=NEW.recorded_at",
        "profile.launch_policy_digest=NEW.launch_policy_digest",
        "$.network_egress_policy_id",
        "$.network_egress_policy_revision",
        "$.network_egress_policy_digest",
        "profile.service_actor_id=NEW.service_actor_id",
    ] {
        assert!(
            ROOTS.contains(required),
            "missing exact V255 root {required}"
        );
    }
    for required in [
        "NEW.sequence=1",
        "predecessor.sequence=NEW.sequence-1",
        "predecessor.recorded_at<=NEW.recorded_at",
        "successor.predecessor_target_id=predecessor.target_id",
        "target.recorded_at<=NEW.revoked_at",
        "successor.predecessor_target_id=target.target_id",
        "prior.target_id=target.target_id",
        "UNIQUE (provider_binding_id,sequence)",
        "UNIQUE (predecessor_target_id)",
        "uq_external_pool_adapter_upstream_transport_target_genesis",
        "recorded_at=revoked_at",
    ] {
        assert!(
            LINEAGE.contains(required) || TABLES.contains(required),
            "missing lineage or causal-time fence {required}"
        );
    }
    assert!(!LINEAGE.contains("revoked.target_id=predecessor.target_id"));
    for required in [
        "CAST(substr(NEW.{column},6,2) AS INTEGER) NOT BETWEEN 1 AND 12",
        "substr(NEW.{column},21,9) GLOB '*[^0-9]*'",
        "CAST(substr(NEW.{column},9,2) AS INTEGER) NOT BETWEEN 1 AND",
        "%400=0",
        "%100<>0",
        "NOT BETWEEN 0 AND 23",
        "NOT BETWEEN 0 AND 59",
        "julianday(NEW.{column})>julianday('now','+5 minutes')",
    ] {
        assert!(
            TIMESTAMP.contains(required),
            "missing civil-time fence {required}"
        );
    }
}

#[test]
fn v258_is_append_only_and_current_view_is_structural_display_only() {
    for required in ["_no_update", "_no_delete", "_no_replace"] {
        assert!(IMMUTABILITY.contains(required));
    }
    for required in [
        "compute_external_pool_adapter_upstream_transport_target_current",
        "SELECT head.*",
        "revocation.revocation_id",
        "revocation.revocation_digest",
        "revocation.revoked_at",
        "successor.predecessor_target_id=target.target_id",
        "'head' AS head_status",
        "upstream_transport_target_current_inert",
        "historical_only",
        "launch_profile_current_inert",
        "server_policy_current",
        "AS revocation_status",
        "head.target_policy_digest=__POLICY_DIGEST_SQL__",
        "json(head.target_policy_json)=json(__POLICY_JSON_SQL__)",
        "__POLICY_ID_SQL__",
        "__POLICY_REVISION__",
    ] {
        assert!(
            VIEW.contains(required),
            "missing current view projection {required}"
        );
    }
    for forbidden in ["UPDATE ", "INSERT INTO ", "DELETE FROM "] {
        assert!(
            !VIEW.contains(forbidden),
            "view contains side effect {forbidden}"
        );
    }
}

#[test]
fn v258_preserves_v254_absolute_denies_and_claims_no_runtime_or_tls_evidence() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6",
        "V254 deny trigger source changed; V258 requires exact byte parity"
    );
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 deny {name}");
    }
    let v258 = concat!(
        include_str!("../compute_external_pool_adapter_upstream_transport_target.rs"),
        include_str!("tables.sql"),
        include_str!("view.sql"),
        include_str!("guards/hostname.rs"),
        include_str!("guards/immutability.rs"),
        include_str!("guards/lineage.rs"),
        include_str!("guards/policy_projection.rs"),
        include_str!("guards/receipt_projection.rs"),
        include_str!("guards/roots.rs"),
        include_str!("guards/timestamp.rs"),
    );
    for forbidden in [
        "DROP TRIGGER",
        "UPDATE compute_providers",
        "INSERT INTO compute_route_",
        "provider.status='active'",
        "target_status='active'",
        "broker_connect_ready=1",
        "upstream_probe_observed=1",
        "runtime_launch_ready=1",
        "activation_ready=1",
        "tls_leaf_not_before",
        "tls_leaf_not_after",
    ] {
        assert!(!v258.contains(forbidden), "V258 crosses no-go {forbidden}");
    }
}

fn insert_columns(source: &str, table: &str) -> Vec<String> {
    let marker = format!("INSERT INTO {table}(");
    source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing Store INSERT for {table}"))
        .1
        .split_once(") VALUES (")
        .unwrap_or_else(|| panic!("missing Store VALUES for {table}"))
        .0
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect()
}

fn ddl_columns(source: &str, table: &str) -> Vec<String> {
    let marker = format!("CREATE TABLE IF NOT EXISTS {table} (");
    let body = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing DDL for {table}"))
        .1
        .split_once("\n);")
        .unwrap_or_else(|| panic!("unterminated DDL for {table}"))
        .0;
    body.lines()
        .filter_map(|line| {
            let column = line.trim().split_whitespace().next()?;
            (!column.starts_with("CHECK")
                && !column.starts_with("UNIQUE")
                && !column.starts_with("FOREIGN"))
            .then(|| column.to_owned())
        })
        .collect()
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];
