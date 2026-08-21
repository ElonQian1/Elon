const API: &str = include_str!("api.rs");
const MCP: &str = include_str!("mcp.rs");
const SERVICE: &str = include_str!("service.rs");
const TRANSPORT: &str = include_str!("transport.rs");
const STORE_FACADE: &str =
    include_str!("../../store/compute_federation_historical_causal_reference.rs");
const EXECUTION_OWNER: &str = include_str!("../../store/compute_attempt_execution_receipts.rs");
const SETTLEMENT_OWNER: &str = include_str!("../../store/compute_attempt_settlements.rs");
const AGGREGATOR: &str = include_str!("../../compute_federation_mcp.rs");
const MCP_TRANSPORT: &str = include_str!("../../open_commerce_mcp.rs");
const ROUTER: &str = include_str!("../../router.rs");
const COMPUTE_MODULE: &str = include_str!("../mod.rs");

#[test]
fn transport_is_exact_five_key_serialize_only_redacted_view() {
    assert!(TRANSPORT.contains("#[derive(Serialize)]"));
    assert!(!TRANSPORT.contains("Deserialize"));
    let declaration = declaration(TRANSPORT, "struct FederationHistoricalLineageReadDocument");
    let fields = declaration
        .lines()
        .filter(|line| line.trim_end().ends_with(','))
        .map(|line| line.trim().split(':').next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        [
            "schema",
            "lineage_kind",
            "lineage_digest",
            "canonical_carrier_json",
            "read_effect",
        ]
    );
    assert!(TRANSPORT.contains("compute_federation.core_historical_causal_reference.read.v1"));
    assert!(TRANSPORT.contains("read_effect: \"none\""));
    for forbidden in [
        "user_id",
        "project_id",
        "actor",
        "timestamp",
        "current",
        "replayed",
        "status",
    ] {
        assert!(
            !TRANSPORT.contains(forbidden),
            "transport leaks {forbidden}"
        );
    }
}

#[test]
fn http_and_mcp_publish_only_the_frozen_read_abi() {
    for route in [
        "/api/me/compute/attempt-leases/:lease_id/execution-source-lineage",
        "/api/me/compute/attempt-leases/:lease_id/settlement-source-lineage",
        "/api/admin/compute/attempt-leases/:lease_id/execution-source-lineage",
        "/api/admin/compute/attempt-leases/:lease_id/settlement-source-lineage",
    ] {
        assert!(API.contains(route), "missing GET route {route}");
    }
    assert_eq!(API.matches(".route(").count(), 4);
    assert!(API.contains("routing::get"));
    for forbidden in [
        "routing::post",
        "routing::put",
        "routing::delete",
        ".post(",
        ".put(",
        ".delete(",
    ] {
        assert!(
            !API.contains(forbidden),
            "lineage API must be GET-only: {forbidden}"
        );
    }

    for tool in [
        "compute_get_my_execution_source_lineage",
        "compute_get_my_settlement_source_lineage",
        "compute_admin_get_execution_source_lineage",
        "compute_admin_get_settlement_source_lineage",
    ] {
        assert_eq!(MCP.matches(tool).count(), 1, "MCP tool ABI drift: {tool}");
    }
    let schema = MCP
        .split("fn lease_schema()")
        .nth(1)
        .expect("lease-only MCP schema must exist");
    assert!(schema.contains("required\":[\"lease_id\"]"));
    assert!(schema.contains("properties\":{\"lease_id\""));
    assert!(schema.contains("additionalProperties\":false"));
    for forbidden in ["user_id", "project_id", "receipt_id", "digest"] {
        assert!(!schema.contains(forbidden), "MCP input exposes {forbidden}");
    }
    assert!(MCP.contains("Some(project_id)"));
    assert!(SERVICE.contains("lineage.belongs_to_project(project_id)"));
    assert!(MCP_TRANSPORT.contains("project_access(state, &user.id, project_id)"));
    assert!(MCP_TRANSPORT.contains("definitions_for_platform_role(&caller.platform_role)"));
    assert!(MCP_TRANSPORT.contains("call_tool_for_platform_role("));
    assert!(AGGREGATOR.contains("federation_historical_lineage_read::definitions()"));
    assert!(AGGREGATOR.contains("federation_historical_lineage_read::admin_definitions()"));
    assert!(AGGREGATOR.contains("federation_historical_lineage_read::call_if_handled("));
    assert!(AGGREGATOR.contains("federation_historical_lineage_read::call_admin_if_handled("));
    assert!(ROUTER.contains("federation_historical_lineage_read::routes()"));
    assert!(COMPUTE_MODULE.contains("pub(crate) mod federation_historical_lineage_read;"));
}

