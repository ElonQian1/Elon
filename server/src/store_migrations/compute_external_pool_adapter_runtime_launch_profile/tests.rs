use sha2::{Digest, Sha256};

use super::*;

const TABLES: &str = include_str!("tables.sql");
const VIEW: &str = include_str!("view.sql");
const IMMUTABILITY: &str = include_str!("guards/immutability.rs");
const LINEAGE: &str = include_str!("guards/lineage.rs");
const PROFILE_PROJECTION: &str = include_str!("guards/profile_projection.rs");
const POLICY_PROJECTION: &str = include_str!("guards/policy_projection.rs");
const ROOTS: &str = include_str!("guards/roots.rs");
const POLICY_DOMAIN: &str =
    include_str!("../../compute_federation/external_pool_adapter_runtime_launch_profile/policy.rs");
const PERSISTENCE: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_launch_profile/persistence.rs");
const V254_FENCES: &str =
    include_str!("../compute_external_pool_provider_activation_candidate/guards/fences.rs");

#[test]
fn v255_store_insert_columns_match_frozen_tables_exactly() {
    for (table, expected) in [
        ("compute_external_pool_adapter_runtime_launch_profiles", 63),
        (
            "compute_external_pool_adapter_runtime_launch_profile_revocations",
            31,
        ),
    ] {
        let columns = insert_columns(PERSISTENCE, table);
        assert_eq!(columns.len(), expected, "{table} persistence ABI drift");
        assert_eq!(columns, ddl_columns(TABLES, table));
    }
}

#[test]
fn v255_full_receipt_and_policy_projection_counts_are_frozen() {
    assert_eq!(guards::profile_projection_counts(), (62, 30));
    assert_eq!(guards::policy_projection_count(), 45);
    assert!(POLICY_PROJECTION.contains("server_runtime_launch_policy_catalog"));
    assert!(POLICY_DOMAIN.contains("server_runtime_launch_policy_catalog"));
    for column in insert_columns(
        PERSISTENCE,
        "compute_external_pool_adapter_runtime_launch_profiles",
    )
    .into_iter()
    .filter(|column| column != "profile_json" && column != "launch_policy_json")
    {
        assert!(
            PROFILE_PROJECTION.contains(&format!("\"{column}\"")),
            "profile projection lacks {column}"
        );
    }
    for column in insert_columns(
        PERSISTENCE,
        "compute_external_pool_adapter_runtime_launch_profile_revocations",
    )
    .into_iter()
    .filter(|column| column != "revocation_json")
    {
        assert!(
            PROFILE_PROJECTION.contains(&format!("\"{column}\"")),
            "revocation projection lacks {column}"
        );
    }
    for required in [
        "COUNT(*) FROM json_each(NEW.{json_column}))!=7",
        "material_count",
        "json(json_extract",
        "credential_ref_scheme",
        "credential_locator_commitment",
        "entrypoint_relative_path",
        "adapter_effect",
        "usage_effect",
        "host_environment",
        "binary_format",
        "max_runtime_temp_bytes",
    ] {
        assert!(
            PROFILE_PROJECTION.contains(required) || POLICY_PROJECTION.contains(required),
            "missing full projection fence {required}"
        );
    }
}

