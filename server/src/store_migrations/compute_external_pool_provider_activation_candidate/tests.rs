use super::*;

const TABLES: &str = include_str!("tables.sql");
const PROJECTION: &str = include_str!("guards/projection.rs");
const ROOTS: &str = include_str!("guards/roots.rs");
const LINEAGE: &str = include_str!("guards/lineage.rs");
const FENCES: &str = include_str!("guards/fences.rs");
const PRECHECK: &str = include_str!("precheck.rs");
const PERSISTENCE: &str =
    include_str!("../../store/compute_external_pool_provider_activation_candidate/persistence.rs");

#[test]
fn v254_migration_is_repeatable_on_fresh_current_schema() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    crate::store_schema::apply_migrations(&connection).unwrap();
    migration_v254(&connection).unwrap();
    migration_v254(&connection).unwrap();

    for object in [
        "compute_external_pool_provider_activation_delegations",
        "compute_external_pool_provider_activation_candidates",
        "compute_external_pool_provider_activation_delegation_revocations",
        "external_pool_provider_activation_delegation_exact_roots",
        "external_pool_provider_activation_candidate_lineage",
        "external_pool_provider_activation_revocation_json_projection",
        "v254_external_pool_provider_activation_fence",
        "v254_external_pool_capacity_pool_insert_active_fence",
        "v254_external_pool_offer_insert_market_fence",
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
                [object],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing or duplicated V254 object {object}");
    }
}

#[test]
fn v254_persistence_columns_match_frozen_tables() {
    for (table, expected) in [
        ("compute_external_pool_provider_activation_delegations", 41),
        ("compute_external_pool_provider_activation_candidates", 46),
        (
            "compute_external_pool_provider_activation_delegation_revocations",
            28,
        ),
    ] {
        let columns = insert_columns(PERSISTENCE, table);
        assert_eq!(columns.len(), expected, "{table} persistence ABI drift");
        for column in columns {
            assert!(
                TABLES.contains(&format!("{column} TEXT"))
                    || TABLES.contains(&format!("{column} INTEGER")),
                "DDL lacks persisted {table}.{column}"
            );
        }
    }
}

#[test]
fn v254_tables_keep_static_candidate_contract() {
    for required in [
        "compute_federation.external_pool_provider_activation_delegation.v1",
        "compute_federation.external_pool_provider_activation_candidate.v1",
        "compute_federation.external_pool_provider_activation_delegation_revocation.v1",
        "platform_dispatch_service",
        "candidate_current_not_activation_ready",
        "activation_closure_not_implemented",
        "owner_delegation_recorded",
        "activation_candidate_recorded",
        "owner_delegation_revoked",
        "UNIQUE(provider_binding_id,sequence)",
        "UNIQUE(predecessor_delegation_id)",
        "UNIQUE(predecessor_candidate_id)",
        "UNIQUE(delegation_id)",
        "UNIQUE(candidate_id)",
        "length(service_actor_id)=104",
        "external_pool_platform_dispatch_service_[0-9a-f]*",
    ] {
        assert!(
            TABLES.contains(required),
            "missing V254 table fence {required}"
        );
    }
    for forbidden in [
        "vulnerability_reattestation_receipt_id",
        "sandbox_reattestation_receipt_id",
        "credential_reattestation_receipt_id",
        "expires_at",
        "activation_ready INTEGER",
    ] {
        assert!(
            !TABLES.contains(forbidden),
            "static candidate stores {forbidden}"
        );
    }
}

#[test]
fn v254_full_json_projection_covers_every_persisted_scalar() {
    for (table, json, top, material) in [
        (
            "compute_external_pool_provider_activation_delegations",
            "delegation_json",
            7,
            34,
        ),
        (
            "compute_external_pool_provider_activation_candidates",
            "candidate_json",
            7,
            39,
        ),
        (
            "compute_external_pool_provider_activation_delegation_revocations",
            "revocation_json",
            7,
            21,
        ),
    ] {
        let columns = insert_columns(PERSISTENCE, table);
        assert_eq!(columns.len(), top + material);
        for column in columns.into_iter().filter(|column| *column != json) {
            assert!(
                PROJECTION.contains(&format!("\"{column}\"")),
                "projection lacks {table}.{column}"
            );
        }
    }
    assert!(PROJECTION.contains("json_type(NEW.{json_column},'{path}') IS NULL"));
    assert!(PROJECTION.contains("json(json_extract"));
    assert!(PROJECTION.contains("COUNT(*) FROM json_each(NEW.{json_column}))!=7"));
    assert!(PROJECTION.contains("material_field_count"));
}

#[test]
fn v254_exact_roots_do_not_claim_dynamic_preflight() {
    for required in [
        "compute_external_pool_adapter_registry_provider_bindings",
        "compute_external_pool_adapter_registry_provider_binding_current",
        "compute_external_pool_adapter_registry_releases",
        "compute_external_pool_adapter_installation_receipts",
        "compute_external_pool_adapter_installation_terminal_receipts",
        "compute_provider_versions",
        "provider.provider_kind='external_pool'",
        "provider.status='registering'",
        "json_extract(version.provider_json,'$.status')='registering'",
        "binding.route_adapter_projection_id=NEW.route_adapter_projection_id",
        "release.implementation_digest=NEW.implementation_digest",
        "release.capability_set_digest=NEW.capability_set_digest",
        "release.credential_verifier_digest=NEW.credential_verifier_digest",
        "NEW.issued_by_owner_user_id=provider.owner_account_id",
    ] {
        assert!(ROOTS.contains(required), "missing exact root {required}");
    }
    for forbidden in [
        "vulnerability_reattestation_current",
        "sandbox_reattestation_current",
        "credential_reattestation_current",
        "julianday('now')",
        "PreparedExternalPoolAdapterInstallation",
    ] {
        assert!(!ROOTS.contains(forbidden), "SQL roots claim {forbidden}");
    }
}