#[test]
fn adoption_is_historical_read_only_typed_and_redacted() {
    assert!(SERVICE.contains("resolve_compute_execution_source_lineage_for_lease"));
    assert!(SERVICE.contains("resolve_compute_settlement_source_lineage_for_lease"));
    assert!(SERVICE.contains("FederationHistoricalLineageReadError::NotVisible"));
    assert!(SERVICE.contains("FederationHistoricalLineageReadError::IntegrityConflict"));
    for code in [
        "FEDERATION_LINEAGE_INVALID_LEASE_ID",
        "FEDERATION_LINEAGE_INVALID_REQUEST_INPUT",
        "FEDERATION_LINEAGE_NOT_VISIBLE",
        "FEDERATION_LINEAGE_PROJECT_FORBIDDEN",
        "FEDERATION_LINEAGE_NOT_FOUND",
        "FEDERATION_LINEAGE_INTEGRITY_CONFLICT",
        "FEDERATION_LINEAGE_ADMIN_FORBIDDEN",
        "FEDERATION_LINEAGE_UNAUTHENTICATED",
    ] {
        assert!(SERVICE.contains(code), "missing stable error code {code}");
    }
    assert!(API.contains("uri.query().is_some()"));
    assert!(API.contains("to_bytes(body, 1)"));
    assert!(API.contains("Path::<String>::from_request_parts(parts, state)"));
    assert!(!API.contains("Path(lease_id): Path<String>"));

    assert!(STORE_FACADE
        .contains("compute_attempt_historical_execution_receipt_by_lease_on(&tx, lease_id)"));
    assert!(
        STORE_FACADE.contains("compute_attempt_historical_settlement_by_lease_on(&tx, lease_id)")
    );
    assert!(
        EXECUTION_OWNER.contains("fn compute_attempt_historical_execution_receipt_by_lease_on(")
    );
    assert!(SETTLEMENT_OWNER.contains("fn compute_attempt_historical_settlement_by_lease_on("));

    let adoption = [API, MCP, SERVICE, TRANSPORT, STORE_FACADE].join("\n");
    let lower = adoption.to_ascii_lowercase();
    for forbidden in [
        "transactionbehavior::immediate",
        ".execute(",
        ".execute_batch(",
        "insert into",
        "update ",
        "delete from",
        "create table",
        "migration",
        "utc::now",
        "new_id",
        "current_",
        "latest",
    ] {
        assert!(
            !lower.contains(forbidden),
            "read adoption contains {forbidden}"
        );
    }
}

#[test]
fn store_read_result_and_scope_remain_non_clone_non_serde() {
    for marker in [
        "pub(crate) struct ValidatedFederationHistoricalLineage",
        "struct FederationHistoricalLineageAccessScope",
    ] {
        let start = STORE_FACADE
            .find(marker)
            .expect("private Store type must exist");
        let prefix = STORE_FACADE[..start]
            .lines()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !prefix.contains("#[derive"),
            "Store type gained derives: {marker}"
        );
    }
    for forbidden in [
        "impl Clone for ValidatedFederationHistoricalLineage",
        "Serialize for ValidatedFederationHistoricalLineage",
        "Deserialize for ValidatedFederationHistoricalLineage",
        "impl Clone for FederationHistoricalLineageAccessScope",
        "Serialize for FederationHistoricalLineageAccessScope",
        "Deserialize for FederationHistoricalLineageAccessScope",
    ] {
        assert!(!STORE_FACADE.contains(forbidden));
    }
}

fn declaration<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source.find(marker).expect("declaration marker must exist");
    let tail = &source[start..];
    let end = tail.find("\n}").expect("declaration must be bounded");
    &tail[..end]
}