#[test]
fn v255_exact_roots_keep_profile_inert_and_provider_registering() {
    for required in [
        "compute_external_pool_adapter_registry_provider_binding_current",
        "current_binding.current_status='binding_current'",
        "current_binding.projection_status='reserved'",
        "compute_external_pool_adapter_registry_release_current",
        "current_release.current_status='release_current'",
        "compute_external_pool_adapter_installation_current",
        "current_installation.current_status='installed_upstreams_current'",
        "compute_external_pool_onboarding_applications onboarding",
        "onboarding.application_id=binding.application_id",
        "onboarding.application_digest=binding.application_digest",
        "compute_external_pool_provider_activation_candidates later",
        "compute_external_pool_provider_activation_delegations later_delegation",
        "later_delegation.sequence>delegation.sequence",
        "compute_external_pool_provider_activation_delegation_revocations revoked",
        "candidate.candidate_status='candidate_current_not_activation_ready'",
        "candidate.activation_closure_status='activation_closure_not_implemented'",
        "delegation.issued_at<=NEW.recorded_at",
        "candidate.checked_at<=NEW.recorded_at",
        "provider.status='registering'",
        "json_extract(provider_version.provider_json,'$.status')='registering'",
        "binding.credential_locator_commitment=NEW.credential_locator_commitment",
        "NEW.credential_ref_scheme='vault_ref'",
        "substr(onboarding.non_bearer_credential_ref,1,10)='vault-ref:'",
        "installation.entry_inventory_digest=NEW.entry_inventory_digest",
        "installation.entrypoint_path=NEW.entrypoint_relative_path",
        "release.credential_verifier_digest=NEW.credential_verifier_digest",
    ] {
        assert!(ROOTS.contains(required), "missing exact root {required}");
    }
    for forbidden in [
        "vulnerability_reattestation_current",
        "sandbox_reattestation_current",
        "credential_reattestation_current",
        "provider.status='active'",
        "julianday('now')",
    ] {
        assert!(
            !ROOTS.contains(forbidden),
            "V255 claims forbidden root {forbidden}"
        );
    }
}

#[test]
fn v255_lineage_is_latest_append_only_and_revocation_is_recoverable() {
    for required in [
        "NEW.sequence=1",
        "predecessor.sequence=NEW.sequence-1",
        "successor.predecessor_profile_id=predecessor.profile_id",
        "target.recorded_at<=NEW.revoked_at",
        "successor.predecessor_profile_id=target.profile_id",
        "prior.profile_id=target.profile_id",
        "UNIQUE(provider_binding_id,sequence)",
        "UNIQUE(predecessor_profile_id)",
        "uq_external_pool_adapter_runtime_launch_profile_genesis",
    ] {
        assert!(
            LINEAGE.contains(required) || TABLES.contains(required),
            "missing lineage fence {required}"
        );
    }
    assert!(LINEAGE.contains("exact structural predecessor head"));
    assert!(!LINEAGE.contains("revoked.profile_id=predecessor.profile_id"));
    for required in ["_no_update", "_no_delete", "_no_replace"] {
        assert!(IMMUTABILITY.contains(required));
    }
}

#[test]
fn v255_current_view_is_display_only_and_never_runtime_ready() {
    for required in [
        "compute_federation.external_pool_adapter_runtime_launch_profile_currentness.v1",
        "launch_profile_current_inert",
        "historical_only",
        "exact_registering",
        "revocation_status",
        "0 AS runtime_launch_ready",
        "json_extract(display.launch_policy_json,'$.policy_id')",
        "json_extract(display.launch_policy_json,'$.policy_revision')",
        "compute_external_pool_provider_activation_delegations later_delegation",
        "later_delegation.sequence>delegation.sequence",
    ] {
        assert!(
            VIEW.contains(required),
            "missing current display {required}"
        );
    }
    for forbidden in [
        "UPDATE ",
        "INSERT INTO ",
        "DELETE FROM ",
        "runtime_launch_ready=1",
    ] {
        assert!(
            !VIEW.contains(forbidden),
            "view contains side effect {forbidden}"
        );
    }
}

#[test]
fn v255_source_preserves_all_v254_absolute_denies_exactly() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6",
        "V254 deny trigger source changed; V255 requires exact byte parity"
    );
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 deny {name}");
    }
    let v255 = concat!(
        include_str!("../compute_external_pool_adapter_runtime_launch_profile.rs"),
        include_str!("tables.sql"),
        include_str!("view.sql"),
        include_str!("guards/immutability.rs"),
        include_str!("guards/lineage.rs"),
        include_str!("guards/policy_projection.rs"),
        include_str!("guards/profile_projection.rs"),
        include_str!("guards/roots.rs"),
    );
    for forbidden in [
        "DROP TRIGGER",
        "UPDATE compute_providers",
        "INSERT INTO compute_route_",
        "INSERT INTO compute_service_actor_authorizations",
        "INSERT INTO compute_capacity_pools",
        "INSERT INTO compute_offers",
        "status='active'",
        "status=\"active\"",
    ] {
        assert!(!v255.contains(forbidden), "V255 crosses no-go {forbidden}");
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