#[test]
fn v254_lineage_is_linear_and_revocation_permanently_closes_head() {
    for required in [
        "NEW.sequence=1",
        "predecessor.sequence=NEW.sequence-1",
        "later.sequence>predecessor.sequence",
        "compute_external_pool_provider_activation_delegation_revocations revoked",
        "revoked.delegation_id=predecessor.delegation_id",
        "companion.sequence=predecessor.sequence",
        "predecessor.sequence=NEW.sequence-1",
        "predecessor_delegation.delegation_id=delegation.predecessor_delegation_id",
        "NEW.checked_at>=delegation.issued_at",
        "NEW.revoked_at>=candidate.checked_at",
        "later.sequence>delegation.sequence",
    ] {
        assert!(
            LINEAGE.contains(required),
            "missing lineage fence {required}"
        );
    }
}

#[test]
fn v254_temporary_fences_cover_real_v213_insert_tables() {
    for required in [
        "BEFORE UPDATE ON compute_providers",
        "BEFORE INSERT ON compute_providers",
        "BEFORE INSERT ON compute_provider_versions",
        "BEFORE INSERT ON compute_route_adapters",
        "BEFORE INSERT ON compute_route_adapter_versions",
        "BEFORE INSERT ON compute_service_actor_authorizations",
        "BEFORE INSERT ON compute_route_credential_versions",
        "BEFORE INSERT ON compute_route_authorization_receipts",
        "BEFORE INSERT ON compute_route_authorization_capabilities",
        "BEFORE INSERT ON compute_route_authorization_seals",
        "BEFORE INSERT ON compute_capacity_pools",
        "BEFORE UPDATE ON compute_capacity_pools",
        "BEFORE INSERT ON compute_capacity_pool_versions",
        "BEFORE INSERT ON compute_offers",
        "BEFORE UPDATE ON compute_offers",
        "BEFORE INSERT ON compute_offer_versions",
    ] {
        assert!(FENCES.contains(required), "missing V254 fence {required}");
    }
    assert!(FENCES.contains("binding.route_adapter_projection_id=NEW.adapter_id"));
    assert!(FENCES.contains("candidate.service_actor_id=NEW.service_actor_id"));
    assert!(FENCES.contains("provider.provider_kind='external_pool'"));
    assert!(FENCES.contains("json_extract(NEW.provider_json,'$.status')='active'"));
    assert!(FENCES.contains("NEW.service_actor_kind='platform_dispatch_service'"));
    assert!(FENCES.contains("OLD.provider_kind IS NOT NEW.provider_kind"));
    assert!(
        FENCES.contains("OLD.provider_kind='external_pool' OR NEW.provider_kind='external_pool'")
    );
    assert!(FENCES.contains("OLD.provider_id IS NOT NEW.provider_id"));
    assert!(FENCES.contains("pool.provider_id=NEW.provider_id AND pool.status='active'"));
    assert!(FENCES.contains("version.provider_id=NEW.provider_id"));
    assert!(FENCES.contains("offer.provider_id=NEW.provider_id"));
    assert!(FENCES.contains("json_extract(version.offer_json,'$.provider_id')=NEW.provider_id"));
    assert!(FENCES.contains("actor.provider_id=NEW.provider_id"));
    assert!(FENCES.contains("credential.provider_id=NEW.provider_id"));
    assert!(FENCES.contains("route.provider_id=NEW.provider_id"));
    assert!(FENCES.contains("pool.status='active'"));
    assert!(FENCES.contains("NEW.status IN ('draft','active')"));
    assert!(FENCES.contains("json_extract(NEW.offer_json,'$.status') IN ('draft','active')"));
    assert!(FENCES.contains("json_extract(NEW.offer_json,'$.provider_kind')='external_pool'"));
}

#[test]
fn v254_upgrade_precheck_refuses_preexisting_partial_activation() {
    for required in [
        "status='active'",
        "json_extract(version.provider_json,'$.status')='active'",
        "compute_route_adapters adapter",
        "compute_route_adapter_versions version",
        "compute_service_actor_authorizations actor",
        "compute_route_credential_versions credential",
        "compute_route_authorization_receipts route",
        "compute_capacity_pools pool",
        "compute_capacity_pool_versions version",
        "compute_offers offer",
        "compute_offer_versions version",
        "offer.status IN ('draft','active')",
        "version.status IN ('draft','active')",
    ] {
        assert!(PRECHECK.contains(required), "missing precheck {required}");
    }
}

fn insert_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    let marker = format!("INSERT INTO {table}(");
    let after = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing INSERT for {table}"))
        .1;
    let columns = after
        .split_once(") VALUES")
        .unwrap_or_else(|| panic!("missing VALUES for {table}"))
        .0;
    columns
        .split(',')
        .map(str::trim)
        .filter(|column| !column.is_empty())
        .collect()
}
